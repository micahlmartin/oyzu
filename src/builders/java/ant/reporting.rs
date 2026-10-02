//! Exact native Ant targets retain their lifecycle; arbitrary shell is not parsed.
use crate::{
    builders::{strings, ReportFormat, ReportSpec, TaskPlan},
    model::Task,
    reports::ReportSource,
};
use std::collections::BTreeMap;

fn command(target: &str, test: &str, coverage: &str, version: &str) -> Vec<String> {
    strings(&["sh", "/oyzu/ant-test.sh", test, coverage, version, target])
}

pub(super) fn test(id: &str, version: &str) -> TaskPlan {
    TaskPlan {
        argv: command(
            "test",
            &format!("/out/{id}/reports/junit.xml"),
            &format!("/out/{id}/reports/jacoco.xml"),
            version,
        ),
        reports: vec![
            ReportSpec {
                format: ReportFormat::Junit,
                filename: "junit.xml",
                source: ReportSource::File,
                name: None,
                input: None,
            },
            ReportSpec {
                format: ReportFormat::Jacoco,
                filename: "jacoco.xml",
                source: ReportSource::File,
                name: None,
                input: None,
            },
        ],
        ..Default::default()
    }
}

pub(super) fn instrument(task: &Task, env: &BTreeMap<String, String>) -> Option<Vec<String>> {
    if task.name != "test" {
        return None;
    }
    let argv: Vec<_> = task.argv.iter().map(String::as_str).collect();
    let target = match argv.as_slice() {
        ["ant", target] => *target,
        ["sh", "-c", body] => body.strip_prefix("ant ")?,
        _ => return None,
    };
    // One literal native target, never shell expansion, options or a compound command.
    if target.is_empty()
        || target.starts_with('-')
        || !target
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return None;
    }
    Some(command(
        target,
        env.get("OYZU_TEST_REPORT")?,
        env.get("OYZU_COVERAGE_REPORT")?,
        env.get("OYZU_VERSION")?,
    ))
}
