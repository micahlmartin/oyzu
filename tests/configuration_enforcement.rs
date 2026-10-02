use oyzu::{
    config::{
        constraints::Constraints,
        policy::Policy,
        registry::{Registry, Scope},
        resolve::{resolve, EffectiveConfig, Selection},
        sources::ConfigSource,
    },
    discovery, tasks,
};
use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path};

fn administrative(settings: serde_json::Value) -> EffectiveConfig {
    let registry = Registry::default();
    let mut constraints = Constraints::default();
    let policy = Policy::parse(
        &serde_json::to_vec(&json!({
            "schemaVersion":1,"kind":"local-admin-policy","settings":settings,
            "profiles":{},"requiredCapabilities":[]
        }))
        .unwrap(),
    )
    .unwrap();
    let source = policy
        .source(
            "admin",
            Path::new("."),
            Scope::Admin,
            &registry,
            &mut constraints,
        )
        .unwrap();
    resolve(
        &[source],
        &registry,
        false,
        &Selection::default(),
        constraints,
        false,
    )
    .unwrap()
}

#[test]
fn task_environment_cannot_shadow_a_locked_global_value() {
    let registry = Registry::default();
    let admin = administrative(json!({"env.REVIEW_MODE":{"locked":true,"value":"approved"}}));
    for (value, accepted) in [("approved", true), ("unapproved", false)] {
        let source = ConfigSource::parse("project", Path::new("."), Scope::Project, false,
            &format!("[env]\nREVIEW_MODE='approved'\n[tasks.probe]\nargv=['unused']\n[tasks.probe.env]\nREVIEW_MODE='{value}'\n"), &registry).unwrap();
        let result = resolve(
            &[source],
            &registry,
            false,
            &Selection::default(),
            admin.constraints.clone(),
            false,
        );
        assert_eq!(result.is_ok(), accepted);
        if let Err(error) = result {
            assert!(error.to_string().contains("CONFIG_OVERRIDE_DENIED"));
        }
    }
}

#[test]
fn development_admission_checks_tools_routes_and_final_environment_before_launch() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"review","version":"1.0.0","scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    let mut workspace = discovery::discover(root.path()).unwrap();
    // An invalid executable makes accidental launch distinguishable from denial.
    workspace.tasks.get_mut("project:test").unwrap().argv =
        vec!["oyzu-review-must-not-launch".into()];
    for settings in [
        json!({"tools.allowed":{"locked":true,"value":[]}}),
        json!({"tools.node":{"default":"999.0.0"}}),
        json!({"registries.routes":{"locked":true,"value":[{"protocol":"npm","scope":"*","connectorId":"approved"}]}}),
        json!({"tools.catalogs":{"locked":true,"value":["approved"]}}),
    ] {
        workspace
            .configuration
            .insert("project".into(), administrative(settings));
        let error = tasks::run(&workspace, "project:test", &[]).unwrap_err();
        assert!(error.to_string().starts_with("CONFIG_"), "{error}");
    }
    workspace.configuration.insert(
        "project".into(),
        administrative(json!({"env.REVIEW_MODE":{"locked":true,"value":"approved"}})),
    );
    workspace
        .tasks
        .get_mut("project:test")
        .unwrap()
        .env
        .insert("REVIEW_MODE".into(), "unapproved".into());
    assert!(tasks::run(&workspace, "project:test", &[])
        .unwrap_err()
        .to_string()
        .contains("CONFIG_OVERRIDE_DENIED"));
    workspace.tasks.get_mut("project:test").unwrap().env.clear();
    let mut managed = administrative(json!({}));
    managed.management = Some(json!({"revision":"test"}));
    workspace.configuration.insert("project".into(), managed);
    assert!(tasks::run(&workspace, "project:test", &[])
        .unwrap_err()
        .to_string()
        .contains("managed host tasks"));
}

#[test]
fn nested_removal_exposes_native_task_in_lookup_and_build_plan() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("app")).unwrap();
    fs::write(
        root.path().join("build.yaml"),
        "app:\n  uses: node/app\n  path: app\n",
    )
    .unwrap();
    fs::write(
        root.path().join("app/package.json"),
        r#"{"name":"review","version":"1.0.0","scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("oyzu.toml"),
        "[tasks.test]\nargv=['removed-root-body']\n",
    )
    .unwrap();
    fs::write(
        root.path().join("app/oyzu.toml"),
        "[overrides]\nremove=['tasks.test']\n",
    )
    .unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    assert!(workspace.configuration["app"].get("tasks.test").is_none());
    assert_eq!(tasks::resolve(&workspace, "test").unwrap(), "app:test");
    let captured = tempfile::tempdir().unwrap();
    let source = oyzu::snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
    let images = BTreeMap::from([(
        "app".to_owned(),
        oyzu::executor::Image {
            reference: "node:test".into(),
            digest: format!("sha256:{}", "1".repeat(64)),
            os: "linux".into(),
            arch: "amd64".into(),
        },
    )]);
    let plan = oyzu::build::plan(&workspace, &source, &images).unwrap();
    assert!(!plan["actions"].to_string().contains("removed-root-body"));
    assert!(plan["actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["id"] == "app:test"));
    // Replacing, rather than removing, a root operation uses the same final cascade.
    fs::write(
        root.path().join("app/oyzu.toml"),
        "[tasks.test]\nargv=['replacement-body']\n",
    )
    .unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    assert_eq!(
        workspace.tasks[&tasks::resolve(&workspace, "test").unwrap()].argv,
        vec!["replacement-body"]
    );
}
