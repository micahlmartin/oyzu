//! Tool-grant verification and in-memory expiry; transport, trusted context/key
//! provisioning, revocation notifications and process authorization belong to
//! the agent/executor. No token or verified state is serializable here.
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::VerifyingKey;
use jwt_compact::{alg::Ed25519, AlgorithmExt, Renamed, UntrustedToken};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

const SAFE: i64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ToolGrantOperation {
    Resolve,
    Install,
    Activate,
    Exec,
    Build,
}

/// These values must come from authenticated agent state and the exact requested
/// selection, never from the token being verified or project-controlled JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolGrantContext {
    pub request_id: String,
    pub context_id: String,
    pub policy_revision: String,
    pub selection_digest: String,
    pub issuer: String,
    pub tenant: String,
    pub subject: String,
    pub revocation_epoch: i64,
    pub operation: ToolGrantOperation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    alg: String,
    kid: String,
    typ: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    protocol: String,
    request_id: String,
    context_id: String,
    decision_id: String,
    decision: String,
    reason_codes: Vec<String>,
    policy_revision: String,
    selection_digest: String,
    operations: Vec<ToolGrantOperation>,
    not_before: i64,
    not_after: i64,
    offline_allowed: bool,
    revocation_epoch: i64,
    issuer: String,
    audience: String,
    tenant: String,
    subject: String,
}

/// Signature-verified claims with bounded in-process validity. This proves only
/// the supplied token/context checks, not authentication of caller inputs, backend
/// admission or execution permission. The owning agent must invalidate on logout,
/// key/policy changes and uncertain suspend/resume; restart drops this object.
pub struct VerifiedToolGrant {
    payload: Payload,
    monotonic_high_water: Instant,
    deadline: Instant,
    online_deadline: Instant,
    wall_high_water: i64,
    invalidated: bool,
}

