//! Independent experimental frontend. Upstream behavior is linked, not copied.
use std::{collections::BTreeMap, env, fs, path::PathBuf, process::Command, sync::Arc};

use eyre::{Result, bail, ensure};
use mise::{
    args::BackendArg,
    config::{Config, Settings},
    env_diff::{EnvDiff, EnvDiffOperation, EnvMap},
    install_context::InstallContext,
    lockfile::PlatformInfo,
    shell::{ActivateOptions, ShellType},
    toolset::{ResolveOptions, ToolRequest, ToolSource, ToolVersion, Toolset},
};
use serde::{Deserialize, Serialize};

const PIN: &str = "da0db43e9398b46bafa95232014708a51120e731";

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {
    #[serde(default)]
    tools: BTreeMap<String, String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Lock {
    format: u32,
    tool: Vec<LockedTool>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LockedTool {
    id: String,
    request: String,
    version: String,
    backend_digest: String,
    distribution: Vec<Distribution>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Distribution {
    platform: String,
    digest: String,
    size: u64,
    source_id: String,
    verification: String,
    dependencies: Vec<String>,
}

#[derive(Debug)]
struct Quiet;
impl mise::ui::progress_report::SingleReport for Quiet {}

#[allow(clippy::disallowed_methods)] // Process exit is confined to the outermost boundary.
fn main() {
    // Recover native launcher arguments before discarding ambient mise settings.
    let args = mise::env::args_safe();
    *mise::env::ARGS.write().unwrap() = args.clone();
    // Initialize process-wide upstream settings before starting runtime threads.
    // This is intentionally a CLI process, not yet a multi-workspace daemon.
    let result = initialize().and_then(|()| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?
            .block_on(run(args))
    });
    let status = match result {
        Ok(status) => status,
        Err(error) => {
            eprintln!("oyzu experiment: {error:#}");
            1
        }
    };
    std::process::exit(status);
}

fn initialize() -> Result<()> {
    let state = PathBuf::from(env::var("OYZU_SPIKE_STATE")?);
    ensure!(state.is_absolute(), "state directory must be absolute");
    fs::create_dir_all(&state)?;
    let diff = env::var("OYZU_SPIKE_DIFF").ok();
    for (key, _) in env::vars_os() {
        let name = key.to_string_lossy().to_ascii_uppercase();
        if name.starts_with("MISE_") || name.starts_with("__MISE_") {
            // SAFETY: called before this program starts any threads.
            unsafe { env::remove_var(key) };
        }
    }
    for (key, suffix) in [
        ("MISE_DATA_DIR", "data"),
        ("MISE_CACHE_DIR", "cache"),
        ("MISE_STATE_DIR", "state"),
        ("MISE_CONFIG_DIR", "config"),
        ("MISE_SYSTEM_CONFIG_DIR", "system-config"),
        ("MISE_SYSTEM_DATA_DIR", "system-data"),
    ] {
        unsafe { env::set_var(key, state.join(suffix)) };
    }
    if let Some(diff) = diff {
        unsafe { env::set_var("__MISE_DIFF", diff) };
    }
    mise::config::settings::set_loader(mise::config::settings::load_defaults);
    let mut settings = (*mise::config::settings::load_defaults()?).clone();
    settings.node.compile = Some(false);
    settings.node.gpg_verify = Some(false); // Experiment uses independently locked SHA-256.
    settings.node.corepack = false;
    settings.node.npm_shim = false;
    settings.node.default_packages_file = Some(state.join("no-default-packages"));
    settings.node.mirror_url = env::var("OYZU_SPIKE_ROUTE")
        .ok()
        .map(|route| format!("{}/", route.trim_end_matches('/')));
    mise::config::settings::store(Arc::new(settings));
    mise::register_util_hooks();
    mise::frontend::register(mise::frontend::Frontend {
        lockfiles_after_install: |_, _| Box::pin(async { Ok(()) }),
        subcommand_names: || vec!["exec".into(), "hook-env".into(), "install".into()],
    });
    Ok(())
}

fn project() -> Result<Option<(PathBuf, Project)>> {
    let cwd = env::current_dir()?;
    for directory in cwd.ancestors() {
        let file = directory.join("oyzu.toml");
        if file.exists() {
            return Ok(Some((
                directory.into(),
                toml::from_str(&fs::read_to_string(file)?)?,
            )));
        }
    }
    Ok(None)
}

fn shell(args: &[String]) -> ShellType {
    match args
        .windows(2)
        .find(|a| a[0] == "-s")
        .map(|a| a[1].as_str())
    {
        Some("pwsh" | "powershell") => ShellType::Pwsh,
        Some("zsh") => ShellType::Zsh,
        _ => ShellType::Bash,
    }
}

async fn selected(config: &Arc<Config>, install: bool) -> Result<(Toolset, EnvMap)> {
    let mut ts = Toolset::new(ToolSource::Argument);
    let Some((directory, project)) = project()? else {
        return Ok((ts, EnvMap::new()));
    };
    ensure!(
        project.tools.len() == 1,
        "experiment requires one explicit Node tool"
    );
    let request = project
        .tools
        .get("node")
        .ok_or_else(|| eyre::eyre!("unsupported tool"))?;
    let lock: Lock = toml::from_str(&fs::read_to_string(directory.join("oyzu.lock"))?)?;
    ensure!(
        lock.format == 1 && lock.tool.len() == 1,
        "unsupported lock format or tool count"
    );
    let locked = &lock.tool[0];
    ensure!(
        locked.id == "core:node" && locked.request == *request,
        "lock/config identity mismatch"
    );
    ensure!(
        locked.backend_digest == format!("git:{PIN}"),
        "backend pin mismatch"
    );
    ensure!(
        env::var("OYZU_SPIKE_DENY_VERSION").ok().as_ref() != Some(&locked.version),
        "policy denies locked version, including cached installs"
    );
    let ba = Arc::new(BackendArg::new("node".into(), Some("core:node".into())));
    let tr = ToolRequest::new(ba, &locked.version, ToolSource::Argument)?;
    let backend = tr.backend()?;
    let platform = backend.get_platform_key();
    let distribution = locked
        .distribution
        .iter()
        .find(|d| d.platform == platform)
        .ok_or_else(|| eyre::eyre!("lock has no distribution for {platform}"))?;
    ensure!(
        distribution.source_id == "node-fixture" && distribution.dependencies.is_empty(),
        "unsupported acquisition route/dependencies"
    );
    ensure!(
        distribution.verification == "fixture-sha256" && distribution.size > 0,
        "missing verification evidence"
    );
    let digest = distribution
        .digest
        .strip_prefix("sha256:")
        .ok_or_else(|| eyre::eyre!("digest algorithm"))?;
    ensure!(
        digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()),
        "invalid digest"
    );
    let mut tv = ToolVersion::new(tr.clone(), locked.version.clone());
    let os = if cfg!(windows) {
        "win"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
    if install {
        let route = env::var("OYZU_SPIKE_ROUTE")?;
        tv.lock_platforms.insert(
            platform,
            PlatformInfo {
                url: Some(format!(
                    "{}/v{v}/node-v{v}-{os}-{arch}.{ext}",
                    route.trim_end_matches('/'),
                    v = locked.version
                )),
                checksum: Some(distribution.digest.clone()),
                size: Some(distribution.size),
                ..Default::default()
            },
        );
        let context = InstallContext {
            config: config.clone(),
            ts: Arc::new(ts.clone()),
            pr: Arc::new(Quiet),
            force: false,
            dry_run: false,
            explicit_yes: true,
            locked: true,
            before_date: None,
            dependency_context: Default::default(),
        };
        tv = backend.install_version(context, tv).await?;
    } else {
        ensure!(
            backend.is_version_installed(config, &tv, true),
            "locked tool is not installed; activation does not install"
        );
    }
    ts.add_version(tr);
    for list in ts.versions.values_mut() {
        list.versions = vec![tv.clone()];
    }
    Ok((ts, project.env))
}

async fn run(raw_args: Vec<String>) -> Result<i32> {
    let argv0 = PathBuf::from(&raw_args[0]);
    let mut args: Vec<String> = raw_args.into_iter().skip(1).collect();
    let invoked = argv0.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if invoked == "node" {
        args.splice(0..0, ["exec".into(), "--".into(), "node".into()]);
    }
    let command = args.first().map(String::as_str).unwrap_or("inspect");
    let sh = shell(&args).as_shell();
    if command == "activate" {
        print!(
            "{}",
            sh.activate(ActivateOptions {
                exe: env::current_exe()?,
                flags: String::new(),
                no_hook_env: false,
                prelude: vec![],
            })
        );
        return Ok(0);
    }
    // The experiment deliberately requires an empty configuration constructor;
    // the unmodified-source build records whether this boundary is available.
    mise::backend::load_tools().await?;
    let config = Config::oyzu_experiment_empty().await?;
    if command == "resolve" || command == "lock" {
        let (directory, project) = project()?.ok_or_else(|| eyre::eyre!("missing oyzu.toml"))?;
        let request = project
            .tools
            .get("node")
            .ok_or_else(|| eyre::eyre!("missing node request"))?;
        let tr = ToolRequest::new(
            Arc::new(BackendArg::new("node".into(), Some("core:node".into()))),
            request,
            ToolSource::Argument,
        )?;
        let tv = tr
            .resolve(
                &config,
                &ResolveOptions {
                    use_locked_version: false,
                    latest_versions: true,
                    ..Default::default()
                },
            )
            .await?;
        if command == "lock" {
            ensure!(
                !directory.join("oyzu.lock").exists(),
                "explicit lock update is outside this spike; refusing overwrite"
            );
            let backend = tv.backend()?;
            let target = mise::backend::platform_target::PlatformTarget::from_current();
            let metadata = backend.resolve_lock_info(&tv, &target).await?;
            let digest = metadata
                .checksum
                .ok_or_else(|| eyre::eyre!("backend provided no integrity evidence"))?;
            let url = metadata
                .url
                .ok_or_else(|| eyre::eyre!("backend provided no distribution URL"))?;
            let size = mise::http::HTTP
                .head(&url)
                .await?
                .headers()
                .get("content-length")
                .ok_or_else(|| eyre::eyre!("missing archive size"))?
                .to_str()?
                .parse()?;
            let lock = Lock {
                format: 1,
                tool: vec![LockedTool {
                    id: "core:node".into(),
                    request: request.clone(),
                    version: tv.version.clone(),
                    backend_digest: format!("git:{PIN}"),
                    distribution: vec![Distribution {
                        platform: target.to_key(),
                        digest,
                        size,
                        source_id: "node-fixture".into(),
                        verification: "fixture-sha256".into(),
                        dependencies: vec![],
                    }],
                }],
            };
            let mut staged = tempfile::NamedTempFile::new_in(&directory)?;
            use std::io::Write;
            staged.write_all(toml::to_string_pretty(&lock)?.as_bytes())?;
            staged.as_file().sync_all()?;
            staged.persist_noclobber(directory.join("oyzu.lock"))?;
        }
        println!("{}", tv.version);
        return Ok(0);
    }
    let (ts, additions) = selected(&config, command == "install").await?;
    if command == "benchmark" {
        let mut core_us = Vec::new();
        let mut adapter_us = Vec::new();
        for _ in 0..50 {
            let start = std::time::Instant::now();
            let mut environment = mise::env::PRISTINE_ENV.clone();
            environment.extend(ts.env_with_path(&config).await?);
            environment.extend(additions.clone());
            std::hint::black_box(
                EnvDiff::from_final_env(&mise::env::PRISTINE_ENV, &environment).serialize()?,
            );
            core_us.push(start.elapsed().as_micros());
            let start = std::time::Instant::now();
            let (selected_ts, selected_env) = selected(&config, false).await?;
            let mut environment = mise::env::PRISTINE_ENV.clone();
            environment.extend(selected_ts.env_with_path(&config).await?);
            environment.extend(selected_env);
            std::hint::black_box(
                EnvDiff::from_final_env(&mise::env::PRISTINE_ENV, &environment).serialize()?,
            );
            adapter_us.push(start.elapsed().as_micros());
        }
        core_us.sort();
        adapter_us.sort();
        println!(
            "{}",
            serde_json::json!({
                "samples": 50,
                "core_p50_us": core_us[25], "core_p95_us": core_us[47],
                "adapter_p50_us": adapter_us[25], "adapter_p95_us": adapter_us[47],
                "scope": "same process and installed toolchain: upstream environment/diff versus those calls plus Oyzu TOML/lock adapter; no standalone mise executable"
            })
        );
        return Ok(0);
    }
    let mut final_env = mise::env::PRISTINE_ENV.clone();
    if command == "hook-env" {
        let (environment, _, user_paths, tool_paths, _) =
            ts.env_with_path_and_split(&config).await?;
        final_env.extend(environment);
        let mut paths = mise::path_env::PathEnv::from_iter(mise::env::PATH.iter().cloned());
        for path in user_paths.into_iter().chain(tool_paths) {
            paths.add(path);
        }
        final_env.insert(
            mise::env::PATH_KEY.to_string(),
            paths.join_verbatim().to_string_lossy().into_owned(),
        );
    } else {
        final_env.extend(ts.env_with_path(&config).await?);
    }
    final_env.extend(additions);
    if command == "hook-env" {
        let current: EnvMap = mise::env::vars_safe()
            .map(|(k, v)| (mise::env::normalize_path_key(k), v))
            .collect();
        let diff = EnvDiff::from_final_env(&mise::env::PRISTINE_ENV, &final_env);
        let mut patches = EnvDiff::new(&current, final_env.clone()).to_patches();
        for key in mise::env::__MISE_DIFF.new.keys() {
            if !final_env.contains_key(key) && current.contains_key(key) {
                patches.push(EnvDiffOperation::Remove(key.clone()));
            }
        }
        patches.push(EnvDiffOperation::Add(
            "OYZU_SPIKE_DIFF".into(),
            diff.serialize()?,
        ));
        print!(
            "{}",
            mise::hook_env::build_env_commands(sh.as_ref(), &patches)
        );
    } else if command == "exec" || command == "x" {
        let start = args
            .iter()
            .position(|a| a == "--")
            .map(|i| i + 1)
            .unwrap_or(1);
        ensure!(args.len() > start, "exec requires a command");
        let bin = &args[start];
        let resolved = ts.which(&config, bin).await;
        let path =
            resolved.ok_or_else(|| eyre::eyre!("executable not in locked toolchain: {bin}"))?;
        let executable = path
            .0
            .which(&config, &path.1, bin)
            .await?
            .ok_or_else(|| eyre::eyre!("missing executable"))?;
        let mut child = Command::new(executable);
        child.args(&args[start + 1..]).env_clear().envs(final_env);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            return Err(child.exec().into());
        }
        #[cfg(windows)]
        return Ok(child.status()?.code().unwrap_or(1));
    } else if command == "reshim" {
        mise::shims::reshim_for(&config, &ts, false, mise::shims::ShimScope::User).await?;
    } else if command == "inspect" || command == "install" {
        println!(
            "{}",
            serde_json::json!({"upstream": PIN, "versions": ts.list_current_versions().iter().map(|(_,v)| &v.version).collect::<Vec<_>>(), "paths": ts.list_paths(&config).await, "platform": mise::platform::Platform::current().to_key(), "settings_mirror": Settings::get().node.mirror_url})
        );
    } else {
        bail!("unsupported experimental command: {command}");
    }
    Ok(0)
}
