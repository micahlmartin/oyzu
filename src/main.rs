mod config_args;
use anyhow::Result;
use clap::{Parser, Subcommand};
use oyzu::{build, config, discovery, tasks};
use std::path::PathBuf;

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
                serde_json::to_string_pretty(&config::operations::run(
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
                        println!("{:<32} {:<12} {}", task.id(), task.provider, detail);
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
                        eprintln!(
                            "{}: {} (development execution)",
                            outcome.task, outcome.status
                        );
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
