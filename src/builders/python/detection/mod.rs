//! Independent manager detectors over a single parsed pyproject snapshot.
use crate::discovery::{
    detectors::{exclusive, Detector, Finding, Source},
    Resolution,
};
use anyhow::Result;
use std::path::Path;

pub(super) struct Profile {
    pub project: toml::Value,
    pub manager: Resolution,
}
struct ContextData {
    source: Source,
    project: toml::Value,
}
struct NativeManager {
    id: &'static str,
    manager: &'static str,
    lock: &'static str,
}
impl Detector<ContextData> for NativeManager {
    fn id(&self) -> &'static str {
        self.id
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let mut evidence = Vec::new();
        if let Some(e) = context.source.evidence(self.lock, "lockfile") {
            evidence.push(e);
        }
        if context
            .project
            .get("tool")
            .and_then(|v| v.get(self.manager))
            .is_some()
        {
            evidence.push(
                context
                    .source
                    .evidence("pyproject.toml", &format!("tool.{}", self.manager))
                    .unwrap(),
            );
        }
        Ok(if evidence.is_empty() {
            vec![]
        } else {
            vec![Finding::native(self.manager, evidence)]
        })
    }
}
struct PipDefault;
impl Detector<ContextData> for PipDefault {
    fn id(&self) -> &'static str {
        "python/default-manager"
    }
    fn detect(&self, _: &ContextData) -> Result<Vec<Finding>> {
        Ok(vec![Finding::fallback("pip")])
    }
}
static MANAGERS: &[&dyn Detector<ContextData>] = &[
    &NativeManager {
        id: "python/uv",
        manager: "uv",
        lock: "uv.lock",
    },
    &NativeManager {
        id: "python/poetry",
        manager: "poetry",
        lock: "poetry.lock",
    },
    &PipDefault,
];
pub(super) fn detect(root: &Path) -> Result<Profile> {
    let source = Source::read(root, &["pyproject.toml", "uv.lock", "poetry.lock"])?;
    let project = source
        .text("pyproject.toml")
        .map(toml::from_str)
        .transpose()?
        .unwrap_or(toml::Value::Table(Default::default()));
    let context = ContextData { source, project };
    let manager = exclusive("Python package manager", &context, MANAGERS)?;
    Ok(Profile {
        project: context.project,
        manager,
    })
}
