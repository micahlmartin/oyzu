//! Conservative reaping of final process-lease records, never installations.
use super::Record;
use crate::tools::{
    lock,
    store::access::{self, Directory, Kind},
};
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::{
    fs::{File, TryLockError},
    io::{self, Read},
    path::Path,
};

#[derive(Debug, Default, Serialize)]
pub struct LeaseRecovery {
    pub dry_run: bool,
    pub stale: usize,
    pub removed: usize,
    pub active_or_busy: usize,
    pub unverified_or_failed: usize,
}

pub(in crate::tools) fn recover(store: &Path, dry_run: bool) -> Result<LeaseRecovery> {
    let root = Directory::open(store)?;
    let mut report = LeaseRecovery {
        dry_run,
        ..Default::default()
    };
    let Some(directory) = optional_child(&root, "leases")? else {
        return Ok(report);
    };
    let locks = optional_child(&root, "locks")?;
    let entries = directory.entries()?;
    ensure!(
        entries.len() <= 4096,
        "lease recovery record limit exceeded"
    );
    let mut budget: usize = 32 * 1024 * 1024;
    for (name, kind) in entries {
        let outcome = (|| -> Result<bool> {
            ensure!(kind == Kind::File, "non-file lease entry");
            let id = name
                .strip_suffix(".json")
                .context("unfinished or unknown lease entry")?;
            validate_id(id)?;
            let locks = locks.as_ref().context("lease lock evidence is missing")?;
            let guard = locks.existing_lock_file(&format!("lease-{id}"))?;
            if !exclusive(&guard)? {
                return Ok(false);
            }
            let file = directory.file(&name)?;
            ensure!(access::identity(&file)?.links == 1, "linked lease record");
            ensure!(budget > 0, "lease recovery byte budget exhausted");
            let mut bytes = Vec::new();
            let limit = budget.min(512 * 1024);
            let read = file.take(limit as u64 + 1).read_to_end(&mut bytes);
            budget = budget.saturating_sub(bytes.len());
            read?;
            ensure!(bytes.len() <= limit, "lease recovery byte budget exceeded");
            let value = crate::config::policy::strict_json_limit(&bytes, 512 * 1024)?;
            let record: Record = serde_json::from_value(value)?;
            validate(&record, id)?;
            // Nonblocking acquisitions avoid cycles with writers, which acquire
            // installation locks before creating their new journal guard.
            let mut held = Vec::new();
            for key in &record.installation_keys {
                let file = locks.existing_lock_file(&key[7..])?;
                if !exclusive(&file)? {
                    return Ok(false);
                }
                held.push(file);
            }
            for key in &record.installation_keys {
                let file = locks.existing_lock_file(&format!("{}.lease", &key[7..]))?;
                if !exclusive(&file)? {
                    return Ok(false);
                }
                held.push(file);
            }
            if !dry_run {
                directory.remove_file(&name)?;
            }
            Ok(true)
        })();
        match outcome {
            Ok(true) => {
                report.stale += 1;
                if !dry_run {
                    report.removed += 1;
                }
            }
            Ok(false) => report.active_or_busy += 1,
            Err(_) => report.unverified_or_failed += 1,
        }
    }
    Ok(report)
}

fn optional_child(root: &Directory, name: &str) -> Result<Option<Directory>> {
    match root.child(name) {
        Ok(directory) => Ok(Some(directory)),
        Err(error)
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::NotFound) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn exclusive(file: &File) -> Result<bool> {
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

fn validate_id(id: &str) -> Result<()> {
    ensure!(id.len() <= 72, "lease identifier too long");
    let parts: Vec<_> = id.split('-').collect();
    ensure!(parts.len() == 3, "invalid lease identifier");
    let pid: u32 = parts[0].parse()?;
    let time: u128 = parts[1].parse()?;
    let sequence: u64 = parts[2].parse()?;
    ensure!(
        pid > 0 && id == format!("{pid}-{time}-{sequence}"),
        "noncanonical lease identifier"
    );
    Ok(())
}

fn validate(record: &Record, id: &str) -> Result<()> {
    ensure!(
        record.format == 1 && record.lease_id == id,
        "lease record identity mismatch"
    );
    let prefix = format!("{}-{}-", record.owner_pid, record.created_unix_nanos);
    ensure!(id.starts_with(&prefix), "lease owner observation mismatch");
    lock::digest(&record.selection_digest)?;
    ensure!(
        record.installation_keys.len() <= 4096,
        "lease closure limit exceeded"
    );
    ensure!(
        record
            .installation_keys
            .windows(2)
            .all(|pair| pair[0] < pair[1]),
        "lease keys are not sorted and unique"
    );
    for key in &record.installation_keys {
        lock::digest(key)?;
    }
    Ok(())
}
