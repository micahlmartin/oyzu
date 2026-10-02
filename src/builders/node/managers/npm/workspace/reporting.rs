//! Shared framework selection and per-package report obligations.
use crate::builders::{node::detection, ReportFormat, ReportSpec, TaskPlan};
use anyhow::{bail, Result};

pub(super) fn framework(profile: &detection::Profile) -> Result<String> {
    let name = profile.framework.selected();
    if let Some(script) = profile.package["scripts"]["test"].as_str() {
        if (name == "jest" && !crate::builders::node::jest::recognized(script))
            || (name == "vitest" && !crate::builders::node::vitest::recognized(script))
            || (name == "mocha" && !crate::builders::node::mocha::recognized(script))
        {
            return Ok("custom".into());
        }
    }
    if !["node-test", "jest", "vitest", "mocha", "custom"].contains(&name) {
        bail!("npm workspace {name} reporting integration is not implemented yet");
    }
    Ok(name.into())
}

pub(super) fn reports(task: &mut TaskPlan, module: &str, workspace_input: bool) {
    for (format, filename) in [
        (ReportFormat::Junit, "junit.xml"),
        (ReportFormat::Lcov, "coverage.lcov"),
    ] {
        task.reports.push(ReportSpec {
            format,
            filename,
            source: crate::reports::ReportSource::File,
            name: Some(module.into()),
            input: workspace_input.then(|| format!(".oyzu-build/reports/{module}/{filename}")),
        });
    }
}
