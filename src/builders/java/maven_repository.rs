//! Inventory native Maven-layout files, independent of the consuming JVM manager.
use crate::snapshot::Snapshot;
use anyhow::{bail, Result};
use serde_json::{json, Value};

pub(super) fn inventory(tree: &Snapshot, source_id: &str) -> Result<Vec<Value>> {
    tree.entries.iter().filter(|entry| {
        entry.kind == "file" && entry.path.starts_with("repository/")
            && [".jar", ".pom", ".module"].iter().any(|extension| entry.path.ends_with(extension))
    }).map(|entry| {
        let coordinate = entry.path.strip_prefix("repository/").unwrap();
        let parts: Vec<_> = coordinate.split('/').collect();
        if parts.len() < 4 { bail!("invalid native Maven repository coordinate"); }
        let version = parts[parts.len() - 2];
        let name = format!("{}:{}", parts[..parts.len() - 3].join("."), parts[parts.len() - 3]);
        Ok(json!({"id":entry.path,"name":name,"version":version,"sourceId":source_id,
            "digest":entry.digest,"size":entry.size,"purpose":"build","dependencies":[],"verification":"digest-only"}))
    }).collect()
}
