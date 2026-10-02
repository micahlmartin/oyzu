//! Native Go test commands and shared report obligations. Unknown replacements
//! must emit report files; arbitrary stdout is never treated as Go test events.
use crate::{
    builders::{strings, ReportFormat, ReportSpec, TaskPlan},
    model::{Target, Task},
    reports::ReportSource,
};

pub(super) fn reports(source: ReportSource) -> Vec<ReportSpec> {
    vec![
        ReportSpec {
            format: ReportFormat::Junit,
            filename: "junit.xml",
            source,
            name: None,
            input: None,
        },
        ReportSpec {
            format: ReportFormat::GoCover,
            filename: "coverage.out",
            source: ReportSource::File,
            name: None,
            input: None,
        },
    ]
}

pub(super) fn development(target: &Target, task: &Task) -> Option<TaskPlan> {
    if task.name != "test" {
        return None;
    }
    let argv: Vec<_> = task.argv.iter().map(String::as_str).collect();
    let native = match argv.as_slice() {
        ["go", "test", "./..."] | ["sh", "-c", "go test ./..."] => true,
        [shell, "-NoProfile", "-NonInteractive", "-Command", "go test ./..."] => matches!(
            *shell,
            "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe"
        ),
        _ => false,
    };
    Some(TaskPlan {
        argv: if native {
            strings(&[
                "go",
                "test",
                "-json",
                &format!("-coverprofile=/out/{}/reports/coverage.out", target.name),
                "-count=1",
                "./...",
            ])
        } else {
            task.argv.clone()
        },
        reports: reports(if native {
            ReportSource::GoTestEvents
        } else {
            ReportSource::File
        }),
        ..TaskPlan::default()
    })
}
