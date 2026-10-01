//! Node observations; selection belongs to the shared role resolver.
mod frameworks;
mod managers;

use crate::discovery::detectors::{exclusive, Source};
use crate::discovery::Resolution;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;

pub(super) struct Profile {
    pub package: Value,
    pub manager: Resolution,
    pub framework: Resolution,
    pub locked: bool,
}

struct ContextData {
    source: Source,
    package: Value,
}
pub(super) fn detect(root: &Path) -> Result<Profile> {
    let mut inputs = vec![
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
    ];
    inputs.extend(frameworks::inputs());
    let source = Source::read(root, &inputs)?;
    let package: Value = serde_json::from_str(
        source
            .text("package.json")
            .context("missing package.json")?,
    )?;
    if !package.is_object() {
        bail!("package.json must be an object");
    }
    let context = ContextData { source, package };
    let manager = exclusive("Node package manager", &context, managers::MANAGERS)?;
    let framework = exclusive("Node test framework", &context, frameworks::DETECTORS)?;
    let locked = ["package-lock.json", "pnpm-lock.yaml", "yarn.lock"]
        .iter()
        .any(|p| context.source.text(p).is_some());
    Ok(Profile {
        package: context.package,
        manager,
        framework,
        locked,
    })
}
