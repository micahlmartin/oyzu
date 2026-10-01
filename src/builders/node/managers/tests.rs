use crate::{
    builders::{Builder, PlanningContext},
    dependencies::Prepared,
    discovery, snapshot,
};
use serde_json::json;
use std::{collections::BTreeMap, fs};

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
        assert_eq!(plan.package.argv[0], manager);
        assert!(plan.package.argv.iter().any(|a| a == pack_flag));
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
