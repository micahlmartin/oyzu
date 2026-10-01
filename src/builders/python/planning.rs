use super::super::{
    strings, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext, ReportFormat, ReportSpec,
    TaskPlan,
};
use anyhow::{bail, Context, Result};
use std::{collections::BTreeMap, fs};

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let target = context.target;
    let id = &target.name;
    if context.dependencies.is_none() {
        bail!("{id}: missing prepared Python dependency snapshot");
    }
    let project =
        toml::from_str::<toml::Value>(&fs::read_to_string(target.path.join("pyproject.toml"))?)?;
    let name = project
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .context("Python project requires a static name")?;
    if !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        bail!("invalid Python distribution name");
    }
    let name = name.replace(['-', '.'], "_").to_lowercase();
    let version = format!(
        "{}.dev0+g{}",
        target.version.split('+').next().unwrap_or("0.0.0"),
        &context.source.digest[7..19]
    );
    let mut plan = BuilderPlan::new(
        version.clone(),
        CommandSpec::new("package", &["python", "-I", "/oyzu/python.py", "package"]),
    );
    plan.env.extend(BTreeMap::from([
        ("PIP_NO_INDEX".into(), "1".into()),
        ("PIP_CONFIG_FILE".into(), "/dev/null".into()),
        ("PIP_DISABLE_PIP_VERSION_CHECK".into(), "1".into()),
        ("SOURCE_DATE_EPOCH".into(), "0".into()),
        ("OYZU_TARGET".into(), id.clone()),
        ("OYZU_PYTHON_MANAGER".into(), target.manager.clone()),
        ("UV_CACHE_DIR".into(), "/tmp/uv-cache".into()),
        ("UV_PROJECT_ENVIRONMENT".into(), ".oyzu-build/venv".into()),
    ]));
    plan.prepare.push(CommandSpec::new(
        "prepare",
        &["python", "-I", "/oyzu/python.py", "prepare"],
    ));
    plan.tasks.insert(
        "build".into(),
        TaskPlan::command(&["python", "-I", "/oyzu/python.py", "build"]),
    );
    let mut test = TaskPlan::command(&[
        ".oyzu-build/venv/bin/python",
        "-I",
        "-m",
        "pytest",
        "--import-mode=importlib",
        &format!("--junitxml=/out/{id}/reports/junit.xml"),
        "--cov",
        &format!("--cov-report=xml:/out/{id}/reports/coverage.xml"),
    ]);
    if let Some(packages) = project
        .get("tool")
        .and_then(|p| p.get("setuptools"))
        .and_then(|p| p.get("packages"))
        .and_then(|p| p.as_array())
    {
        test.argv.retain(|v| v != "--cov");
        for package in packages.iter().filter_map(|p| p.as_str()) {
            test.argv.push(format!("--cov={package}"));
        }
    }
    if target.manager == "uv" {
        let mut wrapper = strings(&[
            "uv",
            "run",
            "--offline",
            "--no-sync",
            "--no-python-downloads",
            "--no-managed-python",
            "--python",
            ".oyzu-build/venv/bin/python",
            "--",
        ]);
        wrapper.extend(test.argv);
        test.argv = wrapper;
    }
    test.reports = vec![
        ReportSpec {
            format: ReportFormat::Junit,
            filename: "junit.xml",
            source: crate::reports::ReportSource::File,
        },
        ReportSpec {
            format: ReportFormat::Cobertura,
            filename: "coverage.xml",
            source: crate::reports::ReportSource::File,
        },
    ];
    plan.tasks.insert("test".into(), test);
    for task in target.tasks.values() {
        if task.argv.first().is_some_and(|v| v == "ruff") {
            let mut argv = task.argv.clone();
            argv[0] = ".oyzu-build/venv/bin/ruff".into();
            plan.tasks.insert(
                task.name.clone(),
                TaskPlan {
                    argv,
                    ..TaskPlan::default()
                },
            );
        }
    }
    plan.artifacts = vec![
        ArtifactSpec {
            name: "wheel",
            filename: format!("{name}-{version}-py3-none-any.whl"),
            media_type: "application/zip",
        },
        ArtifactSpec {
            name: "sdist",
            filename: format!("{name}-{version}.tar.gz"),
            media_type: "application/gzip",
        },
    ];
    Ok(plan)
}
