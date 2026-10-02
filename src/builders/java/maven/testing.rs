//! Native reactor observation and direct test report obligations.
use super::{metadata, RUNTIME};
use crate::{
    builders::{ReportFormat, ReportSpec, TaskPlan},
    model::{Target, Task},
    reports::ReportSource,
};
use anyhow::{bail, Context, Result};
use std::{fs, path::Path, process::Command};

pub(super) fn has_tests(project: &metadata::Project, root: &Path) -> bool {
    project.test_roots.iter().any(|path| {
        walkdir::WalkDir::new(root.join(path))
            .into_iter()
            .flatten()
            .any(|entry| entry.file_type().is_file())
    })
}

pub(super) fn development(target: &Target, task: &Task) -> Result<Option<TaskPlan>> {
    if task.name != "test" {
        return Ok(None);
    }
    let native = if target.path.join("mvnw").is_file() {
        if cfg!(windows) {
            "mvnw.cmd"
        } else {
            "./mvnw"
        }
    } else {
        "mvn"
    };
    let temporary = tempfile::Builder::new()
        .prefix("oyzu-maven-host-")
        .tempdir()?;
    for file in RUNTIME {
        fs::write(temporary.path().join(file.name), file.contents)?;
    }
    let path = task
        .env
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| std::ffi::OsStr::new(value));
    let metadata_file = temporary.path().join("metadata.xml");
    let result = Command::new(crate::launch::program("python", path))
        .arg("-I")
        .arg(temporary.path().join("maven-host.py"))
        .args(["describe", native])
        .arg(&metadata_file)
        .current_dir(&target.path)
        .envs(&task.env)
        .output()
        .context(
            "Maven direct tests require provisioned Maven, JDK, Python and native dependencies",
        )?;
    if !result.status.success() {
        bail!(
            "Maven test metadata failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let projects = metadata::read(&metadata_file)?;
    let mut plan = TaskPlan::default();
    let mut modules = Vec::new();
    for project in projects
        .iter()
        .filter(|project| has_tests(project, &target.path))
    {
        if project.test_reports.is_empty() {
            bail!("Maven test sources have no native test lifecycle reports");
        }
        let id = crate::names::scoped("module", &format!("{}.{}", project.group, project.artifact));
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
        modules.push(serde_json::json!({"id":id,"reports":project.test_reports,"directory":project.directory}));
    }
    if modules.is_empty() {
        bail!("Maven reactor has no discovered native test sources");
    }
    if task.argv == [native, "-B", "test"] {
        let encoded = serde_json::to_string(&modules)?;
        if encoded.len() > 20_000 {
            bail!("Maven host report plan exceeds argument limit");
        }
        plan.argv = vec![
            "python".into(),
            "-I".into(),
            "/oyzu/maven-host.py".into(),
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
