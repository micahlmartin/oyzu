//! Native manager declarations and lock ownership.
use super::ContextData;
use crate::discovery::detectors::{Detector, Finding};
use anyhow::{bail, Context, Result};

struct NativeManager {
    id: &'static str,
    manager: &'static str,
    path: &'static str,
    location: &'static str,
}
impl Detector<ContextData> for NativeManager {
    fn id(&self) -> &'static str {
        self.id
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let mut evidence = Vec::new();
        if let Some(e) = context.source.evidence(self.path, self.location) {
            evidence.push(e);
        }
        Ok(if evidence.is_empty() {
            vec![]
        } else {
            vec![Finding::native(self.manager, evidence)]
        })
    }
}
struct DeclaredManager;
impl Detector<ContextData> for DeclaredManager {
    fn id(&self) -> &'static str {
        "node/declared-manager"
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let Some(value) = context.package.get("packageManager") else {
            return Ok(vec![]);
        };
        let declared = value.as_str().context("packageManager must be a string")?;
        let manager = declared.split('@').next().unwrap_or("");
        if !["npm", "pnpm", "yarn"].contains(&manager) {
            bail!("unsupported Node manager {manager}");
        }
        Ok(vec![Finding::native(
            manager,
            vec![context
                .source
                .evidence("package.json", "/packageManager")
                .unwrap()],
        )])
    }
}
struct NpmDefault;
impl Detector<ContextData> for NpmDefault {
    fn id(&self) -> &'static str {
        "node/default-manager"
    }
    fn detect(&self, _: &ContextData) -> Result<Vec<Finding>> {
        Ok(vec![Finding::fallback("npm")])
    }
}
pub(super) static MANAGERS: &[&dyn Detector<ContextData>] = &[
    &NativeManager {
        id: "node/npm-shrinkwrap",
        manager: "npm",
        path: "npm-shrinkwrap.json",
        location: "lockfile",
    },
    &NativeManager {
        id: "node/npm-lock",
        manager: "npm",
        path: "package-lock.json",
        location: "lockfile",
    },
    &NativeManager {
        id: "node/pnpm-lock",
        manager: "pnpm",
        path: "pnpm-lock.yaml",
        location: "lockfile",
    },
    &NativeManager {
        id: "node/pnpm-workspace",
        manager: "pnpm",
        path: "pnpm-workspace.yaml",
        location: "workspace",
    },
    &NativeManager {
        id: "node/yarn-lock",
        manager: "yarn",
        path: "yarn.lock",
        location: "lockfile",
    },
    &DeclaredManager,
    &NpmDefault,
];
