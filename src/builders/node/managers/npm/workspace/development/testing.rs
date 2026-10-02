//! Select native member tests; scripts retain npm lifecycle and argument semantics.
use super::super::{scope::exclusions, Metadata};
use super::{Operation, Step};
use crate::{builders::node::detection, model::Task};
use anyhow::{bail, Result};

/// Observe installed membership only after shared task admission, then freeze
/// package scopes and reporting obligations before any native test is run.
pub(in crate::builders::node::managers::npm) fn report_plan(
    target: &crate::model::Target,
    task: &Task,
) -> Result<Option<crate::builders::TaskPlan>> {
    if task.name != "test" {
        return Ok(None);
    }
    let mut observation = task.clone();
    observation.cwd = target.path.clone();
    let (_, metadata) = super::describe(&observation)?;
    let members = metadata
        .members
        .iter()
        .map(|m| crate::builders::node::workspace::Member {
            name: m.name.clone(),
            path: m.path.clone(),
        })
        .collect::<Vec<_>>();
    let root = detection::detect(&target.path)?;
    let expected: Vec<String> = if root.package["scripts"]["test"].is_string() {
        vec!["npm", "run", "test"]
    } else {
        vec!["npm", "run", "test", "--workspaces"]
    }
    .into_iter()
    .map(str::to_owned)
    .collect();
    crate::builders::node::workspace::report_plan(
        target,
        task,
        &members,
        &expected,
        "npm-workspace-test-host.mjs",
        serde_json::Value::Null,
    )
    .map(Some)
}

pub(super) fn steps(task: &Task, metadata: &Metadata) -> Result<Vec<Step>> {
    let root = detection::detect(&task.cwd)?;
    let mut packages: Vec<_> = metadata
        .members
        .iter()
        .map(|member| (member.name.as_str(), member.path.as_str()))
        .collect();
    if root.package["private"] != true {
        packages.push(("root", "."));
    }
    let mut result = Vec::new();
    for (name, path) in packages {
        let profile = detection::detect(&task.cwd.join(path))?;
        let operation = if profile.package["scripts"]["test"].is_string() {
            Operation::Script {
                script: "test".into(),
            }
        } else {
            let framework = profile.framework.selected();
            if !["node-test", "jest", "vitest", "mocha"].contains(&framework) {
                bail!("{path}: implicit {framework} workspace test integration is not implemented yet");
            }
            Operation::Test {
                framework: framework.into(),
                excludes: exclusions(metadata, path),
            }
        };
        result.push(Step {
            name: name.into(),
            path: path.into(),
            operation,
        });
    }
    Ok(result)
}
