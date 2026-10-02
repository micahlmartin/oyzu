mod config_args;
mod presentation;
use anyhow::Result;
use clap::{Parser, Subcommand};
use oyzu::{build, config, discovery, tasks};
use std::{io::IsTerminal, path::PathBuf};

#[derive(Parser)]
#[command(version, about = "Opinionated native project tasks and builds")]
struct Cli {
    #[arg(short = 'C', long = "directory", global = true)]
    directory: Option<PathBuf>,
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[arg(long, global = true, conflicts_with = "no_profile")]
    profile: Option<String>,
    #[arg(long, global = true)]
    no_profile: bool,
    #[arg(long, global = true)]
    local_overrides: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Emit upstream-rendered Oyzu shell activation hooks.
    #[cfg(feature = "mise-integration")]
    Activate {
        #[arg(value_parser = ["bash", "zsh", "pwsh"])]
        shell: String,
        #[arg(long)]
        store: Option<PathBuf>,
    },
    /// Restore this session's environment and remove its hooks.
    #[cfg(feature = "mise-integration")]
    Deactivate { shell: Option<String> },
    #[cfg(feature = "mise-integration")]
    #[command(hide = true)]
    HookEnv {
        #[arg(short = 's', long)]
        shell: String,
        #[arg(long)]
        shell_pid: Option<u32>,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        force: bool,
    },
    /// Inspect the frozen environment or render literal shell assignments.
    #[cfg(feature = "mise-integration")]
    Env {
        #[arg(long, default_value = ".oyzu/tools")]
        store: PathBuf,
        #[arg(long, value_parser = ["bash", "zsh", "pwsh"])]
        shell: Option<String>,
    },
    /// Install the configured Node tool (opt-in development integration).
    #[cfg(feature = "mise-integration")]
    Install {
        /// Require an existing matching lock without resolving new versions.
        #[arg(long)]
        frozen: bool,
        /// Use only existing locks, installations and cached archives.
        #[arg(long)]
        offline: bool,
        /// Resolve configured requirements again (currently node/core:node).
        #[arg(long, num_args = 0.., conflicts_with_all = ["frozen", "offline"])]
        update: Option<Vec<String>>,
        #[arg(long, default_value = ".oyzu/tools")]
        store: PathBuf,
    },
    /// Locate a verified installed command using the frozen project selection.
    #[cfg(feature = "mise-integration")]
    Which {
        #[arg(long, default_value = ".oyzu/tools")]
        store: PathBuf,
        command: String,
    },
    /// Execute a frozen installed Node command (opt-in development integration).
    #[cfg(feature = "mise-integration")]
    Exec {
        #[arg(long, default_value = ".oyzu/tools")]
        store: PathBuf,
        #[arg(last = true, required = true)]
        args: Vec<std::ffi::OsString>,
    },
    /// Inspect tool-management records without installing or executing tools.
    Tools {
        #[command(subcommand)]
        command: ToolCommand,
    },
    /// Inspect, validate or edit cascading configuration without executing tasks.
    Config {
        #[command(subcommand)]
        command: config_args::Command,
    },
    /// List inferred tasks or execute a task in the development environment.
    Run {
        task: Option<String>,
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Inspect static builder and native task discovery without executing project code.
    Discover,
    /// Build captured source with a provisioned container toolchain.
    Build {
        /// Emit the resolved deterministic plan without executing it.
        #[arg(long)]
        plan: bool,
        /// Select a provisioned toolchain image, e.g. npm=node:22-bookworm-slim.
        #[arg(long)]
        image: Vec<String>,
    },
    /// Verify recorded content in a build bundle or exported OCI layout tar.
    Inspect { bundle: PathBuf },
}

#[derive(Subcommand)]
enum ToolCommand {
    /// Validate a backend descriptor identity without admitting its source.
    #[command(name = "inspect-backend")]
    Backend { path: PathBuf },
    /// Observe a payload tree without following links or trusting a receipt.
    #[command(name = "inspect-tree")]
    Tree { path: PathBuf },
    /// Validate format-2 lock structure and identity (not trust or installed state).
    #[command(name = "inspect-lock")]
    Lock {
        #[arg(default_value = "oyzu.lock")]
        path: PathBuf,
    },
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("oyzu: {error:#}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<i32> {
    #[cfg(feature = "mise-integration")]
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "__oyzu-node-worker")
    {
        return oyzu::tools::development::worker();
    }
    #[cfg(feature = "mise-integration")]
    if let Some(code) = oyzu::tools::shims::dispatch()? {
        return Ok(code);
    }
    let cli = Cli::parse();
    let directory = cli
        .directory
        .as_ref()
        .or(cli.root.as_ref())
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    let options = config::session::Options {
        root: cli.root.clone(),
        profile: cli.profile.clone(),
        no_profile: cli.no_profile,
        local_overrides: cli.local_overrides,
        ..Default::default()
    };
    match &cli.command {
        #[cfg(feature = "mise-integration")]
        Commands::Activate { shell, store } => {
            anyhow::ensure!(!cli.json, "activation emits shell code, not JSON");
            return oyzu::tools::shell::activate(&directory, &options, store.as_deref(), shell);
        }
        #[cfg(feature = "mise-integration")]
        Commands::Deactivate { shell } => {
            anyhow::ensure!(!cli.json, "deactivation emits shell code, not JSON");
            let shell = shell
                .clone()
                .or_else(|| std::env::var("OYZU_SHELL").ok())
                .ok_or_else(|| anyhow::anyhow!("specify the active shell"))?;
            return oyzu::tools::shell::transition(&directory, &shell, true);
        }
        #[cfg(feature = "mise-integration")]
        Commands::HookEnv { shell, .. } => {
            return oyzu::tools::shell::transition(&directory, shell, false)
        }
        #[cfg(feature = "mise-integration")]
        Commands::Env { store, shell } => {
            return oyzu::tools::development::environment(
                &directory,
                &options,
                &directory.join(store),
                shell.as_deref(),
                cli.json,
            );
        }
        #[cfg(feature = "mise-integration")]
        Commands::Install {
            store,
            frozen,
            offline,
            update,
        } => {
            return oyzu::tools::development::install(
                &directory,
                &options,
                &directory.join(store),
                *frozen,
                *offline,
                update.as_deref(),
            );
        }
        #[cfg(feature = "mise-integration")]
        Commands::Which { store, command } => {
            return oyzu::tools::development::which(
                &directory,
                &options,
                &directory.join(store),
                command,
            );
        }
        #[cfg(feature = "mise-integration")]
        Commands::Exec { store, args } => {
            anyhow::ensure!(
                !cli.json,
                "--json is not supported for exec; use oyzu env --json to inspect selection"
            );
            return oyzu::tools::development::exec(
                &directory,
                &options,
                &directory.join(store),
                args,
            );
        }
        Commands::Tools {
            command: ToolCommand::Backend { path },
        } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&oyzu::tools::inspect_backend(
                    &directory.join(path)
                )?)?
            );
            return Ok(0);
        }
        Commands::Tools {
            command: ToolCommand::Tree { path },
        } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&oyzu::tools::inspect_tree(&directory.join(path))?)?
            );
            return Ok(0);
        }
        Commands::Tools {
            command: ToolCommand::Lock { path },
        } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&oyzu::tools::inspect_lock(&directory.join(path))?)?
            );
            return Ok(0);
        }
        Commands::Config { command } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&oyzu::invocation::configuration(
                    &command.operation(),
                    &directory,
                    &options
                )?)?
            );
            return Ok(0);
        }
        Commands::Build { plan, image } => {
            let result = build::run_with_options(&directory, image, *plan, &options)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(if *plan || result["status"] == "succeeded" {
                0
            } else {
                1
            });
        }
        Commands::Inspect { bundle } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build::inspect(&directory.join(bundle))?)?
            );
            return Ok(0);
        }
        _ => {}
    }
    let workspace = discovery::discover_with_options(&directory, None, &options)?;
    match cli.command {
        Commands::Discover => println!("{}", serde_json::to_string_pretty(&workspace)?),
        Commands::Run { task, args } => {
            let requested = task.as_deref().unwrap_or("list");
            if requested == "list" {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&workspace.tasks)?);
                } else {
                    for task in workspace.tasks.values() {
                        let detail = task.availability.as_deref().unwrap_or("available");
                        let label = format!("{:<32}", task.id());
                        let label = presentation::label(
                            &workspace,
                            task,
                            &label,
                            task.availability.is_none(),
                            std::io::stdout().is_terminal(),
                        );
                        println!("{label} {:<12} {}", task.provider, detail);
                    }
                }
            } else {
                let outcomes = tasks::run(&workspace, requested, &args)?;
                let code = outcomes.last().map_or(0, |o| o.exit_code);
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&outcomes)?);
                } else {
                    for outcome in outcomes {
                        print!("{}", outcome.stdout);
                        eprint!("{}", outcome.stderr);
                        let status = workspace.tasks.get(&outcome.task).map_or_else(
                            || outcome.status.clone(),
                            |task| {
                                presentation::label(
                                    &workspace,
                                    task,
                                    &outcome.status,
                                    outcome.exit_code == 0,
                                    std::io::stderr().is_terminal(),
                                )
                            },
                        );
                        eprintln!("{}: {status} (development execution)", outcome.task);
                    }
                }
                return Ok(code);
            }
        }
        Commands::Build { .. }
        | Commands::Inspect { .. }
        | Commands::Config { .. }
        | Commands::Tools { .. } => {
            unreachable!()
        }
        #[cfg(feature = "mise-integration")]
        Commands::Install { .. }
        | Commands::Exec { .. }
        | Commands::Which { .. }
        | Commands::Env { .. } => unreachable!(),
        #[cfg(feature = "mise-integration")]
        Commands::Activate { .. } | Commands::Deactivate { .. } | Commands::HookEnv { .. } => {
            unreachable!()
        }
    }
    Ok(0)
}
