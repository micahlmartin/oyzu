//! Strict format-2 decoding and whole-document graph validation (OEP-0003).
//! No backend discovery, filesystem traversal or policy decisions occur here.
mod graph;
#[cfg(test)]
mod tests;

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Lock {
    pub format: u32,
    pub environment: Vec<Environment>,
    pub tool: Vec<Tool>,
    #[serde(skip)]
    pub selections: Vec<super::LockedSelection>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Environment {
    pub scope: String,
    pub profile: String,
    pub request_digest: String,
    pub roots: Vec<String>,
    pub requests: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tool {
    pub key: String,
    pub id: String,
    pub version: String,
    pub backend_digest: String,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
    pub distribution: Vec<Distribution>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Distribution {
    pub platform: String,
    pub digest: String,
    pub size: u64,
    pub source_id: String,
    pub artifact_id: String,
    pub layout_digest: String,
    pub verification: Verification,
    pub dependencies: Vec<String>,
    pub package_closure_digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Verification {
    pub kind: VerificationKind,
    pub evidence_digest: String,
    pub verifier_digest: String,
    pub subject_digest: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum VerificationKind {
    DigestOnly,
    PublisherSignature,
    Attestation,
}

pub(super) fn parse(bytes: &[u8]) -> Result<Lock> {
    ensure!(bytes.len() <= MAX_BYTES, "tool lock exceeds 8 MiB");
    let input = std::str::from_utf8(bytes).context("tool lock is not UTF-8")?;
    // Decode directly into closed records; TOML rejects duplicate keys before
    // constructing records. Keep the parser's default recursion limit enabled.
    let mut lock: Lock = toml::from_str(input).context("invalid format-2 tool lock")?;
    ensure!(lock.format == 2, "unsupported tool lock format {}; explicit re-resolution is required (automatic migration is disabled)", lock.format);
    ensure!(
        lock.environment.len() <= 1024,
        "too many locked environments"
    );
    ensure!(lock.tool.len() <= 4096, "too many locked tools");
    let mut environments = BTreeSet::new();
    for environment in &mut lock.environment {
        scope(&environment.scope)?;
        ensure!(
            crate::names::valid(&environment.profile),
            "invalid profile name"
        );
        ensure!(
            environments.insert((environment.scope.clone(), environment.profile.clone())),
            "duplicate locked scope/profile"
        );
        digest(&environment.request_digest)?;
        ensure!(environment.roots.len() <= 256, "too many environment roots");
        unique_sorted(&mut environment.roots)?;
        for root in &environment.roots {
            digest(root)?;
        }
        for (id, request) in &environment.requests {
            canonical_id(id)?;
            text(request, "request")?;
        }
    }
    let mut keys = BTreeSet::new();
    let mut edges = 0usize;
    for tool in &mut lock.tool {
        canonical_id(&tool.id)?;
        text(&tool.version, "version")?;
        digest(&tool.key)?;
        digest(&tool.backend_digest)?;
        for (name, value) in &tool.options {
            text(name, "option name")?;
            text(value, "option value")?;
        }
        ensure!(
            tool.key == tool_key(tool)?,
            "tool key does not match record identity"
        );
        ensure!(keys.insert(tool.key.clone()), "duplicate tool key");
        ensure!(
            !tool.distribution.is_empty() && tool.distribution.len() <= 64,
            "invalid distribution count"
        );
        let mut platforms = BTreeSet::new();
        for distribution in &mut tool.distribution {
            platform(&distribution.platform)?;
            ensure!(
                platforms.insert(distribution.platform.clone()),
                "duplicate tool platform"
            );
            ensure!(
                (1..=9_007_199_254_740_991).contains(&distribution.size),
                "distribution size outside canonical integer range"
            );
            digest(&distribution.digest)?;
            digest(&distribution.layout_digest)?;
            logical_id(&distribution.source_id)?;
            logical_id(&distribution.artifact_id)?;
            digest(&distribution.verification.evidence_digest)?;
            digest(&distribution.verification.verifier_digest)?;
            digest(&distribution.verification.subject_digest)?;
            ensure!(
                distribution.verification.subject_digest == distribution.digest,
                "verification subject differs from artifact"
            );
            if let Some(closure) = &distribution.package_closure_digest {
                digest(closure)?;
            }
            unique_sorted(&mut distribution.dependencies)?;
            for dependency in &distribution.dependencies {
                digest(dependency)?;
            }
            edges += distribution.dependencies.len();
            ensure!(edges <= 16_384, "too many tool dependency edges");
        }
        tool.distribution
            .sort_by(|a, b| a.platform.cmp(&b.platform));
    }
    lock.tool.sort_by(|a, b| a.key.cmp(&b.key));
    lock.environment
        .sort_by(|a, b| (&a.scope, &a.profile).cmp(&(&b.scope, &b.profile)));
    lock.selections = graph::validate(&lock)?;
    Ok(lock)
}

fn tool_key(tool: &Tool) -> Result<String> {
    crate::records::digest(
        "oyzu.tool-record.v2",
        &serde_json::json!({
            "id": tool.id, "version": tool.version,
            "backend_digest": tool.backend_digest, "options": tool.options
        }),
    )
}

pub(super) fn digest(value: &str) -> Result<()> {
    ensure!(
        value.len() == 71
            && value.starts_with("sha256:")
            && value[7..]
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid SHA-256 identity"
    );
    Ok(())
}

fn text(value: &str, field: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && !value.chars().any(char::is_control),
        "invalid {field}"
    );
    Ok(())
}

pub(super) fn canonical_id(value: &str) -> Result<()> {
    text(value, "canonical tool ID")?;
    let (backend, name) = value
        .split_once(':')
        .context("tool ID requires an explicit backend")?;
    ensure!(
        !backend.is_empty() && !name.is_empty() && !value.chars().any(char::is_whitespace),
        "invalid canonical tool ID"
    );
    Ok(())
}

fn logical_id(value: &str) -> Result<()> {
    ensure!(
        (1..=256).contains(&value.len()) && value.bytes().all(|b| (32..=126).contains(&b)),
        "invalid logical artifact/source ID"
    );
    ensure!(
        !value.contains("://")
            && !value.contains('/')
            && !value.contains('\\')
            && !value.contains(':')
            && value != "."
            && value != "..",
        "artifact/source ID cannot be a URL or filesystem path"
    );
    Ok(())
}

fn scope(value: &str) -> Result<()> {
    ensure!(
        value == "."
            || (!value.is_empty()
                && !value.contains(['\\', ':'])
                && value
                    .split('/')
                    .all(|p| !p.is_empty() && p != "." && p != "..")),
        "scope must be a normalized workspace-relative path"
    );
    text(value, "scope")
}

pub(super) fn platform(value: &str) -> Result<()> {
    let parts: Vec<_> = value.split('/').collect();
    ensure!(
        matches!(
            parts.as_slice(),
            ["linux", "amd64" | "arm64", "gnu" | "musl"]
                | ["darwin", "amd64" | "arm64", "native"]
                | ["windows", "amd64" | "arm64", "msvc"]
        ),
        "unsupported tool platform {value}"
    );
    Ok(())
}

fn unique_sorted(values: &mut [String]) -> Result<()> {
    values.sort();
    ensure!(
        !values.windows(2).any(|pair| pair[0] == pair[1]),
        "duplicate set entry"
    );
    Ok(())
}
