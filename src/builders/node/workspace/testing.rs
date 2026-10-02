//! Freeze package reports after native observation and before task execution.
use super::{reporting, Member};
use crate::{
    builders::{node::detection, TaskPlan},
    model::{Target, Task},
};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{fs, process::Command};

pub(in crate::builders::node) fn report_plan(
    target: &Target,
    task: &Task,
    members: &[Member],
    expected: &[String],
    host: &str,
    native: Value,
) -> Result<TaskPlan> {
    super::validate(members)?;
    let root = detection::detect(&target.path)?;
    let root_scripts = root.package["scripts"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let root_test = root_scripts.contains_key("test");
    let mut plan = TaskPlan::default();
    let mut modules = Vec::new();
    for member in members {
        let profile = detection::detect(&target.path.join(&member.path))?;
        let id = crate::names::scoped("package", &member.name);
        if !root_test {
            reporting::reports(&mut plan, &id, false);
        }
        modules.push(json!({"id":id,"name":member.name,"path":member.path,
            "scripts":profile.package["scripts"].as_object().cloned().unwrap_or_default(),
            "framework":reporting::framework(&profile)?,
            "testExcludes":super::exclusions(members.iter().map(|m|m.path.as_str()), &member.path)}));
    }
    let root_artifact = (root.package["private"] != true).then(|| json!({"path":"."}));
    if root_test || root_artifact.is_some() {
        reporting::reports(&mut plan, "root", false);
    }
    if task.argv == expected {
        let specification = json!({"rootScripts":root_scripts,
            "rootFramework":reporting::framework(&root)?,"rootArtifact":root_artifact,
            "modules":modules,"native":native,
            "nodeTestArguments":crate::builders::node::reporting::arguments("__OYZU_TEST_REPORT__", "__OYZU_COVERAGE_REPORT__")});
        let encoded = serde_json::to_string(&specification)?;
        if encoded.len() > 20_000 {
            bail!("workspace development plan exceeds host argument limit");
        }
        plan.argv = vec![
            "node".into(),
            format!("/oyzu/{host}"),
            encoded,
            format!("/out/{}/reports", target.name),
        ];
    } else {
        plan.argv = task.argv.clone();
    }
    Ok(plan)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Native {
    members: Vec<Member>,
    command: Vec<String>,
    directory: String,
    separator: Vec<String>,
}

/// The manager selects its native observer and expected implicit argv. The
/// shared layer only stages runtime assets and admits the returned report scope.
pub(in crate::builders::node) fn native_plan(
    target: &Target,
    task: &Task,
    observer: &str,
    expected: &[String],
) -> Result<Option<TaskPlan>> {
    if task.name != "test" {
        return Ok(None);
    }
    let temporary = tempfile::Builder::new()
        .prefix("oyzu-workspace-model-")
        .tempdir()?;
    for file in crate::builders::node::RUNTIME {
        fs::write(temporary.path().join(file.name), file.contents)?;
    }
    let path = task
        .env
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
        .map(|(_, v)| std::ffi::OsStr::new(v));
    let result = Command::new(crate::launch::program("node", path))
        .arg(temporary.path().join(observer))
        .current_dir(&target.path)
        .envs(&task.env)
        .output()
        .context("workspace tests require provisioned native tools and installed dependencies")?;
    if !result.status.success() {
        bail!(
            "native workspace observation failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    if result.stdout.len() > 4 * 1024 * 1024 {
        bail!("native workspace metadata exceeds limit");
    }
    let native: Native = serde_json::from_slice(&result.stdout)?;
    if native.command.len() < 2 || native.command.iter().any(String::is_empty) {
        bail!("invalid native manager command");
    }
    let command =
        json!({"command":native.command,"directory":native.directory,"separator":native.separator});
    report_plan(
        target,
        task,
        &native.members,
        expected,
        "workspace-test-host.mjs",
        command,
    )
    .map(Some)
}
