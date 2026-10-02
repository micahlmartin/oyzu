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
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

const PIN: &str = "9290bcac695c8ff8a56760ccebd785d5062b459c";
const SOURCE: &str = "https://nodejs.org/dist/";

#[derive(Serialize, Deserialize)]
enum WorkerRequest {
    Hooks {
        shell: String,
        activate: bool,
    },
    Metadata(Request),
    RenderEnvironment {
        shell: String,
        original: BTreeMap<String, String>,
        desired: BTreeMap<String, String>,
        remove: Vec<String>,
    },
}

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
    let operation: WorkerRequest = serde_json::from_reader(std::io::stdin())?;
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
    let request = match operation {
        WorkerRequest::Hooks { shell, activate } => {
            let kind = shell_kind(&shell)?;
            let renderer = kind.as_shell();
            let mut settings = (*mise::config::Settings::get()).clone();
            settings.not_found_auto_install = false;
            mise::config::settings::store(Arc::new(settings));
            // Namespace adaptation occurs before inserting any user/path value.
            let template = if activate {
                renderer.activate(mise::shell::ActivateOptions {
                    exe: PathBuf::from("__OYZU_FRONTEND_TOKEN__"),
                    flags: String::new(),
                    no_hook_env: false,
                    prelude: vec![],
                })
            } else {
                renderer.deactivate()
            };
            let template = template
                .replace("mise", "oyzu")
                .replace("MISE", "OYZU")
                .replace("Mise", "Oyzu");
            let script = if activate {
                let prefix = renderer.set_env(
                    "OYZU_FRONTEND",
                    std::env::current_exe()?
                        .to_str()
                        .context("frontend path must be UTF-8")?,
                );
                let template = if shell == "pwsh" {
                    template.replace("'__OYZU_FRONTEND_TOKEN__'", "$env:OYZU_FRONTEND")
                } else {
                    template.replace("__OYZU_FRONTEND_TOKEN__", "\"${OYZU_FRONTEND}\"")
                };
                format!("{prefix}{template}")
            } else {
                template
            };
            serde_json::to_writer(std::io::stdout(), &script)?;
            return Ok(0);
        }
        WorkerRequest::Metadata(request) => request,
        WorkerRequest::RenderEnvironment {
            shell,
            original,
            desired,
            remove,
        } => {
            let renderer = shell_kind(&shell)?.as_shell();
            let diff = mise::env_diff::EnvDiff::new(&original, desired);
            let mut script = String::new();
            for patch in diff.to_patches() {
                use mise::env_diff::EnvDiffOperation;
                script.push_str(&match patch {
                    EnvDiffOperation::Add(key, value) | EnvDiffOperation::Change(key, value) => {
                        renderer.set_env(&key, &value)
                    }
                    EnvDiffOperation::Remove(key) => renderer.unset_env(&key),
                });
            }
            for key in remove {
                script.push_str(&renderer.unset_env(&key));
            }
            serde_json::to_writer(std::io::stdout(), &script)?;
            return Ok(0);
        }
    };
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
    worker_call(&WorkerRequest::Metadata(Request {
        request: request.request.clone(),
        exact: request.exact.clone(),
        target: request.target.clone(),
    }))
}

