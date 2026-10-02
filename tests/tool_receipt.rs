use serde_json::{json, Value};
use std::{fs, path::PathBuf};

struct Fixture {
    _temp: tempfile::TempDir,
    lock: PathBuf,
    store: PathBuf,
    install: PathBuf,
    receipt: Value,
    platform: &'static str,
    installer: String,
}
impl Fixture {
    fn stage(&self) -> PathBuf {
        let staging = self._temp.path().join("staging");
        fs::create_dir(&staging).unwrap();
        fs::rename(self.store.join("installs"), staging.join("installs")).unwrap();
        staging
    }
    fn lease(
        &self,
        staging: Option<&std::path::Path>,
    ) -> anyhow::Result<oyzu::tools::InstallationLease> {
        oyzu::tools::lease_installation_selection(
            &self.lock,
            &self.store,
            staging,
            ".",
            "default",
            self.platform,
            &self.installer,
        )
    }
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let store = temp.path().join("store");
        let lock = temp.path().join("oyzu.lock");
        let platform = if cfg!(windows) {
            "windows/amd64/msvc"
        } else {
            "linux/amd64/gnu"
        };
        let source =
            include_str!("fixtures/tool-lock/valid.toml").replace("linux/amd64/gnu", platform);
        fs::write(&lock, &source).unwrap();
        let inspection = oyzu::tools::inspect_lock(&lock).unwrap();
        let (tool_key, key) = inspection.selections[0]
            .installation_keys
            .iter()
            .next()
            .unwrap();
        let install = store
            .join("installs")
            .join(key.strip_prefix("sha256:").unwrap());
        let payload = install.join("payload");
        fs::create_dir_all(payload.join("bin")).unwrap();
        fs::write(payload.join("bin/node"), b"not executed by this fixture").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(payload.join("bin/node"), fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let tree = oyzu::tools::inspect_tree(&payload).unwrap();
        let document: toml::Value = toml::from_str(&source).unwrap();
        let tool = &document["tool"][0];
        let distribution = &tool["distribution"][0];
        let installer = format!("sha256:{}", "1".repeat(64));
        // The same closed shape is validated by the JSON Schema checker. Bind
        // its synthetic placeholders to this host's actual locked payload.
        let mut receipt: Value =
            serde_json::from_str(include_str!("fixtures/tool-receipt/receipt.json")).unwrap();
        let bindings = json!({"installation_key":key,"tool_key":tool_key,"tool_id":tool["id"],"version":tool["version"],"platform":platform,
            "backend_digest":tool["backend_digest"],"distribution_digest":distribution["digest"],"distribution_size":distribution["size"],"layout_digest":distribution["layout_digest"],
            "verification":distribution["verification"],"tree_digest":tree.digest,"tree_manifest_digest":tree.manifest_digest,
            "installer_release_digest":installer});
        receipt
            .as_object_mut()
            .unwrap()
            .extend(bindings.as_object().unwrap().clone());
        receipt["environment"]["PATH"]["paths"][0]["installation_key"] = json!(key);
        let fixture = Self {
            _temp: temp,
            lock,
            store,
            install,
            receipt,
            platform,
            installer,
        };
        fixture.save(&fixture.receipt);
        fixture
    }
    fn save(&self, value: &Value) {
        fs::write(
            self.install.join("receipt.json"),
            serde_json::to_vec(value).unwrap(),
        )
        .unwrap();
    }
    fn verify(&self) -> anyhow::Result<String> {
        oyzu::tools::verify_installation_selection(
            &self.lock,
            &self.store,
            ".",
            "default",
            self.platform,
            &self.installer,
        )
    }
}

#[test]
fn receipt_runtime_rejects_shared_schema_invalid_shape_corpus() {
    let fixture = Fixture::new();
    assert!(fixture.verify().is_ok());
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/tool-receipt/invalid-shapes.json")).unwrap();
    for case in cases {
        let mut receipt = fixture.receipt.clone();
        let pointer = case["pointer"].as_str().unwrap();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        let target = receipt
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap();
        if case["remove"].as_bool() == Some(true) {
            target.remove(key).unwrap();
        } else {
            target.insert(key.to_owned(), case["value"].clone());
        }
        fixture.save(&receipt);
        assert!(fixture.verify().is_err(), "{}", case["name"]);
    }
    fixture.save(&fixture.receipt);
    assert!(fixture.verify().is_ok());
}

