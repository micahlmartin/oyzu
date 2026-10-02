//! Resolve installed native workspace facts only for an explicit development run.
use super::{planning::ordered, Metadata};
use crate::{builders::DevelopmentCommand, model::Task};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, process::Command};
mod quality;
mod testing;

#[derive(Serialize)]
struct Step {
    name: String,
    path: String,
    operation: Operation,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum Operation {
    Script {
        script: String,
    },
    NoCompilation,
    Test {
        framework: String,
        excludes: Vec<String>,
    },
    Quality {
        framework: String,
        excludes: Vec<String>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Native {
    command: Vec<String>,
    workspaces: serde_json::Value,
}

pub(in crate::builders::node::managers::npm) fn command(
    task: &Task,
) -> Result<Option<DevelopmentCommand>> {
    let argv: Vec<_> = task.argv.iter().map(String::as_str).collect();
    let ["npm", "run", stage, "--workspaces"] = argv.as_slice() else {
        return Ok(None);
    };
    if task.provider != "npm"
        || !["build", "test", "lint", "format-check", "format"].contains(stage)
    {
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
    let steps: Vec<_> = if *stage == "build" {
        ordered(&metadata)?
            .into_iter()
            .map(|m| Step {
                name: m.name.clone(),
                path: m.path.clone(),
                operation: if m.scripts.contains_key("build") {
                    Operation::Script {
                        script: "build".into(),
                    }
                } else {
                    Operation::NoCompilation
                },
            })
            .collect()
    } else if *stage == "test" {
        testing::steps(task, &metadata)?
    } else {
        quality::steps(task, &metadata, stage)?
    };
    let quality = if steps
        .iter()
        .any(|step| matches!(step.operation, Operation::Quality { .. }))
    {
        Some(include_str!("../../../runtime/quality.mjs"))
    } else {
        None
    };
    let test_scope = (*stage == "test").then_some(include_str!(
        "../../../runtime/npm-workspace-test-scope.mjs"
    ));
    let spec = serde_json::to_string(
        &json!({"command":native.command,"steps":steps,"stage":stage,"quality":quality,"testScope":test_scope}),
    )?;
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
