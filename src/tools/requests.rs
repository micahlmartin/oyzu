//! Effective configuration projection; version interpretation stays in the backend.
use crate::config::resolve::EffectiveConfig;
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
pub struct ToolRequestIdentity {
    pub requests: BTreeMap<String, String>,
    pub native_constraints: BTreeMap<String, Vec<String>>,
    pub required_capabilities: Vec<String>,
    pub digest: String,
}

/// Project captured effective values using a caller-trusted, unambiguous alias
/// catalog. Never obtain this catalog or capabilities from project extension
/// fields. Native constraints come from builder-owned static evidence. This does
/// not interpret versions, admit a backend, enforce policy or resolve a lock.
pub fn project_tool_requests(
    effective: &EffectiveConfig,
    aliases: &BTreeMap<String, String>,
    native_constraints: &BTreeMap<String, Vec<String>>,
    required_capabilities: &[String],
) -> Result<ToolRequestIdentity> {
    ensure!(aliases.len() <= 4096, "tool alias catalog exceeds limit");
    let mut budget = 0usize;
    let mut admitted = BTreeSet::new();
    for (alias, canonical) in aliases {
        text(alias, &mut budget)?;
        text(canonical, &mut budget)?;
        super::lock::canonical_id(canonical)?;
        ensure!(
            !alias.contains(':') || alias == canonical,
            "canonical tool IDs cannot be rebound by an alias"
        );
        admitted.insert(canonical.as_str());
    }
    let mut requests = BTreeMap::new();
    for (name, value) in effective.values() {
        let Some(alias) = name.strip_prefix("tools.") else {
            continue;
        };
        if matches!(alias, "allowed" | "catalogs") {
            continue;
        }
        let canonical = aliases
            .get(alias)
            .map(String::as_str)
            .or_else(|| admitted.get(alias).copied())
            .context("TOOL_ALIAS_UNSUPPORTED: request is absent from the admitted alias catalog")?;
        let version = value
            .as_str()
            .context("tool request must be a literal version string")?;
        text(version, &mut budget)?;
        ensure!(
            requests
                .insert(canonical.to_owned(), version.to_owned())
                .is_none(),
            "TOOL_ALIAS_AMBIGUOUS: multiple configured names identify the same tool"
        );
        ensure!(requests.len() <= 256, "tool root request limit exceeded");
    }
    ensure!(
        native_constraints.len() <= 4096,
        "native tool constraint limit exceeded"
    );
    let mut constraints = BTreeMap::new();
    let mut count = 0usize;
    for (canonical, values) in native_constraints {
        super::lock::canonical_id(canonical)?;
        ensure!(
            admitted.contains(canonical.as_str()),
            "native constraint tool is not admitted in alias catalog"
        );
        ensure!(
            !values.is_empty(),
            "native tool constraints cannot be empty"
        );
        text(canonical, &mut budget)?;
        count += values.len();
        ensure!(count <= 16384, "native tool constraint count exceeds limit");
        for value in values {
            text(value, &mut budget)?;
        }
        let mut sorted = values.clone();
        sorted.sort();
        ensure!(
            sorted.windows(2).all(|pair| pair[0] != pair[1]),
            "duplicate native tool constraint"
        );
        constraints.insert(canonical.clone(), sorted);
    }
    ensure!(
        required_capabilities.len() <= 256,
        "tool capability limit exceeded"
    );
    for capability in required_capabilities {
        text(capability, &mut budget)?;
        ensure!(
            capability.len() <= 256
                && capability
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte)),
            "invalid tool capability ID"
        );
    }
    let mut capabilities = required_capabilities.to_vec();
    capabilities.sort();
    ensure!(
        capabilities.windows(2).all(|pair| pair[0] != pair[1]),
        "duplicate tool capability"
    );
    let digest = crate::records::digest(
        "oyzu.tool-requests.v2",
        &serde_json::json!({
            "requests": requests, "native_constraints": constraints, "required_capabilities": capabilities
        }),
    )?;
    Ok(ToolRequestIdentity {
        requests,
        native_constraints: constraints,
        required_capabilities: capabilities,
        digest,
    })
}

fn text(value: &str, budget: &mut usize) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 16 * 1024 && !value.chars().any(char::is_control),
        "invalid or oversized tool request text"
    );
    *budget = budget
        .checked_add(value.len())
        .context("tool request size overflow")?;
    ensure!(
        *budget <= 8 * 1024 * 1024,
        "tool request projection exceeds limit"
    );
    Ok(())
}
