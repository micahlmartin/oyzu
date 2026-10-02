//! Opt-in standalone Node integration. Oyzu owns configuration, locks and storage;
//! a fresh same-image child owns mise globals. Initial development transport uses
//! child stdio; authenticated worker IPC and process hardening are future work.
use super::{lock, ToolCandidateRequest, ToolLaunch};
use crate::{broker, config, records};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
};

const PIN: &str = "9290bcac695c8ff8a56760ccebd785d5062b459c";
const SOURCE: &str = "https://nodejs.org/dist/";

#[derive(Serialize, Deserialize)]
struct Request {
    request: String,
    exact: Option<String>,
    target: String,
}
#[derive(Serialize, Deserialize)]
struct Archive {
    version: String,
    target: String,
    archive_url: String,
    archive_kind: String,
    strip_prefix: String,
    node_relative_path: String,
    bin_relative_path: String,
}
#[derive(Serialize, Deserialize)]
struct Metadata {
    archive: Archive,
    declared_sha256: Option<String>,
    aliases: BTreeMap<String, String>,
}

fn fetcher() -> Result<broker::Fetcher> {
    broker::Fetcher::new(vec![broker::Source::new("node-releases", SOURCE, None)?])
}
fn identity() -> Result<String> {
    records::digest(
        "oyzu.development-node-adapter.v1",
        &json!({"source":PIN,"features":["rustls","vendored-lua"],"adapter":1}),
    )
}
fn platform() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("linux/amd64/gnu"),
        ("macos", "aarch64") => Ok("darwin/arm64/native"),
        ("windows", "x86_64") => Ok("windows/amd64/msvc"),
        _ => anyhow::bail!("Node development integration does not support this host yet"),
    }
}

/// Internal child entrypoint. Call before creating any application thread.
pub fn worker() -> Result<i32> {
    let request: Request = serde_json::from_reader(std::io::stdin())?;
    let state = tempfile::tempdir()?;
    let transport: Arc<mise::embedding::HttpTransport> = Arc::new(|request| {
        Box::pin(async move {
            if request.method.as_str() != "GET" {
                return Err(std::io::Error::other("metadata requires GET").into());
            }
            let url = request.url.to_string();
            let response = tokio::task::spawn_blocking(move || fetcher()?.fetch(&url))
                .await?
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let response = http::Response::builder()
                .status(response.status)
                .header("content-type", response.content_type)
                .body(response.body)?;
            Ok(reqwest_mise::Response::from(response))
        })
    });
    let session = mise::embedding::Session::initialize(mise::embedding::Options {
        state: state.path().canonicalize()?,
        frontend: std::env::current_exe()?,
        tools: ["node".to_owned()].into(),
        transport: Some(transport),
    })
    .map_err(|error| anyhow::anyhow!("{error:#}"))?;
    let aliases = session
        .tool_aliases()
        .map_err(|error| anyhow::anyhow!("{error:#}"))?;
    let result = if let Some(version) = request.exact {
        let archive = session
            .node_archive_facts(&version, &request.target)
            .map_err(|error| anyhow::anyhow!("{error:#}"))?;
        Metadata {
            archive: serde_json::from_value(serde_json::to_value(archive)?)?,
            declared_sha256: None,
            aliases,
        }
    } else {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let metadata = runtime
            .block_on(async {
                let version = session.resolve_node_version(&request.request, &[]).await?;
                session
                    .node_archive_metadata(&version, &request.target)
                    .await
            })
            .map_err(|error| anyhow::anyhow!("{error:#}"))?;
        Metadata {
            archive: serde_json::from_value(serde_json::to_value(metadata.archive)?)?,
            declared_sha256: Some(metadata.declared_sha256),
            aliases,
        }
    };
    serde_json::to_writer(std::io::stdout(), &result)?;
    Ok(0)
}

