use super::*;
use serde_json::json;

#[test]
fn provider_selection_preserves_ambiguity_and_managed_admission_before_effects() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Dockerfile"), "FROM python:3.13-slim\n").unwrap();
    fs::write(root.path().join("requirements.txt"), "six==1.17.0\n").unwrap();
    let target =
        crate::discovery::discover_target("image", root.path(), Some("docker/image")).unwrap();
    let registry = crate::config::registry::Registry::default();
    let configuration = crate::config::resolve::resolve(
        &[],
        &registry,
        false,
        &Default::default(),
        Default::default(),
        false,
    )
    .unwrap();
    let runtime = executor::Image {
        reference: "python:3.13-slim".into(),
        digest: format!("sha256:{}", "1".repeat(64)),
        os: "linux".into(),
        arch: "amd64".into(),
    };
    let destination = root.path().join("not-created");
    let platform = runtime.platform().unwrap();
    let mut context = builders::PreparationContext {
        configuration: &configuration,
        dependency_selector: None,
        target: &target,
        destination: &destination,
        image: &runtime,
        target_platform: &platform,
        source_digest: "fixture",
        execution_name: "must-not-launch",
    };
    assert_eq!(select(&context).unwrap().id(), "python/pip");
    fs::write(
        root.path().join("pyproject.toml"),
        "[project]\nname='demo'\nversion='1.0'\n",
    )
    .unwrap();
    fs::write(root.path().join("uv.lock"), "fixture").unwrap();
    assert!(select(&context)
        .err()
        .unwrap()
        .to_string()
        .contains("unambiguous implemented"));
    context.dependency_selector = Some("python/uv");
    assert_eq!(select(&context).unwrap().id(), "python/uv");
    context.dependency_selector = Some("python/poetry");
    assert!(select(&context)
        .err()
        .unwrap()
        .to_string()
        .contains("native manifest"));
    fs::write(root.path().join("poetry.lock"), "fixture").unwrap();
    assert_eq!(select(&context).unwrap().id(), "python/poetry");
    context.dependency_selector = None;
    fs::write(root.path().join("package.json"), "{}").unwrap();
    assert!(select(&context)
        .err()
        .unwrap()
        .to_string()
        .contains("unambiguous ecosystem"));
    context.dependency_selector = Some("unregistered/provider");
    assert!(select(&context)
        .err()
        .unwrap()
        .to_string()
        .contains("not implemented"));
    context.dependency_selector = Some("node/npm");
    assert!(select(&context)
        .err()
        .unwrap()
        .to_string()
        .contains("native manifest"));
    fs::write(root.path().join("package-lock.json"), "{}").unwrap();
    assert_eq!(select(&context).unwrap().id(), "node/npm");
    context.dependency_selector = Some("node/yarn");
    assert!(select(&context).is_err());
    fs::write(root.path().join("yarn.lock"), "# yarn lockfile v1\n").unwrap();
    assert_eq!(select(&context).unwrap().id(), "node/yarn");
    for name in [
        "requirements.txt",
        "pyproject.toml",
        "uv.lock",
        "poetry.lock",
    ] {
        fs::remove_file(root.path().join(name)).unwrap();
    }
    context.dependency_selector = None;
    assert!(select(&context)
        .err()
        .unwrap()
        .to_string()
        .contains("unambiguous implemented"));
    fs::remove_file(root.path().join("package-lock.json")).unwrap();
    assert_eq!(select(&context).unwrap().id(), "node/yarn");
    fs::write(
        root.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '9.0'\n",
    )
    .unwrap();
    assert!(select(&context).is_err());
    context.dependency_selector = Some("node/pnpm");
    assert_eq!(select(&context).unwrap().id(), "node/pnpm");
    context.dependency_selector = Some("go/modules");
    assert!(select(&context).is_err());
    fs::write(root.path().join("go.mod"), "module example.test/fixture\n").unwrap();
    assert_eq!(select(&context).unwrap().id(), "go/modules");
    fs::write(root.path().join("requirements.txt"), "six==1.17.0\n").unwrap();
    context.dependency_selector = Some("python/pip");
    assert_eq!(select(&context).unwrap().id(), "python/pip");
    let other = executor::Image {
        arch: "arm64".into(),
        ..runtime.clone()
    };
    assert!(prepare(&context, &other)
        .err()
        .unwrap()
        .to_string()
        .contains("consumer target platform"));
    let mut managed = configuration.clone();
    managed.management = Some(json!({"mode":"managed"}));
    context.configuration = &managed;
    assert!(prepare(&context, &runtime)
        .err()
        .unwrap()
        .to_string()
        .contains("approved connector bindings"));
    assert!(!destination.exists());
}

#[test]
fn context_identity_requires_exact_runtime_source_and_preparation() {
    let runtime = executor::Image {
        reference: "python:3.13-slim".into(),
        digest: format!("sha256:{}", "1".repeat(64)),
        os: "linux".into(),
        arch: "amd64".into(),
    };
    let platform = runtime.platform().unwrap();
    let snapshot = json!({"sourceDigest":"source","manager":{"digest":runtime.digest}});
    let mut captured = Captured {
        binding: executor::DependencyContext {
            store: "contexts/packages".into(),
            tree_digest: format!("sha256:{}", "2".repeat(64)),
            platform: platform.clone(),
        },
        provider: "python/pip".into(),
        snapshot_digest: records::digest("oyzu.dependencies.v1alpha1", &snapshot).unwrap(),
        snapshot,
        runtime: runtime.clone(),
    };
    let images = vec![executor::ImageInput {
        reference: runtime.reference.clone(),
        name: runtime.reference.clone(),
        config: runtime.digest.clone(),
        store: "images/base-0".into(),
        manifest: format!("sha256:{}", "3".repeat(64)),
        tree_digest: format!("sha256:{}", "4".repeat(64)),
    }];
    captured
        .validate(&runtime.reference, &images, &platform, "source")
        .unwrap();
    assert!(captured
        .validate(&runtime.reference, &images, &platform, "changed")
        .is_err());
    assert!(captured
        .validate(&runtime.reference, &[], &platform, "source")
        .is_err());
    captured.snapshot["extra"] = json!("changed");
    assert!(captured
        .validate(&runtime.reference, &images, &platform, "source")
        .is_err());
}
