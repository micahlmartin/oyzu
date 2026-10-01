use oyzu::{discovery, tasks};
use std::{fs, path::Path};

fn write(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, text).unwrap();
}

#[test]
fn native_scripts_and_go_checks_are_discovered_without_execution() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "package.json",
        r#"{"name":"demo","version":"1.2.3","scripts":{"foo":"DO NOT EXECUTE","test":"node --test"}}"#,
    );
    let ws = discovery::discover(root.path()).unwrap();
    let target = ws.targets.values().next().unwrap();
    assert_eq!(target.version, "1.2.3");
    assert_eq!(target.tasks["foo"].argv, vec!["npm", "run", "foo"]);
    assert!(!target.tasks["foo"].build_stage);
    assert!(target.tasks["test"].build_stage);
    assert!(target.tasks["format"].availability.is_some());
    fs::remove_file(root.path().join("package.json")).unwrap();
    write(
        root.path(),
        "go.mod",
        "module example.invalid/demo\n\ngo 1.22\n",
    );
    let ws = discovery::discover(root.path()).unwrap();
    let target = ws.targets.values().next().unwrap();
    assert!(target.tasks["format"].mutates_source);
    assert!(!target.tasks["format"].build_stage);
    assert!(target.tasks["format-check"].stdout_must_be_empty);
}

#[test]
fn hooks_override_in_place_and_cycles_fail_before_execution() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "build.yaml", "api:\n  uses: node/app\n");
    write(
        root.path(),
        "package.json",
        r#"{"scripts":{"test":"node --test"}}"#,
    );
    write(
        root.path(),
        "oyzu.toml",
        r#"
[tasks."api:test"]
argv = ["replacement"]
[tasks."api:pre_test"]
argv = ["before"]
[tasks."api:post_test"]
argv = ["after"]
"#,
    );
    let ws = discovery::discover(root.path()).unwrap();
    assert_eq!(ws.tasks["api:test"].argv, vec!["replacement"]);
    assert!(ws.tasks["api:test"].build_stage);
    assert_eq!(
        tasks::sequence(&ws, "test").unwrap(),
        vec!["api:pre_test", "api:test", "api:post_test"]
    );
    assert_eq!(
        tasks::sequence(&ws, "api:pre_test").unwrap(),
        vec!["api:pre_test"]
    );
    write(
        root.path(),
        "oyzu.toml",
        r#"
[tasks."api:test"]
argv = ["replacement"]
depends_on = ["api:pre_test"]
[tasks."api:pre_test"]
argv = ["before"]
depends_on = ["api:test"]
"#,
    );
    let ws = discovery::discover(root.path()).unwrap();
    assert!(tasks::sequence(&ws, "test")
        .unwrap_err()
        .to_string()
        .contains("cycle"));
}

#[test]
fn ambiguous_managers_and_escaping_targets_fail() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "package.json", "{}");
    write(root.path(), "package-lock.json", "{}");
    write(root.path(), "pnpm-lock.yaml", "lockfileVersion: 9");
    assert!(discovery::discover(root.path())
        .unwrap_err()
        .to_string()
        .contains("conflicting"));
    write(
        root.path(),
        "build.yaml",
        "api:\n  uses: node/app\n  path: ../elsewhere\n",
    );
    assert!(discovery::discover(root.path())
        .unwrap_err()
        .to_string()
        .contains("escapes"));
}

#[test]
fn multiple_targets_require_qualified_names_and_native_maven_is_one_owner() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "build.yaml",
        "api:\n  uses: node/app\n  path: api\nweb:\n  uses: node/app\n  path: web\n",
    );
    for dir in ["api", "web"] {
        write(
            root.path(),
            &format!("{dir}/package.json"),
            r#"{"scripts":{"test":"node --test"}}"#,
        );
    }
    let ws = discovery::discover(root.path()).unwrap();
    assert!(tasks::resolve(&ws, "test")
        .unwrap_err()
        .to_string()
        .contains("ambiguous"));
    assert_eq!(tasks::resolve(&ws, "api:test").unwrap(), "api:test");
}

#[test]
fn existing_builder_examples_are_discoverable_without_native_toolchains() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/builds");
    for project in [
        "node-package/project",
        "node-managers/npm",
        "node-managers/pnpm",
        "node-managers/yarn",
        "python-api/project",
        "python-uv-library/project",
        "python-poetry/project",
        "python-legacy-native/project",
        "go-app/project",
        "go-workspace-cgo/project",
        "rust-app/project",
        "rust-workspace/project",
        "java-maven-reactor/project",
        "java-gradle-multi-project/project",
        "java-ant/conventional",
        "docker-offline/project",
        "helm-chart/project/chart",
        "helm-chart/project",
        "container-variants/project",
        "image-and-chart/project",
        "mixed-monorepo/project",
        "materialize-selected-artifacts/project",
        "materialize-directory/project",
    ] {
        let ws = discovery::discover(&examples.join(project))
            .unwrap_or_else(|error| panic!("{project}: {error:#}"));
        assert!(!ws.tasks.is_empty(), "{project}");
    }
}
