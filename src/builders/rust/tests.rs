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
fn cargo_required_features_select_only_resolved_binary_targets() {
    let mut native = metadata();
    native.packages[0].targets[0].required_features = vec!["extra".into()];
    assert!(native.binaries().is_err());
    native.resolve = Some(super::metadata::Resolution {
        nodes: vec![super::metadata::Node {
            id: "opaque-a".into(),
            features: vec![],
        }],
    });
    assert!(native.binaries().unwrap().is_empty());
    native.resolve.as_mut().unwrap().nodes[0]
        .features
        .push("extra".into());
    assert_eq!(native.binaries().unwrap().len(), 1);
}

#[test]
fn cargo_registry_packages_are_inputs_not_workspace_outputs() {
    let mut native = metadata();
    native.packages.push(serde_json::from_value(json!({
        "id":"registry-package", "name":"third-party", "version":"1.0.0",
        "manifest_path":"/tmp/oyzu-cargo/registry/src/native/third-party-1.0.0/Cargo.toml",
        "source":super::acquisition::CRATES_IO,"dependencies":[],
        "targets":[{"name":"example-bin","kind":["bin"],"src_path":"/tmp/oyzu-cargo/registry/src/native/third-party-1.0.0/src/main.rs"}]
    })).unwrap());
    native.validate().unwrap();
    assert_eq!(native.binaries().unwrap().len(), 1);
    native.packages.last_mut().unwrap().manifest_path =
        "/tmp/oyzu-cargo/registry/src/../../outside/Cargo.toml".into();
    assert!(native.validate().is_err());
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
            .find(|p| {
                artifact.name == crate::names::scoped("bin", &p.name)
                    || artifact.name == crate::names::scoped("crate", &p.name)
            })
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
    assert_eq!(plan.tasks["test"].reports[1].format.name(), "cobertura");
    assert_eq!(plan.env["CARGO_NET_OFFLINE"], "true");
    assert_eq!(plan.env["CARGO_LLVM_COV_SETUP"], "no");
    assert_eq!(plan.fixed_env["CARGO_TARGET_DIR"], ".oyzu-build/target");
    assert_eq!(
        plan.fixed_env["CARGO_BUILD_TARGET"],
        "x86_64-unknown-linux-gnu"
    );
    assert!(plan
        .package
        .argv
        .contains(&".oyzu-build/target/oyzu-binaries/0".into()));
    assert!(plan
        .package
        .argv
        .contains(&".oyzu-build/target/oyzu-binaries/1".into()));
    assert!(plan.tasks["build"]
        .argv
        .contains(&"--message-format=json-render-diagnostics".into()));
}

#[test]
fn nextest_report_destination_is_independent_of_native_store_and_source_config() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join(".config")).unwrap();
    let source = "[store]\ndir='custom-store'\n[profile.default]\nretries=2\n[profile.default.junit]\npath='previous.xml'\nstore-failure-output=false\n";
    let path = root.path().join(".config/nextest.toml");
    fs::write(&path, source).unwrap();
    let configured: toml::Value = toml::from_str(
        &super::reporting::configuration(root.path(), "/out/api/reports/junit.xml").unwrap(),
    )
    .unwrap();
    assert_eq!(configured["store"]["dir"].as_str(), Some("custom-store"));
    assert_eq!(
        configured["profile"]["default"]["retries"].as_integer(),
        Some(2)
    );
    assert_eq!(
        configured["profile"]["default"]["junit"]["path"].as_str(),
        Some("/out/api/reports/junit.xml")
    );
    assert_eq!(
        configured["profile"]["default"]["junit"]["store-failure-output"].as_bool(),
        Some(false)
    );
    assert_eq!(fs::read_to_string(path).unwrap(), source);
}
