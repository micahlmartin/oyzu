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
    let discovery = &p["targets"][0]["extensions"]["oyzu.dev/discovery"]["package-manager"];
    assert_eq!(discovery["selected"], "npm");
    assert_eq!(discovery["registry"]["node/default-manager"], "1");
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
            "project:lint",
            "project:format-check",
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
fn implicit_node_tests_and_custom_scripts_inherit_report_obligations() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"default-test","version":"1.0.0"}"#,
    )
    .unwrap();
    let capture = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), capture.path());
    let action = plan["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "project:test")
        .unwrap();
    assert_eq!(action["argv"][0], "node");
    assert_eq!(action["argv"][1], "--test");
    assert_eq!(action["reports"].as_array().unwrap().len(), 2);
    assert!(action["argv"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a == "--test-reporter=junit"));
    fs::write(root.path().join("package.json"), r#"{"name":"custom-test","version":"1.0.0","scripts":{"test":"node arbitrary-harness.mjs"}}"#).unwrap();
    let capture = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), capture.path());
    let action = plan["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "project:test")
        .unwrap();
    assert_eq!(action["argv"], json!(["npm", "run", "test"]));
    assert_eq!(action["reports"].as_array().unwrap().len(), 2);
    assert!(action["env"]["OYZU_TEST_REPORT"].is_string());
    assert!(action["env"]["OYZU_COVERAGE_REPORT"].is_string());
    // An explicit wrapper around an opaque npm script must not acquire Node-only flags.
    fs::write(
        root.path().join("oyzu.toml"),
        "[tasks.test]\nargv=['npm','run','test']\n",
    )
    .unwrap();
    let capture = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), capture.path());
    let action = plan["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "test")
        .unwrap();
    assert_eq!(action["argv"], json!(["npm", "run", "test"]));
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
    assert_eq!(task["reports"].as_array().unwrap().len(), 2);
    assert!(task["reports"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["required"] == true));
    assert_eq!(
        task["env"]["OYZU_TEST_REPORT"],
        "/out/project/reports/junit.xml"
    );
    assert_eq!(
        task["env"]["OYZU_COVERAGE_REPORT"],
        "/out/project/reports/coverage.lcov"
    );
}

#[test]
fn conflicting_managers_preserve_preflight_failure_bundle() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"unsupported","packageManager":"pnpm@10.0.0"}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '9.0'\n",
    )
    .unwrap();
    fs::write(root.path().join("package-lock.json"), "{}").unwrap();
    let failed = build::run(root.path(), &[], false).unwrap();
    assert_eq!(failed["status"], "failed");
    assert!(failed["planDigest"].is_null());
    assert!(failed["actions"].as_array().unwrap().is_empty());
    assert!(failed["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("conflicting"));
    assert_eq!(build::inspect(&root.path().join("dist")).unwrap(), failed);
}

#[test]
fn materialization_adds_symbolic_inputs_and_orders_the_producer_before_its_consumer() {
    let root = tempfile::tempdir().unwrap();
    for name in ["consumer", "producer"] {
        fs::create_dir(root.path().join(name)).unwrap();
        fs::write(
            root.path().join(name).join("package.json"),
            format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
        )
        .unwrap();
    }
    fs::write(root.path().join("build.yaml"), "consumer:\n  uses: node/package\n  path: consumer\n  materialize:\n    - from: producer\n      to: inputs/package.tgz\nproducer:\n  uses: node/package\n  path: producer\n").unwrap();
    let temp = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), temp.path());
    let actions = plan["actions"].as_array().unwrap();
    let producer = actions
        .iter()
        .position(|a| a["id"] == "producer:package")
        .unwrap();
    let consumer = actions
        .iter()
        .position(|a| a["target"] == "consumer")
        .unwrap();
    assert!(producer < consumer);
    let input = actions[consumer]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == "artifact")
        .unwrap();
    assert_eq!(
        input,
        &json!({"kind":"artifact","artifact":"producer/primary","producer":"producer:package","mount":"consumer/inputs/package.tgz"})
    );
    assert!(
        input.get("digest").is_none(),
        "future bytes cannot have a fabricated digest"
    );
    assert!(actions[consumer]["dependsOn"]
        .as_array()
        .unwrap()
        .contains(&json!("producer:package")));
}

#[test]
fn single_target_build_honors_root_tasks_and_hooks_without_cross_target_fanout() {
    let root = tempfile::tempdir().unwrap();
    let capture = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    fs::write(root.path().join("oyzu.toml"), "[tasks.test]\nargv=['custom-test']\n[tasks.pre_test]\nargv=['custom-pre']\n[tasks.post_test]\nargv=['custom-post']\n").unwrap();
    let plan = planned(root.path(), capture.path());
    let actions = plan["actions"].as_array().unwrap();
    let selected: Vec<_> = actions
        .iter()
        .filter(|a| ["pre_test", "test", "post_test"].contains(&a["id"].as_str().unwrap()))
        .collect();
    assert_eq!(selected.len(), 3);
    assert_eq!(selected[1]["argv"], json!(["custom-test"]));
    assert_eq!(selected[1]["reports"].as_array().unwrap().len(), 2);
    assert!(selected[0]["reports"].as_array().unwrap().is_empty());
    assert!(selected[2]["reports"].as_array().unwrap().is_empty());
    assert!(!actions.iter().any(|a| a["id"] == "project:test"));
    fs::write(
        root.path().join("build.yaml"),
        "a:\n  uses: node/package\nb:\n  uses: node/package\n",
    )
    .unwrap();
    let capture = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), capture.path());
    let actions = plan["actions"].as_array().unwrap();
    assert!(!actions.iter().any(|a| a["id"] == "test"));
    assert!(actions.iter().any(|a| a["id"] == "a:test"));
    assert!(actions.iter().any(|a| a["id"] == "b:test"));
}

