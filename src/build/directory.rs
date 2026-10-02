//! Directory artifact inventories and integrity. Native output selection stays
//! with builders; source-tree identity and bounded traversal belong to snapshot.
use super::bundle::safe_report_parent;
use crate::snapshot;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn path(root: &Path, relative: &str) -> Result<PathBuf> {
    if !snapshot::portable(relative) {
        bail!("invalid directory artifact path");
    }
    // Include the directory itself in the existing parent-chain validation.
    safe_report_parent(root, &format!("{relative}/entry"))?;
    Ok(root.join(relative))
}

fn size(tree: &snapshot::Snapshot) -> u64 {
    tree.entries.iter().map(|entry| entry.size).sum()
}

pub(super) fn capture(out: &Path, bundle: &Path, intent: &Value) -> Result<Value> {
    let relative = intent["path"]
        .as_str()
        .context("missing directory artifact path")?;
    let source = path(out, relative)?;
    let destination = bundle.join(relative);
    fs::create_dir_all(destination.parent().context("missing artifact parent")?)?;
    let tree = snapshot::capture_prepared(&source, &destination)?;
    let mut artifact = intent.clone();
    artifact["digest"] = json!(tree.digest);
    artifact["size"] = json!(size(&tree));
    artifact["entries"] = json!(tree.entries);
    Ok(artifact)
}

pub(super) fn verify(root: &Path, artifact: &Value) -> Result<PathBuf> {
    let relative = artifact["path"]
        .as_str()
        .context("missing directory artifact path")?;
    let source = path(root, relative)?;
    verify_tree(&snapshot::inspect_tree(&source)?, artifact)?;
    Ok(source)
}

fn verify_tree(tree: &snapshot::Snapshot, artifact: &Value) -> Result<()> {
    if artifact["digest"] != tree.digest
        || artifact["size"] != size(tree)
        || artifact["entries"] != json!(tree.entries)
    {
        bail!("directory artifact inventory, size or content digest mismatch");
    }
    Ok(())
}

/// Flatten the artifact root into a new consumer directory. Capture rejects an
/// existing destination and retains independent bytes and executable intent.
pub(super) fn copy(source: &Path, destination: &Path, artifact: &Value) -> Result<()> {
    let copied = snapshot::capture_prepared(source, destination)?;
    verify_tree(&copied, artifact)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_identity_covers_all_files_empty_directories_and_complete_inventory() {
        let out = tempfile::tempdir().unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let source = out.path().join("site");
        fs::create_dir_all(source.join("dist/empty")).unwrap();
        fs::write(source.join("dist/index.html"), "<h1>Hello</h1>").unwrap();
        fs::write(source.join(".hidden"), "included").unwrap();
        let intent = json!({"kind":"directory","path":"site"});
        let artifact = capture(out.path(), bundle.path(), &intent).unwrap();
        assert_eq!(artifact["size"], 22);
        assert_eq!(artifact["entries"].as_array().unwrap().len(), 4);
        verify(bundle.path(), &artifact).unwrap();
        let other = tempfile::tempdir().unwrap();
        assert_eq!(
            capture(out.path(), other.path(), &intent).unwrap(),
            artifact
        );
        assert!(capture(out.path(), bundle.path(), &intent).is_err());
        let mut changed = artifact.clone();
        changed["entries"].as_array_mut().unwrap().pop();
        assert!(verify(bundle.path(), &changed).is_err());
        fs::write(bundle.path().join("site/new"), "undeclared").unwrap();
        assert!(verify(bundle.path(), &artifact).is_err());
        fs::remove_file(bundle.path().join("site/new")).unwrap();
        fs::remove_dir(bundle.path().join("site/dist/empty")).unwrap();
        assert!(verify(bundle.path(), &artifact).is_err());
        fs::create_dir(bundle.path().join("site/dist/empty")).unwrap();
        fs::write(bundle.path().join("site/dist/index.html"), "changed").unwrap();
        assert!(verify(bundle.path(), &artifact).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn directory_inventory_rejects_links_and_binds_executable_bits() {
        use std::os::unix::{fs::symlink, fs::PermissionsExt};
        let out = tempfile::tempdir().unwrap();
        let bundle = tempfile::tempdir().unwrap();
        fs::create_dir(out.path().join("site")).unwrap();
        fs::write(out.path().join("site/run"), "binary").unwrap();
        fs::set_permissions(
            out.path().join("site/run"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        let artifact = capture(
            out.path(),
            bundle.path(),
            &json!({"kind":"directory","path":"site"}),
        )
        .unwrap();
        assert_eq!(artifact["entries"][0]["executable"], true);
        verify(bundle.path(), &artifact).unwrap();
        fs::set_permissions(
            bundle.path().join("site/run"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(verify(bundle.path(), &artifact).is_err());
        symlink("../outside", out.path().join("site/link")).unwrap();
        let fresh = tempfile::tempdir().unwrap();
        assert!(capture(
            out.path(),
            fresh.path(),
            &json!({"kind":"directory","path":"site"})
        )
        .is_err());
        symlink("site", out.path().join("alias")).unwrap();
        assert!(path(out.path(), "alias").is_err());
    }
}
