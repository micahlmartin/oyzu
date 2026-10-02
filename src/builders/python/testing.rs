//! Python owns native pytest invocation and report obligations. Host execution
//! uses the selected manager; scheduling and report retention remain shared.
use crate::{
    builders::{strings, ReportFormat, ReportSpec, TaskPlan},
    model::{Target, Task},
    reports::ReportSource,
};

pub(super) fn reports() -> Vec<ReportSpec> {
    [
        (ReportFormat::Junit, "junit.xml"),
        (ReportFormat::Cobertura, "coverage.xml"),
    ]
    .into_iter()
    .map(|(format, filename)| ReportSpec {
        format,
        filename,
        source: ReportSource::File,
        name: None,
        input: None,
    })
    .collect()
}

pub(super) fn development(target: &Target, task: &Task) -> Option<TaskPlan> {
    if task.name != "test" || target.discovery.get("test-framework")?.selected() != "pytest" {
        return None;
    }
    let default = match target.manager.as_str() {
        "pip" => "python -m pytest",
        "uv" => "uv run --locked pytest",
        "poetry" => "poetry run pytest",
        _ => return None,
    };
    let argv: Vec<_> = task.argv.iter().map(String::as_str).collect();
    let command = match argv.as_slice() {
        ["sh", "-c", command] => Some(*command),
        [shell, "-NoProfile", "-NonInteractive", "-Command", command]
            if matches!(
                *shell,
                "powershell.exe" | "powershell" | "pwsh.exe" | "pwsh"
            ) =>
        {
            Some(*command)
        }
        _ => None,
    };
    // Only exact shell shorthand is normalized. No shell expression is parsed.
    let normalized = command
        .filter(|c| *c == default || matches!(*c, "pytest" | "python -m pytest"))
        .map(|c| c.split(' ').collect::<Vec<_>>())
        .unwrap_or(argv);
    let prefix = match normalized.as_slice() {
        ["python", "-m", "pytest"] | ["pytest"] => Some(strings(&["python"])),
        ["uv", "run", "--locked", "pytest"] if target.manager == "uv" => {
            Some(strings(&["uv", "run", "--locked", "python"]))
        }
        ["poetry", "run", "pytest"] if target.manager == "poetry" => {
            Some(strings(&["poetry", "run", "--", "python"]))
        }
        _ => None,
    };
    let argv = if let Some(mut command) = prefix {
        command.extend(strings(&[
            "/oyzu/python-reporting.py",
            "--host",
            "--junit",
            &format!("/out/{}/reports/junit.xml", target.name),
            "--coverage",
            &format!("/out/{}/reports/coverage.xml", target.name),
            "--",
        ]));
        command
    } else {
        task.argv.clone()
    };
    Some(TaskPlan {
        argv,
        reports: reports(),
        ..TaskPlan::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builders::Builder, discovery};
    use std::fs;

    #[test]
    fn host_adaptation_preserves_manager_or_explicit_interpreter_and_report_obligations() {
        for (marker, prefix) in [
            ("requirements.txt", vec!["python"]),
            ("uv.lock", vec!["uv", "run", "--locked", "python"]),
            ("poetry.lock", vec!["poetry", "run", "--", "python"]),
        ] {
            let root = tempfile::tempdir().unwrap();
            fs::write(
                root.path().join("pyproject.toml"),
                "[project]\nname='demo'\nversion='1.0.0'\n",
            )
            .unwrap();
            fs::write(root.path().join(marker), "").unwrap();
            let workspace = discovery::discover(root.path()).unwrap();
            let target = &workspace.targets["project"];
            let mut task = target.tasks["test"].clone();
            let native = super::super::Python
                .development_test(target, &task)
                .unwrap()
                .unwrap();
            assert_eq!(&native.argv[..prefix.len()], &prefix);
            assert_eq!(native.reports.len(), 2);
            task.argv = strings(&["python", "-m", "pytest"]);
            assert_eq!(development(target, &task).unwrap().argv[0], "python");
            task.argv = strings(&["sh", "-c", "pytest && echo cleanup"]);
            assert_eq!(development(target, &task).unwrap().argv, task.argv);
            assert_eq!(development(target, &task).unwrap().reports.len(), 2);
            task.name = "lint".into();
            assert!(development(target, &task).is_none());
        }
    }
}
