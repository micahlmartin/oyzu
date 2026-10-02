use super::super::task::insert;
use crate::model::Target;
use anyhow::{bail, Result};
use serde_json::Value;

pub(super) fn discover(target: &mut Target) -> Result<()> {
    let profile = super::detection::detect(&target.path)?;
    let framework = profile.framework.selected().to_string();
    target.discovery.insert("linter".into(), profile.linter);
    target
        .discovery
        .insert("formatter".into(), profile.formatter);
    target
        .discovery
        .insert("test-framework".into(), profile.framework);
    let value = profile.package;
    let manager = profile.manager.selected().to_string();
    target.manager = manager.clone();
    target
        .discovery
        .insert("package-manager".into(), profile.manager);
    target.version = value
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("0.0.0")
        .into();
    let args: Vec<&str> = match manager.as_str() {
        "npm" if profile.locked => vec!["npm", "ci"],
        "npm" => vec!["npm", "install"],
        "pnpm" => vec!["pnpm", "install", "--frozen-lockfile"],
        "yarn" => vec!["yarn", "install", "--frozen-lockfile"],
        _ => bail!("no task adapter for detected Node manager {manager}"),
    };
    insert(target, "install", &args, false);
    if let Some(scripts) = value.get("scripts").and_then(Value::as_object) {
        for (name, command) in scripts {
            if !command.is_string() {
                bail!("Node script {name} is not a string");
            }
            let stage = matches!(
                name.as_str(),
                "build" | "test" | "lint" | "format:check" | "format-check"
            );
            insert(target, name, &[&manager, "run", name], stage);
            target.tasks.get_mut(name).unwrap().mutates_source = name == "format";
        }
    }
    if !target.tasks.contains_key("test") {
        if framework == "node-test" {
            insert(target, "test", &["node", "--test"], true);
        } else if framework == "jest" {
            insert(target, "test", super::jest::DEFAULT, true);
        } else if framework == "vitest" {
            insert(target, "test", super::vitest::DEFAULT, true);
        } else {
            super::super::unavailable(target, "test", &format!("Detected {framework}; its implicit runner/report integration is not implemented yet"));
        }
    }
    super::quality::discover(target);
    if manager == "npm" && value.get("workspaces").is_some() {
        if !target.tasks.contains_key("build") {
            insert(
                target,
                "build",
                &["npm", "run", "build", "--workspaces"],
                true,
            );
        }
        super::super::unavailable(
            target,
            "format:check",
            "Captured npm workspace operation; direct development integration remains pending",
        );
    }
    Ok(())
}
