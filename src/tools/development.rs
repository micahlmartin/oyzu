//! Opt-in standalone core tool integration. Oyzu owns configuration, locks and storage;
//! a fresh same-image child owns mise globals. Initial development transport uses
//! child stdio; authenticated worker IPC and process hardening are future work.
use super::{acquisition::Acquisition, development_backend::Tool};
use super::{lock, ToolCandidateRequest, ToolLaunch};
use crate::{config, records};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

const PIN: &str = "9290bcac695c8ff8a56760ccebd785d5062b459c";
#[derive(Serialize, Deserialize)]
enum WorkerReply {
    Fetch(String),
    Complete(serde_json::Value),
}
#[derive(Serialize, Deserialize)]
enum TransportReply {
    Response {
        status: u16,
        content_type: String,
        body: Vec<u8>,
    },
    Error(String),
}

fn write_line(writer: &mut dyn Write, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}
fn read_line<T: serde::de::DeserializeOwned>(reader: &mut dyn BufRead) -> Result<T> {
    let mut line = String::new();
    ensure!(
        reader.read_line(&mut line)? != 0,
        "integration channel closed before response"
    );
    Ok(serde_json::from_str(&line)?)
}
fn complete(value: &impl Serialize) -> Result<()> {
    write_line(
        &mut std::io::stdout(),
        &WorkerReply::Complete(serde_json::to_value(value)?),
    )
}

#[derive(Serialize, Deserialize)]
enum WorkerRequest {
    Aliases,
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
    tool: Tool,
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
    #[serde(alias = "node_relative_path", alias = "go_relative_path")]
    executable_relative_path: String,
    bin_relative_path: String,
}
#[derive(Serialize, Deserialize)]
struct Metadata {
    archive: Archive,
    declared_sha256: Option<String>,
    declared_size: Option<u64>,
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
    let operation: WorkerRequest = read_line(&mut std::io::stdin().lock())?;
    let state = tempfile::tempdir()?;
    let exchange = Arc::new(std::sync::Mutex::new(()));
    let transport: Arc<mise::embedding::HttpTransport> = Arc::new(move |request| {
        let exchange = exchange.clone();
        Box::pin(async move {
            if request.method.as_str() != "GET" {
                return Err(std::io::Error::other("metadata requires GET").into());
            }
            let url = request.url.to_string();
            let response = tokio::task::spawn_blocking(move || -> Result<TransportReply> {
                let _guard = exchange
                    .lock()
                    .map_err(|_| anyhow::anyhow!("metadata exchange unavailable"))?;
                write_line(&mut std::io::stdout(), &WorkerReply::Fetch(url))?;
                read_line(&mut std::io::stdin().lock())
            })
            .await?
            .map_err(|error| std::io::Error::other(error.to_string()))?;
            let TransportReply::Response {
                status,
                content_type,
                body,
            } = response
            else {
                let TransportReply::Error(message) = response else {
                    unreachable!()
                };
                return Err(std::io::Error::other(message).into());
            };
            let response = http::Response::builder()
                .status(status)
                .header("content-type", content_type)
                .body(body)?;
            Ok(reqwest_mise::Response::from(response))
        })
    });
    let session = mise::embedding::Session::initialize(mise::embedding::Options {
        state: state.path().canonicalize()?,
        frontend: std::env::current_exe()?,
        tools: ["node".to_owned(), "go".to_owned()].into(),
        transport: Some(transport),
    })
    .map_err(|error| anyhow::anyhow!("{error:#}"))?;
    let request = match operation {
        WorkerRequest::Aliases => {
            complete(
                &session
                    .tool_aliases()
                    .map_err(|error| anyhow::anyhow!("{error:#}"))?,
            )?;
            return Ok(0);
        }
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
            complete(&script)?;
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
            complete(&script)?;
            return Ok(0);
        }
    };
    let result = if let Some(version) = request.exact {
        let archive = match request.tool {
            Tool::Node => serde_json::to_value(
                session
                    .node_archive_facts(&version, &request.target)
                    .map_err(|error| anyhow::anyhow!("{error:#}"))?,
            )?,
            Tool::Go => serde_json::to_value(
                session
                    .go_archive_facts(&version, &request.target)
                    .map_err(|error| anyhow::anyhow!("{error:#}"))?,
            )?,
        };
        Metadata {
            archive: serde_json::from_value(archive)?,
            declared_sha256: None,
            declared_size: None,
        }
    } else {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let metadata: serde_json::Value = runtime.block_on(async {
            match request.tool {
                Tool::Node => {
                    let version = session
                        .resolve_node_version(&request.request, &[])
                        .await
                        .map_err(|error| anyhow::anyhow!("{error:#}"))?;
                    Ok::<_, anyhow::Error>(serde_json::to_value(
                        session
                            .node_archive_metadata(&version, &request.target)
                            .await
                            .map_err(|error| anyhow::anyhow!("{error:#}"))?,
                    )?)
                }
                Tool::Go => {
                    let version = session
                        .resolve_go_version(&request.request, &[])
                        .await
                        .map_err(|error| anyhow::anyhow!("{error:#}"))?;
                    Ok(serde_json::to_value(
                        session
                            .go_archive_metadata(&version.version, &request.target)
                            .await
                            .map_err(|error| anyhow::anyhow!("{error:#}"))?,
                    )?)
                }
            }
        })?;
        Metadata {
            archive: serde_json::from_value(metadata["archive"].clone())?,
            declared_sha256: metadata["declared_sha256"].as_str().map(str::to_owned),
            declared_size: metadata["declared_size"].as_u64(),
        }
    };
    complete(&result)?;
    Ok(0)
}