#[test]
fn checks_complete_receipt_identity_and_fresh_payload_bytes() {
    let fixture = Fixture::new();
    assert!(fixture.verify().is_ok());
    for field in [
        "installation_key",
        "tool_key",
        "tool_id",
        "version",
        "platform",
        "backend_digest",
        "distribution_digest",
        "layout_digest",
        "tree_digest",
        "tree_manifest_digest",
        "installer_release_digest",
    ] {
        let mut changed = fixture.receipt.clone();
        changed[field] = json!("changed");
        fixture.save(&changed);
        assert!(fixture.verify().is_err(), "{field}");
    }
    for (field, value) in [
        ("format", json!(2)),
        ("distribution_size", json!(33)),
        ("dependency_installation_keys", json!(["unexpected"])),
        ("package_closure_digest", json!("changed")),
    ] {
        let mut changed = fixture.receipt.clone();
        changed[field] = value;
        fixture.save(&changed);
        assert!(fixture.verify().is_err());
    }
    fixture.save(&fixture.receipt);
    fs::write(fixture.install.join("payload/bin/node"), b"tampered").unwrap();
    assert!(fixture.verify().is_err());
}

#[test]
fn rejects_open_records_duplicate_keys_and_invalid_launch_references() {
    let fixture = Fixture::new();
    let mut changed = fixture.receipt.clone();
    changed["unknown"] = json!(true);
    fixture.save(&changed);
    assert!(fixture.verify().is_err());
    let mut changed = fixture.receipt.clone();
    changed
        .as_object_mut()
        .unwrap()
        .remove("package_closure_digest");
    fixture.save(&changed);
    assert!(fixture.verify().is_err());
    let duplicate =
        serde_json::to_string(&fixture.receipt)
            .unwrap()
            .replacen('{', "{\"format\":1,", 1);
    fs::write(fixture.install.join("receipt.json"), duplicate).unwrap();
    assert!(fixture.verify().is_err());
    for path in ["../outside", "bin/missing", "bin", "/absolute"] {
        let mut changed = fixture.receipt.clone();
        changed["entrypoints"]["node"]["payload_relative_path"] = json!(path);
        fixture.save(&changed);
        assert!(fixture.verify().is_err());
    }
    let mut changed = fixture.receipt.clone();
    changed["environment"]["PATH"]["paths"][0]["installation_key"] =
        json!(format!("sha256:{}", "2".repeat(64)));
    fixture.save(&changed);
    assert!(fixture.verify().is_err());
    let mut changed = fixture.receipt.clone();
    changed["environment"]["PATH"] = json!({"kind":"literal","value":"/ambient"});
    fixture.save(&changed);
    assert!(fixture.verify().is_err());
}

#[test]
fn unavailable_selection_and_missing_receipt_never_adopt_payload() {
    let fixture = Fixture::new();
    assert!(oyzu::tools::verify_installation_selection(
        &fixture.lock,
        &fixture.store,
        "other",
        "default",
        fixture.platform,
        &fixture.installer
    )
    .is_err());
    fs::remove_file(fixture.install.join("receipt.json")).unwrap();
    assert!(fixture.verify().is_err());
}

#[test]
fn changed_archive_identity_never_reuses_same_version_directory() {
    let fixture = Fixture::new();
    assert!(fixture.verify().is_ok());
    let original = fs::read_to_string(&fixture.lock).unwrap();
    // Both digest and its verification subject change; version/tool key remain
    // unchanged and the lock is still internally valid.
    let changed = original.replace(&"b".repeat(64), &"9".repeat(64));
    fs::write(&fixture.lock, changed).unwrap();
    assert!(oyzu::tools::inspect_lock(&fixture.lock).is_ok());
    assert!(fixture.verify().is_err());
}

