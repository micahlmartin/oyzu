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
fn build_inventory_is_frozen_and_must_match_captured_source() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0","scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    let original = "app: {uses: node/package}\n";
    fs::write(root.path().join("build.yaml"), original).unwrap();
    let workspace = discovery::discover_with_shell(root.path(), Some("sh")).unwrap();
    let captured = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
    let images = BTreeMap::from([(
        "app".into(),
        Image {
            reference: "node:test".into(),
            digest: format!("sha256:{}", "1".repeat(64)),
            os: "linux".into(),
            arch: "amd64".into(),
        },
    )]);
    let before = build::plan(&workspace, &source, &images).unwrap();
    // Pure planning consumes the captured inventory, not this live file.
    fs::write(
        root.path().join("build.yaml"),
        "app: {uses: node/package, matrix: {node: ['22.14.0', '24.14.1']}}\n",
    )
    .unwrap();
    assert_eq!(build::plan(&workspace, &source, &images).unwrap(), before);
    let later = tempfile::tempdir().unwrap();
    let changed_source = snapshot::capture(root.path(), &later.path().join("source")).unwrap();
    assert!(build::plan(&workspace, &changed_source, &images)
        .unwrap_err()
        .to_string()
        .contains("changed between discovery and source capture"));
    fs::remove_file(root.path().join("build.yaml")).unwrap();
    let without = tempfile::tempdir().unwrap();
    let absent_source = snapshot::capture(root.path(), &without.path().join("source")).unwrap();
    assert!(build::plan(&workspace, &absent_source, &images).is_err());
    let inferred = discovery::discover_with_shell(root.path(), Some("sh")).unwrap();
    assert!(build::plan(&inferred, &source, &images)
        .unwrap_err()
        .to_string()
        .contains("changed between discovery and source capture"));
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
fn inspector_verifies_directory_inventory_without_modifying_the_bundle() {
    let root = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    fs::write(output.path().join("index.html"), "hello").unwrap();
    fs::create_dir(output.path().join("empty")).unwrap();
    let tree = snapshot::capture(output.path(), &root.path().join("site")).unwrap();
    records::write(&root.path().join("envelope.json"), &json!({})).unwrap();
    let mut manifest = json!({"kind":"build-manifest","status":"failed","planPath":null,
        "envelopePath":"envelope.json","envelopeDigest":snapshot::file_digest(&root.path().join("envelope.json")).unwrap(),
        "artifacts":[{"kind":"directory","path":"site","size":5,"digest":tree.digest,"entries":tree.entries}],"reports":[]});
    records::write(&root.path().join("manifest.json"), &manifest).unwrap();
    assert_eq!(build::inspect(root.path()).unwrap(), manifest);
    let inspected = std::process::Command::new(env!("CARGO_BIN_EXE_oyzu"))
        .arg("inspect")
        .arg(root.path())
        .output()
        .unwrap();
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&inspected.stdout).unwrap(),
        manifest
    );
    assert_eq!(
        fs::read(root.path().join("site/index.html")).unwrap(),
        b"hello"
    );
    manifest["artifacts"][0]["size"] = json!(6);
    records::write(&root.path().join("manifest.json"), &manifest).unwrap();
    assert!(build::inspect(root.path()).is_err());
    manifest["artifacts"][0]["size"] = json!(5);
    records::write(&root.path().join("manifest.json"), &manifest).unwrap();
    fs::write(root.path().join("site/new.html"), "extra").unwrap();
    assert!(build::inspect(root.path()).is_err());
    assert!(!std::process::Command::new(env!("CARGO_BIN_EXE_oyzu"))
        .arg("inspect")
        .arg(root.path())
        .output()
        .unwrap()
        .status
        .success());
    fs::remove_file(root.path().join("site/new.html")).unwrap();
    fs::remove_dir(root.path().join("site/empty")).unwrap();
    assert!(build::inspect(root.path()).is_err());
    manifest["artifacts"][0]["path"] = json!("../site");
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
fn plan_edges_preserve_target_order_without_serializing_independent_targets() {
    let root = tempfile::tempdir().unwrap();
    for id in ["alpha", "beta", "consumer"] {
        fs::create_dir(root.path().join(id)).unwrap();
        fs::write(
            root.path().join(id).join("package.json"),
            format!(r#"{{"name":"{id}","version":"1.0.0"}}"#),
        )
        .unwrap();
    }
    fs::write(root.path().join("build.yaml"),
        "alpha: {uses: node/package, path: alpha}\nbeta: {uses: node/package, path: beta}\nconsumer: {uses: node/package, path: consumer, depends_on: [alpha]}\n").unwrap();
    let captured = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), captured.path());
    let actions = plan["actions"].as_array().unwrap();
    for target in ["alpha", "beta", "consumer"] {
        let chain: Vec<_> = actions.iter().filter(|a| a["target"] == target).collect();
        for (index, action) in chain.iter().enumerate() {
            let mut expected = Vec::new();
            if index > 0 {
                expected.push(chain[index - 1]["id"].as_str().unwrap());
            }
            if target == "consumer" {
                expected.push("alpha:package");
            }
            expected.sort();
            assert_eq!(action["dependsOn"], json!(expected));
        }
    }
}

