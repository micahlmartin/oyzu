//! Oyzu-owned tool identity. Inspection is read-only and confers no install or
//! execution authority; backend admission and receipt validation are separate.
mod lock;
mod store;
pub use store::{TreeEntry, TreeInspection};

use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::{fs::File, io::Read, path::Path};

#[derive(Debug, Serialize)]
pub struct LockInspection {
    pub format: u32,
    pub environments: usize,
    pub tools: usize,
    pub platforms: Vec<String>,
    pub selections: Vec<LockedSelection>,
    pub validation: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct LockedSelection {
    pub scope: String,
    pub profile: String,
    pub platform: String,
    pub digest: String,
    pub installation_keys: std::collections::BTreeMap<String, String>,
}

/// Validate a bounded format-2 document without discovery, network, execution or
/// mutation. This checks structure and content identities, not source trust,
/// current configuration compatibility, installed content or authorization.
pub fn inspect_lock(path: &Path) -> Result<LockInspection> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    ensure!(
        file.metadata()?.is_file(),
        "tool lock must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(lock::MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let document = lock::parse(&bytes)?;
    Ok(LockInspection {
        format: document.format,
        environments: document.environment.len(),
        tools: document.tool.len(),
        platforms: document
            .tool
            .iter()
            .flat_map(|tool| tool.distribution.iter())
            .map(|distribution| distribution.platform.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
        validation: "structure-and-identity-only",
        selections: document.selections,
    })
}

/// Observe payload content through no-follow filesystem handles. This does not
/// create an installation receipt or grant permission to execute the payload.
pub fn inspect_tree(path: &Path) -> Result<TreeInspection> {
    store::inspect(&std::path::absolute(path)?)
}

/// Materialize verified tar bytes into an empty operation-owned staging
/// directory. On failure the caller must discard staging. This does not publish
/// an installation, produce a receipt, execute code or grant selection authority.
/// GNU tar and gzip are supported; PAX, hardlinks and Windows symlinks are not yet
/// admitted. Source and staging must not be concurrently writable by other users.
pub fn materialize_archive(
    source: &Path,
    staging: &Path,
    digest: &str,
    size: u64,
    gzip: bool,
) -> Result<TreeInspection> {
    store::materialize(
        &std::path::absolute(source)?,
        &std::path::absolute(staging)?,
        digest,
        size,
        gzip,
    )
}

/// Fully rehash every installation in a locked selection and compare closed
/// receipts against that lock and a caller-trusted installer release identity.
/// This verifies content bindings only, not layout admission or execution grants.
pub fn verify_installation_selection(
    lock_path: &Path,
    store: &Path,
    scope: &str,
    profile: &str,
    platform: &str,
    installer: &str,
) -> Result<String> {
    let mut bytes = Vec::new();
    File::open(lock_path)?
        .take(lock::MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let lock = lock::parse(&bytes)?;
    store::verify(
        &lock,
        &std::path::absolute(store)?,
        scope,
        profile,
        platform,
        installer,
    )
}
