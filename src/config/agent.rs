//! Headless policy distribution, immutable cache entries and OS-backed high-water state.
use super::{
    locations::Locations,
    managed::{Bootstrap, ConfigurationContext, HighWater, Operation, VerifiedPolicySnapshot},
    policy::strict_json,
    registry::CAPABILITIES,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    water: HighWater,
    entry: String,
    denied: bool,
    #[serde(default)]
    failures: u32,
    #[serde(default)]
    retry_after: i64,
}
pub struct Agent {
    bootstrap: Bootstrap,
    context: ConfigurationContext,
    directory: PathBuf,
    state_id: String,
    runtime: Arc<dyn super::policy_runtime::Runtime>,
}
pub struct Acquired {
    pub snapshot: VerifiedPolicySnapshot,
    pub online: bool,
}
impl Agent {
    pub fn new(
        bootstrap: Bootstrap,
        locations: &Locations,
        workspace: &Path,
        ci: bool,
    ) -> Result<Self> {
        let context = ConfigurationContext {
            organization_id: bootstrap.organization_id.clone(),
            enrollment_id: bootstrap.enrollment_id.clone(),
            subject_id: None,
            workspace_id: Some(crate::records::digest(
                "oyzu.workspace.selector.v1",
                &json!(workspace.canonicalize()?),
            )?),
            os: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            execution_class: if ci { "ci" } else { "local" }.into(),
        };
        let state_id = context.digest()?;
        let directory = locations.state.join("policy").join(&state_id);
        Ok(Self {
            bootstrap,
            context,
            directory,
            state_id,
            runtime: Arc::new(super::policy_runtime::Native::new()?),
        })
    }
    fn state(&self) -> Result<Option<State>> {
        self.runtime
            .state(&self.state_id)?
            .map(|bytes| {
                serde_json::from_value(strict_json(&bytes)?)
                    .context("POLICY_INVALID: integrity state is corrupt")
            })
            .transpose()
    }
    fn save(&self, state: &State) -> Result<()> {
        self.runtime
            .save(&self.state_id, &serde_json::to_vec(state)?)
    }
    fn cached(&self, state: &State, time: i64) -> Result<VerifiedPolicySnapshot> {
        if state.denied {
            bail!("POLICY_UNAVAILABLE: server denied this configuration context; refresh required");
        }
        if state.entry.len() != 64 || !state.entry.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("POLICY_INVALID: invalid cache pointer");
        }
        let path = self.directory.join(format!("{}.jws", state.entry));
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("POLICY_INVALID: cache entry is a link");
        }
        let file = fs::File::open(path)?;
        if file.metadata()?.len() > 2 * 1024 * 1024 {
            bail!("CONFIG_LIMIT: oversized cached envelope");
        }
        let mut envelope = String::new();
        file.take(2 * 1024 * 1024 + 1)
            .read_to_string(&mut envelope)?;
        VerifiedPolicySnapshot::verify(
            &envelope,
            &self.bootstrap,
            &self.context,
            time,
            Some(&state.water),
        )
    }
    pub fn acquire(&self, force: bool) -> Result<Acquired> {
        secure_directory(&self.directory)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.directory.join("refresh.lock"))?;
        lock.lock()
            .context("POLICY_UNAVAILABLE: cannot serialize policy refresh")?;
        let time = self.runtime.now()?;
        let mut state = self.state()?;
        let cached = state.as_ref().and_then(|s| self.cached(s, time).ok());
        let ci = self.context.execution_class == "ci";
        if !force && !ci {
            if let Some(snapshot) = cached.as_ref().filter(|s| {
                time < s
                    .refresh_after()
                    .max(state.as_ref().map_or(0, |state| state.retry_after))
            }) {
                if snapshot
                    .authorize(Operation::LocalBuild, time, false, state.is_some())
                    .is_ok()
                {
                    if let Some(state) = state.as_mut() {
                        state.water.observed_at = time;
                        self.save(state)?;
                    }
                    return Ok(Acquired {
                        snapshot: snapshot.clone(),
                        online: false,
                    });
                }
            }
        }
        let endpoint = format!(
            "{}/v1/build-contexts:resolve",
            self.bootstrap.platform_url.trim_end_matches('/')
        );
        let request = json!({"purpose":"configuration","schemaVersions":[1],"capabilities":CAPABILITIES,"organizationId":self.bootstrap.organization_id,"enrollmentId":self.bootstrap.enrollment_id,"configurationContext":self.context,"lastKnownRevision":cached.as_ref().map(|s|s.revision())});
        let envelope = match self.runtime.refresh(&endpoint, &request)? {
            super::policy_runtime::Refresh::Snapshot(envelope) => envelope,
            super::policy_runtime::Refresh::TransportFailure => {
                return self.offline(force, ci, cached, state, time)
            }
            super::policy_runtime::Refresh::Denied => {
                if let Some(mut state) = state {
                    state.denied = true;
                    self.save(&state)?;
                }
                bail!(
                    "POLICY_UNAVAILABLE: server denied configuration refresh; no offline downgrade"
                );
            }
        };
        // A successful online context reconciliation may repair a backward clock
        // observation, but never weakens the sequence or payload high-water check.
        let online_water = state.as_ref().map(|state| {
            let mut water = state.water.clone();
            water.observed_at = water.observed_at.min(time);
            water
        });
        let snapshot = VerifiedPolicySnapshot::verify(
            &envelope,
            &self.bootstrap,
            &self.context,
            time,
            online_water.as_ref(),
        )?;
        use sha2::{Digest, Sha256};
        let entry = format!("{:x}", Sha256::digest(envelope.as_bytes()));
        let mut file = tempfile::NamedTempFile::new_in(&self.directory)?;
        file.write_all(envelope.as_bytes())?;
        file.as_file().sync_all()?;
        let path = self.directory.join(format!("{entry}.jws"));
        // Verified content determines the name. Atomic replacement also repairs a
        // corrupt entry after online reconciliation without trusting its old bytes.
        file.persist(&path).map_err(|e| e.error)?;
        #[cfg(unix)]
        fs::File::open(&self.directory)?.sync_all()?;
        // The OS-protected pointer is the commit point, after durable immutable bytes.
        self.save(&State {
            water: snapshot.high_water(&self.bootstrap, time)?,
            entry,
            denied: false,
            failures: 0,
            retry_after: 0,
        })?;
        snapshot.authorize(
            if ci {
                Operation::CiBuild
            } else {
                Operation::LocalBuild
            },
            time,
            true,
            true,
        )?;
        Ok(Acquired {
            snapshot,
            online: true,
        })
    }
    fn offline(
        &self,
        force: bool,
        ci: bool,
        cached: Option<VerifiedPolicySnapshot>,
        state: Option<State>,
        time: i64,
    ) -> Result<Acquired> {
        if force {
            bail!("POLICY_UNAVAILABLE: refresh transport failed; existing cache retained");
        }
        let snapshot =
            cached.context("POLICY_UNAVAILABLE: no applicable verified cached snapshot")?;
        snapshot.authorize(
            if ci {
                Operation::CiBuild
            } else {
                Operation::LocalBuild
            },
            time,
            false,
            state.is_some(),
        )?;
        if let Some(mut state) = state {
            state.water.observed_at = time;
            state.failures = state.failures.saturating_add(1);
            let exponential = 5i64
                .saturating_mul(1i64 << state.failures.saturating_sub(1).min(6))
                .min(300);
            let jitter = (time.rem_euclid(21) - 10) * exponential / 100;
            state.retry_after = time
                .saturating_add((exponential + jitter).clamp(5, 300))
                .min(snapshot.deadline());
            self.save(&state)?;
        }
        Ok(Acquired {
            snapshot,
            online: false,
        })
    }
    pub fn status(&self) -> Result<serde_json::Value> {
        let state = self.state()?;
        match state {
            Some(state) => {
                let time = self.runtime.now()?;
                let snapshot = self.cached(&state, time);
                match snapshot {
                    Ok(s) => Ok(
                        json!({"mode":"managed","revision":s.revision(),"offlineDeadline":s.deadline(),"refreshAfter":s.refresh_after(),"status":"verified","offlineAvailable":self.context.execution_class == "local" && s.authorize(Operation::LocalBuild,time,false,true).is_ok()}),
                    ),
                    Err(_) => Ok(json!({"mode":"managed","status":"refresh-required"})),
                }
            }
            None => Ok(json!({"mode":"managed","status":"online-reconciliation-required"})),
        }
    }
}
fn secure_directory(path: &Path) -> Result<()> {
    for parent in path.ancestors() {
        if let Ok(m) = fs::symlink_metadata(parent) {
            if m.file_type().is_symlink() {
                bail!("POLICY_INVALID: cache directory redirects through a link");
            }
        }
    }
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::policy_runtime::{Refresh, Runtime};
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use ed25519_dalek::{Signer, SigningKey};
    use std::sync::Mutex;
    struct Fake {
        time: Mutex<i64>,
        state: Mutex<Option<Vec<u8>>>,
        reply: Mutex<Option<String>>,
        denied: Mutex<bool>,
        fail_store: Mutex<bool>,
    }
    impl Runtime for Fake {
        fn now(&self) -> Result<i64> {
            Ok(*self.time.lock().unwrap())
        }
        fn state(&self, _: &str) -> Result<Option<Vec<u8>>> {
            Ok(self.state.lock().unwrap().clone())
        }
        fn save(&self, _: &str, value: &[u8]) -> Result<()> {
            if *self.fail_store.lock().unwrap() {
                bail!("injected integrity store failure");
            }
            *self.state.lock().unwrap() = Some(value.into());
            Ok(())
        }
        fn refresh(&self, _: &str, _: &serde_json::Value) -> Result<Refresh> {
            if *self.denied.lock().unwrap() {
                Ok(Refresh::Denied)
            } else {
                Ok(self
                    .reply
                    .lock()
                    .unwrap()
                    .clone()
                    .map_or(Refresh::TransportFailure, Refresh::Snapshot))
            }
        }
    }
    fn setup(root: &Path) -> (Agent, Arc<Fake>, SigningKey) {
        // macOS exposes its temporary root through /var -> /private/var.
        // Test the physical fixture, retaining the production no-link rule.
        let physical_root = root.canonicalize().unwrap();
        let root = physical_root.as_path();
        let key = SigningKey::from_bytes(&[19; 32]);
        let bootstrap=Bootstrap::parse(&serde_json::to_vec(&json!({"schemaVersion":1,"kind":"management-bootstrap","organizationId":"test","enrollmentId":"test","platformUrl":"https://example.test","policyKeys":[{"kid":"test","kty":"OKP","crv":"Ed25519","x":URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes())}]})).unwrap()).unwrap();
        let locations = Locations {
            machine: root.join("machine"),
            user: root.join("user/config.toml"),
            state: root.join("state"),
            diagnostics: vec![],
        };
        let time = chrono::DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")
            .unwrap()
            .timestamp();
        let runtime = Arc::new(Fake {
            time: Mutex::new(time),
            state: Mutex::new(None),
            reply: Mutex::new(None),
            denied: Mutex::new(false),
            fail_store: Mutex::new(false),
        });
        let mut agent = Agent::new(bootstrap, &locations, root, false).unwrap();
        agent.runtime = runtime.clone();
        *runtime.reply.lock().unwrap() = Some(envelope(&agent, &key, 1));
        (agent, runtime, key)
    }
    fn envelope(agent: &Agent, key: &SigningKey, sequence: u64) -> String {
        let payload = json!({"schemaVersion":1,"kind":"managed-policy","snapshotId":"test","revision":format!("r{sequence}"),"organizationId":"test","enrollmentId":"test","audience":"oyzu-config","contextDigest":agent.context.digest().unwrap(),"sequence":sequence,"issuedAt":"2026-10-01T00:00:00Z","refreshAfter":"2026-10-01T00:15:00Z","expiresAt":"2026-10-03T00:00:00Z","offline":{"localBuilds":true,"maxAgeSeconds":86400},"requiredCapabilities":[],"settings":{},"profiles":{}});
        let input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(br#"{"alg":"Ed25519","kid":"test","typ":"oyzu-policy+jws"}"#),
            URL_SAFE_NO_PAD.encode(serde_json_canonicalizer::to_vec(&payload).unwrap())
        );
        format!(
            "{input}.{}",
            URL_SAFE_NO_PAD.encode(key.sign(input.as_bytes()).to_bytes())
        )
    }
    #[test]
    fn refresh_persists_then_offline_obeys_deadline_and_missing_state() {
        let temp = tempfile::tempdir().unwrap();
        let (agent, runtime, _) = setup(temp.path());
        assert!(agent.acquire(true).unwrap().online);
        *runtime.reply.lock().unwrap() = None;
        *runtime.time.lock().unwrap() += 86399;
        assert!(!agent.acquire(false).unwrap().online);
        *runtime.time.lock().unwrap() += 1;
        assert!(agent.acquire(false).is_err());
        *runtime.time.lock().unwrap() -= 2;
        *runtime.state.lock().unwrap() = None;
        assert!(agent.acquire(false).is_err());
    }
    #[test]
    fn explicit_denial_cannot_become_offline_success() {
        let temp = tempfile::tempdir().unwrap();
        let (agent, runtime, _) = setup(temp.path());
        agent.acquire(true).unwrap();
        *runtime.denied.lock().unwrap() = true;
        assert!(agent.acquire(true).is_err());
        *runtime.denied.lock().unwrap() = false;
        *runtime.reply.lock().unwrap() = None;
        assert!(agent.acquire(false).is_err());
    }
    #[test]
    fn failed_commit_does_not_activate_new_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let (agent, runtime, key) = setup(temp.path());
        agent.acquire(true).unwrap();
        let old = runtime.state.lock().unwrap().clone();
        *runtime.reply.lock().unwrap() = Some(envelope(&agent, &key, 2));
        *runtime.fail_store.lock().unwrap() = true;
        assert!(agent.acquire(true).is_err());
        assert_eq!(*runtime.state.lock().unwrap(), old);
        *runtime.fail_store.lock().unwrap() = false;
        *runtime.reply.lock().unwrap() = None;
        assert_eq!(agent.acquire(false).unwrap().snapshot.revision(), "r1");
    }
    #[test]
    fn truncated_cache_never_becomes_unsigned_policy() {
        let temp = tempfile::tempdir().unwrap();
        let (agent, runtime, _) = setup(temp.path());
        agent.acquire(true).unwrap();
        let state = agent.state().unwrap().unwrap();
        fs::write(
            agent.directory.join(format!("{}.jws", state.entry)),
            "truncated",
        )
        .unwrap();
        let valid = runtime.reply.lock().unwrap().take();
        assert!(agent.acquire(false).is_err());
        *runtime.reply.lock().unwrap() = valid;
        assert!(agent.acquire(true).unwrap().online);
        *runtime.reply.lock().unwrap() = None;
        assert!(!agent.acquire(false).unwrap().online);
    }
}
