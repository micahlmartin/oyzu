//! Capture locked crates.io inputs in Cargo's native local-registry layout.
//! No project code runs here; Cargo owns resolution, extraction and compilation.
use crate::{broker, snapshot};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

pub(super) const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

#[derive(Deserialize)]
struct Lock {
    package: Vec<LockedPackage>,
}

#[derive(Deserialize)]
struct LockedPackage {
    name: String,
    version: String,
    source: Option<String>,
    checksum: Option<String>,
}

pub(super) fn capture(lock: &Path, destination: &Path) -> Result<Vec<Value>> {
    let mut fetcher = broker::Fetcher::new(vec![
        broker::Source::new("cargo-index", "https://index.crates.io/", None)?,
        broker::Source::new("cargo-archives", "https://static.crates.io/crates/", None)?,
    ])?;
    capture_with(lock, destination, |url| {
        let response = fetcher.fetch(url)?;
        if response.status != 200 {
            bail!("Cargo approved source returned HTTP {}", response.status);
        }
        Ok(response.body)
    })
}

fn capture_with(
    lock: &Path,
    destination: &Path,
    mut fetch: impl FnMut(&str) -> Result<Vec<u8>>,
) -> Result<Vec<Value>> {
    let lock: Lock = toml::from_str(&fs::read_to_string(lock)?)?;
    let mut packages = BTreeMap::new();
    // Admit the complete lock before allowing the first upstream request.
    for package in lock.package {
        let Some(source) = &package.source else {
            continue;
        };
        if source != CRATES_IO {
            bail!("Cargo source acquisition is not implemented for {source}");
        }
        if package.name.is_empty()
            || package.name.len() > 64
            || !package
                .name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
            || package.version.is_empty()
            || package.version.len() > 128
            || !package
                .version
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'-' | b'+'))
        {
            bail!("invalid Cargo registry package identity");
        }
        let checksum = package
            .checksum
            .as_deref()
            .context("locked Cargo registry package lacks checksum")?;
        if checksum.len() != 64
            || !checksum
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            bail!("invalid locked Cargo archive checksum");
        }
        if packages
            .insert(
                (package.name.to_ascii_lowercase(), package.version.clone()),
                package,
            )
            .is_some()
        {
            bail!("duplicate locked Cargo registry package");
        }
    }
    let registry = destination.join("registry");
    fs::create_dir_all(registry.join("index"))?;
    let mut indexes: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut selected: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut inventory = Vec::new();
    for ((name, _), package) in packages {
        let relative = index_path(&name);
        if !indexes.contains_key(&name) {
            indexes.insert(
                name.clone(),
                fetch(&format!("https://index.crates.io/{relative}"))?,
            );
        }
        let mut matching = Vec::new();
        for line in indexes[&name]
            .split(|c| *c == b'\n')
            .filter(|l| !l.is_empty())
        {
            let entry: Value = serde_json::from_slice(line)?;
            if entry["name"] == package.name && entry["vers"] == package.version {
                if entry["cksum"].as_str() != package.checksum.as_deref() {
                    bail!(
                        "Cargo index checksum disagrees with lock for {}",
                        package.name
                    );
                }
                matching.push(line);
            }
        }
        if matching.len() != 1 {
            bail!(
                "Cargo index must contain exactly one locked version of {}",
                package.name
            );
        }
        let file = format!("{}-{}.crate", package.name, package.version);
        let bytes = fetch(&format!(
            "https://static.crates.io/crates/{}/{file}",
            package.name
        ))?;
        let path = registry.join(&file);
        fs::write(&path, &bytes)?;
        let digest = snapshot::file_digest(&path)?;
        if digest != format!("sha256:{}", package.checksum.as_deref().unwrap()) {
            bail!(
                "Cargo archive checksum disagrees with lock for {}",
                package.name
            );
        }
        let index = selected.entry(relative).or_default();
        index.extend_from_slice(matching[0]);
        index.push(b'\n');
        inventory.push(json!({"id":format!("cargo/package-{}",inventory.len()),"name":package.name,
            "version":package.version,"sourceId":"cargo-archives","digest":digest,"size":bytes.len(),
            "purpose":"build","dependencies":[],"verification":"digest-only"}));
    }
    for (relative, bytes) in selected {
        let path = registry.join("index").join(relative);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, bytes)?;
    }
    // An empty replacement hides Cargo's temporary registry of unpublished
    // workspace packages. No registry input means no replacement is needed:
    // execution still uses a private Cargo home and locked, offline commands.
    let config = if inventory.is_empty() {
        "# No external registry packages in the captured lock.\n"
    } else {
        "[source.crates-io]\nreplace-with = 'oyzu-captured'\n[source.oyzu-captured]\nlocal-registry = '/dependencies/registry'\n"
    };
    fs::write(destination.join("cargo-config.toml"), config)?;
    Ok(inventory)
}

fn index_path(name: &str) -> String {
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    }
}

#[cfg(test)]
mod tests;
