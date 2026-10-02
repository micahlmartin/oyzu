use super::super::{
    strings, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext, ReportFormat, ReportSpec,
    TaskPlan,
};
use anyhow::{bail, Context, Result};
use std::{collections::BTreeMap, fs};

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    if super::application::matches(&context.target.path) {
        return super::application::plan(context);
    }
    let target = context.target;
    let id = &target.name;
    if context.dependencies.is_none() {
        bail!("{id}: missing prepared Python dependency snapshot");
    }
    let legacy = if super::legacy::matches(&target.path) {
        Some(super::legacy::Metadata::read(
            context.dependencies.unwrap().record["extensions"]["oyzu.dev/python-legacy"].clone(),
        )?)
    } else {
        None
    };
    let (name, version, wheel_tag) = if let Some(metadata) = &legacy {
        if !metadata
            .version
            .ends_with(&format!(".dev0+g{}", &context.source.digest[7..19]))
        {
            bail!("legacy Python version does not bind captured source");
        }
        (
            metadata.name.clone(),
            metadata.version.clone(),
            metadata.wheel_tag.clone(),
        )
    } else {
        let project = toml::from_str::<toml::Value>(&fs::read_to_string(
            target.path.join("pyproject.toml"),
        )?)?;
        let name = project
            .get("project")
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .context("Python project requires a static name")?;
        (
            name.to_string(),
            format!(
                "{}.dev0+g{}",
                target.version.split('+').next().unwrap_or("0.0.0"),
                &context.source.digest[7..19]
            ),
            "py3-none-any".into(),
        )
    };
    if !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    {
        bail!("invalid Python distribution name");
    }
    let name = name.replace(['-', '.'], "_").to_lowercase();
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
    if let Some(metadata) = &legacy {
        plan.env
            .insert("OYZU_SOURCE_DIGEST".into(), context.source.digest.clone());
        plan.env.insert("PYTHONHASHSEED".into(), "0".into());
        for name in [
            "OYZU_VERSION",
            "OYZU_SOURCE_DIGEST",
            "PYTHONHASHSEED",
            "SOURCE_DATE_EPOCH",
        ] {
            plan.fixed_env.insert(name.into(), plan.env[name].clone());
        }
        if let Some(compiler) = &metadata.compiler {
            let command = compiler.command.join(" ");
            plan.env.insert("CC".into(), command.clone());
            plan.fixed_env.insert("CC".into(), command);
        }
    }
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
        "/oyzu/python-reporting.py",
        "--distribution",
        &name,
        "--junit",
        &format!("/out/{id}/reports/junit.xml"),
        "--coverage",
        &format!("/out/{id}/reports/coverage.xml"),
    ]);
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
            name: None,
            input: None,
        },
        ReportSpec {
            format: ReportFormat::Cobertura,
            filename: "coverage.xml",
            source: crate::reports::ReportSource::File,
            name: None,
            input: None,
        },
    ];
    plan.tasks.insert("test".into(), test);
    super::quality::plan(target, &mut plan);
    plan.artifacts = vec![
        ArtifactSpec {
            kind: crate::builders::ArtifactKind::File,
            name: "wheel".into(),
            version: None,
            filename: format!("{name}-{version}-{wheel_tag}.whl"),
            media_type: "application/zip",
        },
        ArtifactSpec {
            kind: crate::builders::ArtifactKind::File,
            name: "sdist".into(),
            version: None,
            filename: format!("{name}-{version}.tar.gz"),
            media_type: "application/gzip",
        },
    ];
    Ok(plan)
}
