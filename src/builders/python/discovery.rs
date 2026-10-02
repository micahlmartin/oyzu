use super::super::task::insert;
use crate::model::Target;
use anyhow::{bail, Result};

pub(super) fn discover(target: &mut Target) -> Result<()> {
    let profile = super::detection::detect(&target.path)?;
    let value = profile.project;
    target.manager = profile.manager.selected().to_string();
    target
        .discovery
        .insert("package-manager".into(), profile.manager);
    target.discovery.insert("linter".into(), profile.linter);
    target
        .discovery
        .insert("formatter".into(), profile.formatter);
    if let Some(version) = value
        .get("project")
        .and_then(|v| v.get("version"))
        .and_then(|v| v.as_str())
    {
        target.version = version.into();
    }
    if target.manager == "uv" {
        insert(target, "install", &["uv", "sync", "--locked"], false);
        insert(target, "build", &["uv", "build"], true);
        insert(target, "test", &["uv", "run", "--locked", "pytest"], true);
    } else if target.manager == "poetry" {
        insert(target, "install", &["poetry", "install"], false);
        insert(target, "build", &["poetry", "build"], true);
        insert(target, "test", &["poetry", "run", "pytest"], true);
    } else if target.manager == "pip" {
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
        } else if super::application::matches(&target.path) {
            insert(
                target,
                "build",
                &["python", "-m", "compileall", "-q", "."],
                true,
            );
        }
        insert(target, "test", &["python", "-m", "pytest"], true);
    } else {
        bail!(
            "no task adapter for detected Python manager {}",
            target.manager
        );
    }
    let has_tests = target.path.join("tests").is_dir() || target.path.join("test").is_dir();
    if !has_tests {
        if let Some(test) = target.tasks.get_mut("test") {
            test.availability = Some("No conventional test directory detected".into());
            test.build_stage = false;
        }
    }
    super::quality::discover(target);
    Ok(())
}
