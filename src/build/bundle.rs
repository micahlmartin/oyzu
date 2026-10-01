//! Bundle path containment, output capture and recorded-content verification.
use crate::{records, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Retain bounded raw evidence, including bytes that a report parser may reject.
/// Read before creating the destination so oversized inputs leave no bundle file.
pub(super) fn capture_bounded_output(
    out: &Path,
    source_relative: &str,
    bundle: &Path,
    relative: &str,
    limit: u64,
) -> Result<PathBuf> {
    let source = safe_file(out, source_relative)?;
    if relative.contains(['\\', ':'])
        || relative
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        bail!("invalid report destination");
    }
    let mut bytes = Vec::new();
    fs::File::open(source)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("report exceeds {limit} bytes");
    }
    let destination = bundle.join(relative);
    let mut output = create_output(&destination)?;
    output.write_all(&bytes)?;
    output.sync_all()?;
    Ok(destination)
}

fn create_output(destination: &Path) -> Result<fs::File> {
    fs::create_dir_all(destination.parent().context("missing output parent")?)?;
    Ok(fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?)
}

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
    let mut input = fs::File::open(source)?;
    let mut output = create_output(&destination)?;
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
            if field == "artifacts"
                && matches!(item["kind"].as_str(), Some("oci-image" | "oci-index"))
                && item["path"].as_str().is_none()
            {
                bail!("OCI bundle artifact has no contained archive path");
            }
            if let Some(path) = item["path"].as_str() {
                let file = safe_file(root, path)?;
                if snapshot::file_digest(&file)? != item["digest"] {
                    bail!("{path}: content digest mismatch");
                }
                if field == "artifacts" && Some(fs::metadata(&file)?.len()) != item["size"].as_u64()
                {
                    bail!("{path}: size mismatch");
                }
                if field == "artifacts"
                    && matches!(item["kind"].as_str(), Some("oci-image" | "oci-index"))
                {
                    let verified = crate::oci::verify(&file)?;
                    if item["ociDigest"] != verified.digest
                        || item["kind"] != verified.kind
                        || verified.platforms.is_empty()
                    {
                        bail!("{path}: OCI publication identity mismatch");
                    }
                }
            }
        }
    }
    Ok(manifest)
}
