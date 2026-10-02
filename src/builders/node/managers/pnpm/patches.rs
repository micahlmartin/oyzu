//! Bind source patches to dependency evidence after native frozen validation.
use crate::{dependencies::Prepared, records, snapshot};
use anyhow::{ensure, Context, Result};
use serde_json::json;
use std::{fs, path::Path};

pub(super) fn record(root: &Path, prepared: &mut Prepared) -> Result<()> {
    let lock: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(root.join("pnpm-lock.yaml"))?)?;
    let Some(patches) = lock.get("patchedDependencies") else {
        return Ok(());
    };
    let mut evidence = Vec::new();
    for (selector, patch) in patches
        .as_mapping()
        .context("invalid pnpm patch inventory")?
    {
        let path = patch["path"].as_str().context("missing pnpm patch path")?;
        let normalized = path
            .split('/')
            .filter(|part| *part != ".")
            .collect::<Vec<_>>()
            .join("/");
        ensure!(
            snapshot::portable(&normalized),
            "nonportable pnpm patch path"
        );
        let file = root.join(&normalized);
        ensure!(
            file.canonicalize()?.starts_with(root.canonicalize()?),
            "pnpm patch escapes captured source"
        );
        evidence.push(json!({
            "selector":selector.as_str().context("invalid pnpm patch selector")?,
            "path":normalized, "digest":snapshot::file_digest(&file)?,
            "size":fs::metadata(file)?.len(),
            "nativeHash":patch["hash"].as_str().context("missing native pnpm patch hash")?
        }));
    }
    evidence.sort_by(|left, right| left["selector"].as_str().cmp(&right["selector"].as_str()));
    prepared.record["extensions"]["oyzu.dev/pnpm"]["sourcePatches"] = json!(evidence);
    prepared.digest = records::digest("oyzu.dependencies.v1alpha1", &prepared.record)?;
    Ok(())
}
