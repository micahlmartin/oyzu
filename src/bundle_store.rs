//! Workspace bundle publication and retention. Callers own record contents and
//! execution; this module owns the shared lock, staging and destination checks.
//! No native commands, report interpretation or ecosystem rules belong here.
use crate::{records, snapshot};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) struct Transaction {
    state: PathBuf,
    dist: PathBuf,
    previous: Option<String>,
    stage: tempfile::TempDir,
    // Keep the lock alive through publication and rollback, including failures.
    _lock: fs::File,
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let redirected = metadata.file_type().is_symlink();
            #[cfg(windows)]
            let redirected = {
                use std::os::windows::fs::MetadataExt;
                redirected || metadata.file_attributes() & 0x400 != 0
            };
            if redirected {
                bail!(
                    "bundle destination must not contain a symbolic link or reparse point: {}",
                    path.display()
                );
            }
            Ok(Some(metadata))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn directory(path: &Path) -> Result<()> {
    if let Some(metadata) = metadata(path)? {
        if !metadata.is_dir() {
            bail!("bundle directory is not a directory: {}", path.display());
        }
    } else {
        fs::create_dir(path)?;
    }
    Ok(())
}

fn identity(dist: &Path) -> Result<Option<String>> {
    let Some(entry) = metadata(dist)? else {
        return Ok(None);
    };
    let marker = dist.join("manifest.json");
    if !entry.is_dir()
        || metadata(&marker)?.is_none_or(|m| !m.is_file())
        || records::read(&marker)
            .ok()
            .is_none_or(|v| v["kind"] != "build-manifest")
    {
        bail!("dist exists and is not an Oyzu bundle; preserve or move it before building");
    }
    Ok(Some(snapshot::file_digest(&marker)?))
}

impl Transaction {
    pub fn begin(root: &Path) -> Result<Self> {
        let state = root.join(".oyzu");
        directory(&state)?;
        let lock_path = state.join("build.lock");
        if metadata(&lock_path)?.is_some_and(|m| !m.is_file()) {
            bail!("bundle lock is not a regular file");
        }
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)?;
        lock.try_lock()
            .context("another build holds the workspace lock")?;
        let dist = root.join("dist");
        let previous = identity(&dist)?;
        let stage = tempfile::Builder::new()
            .prefix("bundle-")
            .tempdir_in(&state)?;
        Ok(Self {
            state,
            dist,
            previous,
            stage,
            _lock: lock,
        })
    }

    pub fn path(&self) -> &Path {
        self.stage.path()
    }

    /// Publish only finalized records. If publication fails, preserve the staged
    /// evidence and report its location instead of discarding diagnostic output.
    /// This guards cooperating invocations and detected destination changes; it
    /// is not a filesystem transaction against hostile concurrent mutation.
    pub fn publish(self, run_id: &str) -> Result<()> {
        let result = self.publish_inner(run_id);
        if let Err(error) = result {
            let retained = self.stage.keep();
            return Err(error.context(format!(
                "unpublished bundle retained at {}",
                retained.display()
            )));
        }
        Ok(())
    }

    fn publish_inner(&self, run_id: &str) -> Result<()> {
        if run_id.is_empty()
            || run_id.len() > 255
            || !run_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            bail!("invalid bundle run identity");
        }
        metadata(&self.state)?.context("bundle state directory disappeared")?;
        identity(self.stage.path())?.context("staged bundle has no finalized manifest")?;
        if identity(&self.dist)? != self.previous {
            bail!("dist changed during the invocation; refusing to replace it");
        }
        if self.previous.is_some() {
            let history = self.state.join("history");
            directory(&history)?;
            let previous = history.join(run_id);
            if metadata(&previous)?.is_some() {
                bail!("bundle history destination already exists");
            }
            fs::rename(&self.dist, &previous)?;
            if let Err(error) = fs::rename(self.stage.path(), &self.dist) {
                fs::rename(&previous, &self.dist).with_context(|| {
                    format!(
                        "publication failed ({error}); could not restore previous bundle from {}",
                        previous.display()
                    )
                })?;
                return Err(error.into());
            }
        } else {
            fs::rename(self.stage.path(), &self.dist)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn finalize(path: &Path, value: &str) {
        records::write(
            &path.join("manifest.json"),
            &json!({"kind":"build-manifest","runId":value}),
        )
        .unwrap();
    }

    #[test]
    fn lock_staging_and_retention_share_one_lifecycle() {
        let root = tempfile::tempdir().unwrap();
        let first = Transaction::begin(root.path()).unwrap();
        let stage = first.path().to_owned();
        assert!(Transaction::begin(root.path())
            .err()
            .unwrap()
            .to_string()
            .contains("workspace lock"));
        finalize(first.path(), "first");
        first.publish("123-456").unwrap();
        assert!(!stage.exists());
        let second = Transaction::begin(root.path()).unwrap();
        finalize(second.path(), "second");
        second.publish("second").unwrap();
        assert_eq!(
            records::read(&root.path().join("dist/manifest.json")).unwrap()["runId"],
            "second"
        );
        assert_eq!(
            records::read(&root.path().join(".oyzu/history/second/manifest.json")).unwrap()
                ["runId"],
            "first"
        );
        let uncommitted = Transaction::begin(root.path()).unwrap();
        let abandoned = uncommitted.path().to_owned();
        drop(uncommitted);
        assert!(!abandoned.exists());
        assert!(root.path().join("dist/manifest.json").is_file());
    }

    #[test]
    fn changed_or_foreign_destinations_preserve_both_source_and_new_evidence() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("dist")).unwrap();
        fs::write(root.path().join("dist/foreign"), "keep").unwrap();
        assert!(Transaction::begin(root.path()).is_err());
        finalize(&root.path().join("dist"), "old");
        let transaction = Transaction::begin(root.path()).unwrap();
        let stage = transaction.path().to_owned();
        finalize(&stage, "new");
        finalize(&root.path().join("dist"), "concurrent");
        let error = transaction.publish("new").unwrap_err();
        assert!(format!("{error:#}").contains("dist changed"));
        assert!(stage.join("manifest.json").is_file());
        assert_eq!(
            fs::read_to_string(root.path().join("dist/foreign")).unwrap(),
            "keep"
        );
        assert_eq!(
            records::read(&root.path().join("dist/manifest.json")).unwrap()["runId"],
            "concurrent"
        );
        // Failed publication releases the invocation lock.
        assert!(Transaction::begin(root.path()).is_ok());
    }

    #[test]
    fn a_destination_created_during_execution_and_history_collisions_are_not_overwritten() {
        let root = tempfile::tempdir().unwrap();
        let transaction = Transaction::begin(root.path()).unwrap();
        finalize(transaction.path(), "new");
        fs::create_dir(root.path().join("dist")).unwrap();
        finalize(&root.path().join("dist"), "concurrent");
        assert!(transaction.publish("new").is_err());
        let transaction = Transaction::begin(root.path()).unwrap();
        finalize(transaction.path(), "later");
        fs::create_dir_all(root.path().join(".oyzu/history/later")).unwrap();
        fs::write(root.path().join(".oyzu/history/later/keep"), "keep").unwrap();
        assert!(transaction.publish("later").is_err());
        assert_eq!(
            records::read(&root.path().join("dist/manifest.json")).unwrap()["runId"],
            "concurrent"
        );
        assert!(root.path().join(".oyzu/history/later/keep").is_file());
    }

    #[test]
    fn unfinished_records_and_escaping_run_ids_cannot_be_published() {
        let root = tempfile::tempdir().unwrap();
        let transaction = Transaction::begin(root.path()).unwrap();
        let stage = transaction.path().to_owned();
        fs::write(stage.join("test-output"), "evidence").unwrap();
        assert!(transaction.publish("123-456").is_err());
        assert!(stage.join("test-output").is_file());
        assert!(!root.path().join("dist").exists());
        for id in ["../escape", "nested/run", "", "C:drive"] {
            let transaction = Transaction::begin(root.path()).unwrap();
            finalize(transaction.path(), "new");
            assert!(transaction.publish(id).is_err());
            assert!(!root.path().join("dist").exists());
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_junctions_cannot_redirect_bundle_storage() {
        let destination = tempfile::tempdir().unwrap();
        fs::write(destination.path().join("keep"), "outside").unwrap();
        for name in [".oyzu", "dist"] {
            let root = tempfile::tempdir().unwrap();
            let status = std::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command",
                    "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:OYZU_TEST_LINK -Target $env:OYZU_TEST_DEST | Out-Null"])
                .env("OYZU_TEST_LINK", root.path().join(name))
                .env("OYZU_TEST_DEST", destination.path())
                .output().unwrap();
            assert!(
                status.status.success(),
                "{}",
                String::from_utf8_lossy(&status.stderr)
            );
            assert!(Transaction::begin(root.path()).is_err());
            // Remove only the link. Do not recursively delete a junction target.
            fs::remove_dir(root.path().join(name)).unwrap();
            assert_eq!(
                fs::read_to_string(destination.path().join("keep")).unwrap(),
                "outside"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn dangling_and_live_symlinks_are_rejected_without_following_them() {
        use std::os::unix::fs::symlink;
        for name in [".oyzu", "dist", ".oyzu/build.lock"] {
            let root = tempfile::tempdir().unwrap();
            let link = root.path().join(name);
            fs::create_dir_all(link.parent().unwrap()).unwrap();
            symlink(root.path().join("absent"), &link).unwrap();
            assert!(Transaction::begin(root.path()).is_err(), "{name}");
            assert!(!root.path().join("absent").exists());
        }
    }
}
