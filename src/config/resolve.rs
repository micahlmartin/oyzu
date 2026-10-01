//! Pure deterministic resolution. A returned snapshot owns all values it consumes.
use super::{
    constraints::Constraints,
    registry::{Effect, Registry, Scope},
    sources::{capabilities, warning, ConfigSource, Diagnostic, Overlay},
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Default)]
pub struct Selection {
    pub explicit: Option<String>,
    pub no_profile: bool,
    pub environment: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Origin {
    pub source: String,
    pub scope: Option<Scope>,
    pub profile: Option<String>,
    pub operation: String,
    pub span: Option<(usize, usize)>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectiveConfig {
    values: BTreeMap<String, Value>,
    pub profile: Option<String>,
    pub selection_reason: String,
    pub profiles: BTreeMap<String, Vec<String>>,
    pub origins: BTreeMap<String, Vec<Origin>>,
    pub diagnostics: Vec<Diagnostic>,
    pub exclusions: Vec<String>,
    pub constraints: Constraints,
    pub removed: BTreeSet<String>,
    pub digest: String,
    pub management: Option<Value>,
}
impl EffectiveConfig {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }
    pub fn values(&self) -> &BTreeMap<String, Value> {
        &self.values
    }
    pub fn public_values(&self, registry: &Registry) -> BTreeMap<String, Value> {
        self.values
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    if registry.definition(k).is_some_and(|d| d.sensitive) {
                        json!("[REDACTED]")
                    } else {
                        v.clone()
                    },
                )
            })
            .collect()
    }
    pub fn explain(&self, registry: &Registry) -> Value {
        json!({"values":self.public_values(registry),"profile":self.profile,"selectionReason":self.selection_reason,"profiles":self.profiles,"origins":self.origins,"exclusions":self.exclusions,"diagnostics":self.diagnostics,"constraints":self.values.keys().map(|key|(key.clone(),self.constraints.explain(key,registry))).collect::<BTreeMap<_,_>>(),"constraintSources":self.values.keys().map(|key|(key.clone(),self.constraints.origins(key))).collect::<BTreeMap<_,_>>(),"digest":self.digest})
    }
}
pub fn resolve(
    sources: &[ConfigSource],
    registry: &Registry,
    ci: bool,
    selection: &Selection,
    constraints: Constraints,
    all_profiles: bool,
) -> Result<EffectiveConfig> {
    if sources.len() > 128
        || sources.iter().map(|s| s.syntax.len()).sum::<usize>() > 8 * 1024 * 1024
    {
        bail!("CONFIG_LIMIT: aggregate source limit");
    }
    if selection.no_profile && selection.explicit.is_some() {
        bail!("CONFIG_INVALID_VALUE: conflicting profile switches");
    }
    let mut profiles = BTreeMap::from([("ci".to_string(), vec!["built-in".to_string()])]);
    let mut default = None;
    let mut diagnostics = Vec::new();
    let mut exclusions = Vec::new();
    for source in sources {
        diagnostics.extend(source.diagnostics.clone());
        if ci && source.scope == Scope::User {
            exclusions.push(format!(
                "{}: personal computation settings excluded in CI",
                source.identity
            ));
            continue;
        }
        if let Some(v) = source.base.values.get("profile.default") {
            default = v.as_str().map(str::to_owned);
        }
        for (name, overlay) in &source.profiles {
            profiles
                .entry(name.clone())
                .or_insert_with(Vec::new)
                .push(source.identity.clone());
            if all_profiles {
                capabilities(&overlay.requires)?;
            }
        }
    }
    let mut cases = BTreeSet::new();
    for name in profiles.keys() {
        if !cases.insert(name.to_ascii_lowercase()) {
            bail!("CONFIG_INVALID_VALUE: case-colliding profiles across sources");
        }
    }
    if profiles.len() > 128 {
        bail!("CONFIG_LIMIT: profile limit");
    }
    let (profile, reason) = if selection.no_profile {
        (None, "--no-profile")
    } else if let Some(name) = &selection.explicit {
        (Some(name.clone()), "--profile")
    } else if let Some(name) = selection.environment.as_ref().filter(|s| !s.is_empty()) {
        (Some(name.clone()), "OYZU_PROFILE")
    } else if default.is_some() {
        (default, "profile.default")
    } else if ci {
        (Some("ci".into()), "inferred CI")
    } else {
        (None, "no default")
    };
    if let Some(name) = &profile {
        if !profiles.contains_key(name) {
            bail!(
                "CONFIG_PROFILE_UNKNOWN: {name}; available: {}",
                profiles.keys().cloned().collect::<Vec<_>>().join(", ")
            );
        }
    }
    let mut result = EffectiveConfig {
        values: registry.defaults(),
        profile,
        selection_reason: reason.into(),
        profiles,
        origins: BTreeMap::new(),
        diagnostics,
        exclusions,
        constraints,
        removed: BTreeSet::new(),
        digest: String::new(),
        management: None,
    };
    for key in result.values.keys() {
        result.origins.insert(
            key.clone(),
            vec![Origin {
                source: "built-in".into(),
                scope: None,
                profile: None,
                operation: "default".into(),
                span: None,
            }],
        );
    }
    for source in sources {
        if !(ci && source.scope == Scope::User) {
            capabilities(&source.base.requires)?;
        }
        apply(&mut result, &source.base, source, None, registry, ci)?;
        if let Some((name, overlay)) = result
            .profile
            .clone()
            .and_then(|name| source.profiles.get(&name).map(|o| (name, o)))
        {
            if !(ci && source.scope == Scope::User) {
                capabilities(&overlay.requires)?;
            }
            apply(&mut result, overlay, source, Some(name), registry, ci)?;
        }
    }
    result
        .constraints
        .apply(&mut result.values, &result.removed)?;
    if let Some(allowed) = result.values.get("config.allowedProfiles") {
        let requested = json!(result.profile.as_deref().unwrap_or("@none"));
        if !allowed.as_array().unwrap().contains(&requested) {
            bail!("CONFIG_OVERRIDE_DENIED: selected profile is not permitted");
        }
    }
    let environment: Vec<_> = result
        .values
        .keys()
        .filter_map(|key| key.strip_prefix("env."))
        .collect();
    super::registry::validate_environment_case(environment.iter().copied())?;
    for (key, task) in &result.values {
        if key.starts_with("tasks.") {
            if let Some(env) = task.get("env").and_then(Value::as_object) {
                super::registry::validate_environment_case(
                    environment
                        .iter()
                        .copied()
                        .chain(env.keys().map(String::as_str)),
                )?;
            }
        }
    }
    let identity: BTreeMap<_, _> = result
        .values
        .iter()
        .filter(|(k, _)| {
            registry
                .definition(k)
                .is_some_and(|d| matches!(d.effect, Effect::Computation | Effect::Execution))
        })
        .filter(|(k, v)| !k.starts_with("env.") || v.is_string())
        .collect();
    result.digest = crate::records::digest("oyzu.config.v1", &json!(identity))?;
    Ok(result)
}
fn apply(
    result: &mut EffectiveConfig,
    overlay: &Overlay,
    source: &ConfigSource,
    profile: Option<String>,
    registry: &Registry,
    ci: bool,
) -> Result<()> {
    let eligible = |key: &str| {
        !(ci && source.scope == Scope::User)
            || registry
                .definition(key)
                .is_some_and(|d| d.effect == Effect::Presentation)
    };
    for key in &overlay.remove {
        if !eligible(key) || !(key.starts_with("env.") || key.starts_with("tasks.")) {
            continue;
        }
        if result.values.remove(key).is_none() {
            result
                .diagnostics
                .push(warning(&source.identity, key, "removal of absent setting"));
        }
        result.removed.insert(key.clone());
        result.origins.entry(key.clone()).or_default().push(Origin {
            source: source.identity.clone(),
            scope: Some(source.scope),
            profile: profile.clone(),
            operation: "remove".into(),
            span: source
                .spans
                .get(&profile.as_ref().map_or_else(
                    || "overrides.remove".to_string(),
                    |p| format!("profiles.{p}.overrides.remove"),
                ))
                .copied(),
        });
    }
    for (key, value) in &overlay.values {
        if !eligible(key) {
            continue;
        }
        result.values.insert(key.clone(), value.clone());
        result.removed.remove(key);
        result.origins.entry(key.clone()).or_default().push(Origin {
            source: source.identity.clone(),
            scope: Some(source.scope),
            profile: profile.clone(),
            operation: "replace".into(),
            span: source
                .spans
                .get(
                    &profile
                        .as_ref()
                        .map_or_else(|| key.clone(), |p| format!("profiles.{p}.{key}")),
                )
                .copied(),
        });
    }
    Ok(())
}
