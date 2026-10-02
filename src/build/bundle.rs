//! Bundle path containment, output capture and recorded-content verification.
use crate::bundle_store::{create_output, safe_file};
use crate::{records, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

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
        let invocation = &plan["extensions"]["oyzu.dev/invocation"];
        if manifest["extensions"]["oyzu.dev/invocation"] != *invocation {
            bail!("invocation context differs from plan");
        }
        if !invocation.is_null() {
            let envelope = records::read(&safe_file(
                root,
                manifest["envelopePath"]
                    .as_str()
                    .context("missing envelope path")?,
            )?)?;
            if envelope["extensions"]["oyzu.dev/invocation"] != *invocation {
                bail!("invocation context differs from envelope");
            }
        }
        if manifest["extensions"]["oyzu.dev/selection"] != plan["extensions"]["oyzu.dev/selection"]
        {
            bail!("build selection differs from plan");
        }
        if manifest["targets"] != plan["targets"] {
            bail!("build target identities differ from plan");
        }
        for artifact in manifest["artifacts"]
            .as_array()
            .context("missing bundle artifacts")?
        {
            let declared = plan["artifacts"]
                .as_array()
                .and_then(|artifacts| artifacts.iter().find(|a| a["id"] == artifact["id"]))
                .context("bundle artifact is not declared by its plan")?;
            if artifact["target"] != declared["target"]
                || artifact["variant"] != declared["variant"]
            {
                bail!("artifact target/variant differs from plan");
            }
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
            if field == "artifacts" && item["kind"] == "directory" {
                super::directory::verify(root, item)?;
                continue;
            }
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
    if let Some(inventory) = manifest["extensions"].get("oyzu.dev/host-outputs") {
        let entries: Vec<snapshot::Entry> = serde_json::from_value(inventory.clone())?;
        crate::bundle_store::verify_host_outputs(root, &entries)?;
    }
    Ok(manifest)
}
