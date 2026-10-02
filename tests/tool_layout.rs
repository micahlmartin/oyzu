mod tool_store_fixture;

use oyzu::tools::{self, ToolCandidateRequest};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn identity(value: &Value) -> String {
    let mut bytes = b"oyzu.archive-layout.v1\0".to_vec();
    bytes.extend(serde_json_canonicalizer::to_vec(value).unwrap());
    hash(&bytes)
}
fn archive(extra: Option<(&str, &[u8])>) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    for (path, bytes) in
        std::iter::once(("release/bin/node", b"synthetic, never executed".as_slice())).chain(extra)
    {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        header.set_size(bytes.len() as u64);
        header.set_cksum();
        archive.append_data(&mut header, path, bytes).unwrap();
    }
    archive.into_inner().unwrap()
}
struct Fixture {
    _temp: tempfile::TempDir,
    store: PathBuf,
    staging: PathBuf,
    lock: PathBuf,
    tool_key: String,
    key: String,
    platform: &'static str,
    installer: String,
    layout: Value,
    bytes: Vec<u8>,
}
impl Fixture {
    fn new() -> Self {
        let temp = tool_store_fixture::directory().unwrap();
        let store = temp.path().join("store");
        let staging = temp.path().join("candidates");
        let lock = temp.path().join("oyzu.lock");
        fs::create_dir(&store).unwrap();
        let platform = if cfg!(windows) {
            "windows/amd64/msvc"
        } else if cfg!(target_os = "macos") {
            "darwin/arm64/native"
        } else {
            "linux/amd64/gnu"
        };
        let mut layout: Value =
            serde_json::from_str(include_str!("fixtures/tool-layout/plan.json")).unwrap();
        layout["platform"] = json!(platform);
        let mut result = Self {
            _temp: temp,
            store,
            staging,
            lock,
            tool_key: String::new(),
            key: String::new(),
            platform,
            installer: format!("sha256:{}", "1".repeat(64)),
            layout,
            bytes: archive(None),
        };
        result.prepare();
        result
    }
    fn prepare(&mut self) {
        let mut lock: toml::Value =
            toml::from_str(include_str!("fixtures/tool-lock/valid.toml")).unwrap();
        self.layout["input_blob_digests"] = json!([hash(&self.bytes)]);
        let distribution = &mut lock["tool"][0]["distribution"][0];
        distribution["digest"] = hash(&self.bytes).into();
        distribution["size"] = (self.bytes.len() as i64).into();
        distribution["verification"]["subject_digest"] = hash(&self.bytes).into();
        distribution["platform"] = self.platform.into();
        distribution["layout_digest"] = identity(&self.layout).into();
        fs::write(&self.lock, toml::to_string(&lock).unwrap()).unwrap();
        let inspection = tools::inspect_lock(&self.lock).unwrap();
        let (tool_key, key) = inspection.selections[0]
            .installation_keys
            .iter()
            .next()
            .unwrap();
        self.tool_key = tool_key.clone();
        self.key = key.clone();
        fs::create_dir_all(self.candidate()).unwrap();
    }
    fn candidate(&self) -> PathBuf {
        self.staging.join("installs").join(&self.key[7..])
    }
    fn stage(&self) -> anyhow::Result<String> {
        self.stage_bytes(&serde_json::to_vec(&self.layout)?)
    }
    fn stage_bytes(&self, plan: &[u8]) -> anyhow::Result<String> {
        let blob = tools::cache_tool_blob(
            &self.store,
            &mut self.bytes.as_slice(),
            &hash(&self.bytes),
            self.bytes.len() as u64,
        )?;
        tools::stage_tool_candidate(
            ToolCandidateRequest {
                lock_path: &self.lock,
                staging: &self.candidate(),
                scope: ".",
                profile: "default",
                platform: self.platform,
                tool_key: &self.tool_key,
                installer_release_digest: &self.installer,
                admitted_layout_digest: &identity(&self.layout),
            },
            plan,
            blob,
        )
    }
}

#[test]
fn runtime_rejects_shared_schema_invalid_shape_corpus() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/tool-layout/invalid-shapes.json")).unwrap();
    for case in cases {
        let mut fixture = Fixture::new();
        let (parent, field) = case["pointer"].as_str().unwrap().rsplit_once('/').unwrap();
        let object = fixture
            .layout
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap();
        if case["remove"] == true {
            object.remove(field);
        } else {
            object.insert(field.into(), case["value"].clone());
        }
        fixture.prepare();
        assert!(fixture.stage().is_err(), "{}", case["name"]);
        assert!(!fixture.candidate().join("receipt.json").exists());
    }
}

