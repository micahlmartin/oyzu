use crate::{
    builders::{Builder, PlanningContext},
    dependencies::Prepared,
    discovery, snapshot,
};
use serde_json::json;
use std::{collections::BTreeMap, fs};

#[test]
fn yarn_workspace_plans_every_member_and_required_stage_from_captured_facts() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        r#"{"name":"root","version":"0.1.0","private":true,"workspaces":["packages/*"]}"#,
    )
    .unwrap();
    fs::write(root.path().join("yarn.lock"), "# yarn lockfile v1\n").unwrap();
    let mut members = Vec::new();
    for name in ["app", "shared"] {
        let path = format!("packages/{name}");
        fs::create_dir_all(root.path().join(&path)).unwrap();
        fs::write(
            root.path().join(&path).join("package.json"),
            json!({"name":name,"version":"0.1.0","scripts":{"test":"node --test"}}).to_string(),
        )
        .unwrap();
        members.push(json!({"name":name,"path":path,"version":"0.1.0","private":false,"scripts":{"test":"node --test"},"dependencies":if name=="app" {vec![json!({"name":"shared","target":"shared","kind":"prod","spec":"0.1.0"})]} else {vec![]}}));
    }
    let captured = tempfile::tempdir().unwrap();
    let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
    let workspace = discovery::discover(&captured.path().join("source")).unwrap();
    let target = &workspace.targets["project"];
    assert!(target.tasks["build"].build_stage);
    let prepared = Prepared {
        root: captured.path().into(),
        digest: "sha256:prepared".into(),
        record: json!({"extensions":{"oyzu.dev/yarn":{"workspaces":{"schemaVersion":1,"members":members}}}}),
    };
    let plan = super::super::Node
        .plan(PlanningContext {
            target,
            source: &source,
            dependencies: Some(&prepared),
        })
        .unwrap();
    plan.validate().unwrap();
    assert_eq!(plan.artifacts.len(), 2);
    assert!(plan
        .artifacts
        .iter()
        .all(|a| a.version.as_ref().unwrap().contains("-dev.g")));
    assert_eq!(plan.tasks["test"].reports.len(), 4);
    for stage in ["build", "test", "lint", "format-check"] {
        assert!(plan.tasks.contains_key(stage));
    }
    assert_eq!(plan.prepare[0].argv[2], "project");
    assert_eq!(plan.prepare[1].argv[1], "/oyzu/yarn.mjs");
    assert_eq!(plan.package.argv[1], "/oyzu/yarn-workspace-build.mjs");
}

#[test]
fn manager_profiles_keep_native_packing_and_report_forwarding() {
    let examples =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/builds/node-managers");
    for (manager, lock, pack_flag) in [
        ("pnpm", "pnpm-lock.yaml", "--out"),
        ("yarn", "yarn.lock", "--filename"),
    ] {
        let root = tempfile::tempdir().unwrap();
        for name in ["package.json", lock] {
            fs::copy(examples.join(manager).join(name), root.path().join(name)).unwrap();
        }
        let capture = tempfile::tempdir().unwrap();
        let source = snapshot::capture(root.path(), &capture.path().join("source")).unwrap();
        let workspace = discovery::discover(&capture.path().join("source")).unwrap();
        let target = &workspace.targets["project"];
        let builder = super::super::Node;
        assert!(builder
            .plan(PlanningContext {
                target,
                source: &source,
                dependencies: None
            })
            .is_err());
        let prepared = Prepared {
            root: capture.path().into(),
            digest: format!("sha256:{}", "a".repeat(64)),
            record: json!({}),
        };
        let plan = builder
            .plan(PlanningContext {
                target,
                source: &source,
                dependencies: Some(&prepared),
            })
            .unwrap();
        if manager == "yarn" {
            assert_eq!(&plan.package.argv[..2], &["sh", "-c"]);
            assert!(
                plan.package.argv[2].contains("yarn --offline --non-interactive pack --filename")
            );
            assert!(plan.package.argv[2].contains("node /oyzu/node-archive.mjs"));
        } else {
            assert_eq!(plan.package.argv[0], manager);
            assert!(plan.package.argv.iter().any(|a| a == pack_flag));
        }
        assert!(plan
            .package
            .argv
            .last()
            .unwrap()
            .ends_with(&plan.artifacts[0].filename));
        assert_eq!(&plan.tasks["test"].argv[..3], &[manager, "run", "test"]);
        assert_eq!(plan.tasks["test"].argv[3], "--experimental-test-coverage");
        assert_eq!(plan.tasks["test"].reports.len(), 2);
        let env = BTreeMap::from([
            ("OYZU_TEST_REPORT".into(), "/out/custom.xml".into()),
            ("OYZU_COVERAGE_REPORT".into(), "/out/custom.lcov".into()),
        ]);
        let override_command = builder
            .instrument_override(target, &workspace.tasks["project:test"], &env)
            .unwrap();
        assert_eq!(&override_command[..3], &[manager, "run", "test"]);
        assert!(override_command
            .iter()
            .any(|arg| arg.contains("/out/custom.xml")));
        let mut variant = target.clone();
        variant.variant.insert("node".into(), "24.14.1".into());
        assert!(builder
            .variant_toolchain(&variant)
            .unwrap()
            .contains(&format!("/node:{manager}")));
        assert!(builder
            .variant_toolchain(&variant)
            .unwrap()
            .ends_with("-node24.14.1"));
        assert!(builder
            .plan(PlanningContext {
                target: &variant,
                source: &source,
                dependencies: Some(&prepared),
            })
            .err()
            .unwrap()
            .to_string()
            .contains("preflight evidence"));
        let namespace = format!("oyzu.dev/{manager}");
        let mut observed = Prepared {
            root: capture.path().into(),
            digest: prepared.digest.clone(),
            record: json!({"extensions":{namespace.clone(): {"nodeVersion":"22.14.0"}}}),
        };
        assert!(builder
            .plan(PlanningContext {
                target: &variant,
                source: &source,
                dependencies: Some(&observed),
            })
            .is_err());
        observed.record["extensions"][&namespace]["nodeVersion"] = json!("24.14.1");
        let variant_plan = builder
            .plan(PlanningContext {
                target: &variant,
                source: &source,
                dependencies: Some(&observed),
            })
            .unwrap();
        assert_eq!(variant_plan.package.argv, plan.package.argv);
        assert_eq!(variant_plan.tasks["test"].argv, plan.tasks["test"].argv);
        let package_file = capture.path().join("source/package.json");
        let mut package = crate::records::read(&package_file).unwrap();
        package["dependencies"]["uncaptured"] = "1.0.0".into();
        fs::write(package_file, package.to_string()).unwrap();
        assert!(builder
            .plan(PlanningContext {
                target,
                source: &source,
                dependencies: Some(&prepared)
            })
            .err()
            .expect("expected admission failure")
            .to_string()
            .contains("capture"));
    }
}
