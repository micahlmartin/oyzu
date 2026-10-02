//! Verified corporate snapshots. Construction is restricted to signature verification.
use super::{
    policy::{strict_json, Policy},
    sources::capabilities,
};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jwt_compact::{alg::Ed25519, AlgorithmExt, Renamed, UntrustedToken};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bootstrap {
    pub schema_version: u64,
    pub kind: String,
    pub organization_id: String,
    pub enrollment_id: String,
    pub platform_url: String,
    pub policy_keys: Vec<PublicKey>,
    pub extensions: Option<BTreeMap<String, Value>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicKey {
    pub kid: String,
    pub kty: String,
    pub crv: String,
    pub x: String,
}
fn identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().count() <= 256 && !s.chars().any(char::is_control)
}
impl Bootstrap {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let result: Self = serde_json::from_value(strict_json(bytes)?)
            .map_err(|_| anyhow::anyhow!("POLICY_INVALID: invalid enrollment schema"))?;
        if result.schema_version != 1
            || result.kind != "management-bootstrap"
            || !identifier(&result.organization_id)
            || !identifier(&result.enrollment_id)
            || result.policy_keys.is_empty()
        {
            bail!("POLICY_INVALID: invalid enrollment");
        }
        let url = reqwest::Url::parse(&result.platform_url)
            .context("POLICY_INVALID: invalid platform URL")?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("POLICY_INVALID: platform must be an HTTPS endpoint without credentials");
        }
        let mut ids = BTreeSet::new();
        for key in &result.policy_keys {
            if !identifier(&key.kid)
                || !ids.insert(&key.kid)
                || key.kty != "OKP"
                || key.crv != "Ed25519"
                || URL_SAFE_NO_PAD
                    .decode(&key.x)
                    .ok()
                    .is_none_or(|b| b.len() != 32)
            {
                bail!("POLICY_INVALID: invalid pinned policy key");
            }
        }
        Ok(result)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationContext {
    pub organization_id: String,
    pub enrollment_id: String,
    pub subject_id: Option<String>,
    pub workspace_id: Option<String>,
    pub os: String,
    pub architecture: String,
    pub execution_class: String,
}
impl ConfigurationContext {
    pub fn digest(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json_canonicalizer::to_vec(self)?)
        ))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Offline {
    local_builds: bool,
    max_age_seconds: u64,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Payload {
    schema_version: u64,
    kind: String,
    snapshot_id: String,
    revision: String,
    organization_id: String,
    enrollment_id: String,
    audience: String,
    context_digest: String,
    sequence: u64,
    issued_at: String,
    refresh_after: String,
    expires_at: String,
    offline: Offline,
    required_capabilities: Vec<String>,
    settings: BTreeMap<String, super::constraints::Entry>,
    profiles: BTreeMap<String, Value>,
    default_profile: Option<String>,
    extensions: Option<BTreeMap<String, Value>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HighWater {
    pub sequence: u64,
    pub payload_digest: String,
    pub observed_at: i64,
    pub bootstrap_digest: String,
}
#[derive(Clone, Debug)]
pub struct VerifiedPolicySnapshot {
    payload: Payload,
    digest: String,
    issued: i64,
    refresh: i64,
    expires: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    LocalBuild,
    CiBuild,
    Publish,
    Sign,
    CredentialGrant,
}
fn timestamp(value: &str) -> Result<i64> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!("POLICY_INVALID: invalid timestamp"))?;
    if !value.ends_with('Z') || value.contains('.') || parsed.timestamp_subsec_nanos() != 0 {
        bail!("POLICY_INVALID: timestamps must be whole UTC seconds");
    }
    Ok(parsed.timestamp())
}
impl VerifiedPolicySnapshot {
    pub fn verify(
        envelope: &str,
        bootstrap: &Bootstrap,
        context: &ConfigurationContext,
        now: i64,
        high_water: Option<&HighWater>,
    ) -> Result<Self> {
        if envelope.len() > 2 * 1024 * 1024 {
            bail!("CONFIG_LIMIT: policy envelope exceeds 2 MiB");
        }
        let parts: Vec<_> = envelope.split('.').collect();
        if parts.len() != 3 {
            bail!("POLICY_INVALID: compact JWS required");
        }
        let header = strict_json(
            &URL_SAFE_NO_PAD
                .decode(parts[0])
                .context("POLICY_INVALID: header encoding")?,
        )?;
        let object = header
            .as_object()
            .context("POLICY_INVALID: invalid header")?;
        if object
            .keys()
            .any(|k| !matches!(k.as_str(), "alg" | "kid" | "typ"))
            || header["alg"] != "Ed25519"
            || header["typ"] != "oyzu-policy+jws"
        {
            bail!("POLICY_INVALID: unsupported protected JWS header");
        }
        let key = bootstrap
            .policy_keys
            .iter()
            .find(|k| Some(k.kid.as_str()) == header["kid"].as_str())
            .context("POLICY_INVALID: unpinned signing key")?;
        let bytes: [u8; 32] = URL_SAFE_NO_PAD
            .decode(&key.x)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("POLICY_INVALID: key length"))?;
        let key = ed25519_dalek::VerifyingKey::from_bytes(&bytes)
            .context("POLICY_INVALID: invalid verification key")?;
        let token = UntrustedToken::new(envelope)
            .map_err(|_| anyhow::anyhow!("POLICY_INVALID: malformed JWS"))?;
        Renamed::new(Ed25519, "Ed25519")
            .validator::<Value>(&key)
            .validate(&token)
            .map_err(|_| anyhow::anyhow!("POLICY_INVALID: signature verification failed"))?;
        let bytes = URL_SAFE_NO_PAD.decode(parts[1])?;
        let value = strict_json(&bytes)?;
        if serde_json_canonicalizer::to_vec(&value)? != bytes {
            bail!("POLICY_INVALID: payload is not canonical JSON");
        }
        super::policy::validate_entry_shapes(&value)?;
        let payload: Payload = serde_json::from_value(value)
            .map_err(|_| anyhow::anyhow!("POLICY_INVALID: invalid snapshot schema"))?;
        if payload.schema_version != 1
            || payload.kind != "managed-policy"
            || payload.audience != "oyzu-config"
            || payload.organization_id != bootstrap.organization_id
            || payload.enrollment_id != bootstrap.enrollment_id
            || payload.context_digest != context.digest()?
            || context.organization_id != bootstrap.organization_id
            || context.enrollment_id != bootstrap.enrollment_id
            || !identifier(&payload.snapshot_id)
            || !identifier(&payload.revision)
            || payload.sequence == 0
            || payload.sequence > 9_007_199_254_740_991
        {
            bail!("POLICY_INVALID: snapshot identity, scope or sequence mismatch");
        }
        capabilities(&payload.required_capabilities)?;
        if payload
            .required_capabilities
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != payload.required_capabilities.len()
        {
            bail!("POLICY_INVALID: repeated capability");
        }
        let issued = timestamp(&payload.issued_at)?;
        let refresh = timestamp(&payload.refresh_after)?;
        let expires = timestamp(&payload.expires_at)?;
        if issued > refresh
            || refresh >= expires
            || issued > now.saturating_add(120)
            || payload.offline.max_age_seconds > 86400
            || payload.offline.local_builds != (payload.offline.max_age_seconds > 0)
        {
            bail!("POLICY_INVALID: invalid timing or offline permission");
        }
        if now >= expires {
            bail!("POLICY_EXPIRED: snapshot has expired");
        }
        let digest = crate::records::digest("oyzu.policy.payload.v1", &strict_json(&bytes)?)?;
        if let Some(high) = high_water {
            let binding =
                crate::records::digest("oyzu.management.v1", &serde_json::to_value(bootstrap)?)?;
            if high.bootstrap_digest != binding {
                bail!("POLICY_INVALID: bootstrap changed; reconcile online");
            }
            if now < high.observed_at {
                bail!("POLICY_CLOCK_UNCERTAIN: wall clock moved backward");
            }
            if payload.sequence < high.sequence
                || payload.sequence == high.sequence && digest != high.payload_digest
            {
                bail!("POLICY_INVALID: rollback or changed equal-sequence payload");
            }
        }
        let result = Self {
            payload,
            digest,
            issued,
            refresh,
            expires,
        };
        // Verify all enforcement semantics before exposing this type.
        result.policy().source(
            "corporate",
            std::path::Path::new("."),
            super::registry::Scope::Corporate,
            &super::registry::Registry::default(),
            &mut super::constraints::Constraints::default(),
        )?;
        Ok(result)
    }
    pub fn policy(&self) -> Policy {
        Policy {
            schema_version: 1,
            kind: "local-admin-policy".into(),
            settings: self.payload.settings.clone(),
            profiles: self.payload.profiles.clone(),
            required_capabilities: self.payload.required_capabilities.clone(),
            default_profile: self.payload.default_profile.clone(),
            extensions: self.payload.extensions.clone(),
        }
    }
    pub fn authorize(
        &self,
        operation: Operation,
        now: i64,
        online_context: bool,
        integrity_state: bool,
    ) -> Result<()> {
        if now >= self.expires {
            bail!("POLICY_EXPIRED: snapshot has expired");
        }
        if matches!(
            operation,
            Operation::Publish | Operation::Sign | Operation::CredentialGrant
        ) {
            bail!("POLICY_UNAVAILABLE: fresh operation-bound authorization is required");
        }
        if online_context {
            return Ok(());
        }
        if !integrity_state || now < self.issued {
            bail!("POLICY_CLOCK_UNCERTAIN: online reconciliation required");
        }
        if operation != Operation::LocalBuild
            || !self.payload.offline.local_builds
            || now >= self.deadline()
        {
            bail!("POLICY_EXPIRED: operation is not covered by offline policy");
        }
        Ok(())
    }
    pub fn deadline(&self) -> i64 {
        self.expires
            .min(self.issued + self.payload.offline.max_age_seconds as i64)
    }
    pub fn refresh_after(&self) -> i64 {
        self.refresh
    }
    pub fn revision(&self) -> &str {
        &self.payload.revision
    }
    pub fn high_water(&self, bootstrap: &Bootstrap, now: i64) -> Result<HighWater> {
        Ok(HighWater {
            sequence: self.payload.sequence,
            payload_digest: self.digest.clone(),
            observed_at: now,
            bootstrap_digest: crate::records::digest(
                "oyzu.management.v1",
                &serde_json::to_value(bootstrap)?,
            )?,
        })
    }
}
