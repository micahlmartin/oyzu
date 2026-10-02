use super::*;
use crate::{discovery, snapshot};
use std::{fs, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.autocrlf=false",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Oyzu Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn fixture(root: &Path) {
    for name in ["shared", "api", "web", "unused"] {
        fs::create_dir(root.join(name)).unwrap();
        fs::write(
            root.join(name).join("package.json"),
            json!({"name":name,"version":"1.0.0"}).to_string(),
        )
        .unwrap();
        fs::write(
            root.join(name).join("index.mjs"),
            "export const value = 1;\n",
        )
        .unwrap();
    }
    fs::write(root.join("build.yaml"),"shared: {uses: node/package, path: shared}\napi: {uses: node/package, path: api, depends_on: [shared]}\nweb: {uses: node/package, path: web, depends_on: [api]}\nunused: {uses: node/package, path: unused}\n").unwrap();
    git(root, &["init", "--template=", "-b", "main"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "baseline"]);
}
fn impact(root: &Path, reference: &str) -> Impact {
    let capture = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root, &capture.path().join("source")).unwrap();
    resolve(&discovery::discover(root).unwrap(), &source, reference).unwrap()
}
#[test]
fn changed_owners_include_transitive_consumers_but_not_unrelated_targets() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    super::git::baseline(root.path(), "HEAD").unwrap();
    let unchanged = impact(root.path(), "HEAD");
    assert!(unchanged.targets.is_empty(), "{}", unchanged.evidence);
    fs::write(
        root.path().join("api/index.mjs"),
        "export const value = 2;\n",
    )
    .unwrap();
    let selected = impact(root.path(), "HEAD");
    assert_eq!(
        selected.targets,
        BTreeSet::from(["api".into(), "web".into()])
    );
    assert_eq!(selected.evidence["changedPaths"], json!(["api/index.mjs"]));
    assert!(selected.evidence["fallback"].is_null());
    fs::remove_file(root.path().join("shared/index.mjs")).unwrap();
    assert_eq!(
        impact(root.path(), "HEAD").targets,
        BTreeSet::from(["shared".into(), "api".into(), "web".into()])
    );
}
#[test]
fn missing_baseline_new_inputs_and_configuration_changes_select_everything() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let missing = impact(root.path(), "missing-ref");
    assert_eq!(missing.targets.len(), 4);
    assert_eq!(missing.evidence["fallback"], "baseline-unavailable");
    fs::write(root.path().join("api/new.mjs"), "export default 1;\n").unwrap();
    assert_eq!(
        impact(root.path(), "HEAD").evidence["fallback"],
        "new-or-untracked-input"
    );
    fs::remove_file(root.path().join("api/new.mjs")).unwrap();
    fs::write(
        root.path().join("api/package.json"),
        r#"{"name":"api","version":"2.0.0"}"#,
    )
    .unwrap();
    assert_eq!(
        impact(root.path(), "HEAD").evidence["fallback"],
        "configuration-or-lock-change"
    );
}
#[test]
fn comparison_uses_captured_bytes_and_ignores_generated_outputs() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    fs::create_dir(root.path().join("dist")).unwrap();
    fs::write(root.path().join("dist/old.txt"), "output").unwrap();
    let capture = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &capture.path().join("source")).unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    fs::write(root.path().join("api/index.mjs"), "changed after capture").unwrap();
    assert!(resolve(&workspace, &source, "HEAD")
        .unwrap()
        .targets
        .is_empty());
}
