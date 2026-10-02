//! Opt-in real upstream artifact qualification, independent of backend authorization.
mod tool_store_fixture;

use oyzu::{
    records,
    tools::{self, ToolCandidateRequest},
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

const PLATFORM: &str = "windows/amd64/msvc";

struct ArtifactCase {
    tool: &'static str,
    fixture: &'static str,
    canonical: &'static str,
    kind: &'static str,
    version: &'static str,
    digest: &'static str,
    size: u64,
    prefix: &'static str,
    artifact: &'static str,
    executable: &'static str,
    path: &'static str,
    ratio: u32,
}

#[test]
#[ignore = "requires externally provisioned Node archive and independent manifest; see reference"]
fn real_node_zip_publication_parity_and_changed_lock_denial() -> anyhow::Result<()> {
    qualify(ArtifactCase {
        tool: "node",
        fixture: "NODE",
        canonical: "core:node",
        kind: "zip",
        version: "22.14.0",
        digest: "sha256:55b639295920b219bb2acbcfa00f90393a2789095b7323f79475c9f34795f217",
        size: 34906389,
        prefix: "node-v22.14.0-win-x64",
        artifact: "node-v22.14.0-win-x64.zip",
        executable: "node.exe",
        path: ".",
        ratio: 200,
    })
}

#[test]
#[ignore = "requires externally provisioned Go archive and independent manifest; see reference"]
fn real_go_zip_publication_parity_and_changed_lock_denial() -> anyhow::Result<()> {
    qualify(ArtifactCase {
        tool: "go",
        fixture: "GO",
        canonical: "core:go",
        kind: "zip",
        version: "1.24.13",
        digest: "sha256:40b16bc8f00540a2cb02dff4de72b73e966fdd8d65f95e33d8e4080b48a2459a",
        size: 87295983,
        prefix: "go",
        artifact: "go1.24.13.windows-amd64.zip",
        executable: "bin/go.exe",
        path: "bin",
        ratio: 800,
    })
}

#[test]
#[ignore = "requires externally provisioned Go 1.25 archive and independent manifest; see reference"]
fn real_go125_zip_publication_parity_and_changed_lock_denial() -> anyhow::Result<()> {
    qualify(ArtifactCase {
        tool: "go",
        fixture: "GO125",
        canonical: "core:go",
        kind: "zip",
        version: "1.25.0",
        digest: "sha256:89efb4f9b30812eee083cc1770fdd2913c14d301064f6454851428f9707d190b",
        size: 67418204,
        prefix: "go",
        artifact: "go1.25.0.windows-amd64.zip",
        executable: "bin/go.exe",
        path: "bin",
        ratio: 800,
    })
}

#[test]
#[ignore = "requires externally provisioned jq artifact and independent manifest; see reference"]
fn real_jq_raw_publication_parity_and_changed_lock_denial() -> anyhow::Result<()> {
    qualify(ArtifactCase {
        tool: "jq",
        fixture: "JQ",
        canonical: "aqua:jqlang/jq",
        kind: "raw",
        version: "1.8.1",
        digest: "sha256:23cb60a1354eed6bcc8d9b9735e8c7b388cd1fdcb75726b93bc299ef22dd9334",
        size: 1026560,
        prefix: "",
        artifact: "jq-windows-amd64.exe",
        executable: "jq.exe",
        path: ".",
        ratio: 200,
    })
}

fn qualify(case: ArtifactCase) -> anyhow::Result<()> {
    let prefix = format!("OYZU_{}_STORE", case.fixture);
    let archive = PathBuf::from(
        std::env::var_os(format!("{prefix}_ARCHIVE")).expect("archive path required"),
    );
    let manifest = PathBuf::from(
        std::env::var_os(format!("{prefix}_MANIFEST")).expect("manifest path required"),
    );
    let expected: Value = serde_json::from_slice(&fs::read(manifest)?)?;
    assert_eq!(expected["archive_digest"], case.digest);
    assert_eq!(expected["archive_size"], case.size);
    assert_eq!(expected["version"], case.version);
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
            "id":case.canonical.to_owned(), "version":case.version, "backend_digest":backend, "options":{}
        }),
    )?;
    let mut plan: Value = serde_json::from_str(include_str!("fixtures/tool-layout/plan.json"))?;
    plan["platform"] = json!(PLATFORM);
    plan["archive_kind"] = json!(case.kind);
    plan["extraction_bounds"]["max_expansion_ratio"] = json!(case.ratio);
    plan["strip_prefix"] = if case.kind == "raw" {
        Value::Null
    } else {
        json!(case.prefix)
    };
    plan["input_blob_digests"] = json!([case.digest]);
    plan["required_paths"] = json!([
        {"path":"LICENSE", "kind":"file"},
        {"path":case.executable, "kind":"file"}
    ]);
    if case.kind == "raw" {
        plan["required_paths"] = json!([{"path":case.executable, "kind":"file"}]);
    }
    plan["entrypoints"] = json!({(case.tool): {
        "kind":"native", "payload_relative_path":case.executable,
        "interpreter_tool_key":null, "interpreter_relative_path":null, "prefix_args":[]
    }});
    plan["environment"]["PATH"]["paths"][0]["relative_path"] = json!(case.path);
    if case.tool == "go" {
        plan["environment"]["GOROOT"] = json!({
            "kind":"paths", "paths":[{"owner":"self", "relative_path":"."}]
        });
    }
    let layout_digest = records::digest("oyzu.archive-layout.v1", &plan)?;
    let mut lock: toml::Value = toml::from_str(include_str!("fixtures/tool-lock/valid.toml"))?;
    lock["environment"][0]["roots"] = vec![key.clone()].into();
    let mut requests = toml::map::Map::new();
    requests.insert(case.canonical.to_owned(), case.version.into());
    lock["environment"][0]["requests"] = toml::Value::Table(requests);
    lock["tool"][0]["id"] = case.canonical.to_owned().into();
    lock["tool"][0]["key"] = key.clone().into();
    lock["tool"][0]["version"] = case.version.into();
    let distribution = &mut lock["tool"][0]["distribution"][0];
    distribution["platform"] = PLATFORM.into();
    distribution["digest"] = case.digest.into();
    distribution["size"] = (case.size as i64).into();
    distribution["artifact_id"] = case.artifact.into();
    distribution["source_id"] = format!("{}-releases", case.tool).into();
    distribution["layout_digest"] = layout_digest.clone().into();
    distribution["verification"]["subject_digest"] = case.digest.into();
    fs::write(&lock_path, toml::to_string(&lock)?)?;
    let original_lock = fs::read(&lock_path)?;
    let inspection = tools::inspect_lock(&lock_path)?;
    let installation = &inspection.selections[0].installation_keys[&key];
    let candidate = staging.join("installs").join(&installation[7..]);
    fs::create_dir_all(&candidate)?;
    let blob = tools::cache_tool_blob(
        &store,
        &mut fs::File::open(archive)?,
        case.digest,
        case.size,
    )?;
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
        "independent Python artifact inventory parity"
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
        .join("payload")
        .join(case.executable);
    assert!(executable.is_file());
    #[cfg(windows)]
    {
        let mut command = std::process::Command::new(&executable);
        command
            .arg(if case.tool == "go" {
                "version"
            } else {
                "--version"
            })
            .env_clear()
            .current_dir(temp.path());
        if let Some(root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        let output = command.output()?;
        assert!(output.status.success());
        let expected_version = if case.tool == "go" {
            format!("go version go{} windows/amd64", case.version)
        } else if case.tool == "jq" {
            format!("jq-{}", case.version)
        } else {
            format!("v{}", case.version)
        };
        assert_eq!(String::from_utf8(output.stdout)?.trim(), expected_version);
        if case.tool == "go" {
            qualify_go_build(&executable, temp.path())?;
        }
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
    eprintln!("real {} artifact: {} entries match; publication and changed-lock denial passed; native execution={}", case.tool, tree.entries.len(), cfg!(windows));
    Ok(())
}

/// Exercise the published compiler and standard library, with no inherited Go
/// settings, automatic toolchain download, module proxy or ambient compiler.
/// The caller keeps the verified installation lease alive for this whole run.
#[cfg(windows)]
fn qualify_go_build(executable: &std::path::Path, scratch: &std::path::Path) -> anyhow::Result<()> {
    let payload = executable.parent().unwrap().parent().unwrap();
    let cache = scratch.join("go-cache");
    let temporary = scratch.join("go-temp");
    fs::create_dir(&cache)?;
    fs::create_dir(&temporary)?;
    let command = || {
        let mut command = std::process::Command::new(executable);
        command
            .env_clear()
            .current_dir(scratch)
            .env("GOROOT", payload)
            .env("GOCACHE", &cache)
            .env("GOTMPDIR", &temporary)
            .env("GOPATH", scratch.join("go-path"))
            .env("GOMODCACHE", scratch.join("go-modules"))
            .env("GOTOOLCHAIN", "local")
            .env("GOENV", "off")
            .env("GOWORK", "off")
            .env("GOPROXY", "off")
            .env("GOSUMDB", "off")
            .env("CGO_ENABLED", "0");
        if let Some(root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        command
    };
    let output = command().args(["env", "GOROOT"]).output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::canonicalize(String::from_utf8(output.stdout)?.trim())?,
        fs::canonicalize(payload)?
    );
    fs::write(
        scratch.join("go.mod"),
        "module example.invalid/oyzu-store-check\n\ngo 1.24.0\n",
    )?;
    fs::write(
        scratch.join("main.go"),
        "package main\nimport \"fmt\"\nfunc main() { fmt.Println(\"oyzu-go-store-ok\") }\n",
    )?;
    let binary = scratch.join("go-store-check.exe");
    let output = command()
        .args(["build", "-mod=readonly", "-trimpath", "-o"])
        .arg(&binary)
        .arg(".")
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = std::process::Command::new(binary)
        .env_clear()
        .current_dir(scratch)
        .output()?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?.trim(), "oyzu-go-store-ok");
    eprintln!(
        "real Go: published GOROOT, offline module build and native program execution passed"
    );
    Ok(())
}
