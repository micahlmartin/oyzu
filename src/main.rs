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
        /// Target IDs; omitted means all targets. Required dependencies are included.
        targets: Vec<String>,
        /// Build targets affected since a local Git ref, including dependents.
        #[arg(long, conflicts_with = "targets")]
        affected: Option<String>,
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
        Commands::Build {
            plan,
            image,
            targets,
            affected,
        } => {
            let log = oyzu::logging::Log::console(if cli.json {
                oyzu::logging::Format::Json
            } else {
                oyzu::logging::Format::Text
            });
            let selection = if let Some(reference) = affected {
                build::Targets::Affected(reference)
            } else {
                build::Targets::Explicit(targets)
            };
            let result =
                match build::run_logged(&directory, image, *plan, &options, selection, &log) {
                    Ok(result) => result,
                    Err(error) => {
                        log.progress(&format!("ERROR: {error:#}"));
                        return Ok(2);
                    }
                };
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                presentation::build_result(&result, *plan, &directory);
            }
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
        Commands::Build { .. } | Commands::Inspect { .. } | Commands::Config { .. } => {
            unreachable!()
        }
    }
    Ok(0)
}
