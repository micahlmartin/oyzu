//! Preserve native dist files across host test-only bundles. Existing engine
//! evidence is retained through history, never mixed into a new invocation.
use super::{metadata, Transaction};
use crate::{records, snapshot};
use anyhow::{bail, Context, Result};
use std::{collections::BTreeSet, fs, path::Path};

pub(super) fn identity(dist: &Path) -> Result<Option<String>> {
    let Some(entry) = metadata(dist)? else {
        return Ok(None);
    };
    if !entry.is_dir() {
        bail!("native dist output must be a directory");
    }
    if metadata(&dist.join("manifest.json"))?.is_some() {
        return super::identity(dist);
    }
    Ok(Some(format!(
        "native:{}",
        snapshot::inspect_tree(dist)?.digest
    )))
}

pub(super) fn preserve(transaction: &mut Transaction) -> Result<Vec<snapshot::Entry>> {
    let current = identity(&transaction.dist)?;
    // A native task may create/change its output files. It may not replace an
    // already-finalized manifest or introduce an unexpected competing bundle.
    if transaction
        .previous
        .as_ref()
        .is_some_and(|v| !v.starts_with("native:"))
    {
        if current != transaction.previous {
            bail!("dist manifest changed during host tests");
        }
    } else if current.as_ref().is_some_and(|v| !v.starts_with("native:")) {
        bail!("another bundle appeared during host tests");
    }
    if current.is_none() {
        transaction.previous = None;
        return Ok(Vec::new());
    }
    let control = tempfile::tempdir_in(&transaction.state)?;
    let copy = control.path().join("native");
    let tree = snapshot::capture_prepared(&transaction.dist, &copy)?;
    let mut managed = BTreeSet::new();
    if copy.join("manifest.json").exists() {
        let manifest = records::read(&copy.join("manifest.json"))?;
        managed.extend(["manifest.json", "plan.json", "envelope.json", "logs"].map(str::to_owned));
        // Previous artifacts remain ordinary host files, without inheriting
        // artifact claims in this test-only invocation.
        for kind in ["reports", "evidence"] {
            for record in manifest[kind]
                .as_array()
                .context("invalid prior bundle inventory")?
            {
                if let Some(path) = record["path"].as_str() {
                    if !snapshot::portable(path) {
                        bail!("invalid prior bundle output path");
                    }
                    managed.insert(path.into());
                }
            }
        }
    } else if ["plan.json", "envelope.json", "logs"]
        .iter()
        .any(|name| copy.join(name).exists())
    {
        bail!("native dist contains reserved test bundle paths; preserve or relocate the conflicting output");
    }
    let mut retained: Vec<_> = tree
        .entries
        .into_iter()
        .filter(|entry| {
            !managed
                .iter()
                .any(|path| entry.path == *path || entry.path.starts_with(&format!("{path}/")))
        })
        .collect();
    // Empty ancestors belonging only to prior evidence are not native output.
    let files: Vec<_> = retained
        .iter()
        .filter(|e| e.kind == "file")
        .map(|e| e.path.clone())
        .collect();
    retained.retain(|entry| {
        entry.kind != "directory"
            || !managed
                .iter()
                .any(|p| p.starts_with(&format!("{}/", entry.path)))
            || files
                .iter()
                .any(|p| p.starts_with(&format!("{}/", entry.path)))
    });
    for entry in &retained {
        let path = transaction.stage.path().join(&entry.path);
        if entry.kind == "directory" {
            if path.exists() && !path.is_dir() {
                bail!("native output collides with test evidence: {}", entry.path);
            }
            fs::create_dir_all(path)?;
        } else {
            if path.exists() {
                bail!("native output collides with test evidence: {}", entry.path);
            }
            let parent = path.parent().context("missing native output parent")?;
            fs::create_dir_all(parent)?;
            let mut source = fs::File::open(copy.join(&entry.path))?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            std::io::copy(&mut source, &mut output)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    &path,
                    fs::Permissions::from_mode(if entry.executable { 0o755 } else { 0o644 }),
                )?;
            }
        }
    }
    if snapshot::inspect_tree(&transaction.dist)?.digest != tree.digest {
        bail!("native dist changed while preserving test output");
    }
    transaction.previous = identity(&transaction.dist)?;
    Ok(retained)
}

