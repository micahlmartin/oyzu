use super::*;
use crate::{discovery, snapshot};
use serde_json::json;

#[test]
fn native_ant_outputs_are_contained_and_packaging_is_deterministic() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("build.xml"), "<project default='jar'><target name='compile'/><target name='test'/><target name='jar'/></project>").unwrap();
    let captured = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
    let workspace = discovery::discover(&captured.path().join("source")).unwrap();
    fs::write(root.path().join("metadata.xml"), "<project antVersion='Apache Ant(TM) version 1.10.18' version='1.2.3-dev.g0123456789ab'><target name='jar'><jar path='/workspace/dist/demo-1.2.3-dev.g0123456789ab.jar'/></target></project>").unwrap();
    let prepared = Prepared {
        root: root.path().into(),
        digest: source.digest.clone(),
        record: json!({}),
    };
    let plan = planning::plan(PlanningContext {
        target: &workspace.targets["project"],
        source: &source,
        dependencies: Some(&prepared),
    })
    .unwrap();
    plan.validate().unwrap();
    assert_eq!(
        plan.artifacts[0].filename,
        "demo-1.2.3-dev.g0123456789ab.jar"
    );
    assert_eq!(plan.tasks["build"].argv.last().unwrap(), "compile");
    assert_eq!(plan.tasks["archive"].argv.last().unwrap(), "jar");
    assert_eq!(plan.tasks["test"].reports.len(), 2);
    assert_eq!(plan.tasks["test"].reports[1].format.name(), "jacoco");
    assert!(plan.tasks.contains_key("lint"));
    for path in [
        "/outside.jar",
        "/workspace/../outside.jar",
        "/workspace/dist/not-a-jar",
    ] {
        fs::write(
            root.path().join("metadata.xml"),
            format!(
                "<project antVersion='Apache Ant(TM) version 1.10.18' version='1'><target name='jar'><jar path='{path}'/></target></project>"
            ),
        )
        .unwrap();
        assert!(metadata::read(&root.path().join("metadata.xml")).is_err());
    }
}

#[test]
fn native_ant_override_reporting_requires_one_literal_target() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("build.xml"),
        "<project><target name='test'/></project>",
    )
    .unwrap();
    let workspace = discovery::discover(root.path()).unwrap();
    let mut task = workspace.tasks["project:test"].clone();
    let env = std::collections::BTreeMap::from([
        ("OYZU_TEST_REPORT".into(), "/out/junit.xml".into()),
        ("OYZU_COVERAGE_REPORT".into(), "/out/jacoco.xml".into()),
        ("OYZU_VERSION".into(), "1.0.0-dev.g0123456789ab".into()),
    ]);
    for argv in [
        vec!["ant", "verify-contract"],
        vec!["sh", "-c", "ant verify-contract"],
    ] {
        task.argv = argv.into_iter().map(str::to_string).collect();
        let command = reporting::instrument(&task, &env).unwrap();
        assert_eq!(command.last().unwrap(), "verify-contract");
        assert!(command.contains(&"/out/jacoco.xml".into()));
    }
    for body in [
        "ant test && echo done",
        "ant $TARGET",
        "ant -version",
        "ant test other",
        "ant $(echo test)",
    ] {
        task.argv = vec!["sh".into(), "-c".into(), body.into()];
        assert!(reporting::instrument(&task, &env).is_none());
    }
}
