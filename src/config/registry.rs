//! Typed OEP-0002 setting definitions. Unknown syntax never becomes executable.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const TASK_FIELDS: &[&str] = &[
    "run",
    "argv",
    "shell",
    "cwd",
    "env",
    "depends_on",
    "inputs",
    "outputs",
    "cache",
    "interactive",
    "reports",
];
pub const CAPABILITIES: &[&str] = &[
    "config.cascade/v1",
    "config.profiles/v1",
    "config.removal/v1",
    "policy.constraints/v1",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    Corporate,
    Admin,
    Machine,
    User,
    Project,
    Local,
    Invocation,
}
impl Scope {
    pub fn administrative(self) -> bool {
        matches!(self, Self::Corporate | Self::Admin)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effect {
    Computation,
    Execution,
    Transport,
    Presentation,
    Selector,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Boolean,
    Name,
    Jobs,
    Percent,
    Color,
    Strings,
    Tool,
    Environment,
    Task,
    Remote,
    Profile,
    Routes,
}
#[derive(Clone, Debug)]
pub struct SettingDefinition {
    pub key: String,
    pub kind: Kind,
    pub default: Option<Value>,
    pub administrative: bool,
    pub profile: bool,
    pub effect: Effect,
    pub sensitive: bool,
    pub set: bool,
}
#[derive(Clone, Debug)]
pub struct Registry {
    definitions: BTreeMap<String, SettingDefinition>,
}
impl Default for Registry {
    fn default() -> Self {
        let mut registry = Self {
            definitions: BTreeMap::new(),
        };
        for (key, kind, default, administrative, effect) in [
            ("cache.remote", Kind::Remote, None, false, Effect::Transport),
            (
                "cache.read",
                Kind::Boolean,
                Some(json!(true)),
                false,
                Effect::Execution,
            ),
            (
                "cache.write",
                Kind::Boolean,
                Some(json!(false)),
                false,
                Effect::Execution,
            ),
            (
                "cache.local",
                Kind::Boolean,
                Some(json!(true)),
                false,
                Effect::Execution,
            ),
            (
                "build.jobs",
                Kind::Jobs,
                Some(json!(
                    std::thread::available_parallelism().map_or(1, usize::from)
                )),
                false,
                Effect::Execution,
            ),
            (
                "checks.required",
                Kind::Strings,
                Some(json!([])),
                false,
                Effect::Computation,
            ),
            (
                "checks.coverageMinimum",
                Kind::Percent,
                Some(json!(0)),
                false,
                Effect::Computation,
            ),
            (
                "ui.color",
                Kind::Color,
                Some(json!("auto")),
                false,
                Effect::Presentation,
            ),
            (
                "profile.default",
                Kind::Profile,
                None,
                false,
                Effect::Selector,
            ),
            (
                "config.localOverridesInCi",
                Kind::Boolean,
                Some(json!(false)),
                true,
                Effect::Execution,
            ),
            (
                "config.allowedProfiles",
                Kind::Strings,
                None,
                true,
                Effect::Selector,
            ),
            (
                "tools.allowed",
                Kind::Strings,
                None,
                true,
                Effect::Computation,
            ),
            (
                "tools.catalogs",
                Kind::Strings,
                Some(json!(["public"])),
                true,
                Effect::Transport,
            ),
            (
                "registries.routes",
                Kind::Routes,
                Some(json!([])),
                true,
                Effect::Transport,
            ),
            ("tools.*", Kind::Tool, None, false, Effect::Computation),
            ("env.*", Kind::Environment, None, false, Effect::Computation),
            ("tasks.*", Kind::Task, None, false, Effect::Computation),
        ] {
            registry
                .register(SettingDefinition {
                    key: key.into(),
                    kind,
                    default,
                    administrative,
                    profile: !administrative && kind != Kind::Profile,
                    effect,
                    sensitive: matches!(kind, Kind::Environment | Kind::Task),
                    set: kind == Kind::Strings && key != "tools.catalogs",
                })
                .expect("unique built-in settings");
        }
        for builder in crate::builders::all() {
            builder
                .register_settings(&mut registry)
                .expect("unique builder settings");
        }
        registry
    }
}
impl Registry {
    pub fn register(&mut self, definition: SettingDefinition) -> Result<()> {
        if self.definitions.contains_key(&definition.key) {
            bail!("CONFIG_INVALID_VALUE: duplicate setting registration");
        }
        self.definitions.insert(definition.key.clone(), definition);
        Ok(())
    }
    pub fn has_namespace(&self, root: &str) -> bool {
        self.definitions.keys().any(|key| {
            key.split_once('.')
                .is_some_and(|(prefix, _)| prefix == root)
        })
    }
    pub fn definition(&self, key: &str) -> Option<&SettingDefinition> {
        self.definitions.get(key).or_else(|| {
            key.split_once('.').and_then(|(root, name)| {
                (!name.is_empty())
                    .then(|| self.definitions.get(&format!("{root}.*")))
                    .flatten()
            })
        })
    }
    pub fn defaults(&self) -> BTreeMap<String, Value> {
        self.definitions
            .iter()
            .filter_map(|(k, d)| d.default.clone().map(|v| (k.clone(), v)))
            .collect()
    }
    pub fn validate(&self, key: &str, value: &Value) -> Result<Value> {
        let Some(d) = self.definition(key) else {
            bail!("CONFIG_UNKNOWN_OPTIONAL: unknown setting {key}");
        };
        let mut normalized = value.clone();
        if d.kind == Kind::Task {
            if let Some(object) = normalized.as_object_mut() {
                object.retain(|key, _| TASK_FIELDS.contains(&key.as_str()));
            }
        }
        let value = &normalized;
        let valid = match d.kind {
            Kind::Boolean => value.is_boolean(),
            Kind::Name => value.as_str().is_some_and(crate::names::valid),
            Kind::Jobs => value.as_u64().is_some_and(|n| (1..=65535).contains(&n)),
            Kind::Percent => value.as_u64().is_some_and(|n| n <= 100),
            Kind::Color => value
                .as_str()
                .is_some_and(|s| matches!(s, "auto" | "always" | "never")),
            Kind::Profile => value.as_str().is_some_and(crate::names::valid),
            Kind::Strings => value
                .as_array()
                .is_some_and(|a| a.iter().all(|v| v.as_str().is_some_and(|s| !s.is_empty()))),
            Kind::Tool => value
                .as_str()
                .is_some_and(|s| !s.is_empty() && !s.chars().any(char::is_control)),
            Kind::Environment => {
                value.is_string()
                    || value.as_object().is_some_and(|o| {
                        o.len() == 1
                            && o.get("secret")
                                .and_then(Value::as_str)
                                .is_some_and(|s| !s.is_empty())
                    })
            }
            Kind::Task => {
                let parsed = serde_json::from_value::<super::TaskConfig>(value.clone());
                parsed.is_ok_and(|t| {
                    t.run.is_some() != t.argv.is_some()
                        && t.argv.as_ref().is_none_or(|a| !a.is_empty())
                        && crate::reports::validate_declarations(&t.reports).is_ok()
                })
            }
            Kind::Remote => value
                .as_str()
                .and_then(|s| reqwest::Url::parse(s).ok())
                .is_some_and(|u| {
                    u.scheme() == "oci"
                        && u.host_str().is_some()
                        && u.username().is_empty()
                        && u.password().is_none()
                        && u.query().is_none()
                        && u.fragment().is_none()
                }),
            Kind::Routes => value.as_array().is_some_and(|a| {
                a.iter().all(|v| {
                    v.as_object().is_some_and(|o| {
                        o.len() == 3
                            && o.get("protocol").and_then(Value::as_str).is_some_and(|s| {
                                matches!(
                                    s,
                                    "npm" | "pypi" | "maven" | "cargo" | "go" | "oci" | "tools"
                                )
                            })
                            && ["scope", "connectorId"].iter().all(|k| {
                                o.get(*k)
                                    .and_then(Value::as_str)
                                    .is_some_and(|s| !s.is_empty())
                            })
                    })
                })
            }),
        };
        if !valid {
            bail!("CONFIG_INVALID_VALUE: invalid value for {key}");
        }
        if d.kind == Kind::Task {
            if let Some(env) = value.get("env").and_then(Value::as_object) {
                let mut names = BTreeSet::new();
                for (name, value) in env {
                    self.validate(&format!("env.{name}"), value)?;
                    if !names.insert(name.to_ascii_uppercase()) {
                        bail!("CONFIG_INVALID_VALUE: case-colliding task environment");
                    }
                }
            }
        }
        if key == "config.allowedProfiles"
            && value.as_array().unwrap().iter().any(|v| {
                v.as_str()
                    .is_none_or(|s| s != "@none" && !crate::names::valid(s))
            })
        {
            bail!("CONFIG_INVALID_VALUE: invalid allowed profile name");
        }
        if matches!(key, "tools.allowed" | "tools.catalogs")
            && value.as_array().unwrap().iter().any(|v| {
                v.as_str().is_none_or(|s| {
                    s.contains("://") || s.chars().any(|c| c.is_whitespace() || c.is_control())
                })
            })
        {
            bail!("CONFIG_INVALID_VALUE: expected canonical tool or catalog identifiers");
        }
        if d.kind == Kind::Routes {
            let mut routes = BTreeSet::new();
            for route in value.as_array().unwrap() {
                let protocol = route["protocol"].as_str().unwrap();
                let scope = route["scope"].as_str().unwrap();
                if scope.chars().any(|c| c.is_whitespace() || c.is_control())
                    || scope.contains("://")
                    || scope.contains("..")
                    || scope.contains('\\')
                    || scope.contains('*') && scope != "*"
                    || !routes.insert((protocol, scope))
                {
                    bail!("CONFIG_INVALID_VALUE: invalid or ambiguous registry route");
                }
            }
        }
        if key == "checks.required"
            && value
                .as_array()
                .unwrap()
                .iter()
                .any(|v| !matches!(v.as_str(), Some("tests" | "coverage" | "lint" | "format")))
        {
            bail!("CONFIG_INVALID_VALUE: unregistered check ID");
        }
        if let Some(name) = key.strip_prefix("env.") {
            if name.is_empty()
                || !name.bytes().enumerate().all(|(i, b)| {
                    b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                })
                || name.to_ascii_uppercase().starts_with("OYZU_")
            {
                bail!("CONFIG_INVALID_VALUE: reserved or invalid environment name");
            }
        }
        if d.set {
            let members: BTreeSet<_> = value
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            return Ok(json!(members));
        }
        Ok(value.clone())
    }
}
