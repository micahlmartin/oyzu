//! Shared native chart validation/report obligations for captured and host tests.
use crate::{
    builders::{CoverageApplicability, ReportFormat, ReportSpec, TaskPlan},
    model::{Target, Task},
    reports::ReportSource,
};
use anyhow::Result;

pub(super) fn coverage() -> CoverageApplicability {
    CoverageApplicability::Inapplicable {
        reason: "Chart validation and assertions have no application-source coverage denominator."
            .into(),
    }
}

pub(super) fn plan(id: &str, chart: &str, kind: &str, unittest: bool, rendered: &str) -> TaskPlan {
    let mut plan = TaskPlan::command(&[
        "python",
        "-I",
        "/oyzu/helm-test.py",
        chart,
        kind,
        &format!("/out/{id}/reports/junit.xml"),
        rendered,
    ]);
    plan.coverage = Some(coverage());
    plan.reports.push(ReportSpec {
        format: ReportFormat::Junit,
        filename: "junit.xml",
        source: ReportSource::File,
        name: None,
        input: None,
    });
    if unittest {
        plan.argv.extend([
            "--unittest-report".into(),
            format!("/out/{id}/reports/unittest/unittest.xml"),
        ]);
        plan.reports.push(ReportSpec {
            format: ReportFormat::Junit,
            filename: "unittest.xml",
            source: ReportSource::File,
            name: Some("unittest".into()),
            input: None,
        });
    }
    plan
}

pub(super) fn development(target: &Target, task: &Task) -> Result<Option<TaskPlan>> {
    if task.name != "test" {
        return Ok(None);
    }
    let chart = super::metadata::chart_path(&target.path)?;
    let metadata = super::metadata::read(&target.path.join(chart))?;
    let kind = if metadata.kind == "library" {
        "library"
    } else {
        "application"
    };
    let unittest = target.discovery["test-framework"].selected() == "helm-unittest";
    let expected = if unittest {
        vec!["helm", "unittest", "--strict", chart]
    } else if kind == "library" {
        vec!["helm", "lint", chart, "--strict", "--with-subcharts"]
    } else {
        vec!["helm", "template", "oyzu-check", chart]
    };
    let mut test = plan(
        &target.name,
        chart,
        kind,
        unittest,
        &format!("/out/{}/reports/rendered.yaml", target.name),
    );
    if task.argv.iter().map(String::as_str).collect::<Vec<_>>() == expected {
        test.argv.push("--host".into());
    } else {
        // Replacements retain their native semantics and must supply the same
        // reports through shared bindings; do not interpret arbitrary shell.
        test.argv = task.argv.clone();
    }
    Ok(Some(test))
}
