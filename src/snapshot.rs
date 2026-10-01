//! Captured source trees for the build engine; never a live view of the checkout.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::Path,
};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    pub size: u64,
    pub executable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub digest: String,
    pub entries: Vec<Entry>,
}

fn included(entry: &DirEntry) -> bool {
    entry.depth() == 0
        || !matches!(
            entry.file_name().to_str(),
            Some(
                ".git"
                    | ".oyzu"
                    | "node_modules"
                    | "dist"
                    | "target"
                    | ".venv"
                    | "__pycache__"
                    | ".pytest_cache"
                    | ".gradle"
                    | ".events"
            )
        )
}

pub(crate) fn portable(value: &str) -> bool {
    !value.is_empty()
        && value.split('/').all(|part| {
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                    && stem.len() == 4
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'));
            !device
                && part != ".."
                && part != "."
                && !part.is_empty()
                && !part.ends_with(['.', ' '])
                && !part.chars().any(|c| {
                    c.is_control() || matches!(c, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
                })
        })
}

pub fn file_digest(path: &Path) -> Result<String> {
    let mut source = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// Destination must be new and outside source. Failed capture is never reusable.
pub fn capture(source: &Path, destination: &Path) -> Result<Snapshot> {
    capture_tree(source, destination, true)
}

/// Prepared repositories are complete inputs: source-tree ignore rules do not
/// apply to native package coordinates or resolver metadata.
pub(crate) fn capture_prepared(source: &Path, destination: &Path) -> Result<Snapshot> {
    capture_tree(source, destination, false)
}

fn capture_tree(source: &Path, destination: &Path, source_rules: bool) -> Result<Snapshot> {
    let source = source.canonicalize()?;
    if destination.exists() {
        bail!("snapshot destination already exists");
    }
    let parent = destination
        .parent()
        .context("snapshot destination requires a parent")?
        .canonicalize()?;
    if parent.starts_with(&source) {
        bail!("snapshot destination cannot be inside source");
    }
    fs::create_dir(destination)?;
    let mut entries = vec![];
    let mut paths = BTreeSet::new();
    let mut size = 0u64;
    for item in WalkDir::new(&source)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !source_rules || included(entry))
    {
        let item = item?;
        if item.depth() == 0 {
            continue;
        }
        if entries.len() >= 100_000 {
            bail!("source exceeds 100000 captured entries");
        }
        let relative = item.path().strip_prefix(&source)?;
        let relative = relative
            .to_str()
            .context("source path is not UTF-8")?
            .replace('\\', "/");
        if !portable(&relative) {
            bail!("nonportable source path {relative}");
        }
        if !paths.insert(relative.to_lowercase()) {
            bail!("case-colliding source path {relative}");
        }
        let out = destination.join(&relative);
        let metadata = fs::symlink_metadata(item.path())?;
        if metadata.file_type().is_symlink() {
            bail!("source symlink capture is not yet supported: {relative}");
        }
        if metadata.is_dir() {
            fs::create_dir(&out)?;
            entries.push(Entry {
                path: relative,
                kind: "directory".into(),
                digest: None,
                size: 0,
                executable: false,
            });
        } else if metadata.is_file() {
            size = size
                .checked_add(metadata.len())
                .context("source byte limit overflow")?;
            if size > 10 * 1024 * 1024 * 1024 {
                bail!("source exceeds 10 GiB capture limit");
            }
            let mut input = fs::File::open(item.path())?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&out)?;
            let mut hasher = Sha256::new();
            let mut buffer = [0u8; 65536];
            let mut copied = 0u64;
            loop {
                let count = input.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                copied += count as u64;
                if copied > metadata.len() {
                    bail!("source changed during capture: {relative}");
                }
                hasher.update(&buffer[..count]);
                output.write_all(&buffer[..count])?;
            }
            output.sync_all()?;
            let digest = format!("sha256:{:x}", hasher.finalize());
            if copied != metadata.len() || file_digest(item.path())? != digest {
                bail!("source changed during capture: {relative}");
            }
            #[cfg(unix)]
            let executable = {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            };
            #[cfg(not(unix))]
            let executable = false;
            fs::set_permissions(&out, metadata.permissions())?;
            entries.push(Entry {
                path: relative,
                kind: "file".into(),
                digest: Some(digest),
                size: copied,
                executable,
            });
        } else {
            bail!("unsupported source file type: {relative}");
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let normalized = serde_json::to_value(&entries)?;
    let mut hasher = Sha256::new();
    hasher.update(b"oyzu.tree.v1alpha1\0");
    hasher.update(serde_json_canonicalizer::to_vec(&normalized)?);
    Ok(Snapshot {
        digest: format!("sha256:{:x}", hasher.finalize()),
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepared_repository_names_are_never_filtered_as_source_outputs() {
        let input = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        fs::create_dir_all(input.path().join("repository/example/target/1.0")).unwrap();
        let jar = input
            .path()
            .join("repository/example/target/1.0/target-1.0.jar");
        fs::write(&jar, "first acquired bytes").unwrap();
        let first = capture_prepared(input.path(), &output.path().join("first")).unwrap();
        assert!(first
            .entries
            .iter()
            .any(|e| e.path.ends_with("target-1.0.jar")));
        fs::write(jar, "different acquired bytes").unwrap();
        let second = capture_prepared(input.path(), &output.path().join("second")).unwrap();
        assert_ne!(first.digest, second.digest);
    }
}
