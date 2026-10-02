//! Native Cargo test selection and shared captured/host report obligations.
use super::metadata::Metadata;
use crate::{
    builders::{ReportFormat, ReportSpec, TaskPlan},
    model::{Target, Task},
    reports::ReportSource,
};
use anyhow::{bail, Context, Result};
use std::{fs, process::Command};

pub(super) fn plan(id: &str, metadata: &Metadata) -> TaskPlan {
    let packages: Vec<_> = metadata
        .packages
        .iter()
        .filter(|p| {
            metadata.workspace_members.contains(&p.id) && p.targets.iter().any(|t| t.doctest)
        })
        .map(|p| p.name.clone())
        .collect();
    let doctest = if packages.is_empty() {
        String::new()
    } else {
        format!("/out/{id}/reports/doctest/doctest.xml")
    };
    let mut plan = TaskPlan::command(&[
        "python3",
        "-I",
        "/oyzu/rust-test.py",
        "captured",
        "/dependencies/nextest.toml",
        &format!("/out/{id}/reports/junit.xml"),
        &format!("/out/{id}/reports/coverage.xml"),
        &doctest,
        &serde_json::to_string(&packages).expect("string list is serializable"),
    ]);
    for (format, filename, name) in [
        (ReportFormat::Junit, "junit.xml", None),
        (ReportFormat::Cobertura, "coverage.xml", None),
        (ReportFormat::Junit, "doctest.xml", Some("doctest")),
    ] {
        if name.is_some() && packages.is_empty() {
            continue;
        }
        plan.reports.push(ReportSpec {
            format,
            filename,
            source: ReportSource::File,
            name: name.map(String::from),
            input: None,
        });
    }
    plan
}

pub(super) fn development(target: &Target, task: &Task) -> Result<Option<TaskPlan>> {
    if task.name != "test" {
        return Ok(None);
    }
    let path = task
        .env
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| std::ffi::OsStr::new(value));
    let result = Command::new(crate::launch::program("cargo", path))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--locked",
            "--offline",
        ])
        .current_dir(&target.path)
        .envs(&task.env)
        .output()
        .context("Rust direct tests require provisioned Cargo and installed dependencies")?;
    if !result.status.success() {
        bail!(
            "Cargo test metadata failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let metadata: Metadata = serde_json::from_slice(&result.stdout)?;
    let mut plan = plan(&target.name, &metadata);
    if task.argv == ["cargo", "test", "--locked", "--workspace"] {
        let config_path = target.path.join(".config/nextest.toml");
        let config = if config_path.is_file() {
            fs::read_to_string(config_path)?
        } else {
            String::new()
        };
        let _: toml::Value = toml::from_str(&config)?;
        plan.argv[0] = "python".into();
        plan.argv[3] = "host".into();
        plan.argv[4] = config;
    } else {
        plan.argv = task.argv.clone();
    }
    Ok(Some(plan))
}
