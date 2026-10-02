use super::*;
use crate::{discovery, snapshot};
use serde_json::json;
use std::fs;

fn chart(root: &Path, path: &str, dependencies: &str) {
    let directory = root.join(path);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("Chart.yaml"),
        format!("apiVersion: v2\nname: example\nversion: 1.2.3\n{dependencies}"),
    )
    .unwrap();
}

#[test]
fn conventional_nested_chart_discovers_tasks_without_configuration() {
    let root = tempfile::tempdir().unwrap();
    chart(root.path(), "chart", "");
    let workspace = discovery::discover(root.path()).unwrap();
    assert_eq!(workspace.targets["project"].builder, "helm/chart");
    assert_eq!(
        workspace.tasks["project:test"].argv,
        ["helm", "template", "oyzu-check", "chart"]
    );
    chart(root.path(), ".", "");
    assert!(metadata::chart_path(root.path()).is_err());
}

#[test]
fn dependencies_are_contained_ordered_and_reject_cycles_or_remote_sources() {
    let root = tempfile::tempdir().unwrap();
    chart(
        root.path(),
        "chart",
        "dependencies:\n- name: labels\n  version: 1.2.3\n  repository: file://../labels\n",
    );
    chart(root.path(), "labels", "");
    let order = metadata::local_order(root.path(), &root.path().join("chart")).unwrap();
    assert_eq!(order[0], root.path().join("labels").canonicalize().unwrap());
    chart(
        root.path(),
        "labels",
        "dependencies:\n- name: parent\n  repository: file://../chart\n",
    );
    assert!(
        metadata::local_order(root.path(), &root.path().join("chart"))
            .unwrap_err()
            .to_string()
            .contains("cyclic")
    );
    chart(
        root.path(),
        "labels",
        "dependencies:\n- name: external\n  repository: https://example.invalid/charts\n",
    );
    assert!(
        metadata::local_order(root.path(), &root.path().join("chart"))
            .unwrap_err()
            .to_string()
            .contains("approved registry")
    );
    chart(
        root.path(),
        "labels",
        "dependencies:\n- name: escape\n  repository: file://../../\n",
    );
    assert!(
        metadata::local_order(root.path(), &root.path().join("chart"))
            .unwrap_err()
            .to_string()
            .contains("escapes")
    );
}

#[test]
fn packaging_plans_snapshots_and_retains_native_checks() {
    let root = tempfile::tempdir().unwrap();
    chart(root.path(), "chart", "");
    let source = tempfile::tempdir().unwrap();
    let snapshot = snapshot::capture(root.path(), &source.path().join("captured")).unwrap();
    let workspace = discovery::discover(&source.path().join("captured")).unwrap();
    let prepared = Prepared {
        root: root.path().into(),
        digest: snapshot.digest.clone(),
        record: json!({}),
    };
    let plan = planning::plan(PlanningContext {
        target: &workspace.targets["project"],
        source: &snapshot,
        dependencies: Some(&prepared),
    })
    .unwrap();
    plan.validate().unwrap();
    assert_eq!(plan.artifacts[0].filename, "example-1.2.3.tgz");
    assert!(plan.tasks["lint"].argv.contains(&"--with-subcharts".into()));
    assert!(plan.tasks["test"]
        .argv
        .contains(&"/oyzu/helm-test.py".into()));
    assert_eq!(plan.tasks["test"].reports.len(), 1);
    assert_eq!(
        plan.tasks["test"].reports[0].format,
        crate::reports::Format::Junit
    );
    assert_eq!(
        serde_json::to_value(&plan.coverage).unwrap()["status"],
        "inapplicable"
    );
    assert!(plan.tasks["build"]
        .argv
        .iter()
        .any(|arg| arg.contains("helm package")));
    assert_eq!(plan.env["KUBECONFIG"], "/dev/null");
}

#[test]
fn library_charts_package_and_lint_without_attempting_installable_rendering() {
    let root = tempfile::tempdir().unwrap();
    chart(root.path(), "chart", "type: library\n");
    let source = tempfile::tempdir().unwrap();
    let snapshot = snapshot::capture(root.path(), &source.path().join("captured")).unwrap();
    let workspace = discovery::discover(&source.path().join("captured")).unwrap();
    assert_eq!(workspace.tasks["project:test"].argv[1], "lint");
    let prepared = Prepared {
        root: root.path().into(),
        digest: snapshot.digest.clone(),
        record: json!({}),
    };
    let plan = planning::plan(PlanningContext {
        target: &workspace.targets["project"],
        source: &snapshot,
        dependencies: Some(&prepared),
    })
    .unwrap();
    assert_eq!(plan.artifacts.len(), 1);
    assert_eq!(plan.artifacts[0].name, "chart");
    assert!(plan.tasks["test"].argv.contains(&"library".into()));
    assert_eq!(plan.tasks["test"].reports.len(), 1);
    assert!(plan.tasks.contains_key("lint"));
    assert_eq!(plan.package.argv[0], "cp");
}

