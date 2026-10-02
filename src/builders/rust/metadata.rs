//! Cargo owns workspace membership and dependency resolution; IDs remain opaque.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Deserialize, Serialize)]
pub(super) struct Metadata {
    pub packages: Vec<Package>,
    pub workspace_members: Vec<String>,
    pub workspace_root: String,
    #[serde(default)]
    pub resolve: Option<Resolution>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Resolution {
    pub nodes: Vec<Node>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Node {
    pub id: String,
    pub features: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manifest_path: String,
    pub dependencies: Vec<Dependency>,
    pub targets: Vec<Target>,
    pub source: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Dependency {
    pub name: String,
    pub source: Option<String>,
    pub path: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Target {
    pub name: String,
    pub kind: Vec<String>,
    pub src_path: String,
    #[serde(default, rename = "required-features")]
    pub required_features: Vec<String>,
}

pub(super) fn relative(path: &str) -> Result<&str> {
    let relative = path
        .strip_prefix("/workspace/")
        .context("Cargo path outside captured target")?;
    if relative.is_empty()
        || relative.contains(['\\', ':'])
        || relative
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        bail!("nonportable Cargo path {path}");
    }
    Ok(relative)
}

impl Metadata {
    /// Cargo's resolved package features determine which binary targets exist.
    pub fn binaries(&self) -> Result<Vec<(&Package, &Target)>> {
        let mut selected = Vec::new();
        for package in &self.packages {
            for target in &package.targets {
                if !target.kind.iter().any(|kind| kind == "bin") {
                    continue;
                }
                if !target.required_features.is_empty() {
                    let node = self
                        .resolve
                        .as_ref()
                        .and_then(|r| r.nodes.iter().find(|n| n.id == package.id))
                        .context("Cargo feature-gated binary requires resolved native features")?;
                    if !target
                        .required_features
                        .iter()
                        .all(|feature| node.features.contains(feature))
                    {
                        continue;
                    }
                }
                selected.push((package, target));
            }
        }
        Ok(selected)
    }

    pub fn validate(&self) -> Result<()> {
        if self.workspace_root != "/workspace" || self.packages.is_empty() {
            bail!("Cargo workspace must be rooted in the captured target");
        }
        for package in &self.packages {
            if package.source.is_some() || !self.workspace_members.contains(&package.id) {
                bail!("Cargo external dependency acquisition is not implemented yet");
            }
            relative(&package.manifest_path)?;
            for dependency in &package.dependencies {
                if dependency.source.is_some() {
                    bail!(
                        "Cargo registry/Git dependency acquisition is not implemented yet: {}",
                        dependency.name
                    );
                }
                let path = dependency
                    .path
                    .as_deref()
                    .context("unresolved Cargo path dependency")?;
                if path != "/workspace" {
                    relative(path)?;
                }
            }
            for target in &package.targets {
                relative(&target.src_path)?;
                if target.name.is_empty()
                    || !target
                        .name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    bail!("nonportable Cargo target name {}", target.name);
                }
            }
        }
        Ok(())
    }

    /// Change captured manifests only. Cargo subsequently regenerates and checks its lock.
    pub fn project_versions(&self, root: &Path, source_digest: &str) -> Result<Vec<String>> {
        let suffix = source_digest
            .strip_prefix("sha256:")
            .filter(|s| s.len() == 64)
            .context("invalid source digest")?;
        let versions: BTreeMap<_, _> = self
            .packages
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    format!(
                        "{}-dev.g{}",
                        p.version.split(['-', '+']).next().unwrap(),
                        &suffix[..12]
                    ),
                )
            })
            .collect();
        let mut manifests = vec!["Cargo.toml".to_owned()];
        for package in &self.packages {
            manifests.push(relative(&package.manifest_path)?.into());
        }
        manifests.sort();
        manifests.dedup();
        for path in &manifests {
            let file = root.join(path);
            let mut value: toml::Value = toml::from_str(&fs::read_to_string(&file)?)?;
            if let Some(package) = value.get_mut("package").and_then(toml::Value::as_table_mut) {
                let name = package
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .context("missing Cargo package name")?;
                let version = versions
                    .get(name)
                    .context("package missing from Cargo workspace metadata")?;
                package.insert("version".into(), toml::Value::String(version.clone()));
            }
            project_dependencies(&mut value, &versions);
            fs::write(&file, toml::to_string(&value)?)?;
        }
        Ok(manifests)
    }
}

fn project_dependencies(value: &mut toml::Value, versions: &BTreeMap<String, String>) {
    let Some(table) = value.as_table_mut() else {
        return;
    };
    for (key, value) in table {
        if matches!(
            key.as_str(),
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
            if let Some(dependencies) = value.as_table_mut() {
                for (alias, dependency) in dependencies {
                    let Some(dep) = dependency.as_table_mut() else {
                        continue;
                    };
                    if !dep.contains_key("path") {
                        continue;
                    }
                    let name = dep
                        .get("package")
                        .and_then(toml::Value::as_str)
                        .unwrap_or(alias);
                    if let Some(version) = versions.get(name) {
                        dep.insert("version".into(), toml::Value::String(format!("={version}")));
                    }
                }
            }
        } else {
            project_dependencies(value, versions);
        }
    }
}