#[test]
fn finalizes_stripped_archive_and_publishes_verifiable_locked_receipt() {
    let fixture = Fixture::new();
    let original = fs::read(&fixture.lock).unwrap();
    assert_eq!(fixture.stage().unwrap(), fixture.key);
    assert_eq!(
        fs::read(fixture.candidate().join("payload/bin/node")).unwrap(),
        b"synthetic, never executed"
    );
    assert!(!fixture.candidate().join("payload/release").exists());
    let receipt: Value =
        serde_json::from_slice(&fs::read(fixture.candidate().join("receipt.json")).unwrap())
            .unwrap();
    assert_eq!(
        receipt["environment"]["PATH"]["paths"][0]["installation_key"],
        fixture.key
    );
    let lease = tools::lease_installation_selection(
        &fixture.lock,
        &fixture.store,
        Some(&fixture.staging),
        ".",
        "default",
        fixture.platform,
        &fixture.installer,
    )
    .unwrap();
    assert_eq!(
        tools::verify_installation_selection(
            &fixture.lock,
            &fixture.store,
            ".",
            "default",
            fixture.platform,
            &fixture.installer
        )
        .unwrap(),
        lease.selection_digest
    );
    assert_eq!(fs::read(&fixture.lock).unwrap(), original);
}

fn zip_archive(method: zip::CompressionMethod) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);
    writer.start_file("release/bin/node", options).unwrap();
    writer.write_all(b"synthetic, never executed").unwrap();
    writer.finish().unwrap().into_inner()
}

#[test]
fn zip_stored_and_deflated_layouts_publish_verifiable_receipts() {
    for method in [
        zip::CompressionMethod::Stored,
        zip::CompressionMethod::Deflated,
    ] {
        let mut fixture = Fixture::new();
        fixture.bytes = zip_archive(method);
        fixture.layout["archive_kind"] = json!("zip");
        fixture.prepare();
        assert_eq!(fixture.stage().unwrap(), fixture.key);
        assert_eq!(
            fs::read(fixture.candidate().join("payload/bin/node")).unwrap(),
            b"synthetic, never executed"
        );
        let lease = tools::lease_installation_selection(
            &fixture.lock,
            &fixture.store,
            Some(&fixture.staging),
            ".",
            "default",
            fixture.platform,
            &fixture.installer,
        )
        .unwrap();
        assert_eq!(
            tools::verify_installation_selection(
                &fixture.lock,
                &fixture.store,
                ".",
                "default",
                fixture.platform,
                &fixture.installer,
            )
            .unwrap(),
            lease.selection_digest
        );
    }
}

#[test]
fn zip_corruption_and_header_disagreement_never_create_receipts() {
    for mutation in 0..6 {
        let mut fixture = Fixture::new();
        fixture.bytes = zip_archive(zip::CompressionMethod::Stored);
        fixture.layout["archive_kind"] = json!("zip");
        let central = fixture
            .bytes
            .windows(4)
            .position(|b| b == b"PK\x01\x02")
            .unwrap();
        match mutation {
            0 => fixture.bytes[30 + "release/bin/node".len()] ^= 1,
            1 => fixture.bytes[30] = b'X',
            2 => fixture.bytes[central + 8] |= 1,
            3 => fixture.bytes[central + 24..central + 28].copy_from_slice(&u32::MAX.to_le_bytes()),
            4 => {
                fixture.bytes.pop();
            }
            _ => fixture.bytes[central + 42] = 1,
        }
        fixture.prepare();
        assert!(fixture.stage().is_err(), "mutation {mutation}");
        assert!(!fixture.candidate().join("receipt.json").exists());
    }
}

#[test]
fn rejects_unadmitted_changed_plan_and_open_records_before_extraction() {
    for mutate in [
        |p: &mut Value| p["strip_prefix"] = json!("different"),
        |p: &mut Value| p["arbitrary_script"] = json!("echo unsafe"),
        |p: &mut Value| {
            p.as_object_mut().unwrap().remove("strip_prefix");
        },
        |p: &mut Value| {
            p["entrypoints"]["node"]
                .as_object_mut()
                .unwrap()
                .remove("interpreter_tool_key");
        },
    ] {
        let fixture = Fixture::new();
        let mut changed = fixture.layout.clone();
        mutate(&mut changed);
        assert!(fixture
            .stage_bytes(&serde_json::to_vec(&changed).unwrap())
            .is_err());
        assert_eq!(fs::read_dir(fixture.candidate()).unwrap().count(), 0);
    }
    let fixture = Fixture::new();
    let bytes = serde_json::to_string(&fixture.layout)
        .unwrap()
        .replacen("{", "{\"format\":1,", 1);
    assert!(fixture.stage_bytes(bytes.as_bytes()).is_err());
}