#[test]
fn typed_interpreters_and_arguments_bind_to_declared_payloads() {
    let fixture = Fixture::new();
    let key = fixture.receipt["installation_key"].clone();
    let mut changed = fixture.receipt.clone();
    changed["entrypoints"]["script"] = json!({"kind":"interpreter", "payload_relative_path":"bin/node",
        "interpreter":{"installation_key":key,"relative_path":"bin/node"},
        "prefix_args":[{"kind":"literal","value":"--test"},{"kind":"path","path":{"installation_key":key,"relative_path":"bin"}}]});
    fixture.save(&changed);
    assert!(fixture.verify().is_ok());
    changed["entrypoints"]["script"]["interpreter"]["relative_path"] = json!("bin/absent");
    fixture.save(&changed);
    assert!(fixture.verify().is_err());
}

#[test]
fn dependency_payload_is_rehashed_even_when_parent_is_unchanged() {
    let mut fixture = Fixture::new();
    let mut lock: toml::Value =
        toml::from_str(&fs::read_to_string(&fixture.lock).unwrap()).unwrap();
    let mut dependency = lock["tool"][0].clone();
    dependency["id"] = toml::Value::String("core:go".into());
    let dependency_key = oyzu::records::digest("oyzu.tool-record.v2", &json!({
        "id":"core:go", "version":dependency["version"], "backend_digest":dependency["backend_digest"], "options":dependency["options"]
    })).unwrap();
    dependency["key"] = toml::Value::String(dependency_key.clone());
    lock["tool"][0]["distribution"][0]["dependencies"] =
        toml::Value::Array(vec![toml::Value::String(dependency_key.clone())]);
    lock["tool"].as_array_mut().unwrap().push(dependency);
    fs::write(&fixture.lock, toml::to_string(&lock).unwrap()).unwrap();
    let selection = oyzu::tools::inspect_lock(&fixture.lock)
        .unwrap()
        .selections
        .remove(0);
    let parent_key = fixture.receipt["tool_key"].as_str().unwrap();
    let parent_installation = &selection.installation_keys[parent_key];
    let dependency_installation = &selection.installation_keys[&dependency_key];
    let directory = fixture
        .store
        .join("installs")
        .join(parent_installation.strip_prefix("sha256:").unwrap());
    fs::rename(&fixture.install, &directory).unwrap();
    fixture.install = directory;
    fixture.receipt["installation_key"] = json!(parent_installation);
    fixture.receipt["dependency_installation_keys"] = json!([dependency_installation]);
    fixture.receipt["environment"] = json!({});
    fixture.save(&fixture.receipt);
    let child = fixture
        .store
        .join("installs")
        .join(dependency_installation.strip_prefix("sha256:").unwrap());
    fs::create_dir_all(child.join("payload/bin")).unwrap();
    fs::copy(
        fixture.install.join("payload/bin/node"),
        child.join("payload/bin/node"),
    )
    .unwrap();
    let mut receipt = fixture.receipt.clone();
    receipt["installation_key"] = json!(dependency_installation);
    receipt["tool_key"] = json!(dependency_key);
    receipt["tool_id"] = json!("core:go");
    receipt["dependency_installation_keys"] = json!([]);
    receipt["entrypoints"] = json!({});
    fs::write(
        child.join("receipt.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    assert!(fixture.verify().is_ok());
    fs::write(child.join("payload/bin/node"), b"changed dependency only").unwrap();
    assert!(fixture.verify().is_err());
}

#[test]
fn publishes_whole_directory_then_retains_os_lease_without_changing_lock() {
    let fixture = Fixture::new();
    let original = fs::read(&fixture.lock).unwrap();
    let staging = fixture.stage();
    assert!(fixture.lease(None).is_err());
    let lease = fixture.lease(Some(&staging)).unwrap();
    assert_eq!(lease.selection_digest, fixture.verify().unwrap());
    assert_eq!(fs::read(&fixture.lock).unwrap(), original);
    let key = fixture.receipt["installation_key"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    assert!(!staging.join("installs").join(key).exists());
    let guard = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.store.join("locks").join(format!("{key}.lease")))
        .unwrap();
    assert!(matches!(
        guard.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    let second = fixture.lease(None).unwrap();
    drop(lease);
    assert!(matches!(
        guard.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    drop(second);
    guard.try_lock().unwrap();
}

#[test]
fn invalid_staging_never_publishes_and_corrupt_committed_content_is_not_replaced() {
    let fixture = Fixture::new();
    let staging = fixture.stage();
    let key = fixture.receipt["installation_key"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    let candidate = staging.join("installs").join(key);
    fs::write(candidate.join("payload/bin/node"), b"bad staging").unwrap();
    assert!(fixture.lease(Some(&staging)).is_err());
    assert!(!fixture.install.exists());
    assert!(candidate.exists());
    fs::write(
        candidate.join("payload/bin/node"),
        b"not executed by this fixture",
    )
    .unwrap();
    drop(fixture.lease(Some(&staging)).unwrap());
    fs::write(fixture.install.join("payload/bin/node"), b"bad committed").unwrap();
    assert!(fixture.lease(Some(&staging)).is_err());
    assert_eq!(
        fs::read(fixture.install.join("payload/bin/node")).unwrap(),
        b"bad committed"
    );
}

#[test]
#[ignore = "child fixture invoked by cross-process publication test"]
fn publication_child() {
    let store = PathBuf::from(std::env::var_os("OYZU_TEST_STORE").unwrap());
    let lock = PathBuf::from(std::env::var_os("OYZU_TEST_LOCK").unwrap());
    let staging = PathBuf::from(std::env::var_os("OYZU_TEST_STAGING").unwrap());
    let platform = std::env::var("OYZU_TEST_PLATFORM").unwrap();
    let installer = std::env::var("OYZU_TEST_INSTALLER").unwrap();
    let _lease = oyzu::tools::lease_installation_selection(
        &lock,
        &store,
        Some(&staging),
        ".",
        "default",
        &platform,
        &installer,
    )
    .unwrap();
    if let Some(ready) = std::env::var_os("OYZU_TEST_READY") {
        fs::write(ready, b"lease held").unwrap();
        loop {
            std::thread::park();
        }
    }
}

#[test]
fn separate_publishers_reuse_one_commit_and_process_exit_releases_leases() {
    use std::process::{Command, Stdio};
    let fixture = Fixture::new();
    let staging = fixture.stage();
    let spawn = || {
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "publication_child", "--nocapture"])
            .env("OYZU_TEST_STORE", &fixture.store)
            .env("OYZU_TEST_LOCK", &fixture.lock)
            .env("OYZU_TEST_STAGING", &staging)
            .env("OYZU_TEST_PLATFORM", fixture.platform)
            .env("OYZU_TEST_INSTALLER", &fixture.installer)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    let first = spawn();
    let second = spawn();
    for child in [first, second] {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(
        fs::read_dir(fixture.store.join("installs"))
            .unwrap()
            .count(),
        1
    );
    assert!(fixture.verify().is_ok());
    let key = fixture.receipt["installation_key"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    let guard = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.store.join("locks").join(format!("{key}.lease")))
        .unwrap();
    guard.try_lock().unwrap();
}

#[test]
fn terminated_owner_releases_kernel_lease_and_retains_valid_commit() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let fixture = Fixture::new();
    let staging = fixture.stage();
    let ready = fixture._temp.path().join("ready");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "publication_child", "--nocapture"])
        .env("OYZU_TEST_STORE", &fixture.store)
        .env("OYZU_TEST_LOCK", &fixture.lock)
        .env("OYZU_TEST_STAGING", &staging)
        .env("OYZU_TEST_PLATFORM", fixture.platform)
        .env("OYZU_TEST_INSTALLER", &fixture.installer)
        .env("OYZU_TEST_READY", &ready)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        if Instant::now() > deadline || child.try_wait().unwrap().is_some() {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!(
                "lease owner did not become ready: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let key = fixture.receipt["installation_key"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    let guard = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.store.join("locks").join(format!("{key}.lease")))
        .unwrap();
    let blocked = matches!(guard.try_lock(), Err(std::fs::TryLockError::WouldBlock));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(blocked);
    guard.try_lock().unwrap();
    assert!(fixture.verify().is_ok());
}

#[test]
fn rejects_extra_installation_files_and_linked_receipts() {
    let fixture = Fixture::new();
    fs::write(
        fixture.install.join("unexpected"),
        b"not part of the installation",
    )
    .unwrap();
    assert!(fixture.verify().is_err());
    fs::remove_file(fixture.install.join("unexpected")).unwrap();
    fs::hard_link(
        fixture.install.join("receipt.json"),
        fixture._temp.path().join("external-receipt"),
    )
    .unwrap();
    assert!(fixture.verify().is_err());
}
