//! Frozen scope selection; backend resolution and policy admission are separate.
use super::{lock, read_record, LockedSelection};
use anyhow::{ensure, Context, Result};
use std::path::Path;

/// Choose the nearest locked environment for an already resolved workspace,
/// physical working directory, profile, effective request digest and platform.
/// Reads only workspace/oyzu.lock; never resolves versions, installs or edits.
/// The caller must derive the digest from effective configuration and native
/// constraints. A matching digest is not authorization or installed-content proof.
pub fn select_locked_environment(
    workspace: &Path,
    directory: &Path,
    profile: &str,
    request_digest: &str,
    platform: &str,
) -> Result<LockedSelection> {
    lock::digest(request_digest)?;
    let workspace = workspace
        .canonicalize()
        .context("tool workspace is unavailable")?;
    let directory = directory
        .canonicalize()
        .context("tool working directory is unavailable")?;
    ensure!(
        workspace.is_dir() && directory.is_dir(),
        "tool scope must be a directory"
    );
    let relative = directory
        .strip_prefix(&workspace)
        .context("tool working directory escapes workspace")?;
    let components = relative
        .components()
        .map(|part| {
            part.as_os_str()
                .to_str()
                .context("tool scope must be UTF-8")
        })
        .collect::<Result<Vec<_>>>()?;
    let scope = components.join("/");
    let lock = lock::parse(&read_record(&workspace.join("oyzu.lock"), lock::MAX_BYTES)?)?;
    let mut candidates = std::collections::BTreeMap::new();
    for environment in lock.environment.iter().filter(|env| env.profile == profile) {
        let physical = match workspace.join(&environment.scope).canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let lexical_ancestor = environment.scope == "."
                    || environment.scope == scope
                    || scope
                        .strip_prefix(&environment.scope)
                        .is_some_and(|rest| rest.starts_with('/'));
                ensure!(
                    !lexical_ancestor,
                    "TOOL_LOCK_STALE: locked ancestor disappeared"
                );
                continue;
            }
            Err(error) => return Err(error).context("locked scope is unavailable"),
        };
        ensure!(
            physical.starts_with(&workspace) && physical.is_dir(),
            "TOOL_LOCK_STALE: locked scope escapes workspace or is not a directory"
        );
        if directory.starts_with(&physical) {
            let depth = physical.components().count();
            ensure!(
                candidates.insert(depth, environment).is_none(),
                "TOOL_LOCK_AMBIGUOUS: multiple locked scopes name the same physical ancestor"
            );
        }
    }
    let environment = candidates
        .last_key_value()
        .map(|(_, value)| *value)
        .context("TOOL_LOCK_MISSING: no locked environment for profile and scope")?;
    ensure!(environment.request_digest == request_digest,
        "TOOL_LOCK_STALE: nearest locked environment differs from effective requests; explicitly update the lock");
    lock.selections.into_iter().find(|selection| selection.scope == environment.scope
        && selection.profile == profile && selection.platform == platform)
        .context("TOOL_PLATFORM_UNAVAILABLE: nearest locked environment has no complete target selection")
}
