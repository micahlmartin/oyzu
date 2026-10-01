use super::*;
use crate::{dependencies::Prepared, discovery, snapshot};
use serde_json::json;
use std::{collections::BTreeSet, fs};

#[test]
fn descriptors_have_unique_ids_and_own_their_runtime_files() {
    let mut ids = BTreeSet::new();
    for builder in all() {
        for id in builder.descriptor().ids {
            assert!(ids.insert(id), "duplicate builder id {id}");
            assert!(std::ptr::eq(get(id).unwrap(), *builder));
        }
        let mut files = BTreeSet::new();
        for file in builder.runtime_files() {
            assert!(files.insert(file.name));
            assert!(!file.name.contains(['/', '\\', ':']));
            assert!(!file.contents.is_empty());
        }
    }
    assert!(get("node/package").unwrap().runtime_files().is_empty());
    assert_eq!(get("python/package").unwrap().runtime_files().len(), 2);
    assert!(get("unknown/builder").is_err());
}

#[test]
fn native_workspace_markers_select_one_owner() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("go.mod"), "module example.com/app\n").unwrap();
    fs::write(root.path().join("go.work"), "go 1.24\nuse .\n").unwrap();
    let candidates: Vec<_> = all().iter().filter_map(|b| b.detect(root.path())).collect();
    assert_eq!(candidates, vec!["go/app"]);
    fs::write(root.path().join("package.json"), "{}").unwrap();
    assert!(discovery::discover(root.path()).is_err());
}

#[test]
fn python_planning_requires_captured_dependencies_and_declares_typed_outputs() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("pyproject.toml"),
        "[project]\nname='demo-lib'\nversion='1.2.3'\n",
    )
    .unwrap();
    let capture = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &capture.path().join("source")).unwrap();
    let workspace = discovery::discover(&capture.path().join("source")).unwrap();
    let target = &workspace.targets["project"];
    let builder = get(&target.builder).unwrap();
    assert!(builder
        .plan(PlanningContext {
            target,
            source: &source,
            dependencies: None
        })
        .is_err());
    let dependencies = Prepared {
        root: capture.path().join("dependencies"),
        digest: format!("sha256:{}", "1".repeat(64)),
        record: json!({}),
    };
    let plan = builder
        .plan(PlanningContext {
            target,
            source: &source,
            dependencies: Some(&dependencies),
        })
        .unwrap();
    assert!(plan.version.starts_with("1.2.3.dev0+g"));
    assert_eq!(plan.artifacts.len(), 2);
    assert_eq!(plan.artifacts[0].name, "wheel");
    assert!(plan.artifacts[0].filename.starts_with("demo_lib-"));
    assert_eq!(plan.tasks["test"].reports.len(), 2);
    assert_eq!(plan.tasks["test"].reports[0].format.name(), "junit");
    assert_eq!(plan.env["PIP_NO_INDEX"], "1");
}

#[test]
fn generated_artifact_identifiers_are_bounded_and_invalid_contracts_fail_early() {
    let long = "a".repeat(100);
    let first = crate::names::scoped("crate", &long);
    assert!(crate::names::valid(&first));
    assert_eq!(first.len(), 64);
    assert_eq!(first, crate::names::scoped("crate", &long));
    assert_ne!(first, crate::names::scoped("crate", &(long + "b")));
    assert_ne!(
        crate::names::scoped("crate", "a.b"),
        crate::names::scoped("crate", "a_b")
    );
    let mut plan = BuilderPlan::new("1.0.0".into(), CommandSpec::new("package", &["package"]));
    plan.artifacts.push(ArtifactSpec {
        name: "crate.invalid".into(),
        filename: "package.crate".into(),
        media_type: "application/gzip",
        version: None,
    });
    assert!(plan.validate().is_err());
    plan.artifacts[0].name = "crate-valid".into();
    plan.validate().unwrap();
    plan.artifacts[0].filename = "../escaped".into();
    assert!(plan.validate().is_err());
    plan.artifacts[0].filename = "package.crate".into();
    plan.artifacts.push(ArtifactSpec {
        name: "CRATE-VALID".into(),
        filename: "other.crate".into(),
        media_type: "application/gzip",
        version: None,
    });
    assert!(plan.validate().is_err());
}
