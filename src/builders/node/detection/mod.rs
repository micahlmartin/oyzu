//! Node observations; selection belongs to the shared role resolver.
mod frameworks;
mod managers;
mod outputs;
mod quality;

use crate::discovery::detectors::{exclusive, Source};
use crate::discovery::Resolution;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;

pub(super) struct Profile {
    pub package: Value,
    pub manager: Resolution,
    pub framework: Resolution,
    pub linter: Resolution,
    pub formatter: Resolution,
    pub output: Resolution,
    pub locked: bool,
}

struct ContextData {
    source: Source,
    package: Value,
    application_intent: bool,
}

pub(super) fn output_configuration_files() -> &'static [&'static str] {
    outputs::CONFIGS
}

pub(super) fn detect(root: &Path) -> Result<Profile> {
    detect_with_intent(root, false)
}

pub(super) fn detect_with_intent(root: &Path, application_intent: bool) -> Result<Profile> {
    let mut inputs = vec![
        "package.json",
        "package-lock.json",
        "npm-shrinkwrap.json",
        "pnpm-lock.yaml",
        "pnpm-workspace.yaml",
        "yarn.lock",
    ];
    inputs.extend(frameworks::inputs());
    inputs.extend(quality::inputs());
    inputs.extend(outputs::CONFIGS.iter().copied());
    let source = Source::read(root, &inputs)?;
    let package: Value = serde_json::from_str(
        source
            .text("package.json")
            .context("missing package.json")?,
    )?;
    if !package.is_object() {
        bail!("package.json must be an object");
    }
    let context = ContextData {
        source,
        package,
        application_intent,
    };
    let manager = exclusive("Node package manager", &context, managers::MANAGERS)?;
    let framework = exclusive("Node test framework", &context, frameworks::DETECTORS)?;
    let linter = exclusive("Node linter", &context, quality::LINTERS)?;
    let formatter = exclusive("Node formatter", &context, quality::FORMATTERS)?;
    let output = exclusive("Node output profile", &context, outputs::DETECTORS)?;
    let locked = [
        "npm-shrinkwrap.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
    ]
    .iter()
    .any(|p| context.source.text(p).is_some());
    Ok(Profile {
        package: context.package,
        manager,
        framework,
        linter,
        formatter,
        output,
        locked,
    })
}
