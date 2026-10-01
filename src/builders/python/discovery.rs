use super::super::task::insert;
use crate::model::Target;
use anyhow::{bail, Result};
use std::fs;

pub(super) fn discover(target: &mut Target) -> Result<()> {
    let file = target.path.join("pyproject.toml");
    let value: toml::Value = if file.is_file() {
        toml::from_str(&fs::read_to_string(file)?)?
    } else {
        toml::Value::Table(Default::default())
    };
    let uv = target.path.join("uv.lock").is_file()
        || value.get("tool").and_then(|v| v.get("uv")).is_some();
    let poetry = target.path.join("poetry.lock").is_file()
        || value.get("tool").and_then(|v| v.get("poetry")).is_some();
    if uv && poetry {
        bail!("{}: conflicting Python managers", target.name);
    }
    target.manager = if uv {
        "uv"
    } else if poetry {
        "poetry"
    } else {
        "pip"
    }
    .into();
    if let Some(version) = value
        .get("project")
        .and_then(|v| v.get("version"))
        .and_then(|v| v.as_str())
    {
        target.version = version.into();
    }
    if uv {
        insert(target, "install", &["uv", "sync", "--locked"], false);
        insert(target, "build", &["uv", "build"], true);
        insert(target, "test", &["uv", "run", "--locked", "pytest"], true);
    } else if poetry {
        insert(target, "install", &["poetry", "install"], false);
        insert(target, "build", &["poetry", "build"], true);
        insert(target, "test", &["poetry", "run", "pytest"], true);
    } else {
        if target.path.join("requirements.txt").is_file() {
            insert(
                target,
                "install",
                &["python", "-m", "pip", "install", "-r", "requirements.txt"],
                false,
            );
        }
        if target.path.join("pyproject.toml").is_file() || target.path.join("setup.py").is_file() {
            insert(target, "build", &["python", "-m", "build"], true);
        }
        insert(target, "test", &["python", "-m", "pytest"], true);
    }
    let has_tests = target.path.join("tests").is_dir() || target.path.join("test").is_dir();
    if !has_tests {
        if let Some(test) = target.tasks.get_mut("test") {
            test.availability = Some("No conventional test directory detected".into());
            test.build_stage = false;
        }
    }
    if value.get("tool").and_then(|v| v.get("ruff")).is_some() {
        insert(target, "lint", &["ruff", "check", "."], true);
        insert(
            target,
            "format-check",
            &["ruff", "format", "--check", "."],
            true,
        );
        insert(target, "format", &["ruff", "format", "."], false);
        target.tasks.get_mut("format").unwrap().mutates_source = true;
    }
    Ok(())
}
