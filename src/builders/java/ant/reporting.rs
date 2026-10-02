//! Exact native Ant targets retain their lifecycle; arbitrary shell is not parsed.
use crate::{
    builders::{strings, ReportFormat, ReportSpec, TaskPlan},
    model::{Target, Task},
    reports::ReportSource,
};
use std::collections::BTreeMap;

fn command(target: &str, test: &str, coverage: &str, version: &str) -> Vec<String> {
    strings(&[
        "python3",
        "-I",
        "/oyzu/ant-test.py",
        "captured",
        test,
        coverage,
        version,
        target,
    ])
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
    let target = native_target(task)?;
    Some(command(
        target,
        env.get("OYZU_TEST_REPORT")?,
        env.get("OYZU_COVERAGE_REPORT")?,
        env.get("OYZU_VERSION")?,
    ))
}

fn native_target(task: &Task) -> Option<&str> {
    let target = match task.argv.as_slice() {
        [program, target] if program == "ant" => target.as_str(),
        [shell, flag, body] if shell == "sh" && flag == "-c" => body.strip_prefix("ant ")?,
        [shell, profile, interactive, flag, body]
            if matches!(
                shell.as_str(),
                "powershell.exe" | "powershell" | "pwsh.exe" | "pwsh"
            ) && profile == "-NoProfile"
                && interactive == "-NonInteractive"
                && flag == "-Command" =>
        {
            body.strip_prefix("ant ")?
        }
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
    Some(target)
}

pub(super) fn development(target: &Target, task: &Task) -> Option<TaskPlan> {
    if task.name != "test" {
        return None;
    }
    let mut plan = test(&target.name, "");
    if let Some(native) = native_target(task) {
        plan.argv[0] = "python".into();
        plan.argv[3] = "host".into();
        *plan.argv.last_mut().unwrap() = native.into();
    } else {
        plan.argv = task.argv.clone();
    }
    Some(plan)
}
