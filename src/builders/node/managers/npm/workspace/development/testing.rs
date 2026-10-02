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
    let root = detection::detect(&target.path)?;
    let root_scripts = root.package["scripts"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let root_test = root_scripts.contains_key("test");
    let mut plan = crate::builders::TaskPlan::default();
    let mut modules = Vec::new();
    for member in &metadata.members {
        let profile = detection::detect(&target.path.join(&member.path))?;
        let id = crate::names::scoped("package", &member.name);
        if !root_test {
            super::super::reporting::reports(&mut plan, &id, false);
        }
        modules.push(
            serde_json::json!({"id":id,"name":member.name,"path":member.path,
            "scripts":member.scripts,"framework":super::super::reporting::framework(&profile)?,
            "testExcludes":exclusions(&metadata, &member.path)}),
        );
    }
    let root_artifact = (root.package["private"] != true).then(|| serde_json::json!({"path":"."}));
    if root_test || root_artifact.is_some() {
        super::super::reporting::reports(&mut plan, "root", false);
    }
    let expected = if root_test {
        vec!["npm", "run", "test"]
    } else {
        vec!["npm", "run", "test", "--workspaces"]
    };
    if task.argv == expected {
        let specification = serde_json::json!({"rootScripts":root_scripts,
            "rootFramework":super::super::reporting::framework(&root)?,
            "rootArtifact":root_artifact,"modules":modules,
            "nodeTestArguments":crate::builders::node::reporting::arguments("__OYZU_TEST_REPORT__", "__OYZU_COVERAGE_REPORT__")});
        let encoded = serde_json::to_string(&specification)?;
        if encoded.len() > 20_000 {
            bail!("npm workspace development plan exceeds host argument limit");
        }
        plan.argv = vec![
            "node".into(),
            "/oyzu/npm-workspace-test-host.mjs".into(),
            encoded,
            format!("/out/{}/reports", target.name),
        ];
    } else {
        // A task replacement still owns its body and must supply the report
        // obligations (or explicitly declared collection paths).
        plan.argv = task.argv.clone();
    }
    Ok(Some(plan))
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
