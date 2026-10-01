use super::super::task::insert;
use crate::model::Target;
use anyhow::{bail, Result};
use serde_json::Value;
use std::fs;

pub(super) fn discover(target: &mut Target) -> Result<()> {
    let value: Value =
        serde_json::from_str(&fs::read_to_string(target.path.join("package.json"))?)?;
    let locks: Vec<_> = [
        ("package-lock.json", "npm"),
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
    ]
    .into_iter()
    .filter(|(file, _)| target.path.join(file).is_file())
    .collect();
    if locks.len() > 1 {
        bail!("{}: conflicting Node lockfiles", target.name);
    }
    let declared = value
        .get("packageManager")
        .and_then(Value::as_str)
        .and_then(|v| v.split('@').next());
    if let (Some(requested), Some((_, locked))) = (declared, locks.first()) {
        if requested != *locked {
            bail!("{}: packageManager disagrees with lockfile", target.name);
        }
    }
    let manager = declared
        .or_else(|| locks.first().map(|(_, m)| *m))
        .unwrap_or("npm")
        .to_string();
    if !["npm", "pnpm", "yarn"].contains(&manager.as_str()) {
        bail!("unsupported Node manager {manager}");
    }
    target.manager = manager.clone();
    target.version = value
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("0.0.0")
        .into();
    let args: Vec<&str> = match manager.as_str() {
        "npm" if !locks.is_empty() => vec!["npm", "ci"],
        "npm" => vec!["npm", "install"],
        "pnpm" => vec!["pnpm", "install", "--frozen-lockfile"],
        _ => vec!["yarn", "install", "--frozen-lockfile"],
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
