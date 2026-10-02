//! Independent linter and formatter findings; native configuration beats defaults.
use super::ContextData;
use crate::discovery::detectors::{Detector, Finding};
use anyhow::Result;

struct Ruff;
impl Detector<ContextData> for Ruff {
    fn id(&self) -> &'static str {
        "python/ruff-lint"
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let mut evidence = Vec::new();
        if context
            .project
            .get("tool")
            .and_then(|v| v.get("ruff"))
            .is_some_and(|v| v.get("lint").is_some() || v.get("format").is_none())
        {
            evidence.push(
                context
                    .source
                    .evidence("pyproject.toml", "tool.ruff")
                    .unwrap(),
            );
        }
        for path in ["ruff.toml", ".ruff.toml"] {
            if let Some(text) = context.source.text(path) {
                let config: toml::Value = toml::from_str(text)?;
                if config.get("lint").is_some() || config.get("format").is_none() {
                    evidence.push(context.source.evidence(path, "/").unwrap());
                }
            }
        }
        Ok(vec![if evidence.is_empty() {
            Finding::fallback("ruff")
        } else {
            Finding::native("ruff", evidence)
        }])
    }
}

struct Flake8;
impl Detector<ContextData> for Flake8 {
    fn id(&self) -> &'static str {
        "python/flake8"
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let evidence = [".flake8", "setup.cfg", "tox.ini"]
            .iter()
            .filter_map(|path| {
                let text = context.source.text(path)?;
                text.lines()
                    .any(|line| {
                        let line = line.trim();
                        line.strip_prefix("[flake8]").is_some_and(|tail| {
                            let tail = tail.trim();
                            tail.is_empty() || tail.starts_with(['#', ';'])
                        })
                    })
                    .then(|| context.source.evidence(path, "flake8").unwrap())
            })
            .collect::<Vec<_>>();
        Ok(if evidence.is_empty() {
            vec![]
        } else {
            vec![Finding::native("flake8", evidence)]
        })
    }
}

struct Formatter {
    id: &'static str,
    name: &'static str,
}
impl Detector<ContextData> for Formatter {
    fn id(&self) -> &'static str {
        self.id
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let mut evidence = Vec::new();
        let table = context.project.get("tool").and_then(|v| v.get(self.name));
        let configured = if self.name == "ruff" {
            table.and_then(|v| v.get("format")).is_some()
        } else {
            table.is_some()
        };
        if configured {
            evidence.push(
                context
                    .source
                    .evidence("pyproject.toml", &format!("tool.{}", self.name))
                    .unwrap(),
            );
        }
        if self.name == "ruff" {
            for path in ["ruff.toml", ".ruff.toml"] {
                if let Some(text) = context.source.text(path) {
                    let config: toml::Value = toml::from_str(text)?;
                    if config.get("format").is_some() {
                        evidence.push(context.source.evidence(path, "format").unwrap());
                    }
                }
            }
        }
        Ok(if !evidence.is_empty() {
            vec![Finding::native(self.name, evidence)]
        } else if self.name == "ruff" {
            vec![Finding::fallback("ruff")]
        } else {
            vec![]
        })
    }
}

pub(super) static LINTERS: &[&dyn Detector<ContextData>] = &[&Ruff, &Flake8];
pub(super) static FORMATTERS: &[&dyn Detector<ContextData>] = &[
    &Formatter {
        id: "python/ruff-format",
        name: "ruff",
    },
    &Formatter {
        id: "python/black",
        name: "black",
    },
];
