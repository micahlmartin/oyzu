use super::super::task::insert;
use crate::model::Target;
use anyhow::{bail, Result};
use serde_json::Value;

pub(super) fn discover(target: &mut Target) -> Result<()> {
    let profile = super::detection::detect_with_intent(
        &target.path,
        target.builder == "node/app"
            && target.builder_selection == crate::model::BuilderSelection::Explicit,
    )?;
    let framework = profile.framework.selected().to_string();
    target.discovery.insert("linter".into(), profile.linter);
    target
        .discovery
        .insert("output-profile".into(), profile.output);
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
        } else if framework == "mocha" {
            insert(target, "test", super::mocha::DEFAULT, true);
        } else {
            super::super::unavailable(target, "test", &format!("Detected {framework}; its implicit runner/report integration is not implemented yet"));
        }
    }
    super::quality::discover(target);
    if target.builder == "node/app" {
        let output = match target.discovery["output-profile"].selected() {
            "vite-application" => super::application::output_directory(&value).ok(),
            "dist-application" => Some("dist"),
            _ => None,
        };
        if let Some(output) = output {
            let exclusions = serde_json::to_string(&[output])?;
            for name in ["lint", "format-check", "format"] {
                if let Some(task) = target.tasks.get_mut(name) {
                    task.env
                        .insert("OYZU_NODE_QUALITY_EXCLUDE".into(), exclusions.clone());
                }
            }
        }
    }
    let native = super::managers::get(&manager)?;
    if native.is_workspace(&target.path, &value) && value["scripts"].get("build").is_none() {
        if let Some(argv) = native.workspace_build_command() {
            insert(
                target,
                "build",
                &argv.iter().map(String::as_str).collect::<Vec<_>>(),
                true,
            );
        }
    }
    if native.is_workspace(&target.path, &value) && value["scripts"].get("test").is_none() {
        let argv = native.workspace_test_command();
        insert(
            target,
            "test",
            &argv.iter().map(String::as_str).collect::<Vec<_>>(),
            true,
        );
    }
    if manager == "npm" && value.get("workspaces").is_some() {
        for operation in ["build", "test", "lint", "format-check", "format"] {
            let scripts = &value["scripts"];
            if scripts.get(operation).is_some()
                || (operation == "format-check" && scripts.get("format:check").is_some())
            {
                continue;
            }
            insert(
                target,
                operation,
                &["npm", "run", operation, "--workspaces"],
                operation != "format",
            );
            target.tasks.get_mut(operation).unwrap().mutates_source = operation == "format";
        }
    }
    Ok(())
}
