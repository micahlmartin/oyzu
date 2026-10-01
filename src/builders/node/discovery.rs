use super::super::task::insert;
use crate::model::Target;
use anyhow::{bail, Result};
use serde_json::Value;

pub(super) fn discover(target: &mut Target) -> Result<()> {
    let profile = super::detection::detect(&target.path)?;
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
    Ok(())
}
