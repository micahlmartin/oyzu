use anyhow::{bail, Context, Result};
pub mod agent;
pub mod constraints;
mod container;
pub use container::{Container, ContainerOptions};
pub mod edit;
pub(crate) mod enforcement;
mod inventory;
pub mod locations;
pub mod managed;
pub mod operations;
pub mod policy;
mod policy_runtime;
pub mod registry;
pub mod resolve;
pub mod session;
pub mod sources;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TargetConfig {
    pub uses: String,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub materialize: Vec<Materialize>,
    pub platform: Option<String>,
    #[serde(default)]
    pub matrix: BTreeMap<String, Vec<String>>,
    pub container: Option<Container>,
    pub bindings: Option<serde_yaml::Value>,
    pub dependencies: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Materialize {
    pub from: String,
    pub artifact: Option<String>,
    pub to: PathBuf,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskConfig {
    pub run: Option<String>,
    pub argv: Option<Vec<String>>,
    pub shell: Option<String>,
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub cache: bool,
    #[serde(default)]
    pub interactive: bool,
    #[serde(default)]
    pub reports: Vec<crate::reports::Declaration>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    #[serde(default)]
    pub tasks: BTreeMap<String, TaskConfig>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub tools: BTreeMap<String, toml::Value>,
    pub cache: Option<toml::Value>,
}

pub fn contained(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute()
        || relative.components().any(|p| {
            matches!(
                p,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        bail!("path escapes workspace: {}", relative.display());
    }
    let path = root.join(relative);
    let canonical = path
        .canonicalize()
        .with_context(|| format!("missing project path {}", path.display()))?;
    if !canonical.starts_with(root.canonicalize()?) {
        bail!("symlink escapes workspace: {}", relative.display());
    }
    Ok(canonical)
}

pub fn targets(root: &Path) -> Result<Option<BTreeMap<String, TargetConfig>>> {
    let (targets, diagnostics) = targets_with_diagnostics(root)?;
    for diagnostic in diagnostics {
        eprintln!(
            "{}: {}: {}",
            diagnostic.code, diagnostic.source, diagnostic.message
        );
    }
    Ok(targets)
}
pub type TargetInventory = BTreeMap<String, TargetConfig>;
pub type InventoryResolution = (Option<TargetInventory>, Vec<sources::Diagnostic>);
pub fn targets_with_diagnostics(root: &Path) -> Result<InventoryResolution> {
    let (captured, diagnostics) = capture_targets(root)?;
    Ok((
        captured.source_digest.is_some().then_some(captured.targets),
        diagnostics,
    ))
}

/// One bounded read of build.yaml, shared by discovery, selection and planning.
/// The digest identifies the exact bytes parsed, including optional fields.
#[derive(Clone, Debug, Default)]
pub(crate) struct BuildInventory {
    pub targets: TargetInventory,
    pub source_digest: Option<String>,
}

pub(crate) fn capture_targets(root: &Path) -> Result<(BuildInventory, Vec<sources::Diagnostic>)> {
    let mut diagnostics = Vec::new();
    let path = root.join("build.yaml");
    if !path.is_file() {
        return Ok((BuildInventory::default(), diagnostics));
    }
    use std::io::Read;
    let file = fs::File::open(&path)?;
    if file.metadata()?.len() > 1024 * 1024 {
        bail!("CONFIG_LIMIT: build.yaml exceeds 1 MiB");
    }
    let mut text = String::new();
    file.take(1024 * 1024 + 1).read_to_string(&mut text)?;
    if text.len() > 1024 * 1024 {
        bail!("configuration exceeds 1 MiB: {}", path.display());
    }
    inventory::validate(&text)?;
    let raw: BTreeMap<String, serde_yaml::Value> = serde_yaml::from_str(&text)
        .map_err(|_| anyhow::anyhow!("CONFIG_SYNTAX: invalid target inventory"))?;
    for (name, target) in &raw {
        if let Some(fields) = target.as_mapping() {
            for key in fields.keys().filter_map(serde_yaml::Value::as_str) {
                if ![
                    "uses",
                    "path",
                    "depends_on",
                    "materialize",
                    "platform",
                    "matrix",
                    "container",
                    "bindings",
                    "dependencies",
                ]
                .contains(&key)
                {
                    diagnostics.push(sources::warning(
                        "build.yaml",
                        &format!("{name}.{key}"),
                        "unknown optional inventory field",
                    ));
                }
            }
        }
    }
    let values: BTreeMap<String, TargetConfig> = serde_yaml::from_str(&text)
        .map_err(|_| anyhow::anyhow!("CONFIG_INVALID_VALUE: invalid build.yaml target field"))?;
    if values.len() > 1024 {
        bail!("CONFIG_LIMIT: too many targets");
    }
    if values.is_empty() {
        bail!("build.yaml has no targets");
    }
    let mut names = std::collections::BTreeSet::new();
    for (name, config) in &values {
        if let Some(container) = &config.container {
            container.validate()?;
        }
        if !crate::names::valid(name) {
            bail!("invalid target name {name}");
        }
        if !names.insert(name.to_lowercase()) {
            bail!("case-colliding target {name}");
        }
        if config.platform.is_some() && config.matrix.contains_key("platform") {
            bail!("{name}: platform and matrix.platform conflict");
        }
        for (axis, values) in &config.matrix {
            if !["platform", "python", "node", "go", "rust", "java"].contains(&axis.as_str())
                || values.is_empty()
                || values
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != values.len()
            {
                bail!("CONFIG_INVALID_VALUE: unsupported or repeated matrix values");
            }
        }
        if config
            .matrix
            .keys()
            .filter(|axis| axis.as_str() != "platform")
            .count()
            > 1
        {
            bail!("CONFIG_INVALID_VALUE: multiple language axes");
        }
        for dep in config
            .depends_on
            .iter()
            .chain(config.materialize.iter().map(|m| &m.from))
        {
            if !values.contains_key(dep) {
                bail!("{name}: unknown dependency {dep}");
            }
        }
    }
    use sha2::{Digest, Sha256};
    let source_digest = Some(format!("sha256:{:x}", Sha256::digest(text.as_bytes())));
    Ok((
        BuildInventory {
            targets: values,
            source_digest,
        },
        diagnostics,
    ))
}

pub fn project(root: &Path) -> Result<ProjectConfig> {
    let session = session::Session::open(
        root,
        &session::Options {
            root: Some(root.into()),
            ..Default::default()
        },
    )?;
    project_from_effective(&session.resolve(root, false)?)
}

pub fn project_from_effective(effective: &resolve::EffectiveConfig) -> Result<ProjectConfig> {
    let mut result = ProjectConfig::default();
    for (key, value) in effective.values() {
        if let Some(name) = key.strip_prefix("env.") {
            let literal = value.as_str().context(
                "CONFIG_INVALID_VALUE: secret reference requires an authorized secret consumer",
            )?;
            result.env.insert(name.into(), literal.into());
        } else if let Some(name) = key.strip_prefix("tasks.") {
            result
                .tasks
                .insert(name.into(), serde_json::from_value(value.clone())?);
        } else if let Some(name) = key.strip_prefix("tools.") {
            if !matches!(name, "allowed" | "catalogs") {
                result
                    .tools
                    .insert(name.into(), serde_json::from_value(value.clone())?);
            }
        }
    }
    Ok(result)
}

/// Resolve a declaring-file-relative execution path, then enforce its workspace boundary.
pub fn execution_path(root: &Path, declaring: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute() || relative.to_string_lossy().contains(':') {
        bail!("CONFIG_SCOPE: execution paths must be relative");
    }
    let path = declaring
        .join(relative)
        .canonicalize()
        .context("CONFIG_INVALID_VALUE: missing execution path")?;
    if !path.starts_with(root.canonicalize()?) {
        bail!("CONFIG_SCOPE: execution path escapes workspace");
    }
    Ok(path)
}
