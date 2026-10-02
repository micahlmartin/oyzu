//! Durable observations of cooperative OS leases, never execution authority.
mod recovery;
use super::access::Directory;
use anyhow::Result;
pub(in crate::tools) use recovery::recover;
pub use recovery::LeaseRecovery;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    format: u32,
    lease_id: String,
    owner_pid: u32,
    created_unix_nanos: String,
    selection_digest: String,
    installation_keys: Vec<String>,
}

pub(super) struct Journal {
    directory: Directory,
    id: String,
    _guard: File,
}

impl Journal {
    /// Caller holds all selected mutation and shared lease locks. Publication
    /// is no-replace; a crash can leave a pending record or a stale final record.
    /// PID and timestamps are diagnostic data, not evidence of process liveness.
    pub(super) fn create(root: &Directory, selection: &str, keys: &BTreeSet<&str>) -> Result<Self> {
        let directory = root.create_directory("leases")?;
        let created = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        // Exclusive creation establishes ownership; this identifier is neither
        // an authentication secret nor a reusable process identity.
        let id = format!(
            "{}-{created}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let pending = format!("{id}.pending");
        let guard = root
            .create_directory("locks")?
            .lock_file(&format!("lease-{id}"))?;
        guard.try_lock()?;
        let mut output = directory.create_file(&pending)?;
        let written = (|| -> Result<()> {
            serde_json::to_writer(
                &mut output,
                &Record {
                    format: 1,
                    lease_id: id.clone(),
                    owner_pid: std::process::id(),
                    created_unix_nanos: created.to_string(),
                    selection_digest: selection.into(),
                    installation_keys: keys.iter().map(|key| (*key).to_owned()).collect(),
                },
            )?;
            output.write_all(b"\n")?;
            output.sync_all()?;
            Ok(())
        })();
        drop(output);
        let published =
            written.and_then(|()| directory.publish(&pending, &directory, &format!("{id}.json")));
        if let Err(error) = published {
            // Remove only this operation's exclusively created temporary name.
            // If publication succeeded but its directory sync failed, preserve
            // the final record as uncertain recovery evidence.
            let _ = directory.remove_file(&pending);
            return Err(error);
        }
        root.sync()?;
        Ok(Self {
            directory,
            id,
            _guard: guard,
        })
    }

    pub(super) fn id(&self) -> &str {
        &self.id
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        // The containing InstallationLease drops this field before releasing
        // kernel locks. Failed cleanup conservatively leaves a stale record.
        let _ = self.directory.remove_file(&format!("{}.json", self.id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_record_is_protected_by_its_journal_lock() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().canonicalize().unwrap();
        let root = Directory::open(&path).unwrap();
        let journal = Journal::create(
            &root,
            &format!("sha256:{}", "a".repeat(64)),
            &BTreeSet::new(),
        )
        .unwrap();
        let report = recover(&path, false).unwrap();
        assert_eq!(report.active_or_busy, 1);
        assert_eq!(report.removed, 0);
        drop(journal);
        assert!(root.child("leases").unwrap().entries().unwrap().is_empty());
    }
}