fn text(value: &str) -> bool {
    !value.is_empty() && value.chars().count() <= 256 && !value.chars().any(char::is_control)
}
fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
fn json(bytes: &[u8], limit: usize) -> Result<serde_json::Value> {
    crate::config::policy::strict_json_limit(bytes, limit)
        .map_err(|_| anyhow::anyhow!("TOOL_GRANT_INVALID: malformed or excessive JSON"))
}
impl Payload {
    fn matches(&self, context: &ToolGrantContext) -> bool {
        self.request_id == context.request_id
            && self.context_id == context.context_id
            && self.policy_revision == context.policy_revision
            && self.selection_digest == context.selection_digest
            && self.issuer == context.issuer
            && self.tenant == context.tenant
            && self.subject == context.subject
            && self.revocation_epoch == context.revocation_epoch
    }
}
impl VerifiedToolGrant {
    /// Receive an online-issued compact JWS using an independently trusted key
    /// map. Only EdDSA is accepted; remote keys and header extensions are denied.
    /// Offline reuse requires this live object, not deserialization from disk.
    pub fn verify(
        token: &str,
        keys: &BTreeMap<String, VerifyingKey>,
        context: &ToolGrantContext,
        wall_now: i64,
        monotonic_now: Instant,
    ) -> Result<Self> {
        ensure!(
            token.len() <= 128 * 1024 && !keys.is_empty() && keys.len() <= 256,
            "TOOL_GRANT_INVALID: envelope or key-set limit"
        );
        let parts: Vec<_> = token.split('.').collect();
        ensure!(parts.len() == 3, "TOOL_GRANT_INVALID: compact JWS required");
        let header: Header =
            serde_json::from_value(json(&URL_SAFE_NO_PAD.decode(parts[0])?, 8192)?)
                .map_err(|_| anyhow::anyhow!("TOOL_GRANT_INVALID: header schema"))?;
        ensure!(
            header.alg == "EdDSA"
                && text(&header.kid)
                && header
                    .typ
                    .as_deref()
                    .is_none_or(|t| t == "oyzu-tool-selection+jws"),
            "TOOL_GRANT_INVALID: protected header"
        );
        let key = keys
            .get(&header.kid)
            .context("TOOL_GRANT_INVALID: unpinned key")?;
        let untrusted =
            UntrustedToken::new(token).map_err(|_| anyhow::anyhow!("TOOL_GRANT_INVALID: JWS"))?;
        Renamed::new(Ed25519, "EdDSA")
            .validator::<serde_json::Value>(key)
            .validate(&untrusted)
            .map_err(|_| anyhow::anyhow!("TOOL_GRANT_INVALID: signature"))?;
        let payload: Payload =
            serde_json::from_value(json(&URL_SAFE_NO_PAD.decode(parts[1])?, 64 * 1024)?)
                .map_err(|_| anyhow::anyhow!("TOOL_GRANT_INVALID: payload schema"))?;
        ensure!(
            payload.protocol == "oyzu.tool-selection/1"
                && payload.audience == "oyzu.tool-selection"
                && payload.decision == "allow"
                && uuid(&payload.request_id),
            "TOOL_GRANT_INVALID: protocol or identity"
        );
        ensure!(
            [
                &payload.context_id,
                &payload.decision_id,
                &payload.policy_revision,
                &payload.issuer,
                &payload.tenant,
                &payload.subject
            ]
            .into_iter()
            .all(|v| text(v)),
            "TOOL_GRANT_INVALID: identity text"
        );
        super::lock::digest(&payload.selection_digest)?;
        ensure!(
            payload.reason_codes.len() <= 256
                && payload.reason_codes.iter().all(|v| text(v))
                && payload.reason_codes.iter().collect::<BTreeSet<_>>().len()
                    == payload.reason_codes.len(),
            "TOOL_GRANT_INVALID: reason codes"
        );
        ensure!(
            !payload.operations.is_empty()
                && payload.operations.len() <= 5
                && payload.operations.iter().collect::<BTreeSet<_>>().len()
                    == payload.operations.len(),
            "TOOL_GRANT_INVALID: operations"
        );
        ensure!(
            !payload.operations.contains(&ToolGrantOperation::Resolve)
                || payload.operations.len() == 1,
            "TOOL_GRANT_INVALID: resolve grant cannot authorize other operations"
        );
        ensure!(
            !payload.offline_allowed
                || payload.operations.iter().all(|op| matches!(
                    op,
                    ToolGrantOperation::Activate
                        | ToolGrantOperation::Exec
                        | ToolGrantOperation::Build
                )),
            "TOOL_GRANT_INVALID: offline acquisition scope"
        );
        ensure!(
            (0..=SAFE).contains(&payload.revocation_epoch)
                && (0..=SAFE).contains(&payload.not_before)
                && (0..=SAFE).contains(&payload.not_after)
                && payload.not_after > payload.not_before
                && wall_now >= payload.not_before
                && wall_now < payload.not_after
                && payload.not_after - payload.not_before
                    <= if payload.offline_allowed { 900 } else { 60 },
            "TOOL_GRANT_INVALID: validity"
        );
        ensure!(
            payload.matches(context),
            "TOOL_GRANT_INVALID: context binding"
        );
        let deadline = monotonic_now
            .checked_add(Duration::from_secs((payload.not_after - wall_now) as u64))
            .context("TOOL_GRANT_INVALID: monotonic deadline")?;
        let online_end = payload.not_after.min(payload.not_before + 60);
        ensure!(
            wall_now < online_end,
            "TOOL_GRANT_INVALID: online receipt window expired"
        );
        let online_deadline = monotonic_now
            .checked_add(Duration::from_secs((online_end - wall_now) as u64))
            .context("TOOL_GRANT_INVALID: online deadline")?;
        let mut grant = Self {
            payload,
            monotonic_high_water: monotonic_now,
            deadline,
            online_deadline,
            wall_high_water: wall_now,
            invalidated: false,
        };
        grant.check(context, wall_now, monotonic_now, false)?;
        Ok(grant)
    }

    pub fn invalidate(&mut self) {
        self.invalidated = true;
    }

    /// Recheck on every use. Binding changes, clock rollback beyond five seconds
    /// or monotonic reversal permanently invalidate this instance. A denied
    /// operation does not extend or replace either signed/monotonic deadline.
    pub fn check(
        &mut self,
        context: &ToolGrantContext,
        wall_now: i64,
        monotonic_now: Instant,
        offline: bool,
    ) -> Result<()> {
        if !self.payload.matches(context)
            || wall_now < self.wall_high_water.saturating_sub(5)
            || monotonic_now < self.monotonic_high_water
        {
            self.invalidated = true;
        }
        self.wall_high_water = self.wall_high_water.max(wall_now);
        self.monotonic_high_water = self.monotonic_high_water.max(monotonic_now);
        if wall_now >= self.payload.not_after || monotonic_now >= self.deadline {
            self.invalidated = true;
        }
        ensure!(!self.invalidated, "TOOL_GRANT_INVALIDATED");
        ensure!(
            wall_now >= self.payload.not_before
                && wall_now < self.payload.not_after
                && monotonic_now < self.deadline,
            "TOOL_GRANT_EXPIRED"
        );
        ensure!(
            self.payload.operations.contains(&context.operation),
            "TOOL_GRANT_OPERATION_DENIED"
        );
        ensure!(
            if offline {
                self.payload.offline_allowed
            } else {
                wall_now < self.payload.not_before + 60 && monotonic_now < self.online_deadline
            },
            "TOOL_GRANT_MODE_OR_DEADLINE_DENIED"
        );
        Ok(())
    }
}