fn metadata(request: &Request) -> Result<Metadata> {
    let home = tempfile::tempdir()?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("__oyzu-node-worker")
        .env_clear()
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("TEMP", home.path())
        .env("TMP", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    for name in ["PATH", "SYSTEMROOT", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .context("worker input unavailable")?
        .write_all(&serde_json::to_vec(request)?)?;
    let output = child.wait_with_output()?;
    ensure!(output.status.success(), "mise metadata worker failed");
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn configuration(
    directory: &Path,
    options: &config::session::Options,
) -> Result<(
    config::session::Session,
    config::resolve::EffectiveConfig,
    String,
)> {
    let session = config::session::Session::open(directory, options)?;
    let effective = session.resolve(directory, false)?;
    ensure!(
        effective.management.is_none(),
        "managed tool integration is not available in this standalone development proof"
    );
    ensure!(
        effective
            .values()
            .keys()
            .filter(|key| key.starts_with("tools.")
                && !matches!(key.as_str(), "tools.allowed" | "tools.catalogs"))
            .count()
            == 1,
        "development install currently requires exactly tools.node"
    );
    let request = effective
        .get("tools.node")
        .and_then(|value| value.as_str())
        .context("configure [tools] node in Oyzu TOML")?
        .to_owned();
    Ok((session, effective, request))
}

fn plan(archive: &Archive, digest: &str, backend: &str) -> serde_json::Value {
    json!({"format":1,"backend_digest":backend,"platform":archive.target,
        "input_blob_digests":[digest],"archive_kind":archive.archive_kind,"strip_prefix":archive.strip_prefix,"payload_subtree":".",
        "required_paths":[{"path":archive.node_relative_path,"kind":"file"}],
        "entrypoints":{"node":{"kind":"native","payload_relative_path":archive.node_relative_path,"interpreter_tool_key":null,"interpreter_relative_path":null,"prefix_args":[]}},
        "environment":{"PATH":{"kind":"paths","paths":[{"owner":"self","relative_path":archive.bin_relative_path}]}},
        "extraction_bounds":{"max_entries":200000,"max_bytes":8589934592u64,"max_file_bytes":1073741824,"max_depth":64,"max_expansion_ratio":200},
        "executable_paths":if cfg!(windows) { Vec::<String>::new() } else { vec![archive.node_relative_path.clone()] }})
}

/// Resolve and install a standalone Node selection, or reuse an exact existing lock.
pub fn install(directory: &Path, options: &config::session::Options, store: &Path) -> Result<i32> {
    let directory = directory.canonicalize()?;
    let (session, effective, request) = configuration(&directory, options)?;
    ensure!(
        directory == session.root,
        "initial Node install supports the workspace root scope"
    );
    let lock_path = session.root.join("oyzu.lock");
    let edit = super::ToolLockEdit::capture(&lock_path)?;
    let previous = if lock_path.exists() {
        Some(lock::parse(&super::read_record(
            &lock_path,
            lock::MAX_BYTES,
        )?)?)
    } else {
        None
    };
    if let Some(lock) = &previous {
        ensure!(
            lock.tool.len() == 1 && lock.tool[0].id == "core:node",
            "existing lock is outside the initial Node integration scope"
        );
    }
    let metadata = metadata(&Request {
        request,
        exact: previous.as_ref().map(|lock| lock.tool[0].version.clone()),
        target: platform()?.into(),
    })?;
    let requests =
        super::project_tool_requests(&effective, &metadata.aliases, &BTreeMap::new(), &[])?;
    let profile = effective.profile.as_deref().unwrap_or("default");
    let backend = identity()?;
    let mut fetcher = fetcher()?;
    let response = fetcher.fetch(&metadata.archive.archive_url)?;
    ensure!(
        response.status == 200,
        "Node archive acquisition returned {}",
        response.status
    );
    let digest = match &previous {
        Some(lock) => lock.tool[0]
            .distribution
            .iter()
            .find(|d| d.platform == platform().unwrap())
            .context("locked host platform unavailable")?
            .digest
            .clone(),
        None => metadata
            .declared_sha256
            .clone()
            .context("Node checksum missing")?,
    };
    let layout = plan(&metadata.archive, &digest, &backend);
    let layout_digest = records::digest("oyzu.archive-layout.v1", &layout)?;
    let key = records::digest(
        "oyzu.tool-record.v2",
        &json!({"id":"core:node","version":metadata.archive.version,"backend_digest":backend,"options":{}}),
    )?;
    let bytes = if let Some(lock) = &previous {
        super::select_for_tool_requests(
            &session.root,
            &directory,
            profile,
            &requests,
            platform()?,
        )?;
        ensure!(
            lock.tool[0].backend_digest == backend,
            "locked backend differs; explicit relocking required"
        );
        super::read_record(&lock_path, lock::MAX_BYTES)?
    } else {
        let document = lock::Lock {
            format: 2,
            selections: vec![],
            environment: vec![lock::Environment {
                scope: ".".into(),
                profile: profile.into(),
                request_digest: requests.digest().into(),
                roots: vec![key.clone()],
                requests: requests.requests().clone(),
            }],
            tool: vec![lock::Tool {
                key: key.clone(),
                id: "core:node".into(),
                version: metadata.archive.version.clone(),
                backend_digest: backend.clone(),
                options: BTreeMap::new(),
                distribution: vec![lock::Distribution {
                    platform: platform()?.into(),
                    digest: digest.clone(),
                    size: response.body.len() as u64,
                    source_id: "node-releases".into(),
                    artifact_id: metadata
                        .archive
                        .archive_url
                        .rsplit('/')
                        .next()
                        .context("Node artifact filename missing")?
                        .into(),
                    layout_digest: layout_digest.clone(),
                    dependencies: vec![],
                    package_closure_digest: None,
                    verification: lock::Verification {
                        kind: lock::VerificationKind::DigestOnly,
                        evidence_digest: digest.clone(),
                        verifier_digest: backend.clone(),
                        subject_digest: digest.clone(),
                    },
                }],
            }],
        };
        toml::to_string(&document)?.into_bytes()
    };
    let parsed = lock::parse(&bytes)?;
    let distribution = &parsed.tool[0].distribution[0];
    ensure!(
        distribution.layout_digest == layout_digest,
        "locked layout differs from current adapter"
    );
    std::fs::create_dir_all(store)?;
    let blob = super::cache_tool_blob(
        store,
        &mut std::io::Cursor::new(response.body),
        &digest,
        distribution.size,
    )?;
    let staging = tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)?;
    let candidate_lock = staging.path().join("oyzu.lock");
    std::fs::write(&candidate_lock, &bytes)?;
    let installation = &parsed.selections[0].installation_keys[&key];
    let candidate = staging.path().join("installs").join(&installation[7..]);
    std::fs::create_dir_all(&candidate)?;
    super::stage_tool_candidate(
        ToolCandidateRequest {
            lock_path: &candidate_lock,
            staging: &candidate,
            scope: ".",
            profile,
            platform: platform()?,
            tool_key: &key,
            installer_release_digest: &backend,
            admitted_layout_digest: &layout_digest,
        },
        &serde_json::to_vec(&layout)?,
        blob,
    )?;
    let _lease = super::lease_installation_selection(
        &candidate_lock,
        store,
        Some(staging.path()),
        ".",
        profile,
        platform()?,
        &backend,
    )?;
    edit.propose(&bytes)?.commit()?;
    println!("Installed node {}", metadata.archive.version);
    Ok(0)
}

/// Execute the installed frozen Node command while holding its verified lease.
pub fn exec(
    directory: &Path,
    options: &config::session::Options,
    store: &Path,
    arguments: &[OsString],
) -> Result<i32> {
    let directory = directory.canonicalize()?;
    let (session, effective, request) = configuration(&directory, options)?;
    let lock_path = session.root.join("oyzu.lock");
    let document = lock::parse(&super::read_record(&lock_path, lock::MAX_BYTES)?)?;
    ensure!(
        document.tool.len() == 1 && document.tool[0].id == "core:node",
        "initial exec supports a Node-only lock"
    );
    let metadata = metadata(&Request {
        request,
        exact: Some(document.tool[0].version.clone()),
        target: platform()?.into(),
    })?;
    let requests =
        super::project_tool_requests(&effective, &metadata.aliases, &BTreeMap::new(), &[])?;
    let profile = effective.profile.as_deref().unwrap_or("default");
    let selection = super::select_for_tool_requests(
        &session.root,
        &directory,
        profile,
        &requests,
        platform()?,
    )?;
    let lease = super::lease_installation_selection(
        &lock_path,
        store,
        None,
        &selection.scope,
        profile,
        platform()?,
        &identity()?,
    )?;
    ensure!(
        arguments.first().is_some_and(|argument| argument == "node"),
        "initial exec command must be node"
    );
    let selected = lease.command("node")?;
    let ToolLaunch::Native {
        payload_relative_path,
        prefix_args,
    } = selected.launch
    else {
        anyhow::bail!("Node requires a native launch descriptor")
    };
    ensure!(prefix_args.is_empty(), "unexpected Node prefix arguments");
    let payload = std::path::absolute(store)?
        .join("installs")
        .join(&selected.installation_key[7..])
        .join("payload");
    let mut command = Command::new(payload.join(payload_relative_path));
    command.args(&arguments[1..]).current_dir(&directory);
    for (name, value) in effective.values() {
        if let Some(name) = name.strip_prefix("env.") {
            if let Some(value) = value.as_str() {
                command.env(name, value);
            }
        }
    }
    let mut paths = vec![payload.join(&metadata.archive.bin_relative_path)];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    command.env("PATH", std::env::join_paths(paths)?);
    let status = command.status()?;
    drop(lease);
    Ok(status.code().unwrap_or(1))
}