#[test]
fn native_unittest_suites_select_the_plugin_and_preserve_both_report_obligations() {
    let root = tempfile::tempdir().unwrap();
    chart(root.path(), "chart", "");
    fs::create_dir(root.path().join("chart/tests")).unwrap();
    // Native parsing belongs to execution, not static evidence capture.
    fs::write(
        root.path().join("chart/tests/deployment_test.yaml"),
        "suite: [",
    )
    .unwrap();
    let source = tempfile::tempdir().unwrap();
    let snapshot = snapshot::capture(root.path(), &source.path().join("captured")).unwrap();
    let workspace = discovery::discover(&source.path().join("captured")).unwrap();
    let target = &workspace.targets["project"];
    assert_eq!(
        target.discovery["test-framework"].selected(),
        "helm-unittest"
    );
    assert_eq!(
        target.tasks["test"].argv,
        ["helm", "unittest", "--strict", "chart"]
    );
    let prepared = Prepared {
        root: root.path().into(),
        digest: snapshot.digest.clone(),
        record: json!({}),
    };
    let plan = planning::plan(PlanningContext {
        target,
        source: &snapshot,
        dependencies: Some(&prepared),
    })
    .unwrap();
    plan.validate().unwrap();
    assert_eq!(plan.tasks["test"].reports.len(), 2);
    assert!(plan.tasks["test"]
        .argv
        .contains(&"--unittest-report".into()));
    assert_eq!(plan.fixed_env["HELM_PLUGINS"], "/opt/oyzu-helm-plugins");
    assert!(plan
        .package
        .argv
        .contains(&"/out/project/artifacts/rendered.yaml".into()));
}

#[test]
fn subchart_only_suites_are_static_bounded_and_preserve_nested_evidence() {
    let root = tempfile::tempdir().unwrap();
    chart(root.path(), ".", "");
    chart(root.path(), "charts/child/charts/grandchild", "");
    chart(root.path(), "charts/child", "");
    let relative = "charts/child/charts/grandchild/tests/example_test.yaml";
    let suite = root.path().join(relative);
    fs::create_dir_all(suite.parent().unwrap()).unwrap();
    fs::write(&suite, "suite: [").unwrap();
    let first = detection::detect(root.path()).unwrap();
    assert_eq!(first.selected(), "helm-unittest");
    assert_eq!(first, detection::detect(root.path()).unwrap());
    assert!(serde_json::to_string(&first).unwrap().contains(relative));
    fs::write(&suite, "suite: changed").unwrap();
    assert_ne!(first, detection::detect(root.path()).unwrap());
    fs::remove_file(root.path().join("charts/child/Chart.yaml")).unwrap();
    assert_eq!(
        detection::detect(root.path()).unwrap().selected(),
        "helm-validation"
    );

    chart(root.path(), "charts/child", "");
    for index in 0..128 {
        fs::write(
            suite
                .parent()
                .unwrap()
                .join(format!("extra{index}_test.yaml")),
            "",
        )
        .unwrap();
    }
    assert!(detection::detect(root.path())
        .unwrap_err()
        .to_string()
        .contains("suite count"));
}

#[test]
fn deeply_nested_subchart_discovery_is_bounded() {
    let root = tempfile::tempdir().unwrap();
    chart(root.path(), ".", "");
    let mut relative = String::new();
    for _ in 0..17 {
        relative.push_str("charts/a/");
        chart(root.path(), &relative, "");
    }
    assert!(detection::detect(root.path())
        .unwrap_err()
        .to_string()
        .contains("depth"));
}

#[cfg(unix)]
#[test]
fn subchart_discovery_does_not_follow_symbolic_links() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    chart(root.path(), ".", "");
    chart(outside.path(), ".", "");
    fs::create_dir(root.path().join("charts")).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("charts/external")).unwrap();
    assert!(detection::detect(root.path())
        .unwrap_err()
        .to_string()
        .contains("symlink"));
}
