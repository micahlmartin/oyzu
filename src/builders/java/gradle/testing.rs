//! Native composite observation and direct test report obligations.
use super::{metadata, RUNTIME};
use crate::{
    builders::{ReportFormat, ReportSpec, TaskPlan},
    model::{Target, Task},
    reports::ReportSource,
};
use anyhow::{bail, Context, Result};
use std::{fs, process::Command};

pub(super) fn report_identity(directory: &str, test: &metadata::Test) -> Result<Option<String>> {
    if !test.sources_known {
        bail!(
            "Gradle test {} requires native source attribution",
            test.task
        );
    }
    if test.sources.is_empty() {
        return Ok(None);
    }
    if !test.junit_enabled {
        bail!("Gradle test {} disabled required JUnit evidence", test.task);
    }
    test.coverage
        .as_ref()
        .context("missing native Gradle coverage destination")?;
    Ok(Some(crate::names::scoped(
        "test",
        &format!("{}:{}", directory, test.task),
    )))
}

pub(super) fn development(target: &Target, task: &Task) -> Result<Option<TaskPlan>> {
    if task.name != "test" {
        return Ok(None);
    }
    let native = if target.path.join("gradlew").is_file() {
        if cfg!(windows) {
            "gradlew.bat"
        } else {
            "./gradlew"
        }
    } else {
        "gradle"
    };
    let temporary = tempfile::Builder::new()
        .prefix("oyzu-gradle-host-")
        .tempdir()?;
    for file in RUNTIME {
        fs::write(temporary.path().join(file.name), file.contents)?;
    }
    let path = task
        .env
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| std::ffi::OsStr::new(value));
    let metadata = temporary.path().join("metadata");
    let result = Command::new(crate::launch::program("python", path))
        .arg("-I")
        .arg(temporary.path().join("gradle-host.py"))
        .args(["describe", native])
        .arg(&metadata)
        .current_dir(&target.path)
        .envs(&task.env)
        .output()
        .context("Gradle direct tests require provisioned Gradle, JDK, Python and dependencies")?;
    if !result.status.success() {
        bail!(
            "Gradle test metadata failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let builds = metadata::read(&metadata)?;
    let mut plan = TaskPlan::default();
    let mut modules = Vec::new();
    for model in &builds {
        for project in &model.projects {
            for test in &project.tests {
                let Some(id) = report_identity(&model.directory, test)? else {
                    continue;
                };
                let coverage = test
                    .coverage
                    .as_ref()
                    .context("missing native Gradle coverage destination")?;
                for (format, filename) in [
                    (ReportFormat::Junit, "junit.xml"),
                    (ReportFormat::Jacoco, "jacoco.xml"),
                ] {
                    plan.reports.push(ReportSpec {
                        format,
                        filename,
                        source: ReportSource::File,
                        name: Some(id.clone()),
                        input: None,
                    });
                }
                modules.push(serde_json::json!({"id":id,"junit":test.junit,"coverage":coverage,"build":model.directory,"task":test.task}));
            }
        }
    }
    if modules.is_empty() {
        bail!("Gradle builds have no discovered native test sources");
    }
    if task.argv == [native, "--no-daemon", "test"] {
        let encoded = serde_json::to_string(&modules)?;
        if encoded.len() > 20_000 {
            bail!("Gradle host report plan exceeds argument limit");
        }
        plan.argv = vec![
            "python".into(),
            "-I".into(),
            "/oyzu/gradle-host.py".into(),
            "test".into(),
            native.into(),
            encoded,
            format!("/out/{}/reports", target.name),
        ];
    } else {
        plan.argv = task.argv.clone();
    }
    Ok(Some(plan))
}