/// Verify preserved native bytes without classifying them as build artifacts.
pub(super) fn verify(root: &Path, entries: &[snapshot::Entry]) -> Result<()> {
    let actual = snapshot::inspect_tree(root)?;
    let actual: std::collections::BTreeMap<_, _> = actual
        .entries
        .iter()
        .map(|entry| (&entry.path, entry))
        .collect();
    let mut seen = BTreeSet::new();
    for entry in entries {
        if !snapshot::portable(&entry.path) || !seen.insert(&entry.path) {
            bail!("invalid preserved host output inventory");
        }
        if actual.get(&entry.path).copied() != Some(entry) {
            bail!(
                "{}: preserved host output differs from inventory",
                entry.path
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn native_files_survive_repeated_tests_without_reusing_prior_reports() {
        let root = tempfile::tempdir().unwrap();
        let dist = root.path().join("dist");
        fs::create_dir_all(dist.join("app")).unwrap();
        fs::write(dist.join("app/main.js"), "application").unwrap();
        for run in ["first", "second"] {
            let mut transaction = Transaction::begin_host(root.path()).unwrap();
            fs::create_dir(transaction.path().join("reports")).unwrap();
            fs::write(transaction.path().join("reports/current.xml"), run).unwrap();
            let outputs = transaction.preserve_host_outputs().unwrap();
            assert_eq!(outputs.len(), 2);
            assert!(outputs.iter().all(|entry| entry.path.starts_with("app")));
            records::write(
                &transaction.path().join("manifest.json"),
                &json!({
                    "kind":"build-manifest", "reports":[{"path":"reports/current.xml"}],
                    "artifacts":[],"evidence":[],"extensions":{"oyzu.dev/host-outputs":outputs}
                }),
            )
            .unwrap();
            transaction.publish(run).unwrap();
            assert_eq!(
                fs::read_to_string(dist.join("app/main.js")).unwrap(),
                "application"
            );
            assert_eq!(
                fs::read_to_string(dist.join("reports/current.xml")).unwrap(),
                run
            );
        }
        assert_eq!(
            fs::read_to_string(root.path().join(".oyzu/history/second/reports/current.xml"))
                .unwrap(),
            "first"
        );
    }

    #[test]
    fn captured_artifact_bytes_are_preserved_as_unclaimed_host_output() {
        let root = tempfile::tempdir().unwrap();
        let dist = root.path().join("dist");
        fs::create_dir(&dist).unwrap();
        fs::write(dist.join("app.tgz"), "prior artifact").unwrap();
        records::write(
            &dist.join("manifest.json"),
            &json!({"kind":"build-manifest",
            "artifacts":[{"path":"app.tgz"}],"reports":[],"evidence":[]}),
        )
        .unwrap();
        let mut transaction = Transaction::begin_host(root.path()).unwrap();
        let outputs = transaction.preserve_host_outputs().unwrap();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].path, "app.tgz");
        verify(transaction.path(), &outputs).unwrap();
        fs::write(transaction.path().join("app.tgz"), "changed").unwrap();
        assert!(verify(transaction.path(), &outputs).is_err());
    }

    #[test]
    fn native_reserved_paths_and_evidence_collisions_are_never_overwritten() {
        for name in ["plan.json", "logs", "app.js"] {
            let root = tempfile::tempdir().unwrap();
            fs::create_dir(root.path().join("dist")).unwrap();
            fs::write(root.path().join("dist").join(name), "keep").unwrap();
            let mut transaction = Transaction::begin_host(root.path()).unwrap();
            fs::write(transaction.path().join(name), "new evidence").unwrap();
            assert!(transaction.preserve_host_outputs().is_err());
            assert_eq!(
                fs::read_to_string(root.path().join("dist").join(name)).unwrap(),
                "keep"
            );
        }
    }
}
