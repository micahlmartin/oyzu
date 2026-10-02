use super::access::{self, Directory, Kind};
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    io::Read,
    path::Path,
};

const MAX_ENTRIES: usize = 200_000;
const MAX_FILE: u64 = 1024 * 1024 * 1024;
const MAX_BYTES: u64 = 8 * MAX_FILE;
const MAX_NAMES: usize = 32 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct TreeInspection {
    pub digest: String,
    pub manifest_digest: String,
    pub bytes: u64,
    pub entries: Vec<TreeEntry>,
    pub validation: &'static str,
}

#[derive(Debug, Serialize)]
pub struct TreeEntry {
    pub path: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub executable: u32,
    pub size: Option<u64>,
    pub digest: Option<String>,
    pub target: Option<String>,
}

/// Hash an existing tree through no-follow handles. This operation never follows
/// payload links or executes code. All internal link chains are checked against
/// the completed manifest. Its result is an observation, not a trusted receipt.
pub(in crate::tools) fn inspect(path: &Path) -> Result<TreeInspection> {
    let directory = Directory::open(path)?;
    let mut scan = Scan::default();
    scan.walk(&directory, "", 0)?;
    for (expected, observed) in scan.links.values() {
        ensure!(
            expected == observed,
            "payload hardlink reaches an object outside the payload"
        );
    }
    scan.entries.sort_by(|a, b| a.path.cmp(&b.path));
    validate_links(&scan.entries)?;
    let value = serde_json::to_value(&scan.entries)?;
    let canonical = serde_json_canonicalizer::to_vec(&value)?;
    Ok(TreeInspection {
        digest: crate::records::digest("oyzu.tool-tree.v1", &value)?,
        manifest_digest: format!("sha256:{:x}", Sha256::digest(canonical)),
        bytes: scan.bytes,
        entries: scan.entries,
        validation: "payload-observation-only",
    })
}

#[derive(Default)]
struct Scan {
    entries: Vec<TreeEntry>,
    folded: BTreeSet<String>,
    links: BTreeMap<(u64, u64), (u64, u64)>,
    bytes: u64,
    name_bytes: usize,
}

impl Scan {
    fn walk(&mut self, directory: &Directory, prefix: &str, depth: usize) -> Result<()> {
        ensure!(depth <= 64, "payload depth exceeds 64");
        for (name, kind) in directory.entries()? {
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            access::relative(&path)?;
            ensure!(
                self.entries.len() < MAX_ENTRIES,
                "payload entry limit exceeded"
            );
            ensure!(
                self.folded.insert(path.to_uppercase()),
                "case-colliding payload paths"
            );
            self.name_bytes += path.len();
            ensure!(self.name_bytes <= MAX_NAMES, "payload path budget exceeded");
            let mut record = TreeEntry {
                path: path.clone(),
                kind: "directory",
                executable: 0,
                size: None,
                digest: None,
                target: None,
            };
            match kind {
                Kind::Directory => {
                    let child = directory.child(&name)?;
                    record.executable = child.executable()?;
                    self.entries.push(record);
                    self.walk(&child, &path, depth + 1)?;
                }
                Kind::File => {
                    let mut file = directory.file(&name)?;
                    let before = access::identity(&file)?;
                    ensure!(before.size <= MAX_FILE, "payload file exceeds 1 GiB");
                    ensure!(
                        self.bytes + before.size <= MAX_BYTES,
                        "payload exceeds 8 GiB"
                    );
                    let mut hasher = Sha256::new();
                    let mut size = 0;
                    let mut buffer = [0u8; 64 * 1024];
                    loop {
                        let length = file.read(&mut buffer)?;
                        if length == 0 {
                            break;
                        }
                        size += length as u64;
                        ensure!(
                            size <= before.size,
                            "payload file changed during verification"
                        );
                        hasher.update(&buffer[..length]);
                    }
                    let after = access::identity(&file)?;
                    ensure!(
                        size == before.size
                            && before.size == after.size
                            && before.object == after.object
                            && before.links == after.links
                            && before.executable == after.executable,
                        "payload file changed during verification"
                    );
                    let links = self.links.entry(before.object).or_insert((before.links, 0));
                    ensure!(links.0 == before.links, "payload hardlink count changed");
                    links.1 += 1;
                    self.bytes += size;
                    record.kind = "file";
                    record.executable = before.executable;
                    record.size = Some(size);
                    record.digest = Some(format!("sha256:{:x}", hasher.finalize()));
                    self.entries.push(record);
                }
                Kind::Symlink => {
                    let target = directory.link(&name)?;
                    let target = target.to_str().context("symlink target must be UTF-8")?;
                    #[cfg(windows)]
                    let target = target.replace('\\', "/");
                    let target = target.to_string();
                    validate_target(&target)?;
                    self.name_bytes += target.len();
                    ensure!(self.name_bytes <= MAX_NAMES, "payload path budget exceeded");
                    record.kind = "symlink";
                    record.target = Some(target);
                    self.entries.push(record);
                }
            }
        }
        Ok(())
    }
}

fn validate_target(target: &str) -> Result<()> {
    ensure!(
        !target.is_empty() && !target.starts_with('/') && target.len() <= 16 * 1024,
        "symlink target must be a bounded relative path"
    );
    for component in target.split('/') {
        if component != "." && component != ".." {
            access::component(component)?;
        }
    }
    Ok(())
}

fn validate_links(entries: &[TreeEntry]) -> Result<()> {
    let index: BTreeMap<_, _> = entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    for entry in entries.iter().filter(|entry| entry.kind == "symlink") {
        let mut remaining: VecDeque<_> = entry.path.split('/').map(str::to_owned).collect();
        let mut resolved = Vec::new();
        let mut expansions = 0;
        // Resolve component-by-component, including .. AFTER expanding links.
        // Lexically collapsing x/.. first would miss a symlink-chain escape.
        while let Some(component) = remaining.pop_front() {
            if component == "." {
                continue;
            }
            if component == ".." {
                ensure!(resolved.pop().is_some(), "symlink chain escapes payload");
                continue;
            }
            resolved.push(component);
            ensure!(resolved.len() <= 64, "symlink chain exceeds path depth");
            let key = resolved.join("/");
            let target = index
                .get(key.as_str())
                .context("dangling payload symlink")?;
            if let Some(link) = &target.target {
                expansions += 1;
                ensure!(expansions <= 64, "symlink cycle or excessive chain");
                resolved.pop();
                for component in link.split('/').rev() {
                    remaining.push_front(component.to_owned());
                }
            } else {
                ensure!(
                    remaining.is_empty() || target.kind == "directory",
                    "symlink traverses a non-directory"
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
