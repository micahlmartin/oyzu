//! Bundle path containment, output capture and recorded-content verification.
use crate::{records, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn safe_file(root: &Path, relative: &str) -> Result<PathBuf> {
    let mut path = root.to_path_buf();
    if relative.is_empty() || relative.contains('\\') || relative.contains(':') {
        bail!("invalid bundle path");
    }
    for part in relative.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            bail!("invalid bundle path");
        }
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("bundle path is a symlink");
        }
    }
    if !path.is_file() {
        bail!("bundle output is not a regular file");
    }
    Ok(path)
}

pub(super) fn safe_report_parent(root: &Path, relative: &str) -> Result<()> {
    let mut path = root.to_path_buf();
    let parts: Vec<_> = relative.split('/').collect();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if part.is_empty() || *part == "." || *part == ".." || part.contains(['\\', ':']) {
            bail!("invalid report directory");
        }
        path.push(part);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("unsafe report directory");
        }
    }
    Ok(())
}

pub(super) fn capture_output(out: &Path, bundle: &Path, relative: &str) -> Result<PathBuf> {
    let source = safe_file(out, relative)?;
    let destination = bundle.join(relative);
    fs::create_dir_all(destination.parent().context("missing output parent")?)?;
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)?;
    std::io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let executable = input.metadata()?.permissions().mode() & 0o111 != 0;
        fs::set_permissions(
            &destination,
            fs::Permissions::from_mode(if executable { 0o755 } else { 0o644 }),
        )?;
    }
    Ok(destination)
}

/// Verify recorded content; this is integrity checking, not producer authentication.
pub fn inspect(root: &Path) -> Result<Value> {
    let manifest = records::read(&safe_file(root, "manifest.json")?)?;
    if manifest["kind"] != "build-manifest" {
        bail!("not an Oyzu build bundle");
    }
    {
        let (path_key, digest_key) = ("envelopePath", "envelopeDigest");
        let path = manifest[path_key]
            .as_str()
            .context("missing envelope path")?;
        if snapshot::file_digest(&safe_file(root, path)?)? != manifest[digest_key] {
            bail!("envelope digest mismatch");
        }
    }
    if let Some(path) = manifest["planPath"].as_str() {
        let plan = records::read(&safe_file(root, path)?)?;
        if records::digest("oyzu.plan.v1alpha1", &plan)? != manifest["planDigest"] {
            bail!("plan digest mismatch");
        }
    } else if manifest["status"] == "succeeded" {
        bail!("successful bundle has no plan");
    }
    for field in ["artifacts", "reports", "evidence"] {
        if field == "evidence" && manifest.get(field).is_none() {
            continue;
        }
        for item in manifest[field]
            .as_array()
            .context("missing bundle records")?
        {
            if let Some(path) = item["path"].as_str() {
                let file = safe_file(root, path)?;
                if snapshot::file_digest(&file)? != item["digest"] {
                    bail!("{path}: content digest mismatch");
                }
                if field == "artifacts" && Some(fs::metadata(file)?.len()) != item["size"].as_u64()
                {
                    bail!("{path}: size mismatch");
                }
            }
        }
    }
    Ok(manifest)
}
