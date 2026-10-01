use super::metadata::{relative, Metadata};
use serde_json::json;
use std::fs;

fn metadata() -> Metadata {
    serde_json::from_value(json!({
        "workspace_root":"/workspace", "workspace_members":["opaque-a","opaque-b"],
        "packages":[
            {"id":"opaque-a","name":"api","version":"1.2.3","manifest_path":"/workspace/api/Cargo.toml","source":null,
             "dependencies":[{"name":"core","source":null,"path":"/workspace/core"}],
             "targets":[{"name":"api","kind":["bin"],"src_path":"/workspace/api/src/main.rs"}]},
            {"id":"opaque-b","name":"core","version":"2.0.0","manifest_path":"/workspace/core/Cargo.toml","source":null,
             "dependencies":[],"targets":[{"name":"core","kind":["lib"],"src_path":"/workspace/core/src/lib.rs"}]}
        ]
    })).unwrap()
}

#[test]
fn cargo_paths_cannot_escape_the_captured_target() {
    for path in [
        "/host/Cargo.toml",
        "/workspace/../secret",
        "/workspace/a/../../secret",
        "/workspace/a\\b",
        "/workspace//a",
    ] {
        assert!(relative(path).is_err(), "{path}");
    }
    let mut native = metadata();
    native.validate().unwrap();
    native.packages[0].dependencies[0].path = Some("/workspace".into());
    native.validate().unwrap();
    native.packages[0].dependencies[0].source = Some("registry+https://example.invalid".into());
    assert!(native
        .validate()
        .unwrap_err()
        .to_string()
        .contains("acquisition"));
}

#[test]
fn cargo_projection_preserves_workspace_settings_and_updates_aliased_requirements() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("api")).unwrap();
    fs::create_dir(root.path().join("core")).unwrap();
    fs::write(root.path().join("Cargo.toml"), "[workspace]\nmembers=['api','core']\nresolver='2'\n[workspace.package]\nversion='2.0.0'\n[workspace.dependencies]\nalias={package='core',path='core',version='2'}\n").unwrap();
    fs::write(root.path().join("api/Cargo.toml"), "[package]\nname='api'\nversion='1.2.3'\n[dev-dependencies]\ncore={path='../core'}\n[target.'cfg(unix)'.dependencies]\nalias={package='core',path='../core',version='2',default-features=false}\n").unwrap();
    fs::write(
        root.path().join("core/Cargo.toml"),
        "[package]\nname='core'\nversion.workspace=true\n[features]\nextra=[]\n",
    )
    .unwrap();
    let files = metadata()
        .project_versions(root.path(), &format!("sha256:{}", "a".repeat(64)))
        .unwrap();
    assert_eq!(files.len(), 3);
    let read = |name: &str| -> toml::Value {
        toml::from_str(&fs::read_to_string(root.path().join(name)).unwrap()).unwrap()
    };
    let api = read("api/Cargo.toml");
    assert_eq!(
        api["package"]["version"].as_str(),
        Some("1.2.3-dev.gaaaaaaaaaaaa")
    );
    let dep = &api["target"]["cfg(unix)"]["dependencies"]["alias"];
    assert_eq!(dep["version"].as_str(), Some("=2.0.0-dev.gaaaaaaaaaaaa"));
    assert_eq!(dep["default-features"].as_bool(), Some(false));
    assert_eq!(
        api["dev-dependencies"]["core"]["version"].as_str(),
        Some("=2.0.0-dev.gaaaaaaaaaaaa")
    );
    let core = read("core/Cargo.toml");
    assert_eq!(
        core["package"]["version"].as_str(),
        Some("2.0.0-dev.gaaaaaaaaaaaa")
    );
    assert!(core["features"]["extra"].is_array());
    assert_eq!(
        read("Cargo.toml")["workspace"]["dependencies"]["alias"]["version"].as_str(),
        Some("=2.0.0-dev.gaaaaaaaaaaaa")
    );
}

#[test]
fn cargo_plan_keeps_independent_binary_versions_and_offline_checks() {
    use crate::{builders::PlanningContext, dependencies::Prepared, discovery, snapshot};
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), "[workspace]\nmembers=[]\n").unwrap();
    let capture = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &capture.path().join("source")).unwrap();
    let workspace = discovery::discover(&capture.path().join("source")).unwrap();
    let mut native = metadata();
    native.packages[0].version = "1.2.3-dev.gaaaaaaaaaaaa".into();
    native.packages[1].version = "2.0.0-dev.gaaaaaaaaaaaa".into();
    native.packages[1].targets[0].kind = vec!["bin".into()];
    let prepared = Prepared {
        root: capture.path().join("prepared"),
        digest: source.digest.clone(),
        record: json!({}),
    };
    fs::create_dir(&prepared.root).unwrap();
    fs::write(
        prepared.root.join("metadata.json"),
        serde_json::to_vec(&native).unwrap(),
    )
    .unwrap();
    fs::write(prepared.root.join("host.txt"), "x86_64-unknown-linux-gnu").unwrap();
    let plan = super::planning::plan(PlanningContext {
        target: &workspace.targets["project"],
        source: &source,
        dependencies: Some(&prepared),
    })
    .unwrap();
    assert_eq!(plan.artifacts.len(), 4);
    for artifact in &plan.artifacts {
        let package = native
            .packages
            .iter()
            .find(|p| artifact.name == p.name || artifact.name == format!("crate.{}", p.name))
            .unwrap();
        assert_eq!(artifact.version.as_ref(), Some(&package.version));
        assert!(artifact.filename.contains(&package.version));
        assert!(plan
            .package
            .argv
            .contains(&format!("/out/project/artifacts/{}", artifact.filename)));
    }
    for stage in ["build", "lint", "archive"] {
        assert!(plan.tasks[stage].argv.contains(&"--locked".into()));
        assert!(plan.tasks[stage].argv.contains(&"--offline".into()));
    }
    assert_eq!(plan.tasks["test"].reports[0].format.name(), "junit");
    assert_eq!(plan.env["CARGO_NET_OFFLINE"], "true");
}