#[test]
fn rejects_invalid_templates_transforms_and_bounds_without_receipt() {
    for mutate in [
        |p: &mut Value| p["archive_kind"] = json!("zip"),
        |p: &mut Value| p["payload_subtree"] = json!("other"),
        |p: &mut Value| p["executable_paths"] = json!(["../node"]),
        |p: &mut Value| p["strip_prefix"] = json!("../escape"),
        |p: &mut Value| p["extraction_bounds"]["max_entries"] = json!(0),
        |p: &mut Value| p["extraction_bounds"]["max_depth"] = json!(65),
        |p: &mut Value| p["required_paths"][0]["path"] = json!("bin/missing"),
        |p: &mut Value| p["required_paths"][0]["kind"] = json!("directory"),
        |p: &mut Value| p["entrypoints"]["node"]["payload_relative_path"] = json!("bin"),
        |p: &mut Value| p["entrypoints"]["node"]["interpreter_tool_key"] = json!("self"),
        |p: &mut Value| {
            p["entrypoints"]["node"]["prefix_args"] = json!([{"kind":"literal","value":"\0"}])
        },
        |p: &mut Value| p["environment"]["PATH"] = json!({"kind":"literal","value":"/ambient"}),
        |p: &mut Value| p["environment"]["PATH"]["paths"][0]["owner"] = json!("undeclared"),
        |p: &mut Value| p["environment"]["PATH"]["paths"][0]["relative_path"] = json!("bin/node"),
    ] {
        let mut fixture = Fixture::new();
        mutate(&mut fixture.layout);
        fixture.prepare();
        assert!(fixture.stage().is_err(), "{}", fixture.layout);
        assert!(!fixture.candidate().join("receipt.json").exists());
    }
}

#[test]
fn rejects_entries_outside_prefix_and_enforces_smaller_extraction_limits() {
    let mut fixture = Fixture::new();
    fixture.bytes = archive(Some(("outside", b"bad")));
    fixture.prepare();
    assert!(fixture.stage().is_err());
    assert!(!fixture.candidate().join("receipt.json").exists());
    for (field, value) in [("max_file_bytes", 4), ("max_entries", 1), ("max_depth", 1)] {
        let mut fixture = Fixture::new();
        fixture.layout["extraction_bounds"][field] = json!(value);
        fixture.prepare();
        assert!(fixture.stage().is_err(), "{field}");
        assert!(!fixture.candidate().join("receipt.json").exists());
    }
}

#[test]
fn declared_executable_paths_are_host_specific_bounded_and_receipt_bound() {
    let mut fixture = Fixture::new();
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o600);
    header.set_size(4);
    header.set_cksum();
    builder
        .append_data(&mut header, "release/bin/node", &b"data"[..])
        .unwrap();
    fixture.bytes = builder.into_inner().unwrap();
    fixture.layout["executable_paths"] = json!(["bin/node"]);
    fixture.prepare();
    if cfg!(windows) {
        assert!(fixture.stage().is_err());
        assert_eq!(fs::read_dir(fixture.candidate()).unwrap().count(), 0);
        return;
    }
    fixture.stage().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(fixture.candidate().join("payload/bin/node"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o700
        );
    }
    let lease = tools::lease_installation_selection(
        &fixture.lock,
        &fixture.store,
        Some(&fixture.staging),
        ".",
        "default",
        fixture.platform,
        &fixture.installer,
    )
    .unwrap();
    drop(lease);
    for paths in [
        json!(["bin/node", "bin/node"]),
        json!(["bin/node", "bin"]),
        json!(["bin"]),
        json!(["missing"]),
    ] {
        let mut fixture = Fixture::new();
        fixture.layout["executable_paths"] = paths;
        fixture.prepare();
        assert!(fixture.stage().is_err());
        assert!(!fixture.candidate().join("receipt.json").exists());
    }
}

#[test]
fn interpreted_self_entrypoints_resolve_without_running_code() {
    let mut fixture = Fixture::new();
    fixture.bytes = archive(Some(("release/script.js", b"not executed")));
    fixture.layout["entrypoints"]["runner"] = json!({"kind":"interpreter","payload_relative_path":"script.js",
        "interpreter_tool_key":"self","interpreter_relative_path":"bin/node",
        "prefix_args":[{"kind":"path","path":{"owner":"self","relative_path":"script.js"}}]});
    fixture.prepare();
    fixture.stage().unwrap();
    assert!(tools::lease_installation_selection(
        &fixture.lock,
        &fixture.store,
        Some(&fixture.staging),
        ".",
        "default",
        fixture.platform,
        &fixture.installer
    )
    .is_ok());
}