fn metadata(request: &Request, acquisition: Option<&mut Acquisition>) -> Result<Metadata> {
    worker_call(
        &WorkerRequest::Metadata(Request {
            tool: request.tool,
            request: request.request.clone(),
            exact: request.exact.clone(),
            target: request.target.clone(),
        }),
        acquisition,
    )
}

fn worker_call<T: serde::de::DeserializeOwned>(
    request: &WorkerRequest,
    mut acquisition: Option<&mut Acquisition>,
) -> Result<T> {
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
    let mut input = child.stdin.take().context("worker input unavailable")?;
    let mut output =
        std::io::BufReader::new(child.stdout.take().context("worker output unavailable")?);
    let result = (|| -> Result<T> {
        write_line(&mut input, request)?;
        loop {
            match read_line(&mut output)? {
                WorkerReply::Complete(value) => return Ok(serde_json::from_value(value)?),
                WorkerReply::Fetch(url) => {
                    let response = acquisition
                        .as_deref_mut()
                        .context("network is unavailable for this operation")
                        .and_then(|acquisition| acquisition.fetch(&url));
                    let reply = match response {
                        Ok(response) => TransportReply::Response {
                            status: response.status,
                            content_type: response.content_type,
                            body: response.body,
                        },
                        Err(error) => TransportReply::Error(error.to_string()),
                    };
                    write_line(&mut input, &reply)?;
                }
            }
        }
    })();
    drop(input);
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait()?;
    ensure!(status.success(), "mise integration worker failed");
    result
}

struct Configuration {
    session: config::session::Session,
    effective: config::resolve::EffectiveConfig,
    tools: Vec<(Tool, String)>,
}

fn configuration(directory: &Path, options: &config::session::Options) -> Result<Configuration> {
    let session = config::session::Session::open(directory, options)?;
    let effective = session.resolve(directory, false)?;
    ensure!(
        effective.management.is_none(),
        "managed tool integration is not available in this standalone development proof"
    );
    let mut configured = Vec::new();
    for (key, value) in effective.values() {
        let Some(name) = key.strip_prefix("tools.") else {
            continue;
        };
        let tool = match name {
            "allowed" | "catalogs" => continue,
            "node" => Tool::Node,
            "go" => Tool::Go,
            _ => anyhow::bail!("development integration supports tools.node and tools.go"),
        };
        Acquisition::new(&effective, None, tool)?;
        configured.push((
            tool,
            value
                .as_str()
                .context("tool request must be a string")?
                .to_owned(),
        ));
    }
    ensure!(!configured.is_empty(), "configure Node or Go in Oyzu TOML");
    let names: Vec<_> = configured.iter().map(|(tool, _)| tool.name()).collect();
    config::enforcement::tool_eligibility(&effective, &names)?;
    Ok(Configuration {
        session,
        effective,
        tools: configured,
    })
}

fn plan(tool: Tool, archive: &Archive, digest: &str, backend: &str) -> serde_json::Value {
    json!({"format":1,"backend_digest":backend,"platform":archive.target,
        "input_blob_digests":[digest],"archive_kind":archive.archive_kind,"strip_prefix":archive.strip_prefix,"payload_subtree":".",
        "required_paths":[{"path":archive.executable_relative_path,"kind":"file"}],
        "entrypoints":{tool.name():{"kind":"native","payload_relative_path":archive.executable_relative_path,"interpreter_tool_key":null,"interpreter_relative_path":null,"prefix_args":[]}},
        "environment":{"PATH":{"kind":"paths","paths":[{"owner":"self","relative_path":archive.bin_relative_path}]}},
        "extraction_bounds":{"max_entries":200000,"max_bytes":8589934592u64,"max_file_bytes":1073741824,"max_depth":64,"max_expansion_ratio":200},
        "executable_paths":if cfg!(windows) { Vec::<String>::new() } else { vec![archive.executable_relative_path.clone()] }})
}

mod installation;
pub use installation::install;

pub(super) struct InstalledEnvironment {
    lease: super::InstallationLease,
    pub(super) executables: BTreeMap<String, PathBuf>,
    bins: Vec<(Tool, PathBuf)>,
    pub(super) effective: config::resolve::EffectiveConfig,
}

