//! Compose native scripts and inferred checkers using the normal Node detectors.
use super::super::{scope::exclusions, Metadata};
use super::{Operation, Step};
use crate::{builders::node::detection, model::Task};
use anyhow::{bail, Result};

pub(super) fn steps(task: &Task, metadata: &Metadata, stage: &str) -> Result<Vec<Step>> {
    let aliases: &[&str] = match stage {
        "format-check" => &["format-check", "format:check"],
        "lint" => &["lint"],
        "format" => &["format"],
        _ => bail!("unsupported workspace quality operation {stage}"),
    };
    let mut result = Vec::new();
    for (name, path) in std::iter::once(("root", ".")).chain(
        metadata
            .members
            .iter()
            .map(|m| (m.name.as_str(), m.path.as_str())),
    ) {
        let profile = detection::detect(&task.cwd.join(path))?;
        let script = if path == "." {
            None
        } else {
            aliases
                .iter()
                .find(|s| profile.package["scripts"].get(**s).is_some())
        };
        let operation = if let Some(script) = script {
            Operation::Script {
                script: (*script).into(),
            }
        } else {
            let selected = if stage == "lint" {
                profile.linter.selected()
            } else {
                profile.formatter.selected()
            };
            if selected
                != if stage == "lint" {
                    "eslint"
                } else {
                    "prettier"
                }
            {
                bail!("{path}: implicit {selected} workspace quality integration is not implemented yet");
            }
            Operation::Quality {
                framework: profile.framework.selected().into(),
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
