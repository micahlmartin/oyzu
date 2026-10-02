//! Native quality evidence is independent of test-framework/manager selection.
use super::ContextData;
use crate::discovery::detectors::{Detector, Finding};
use anyhow::Result;

const ESLINT: &[&str] = &[
    "eslint.config.js",
    "eslint.config.mjs",
    "eslint.config.cjs",
    "eslint.config.ts",
    "eslint.config.mts",
    "eslint.config.cts",
    ".eslintrc",
    ".eslintrc.json",
    ".eslintrc.js",
    ".eslintrc.cjs",
    ".eslintrc.yml",
    ".eslintrc.yaml",
];
const PRETTIER: &[&str] = &[
    ".prettierrc",
    ".prettierrc.json",
    ".prettierrc.json5",
    ".prettierrc.yml",
    ".prettierrc.yaml",
    ".prettierrc.toml",
    ".prettierrc.js",
    ".prettierrc.cjs",
    ".prettierrc.mjs",
    ".prettierrc.ts",
    "prettier.config.js",
    "prettier.config.cjs",
    "prettier.config.mjs",
    "prettier.config.ts",
];
const BIOME: &[&str] = &["biome.json", "biome.jsonc"];

pub(super) fn inputs() -> impl Iterator<Item = &'static str> {
    ESLINT.iter().chain(PRETTIER).chain(BIOME).copied()
}

struct Quality {
    id: &'static str,
    name: &'static str,
    property: &'static str,
    files: &'static [&'static str],
    default: bool,
}
impl Detector<ContextData> for Quality {
    fn id(&self) -> &'static str {
        self.id
    }
    fn detect(&self, context: &ContextData) -> Result<Vec<Finding>> {
        let mut evidence: Vec<_> = self
            .files
            .iter()
            .filter_map(|p| context.source.evidence(p, "native quality configuration"))
            .collect();
        if context.package.get(self.property).is_some() {
            evidence.push(
                context
                    .source
                    .evidence("package.json", self.property)
                    .unwrap(),
            );
        }
        Ok(if !evidence.is_empty() {
            vec![Finding::native(self.name, evidence)]
        } else if self.default {
            vec![Finding::fallback(self.name)]
        } else {
            vec![]
        })
    }
}
pub(super) static LINTERS: &[&dyn Detector<ContextData>] = &[
    &Quality {
        id: "node/eslint",
        name: "eslint",
        property: "eslintConfig",
        files: ESLINT,
        default: true,
    },
    &Quality {
        id: "node/biome-lint",
        name: "biome",
        property: "biome",
        files: BIOME,
        default: false,
    },
];
pub(super) static FORMATTERS: &[&dyn Detector<ContextData>] = &[
    &Quality {
        id: "node/prettier",
        name: "prettier",
        property: "prettier",
        files: PRETTIER,
        default: true,
    },
    &Quality {
        id: "node/biome-format",
        name: "biome",
        property: "biome",
        files: BIOME,
        default: false,
    },
];
