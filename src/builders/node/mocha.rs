//! Mocha invocation/report adaptation; native configuration remains runtime-owned.
use super::super::strings;
use crate::model::Task;

pub(super) const DEFAULT: &[&str] = &["node", "node_modules/mocha/bin/mocha.js"];
pub(super) const COMMANDS: &[&str] = &["mocha"];

pub(super) fn recognized(script: &str) -> bool {
    COMMANDS.contains(&script)
}

pub(super) fn configure_reporting(plan: &mut crate::builders::BuilderPlan) {
    let key = "OYZU_NODE_REPORTING_HOME";
    let value = "/opt/oyzu-node-quality";
    plan.env.insert(key.into(), value.into());
    plan.fixed_env.insert(key.into(), value.into());
}

pub(super) fn wrap(command: Vec<String>) -> Vec<String> {
    let mut argv = strings(&["node", "/oyzu/mocha.mjs"]);
    argv.extend(command);
    argv
}

pub(super) fn instrument_override(
    task: &Task,
    native_script: bool,
    manager: &str,
) -> Option<Vec<String>> {
    if task.name != "test" {
        return None;
    }
    let argv: Vec<_> = task.argv.iter().map(String::as_str).collect();
    let command = match argv.as_slice() {
        ["mocha"] | ["sh", "-c", "mocha"] | ["node", "node_modules/mocha/bin/mocha.js"] => {
            strings(DEFAULT)
        }
        [command, "run", "test"] if native_script && *command == manager => {
            super::managers::get(manager).ok()?.script("test", true)
        }
        ["sh", "-c", command] if native_script && *command == format!("{manager} run test") => {
            super::managers::get(manager).ok()?.script("test", true)
        }
        _ => return None,
    };
    Some(wrap(command))
}

#[cfg(test)]
mod tests {
    use crate::{
        builders::{Builder, PlanningContext},
        dependencies::Prepared,
        discovery, snapshot,
    };
    use serde_json::json;
    use std::fs;

    #[test]
    fn workspace_plan_preserves_package_report_ownership() {
        let root = tempfile::tempdir().unwrap();
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tooling/fixtures/mocha-workspace");
        let source = snapshot::capture(&fixture, &root.path().join("source")).unwrap();
        let workspace = discovery::discover(&root.path().join("source")).unwrap();
        let members: Vec<_> = ["implicit", "scripted"]
            .iter()
            .map(|name| {
                let package =
                    crate::records::read(&fixture.join(format!("packages/{name}/package.json")))
                        .unwrap();
                json!({"name":package["name"], "path":format!("packages/{name}"),
                    "version":"1.0.0", "private":false, "dependencies":[],
                    "scripts":package.get("scripts").cloned().unwrap_or(json!({}))})
            })
            .collect();
        let dependency = Prepared {
            root: root.path().join("dependencies"),
            digest: format!("sha256:{}", "1".repeat(64)),
            record: json!({"extensions":{"oyzu.dev/npm":{"workspaces":{"schemaVersion":1,"members":members}}}}),
        };
        let plan = super::super::Node
            .plan(PlanningContext {
                target: &workspace.targets["project"],
                source: &source,
                dependencies: Some(&dependency),
            })
            .unwrap();
        plan.validate().unwrap();
        assert_eq!(plan.artifacts.len(), 3);
        assert_eq!(plan.tasks["test"].reports.len(), 6);
        assert_eq!(
            plan.fixed_env["OYZU_NODE_REPORTING_HOME"],
            "/opt/oyzu-node-quality"
        );
        let specification: serde_json::Value = serde_json::from_str(
            plan.prepare
                .iter()
                .find(|command| command.operation == "version")
                .unwrap()
                .argv
                .last()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(specification["rootFramework"], "mocha");
        assert!(specification["modules"]
            .as_array()
            .unwrap()
            .iter()
            .all(|member| member["framework"] == "mocha"));
    }

    #[test]
    fn native_mocha_contract_keeps_required_reports_and_custom_task_bodies() {
        for script in [None, Some("mocha"), Some("node custom.mjs")] {
            let root = tempfile::tempdir().unwrap();
            let mut package = json!({"name":"mocha-example", "devDependencies":{"mocha":"11.8.0"}});
            if let Some(script) = script {
                package["scripts"]["test"] = script.into();
            }
            fs::write(root.path().join("package.json"), package.to_string()).unwrap();
            fs::write(
                root.path().join(".mocharc.cjs"),
                "throw Error('static discovery must not execute');",
            )
            .unwrap();
            fs::write(
                root.path().join("package-lock.json"),
                r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
            )
            .unwrap();
            let temp = tempfile::tempdir().unwrap();
            let source = snapshot::capture(root.path(), &temp.path().join("source")).unwrap();
            let workspace = discovery::discover(&temp.path().join("source")).unwrap();
            let target = &workspace.targets["project"];
            assert_eq!(target.discovery["test-framework"].selected(), "mocha");
            let dependency = Prepared {
                root: temp.path().join("dependencies"),
                digest: format!("sha256:{}", "1".repeat(64)),
                record: json!({}),
            };
            let plan = super::super::Node
                .plan(PlanningContext {
                    target,
                    source: &source,
                    dependencies: Some(&dependency),
                })
                .unwrap();
            plan.validate().unwrap();
            assert_eq!(plan.tasks["test"].reports.len(), 2);
            assert_eq!(
                plan.tasks["test"]
                    .argv
                    .iter()
                    .any(|a| a == "/oyzu/mocha.mjs"),
                script != Some("node custom.mjs")
            );
        }
    }
}
