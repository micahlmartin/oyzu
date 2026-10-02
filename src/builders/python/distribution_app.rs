//! Application intent on top of the native distribution build, without a second build.
use crate::builders::{ArtifactKind, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext};
use anyhow::{bail, Context, Result};

pub(super) fn extend(
    context: &PlanningContext<'_>,
    plan: &mut BuilderPlan,
    name: &str,
) -> Result<()> {
    if context.target.builder != "python/app" {
        return Ok(());
    }
    let project: toml::Value = toml::from_str(
        &std::fs::read_to_string(context.target.path.join("pyproject.toml")).context(
            "distribution applications require static pyproject.toml console entrypoints",
        )?,
    )?;
    let scripts = project
        .get("project")
        .and_then(|p| p.get("scripts"))
        .and_then(toml::Value::as_table)
        .context("python/app requires one declared [project.scripts] console entrypoint")?;
    if scripts.len() != 1 {
        bail!("python/app requires one unambiguous [project.scripts] console entrypoint; entrypoint selection overrides are not implemented yet");
    }
    let (script, value) = scripts.iter().next().unwrap();
    let value = value
        .as_str()
        .context("console entrypoint must be a string")?;
    let id = &context.target.name;
    for (key, value) in [
        ("OYZU_PYTHON_DISTRIBUTION", name.to_string()),
        ("OYZU_PYTHON_ENTRYPOINT_NAME", script.clone()),
        ("OYZU_PYTHON_ENTRYPOINT_VALUE", value.to_string()),
        ("OYZU_SOURCE_DIGEST", context.source.digest.clone()),
    ] {
        plan.env.insert(key.into(), value.clone());
        plan.fixed_env.insert(key.into(), value);
    }
    for key in ["OYZU_VERSION", "OYZU_TARGET"] {
        plan.fixed_env.insert(key.into(), plan.env[key].clone());
    }
    plan.tasks.get_mut("build").unwrap().argv =
        super::super::strings(&["python", "-I", "/oyzu/python-distribution-app.py", "build"]);
    // Keep the already selected native manager wrapper and report declarations.
    let argv = &mut plan.tasks.get_mut("test").unwrap().argv;
    let index = argv
        .iter()
        .position(|arg| arg == "/oyzu/python-reporting.py")
        .context("missing Python report adapter in application test plan")?;
    argv.truncate(index);
    argv.extend(super::super::strings(&[
        "/oyzu/python-app.py",
        "test",
        &format!("/out/{id}/reports/junit.xml"),
        &format!("/out/{id}/reports/coverage.xml"),
    ]));
    plan.package = CommandSpec::new(
        "package",
        &[
            "python",
            "-I",
            "/oyzu/python-distribution-app.py",
            "package",
        ],
    );
    plan.artifacts.push(ArtifactSpec {
        kind: ArtifactKind::File,
        name: "application".into(),
        version: None,
        filename: format!("{id}-{}.pyz", plan.version),
        media_type: "application/zip",
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builders::Builder, dependencies::Prepared, discovery, snapshot};
    use serde_json::json;
    use std::fs;

    #[test]
    fn console_application_extends_native_plan_and_rejects_ambiguous_selection() {
        let root = tempfile::tempdir().unwrap();
        let metadata =
            "[project]\nname='demo'\nversion='1.0.0'\n[project.scripts]\ndemo='demo:main'\n";
        fs::write(root.path().join("pyproject.toml"), metadata).unwrap();
        fs::write(root.path().join("build.yaml"), "api:\n  uses: python/app\n").unwrap();
        let captured = tempfile::tempdir().unwrap();
        let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
        let workspace = discovery::discover(&captured.path().join("source")).unwrap();
        let mut target = workspace.targets["api"].clone();
        let dependencies = Prepared {
            root: captured.path().join("dependencies"),
            digest: format!("sha256:{}", "1".repeat(64)),
            record: json!({}),
        };
        for manager in ["pip", "uv", "poetry"] {
            target.manager = manager.into();
            let plan = crate::builders::python::Python
                .plan(PlanningContext {
                    target: &target,
                    source: &source,
                    dependencies: Some(&dependencies),
                })
                .unwrap();
            plan.validate().unwrap();
            assert_eq!(plan.artifacts.len(), 3);
            assert!(plan
                .artifacts
                .iter()
                .any(|a| a.name == "application"
                    && a.filename == format!("api-{}.pyz", plan.version)));
            assert_eq!(plan.fixed_env["OYZU_PYTHON_ENTRYPOINT_VALUE"], "demo:main");
            assert_eq!(plan.fixed_env["OYZU_SOURCE_DIGEST"], source.digest);
            assert_eq!(plan.tasks["test"].reports.len(), 2);
            assert!(plan.tasks["test"]
                .argv
                .iter()
                .any(|a| a == "/oyzu/python-app.py"));
            assert_eq!(plan.tasks["test"].argv[0] == "uv", manager == "uv");
        }
        fs::write(
            target.path.join("pyproject.toml"),
            format!("{metadata}other='demo:other'\n"),
        )
        .unwrap();
        assert!(crate::builders::python::Python
            .plan(PlanningContext {
                target: &target,
                source: &source,
                dependencies: Some(&dependencies)
            })
            .err()
            .unwrap()
            .to_string()
            .contains("unambiguous"));
        target.builder = "python/package".into();
        let plan = crate::builders::python::Python
            .plan(PlanningContext {
                target: &target,
                source: &source,
                dependencies: Some(&dependencies),
            })
            .unwrap();
        assert_eq!(
            plan.artifacts.len(),
            2,
            "package builders must not acquire application semantics"
        );
    }
}
