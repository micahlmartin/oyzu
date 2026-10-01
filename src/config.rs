use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub container: Option<serde_yaml::Value>,
    pub bindings: Option<serde_yaml::Value>,
    pub dependencies: Option<String>,
}

#[derive(Debug, Deserialize)]
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
    pub reports: Vec<serde_json::Value>,
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
    let path = root.join("build.yaml");
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)?;
    if text.len() > 1024 * 1024 {
        bail!("configuration exceeds 1 MiB: {}", path.display());
    }
    let values: BTreeMap<String, TargetConfig> =
        serde_yaml::from_str(&text).context("invalid build.yaml")?;
    if values.is_empty() {
        bail!("build.yaml has no targets");
    }
    let mut names = std::collections::BTreeSet::new();
    for (name, config) in &values {
        if name.is_empty()
            || name.len() > 64
            || !name.as_bytes()[0].is_ascii_alphabetic()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            bail!("invalid target name {name}");
        }
        if !names.insert(name.to_lowercase()) {
            bail!("case-colliding target {name}");
        }
        if config.platform.is_some() && config.matrix.contains_key("platform") {
            bail!("{name}: platform and matrix.platform conflict");
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
    Ok(Some(values))
}

pub fn project(root: &Path) -> Result<ProjectConfig> {
    let mut result = ProjectConfig::default();
    for filename in ["oyzu.toml", "oyzu.local.toml"] {
        if filename == "oyzu.local.toml" && std::env::var_os("CI").is_some() {
            continue;
        }
        let path = root.join(filename);
        if !path.is_file() {
            continue;
        }
        let value: ProjectConfig = toml::from_str(&fs::read_to_string(&path)?)
            .with_context(|| format!("invalid {}", path.display()))?;
        result.env.extend(value.env);
        result.tasks.extend(value.tasks);
        result.tools.extend(value.tools);
        if value.cache.is_some() {
            result.cache = value.cache;
        }
    }
    Ok(result)
}