#[test]
fn descriptor_bindings_and_gzip_expansion_bounds_fail_before_receipt() {
    for field in ["backend_digest", "platform"] {
        let mut fixture = Fixture::new();
        fixture.layout[field] = if field == "backend_digest" {
            json!(format!("sha256:{}", "f".repeat(64)))
        } else {
            json!("darwin/amd64/native")
        };
        fixture.prepare();
        assert!(fixture.stage().is_err());
        assert_eq!(fs::read_dir(fixture.candidate()).unwrap().count(), 0);
    }
    use std::io::Write;
    for ratio in [0, 1, 200, 800, 1024, 1025] {
        let mut fixture = Fixture::new();
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&fixture.bytes).unwrap();
        fixture.bytes = encoder.finish().unwrap();
        fixture.layout["archive_kind"] = json!("tar.gz");
        fixture.layout["extraction_bounds"]["max_expansion_ratio"] = json!(ratio);
        fixture.prepare();
        assert_eq!(fixture.stage().is_ok(), (200..=1024).contains(&ratio));
        assert_eq!(
            fixture.candidate().join("receipt.json").exists(),
            (200..=1024).contains(&ratio)
        );
    }
}

#[cfg(unix)]
#[test]
fn stripped_layout_checks_links_in_final_payload_without_rewriting_targets() {
    for target in ["node", "../../outside"] {
        let mut fixture = Fixture::new();
        let mut builder = tar::Builder::new(Vec::new());
        let mut file = tar::Header::new_gnu();
        file.set_mode(0o755);
        file.set_size(4);
        file.set_cksum();
        builder
            .append_data(&mut file, "release/bin/node", &b"data"[..])
            .unwrap();
        let mut link = tar::Header::new_gnu();
        link.set_mode(0o777);
        link.set_size(0);
        link.set_entry_type(tar::EntryType::Symlink);
        link.set_link_name(target).unwrap();
        link.set_cksum();
        builder
            .append_data(&mut link, "release/bin/alias", std::io::empty())
            .unwrap();
        fixture.bytes = builder.into_inner().unwrap();
        fixture.prepare();
        assert_eq!(fixture.stage().is_ok(), target == "node");
        assert_eq!(
            fixture.candidate().join("receipt.json").exists(),
            target == "node"
        );
    }
}

#[test]
fn raw_artifact_uses_exact_declared_path_and_bounded_parent_creation() {
    let mut fixture = Fixture::new();
    fixture.bytes = b"raw synthetic artifact, never executed".to_vec();
    if cfg!(unix) {
        fixture.layout["executable_paths"] = json!(["bin/node"]);
    }
    fixture.layout["archive_kind"] = json!("raw");
    fixture.layout["strip_prefix"] = Value::Null;
    fixture.prepare();
    assert_eq!(fixture.stage().unwrap(), fixture.key);
    assert_eq!(
        fs::read(fixture.candidate().join("payload/bin/node")).unwrap(),
        fixture.bytes
    );
    let lease = tools::lease_installation_selection(
        &fixture.lock,
        &fixture.store,
        Some(&fixture.staging),
        ".",
        "default",
        fixture.platform,
        &fixture.installer,
    )
    .unwrap();
    assert!(!lease.selection_digest.is_empty());
    for (pointer, value) in [
        ("/strip_prefix", json!("prefix")),
        ("/required_paths", json!([])),
        (
            "/required_paths",
            json!([{"path":"bin/node", "kind":"directory"}]),
        ),
        (
            "/required_paths",
            json!([{"path":"bin/node", "kind":"file"}, {"path":"other", "kind":"file"}]),
        ),
        (
            "/required_paths",
            json!([{"path":"../escape", "kind":"file"}]),
        ),
        ("/extraction_bounds/max_entries", json!(1)),
        ("/extraction_bounds/max_depth", json!(1)),
        ("/extraction_bounds/max_file_bytes", json!(1)),
    ] {
        let mut bad = Fixture::new();
        bad.bytes = b"raw synthetic artifact".to_vec();
        bad.layout["archive_kind"] = json!("raw");
        bad.layout["strip_prefix"] = Value::Null;
        *bad.layout.pointer_mut(pointer).unwrap() = value;
        bad.prepare();
        assert!(bad.stage().is_err(), "{pointer}");
        assert!(!bad.candidate().join("receipt.json").exists());
    }
}
