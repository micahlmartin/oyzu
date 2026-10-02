//! Cooperative store transactions. Policy/layout admission precedes this layer.
use super::{
    access::{Directory, Kind},
    lease::{Journal, OwnedLease},
    receipt,
};
use crate::tools::lock::Lock;
use anyhow::{ensure, Context, Result};
use std::{
    collections::BTreeSet,
    fs::File,
    io,
    path::Path,
    time::{Duration, Instant},
};

/// Holds shared OS leases for all installations in a verified closure. Keep this
/// value alive until the entire supervised child tree or shell session ends.
/// This does not grant authorization and is not a same-user tamper boundary.
pub struct InstallationLease {
    pub selection_digest: String,
    // Field drop order matters: remove the journal while kernel leases remain.
    journal: Journal,
    _leases: Vec<OwnedLease>,
    _root: Directory,
    verified: receipt::VerifiedSelection,
}

impl InstallationLease {
    /// Diagnostic journal identity, not an authorization token. Abnormal process
    /// exit may leave this record after its kernel locks have been released.
    pub fn lease_id(&self) -> &str {
        self.journal.id()
    }

    /// Look up only commands in the receipt snapshot verified when this lease
    /// was acquired. Never searches PATH, guesses wrappers or runs a tool. The
    /// returned borrow keeps metadata tied to this lease; callers must separately
    /// revalidate current content/authority before any eventual launch.
    pub fn command(&self, name: &str) -> Result<receipt::LeasedToolCommand<'_>> {
        self.verified.command(name)
    }
}

pub(in crate::tools) fn transact(
    lock: &Lock,
    store: &Path,
    staging: Option<&Path>,
    scope: &str,
    profile: &str,
    platform: &str,
    installer: &str,
) -> Result<InstallationLease> {
    let selection = lock
        .selections
        .iter()
        .find(|s| s.scope == scope && s.profile == profile && s.platform == platform)
        .context("locked selection is unavailable")?;
    let root = Directory::open(store)?;
    let locks = root.create_directory("locks")?;
    let installs = root.create_directory("installs")?;
    let candidates = staging
        .map(Directory::open)
        .transpose()?
        .map(|dir| dir.child("installs"))
        .transpose()?;
    let keys: BTreeSet<_> = selection
        .installation_keys
        .values()
        .map(String::as_str)
        .collect();
    let mut held = Vec::new();
    // Global lexical order avoids cycles even for overlapping selections.
    let deadline = Instant::now() + Duration::from_secs(30);
    for key in &keys {
        let file = locks.lock_file(&key[7..])?;
        acquire(&file, deadline, false)?;
        held.push(file);
    }
    let mut missing = BTreeSet::new();
    for key in &keys {
        match installs.child(&key[7..]) {
            Ok(_) => {}
            Err(error)
                if error
                    .downcast_ref::<io::Error>()
                    .is_some_and(|e| e.kind() == io::ErrorKind::NotFound) =>
            {
                ensure!(candidates.is_some(), "locked installation is missing");
                missing.insert(*key);
            }
            Err(error) => return Err(error),
        }
    }
    // Validate the entire closure before publishing any new member. Existing
    // committed entries are always reverified, never overwritten or repaired.
    let verified = receipt::verify_with(lock, scope, profile, platform, installer, |key| {
        if missing.contains(key) {
            candidates
                .as_ref()
                .context("missing staging")?
                .child(&key[7..])
        } else {
            installs.child(&key[7..])
        }
    })?;
    for key in &missing {
        let candidates = candidates.as_ref().context("missing staging")?;
        {
            let candidate = candidates.child(&key[7..])?;
            sync_tree(&candidate.child("payload")?, 0)?;
            candidate.sync_file("receipt.json")?;
            candidate.sync()?;
        }
        // Child handles are closed here on Windows; both parents stay pinned.
        candidates.publish(&key[7..], &installs, &key[7..])?;
    }
    // Shared lease locks are acquired while the exclusive mutation locks are
    // still held. Prune must take mutation first, then try an exclusive lease.
    let mut leases = Vec::new();
    for key in &keys {
        let file = locks.lock_file(&format!("{}.lease", &key[7..]))?;
        acquire(&file, deadline, true)?;
        leases.push(OwnedLease(file));
    }
    let journal = Journal::create(&root, &verified.digest, &keys)?;
    drop(held);
    Ok(InstallationLease {
        selection_digest: verified.digest.clone(),
        journal,
        _leases: leases,
        _root: root,
        verified,
    })
}

pub(super) fn acquire(file: &File, deadline: Instant, shared: bool) -> Result<()> {
    loop {
        match if shared {
            file.try_lock_shared()
        } else {
            file.try_lock()
        } {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {
                ensure!(Instant::now() < deadline, "store lock deadline exceeded");
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
        }
    }
}

fn sync_tree(directory: &Directory, depth: usize) -> Result<()> {
    ensure!(depth <= 64, "payload sync depth exceeded");
    for (name, kind) in directory.entries()? {
        match kind {
            Kind::File => directory.sync_file(&name)?,
            Kind::Directory => sync_tree(&directory.child(&name)?, depth + 1)?,
            Kind::Symlink => {}
        }
    }
    directory.sync()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_never_replaces_an_empty_destination_directory() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().canonicalize().unwrap();
        std::fs::create_dir(path.join("candidate")).unwrap();
        std::fs::create_dir(path.join("occupied")).unwrap();
        std::fs::write(path.join("candidate/receipt.json"), b"candidate").unwrap();
        let root = Directory::open(&path).unwrap();
        assert!(root.publish("candidate", &root, "occupied").is_err());
        assert_eq!(
            std::fs::read(path.join("candidate/receipt.json")).unwrap(),
            b"candidate"
        );
        assert_eq!(std::fs::read_dir(path.join("occupied")).unwrap().count(), 0);
    }
}
