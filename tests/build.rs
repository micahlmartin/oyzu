use oyzu::{build, discovery, executor::Image, records, snapshot};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

fn planned(root: &Path, temp: &Path) -> Value {
    let source = snapshot::capture(root, &temp.join("captured")).unwrap();
    let workspace = discovery::discover_with_shell(&temp.join("captured"), Some("sh")).unwrap();
    let images = workspace
        .targets
        .keys()
        .map(|id| {
            (
                id.clone(),
                Image {
                    reference: "node:test".into(),
                    digest: format!("sha256:{}", "1".repeat(64)),
                    os: "linux".into(),
                    arch: "amd64".into(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    build::plan(&workspace, &source, &images).unwrap()
}

#[test]
fn plans_are_location_independent_bind_toolchain_and_keep_hooks() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    for root in [a.path(), b.path()] {
        fs::write(
            root.join("package.json"),
            r#"{"name":"demo","version":"1.2.3","scripts":{"test":"node --test"}}"#,
        )
        .unwrap();
        fs::write(root.join("oyzu.toml"),"[tasks.\"project:pre_test\"]\nrun = \"echo prepare\"\n[tasks.\"project:post_test\"]\nrun = \"echo done\"\n").unwrap();
    }
    let sa = tempfile::tempdir().unwrap();
    let sb = tempfile::tempdir().unwrap();
    let p = planned(a.path(), sa.path());
    assert_eq!(p, planned(b.path(), sb.path()));
    let ids: Vec<_> = p["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            "project:version",
            "project:prepare",
            "project:pre_test",
            "project:test",
            "project:post_test",
            "project:package"
        ]
    );
    assert_eq!(p["actions"][2]["argv"][0], "sh");
    assert_eq!(p["actions"][3]["reports"].as_array().unwrap().len(), 2);
    assert!(p["artifacts"][0]["version"]
        .as_str()
        .unwrap()
        .starts_with("1.2.3-dev.g"));
    let mut other = p.clone();
    other["tools"][0]["digest"] = json!(format!("sha256:{}", "2".repeat(64)));
    assert_ne!(
        records::digest("oyzu.plan.v1alpha1", &p).unwrap(),
        records::digest("oyzu.plan.v1alpha1", &other).unwrap()
    );
}

#[test]
fn inspector_rejects_modified_artifact_and_path_escape() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("artifact"), "original").unwrap();
    records::write(&root.path().join("envelope.json"), &json!({})).unwrap();
    let mut manifest = json!({"kind":"build-manifest","status":"failed","planPath":null,"envelopePath":"envelope.json","envelopeDigest":snapshot::file_digest(&root.path().join("envelope.json")).unwrap(),"artifacts":[{"path":"artifact","size":8,"digest":snapshot::file_digest(&root.path().join("artifact")).unwrap()}],"reports":[]});
    records::write(&root.path().join("manifest.json"), &manifest).unwrap();
    build::inspect(root.path()).unwrap();
    fs::write(root.path().join("artifact"), "modified").unwrap();
    assert!(build::inspect(root.path()).is_err());
    manifest["artifacts"][0]["path"] = json!("../artifact");
    records::write(&root.path().join("manifest.json"), &manifest).unwrap();
    assert!(build::inspect(root.path()).is_err());
}

#[test]
fn semantic_digest_canonicalizes_unicode_property_order() {
    let value = json!({"\u{e000}":1,"\u{10000}":2});
    let bytes = serde_json_canonicalizer::to_string(&value).unwrap();
    assert_eq!(bytes, "{\"\u{10000}\":2,\"\u{e000}\":1}");
    assert_ne!(
        records::digest("oyzu.plan.v1alpha1", &value).unwrap(),
        records::digest("oyzu.tree.v1alpha1", &value).unwrap()
    );
}

#[test]
fn explicit_task_overrides_are_not_replaced_by_native_builder_planning() {
    let root = tempfile::tempdir().unwrap();
    let capture = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("oyzu.toml"),
        "[tasks.\"project:test\"]\nargv=['node','custom-tests.mjs']\n",
    )
    .unwrap();
    let plan = planned(root.path(), capture.path());
    let task = plan["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "project:test")
        .unwrap();
    assert_eq!(task["argv"], json!(["node", "custom-tests.mjs"]));
    assert!(task["reports"].as_array().unwrap().is_empty());
}

#[test]
fn unsupported_builder_preserves_preflight_failure_bundle() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("build.xml"),
        "<project name=\"preflight\" default=\"build\"><target name=\"build\"/></project>",
    )
    .unwrap();
    let failed = build::run(root.path(), &[], false).unwrap();
    assert_eq!(failed["status"], "failed");
    assert!(failed["planDigest"].is_null());
    assert!(failed["actions"].as_array().unwrap().is_empty());
    assert!(failed["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("not implemented"));
    assert_eq!(build::inspect(&root.path().join("dist")).unwrap(), failed);
}
