//! Output-profile observations. Native build execution remains manager-owned.
use super::ContextData;
use crate::discovery::detectors::{Detector, Finding};
use anyhow::Result;

pub(super) const CONFIGS: &[&str] = &[
    "vite.config.js",
    "vite.config.ts",
    "vite.config.mjs",
    "vite.config.mts",
    "vite.config.cjs",
    "vite.config.cts",
];

struct Vite;
impl Detector<ContextData> for Vite {
    fn id(&self) -> &'static str {
        "node/output-vite"
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let words: Vec<_> = context.package["scripts"]["build"]
            .as_str()
            .unwrap_or("")
            .split_whitespace()
            .collect();
        if words.starts_with(&["vite", "build"]) {
            let mut evidence = vec![context
                .source
                .evidence("package.json", "/scripts/build")
                .unwrap()];
            evidence.extend(
                CONFIGS
                    .iter()
                    .filter_map(|file| context.source.evidence(file, "native Vite configuration")),
            );
            Ok(vec![Finding::declared("vite-application", evidence)])
        } else {
            Ok(vec![])
        }
    }
}
struct Package;
struct ConventionalApplication;
impl Detector<ContextData> for ConventionalApplication {
    fn id(&self) -> &'static str {
        "node/output-conventional-application"
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        if context.application_intent
            && context.package.get("workspaces").is_none()
            && context.package["scripts"]["build"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty())
        {
            Ok(vec![Finding::conventional(
                "dist-application",
                vec![context
                    .source
                    .evidence("package.json", "/scripts/build")
                    .unwrap()],
            )])
        } else {
            Ok(vec![])
        }
    }
}

impl Detector<ContextData> for Package {
    fn id(&self) -> &'static str {
        "node/output-package"
    }
    fn detect(&self, _context: &ContextData) -> Result<Vec<Finding>> {
        Ok(vec![Finding::fallback("native-package")])
    }
}
pub(super) static DETECTORS: &[&dyn Detector<ContextData>] =
    &[&Vite, &ConventionalApplication, &Package];
