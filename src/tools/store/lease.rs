//! Durable observations of cooperative OS leases, never execution authority.
use super::access::Directory;
use anyhow::Result;
use serde::Serialize;
use std::{
    collections::BTreeSet,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
struct Record<'a> {
    format: u32,
    lease_id: &'a str,
    owner_pid: u32,
    created_unix_nanos: String,
    selection_digest: &'a str,
    installation_keys: &'a BTreeSet<&'a str>,
}

pub(super) struct Journal {
    directory: Directory,
    id: String,
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
        let mut output = directory.create_file(&pending)?;
        let written = (|| -> Result<()> {
            serde_json::to_writer(
                &mut output,
                &Record {
                    format: 1,
                    lease_id: &id,
                    owner_pid: std::process::id(),
                    created_unix_nanos: created.to_string(),
                    selection_digest: selection,
                    installation_keys: keys,
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
        Ok(Self { directory, id })
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