#[test]
fn node_override_reporting_recognizes_exact_commands_without_parsing_shell_programs() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    for (definition, instrumented) in [
        ("argv=['node','--test']", true),
        ("run='node --test'", true),
        ("argv=['npm','run','test']", true),
        ("run='npm run test'", true),
        ("run='node --test && node cleanup.mjs'", false),
        ("argv=['node','custom-tests.mjs']", false),
    ] {
        fs::write(
            root.path().join("oyzu.toml"),
            format!("[tasks.\"project:test\"]\n{definition}\n"),
        )
        .unwrap();
        let capture = tempfile::tempdir().unwrap();
        let plan = planned(root.path(), capture.path());
        let task = plan["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == "project:test")
            .unwrap();
        let argv = task["argv"].as_array().unwrap();
        assert_eq!(
            argv.contains(&json!("--test-reporter=junit")),
            instrumented,
            "{definition}"
        );
        assert_eq!(
            argv.contains(&json!("--test-reporter=lcov")),
            instrumented,
            "{definition}"
        );
        assert_eq!(task["reports"].as_array().unwrap().len(), 2);
    }
}

#[test]
fn declared_reports_bind_captured_task_cwd_and_preserve_other_required_kinds() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("api/checks")).unwrap();
    fs::write(
        root.path().join("build.yaml"),
        "api:\n  uses: node/app\n  path: api\n",
    )
    .unwrap();
    fs::write(
        root.path().join("api/package.json"),
        r#"{"name":"demo","version":"1.0.0","scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("oyzu.toml"),
        r#"
[tasks."api:test"]
argv=['custom-test']
cwd='checks'
reports=[{kind='test',format='junit',path='reports/tests.xml'}]
[tasks."api:post_test"]
argv=['custom-post']
"#,
    )
    .unwrap();
    let capture = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), capture.path());
    let actions = plan["actions"].as_array().unwrap();
    let task = actions.iter().find(|a| a["id"] == "api:test").unwrap();
    assert_eq!(task["reports"].as_array().unwrap().len(), 2);
    let report = task["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "test")
        .unwrap();
    let id = report["id"].as_str().unwrap();
    assert_eq!(
        task["extensions"]["oyzu.dev/report-inputs"][id],
        json!({"root":"workspace","path":"api/checks/reports/tests.xml"})
    );
    assert_eq!(
        task["env"]["OYZU_TEST_REPORT"],
        "/workspace/api/checks/reports/tests.xml"
    );
    assert_eq!(
        task["env"]["OYZU_COVERAGE_REPORT"],
        "/out/api/reports/coverage.lcov"
    );
    assert_eq!(
        task["extensions"]["oyzu.dev/collect-after"],
        "api:post_test"
    );
    let post = actions.iter().find(|a| a["id"] == "api:post_test").unwrap();
    assert_eq!(
        post["env"]["OYZU_TEST_REPORT"],
        task["env"]["OYZU_TEST_REPORT"]
    );
}

#[test]
fn operation_contracts_follow_tasks_first_executed_as_prerequisites() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("package.json"), r#"{"name":"demo","version":"1.0.0","scripts":{"build":"node build.mjs","test":"node --test"}}"#).unwrap();
    fs::write(
        root.path().join("oyzu.toml"),
        r#"
[tasks."project:build"]
argv=['node','build.mjs']
depends_on=['project:test']
[tasks."project:post_test"]
argv=['custom-post']
"#,
    )
    .unwrap();
    let capture = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), capture.path());
    let actions = plan["actions"].as_array().unwrap();
    let test = actions
        .iter()
        .position(|a| a["id"] == "project:test")
        .unwrap();
    let post = actions
        .iter()
        .position(|a| a["id"] == "project:post_test")
        .unwrap();
    let build = actions
        .iter()
        .position(|a| a["id"] == "project:build")
        .unwrap();
    assert!(test < post && post < build);
    assert_eq!(
        actions.iter().filter(|a| a["id"] == "project:test").count(),
        1
    );
    assert_eq!(actions[test]["reports"].as_array().unwrap().len(), 2);
    assert_eq!(
        actions[test]["extensions"]["oyzu.dev/collect-after"],
        "project:post_test"
    );
    assert_eq!(
        actions[post]["env"]["OYZU_TEST_REPORT"],
        "/out/project/reports/junit.xml"
    );
}
