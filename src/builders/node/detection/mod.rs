//! Node observations; selection belongs to the shared role resolver.
use crate::discovery::detectors::{exclusive, Detector, Finding, Source};
use crate::discovery::Resolution;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;

pub(super) struct Profile {
    pub package: Value,
    pub manager: Resolution,
    pub locked: bool,
}

struct ContextData {
    source: Source,
    package: Value,
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
static MANAGERS: &[&dyn Detector<ContextData>] = &[
    &NativeManager {
        id: "node/npm-lock",
        manager: "npm",
        lock: "package-lock.json",
    },
    &NativeManager {
        id: "node/pnpm-lock",
        manager: "pnpm",
        lock: "pnpm-lock.yaml",
    },
    &NativeManager {
        id: "node/yarn-lock",
        manager: "yarn",
        lock: "yarn.lock",
    },
    &DeclaredManager,
    &NpmDefault,
];

pub(super) fn detect(root: &Path) -> Result<Profile> {
    let source = Source::read(
        root,
        &[
            "package.json",
            "package-lock.json",
            "pnpm-lock.yaml",
            "yarn.lock",
        ],
    )?;
    let package: Value = serde_json::from_str(
        source
            .text("package.json")
            .context("missing package.json")?,
    )?;
    if !package.is_object() {
        bail!("package.json must be an object");
    }
    let context = ContextData { source, package };
    let manager = exclusive("Node package manager", &context, MANAGERS)?;
    let locked = ["package-lock.json", "pnpm-lock.yaml", "yarn.lock"]
        .iter()
        .any(|p| context.source.text(p).is_some());
    Ok(Profile {
        package: context.package,
        manager,
        locked,
    })
}
