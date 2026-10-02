//! Hold the invocation lock through publication and release it explicitly.
use anyhow::{Context, Result};
use std::fs::File;

pub(super) struct WorkspaceLock(File);

impl WorkspaceLock {
    pub(super) fn acquire(file: File) -> Result<Self> {
        file.try_lock()
            .context("another build holds the workspace lock")?;
        Ok(Self(file))
    }
}

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        // On Unix, a concurrent fork can briefly retain the same open file
        // description. Closing our handle alone need not release its flock.
        // Explicit unlock ends this invocation's ownership even in that case.
        let _ = self.0.unlock();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn retained_description_does_not_extend_finished_invocation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("build.lock");
        let file = File::create(&path).unwrap();
        let retained = file.try_clone().unwrap();
        let owner = WorkspaceLock::acquire(file).unwrap();
        assert!(WorkspaceLock::acquire(File::open(&path).unwrap()).is_err());
        drop(owner);
        let next = WorkspaceLock::acquire(File::open(&path).unwrap()).unwrap();
        drop(retained);
        drop(next);
    }
}
