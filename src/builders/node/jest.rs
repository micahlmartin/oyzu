//! Jest command adaptation; native configuration and npm script lifecycles stay native.
use super::super::strings;
use crate::model::Task;

pub(super) const DEFAULT: &[&str] = &["node", "node_modules/jest/bin/jest.js", "--ci"];
pub(super) const COMMANDS: &[&str] = &["jest", "jest --ci"];

pub(super) fn recognized(script: &str) -> bool {
    COMMANDS.contains(&script)
}

pub(super) fn wrap(command: Vec<String>) -> Vec<String> {
    let mut argv = strings(&["node", "/oyzu/jest.mjs"]);
    argv.extend(command);
    argv
}

pub(super) fn instrument_override(task: &Task, native_script: bool) -> Option<Vec<String>> {
    if task.name != "test" {
        return None;
    }
    let argv: Vec<_> = task.argv.iter().map(String::as_str).collect();
    let command = match argv.as_slice() {
        ["jest"] | ["sh", "-c", "jest"] | ["jest", "--ci"] | ["sh", "-c", "jest --ci"] => {
            strings(DEFAULT)
        }
        ["node", "node_modules/jest/bin/jest.js", "--ci"] => strings(DEFAULT),
        ["npm", "run", "test"] | ["sh", "-c", "npm run test"] if native_script => {
            strings(&["npm", "run", "test", "--"])
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
    use std::fs;

    #[test]
    fn native_jest_plans_instrument_known_scripts_and_preserve_custom_commands() {
        let root = tempfile::tempdir().unwrap();
        for script in [None, Some("jest --ci"), Some("node custom-harness.cjs")] {
            let mut package = serde_json::json!({"name":"jest-project","version":"1.0.0","devDependencies":{"jest":"29.7.0"}});
            if let Some(script) = script {
                package["scripts"]["test"] = script.into();
            }
            fs::write(root.path().join("package.json"), package.to_string()).unwrap();
            fs::write(
                root.path().join("package-lock.json"),
                r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
            )
            .unwrap();
            let temp = tempfile::tempdir().unwrap();
            let source = snapshot::capture(root.path(), &temp.path().join("source")).unwrap();
            let ws = discovery::discover(&temp.path().join("source")).unwrap();
            let target = &ws.targets["project"];
            assert!(super::super::Node
                .plan(PlanningContext {
                    target,
                    source: &source,
                    dependencies: None
                })
                .is_err());
            let prepared = Prepared {
                root: temp.path().into(),
                digest: format!("sha256:{}", "a".repeat(64)),
                record: serde_json::json!({}),
            };
            let plan = super::super::Node
                .plan(PlanningContext {
                    target,
                    source: &source,
                    dependencies: Some(&prepared),
                })
                .unwrap();
            assert_eq!(plan.tasks["test"].reports.len(), 2);
            let argv = &plan.tasks["test"].argv;
            if script == Some("node custom-harness.cjs") {
                assert_eq!(argv, &["npm", "run", "test"]);
            } else {
                assert_eq!(&argv[..2], &["node", "/oyzu/jest.mjs"]);
                assert_eq!(
                    &argv[2..],
                    if script.is_some() {
                        &["npm", "run", "test", "--"]
                    } else {
                        super::DEFAULT
                    }
                );
            }
            let task = &ws.tasks["project:test"];
            assert_eq!(
                super::instrument_override(task, script.is_some_and(super::recognized)).is_some(),
                script != Some("node custom-harness.cjs")
            );
        }
    }
}
