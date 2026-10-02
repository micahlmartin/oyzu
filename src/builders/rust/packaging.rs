//! Order native workspace packages for an isolated writable registry overlay.
use super::metadata::{Metadata, Package};
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn command(metadata: &Metadata) -> Result<Option<Vec<String>>> {
    let members: Vec<_> = metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .collect();
    let has_registry = metadata.packages.iter().any(|p| p.source.is_some())
        || members
            .iter()
            .any(|p| p.dependencies.iter().any(|d| d.source.is_some()));
    if !has_registry
        || !members
            .iter()
            .any(|p| p.dependencies.iter().any(|d| d.path.is_some()))
    {
        return Ok(None);
    }
    let mut remaining: BTreeMap<&str, (&Package, BTreeSet<&str>)> = BTreeMap::new();
    for member in &members {
        let mut dependencies = BTreeSet::new();
        for dependency in &member.dependencies {
            let Some(path) = &dependency.path else {
                continue;
            };
            let manifest = format!("{path}/Cargo.toml");
            let owner = members
                .iter()
                .find(|p| p.manifest_path == manifest)
                .context("Cargo package dependency has no captured workspace owner")?;
            dependencies.insert(owner.name.as_str());
        }
        if remaining
            .insert(&member.name, (member, dependencies))
            .is_some()
        {
            bail!("duplicate Cargo workspace package name");
        }
    }
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let ready: Vec<_> = remaining
            .iter()
            .filter(|(_, (_, dependencies))| dependencies.is_empty())
            .map(|(name, _)| *name)
            .collect();
        if ready.is_empty() {
            bail!("Cargo registry workspace packaging requires acyclic local package dependencies");
        }
        for name in ready {
            let (package, _) = remaining.remove(name).unwrap();
            ordered.push(json!({"name": package.name, "version": package.version}));
            for (_, dependencies) in remaining.values_mut() {
                dependencies.remove(name);
            }
        }
    }
    Ok(Some(vec![
        "python3".into(),
        "-I".into(),
        "/oyzu/rust-package.py".into(),
        "/dependencies/registry".into(),
        serde_json::to_string(&ordered)?,
        "cargo".into(),
        "package".into(),
        "--locked".into(),
        "--offline".into(),
        "--allow-dirty".into(),
        "--registry".into(),
        "crates-io".into(),
    ]))
}
