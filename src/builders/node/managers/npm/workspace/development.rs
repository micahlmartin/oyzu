//! Resolve installed native workspace facts only for an explicit development run.
use super::{planning::ordered, Metadata};
use crate::{builders::DevelopmentCommand, model::Task};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::{fs, process::Command};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Native {
    command: Vec<String>,
    workspaces: serde_json::Value,
}

pub(in crate::builders::node::managers::npm) fn command(
    task: &Task,
) -> Result<Option<DevelopmentCommand>> {
    if task.provider != "npm" || task.argv != ["npm", "run", "build", "--workspaces"] {
        return Ok(None);
    }
    let temporary = tempfile::Builder::new()
        .prefix("oyzu-npm-development-")
        .tempdir()?;
    for (name, contents) in [
        (
            "npm-native.mjs",
            include_str!("../../../runtime/npm-native.mjs"),
        ),
        (
            "npm-workspaces.mjs",
            include_str!("../../../runtime/npm-workspaces.mjs"),
        ),
        (
            "describe.mjs",
            include_str!("../../../runtime/npm-workspace-describe.mjs"),
        ),
    ] {
        fs::write(temporary.path().join(name), contents)?;
    }
    let path = task
        .env
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
        .map(|(_, v)| std::ffi::OsStr::new(v));
    let result = Command::new(crate::launch::program("node", path))
        .arg(temporary.path().join("describe.mjs"))
        .current_dir(&task.cwd).envs(&task.env).output()
        .context("npm workspace development tasks require provisioned Node/npm and installed project dependencies")?;
    if !result.status.success() {
        bail!(
            "native npm workspace discovery failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    if result.stdout.len() > 4 * 1024 * 1024 {
        bail!("native npm workspace metadata exceeds limit");
    }
    let native: Native = serde_json::from_slice(&result.stdout)?;
    if native.command.len() != 2 || native.command.iter().any(String::is_empty) {
        bail!("invalid native npm entrypoint");
    }
    let metadata = Metadata::read(native.workspaces)?.context("no native npm workspace members")?;
    let steps: Vec<_> = ordered(&metadata)?
        .into_iter()
        .map(|m| json!({"name":m.name, "build":m.scripts.contains_key("build")}))
        .collect();
    let spec = serde_json::to_string(&json!({"command":native.command,"steps":steps}))?;
    if spec.len() > 20_000 {
        bail!("npm workspace development plan exceeds host argument limit");
    }
    // npm's workspace-wide run follows declaration order. Separate native
    // invocations preserve dependency order and each package's pre/post hooks.
    let script = format!(
        "const plan={spec};\n{}",
        include_str!("../../../runtime/npm-workspace-development.cjs")
    );
    Ok(Some(DevelopmentCommand {
        argv: vec![native.command[0].clone(), "-e".into(), script, "--".into()],
        env: Default::default(),
    }))
}
