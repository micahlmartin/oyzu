//! Opt-in real upstream ZIP qualification, independent of backend authorization.
mod tool_store_fixture;

use oyzu::{
    records,
    tools::{self, ToolCandidateRequest},
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

const DIGEST: &str = "sha256:55b639295920b219bb2acbcfa00f90393a2789095b7323f79475c9f34795f217";
const SIZE: u64 = 34906389;
const PLATFORM: &str = "windows/amd64/msvc";

#[test]
#[ignore = "requires externally provisioned Node archive and independent manifest; see reference"]
fn real_node_zip_publication_parity_and_changed_lock_denial() -> anyhow::Result<()> {
    let archive =
        PathBuf::from(std::env::var_os("OYZU_NODE_STORE_ARCHIVE").expect("archive path required"));
    let manifest = PathBuf::from(
        std::env::var_os("OYZU_NODE_STORE_MANIFEST").expect("manifest path required"),
    );
    let expected: Value = serde_json::from_slice(&fs::read(manifest)?)?;
    assert_eq!(expected["archive_digest"], DIGEST);
    assert_eq!(expected["archive_size"], SIZE);
    assert_eq!(expected["version"], "22.14.0");
    let temp = tool_store_fixture::directory()?;
    let store = temp.path().join("store");
    let staging = temp.path().join("candidates");
    let lock_path = temp.path().join("oyzu.lock");
    fs::create_dir(&store)?;
    // Synthetic admission/installer identities deliberately do not claim a
    // production backend. The artifact, file bytes and version are real.
    let backend = format!("sha256:{}", "a".repeat(64));
    let installer = format!("sha256:{}", "1".repeat(64));
    let key = records::digest(
        "oyzu.tool-record.v2",
        &json!({
            "id":"core:node", "version":"22.14.0", "backend_digest":backend, "options":{}
        }),
    )?;
    let mut plan: Value = serde_json::from_str(include_str!("fixtures/tool-layout/plan.json"))?;
    plan["platform"] = json!(PLATFORM);
    plan["archive_kind"] = json!("zip");
    plan["strip_prefix"] = json!("node-v22.14.0-win-x64");
    plan["input_blob_digests"] = json!([DIGEST]);
    plan["required_paths"] = json!([
        {"path":"LICENSE", "kind":"file"},
        {"path":"node.exe", "kind":"file"}
    ]);
    plan["entrypoints"]["node"]["payload_relative_path"] = json!("node.exe");
    plan["environment"]["PATH"]["paths"][0]["relative_path"] = json!(".");
    let layout_digest = records::digest("oyzu.archive-layout.v1", &plan)?;
    let mut lock: toml::Value = toml::from_str(include_str!("fixtures/tool-lock/valid.toml"))?;
    lock["environment"][0]["roots"] = vec![key.clone()].into();
    lock["environment"][0]["requests"]["core:node"] = "22.14.0".into();
    lock["tool"][0]["key"] = key.clone().into();
    lock["tool"][0]["version"] = "22.14.0".into();
    let distribution = &mut lock["tool"][0]["distribution"][0];
    distribution["platform"] = PLATFORM.into();
    distribution["digest"] = DIGEST.into();
    distribution["size"] = (SIZE as i64).into();
    distribution["artifact_id"] = "node-v22.14.0-win-x64.zip".into();
    distribution["layout_digest"] = layout_digest.clone().into();
    distribution["verification"]["subject_digest"] = DIGEST.into();
    fs::write(&lock_path, toml::to_string(&lock)?)?;
    let original_lock = fs::read(&lock_path)?;
    let inspection = tools::inspect_lock(&lock_path)?;
    let installation = &inspection.selections[0].installation_keys[&key];
    let candidate = staging.join("installs").join(&installation[7..]);
    fs::create_dir_all(&candidate)?;
    let blob = tools::cache_tool_blob(&store, &mut fs::File::open(archive)?, DIGEST, SIZE)?;
    assert_eq!(
        tools::stage_tool_candidate(
            ToolCandidateRequest {
                lock_path: &lock_path,
                staging: &candidate,
                scope: ".",
                profile: "default",
                platform: PLATFORM,
                tool_key: &key,
                installer_release_digest: &installer,
                admitted_layout_digest: &layout_digest,
            },
            &serde_json::to_vec(&plan)?,
            blob
        )?,
        *installation
    );
    let tree = tools::inspect_tree(&candidate.join("payload"))?;
    let actual: serde_json::Map<String, Value> = tree
        .entries
        .iter()
        .map(|entry| {
            (
                entry.path.clone(),
                json!({"type": entry.kind, "size": entry.size, "digest": entry.digest}),
            )
        })
        .collect();
    assert_eq!(
        Value::Object(actual),
        expected["entries"],
        "independent Python ZIP inventory parity"
    );
    let lease = tools::lease_installation_selection(
        &lock_path,
        &store,
        Some(&staging),
        ".",
        "default",
        PLATFORM,
        &installer,
    )?;
    assert_eq!(
        tools::verify_installation_selection(
            &lock_path, &store, ".", "default", PLATFORM, &installer
        )?,
        lease.selection_digest
    );
    assert_eq!(fs::read(&lock_path)?, original_lock);
    let executable = store
        .join("installs")
        .join(&installation[7..])
        .join("payload/node.exe");
    assert!(executable.is_file());
    #[cfg(windows)]
    {
        let mut command = std::process::Command::new(&executable);
        command
            .arg("--version")
            .env_clear()
            .current_dir(temp.path());
        if let Some(root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        let output = command.output()?;
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout)?.trim(), "v22.14.0");
    }
    // Preserve the version and bytes; change only the locked archive identity.
    let changed = format!("sha256:{}", "2".repeat(64));
    lock["tool"][0]["distribution"][0]["digest"] = changed.clone().into();
    lock["tool"][0]["distribution"][0]["verification"]["subject_digest"] = changed.into();
    fs::write(&lock_path, toml::to_string(&lock)?)?;
    assert!(tools::verify_installation_selection(
        &lock_path, &store, ".", "default", PLATFORM, &installer
    )
    .is_err());
    assert!(tools::lease_installation_selection(
        &lock_path, &store, None, ".", "default", PLATFORM, &installer
    )
    .is_err());
    fs::write(&lock_path, original_lock)?;
    assert_eq!(
        tools::verify_installation_selection(
            &lock_path, &store, ".", "default", PLATFORM, &installer
        )?,
        lease.selection_digest
    );
    eprintln!("real Node ZIP: {} entries match; publication and changed-lock denial passed; native execution={}", tree.entries.len(), cfg!(windows));
    Ok(())
}