#[test]
fn cross_target_prerequisites_keep_owner_reports_and_hooks() {
    let root = tempfile::tempdir().unwrap();
    for id in ["alpha", "beta"] {
        fs::create_dir(root.path().join(id)).unwrap();
        fs::write(
            root.path().join(id).join("package.json"),
            format!(r#"{{"name":"{id}","version":"1.0.0","scripts":{{"build":"node -e 0"}}}}"#),
        )
        .unwrap();
    }
    fs::write(
        root.path().join("build.yaml"),
        "alpha: {uses: node/package, path: alpha}\nbeta: {uses: node/package, path: beta}\n",
    )
    .unwrap();
    fs::write(root.path().join("oyzu.toml"),
        "[tasks.\"alpha:pre_test\"]\nargv=['node','-e','0']\ndepends_on=['beta:test']\n[tasks.\"beta:post_test\"]\nargv=['node','-e','0']\n").unwrap();
    let capture = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), capture.path());
    let actions = plan["actions"].as_array().unwrap();
    let get = |id: &str| actions.iter().find(|a| a["id"] == id).unwrap();
    assert_eq!(actions.iter().filter(|a| a["id"] == "beta:test").count(), 1);
    assert_eq!(get("beta:test")["cwd"], "beta");
    assert_eq!(get("beta:test")["tools"], json!(["beta"]));
    assert_eq!(get("beta:test")["target"], "beta");
    assert_eq!(get("beta:test")["reports"].as_array().unwrap().len(), 2);
    assert_eq!(
        get("beta:test")["extensions"]["oyzu.dev/collect-after"],
        "beta:post_test"
    );
    assert!(get("alpha:pre_test")["dependsOn"]
        .as_array()
        .unwrap()
        .contains(&json!("beta:post_test")));
    assert!(get("beta:test")["env"]["OYZU_TEST_REPORT"]
        .as_str()
        .unwrap()
        .contains("beta/"));
    fs::write(root.path().join("oyzu.toml"),
        "[tasks.\"alpha:pre_test\"]\nargv=['node','-e','0']\ndepends_on=['beta:verify']\n[tasks.\"beta:verify\"]\nargv=['node','-e','0']\ndepends_on=['helper']\n[tasks.helper]\nargv=['node','-e','0']\n").unwrap();
    let capture = tempfile::tempdir().unwrap();
    let inherited = planned(root.path(), capture.path());
    let helper = inherited["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "helper")
        .unwrap();
    assert_eq!(helper["target"], "beta");
    assert_eq!(helper["tools"], json!(["beta"]));
    // Root tasks retain their declared root cwd.
    assert_eq!(helper["cwd"], ".");
    // Task-only dependencies are acyclic, but native build-before-test order
    // makes these two prerequisites cyclic. Reject before execution.
    fs::write(root.path().join("oyzu.toml"),
        "[tasks.\"alpha:build\"]\nargv=['node','-e','0']\ndepends_on=['beta:test']\n[tasks.\"beta:build\"]\nargv=['node','-e','0']\ndepends_on=['alpha:test']\n").unwrap();
    let captured = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
    let workspace = discovery::discover(&captured.path().join("source")).unwrap();
    let image = Image {
        reference: "node:test".into(),
        digest: format!("sha256:{}", "1".repeat(64)),
        os: "linux".into(),
        arch: "amd64".into(),
    };
    let images = BTreeMap::from([("alpha".into(), image.clone()), ("beta".into(), image)]);
    assert!(build::plan(&workspace, &source, &images)
        .unwrap_err()
        .to_string()
        .contains("action dependency cycle"));
    fs::write(root.path().join("oyzu.toml"),
        "[tasks.\"alpha:pre_test\"]\nargv=['node','-e','0']\ndepends_on=['helper']\n[tasks.\"beta:pre_test\"]\nargv=['node','-e','0']\ndepends_on=['helper']\n[tasks.helper]\nargv=['node','-e','0']\n").unwrap();
    let captured = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
    let workspace = discovery::discover(&captured.path().join("source")).unwrap();
    assert!(build::plan(&workspace, &source, &images)
        .unwrap_err()
        .to_string()
        .contains("shared root task has ambiguous build ownership"));
}

