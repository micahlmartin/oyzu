//! Administrative constraints compose independently from default precedence.
use super::registry::{Kind, Registry};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<Value>>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Constraints {
    entries: BTreeMap<String, Vec<(String, Entry)>>,
}
impl Entry {
    pub fn enforced(&self) -> bool {
        self.locked.is_some()
            || self.value.is_some()
            || self.allowed.is_some()
            || self.minimum.is_some()
            || self.maximum.is_some()
            || self.required.is_some()
    }
    fn accepts(&self, value: &Value) -> bool {
        self.value.as_ref().is_none_or(|v| v == value)
            && self.allowed.as_ref().is_none_or(|a| a.contains(value))
            && self
                .minimum
                .is_none_or(|n| value.as_u64().is_some_and(|v| v >= n))
            && self
                .maximum
                .is_none_or(|n| value.as_u64().is_some_and(|v| v <= n))
            && self.required.as_ref().is_none_or(|r| {
                value
                    .as_array()
                    .is_some_and(|a| r.iter().all(|v| a.contains(v)))
            })
    }
    pub fn validate(&mut self, key: &str, registry: &Registry) -> Result<()> {
        let d = registry
            .definition(key)
            .context("CONFIG_POLICY_CONFLICT: unknown mandatory setting")?;
        if self.locked == Some(false)
            || self.value.is_some() != (self.locked == Some(true))
            || self.value.is_some() && self.default.is_some()
            || !self.enforced() && self.default.is_none()
        {
            bail!("CONFIG_POLICY_CONFLICT: invalid policy entry for {key}");
        }
        if (self.minimum.is_some() || self.maximum.is_some())
            && !matches!(d.kind, Kind::Jobs | Kind::Percent)
            || self.required.is_some() && !d.set
        {
            bail!("CONFIG_POLICY_CONFLICT: unsupported constraint for {key}");
        }
        for v in [&mut self.default, &mut self.value].into_iter().flatten() {
            *v = registry.validate(key, v)?;
        }
        if let Some(allowed) = &mut self.allowed {
            if allowed.is_empty() {
                bail!("CONFIG_POLICY_CONFLICT: empty allowed set");
            }
            for v in allowed {
                *v = registry.validate(key, v)?;
            }
        }
        for bound in [self.minimum, self.maximum].into_iter().flatten() {
            registry.validate(key, &Value::from(bound))?;
        }
        if let Some(required) = &mut self.required {
            *required = registry
                .validate(key, &Value::Array(required.clone()))?
                .as_array()
                .unwrap()
                .clone();
        }
        if self.minimum.zip(self.maximum).is_some_and(|(a, b)| a > b) {
            bail!("CONFIG_POLICY_CONFLICT: reversed bounds for {key}");
        }
        if let Some(value) = self.value.as_ref().or(self.default.as_ref()) {
            if !self.accepts(value) {
                bail!("CONFIG_POLICY_CONFLICT: policy default violates its constraints for {key}");
            }
        }
        Ok(())
    }
}
impl Constraints {
    pub fn add(
        &mut self,
        source: &str,
        key: &str,
        mut entry: Entry,
        registry: &Registry,
    ) -> Result<()> {
        entry.validate(key, registry)?;
        let mut candidate = self.clone();
        candidate
            .entries
            .entry(key.into())
            .or_default()
            .push((source.into(), entry));
        candidate.check_consistency()?;
        *self = candidate;
        Ok(())
    }
    pub fn defaults(&self, source: &str) -> BTreeMap<String, Value> {
        self.entries
            .iter()
            .filter_map(|(key, entries)| {
                entries
                    .iter()
                    .find(|(s, _)| s == source)
                    .and_then(|(_, e)| e.value.clone().or(e.default.clone()))
                    .map(|v| (key.clone(), v))
            })
            .collect()
    }
    pub fn origins(&self, key: &str) -> Vec<String> {
        self.entries
            .get(key)
            .into_iter()
            .flatten()
            .map(|(s, _)| s.clone())
            .collect()
    }
    pub fn explain(&self, key: &str, registry: &Registry) -> Vec<Value> {
        self.entries.get(key).into_iter().flatten().map(|(source,entry)|{
            if registry.definition(key).is_some_and(|definition|definition.sensitive){serde_json::json!({"source":source,"restriction":"[REDACTED]","enforced":entry.enforced()})}
            else{serde_json::json!({"source":source,"restriction":entry})}
        }).collect()
    }
    fn check_consistency(&self) -> Result<()> {
        for (key, entries) in &self.entries {
            let constraints: Vec<_> = entries.iter().map(|(_, e)| e).collect();
            let lock = constraints.iter().find_map(|e| e.value.as_ref());
            let minimum = constraints.iter().filter_map(|e| e.minimum).max();
            let maximum = constraints.iter().filter_map(|e| e.maximum).min();
            let allowed = constraints.iter().find_map(|e| e.allowed.as_ref());
            if minimum.zip(maximum).is_some_and(|(a, b)| a > b)
                || lock.is_some_and(|v| constraints.iter().any(|e| !e.accepts(v)))
                || allowed
                    .is_some_and(|a| !a.iter().any(|v| constraints.iter().all(|e| e.accepts(v))))
            {
                bail!("CONFIG_POLICY_CONFLICT: incompatible administrative constraints for {key}");
            }
        }
        Ok(())
    }
    pub fn apply(
        &self,
        values: &mut BTreeMap<String, Value>,
        removed: &std::collections::BTreeSet<String>,
    ) -> Result<()> {
        self.check_consistency()?;
        for (key, entries) in &self.entries {
            if removed.contains(key) && entries.iter().any(|(_, entry)| entry.enforced()) {
                bail!("CONFIG_OVERRIDE_DENIED: cannot remove constrained {key}");
            }
            let mut members = Vec::new();
            for (_, entry) in entries {
                if let Some(required) = &entry.required {
                    for v in required {
                        if !members.contains(v) {
                            members.push(v.clone());
                        }
                    }
                }
            }
            if !members.is_empty() {
                let value = values
                    .entry(key.clone())
                    .or_insert_with(|| Value::Array(vec![]));
                let a = value
                    .as_array_mut()
                    .context("CONFIG_OVERRIDE_DENIED: required members need a set")?;
                for v in members {
                    if !a.contains(&v) {
                        a.push(v);
                    }
                }
                a.sort_by_key(Value::to_string);
            }
            for (_, entry) in entries {
                if entry.enforced() && values.get(key).is_none_or(|v| !entry.accepts(v)) {
                    bail!("CONFIG_OVERRIDE_DENIED: {key} violates administrative constraints");
                }
            }
        }
        Ok(())
    }
}
