//! Strict administrative JSON; authority is supplied only by protected readers.
use super::{
    constraints::{Constraints, Entry},
    registry::{Registry, Scope},
    sources::{capabilities, ConfigSource},
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub schema_version: u64,
    pub kind: String,
    pub settings: BTreeMap<String, Entry>,
    pub profiles: BTreeMap<String, Value>,
    pub required_capabilities: Vec<String>,
    pub default_profile: Option<String>,
    pub extensions: Option<BTreeMap<String, Value>>,
}
// Deserialize through a recursive visitor so duplicate keys cannot disappear in a map.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite JSON"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = vec![];
                while let Some(v) = seq.next_element::<Unique>()? {
                    values.push(v.0);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, Unique>()? {
                    if values.insert(key, value.0).is_some() {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
pub fn strict_json(bytes: &[u8]) -> Result<Value> {
    strict_json_limit(bytes, 1024 * 1024)
}
pub(crate) fn strict_json_limit(bytes: &[u8], limit: usize) -> Result<Value> {
    if bytes.len() > limit {
        bail!("CONFIG_LIMIT: JSON exceeds byte limit");
    }
    let value = serde_json::from_slice::<Unique>(bytes)
        .map_err(|_| anyhow::anyhow!("POLICY_INVALID: malformed JSON or duplicate key"))?
        .0;
    fn limits(value: &Value, depth: usize, count: &mut usize) -> Result<()> {
        *count += 1;
        if depth > 32 || *count > 10000 {
            bail!("CONFIG_LIMIT: JSON depth or entries");
        }
        match value {
            Value::Object(v) => {
                for v in v.values() {
                    limits(v, depth + 1, count)?
                }
            }
            Value::Array(a) => {
                for v in a {
                    limits(v, depth + 1, count)?
                }
            }
            _ => {}
        }
        Ok(())
    }
    limits(&value, 0, &mut 0)?;
    Ok(value)
}
impl Policy {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let value = strict_json(bytes)?;
        validate_entry_shapes(&value)?;
        let result: Self = serde_json::from_value(value)
            .map_err(|_| anyhow::anyhow!("POLICY_INVALID: invalid administrative schema"))?;
        if result.schema_version != 1 || result.kind != "local-admin-policy" {
            bail!("POLICY_INVALID: unsupported administrative schema");
        }
        capabilities(&result.required_capabilities)?;
        Ok(result)
    }
    pub fn source(
        self,
        identity: &str,
        directory: &Path,
        scope: Scope,
        registry: &Registry,
        constraints: &mut Constraints,
    ) -> Result<ConfigSource> {
        let mut base = serde_json::Map::new();
        let mut unknown = Vec::new();
        for (key, entry) in self.settings {
            if registry.definition(&key).is_none() {
                if entry.enforced() {
                    bail!("CONFIG_POLICY_CONFLICT: unknown mandatory setting {key}");
                }
                unknown.push(key);
                continue;
            }
            constraints.add(identity, &key, entry, registry)?;
        }
        for (key, value) in constraints.defaults(identity) {
            let (root, name) = key
                .split_once('.')
                .context("POLICY_INVALID: invalid setting path")?;
            base.entry(root)
                .or_insert_with(|| Value::Object(Default::default()))
                .as_object_mut()
                .unwrap()
                .insert(name.into(), value);
        }
        base.insert("profiles".into(), serde_json::to_value(self.profiles)?);
        if let Some(profile) = self.default_profile {
            base.insert("profile".into(), serde_json::json!({"default":profile}));
        }
        let toml_value: toml::Value = serde_json::from_value(Value::Object(base))
            .context("POLICY_INVALID: values cannot be represented as settings")?;
        let mut source = ConfigSource::parse(
            identity,
            directory,
            scope,
            false,
            &toml::to_string(&toml_value)?,
            registry,
        )?;
        for key in unknown {
            source.diagnostics.push(super::sources::warning(
                identity,
                &key,
                "unknown administrative default ignored",
            ));
        }
        Ok(source)
    }
}

pub(crate) fn validate_entry_shapes(value: &Value) -> Result<()> {
    if let Some(settings) = value.get("settings").and_then(Value::as_object) {
        for entry in settings.values() {
            if entry
                .as_object()
                .is_some_and(|entry| entry.values().any(Value::is_null))
            {
                bail!("POLICY_INVALID: policy entries cannot contain null operands");
            }
        }
    }
    if value.get("defaultProfile").is_some_and(|v| !v.is_string())
        || value.get("extensions").is_some_and(|v| !v.is_object())
    {
        bail!("POLICY_INVALID: invalid optional policy fields");
    }
    Ok(())
}
