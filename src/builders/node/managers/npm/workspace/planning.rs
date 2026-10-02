//! Compile captured workspace facts into native operations and per-module outputs.
use super::{Member, Metadata};
use crate::builders::node::{detection, managers::Manager};
use crate::builders::{
    ArtifactKind, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext, ReportFormat,
    ReportSpec, TaskPlan,
};
use anyhow::{bail, Context, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

fn ordered(metadata: &Metadata) -> Result<Vec<&Member>> {
    let mut pending: BTreeMap<_, _> = metadata
        .members
        .iter()
        .map(|m| (m.name.as_str(), m))
        .collect();
    let mut done = BTreeSet::new();
    let mut output = Vec::new();
    while !pending.is_empty() {
        let name = pending
            .iter()
            .find(|(_, m)| {
                m.dependencies
                    .iter()
                    .all(|e| done.contains(e.target.as_str()))
            })
            .map(|(name, _)| *name)
            .context("npm workspace dependency cycle requires cycle-aware build integration")?;
        output.push(pending.remove(name).unwrap());
        done.insert(name);
    }
    Ok(output)
}

fn framework(profile: &detection::Profile) -> Result<String> {
    let name = profile.framework.selected();
    if let Some(script) = profile.package["scripts"]["test"].as_str() {
        if (name == "jest" && !crate::builders::node::jest::recognized(script))
            || (name == "vitest" && !crate::builders::node::vitest::recognized(script))
        {
            return Ok("custom".into());
        }
    }
    if !["node-test", "jest", "vitest", "custom"].contains(&name) {
        bail!("npm workspace {name} reporting integration is not implemented yet");
    }
    Ok(name.into())
}

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let prepared = context
        .dependencies
        .context("npm workspace requires captured dependency and module metadata")?;
    let metadata =
        Metadata::read(prepared.record["extensions"]["oyzu.dev/npm"]["workspaces"].clone())?
            .context("missing captured npm workspace model")?;
    let package = crate::records::read(&context.target.path.join("package.json"))?;
    if package["private"] != true {
        bail!("npm workspace root artifact ownership requires a private aggregation root in this profile");
    }
    let root_version = crate::builders::semver_snapshot(context.target, context.source);
    let mut plan = BuilderPlan::new(
        root_version.clone(),
        CommandSpec::new(
            "package",
            &["node", "/oyzu/npm-workspace-build.mjs", "package"],
        ),
    );
    super::super::Npm.configure(&context, &mut plan)?;
    let mut modules = Vec::new();
    let root_scripts = package["scripts"].as_object().cloned().unwrap_or_default();
    let root_test = root_scripts.contains_key("test");
    let mut test = TaskPlan::command(&["node", "/oyzu/npm-workspace-build.mjs", "test"]);
    for member in ordered(&metadata)? {
        let profile = detection::detect(&context.target.path.join(&member.path))?;
        let id = crate::names::scoped("package", &member.name);
        let version = format!(
            "{}-dev.g{}",
            member.version.split(['-', '+']).next().unwrap(),
            &context.source.digest[7..19]
        );
        let filename = format!(
            "{}-{version}.tgz",
            member.name.trim_start_matches('@').replace('/', "-")
        );
        modules.push(json!({"id":id,"name":member.name,"path":member.path,"version":version,"filename":filename,"scripts":member.scripts,"dependencies":member.dependencies,"framework":framework(&profile)?,"quality":{"linter":profile.linter.selected(),"formatter":profile.formatter.selected()}}));
        plan.artifacts.push(ArtifactSpec {
            kind: ArtifactKind::File,
            name: id.clone(),
            filename,
            media_type: "application/gzip",
            version: Some(version),
        });
        if !root_test {
            reports(&mut test, &id);
        }
    }
    if root_test {
        reports(&mut test, "root");
    }
    let root_profile = detection::detect(&context.target.path)?;
    let specification = json!({"rootVersion":root_version,"rootScripts":root_scripts,"rootDependencies":metadata.root_dependencies,"rootFramework":framework(&root_profile)?,"rootQuality":{"linter":root_profile.linter.selected(),"formatter":root_profile.formatter.selected()},"modules":modules,
        "nodeTestArguments":crate::builders::node::reporting::arguments("__OYZU_TEST_REPORT__", "__OYZU_COVERAGE_REPORT__")});
    let encoded = serde_json::to_string(&specification)?;
    if encoded.len() > 120_000 {
        bail!("npm workspace execution specification exceeds native argument limit");
    }
    let digest = format!("{:x}", Sha256::digest(encoded.as_bytes()));
    for (key, value) in [
        ("OYZU_NPM_WORKSPACE_PLAN", digest),
        ("OYZU_TARGET", context.target.name.clone()),
    ] {
        plan.env.insert(key.into(), value.clone());
        plan.fixed_env.insert(key.into(), value);
    }
    let mut project = CommandSpec::new(
        "version",
        &["node", "/oyzu/npm-workspace-build.mjs", "project"],
    );
    project.argv.push(encoded);
    plan.prepare.insert(0, project);
    plan.tasks.insert(
        "build".into(),
        TaskPlan::command(&["node", "/oyzu/npm-workspace-build.mjs", "build"]),
    );
    plan.tasks.insert("test".into(), test);
    // One formatting stage owns the intent, regardless of the member's native
    // spelling. An explicit root script owns the whole workspace once.
    let format_stage = if !root_scripts.contains_key("format-check")
        && root_scripts.contains_key("format:check")
    {
        "format:check"
    } else {
        "format-check"
    };
    plan.stages
        .retain(|s| !["format-check", "format:check"].contains(s) || *s == format_stage);
    for stage in ["lint", format_stage] {
        plan.tasks.insert(
            stage.into(),
            TaskPlan::command(&["node", "/oyzu/npm-workspace-build.mjs", stage]),
        );
    }
    crate::builders::node::quality::plan(context.target, &mut plan)?;
    Ok(plan)
}

fn reports(task: &mut TaskPlan, module: &str) {
    for (format, filename) in [
        (ReportFormat::Junit, "junit.xml"),
        (ReportFormat::Lcov, "coverage.lcov"),
    ] {
        task.reports.push(ReportSpec {
            format,
            filename,
            source: crate::reports::ReportSource::File,
            name: Some(module.into()),
            input: Some(format!(".oyzu-build/reports/{module}/{filename}")),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_dependencies_order_producers_and_reject_cycles() {
        let mut value = json!({"schemaVersion":1,"members":[
            {"name":"app","path":"app","version":"1.0.0","private":true,"scripts":{},"dependencies":[{"name":"shared","target":"shared","kind":"prod","spec":"1.0.0"}]},
            {"name":"shared","path":"shared","version":"1.0.0","private":false,"scripts":{},"dependencies":[]}
        ]});
        let metadata = Metadata::read(value.clone()).unwrap().unwrap();
        assert_eq!(
            ordered(&metadata)
                .unwrap()
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>(),
            ["shared", "app"]
        );
        value["members"][1]["dependencies"] =
            json!([{"name":"app","target":"app","kind":"dev","spec":"1.0.0"}]);
        assert!(ordered(&Metadata::read(value).unwrap().unwrap()).is_err());
    }
}
