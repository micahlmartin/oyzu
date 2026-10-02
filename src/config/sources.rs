//! Bounded, non-executing source parsing and scope-aware discovery.
use super::registry::{Registry, Scope, CAPABILITIES};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    #[default]
    Warning,
    Error,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default)]
    pub span: Option<(usize, usize)>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub remedy: String,
    pub source: String,
    pub key: String,
    pub message: String,
}
#[derive(Clone, Debug, Default)]
pub struct Overlay {
    pub values: BTreeMap<String, Value>,
    pub remove: Vec<String>,
    pub requires: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct ConfigSource {
    pub identity: String,
    pub scope: Scope,
    pub directory: PathBuf,
    pub digest: String,
    pub syntax: String,
    pub base: Overlay,
    pub profiles: BTreeMap<String, Overlay>,
    pub diagnostics: Vec<Diagnostic>,
    pub spans: BTreeMap<String, (usize, usize)>,
}
pub fn warning(source: &str, key: &str, message: &str) -> Diagnostic {
    Diagnostic {
        code: "CONFIG_UNKNOWN_OPTIONAL".into(),
        source: source.into(),
        key: key.into(),
        message: message.into(),
        remedy: "Check the setting spelling and supported configuration capabilities".into(),
        ..Diagnostic::default()
    }
}
pub fn capabilities(values: &[String]) -> Result<()> {
    for value in values {
        if !CAPABILITIES.contains(&value.as_str()) {
            bail!("CONFIG_REQUIRED_CAPABILITY: unsupported capability {value}");
        }
    }
    Ok(())
}
fn strings(v: &Value) -> Result<Vec<String>> {
    serde_json::from_value(v.clone())
        .map_err(|_| anyhow::anyhow!("CONFIG_INVALID_VALUE: expected string array"))
}
fn bounds(v: &Value, depth: usize, count: &mut usize) -> Result<()> {
    *count += 1;
    if depth > 32 || *count > 10000 {
        bail!("CONFIG_LIMIT: configuration nesting or entry limit");
    }
    match v {
        Value::Object(o) => {
            for v in o.values() {
                bounds(v, depth + 1, count)?
            }
        }
        Value::Array(a) => {
            for v in a {
                bounds(v, depth + 1, count)?
            }
        }
        _ => {}
    }
    Ok(())
}
fn overlay(
    value: &Value,
    source: &str,
    scope: Scope,
    nested: bool,
    profile: bool,
    registry: &Registry,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Overlay> {
    let object = value
        .as_object()
        .context("CONFIG_INVALID_VALUE: expected configuration table")?;
    let mut result = Overlay::default();
    for (root, value) in object {
        if root == "profiles" && !profile {
            continue;
        }
        if root == "compatibility" {
            let object = value
                .as_object()
                .context("CONFIG_INVALID_VALUE: compatibility must be a table")?;
            if object
                .get("schema_major")
                .is_some_and(|v| v.as_u64() != Some(1))
            {
                bail!("CONFIG_MAJOR_VERSION: unsupported schema major");
            }
            if let Some(v) = object.get("requires") {
                result.requires = strings(v)?;
            }
            for key in object
                .keys()
                .filter(|k| !matches!(k.as_str(), "schema_major" | "requires"))
            {
                diagnostics.push(warning(
                    source,
                    &format!("compatibility.{key}"),
                    "unknown optional metadata",
                ));
            }
            continue;
        }
        if root == "overrides" {
            let object = value
                .as_object()
                .context("CONFIG_INVALID_VALUE: overrides must be a table")?;
            if let Some(v) = object.get("remove") {
                result.remove = strings(v)?;
            }
            for key in object.keys().filter(|k| k.as_str() != "remove") {
                diagnostics.push(warning(
                    source,
                    &format!("overrides.{key}"),
                    "unknown optional metadata",
                ));
            }
            continue;
        }
        if matches!(
            root.as_str(),
            "management" | "execution" | "trusted" | "releaseAuthority"
        ) || (profile
            && matches!(
                root.as_str(),
                "profiles" | "profile" | "includes" | "constraints"
            ))
        {
            bail!("CONFIG_SCOPE: forbidden configuration section {root}");
        }
        let known_root = registry.has_namespace(root);
        if !known_root {
            diagnostics.push(warning(source, root, "unknown optional section"));
            continue;
        }
        let entries = value
            .as_object()
            .with_context(|| format!("CONFIG_INVALID_VALUE: {root} must be a table"))?;
        for (entry, value) in entries {
            let key = format!("{root}.{entry}");
            let Some(def) = registry.definition(&key) else {
                diagnostics.push(warning(source, &key, "unknown optional setting"));
                continue;
            };
            if def.administrative && !scope.administrative()
                || profile && !def.profile
                || nested && key == "profile.default"
            {
                bail!("CONFIG_SCOPE: {key} is not permitted in this scope");
            }
            if root == "tasks" {
                if let Some(object) = value.as_object() {
                    for field in object
                        .keys()
                        .filter(|field| !super::registry::TASK_FIELDS.contains(&field.as_str()))
                    {
                        diagnostics.push(warning(
                            source,
                            &format!("{key}.{field}"),
                            "unknown optional task field",
                        ));
                    }
                }
            }
            result
                .values
                .insert(key.clone(), registry.validate(&key, value)?);
        }
    }
    for key in &result.remove {
        if result.values.contains_key(key) {
            bail!("CONFIG_INVALID_VALUE: overlay removes and assigns {key}");
        }
        if !key.starts_with("env.") && !key.starts_with("tasks.") {
            if registry.definition(key).is_some() {
                bail!("CONFIG_SCOPE: setting cannot be removed: {key}");
            }
            diagnostics.push(warning(source, key, "unknown optional removal target"));
        }
    }
    Ok(result)
}
impl ConfigSource {
    pub fn parse(
        identity: &str,
        directory: &Path,
        scope: Scope,
        nested: bool,
        text: &str,
        registry: &Registry,
    ) -> Result<Self> {
        if text.len() > 1024 * 1024 {
            bail!("CONFIG_LIMIT: ordinary source exceeds 1 MiB");
        }
        // Never include parser snippets: an invalid line may contain a secret.
        let parsed: toml::Value = toml::from_str(text).map_err(|e: toml::de::Error| {
            anyhow::anyhow!(
                "{}: invalid TOML in {} at {:?}",
                if e.message().contains("duplicate") {
                    "CONFIG_DUPLICATE"
                } else {
                    "CONFIG_SYNTAX"
                },
                identity,
                e.span()
            )
        })?;
        let value = serde_json::to_value(parsed)?;
        bounds(&value, 0, &mut 0)?;
        let document = toml_edit::ImDocument::parse(text)
            .map_err(|_| anyhow::anyhow!("CONFIG_SYNTAX: invalid TOML"))?;
        let mut spans = BTreeMap::new();
        fn collect(
            table: &dyn toml_edit::TableLike,
            prefix: &str,
            spans: &mut BTreeMap<String, (usize, usize)>,
        ) {
            for (key, item) in table.iter() {
                let key = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{prefix}.{key}")
                };
                if let Some(span) = item.span() {
                    spans.insert(key.clone(), (span.start, span.end));
                }
                if let Some(table) = item.as_table_like() {
                    collect(table, &key, spans);
                }
            }
        }
        collect(document.as_table(), "", &mut spans);
        let mut diagnostics = Vec::new();
        let base = overlay(
            &value,
            identity,
            scope,
            nested,
            false,
            registry,
            &mut diagnostics,
        )?;
        for diagnostic in &mut diagnostics {
            diagnostic.span = spans.get(&diagnostic.key).copied();
        }
        let mut profiles = BTreeMap::new();
        let mut names = BTreeSet::new();
        if let Some(v) = value.get("profiles") {
            let object = v
                .as_object()
                .context("CONFIG_INVALID_VALUE: profiles must be a table")?;
            if object.len() > 128 {
                bail!("CONFIG_LIMIT: too many profiles");
            }
            for (name, v) in object {
                if !crate::names::valid(name) || !names.insert(name.to_ascii_lowercase()) {
                    bail!("CONFIG_INVALID_VALUE: invalid or case-colliding profile name");
                }
                let start = diagnostics.len();
                let values = overlay(v, identity, scope, nested, true, registry, &mut diagnostics)?;
                for diagnostic in &mut diagnostics[start..] {
                    diagnostic.span = spans
                        .get(&format!("profiles.{name}.{}", diagnostic.key))
                        .copied();
                }
                profiles.insert(name.clone(), values);
            }
        }
        Ok(Self {
            spans,
            identity: identity.into(),
            scope,
            directory: directory.into(),
            digest: crate::records::digest("oyzu.config.source.v1", &serde_json::json!(text))?,
            syntax: text.into(),
            base,
            profiles,
            diagnostics,
        })
    }
    pub fn read(
        path: &Path,
        scope: Scope,
        nested: bool,
        registry: &Registry,
    ) -> Result<Option<Self>> {
        use std::io::Read;
        let file = match fs::File::open(path) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => bail!("CONFIG_SYNTAX: cannot read {}", path.display()),
        };
        if file.metadata()?.len() > 1024 * 1024 {
            bail!("CONFIG_LIMIT: ordinary source exceeds 1 MiB");
        }
        let mut text = String::new();
        file.take(1024 * 1024 + 1)
            .read_to_string(&mut text)
            .map_err(|_| anyhow::anyhow!("CONFIG_SYNTAX: cannot read configuration UTF-8"))?;
        Self::parse(
            &path.display().to_string(),
            path.parent().unwrap(),
            scope,
            nested,
            &text,
            registry,
        )
        .map(Some)
    }
}
pub fn project_sources(
    root: &Path,
    target: &Path,
    local: bool,
    registry: &Registry,
) -> Result<Vec<ConfigSource>> {
    let root = root.canonicalize()?;
    let target = target.canonicalize()?;
    if !target.starts_with(&root) {
        bail!("CONFIG_SCOPE: target escapes workspace");
    }
    let mut directories = vec![root.clone()];
    let mut current = root.clone();
    for component in target.strip_prefix(&root)?.components() {
        if directories.len() > 32 {
            bail!("CONFIG_LIMIT: project configuration depth exceeds 32");
        }
        current.push(component);
        if current.join(".git").exists() {
            bail!("CONFIG_SCOPE: submodule requires an explicit import");
        }
        if current.join("build.yaml").exists() {
            bail!("CONFIG_SCOPE: nested build.yaml");
        }
        directories.push(current.clone());
    }
    let mut sources = Vec::new();
    let mut captured_bytes = 0usize;
    for (filename, scope) in [
        ("oyzu.toml", Scope::Project),
        ("oyzu.local.toml", Scope::Local),
    ] {
        if scope == Scope::Local && !local {
            continue;
        }
        for directory in &directories {
            let file = directory.join(filename);
            if file.exists() && !file.canonicalize()?.starts_with(&root) {
                bail!("CONFIG_SCOPE: configuration symlink escapes workspace");
            }
            if let Some(source) = ConfigSource::read(
                &directory.join(filename),
                scope,
                directory != &root,
                registry,
            )? {
                captured_bytes += source.syntax.len();
                if sources.len() >= 128 || captured_bytes > 8 * 1024 * 1024 {
                    bail!("CONFIG_LIMIT: project source limit exceeded");
                }
                sources.push(source);
            }
        }
    }
    Ok(sources)
}
pub fn detected_ci() -> bool {
    [
        "CI",
        "GITHUB_ACTIONS",
        "GITLAB_CI",
        "TF_BUILD",
        "BUILDKITE",
        "JENKINS_URL",
        "TEAMCITY_VERSION",
    ]
    .iter()
    .any(|key| {
        std::env::var(key).is_ok_and(|s| {
            !s.is_empty() && !matches!(s.to_ascii_lowercase().as_str(), "false" | "0")
        })
    })
}
