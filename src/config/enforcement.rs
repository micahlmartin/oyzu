//! Enforcement at execution boundaries; unsupported protected routing never falls back.
use super::resolve::EffectiveConfig;
use anyhow::{bail, Result};
use serde_json::Value;
pub(crate) fn execution_preflight(config: &EffectiveConfig, tools: &[&str]) -> Result<()> {
    tool_eligibility(config, tools)?;
    execution_routes(config)
}

/// Enforce configured tool eligibility independently of acquisition transport.
pub(crate) fn tool_eligibility(config: &EffectiveConfig, tools: &[&str]) -> Result<()> {
    if let Some(allowed) = config.get("tools.allowed").and_then(Value::as_array) {
        for tool in tools {
            if !allowed.iter().any(|value| value.as_str() == Some(tool)) {
                bail!("CONFIG_OVERRIDE_DENIED: native tool {tool} is not eligible");
            }
        }
        for key in config
            .values()
            .keys()
            .filter_map(|key| key.strip_prefix("tools."))
            .filter(|key| !matches!(*key, "allowed" | "catalogs"))
        {
            if !allowed.iter().any(|value| value.as_str() == Some(key)) {
                bail!("CONFIG_OVERRIDE_DENIED: configured tool {key} is not eligible");
            }
        }
    }
    Ok(())
}

fn execution_routes(config: &EffectiveConfig) -> Result<()> {
    if config
        .get("registries.routes")
        .and_then(Value::as_array)
        .is_some_and(|routes| !routes.is_empty())
    {
        bail!("CONFIG_OVERRIDE_DENIED: enforced registry routes require provisioned connector bindings; public acquisition is disabled");
    }
    if config
        .get("tools.catalogs")
        .is_some_and(|catalogs| catalogs != &serde_json::json!(["public"]))
    {
        bail!("CONFIG_OVERRIDE_DENIED: approved tool catalog requires a provisioned connector; public fallback is disabled");
    }
    if config.values().keys().any(|key| {
        key.starts_with("tools.") && !matches!(key.as_str(), "tools.allowed" | "tools.catalogs")
    }) {
        bail!("CONFIG_INVALID_VALUE: explicit tool requests require verified locked tool installations; provisioned image selection alone does not establish that identity");
    }
    Ok(())
}
