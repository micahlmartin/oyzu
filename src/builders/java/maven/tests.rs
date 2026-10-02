use super::*;
use crate::{dependencies::Prepared, discovery, snapshot};
use serde_json::json;

#[test]
fn native_reactor_plan_binds_module_artifacts_reports_and_one_lifecycle() {
    let root = tempfile::tempdir().unwrap();
    let prepared = tempfile::tempdir().unwrap();
    fs::write(root.path().join("pom.xml"), "<project><modelVersion>4.0.0</modelVersion><groupId>example</groupId><artifactId>app</artifactId><version>1.0.0</version></project>").unwrap();
    fs::create_dir_all(root.path().join("src/test/java")).unwrap();
    fs::write(
        root.path().join("src/test/java/AppTest.java"),
        "class AppTest {}",
    )
    .unwrap();
    let document = "<reactor><project><groupId>example</groupId><artifactId>app</artifactId><version>1.0.0-dev.g123</version><packaging>jar</packaging><path>.</path><pom>pom.xml</pom><buildDirectory>target</buildDirectory><finalName>app-1.0.0-dev.g123</finalName><testRoots><path>src/test/java</path></testRoots><testReports><directory>target/custom-unit</directory><directory>target/custom-integration</directory></testReports></project></reactor>";
    fs::write(prepared.path().join("metadata.xml"), document).unwrap();
    let captured = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
    let workspace = discovery::discover(&captured.path().join("source")).unwrap();
    let target = &workspace.targets["project"];
    let dependencies = Prepared {
        root: prepared.path().into(),
        digest: "sha256:test".into(),
        record: json!({}),
    };
    let plan = Maven
        .plan(PlanningContext {
            target,
            source: &source,
            dependencies: Some(&dependencies),
        })
        .unwrap();
    plan.validate().unwrap();
    assert_eq!(plan.artifacts.len(), 2);
    assert!(plan
        .artifacts
        .iter()
        .any(|a| a.filename == "app-1.0.0-dev.g123.jar"));
    assert_eq!(plan.tasks.len(), 4);
    assert!(plan.tasks.contains_key("lint"));
    assert!(plan.tasks.contains_key("format-check"));
    assert_eq!(plan.tasks["build"].reports.len(), 2);
    assert_eq!(
        plan.tasks["build"].reports[0].input.as_deref(),
        Some(".oyzu-maven/reports/0/TEST-*.xml")
    );
    assert_eq!(plan.tasks["build"].reports[1].format.name(), "jacoco");
    assert!(!target.tasks["test"].build_stage);
    fs::write(
        prepared.path().join("metadata.xml"),
        document.replace("<buildDirectory>target", "<buildDirectory>../target"),
    )
    .unwrap();
    assert!(metadata::read(&prepared.path().join("metadata.xml")).is_err());
    fs::write(
        prepared.path().join("metadata.xml"),
        document.replace("<directory>target/custom-unit", "<directory>../outside"),
    )
    .unwrap();
    assert!(metadata::read(&prepared.path().join("metadata.xml")).is_err());
    fs::write(
        prepared.path().join("metadata.xml"),
        document.replace("testReports", "oldReports"),
    )
    .unwrap();
    assert!(metadata::read(&prepared.path().join("metadata.xml"))
        .err()
        .unwrap()
        .to_string()
        .contains("regenerate"));
    let second = document
        .replace("<artifactId>app", "<artifactId>other")
        .replace("<pom>pom.xml", "<pom>other/pom.xml");
    fs::write(
        prepared.path().join("metadata.xml"),
        format!(
            "{}{}",
            document.trim_end_matches("</reactor>"),
            second.trim_start_matches("<reactor>")
        ),
    )
    .unwrap();
    assert!(metadata::read(&prepared.path().join("metadata.xml"))
        .err()
        .unwrap()
        .to_string()
        .contains("shared by multiple"));
    fs::write(
        prepared.path().join("metadata.xml"),
        document.replace("target/custom-unit", ".oyzu-maven/reports"),
    )
    .unwrap();
    assert!(metadata::read(&prepared.path().join("metadata.xml"))
        .err()
        .unwrap()
        .to_string()
        .contains("reserved"));
    fs::write(
        prepared.path().join("metadata.xml"),
        document.replace("<finalName>app-", "<finalName>../app-"),
    )
    .unwrap();
    assert!(metadata::read(&prepared.path().join("metadata.xml")).is_err());
}
