//! Framework observations do not execute config files or rewrite native scripts.
use super::ContextData;
use crate::discovery::detectors::{Detector, Finding};
use anyhow::{Context, Result};

const JEST_CONFIGS: &[&str] = &[
    "jest.config.js",
    "jest.config.cjs",
    "jest.config.mjs",
    "jest.config.ts",
    "jest.config.json",
];
const VITEST_CONFIGS: &[&str] = &[
    "vitest.config.js",
    "vitest.config.ts",
    "vitest.config.mjs",
    "vitest.config.mts",
    "vitest.config.cjs",
    "vitest.config.cts",
];

pub(super) fn inputs() -> impl Iterator<Item = &'static str> {
    JEST_CONFIGS.iter().chain(VITEST_CONFIGS).copied()
}

struct NodeTest;
impl Detector<ContextData> for NodeTest {
    fn id(&self) -> &'static str {
        "node/test-native"
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        Ok(
            if context.package["scripts"]["test"].as_str() == Some("node --test") {
                vec![Finding::declared(
                    "node-test",
                    vec![context
                        .source
                        .evidence("package.json", "/scripts/test")
                        .unwrap()],
                )]
            } else {
                vec![]
            },
        )
    }
}

struct Framework {
    id: &'static str,
    name: &'static str,
    commands: &'static [&'static str],
    configs: &'static [&'static str],
}
impl Detector<ContextData> for Framework {
    fn id(&self) -> &'static str {
        self.id
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let mut findings = Vec::new();
        if context.package["scripts"]["test"]
            .as_str()
            .is_some_and(|c| self.commands.contains(&c))
        {
            findings.push(Finding::declared(
                self.name,
                vec![context
                    .source
                    .evidence("package.json", "/scripts/test")
                    .unwrap()],
            ));
        }
        let mut evidence: Vec<_> = self
            .configs
            .iter()
            .filter_map(|p| context.source.evidence(p, "framework configuration"))
            .collect();
        if context.package.get(self.name).is_some() {
            evidence.push(
                context
                    .source
                    .evidence("package.json", &format!("/{}", self.name))
                    .unwrap(),
            );
        }
        if !evidence.is_empty() {
            findings.push(Finding::native(self.name, evidence));
        }
        let dependencies: Vec<_> = ["dependencies", "devDependencies", "optionalDependencies"]
            .iter()
            .filter(|field| context.package[**field].get(self.name).is_some())
            .map(|field| {
                context
                    .source
                    .evidence("package.json", &format!("/{field}/{}", self.name))
                    .unwrap()
            })
            .collect();
        if !dependencies.is_empty() {
            findings.push(Finding::conventional(self.name, dependencies));
        }
        Ok(findings)
    }
}

struct DefaultTest;
impl Detector<ContextData> for DefaultTest {
    fn id(&self) -> &'static str {
        "node/default-test"
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let script = context.package.get("scripts").and_then(|s| s.get("test"));
        if let Some(script) = script {
            script
                .as_str()
                .context("native test script must be a string")?;
        }
        Ok(vec![Finding::fallback(if script.is_some() {
            "custom"
        } else {
            "node-test"
        })])
    }
}

pub(super) static DETECTORS: &[&dyn Detector<ContextData>] = &[
    &NodeTest,
    &Framework {
        id: "node/test-jest",
        name: "jest",
        commands: &["jest", "jest --ci"],
        configs: JEST_CONFIGS,
    },
    &Framework {
        id: "node/test-vitest",
        name: "vitest",
        commands: &["vitest", "vitest run"],
        configs: VITEST_CONFIGS,
    },
    &DefaultTest,
];
