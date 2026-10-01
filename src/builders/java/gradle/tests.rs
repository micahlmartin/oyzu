use super::*;
use crate::{dependencies::Prepared, discovery, snapshot};
use serde_json::json;
use std::fs;

#[test]
fn composite_metadata_plans_native_archives_and_only_existing_test_sources() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("build.gradle"), "plugins { id 'java' }").unwrap();
    let captured = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
    let workspace = discovery::discover(&captured.path().join("source")).unwrap();
    let target = &workspace.targets["project"];
    let prepared = tempfile::tempdir().unwrap();
    fs::create_dir(prepared.path().join("metadata")).unwrap();
    let model = json!({"schemaVersion":1,"gradleVersion":"8.14.3","directory":".","projects":[{
        "path":":","version":"1.2.3-dev.g123",
        "archives":[{"task":":jar","file":"build/libs/app-1.2.3-dev.g123.jar","extension":"jar","version":"1.2.3-dev.g123"}],
        "tests":[{"task":":test","junit":"build/test-results/test","junitEnabled":true,"sourcesKnown":true,"sources":["src/test/java/AppTest.java"],"coverage":"build/reports/oyzu/test/jacoco.xml"},
            {"task":":emptyTest","junit":"build/test-results/empty","junitEnabled":true,"sourcesKnown":true,"sources":[],"coverage":null}]
    }]});
    let file = prepared.path().join("metadata/root.json");
    fs::write(&file, serde_json::to_vec(&model).unwrap()).unwrap();
    let dependencies = Prepared {
        root: prepared.path().into(),
        digest: "sha256:test".into(),
        record: json!({}),
    };
    let plan = Gradle
        .plan(PlanningContext {
            target,
            source: &source,
            dependencies: Some(&dependencies),
        })
        .unwrap();
    plan.validate().unwrap();
    assert_eq!(plan.artifacts.len(), 1);
    assert_eq!(plan.artifacts[0].version.as_deref(), Some("1.2.3-dev.g123"));
    assert_eq!(plan.tasks["build"].reports.len(), 2);
    assert!(!target.tasks["test"].build_stage);
    let mut escaped = model;
    escaped["projects"][0]["archives"][0]["file"] = json!("../outside.jar");
    fs::write(&file, serde_json::to_vec(&escaped).unwrap()).unwrap();
    assert!(metadata::read(&prepared.path().join("metadata")).is_err());
}
