//! Native workspace expansion at explicit task execution, never static discovery.
use crate::model::Task;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{collections::BTreeSet, process::Command};

#[derive(Deserialize)]
struct Workspace {
    #[serde(rename = "Use", default)]
    members: Vec<Member>,
}

#[derive(Deserialize)]
struct Member {
    #[serde(rename = "DiskPath")]
    path: String,
}

pub(super) fn command(task: &Task) -> Result<Option<super::super::DevelopmentCommand>> {
    if task.provider != "go"
        || !task.cwd.join("go.work").is_file()
        || !matches!(task.argv.as_slice(), [go, op, pattern] if go == "go" && matches!(op.as_str(), "build" | "test" | "vet") && pattern == "./...")
    {
        return Ok(None);
    }
    let path = task
        .env
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| std::ffi::OsStr::new(value));
    let output = Command::new(crate::launch::program("go", path))
        .args(["work", "edit", "-json"])
        .current_dir(&task.cwd)
        .envs(&task.env)
        .env("GOWORK", task.cwd.join("go.work"))
        .env("GOTOOLCHAIN", "local")
        .output()
        .context("Go workspace tasks require an explicitly provisioned native Go toolchain")?;
    if !output.status.success() {
        bail!(
            "native Go workspace discovery failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let workspace: Workspace = serde_json::from_slice(&output.stdout)?;
    let root = task.cwd.canonicalize()?;
    let mut patterns = BTreeSet::new();
    for member in workspace.members {
        let path = task.cwd.join(&member.path).canonicalize()?;
        let relative = path
            .strip_prefix(&root)
            .context("Go workspace member outside task root")?;
        let relative = relative
            .to_str()
            .context("non-UTF8 Go workspace path")?
            .replace('\\', "/");
        patterns.insert(if relative.is_empty() {
            "./...".into()
        } else {
            format!("./{relative}/...")
        });
    }
    if patterns.is_empty() {
        bail!("Go workspace has no modules");
    }
    let mut argv = task.argv[..2].to_vec();
    argv.extend(patterns);
    // Let Go derive the workspace from its own cwd spelling. Go 1.24 otherwise
    // treats Windows verbatim prefixes / symlink aliases as different module
    // roots when an inherited absolute GOWORK uses another spelling. We already
    // verified that this exact task root owns a go.work, so no ancestor is used.
    let env = std::collections::BTreeMap::from([("GOWORK".into(), "auto".into())]);
    Ok(Some(super::super::DevelopmentCommand { argv, env }))
}