#[test]
fn local_prerequisite_sequence_cannot_overtake_an_earlier_build_stage() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"ordered","version":"1.0.0","scripts":{"build":"node -e 0"}}"#,
    )
    .unwrap();
    fs::write(root.path().join("oyzu.toml"),
        "[tasks.\"project:pre_test\"]\nargv=['node','-e','0']\ndepends_on=['project:first','project:second']\n[tasks.\"project:first\"]\nargv=['node','-e','0']\n[tasks.\"project:second\"]\nargv=['node','-e','0']\n").unwrap();
    let capture = tempfile::tempdir().unwrap();
    let plan = planned(root.path(), capture.path());
    let actions = plan["actions"].as_array().unwrap();
    for (consumer, producer) in [
        ("project:first", "project:build"),
        ("project:second", "project:first"),
        ("project:pre_test", "project:second"),
    ] {
        let action = actions.iter().find(|a| a["id"] == consumer).unwrap();
        assert!(
            action["dependsOn"]
                .as_array()
                .unwrap()
                .contains(&json!(producer)),
            "{producer} must precede {consumer}"
        );
    }
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
    for name in ["a", "b"] {
        fs::create_dir(root.path().join(name)).unwrap();
        fs::copy(
            root.path().join("package.json"),
            root.path().join(name).join("package.json"),
        )
        .unwrap();
    }
    fs::write(
        root.path().join("build.yaml"),
        "a:\n  uses: node/package\n  path: a\nb:\n  uses: node/package\n  path: b\n",
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
cwd='api/checks'
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

#[test]
fn cli_rejects_unknown_build_target_before_toolchain_resolution() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"demo","version":"1.0.0"}"#,
    )
    .unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_oyzu"))
        .arg("-C")
        .arg(root.path())
        .args(["build", "missing", "--image", "npm=not-provisioned:test"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let manifest: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(manifest["status"], "failed");
    assert_eq!(manifest["actions"], json!([]));
    assert!(manifest["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("unknown build target missing"));
    build::inspect(&root.path().join("dist")).unwrap();
}

#[test]
fn inspector_binds_selection_to_the_frozen_plan() {
    let root = tempfile::tempdir().unwrap();
    let selection = json!({"mode":"explicit","requested":["api"],"selected":["api"],"excluded":[{"target":"other","reason":"outside-selection"}]});
    let plan = json!({"extensions":{"oyzu.dev/selection":selection}});
    records::write(&root.path().join("plan.json"), &plan).unwrap();
    records::write(&root.path().join("envelope.json"), &json!({})).unwrap();
    let mut manifest = json!({"kind":"build-manifest","status":"failed","planPath":"plan.json",
        "planDigest":records::digest("oyzu.plan.v1alpha1", &plan).unwrap(),
        "envelopePath":"envelope.json","envelopeDigest":snapshot::file_digest(&root.path().join("envelope.json")).unwrap(),
        "artifacts":[],"reports":[],"extensions":{"oyzu.dev/selection":selection}});
    records::write(&root.path().join("manifest.json"), &manifest).unwrap();
    build::inspect(root.path()).unwrap();
    manifest["extensions"]["oyzu.dev/selection"]["selected"] = json!(["api", "other"]);
    records::write(&root.path().join("manifest.json"), &manifest).unwrap();
    assert!(build::inspect(root.path())
        .unwrap_err()
        .to_string()
        .contains("selection differs"));
}
