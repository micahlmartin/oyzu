//! Requirements-only applications retain application identity, never wheel metadata.
use crate::builders::{
    semver_snapshot, ArtifactKind, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext,
    TaskPlan,
};
use anyhow::{bail, Result};
use std::path::Path;

pub(super) fn matches(root: &Path) -> bool {
    root.join("requirements.txt").is_file()
        && !root.join("pyproject.toml").exists()
        && !root.join("setup.py").exists()
        && !root.join("setup.cfg").exists()
}

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let id = &context.target.name;
    if context.dependencies.is_none() || context.target.manager != "pip" {
        bail!("requirements applications require captured pip dependencies");
    }
    if !context.target.path.join("__main__.py").is_file()
        && !context.target.path.join("app.py").is_file()
    {
        bail!("requirements application needs an unambiguous app.py or __main__.py entrypoint; custom entrypoint integration is pending");
    }
    let version = semver_snapshot(context.target, context.source);
    let filename = format!("{id}-{version}.pyz");
    let mut plan = BuilderPlan::new(
        version,
        CommandSpec::new(
            "package",
            &["python", "-I", "/oyzu/python-app.py", "package"],
        ),
    );
    plan.env.insert("PIP_NO_INDEX".into(), "1".into());
    plan.env
        .insert("PIP_CONFIG_FILE".into(), "/dev/null".into());
    plan.env.insert("OYZU_TARGET".into(), id.clone());
    plan.env
        .insert("OYZU_SOURCE_DIGEST".into(), context.source.digest.clone());
    for name in ["OYZU_VERSION", "OYZU_TARGET", "OYZU_SOURCE_DIGEST"] {
        plan.fixed_env.insert(name.into(), plan.env[name].clone());
    }
    plan.prepare.push(CommandSpec::new(
        "prepare",
        &["python", "-I", "/oyzu/python-app.py", "prepare"],
    ));
    plan.tasks.insert(
        "build".into(),
        TaskPlan::command(&["python", "-I", "/oyzu/python-app.py", "build"]),
    );
    let mut test = TaskPlan::command(&[
        ".oyzu-build/venv/bin/python",
        "-I",
        "/oyzu/python-app.py",
        "test",
        &format!("/out/{id}/reports/junit.xml"),
        &format!("/out/{id}/reports/coverage.xml"),
    ]);
    test.reports = super::testing::reports();
    plan.tasks.insert("test".into(), test);
    super::quality::plan(context.target, &mut plan);
    plan.artifacts.push(ArtifactSpec {
        kind: ArtifactKind::File,
        name: "application".into(),
        filename,
        media_type: "application/zip",
        version: None,
    });
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builders::Builder, dependencies::Prepared, discovery, snapshot};
    use serde_json::json;
    use std::fs;

    #[test]
    fn requirements_application_has_a_snapshot_archive_without_distribution_metadata() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("requirements.txt"), "packaging==24.2\n").unwrap();
        fs::write(root.path().join("app.py"), "print('hello')\n").unwrap();
        let capture = tempfile::tempdir().unwrap();
        let source = snapshot::capture(root.path(), &capture.path().join("source")).unwrap();
        let workspace = discovery::discover(&capture.path().join("source")).unwrap();
        let target = &workspace.targets["project"];
        assert_eq!(target.builder, "python/app");
        assert!(target.tasks["build"].build_stage);
        let dependencies = Prepared {
            root: capture.path().join("dependencies"),
            digest: format!("sha256:{}", "1".repeat(64)),
            record: json!({}),
        };
        let context = || PlanningContext {
            target,
            source: &source,
            dependencies: Some(&dependencies),
        };
        let plan = super::super::Python.plan(context()).unwrap();
        plan.validate().unwrap();
        assert_eq!(plan.artifacts.len(), 1);
        assert_eq!(plan.artifacts[0].name, "application");
        assert_eq!(
            plan.artifacts[0].filename,
            format!("project-{}.pyz", plan.version)
        );
        assert!(plan.version.starts_with("0.0.0-dev.g"));
        assert_eq!(plan.tasks["test"].reports.len(), 2);
        assert_eq!(plan.tasks["test"].reports[0].format.name(), "junit");
        assert_eq!(plan.tasks["test"].reports[1].format.name(), "cobertura");
        assert_eq!(plan.fixed_env["OYZU_SOURCE_DIGEST"], source.digest);
        assert_eq!(plan.fixed_env["OYZU_VERSION"], plan.version);
        assert!(!target.path.join("pyproject.toml").exists());
        fs::remove_file(target.path.join("app.py")).unwrap();
        assert!(super::super::Python
            .plan(context())
            .err()
            .unwrap()
            .to_string()
            .contains("entrypoint"));
        fs::write(target.path.join("__main__.py"), "print('hello')\n").unwrap();
        assert!(super::super::Python.plan(context()).is_ok());
        // Distribution metadata takes precedence over application inference.
        fs::write(target.path.join("setup.cfg"), "[metadata]\nname=demo\n").unwrap();
        assert!(!matches(&target.path));
    }
}