fn worker_call<T: serde::de::DeserializeOwned>(request: &WorkerRequest) -> Result<T> {
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
    ensure!(output.status.success(), "mise integration worker failed");
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
pub fn install(
    directory: &Path,
    options: &config::session::Options,
    store: &Path,
    frozen: bool,
    offline: bool,
    update: Option<&[String]>,
) -> Result<i32> {
    ensure!(
        update.is_none() || !(frozen || offline),
        "update conflicts with frozen/offline installation"
    );
    if let Some(tools) = update {
        ensure!(
            tools
                .iter()
                .all(|tool| matches!(tool.as_str(), "node" | "core:node")),
            "initial update supports node or core:node"
        );
    }
    let directory = directory.canonicalize()?;
    let (session, effective, request) = configuration(&directory, options)?;
    ensure!(
        directory == session.root,
        "initial Node install supports the workspace root scope"
    );
    let lock_path = session.root.join("oyzu.lock");
    ensure!(
        !(frozen || offline) || lock_path.exists(),
        "frozen/offline install requires an existing oyzu.lock; run oyzu install online first"
    );
    let edit = super::ToolLockEdit::capture(&lock_path)?;
    let captured = if lock_path.exists() {
        Some(lock::parse(&super::read_record(
            &lock_path,
            lock::MAX_BYTES,
        )?)?)
    } else {
        None
    };
    if let Some(lock) = &captured {
        ensure!(
            lock.tool.len() == 1 && lock.tool[0].id == "core:node",
            "existing lock is outside the initial Node integration scope"
        );
        if update.is_some() {
            ensure!(lock.environment.len() == 1 && lock.environment[0].scope == "." && lock.environment[0].profile == effective.profile.as_deref().unwrap_or("default"), "initial update requires a single matching root/profile; other selections are preserved by refusing this unsupported update");
            ensure!(lock.tool[0].distribution.len() == 1 && lock.tool[0].distribution[0].platform == platform()?, "initial update supports only the current host; use a future multi-platform update to preserve other target selections");
        }
    }
    let previous = captured.as_ref().filter(|_| update.is_none());
    let metadata = metadata(&Request {
        request,
        exact: previous.as_ref().map(|lock| lock.tool[0].version.clone()),
        target: platform()?.into(),
    })?;
    let requests =
        super::project_tool_requests(&effective, &metadata.aliases, &BTreeMap::new(), &[])?;
    let profile = effective.profile.as_deref().unwrap_or("default");
    let backend = identity()?;
    if let Some(lock) = &previous {
        let selection = super::select_for_tool_requests(
            &session.root,
            &directory,
            profile,
            &requests,
            platform()?,
        )
        .context("TOOL_LOCK_STALE: run oyzu install --update to resolve changed requirements")?;
        ensure!(
            lock.tool[0].backend_digest == backend,
            "locked backend differs; explicit relocking required"
        );
        let installation = &selection.installation_keys[&lock.tool[0].key];
        if store.join("installs").join(&installation[7..]).exists() {
            let _lease = super::lease_installation_selection(
                &lock_path,
                store,
                None,
                ".",
                profile,
                platform()?,
                &backend,
            )?;
            println!("Already installed node {}", metadata.archive.version);
            return Ok(0);
        }
    }
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
    std::fs::create_dir_all(store)?;
    let cached = if let Some(lock) = &previous {
        let host = platform()?;
        let distribution = lock.tool[0]
            .distribution
            .iter()
            .find(|d| d.platform == host)
            .context("locked host platform unavailable")?;
        ensure!(
            distribution.layout_digest == layout_digest,
            "locked layout differs from current adapter"
        );
        super::store::cached(&std::path::absolute(store)?, &digest, distribution.size)?
    } else {
        None
    };
    let acquired = if cached.is_none() {
        ensure!(
            !offline,
            "locked Node archive is not cached; run oyzu install online to acquire it"
        );
        let response = fetcher()?.fetch(&metadata.archive.archive_url)?;
        ensure!(
            response.status == 200,
            "Node archive acquisition returned {}",
            response.status
        );
        response.body
    } else {
        Vec::new()
    };
    let key = records::digest(
        "oyzu.tool-record.v2",
        &json!({"id":"core:node","version":metadata.archive.version,"backend_digest":backend,"options":{}}),
    )?;
    let bytes = if previous.is_some() {
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
                    size: acquired.len() as u64,
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
    let proposal = if frozen {
        None
    } else {
        Some(edit.propose(&bytes)?)
    };
    if update.is_some() {
        let before = captured
            .as_ref()
            .map(|lock| lock.tool[0].version.as_str())
            .unwrap_or("(unlocked)");
        println!("node: {before} -> {}", metadata.archive.version);
    }
    let parsed = lock::parse(&bytes)?;
    let distribution = &parsed.tool[0].distribution[0];
    ensure!(
        distribution.layout_digest == layout_digest,
        "locked layout differs from current adapter"
    );
    let blob = match cached {
        Some(blob) => blob,
        None => super::cache_tool_blob(
            store,
            &mut std::io::Cursor::new(acquired),
            &digest,
            distribution.size,
        )?,
    };
    // Publication renames candidates atomically, so staging must share the
    // installation store's filesystem (including externally mounted stores).
    let staging_root = store.join("staging");
    std::fs::create_dir_all(&staging_root)?;
    let staging = tempfile::tempdir_in(staging_root.canonicalize()?)?;
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
    if let Some(proposal) = proposal {
        proposal.commit()?;
    }
    println!("Installed node {}", metadata.archive.version);
    Ok(0)
}

pub(super) struct InstalledCommand {
    lease: super::InstallationLease,
    pub(super) executable: PathBuf,
    bin: PathBuf,
    pub(super) effective: config::resolve::EffectiveConfig,
}

// Shared frozen lookup for execution and executable discovery. The returned
// lease keeps the verified installation available for the caller's operation.
pub(super) fn installed_command(
    directory: &Path,
    options: &config::session::Options,
    store: &Path,
) -> Result<InstalledCommand> {
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
    let executable = payload.join(payload_relative_path);
    Ok(InstalledCommand {
        lease,
        executable,
        bin: payload.join(&metadata.archive.bin_relative_path),
        effective,
    })
}

/// Print the same verified executable selected by exec, without launching it.
pub fn which(
    directory: &Path,
    options: &config::session::Options,
    store: &Path,
    name: &str,
) -> Result<i32> {
    ensure!(name == "node", "initial which command must be node");
    let selected = installed_command(directory, options, store)?;
    println!("{}", selected.executable.display());
    Ok(0)
}

/// Execute the installed frozen Node command while holding its verified lease.
pub fn exec(
    directory: &Path,
    options: &config::session::Options,
    store: &Path,
    arguments: &[OsString],
) -> Result<i32> {
    ensure!(
        arguments.first().is_some_and(|argument| argument == "node"),
        "initial exec command must be node"
    );
    let selected = installed_command(directory, options, store)?;
    let mut command = Command::new(&selected.executable);
    command.args(&arguments[1..]).current_dir(directory);
    command.envs(command_environment(&selected)?);
    let status = command.status()?;
    drop(selected.lease);
    Ok(status.code().unwrap_or(1))
}

// Execution and shell output share one environment composition contract.
fn command_environment(selected: &InstalledCommand) -> Result<BTreeMap<String, OsString>> {
    compose_environment(selected, std::env::var_os("PATH").as_deref())
}

pub(super) fn compose_environment(
    selected: &InstalledCommand,
    path: Option<&std::ffi::OsStr>,
) -> Result<BTreeMap<String, OsString>> {
    let mut environment = BTreeMap::new();
    for (name, value) in selected.effective.values() {
        if let Some(name) = name.strip_prefix("env.") {
            if let Some(value) = value.as_str() {
                environment.insert(environment_key(name), OsString::from(value));
            }
        }
    }
    let mut paths = path
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    if paths.first() != Some(&selected.bin) {
        paths.insert(0, selected.bin.clone());
    }
    environment.insert("PATH".into(), std::env::join_paths(paths)?);
    Ok(environment)
}

/// Inspect the frozen environment with redacted values, or explicitly render
/// literal assignments through mise for applying in the named shell. No install
/// or network occurs. The lease protects lookup/rendering, not the caller shell.
pub fn environment(
    directory: &Path,
    options: &config::session::Options,
    store: &Path,
    shell: Option<&str>,
    json_output: bool,
) -> Result<i32> {
    ensure!(
        !(shell.is_some() && json_output),
        "--shell conflicts with --json"
    );
    let selected = installed_command(directory, options, store)?;
    let environment = command_environment(&selected)?;
    if let Some(shell) = shell {
        let desired = environment
            .into_iter()
            .map(|(name, value)| {
                Ok((
                    name,
                    value
                        .into_string()
                        .map_err(|_| anyhow::anyhow!("shell environment requires UTF-8 values"))?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let original = desired
            .keys()
            .filter_map(|key| std::env::var(key).ok().map(|value| (key.clone(), value)))
            .collect();
        let script: String = worker_call(&WorkerRequest::RenderEnvironment {
            shell: shell.into(),
            original,
            desired,
            remove: vec![],
        })?;
        print!("{script}");
    } else {
        let values: BTreeMap<_, _> = environment.keys().map(|key| (key, "<redacted>")).collect();
        if json_output {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &json!({"tool":"core:node", "executable":selected.executable, "environment":values})
                )?
            );
        } else {
            println!("node: {}", selected.executable.display());
            for (name, value) in values {
                println!("{name}={value}");
            }
        }
    }
    Ok(0)
}

fn shell_kind(shell: &str) -> Result<mise::shell::ShellType> {
    Ok(match shell {
        "bash" => mise::shell::ShellType::Bash,
        "zsh" => mise::shell::ShellType::Zsh,
        "pwsh" => mise::shell::ShellType::Pwsh,
        _ => anyhow::bail!("supported shells are bash, zsh and pwsh"),
    })
}

pub(super) fn environment_key(name: &str) -> String {
    if cfg!(windows) {
        name.to_ascii_uppercase()
    } else {
        name.to_owned()
    }
}

pub(super) fn hooks(shell: &str, activate: bool) -> Result<String> {
    worker_call(&WorkerRequest::Hooks {
        shell: shell.into(),
        activate,
    })
}

pub(super) fn render_changes(
    shell: &str,
    original: BTreeMap<String, String>,
    changes: BTreeMap<String, Option<String>>,
) -> Result<String> {
    let mut desired = BTreeMap::new();
    let mut remove = Vec::new();
    for (key, value) in changes {
        if let Some(value) = value {
            desired.insert(key, value);
        } else if original.contains_key(&key) {
            remove.push(key);
        }
    }
    worker_call(&WorkerRequest::RenderEnvironment {
        shell: shell.into(),
        original,
        desired,
        remove,
    })
}
