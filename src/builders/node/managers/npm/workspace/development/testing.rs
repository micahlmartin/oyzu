//! Select native member tests; scripts retain npm lifecycle and argument semantics.
use super::super::{scope::exclusions, Metadata};
use super::{Operation, Step};
use crate::{builders::node::detection, model::Task};
use anyhow::{bail, Result};

pub(super) fn steps(task: &Task, metadata: &Metadata) -> Result<Vec<Step>> {
    let root = detection::detect(&task.cwd)?;
    let mut packages: Vec<_> = metadata
        .members
        .iter()
        .map(|member| (member.name.as_str(), member.path.as_str()))
        .collect();
    if root.package["private"] != true {
        packages.push(("root", "."));
    }
    let mut result = Vec::new();
    for (name, path) in packages {
        let profile = detection::detect(&task.cwd.join(path))?;
        let operation = if profile.package["scripts"]["test"].is_string() {
            Operation::Script {
                script: "test".into(),
            }
        } else {
            let framework = profile.framework.selected();
            if !["node-test", "jest", "vitest", "mocha"].contains(&framework) {
                bail!("{path}: implicit {framework} workspace test integration is not implemented yet");
            }
            Operation::Test {
                framework: framework.into(),
                excludes: exclusions(metadata, path),
            }
        };
        result.push(Step {
            name: name.into(),
            path: path.into(),
            operation,
        });
    }
    Ok(result)
}
