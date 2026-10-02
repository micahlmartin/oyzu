//! Observe native pytest intent; pytest itself owns configuration and collection.
use super::ContextData;
use crate::discovery::detectors::{Detector, Finding};
use anyhow::Result;

struct Pytest;
impl Detector<ContextData> for Pytest {
    fn id(&self) -> &'static str {
        "python/pytest"
    }

    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let mut evidence = Vec::new();
        for path in ["pytest.ini", ".pytest.ini"] {
            if let Some(found) = context.source.evidence(path, "/") {
                evidence.push(found);
            }
        }
        if context
            .project
            .get("tool")
            .and_then(|value| value.get("pytest"))
            .and_then(|value| value.get("ini_options"))
            .is_some()
        {
            evidence.push(
                context
                    .source
                    .evidence("pyproject.toml", "tool.pytest.ini_options")
                    .unwrap(),
            );
        }
        for (path, section) in [("tox.ini", "[pytest]"), ("setup.cfg", "[tool:pytest]")] {
            if context.source.text(path).is_some_and(|text| {
                text.lines().any(|line| {
                    line.trim().strip_prefix(section).is_some_and(|tail| {
                        let tail = tail.trim();
                        tail.is_empty() || tail.starts_with(['#', ';'])
                    })
                })
            }) {
                evidence.push(context.source.evidence(path, section).unwrap());
            }
        }
        Ok(vec![if evidence.is_empty() {
            Finding::fallback("pytest")
        } else {
            Finding::native("pytest", evidence)
        }])
    }
}

pub(super) static FRAMEWORKS: &[&dyn Detector<ContextData>] = &[&Pytest];