// Shared frozen lookup. One lease covers all configured tools for the operation.
pub(super) fn installed_environment(
    directory: &Path,
    options: &config::session::Options,
    store: &Path,
) -> Result<InstalledEnvironment> {
    let directory = directory.canonicalize()?;
    let Configuration {
        session,
        effective,
        tools: configured,
    } = configuration(&directory, options)?;
    let aliases: BTreeMap<String, String> = worker_call(&WorkerRequest::Aliases, None)?;
    let requests = super::project_tool_requests(&effective, &aliases, &BTreeMap::new(), &[])?;
    let profile = effective.profile.as_deref().unwrap_or("default");
    let selection = super::select_for_tool_requests(
        &session.root,
        &directory,
        profile,
        &requests,
        platform()?,
    )?;
    let lease = super::lease_installation_selection(
        &session.root.join("oyzu.lock"),
        store,
        None,
        &selection.scope,
        profile,
        platform()?,
        &identity()?,
    )?;
    let mut executables = BTreeMap::new();
    let mut bins = Vec::new();
    for (tool, _) in configured {
        let selected = lease.command(tool.name())?;
        let ToolLaunch::Native {
            payload_relative_path,
            prefix_args,
        } = selected.launch
        else {
            anyhow::bail!("core tool requires a native launch descriptor");
        };
        ensure!(
            prefix_args.is_empty(),
            "unexpected core tool prefix arguments"
        );
        let payload = std::path::absolute(store)?
            .join("installs")
            .join(&selected.installation_key[7..])
            .join("payload");
        let executable = payload.join(payload_relative_path);
        // Both admitted core backends place the primary command in their bin
        // directory (the payload root for Node on Windows).
        bins.push((
            tool,
            executable
                .parent()
                .context("tool bin directory unavailable")?
                .to_path_buf(),
        ));
        executables.insert(tool.name().into(), executable);
    }
    Ok(InstalledEnvironment {
        lease,
        executables,
        bins,
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
    let selected = installed_environment(directory, options, store)?;
    let executable = selected
        .executables
        .get(name)
        .context("command is not in the selected closure")?;
    println!("{}", executable.display());
    Ok(0)
}

/// Execute a selected core tool command or an explicitly requested executable path
/// with the frozen environment and lease. Bare unknown commands never use PATH.
pub fn exec(
    directory: &Path,
    options: &config::session::Options,
    store: &Path,
    arguments: &[OsString],
) -> Result<i32> {
    let requested = arguments.first().context("exec requires a command")?;
    let selected = installed_environment(directory, options, store)?;
    let executable = if let Some(executable) = requested
        .to_str()
        .and_then(|name| selected.executables.get(name))
    {
        executable.clone()
    } else {
        let path = Path::new(requested);
        ensure!(
            path.is_absolute() || path.components().count() > 1,
            "unknown bare command; use a declared tool command or an explicit executable path"
        );
        #[cfg(windows)]
        ensure!(
            path.is_absolute()
                || !matches!(
                    path.components().next(),
                    Some(std::path::Component::Prefix(_))
                ),
            "drive-relative executable paths are ambiguous; use an absolute path"
        );
        // Resolve against -C before creating the child; Command's relative
        // executable/current_dir interaction differs between operating systems.
        let path = directory
            .join(path)
            .canonicalize()
            .context("explicit executable path is unavailable")?;
        ensure!(path.is_file(), "explicit executable path must be a file");
        path
    };
    let mut command = Command::new(executable);
    command.args(&arguments[1..]).current_dir(directory);
    command.envs(command_environment(&selected)?);
    let status = command.status()?;
    drop(selected.lease);
    Ok(status.code().unwrap_or(1))
}

// Execution and shell output share one environment composition contract.
fn command_environment(selected: &InstalledEnvironment) -> Result<BTreeMap<String, OsString>> {
    compose_environment(selected, std::env::var_os("PATH").as_deref())
}

pub(super) fn compose_environment(
    selected: &InstalledEnvironment,
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
    for (_, bin) in selected.bins.iter().rev() {
        if paths.first() != Some(bin) {
            paths.insert(0, bin.clone());
        }
    }
    environment.insert("PATH".into(), std::env::join_paths(paths)?);
    for (tool, bin) in &selected.bins {
        tool.apply_environment(bin, &mut environment)?;
    }
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
    let selected = installed_environment(directory, options, store)?;
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
        let script: String = worker_call(
            &WorkerRequest::RenderEnvironment {
                shell: shell.into(),
                original,
                desired,
                remove: vec![],
            },
            None,
        )?;
        print!("{script}");
    } else {
        let values: BTreeMap<_, _> = environment.keys().map(|key| (key, "<redacted>")).collect();
        if json_output {
            let tools: BTreeMap<_, _> = selected
                .bins
                .iter()
                .map(|(tool, _)| (tool.id(), &selected.executables[tool.name()]))
                .collect();
            let mut output =
                json!({"tools":tools, "executables":selected.executables, "environment":values});
            if let [(tool, _)] = selected.bins.as_slice() {
                output["tool"] = json!(tool.id());
                output["executable"] = json!(selected.executables[tool.name()]);
            }
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            for (name, executable) in &selected.executables {
                println!("{name}: {}", executable.display());
            }
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
    worker_call(
        &WorkerRequest::Hooks {
            shell: shell.into(),
            activate,
        },
        None,
    )
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
    worker_call(
        &WorkerRequest::RenderEnvironment {
            shell: shell.into(),
            original,
            desired,
            remove,
        },
        None,
    )
}
