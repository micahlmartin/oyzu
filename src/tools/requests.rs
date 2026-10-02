//! Effective configuration projection; version interpretation stays in the backend.
use crate::config::resolve::EffectiveConfig;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Immutable normalized requests and their derived digest. Construct through
/// projection or checked parsing; borrowed accessors cannot invalidate identity.
#[derive(Debug, Serialize)]
pub struct ToolRequestIdentity {
    requests: BTreeMap<String, String>,
    native_constraints: BTreeMap<String, Vec<String>>,
    required_capabilities: Vec<String>,
    digest: String,
}

impl ToolRequestIdentity {
    pub fn requests(&self) -> &BTreeMap<String, String> {
        &self.requests
    }

    pub fn native_constraints(&self) -> &BTreeMap<String, Vec<String>> {
        &self.native_constraints
    }

    pub fn required_capabilities(&self) -> &[String] {
        &self.required_capabilities
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Decode a closed normalized request record against a caller-trusted set of
    /// canonical catalog IDs. Recompute its digest with the same rules as config
    /// projection; never interpret version syntax or grant backend permission.
    /// JSON duplicates, unknown fields, unsorted sets and identity drift fail.
    pub fn parse(bytes: &[u8], admitted_ids: &BTreeSet<String>) -> Result<Self> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Record {
            requests: BTreeMap<String, String>,
            native_constraints: BTreeMap<String, Vec<String>>,
            required_capabilities: Vec<String>,
            digest: String,
        }
        let record: Record = serde_json::from_value(crate::config::policy::strict_json_limit(
            bytes,
            8 * 1024 * 1024,
        )?)?;
        ensure!(admitted_ids.len() <= 4096, "tool catalog exceeds limit");
        for id in admitted_ids {
            super::lock::canonical_id(id)?;
        }
        super::lock::digest(&record.digest)?;
        let admitted = admitted_ids.iter().map(String::as_str).collect();
        let normalized = normalize_requests(
            record.requests,
            &record.native_constraints,
            &record.required_capabilities,
            &admitted,
            0,
        )?;
        ensure!(
            normalized.native_constraints == record.native_constraints
                && normalized.required_capabilities == record.required_capabilities,
            "tool request sets are not normalized"
        );
        ensure!(
            normalized.digest == record.digest,
            "tool request identity mismatch"
        );
        Ok(normalized)
    }
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
        // Bound each copy here; the shared normalizer accounts for the full
        // request/constraint budget once the canonical request map is complete.
        text(version, &mut 0)?;
        ensure!(
            requests
                .insert(canonical.to_owned(), version.to_owned())
                .is_none(),
            "TOOL_ALIAS_AMBIGUOUS: multiple configured names identify the same tool"
        );
        ensure!(requests.len() <= 256, "tool root request limit exceeded");
    }
    normalize_requests(
        requests,
        native_constraints,
        required_capabilities,
        &admitted,
        budget,
    )
}

fn normalize_requests(
    requests: BTreeMap<String, String>,
    native_constraints: &BTreeMap<String, Vec<String>>,
    required_capabilities: &[String],
    admitted: &BTreeSet<&str>,
    mut budget: usize,
) -> Result<ToolRequestIdentity> {
    ensure!(requests.len() <= 256, "tool root request limit exceeded");
    for (canonical, version) in &requests {
        super::lock::canonical_id(canonical)?;
        ensure!(
            admitted.contains(canonical.as_str()),
            "request tool is not admitted in catalog"
        );
        text(version, &mut budget)?;
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
