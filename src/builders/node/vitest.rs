//! Native Vitest command/report integration, separate from detection and managers.
use super::super::strings;
use crate::model::Task;

pub(super) const DEFAULT: &[&str] = &["node", "node_modules/vitest/vitest.mjs", "run"];
pub(super) const COMMANDS: &[&str] = &["vitest", "vitest run"];

pub(super) fn recognized(script: &str) -> bool {
    COMMANDS.contains(&script)
}

pub(super) fn wrap(command: Vec<String>) -> Vec<String> {
    let mut argv = strings(&["node", "/oyzu/vitest.mjs"]);
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
        ["vitest"] | ["vitest", "run"] | ["sh", "-c", "vitest"] | ["sh", "-c", "vitest run"] => {
            strings(DEFAULT)
        }
        ["node", "node_modules/vitest/vitest.mjs", "run"] => strings(DEFAULT),
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
    use std::fs;

    #[test]
    fn inferred_vitest_keeps_native_scripts_and_custom_task_bodies() {
        for script in [
            None,
            Some("vitest"),
            Some("vitest run"),
            Some("node custom.mjs"),
        ] {
            let root = tempfile::tempdir().unwrap();
            let mut package = serde_json::json!({"name":"test-app","version":"1.0.0","devDependencies":{"vitest":"5.0.3","@vitest/coverage-v8":"5.0.3"}});
            if let Some(script) = script {
                package["scripts"]["test"] = script.into();
            }
            if script == Some("node custom.mjs") {
                package.as_object_mut().unwrap().remove("devDependencies");
                fs::write(
                    root.path().join("vitest.config.mjs"),
                    "export default {};\n",
                )
                .unwrap();
            }
            fs::write(root.path().join("package.json"), package.to_string()).unwrap();
            fs::write(
                root.path().join("package-lock.json"),
                r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
            )
            .unwrap();
            let temp = tempfile::tempdir().unwrap();
            let source = snapshot::capture(root.path(), &temp.path().join("source")).unwrap();
            let workspace = discovery::discover(&temp.path().join("source")).unwrap();
            let target = &workspace.targets["project"];
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
            assert!(workspace.tasks["project:test"].availability.is_none());
            assert_eq!(plan.tasks["test"].reports.len(), 2);
            let native = script.is_none_or(super::recognized);
            assert_eq!(
                plan.tasks["test"]
                    .argv
                    .starts_with(&["node".into(), "/oyzu/vitest.mjs".into()]),
                native
            );
            if script.is_none() {
                assert_eq!(workspace.tasks["project:test"].argv, super::DEFAULT);
            }
            if script == Some("node custom.mjs") {
                assert_eq!(plan.tasks["test"].argv, ["npm", "run", "test"]);
            }
            assert_eq!(
                super::super::Node
                    .instrument_override(
                        target,
                        &workspace.tasks["project:test"],
                        &Default::default()
                    )
                    .is_some(),
                native
            );
        }
    }
}
