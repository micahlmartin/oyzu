//! Oyzu-owned tool identity, verified storage and internal control contracts.
//! Inspection is read-only; cache/publication mutations require caller admission.
//! Grant checks and worker framing rely on independently trusted caller context;
//! none of these library operations alone confers execution authority.
mod backend;
mod edit;
mod grant;
pub use grant::{ToolGrantContext, ToolGrantOperation, VerifiedToolGrant};
mod lock;
mod requests;
mod selection;
mod worker;
pub use backend::{inspect as inspect_backend, BackendInspection};
pub use edit::{ToolLockChange, ToolLockChangeKind, ToolLockEdit, ToolLockProposal};
pub use requests::{project_tool_requests, ToolRequestIdentity};
pub use selection::{select_for_tool_requests, select_locked_environment};
pub use worker::{
    native_tool_worker_channel, split_tool_worker_channel, NativeToolWorkerEndpoint,
    NativeToolWorkerIo, NativeToolWorkerReader, NativeToolWorkerWriter, ToolWorkerChannel,
    ToolWorkerExchange, ToolWorkerOperation, ToolWorkerOutcome, ToolWorkerReceiver,
    ToolWorkerRequestContext, ToolWorkerSender, ToolWorkerSession,
};
mod store;
pub use store::InstallationLease;
pub use store::LeaseRecovery;
pub use store::VerifiedBlob;
pub use store::{LeasedToolCommand, ToolArgument, ToolInstallPath, ToolLaunch};
pub use store::{TreeEntry, TreeInspection};

use anyhow::{ensure, Context, Result};
use serde::Serialize;
use std::{fs::OpenOptions, io::Read, path::Path};

/// Reap only final process-lease records whose journal, mutation and installation
/// lease locks are all available. Never deletes installations or lock files.
/// Unknown, old unguarded, pending, redirected and malformed records are retained.
/// Dry-run obtains transient locks but writes no files. This is a cooperative
/// store operation, not PID-based liveness detection or full store recovery.
pub fn recover_tool_leases(store: &Path, dry_run: bool) -> Result<LeaseRecovery> {
    store::recover(&std::path::absolute(store)?, dry_run)
}

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
    let bytes = read_record(request.lock_path, lock::MAX_BYTES)?;
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
    let bytes = read_record(path, lock::MAX_BYTES)?;
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
    let bytes = read_record(lock_path, lock::MAX_BYTES)?;
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
/// A flushed process-lease journal is required before success. Normal drop removes
/// it before releasing OS leases; crashes or cleanup failure leave stale evidence
/// for recovery, never proof that a process remains alive.
pub fn lease_installation_selection(
    lock_path: &Path,
    store: &Path,
    staging: Option<&Path>,
    scope: &str,
    profile: &str,
    platform: &str,
    installer: &str,
) -> Result<InstallationLease> {
    let bytes = read_record(lock_path, lock::MAX_BYTES)?;
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

/// Bounded tool metadata input, not the store's anchored content access. Unix
/// nonblocking open lets us reject FIFOs before reading, including replacements
/// between a caller's path check and open. Regular-file symlinks remain supported.
fn read_record(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    ensure!(
        file.metadata()?.is_file(),
        "tool record must be a regular file"
    );
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= limit, "tool record exceeds byte limit");
    Ok(bytes)
}

// Shared canonical request identity for tool control protocols.
fn valid_request_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
