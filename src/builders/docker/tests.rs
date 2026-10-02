use super::*;
use crate::{
    builders::{Builder, PlanningContext},
    dependencies::Prepared,
    discovery, snapshot,
};
use serde_json::json;
use std::fs;

fn metadata() -> serde_json::Value {
    json!({"schemaVersion":"v1alpha1","frontend":"dockerfile.v0","stages":[{"name":"","base":"scratch"}],"requirements":[],"context":{"files":["Dockerfile","greeting.txt"]}})
}

#[test]
fn plans_snapshot_oci_artifact_and_a_typed_private_worker() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Dockerfile"),
        "FROM scratch\nCOPY greeting.txt /greeting.txt\n",
    )
    .unwrap();
    fs::write(root.path().join("greeting.txt"), "hello\n").unwrap();
    fs::write(root.path().join(".hadolint.yaml"), "ignored: [DL3000]\n").unwrap();
    fs::write(
        root.path().join(".editorconfig"),
        "root = true\n[Dockerfile]\ninsert_final_newline = false\n",
    )
    .unwrap();
    fs::write(
        root.path().join(".dockerignore"),
        "Dockerfile\n.dockerignore\n.hadolint.yaml\n.editorconfig\n",
    )
    .unwrap();
    let capture = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &capture.path().join("source")).unwrap();
    let workspace = discovery::discover(&capture.path().join("source")).unwrap();
    let target = &workspace.targets["project"];
    assert_eq!(target.discovery["linter"].selected(), "hadolint");
    assert_eq!(target.discovery["formatter"].selected(), "dockerfmt");
    assert!(target.tasks["lint"].build_stage && target.tasks["format-check"].build_stage);
    assert!(target.tasks["format"].mutates_source && !target.tasks["format"].build_stage);
    assert!(Docker
        .plan(PlanningContext {
            target,
            source: &source,
            dependencies: None
        })
        .is_err());
    let mut native = metadata();
    native["context"] = json!({"files":["greeting.txt"],"ignoreFile":".dockerignore"});
    let prepared = Prepared {
        root: capture.path().into(),
        digest: format!("sha256:{}", "1".repeat(64)),
        record: json!({"targetPlatform":{"os":"linux","arch":"amd64"},"extensions":{"oyzu.dev/docker":{"metadata":native,"apparmorProfile":"oyzu-buildkit","dockerfileDigest":snapshot::file_digest(&target.path.join("Dockerfile")).unwrap()}}}),
    };
    let plan = Docker
        .plan(PlanningContext {
            target,
            source: &source,
            dependencies: Some(&prepared),
        })
        .unwrap();
    plan.validate().unwrap();
    // Control files stay reserved for admission, even when excluded from COPY.
    assert_eq!(
        plan.source_files.as_ref().unwrap(),
        &[
            ".dockerignore",
            ".editorconfig",
            ".hadolint.yaml",
            "Dockerfile",
            "greeting.txt"
        ]
    );
    let crate::executor::Mode::Buildkit { context_files, .. } = &plan.tasks["build"].execution
    else {
        panic!("expected native worker");
    };
    assert_eq!(context_files, &["greeting.txt"]);
    assert!(plan.version.starts_with("0.0.0-dev.g"));
    assert_eq!(
        serde_json::to_value(&plan.artifacts[0].kind).unwrap(),
        "oci-image"
    );
    let action = &plan.tasks["build"].execution;
    let argv = action.argv("linux/amd64").unwrap();
    assert!(argv.iter().any(|a| a == "force-network-mode=none"));
    assert!(argv.iter().any(|a| a == "--no-cache"));
    assert!(!action.enforced().contains(&"docker-read-only-root"));
    assert!(matches!(
        plan.tasks["test"].execution,
        crate::executor::Mode::OciValidation { .. }
    ));
    assert_eq!(plan.tasks["test"].reports.len(), 1);
    assert_eq!(
        serde_json::to_value(&plan.coverage).unwrap()["status"],
        "inapplicable"
    );
    assert!(serde_json::from_value::<crate::executor::Mode>(
        json!({"kind":"buildkit","hostSocket":"/var/run/docker.sock"})
    )
    .is_err());
}

#[test]
fn requires_explicit_integration_for_external_dynamic_and_secret_inputs() {
    for kind in [
        "image",
        "image-or-context",
        "dynamic-base",
        "dynamic-mount",
        "frontend",
        "secret",
        "ssh",
        "host-network",
        "onbuild",
    ] {
        let mut value = metadata();
        value["requirements"] = json!([{"kind":kind,"stage":0,"line":2,"reference":"external"}]);
        let parsed: metadata::Metadata = serde_json::from_value(value).unwrap();
        assert!(parsed.validate("linux/amd64").is_err(), "accepted {kind}");
    }
    let mut value = metadata();
    value["requirements"] = json!([{"kind":"add-source","stage":0,"line":2,"reference":"https://example.invalid/input"}]);
    assert!(serde_json::from_value::<metadata::Metadata>(value)
        .unwrap()
        .validate("linux/amd64")
        .is_err());
    let mut value = metadata();
    value["requirements"] = json!([{"kind":"cache-mount","stage":0,"line":2},{"kind":"platform","stage":0,"line":1,"reference":"linux/amd64"}]);
    let parsed: metadata::Metadata = serde_json::from_value(value).unwrap();
    parsed.validate("linux/amd64").unwrap();
    assert!(parsed.validate("linux/arm64").is_err());
}
