//! Quality defaults use owned native integration; project scripts remain authoritative.
use crate::{
    builders::{task::insert, BuilderPlan, DevelopmentCommand, TaskPlan},
    model::{Target, Task},
};
use anyhow::{bail, Result};

const RUNTIME: &str = include_str!("runtime/quality.mjs");

pub(super) fn discover(target: &mut Target) {
    if !target.tasks.contains_key("lint") {
        let linter = target.discovery["linter"].selected().to_string();
        insert(target, "lint", &[&linter, "."], true);
    }
    if !target.tasks.contains_key("format-check") && !target.tasks.contains_key("format:check") {
        let formatter = target.discovery["formatter"].selected().to_string();
        insert(target, "format-check", &[&formatter, "--check", "."], true);
    }
    if !target.tasks.contains_key("format") {
        let formatter = target.discovery["formatter"].selected().to_string();
        insert(target, "format", &[&formatter, "--write", "."], false);
        target.tasks.get_mut("format").unwrap().mutates_source = true;
    }
    let framework = target.discovery["test-framework"].selected().to_string();
    for task in target.tasks.values_mut().filter(|t| mode(t).is_some()) {
        task.env
            .insert("OYZU_NODE_TEST_FRAMEWORK".into(), framework.clone());
    }
}

fn mode(task: &Task) -> Option<&'static str> {
    let argv: Vec<_> = task.argv.iter().map(String::as_str).collect();
    match argv.as_slice() {
        ["eslint", "."] => Some("lint"),
        ["prettier", "--check", "."] => Some("format-check"),
        ["prettier", "--write", "."] => Some("format"),
        _ => None,
    }
}

pub(super) fn plan(target: &Target, plan: &mut BuilderPlan) -> Result<()> {
    for task in target.tasks.values() {
        if let Some(mode) = mode(task) {
            plan.tasks
                .entry(task.name.clone())
                .or_insert_with(|| TaskPlan::command(&["node", "/oyzu/node-quality.mjs", mode]));
        } else if task.build_stage && task.argv.first().is_some_and(|v| v == "biome") {
            bail!("Biome native quality integration is not implemented yet; provide an explicit native task script");
        }
    }
    plan.env.insert(
        "OYZU_NODE_QUALITY_HOME".into(),
        "/opt/oyzu-node-quality".into(),
    );
    plan.fixed_env.insert(
        "OYZU_NODE_QUALITY_HOME".into(),
        "/opt/oyzu-node-quality".into(),
    );
    plan.env
        .insert("OYZU_NODE_QUALITY_EXCLUDE".into(), "[]".into());
    plan.fixed_env
        .insert("OYZU_NODE_QUALITY_EXCLUDE".into(), "[]".into());
    Ok(())
}

pub(super) fn development(task: &Task) -> Option<DevelopmentCommand> {
    if !["npm", "pnpm", "yarn"].contains(&task.provider.as_str()) {
        return None;
    }
    let mode = mode(task)?;
    Some(DevelopmentCommand {
        argv: vec![
            "node".into(),
            "--input-type=module".into(),
            "-e".into(),
            RUNTIME.into(),
            mode.into(),
        ],
        env: Default::default(),
    })
}
