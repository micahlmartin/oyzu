//! Oyzu-owned tool identity and verified content storage. Inspection is read-only;
//! cache/publication mutations require caller admission and confer no execution
//! authority. Backend admission and current policy remain separate gates.
mod lock;
mod store;
pub use store::InstallationLease;
pub use store::VerifiedBlob;
pub use store::{TreeEntry, TreeInspection};

use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::{fs::File, io::Read, path::Path};

/// Caller-owned candidate staging and reviewed release identities. The admission
/// digest must come from trusted compiled backend data, never project input.
pub struct ToolCandidateRequest<'a> {
    pub lock_path: &'a Path,
    pub staging: &'a Path,
    pub scope: &'a str,
    pub profile: &'a str,
    pub platform: &'a str,
    pub tool_key: &'a str,
    pub installer_release_digest: &'a str,
    pub admitted_layout_digest: &'a str,
}

/// Materialize a locked data-only layout and write an uncommitted candidate
/// receipt. Does not verify publisher signatures, publish, execute or authorize.
/// Complete selection verification is still required before publication.
pub fn stage_tool_candidate(
    request: ToolCandidateRequest<'_>,
    plan: &[u8],
    blob: VerifiedBlob,
) -> Result<String> {
    let mut bytes = Vec::new();
    File::open(request.lock_path)?
        .take(lock::MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let lock = lock::parse(&bytes)?;
    store::stage(request, plan, blob, &lock)
}

/// Reverify a cache hit or stream exact locked bytes into the content-addressed
/// store. Cache hits never read `source`; corruption never triggers fallback.
/// The returned private snapshot remains independent of later cache mutations.
/// Caller must authorize acquisition and provide a bounded/cancellable transport;
/// this function bounds bytes but cannot interrupt an arbitrary blocking reader.
pub fn cache_tool_blob(
    store: &Path,
    source: &mut dyn Read,
    digest: &str,
    size: u64,
) -> Result<VerifiedBlob> {
    store::cache(&std::path::absolute(store)?, source, digest, size)
}

/// Consume verified private bytes into empty caller-owned staging without
/// reopening the cache path. No receipt, admission or execution grant is created.
pub fn materialize_tool_blob(
    blob: VerifiedBlob,
    staging: &Path,
    gzip: bool,
) -> Result<TreeInspection> {
    store::materialize_blob(blob, &std::path::absolute(staging)?, gzip)
}

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

/// Verify an exact closure, optionally publish missing members from an owned
/// staging store, and retain OS leases. Caller must separately admit the layout,
/// backend and authorization. Staging has installs/<key-hex>/{receipt.json,payload}.
/// No existing installation is overwritten. A partial publication failure leaves
/// only individually committed, unreferenced entries; it never edits the lock.
pub fn lease_installation_selection(
    lock_path: &Path,
    store: &Path,
    staging: Option<&Path>,
    scope: &str,
    profile: &str,
    platform: &str,
    installer: &str,
) -> Result<InstallationLease> {
    let mut bytes = Vec::new();
    File::open(lock_path)?
        .take(lock::MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let lock = lock::parse(&bytes)?;
    let staging = staging.map(std::path::absolute).transpose()?;
    store::transact(
        &lock,
        &std::path::absolute(store)?,
        staging.as_deref(),
        scope,
        profile,
        platform,
        installer,
    )
}
