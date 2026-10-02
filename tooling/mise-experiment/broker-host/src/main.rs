//! Experimental caller of the actual production broker and executor APIs.
use anyhow::{ensure, Context, Result};
use oyzu::{broker, executor};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::PathBuf, time::Duration};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    id: String,
    base: String,
    authorization: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Run {
    image: String,
    workspace: PathBuf,
    output: PathBuf,
    spool: PathBuf,
    private: PathBuf,
    sources: Vec<Source>,
    argv: Vec<String>,
    env: BTreeMap<String, String>,
    timeout_seconds: u64,
    name: String,
}

fn main() -> Result<()> {
    let configuration = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("configuration file required")?,
    )
    .canonicalize()?;
    let run: Run = serde_json::from_slice(&fs::read(&configuration)?)?;
    ensure!(
        (1..=900).contains(&run.timeout_seconds),
        "bounded timeout required"
    );
    ensure!(!run.argv.is_empty(), "worker command required");
    for path in [&run.workspace, &run.output, &run.spool, &run.private] {
        ensure!(
            path.is_absolute(),
            "absolute experiment directories required"
        );
        fs::create_dir_all(path)?;
    }
    let private = run.private.canonicalize()?;
    for mounted in [&run.workspace, &run.output, &run.spool] {
        let mounted = mounted.canonicalize()?;
        ensure!(
            !private.starts_with(&mounted) && !configuration.starts_with(&mounted),
            "host-private state must not be mounted in the worker"
        );
    }
    let sources = run
        .sources
        .into_iter()
        .map(|s| broker::Source::new(&s.id, &s.base, s.authorization))
        .collect::<Result<Vec<_>>>()?;
    let image = executor::resolve(&run.image)?;
    let session = broker::Session::start(&run.spool, &private, sources)?;
    let stdout = run.output.join("stdout.log");
    let stderr = run.output.join("stderr.log");
    let result = executor::execute_with_mounts(
        executor::Request {
            image: &image,
            workspace: &run.workspace,
            output: &run.output,
            cwd: "/workspace",
            argv: &run.argv,
            env: &run.env,
            stdout: &stdout,
            stderr: &stderr,
            timeout: Duration::from_secs(run.timeout_seconds),
            name: &run.name,
        },
        &[executor::Mount {
            source: &run.spool,
            destination: "/broker",
            readonly: false,
        }],
    )?;
    drop(session);
    println!(
        "{}",
        serde_json::json!({
            "code": result.code, "timed_out": result.timed_out,
            "duration_ms": result.duration_ms,
            "worker": {"image": image.digest, "os": image.os, "arch": image.arch},
        })
    );
    ensure!(
        result.code == 0 && !result.timed_out,
        "qualification worker failed; inspect captured output"
    );
    Ok(())
}
