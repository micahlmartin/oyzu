//! Shared isolated lifecycle for native dependency acquisition.
//!
//! Managers supply commands, assets, environment and source routes. This module
//! owns temporary workspace and broker lifetimes; it never interprets native models.
use crate::{
    broker,
    builders::{PreparationContext, RuntimeFile},
    executor, snapshot,
};
use anyhow::{bail, Result};
use std::{collections::BTreeMap, fs, time::Duration};

pub(crate) fn capture(
    context: &PreparationContext<'_>,
    runtime_files: &[RuntimeFile],
    argv: &[String],
    env: &BTreeMap<String, String>,
    sources: Vec<broker::Source>,
) -> Result<snapshot::Snapshot> {
    let control = tempfile::tempdir()?;
    let workspace = control.path().join("workspace");
    snapshot::capture(&context.target.path, &workspace)?;
    fs::create_dir(context.destination)?;
    let runtime = control.path().join("runtime");
    let spool = control.path().join("spool");
    let private = control.path().join("private");
    for path in [&runtime, &spool, &private] {
        fs::create_dir(path)?;
    }
    for file in runtime_files {
        fs::write(runtime.join(file.name), file.contents)?;
    }
    let session = broker::Session::start(&spool, &private, sources)?;
    let stdout = control.path().join("stdout");
    let stderr = control.path().join("stderr");
    let result = executor::execute_with_mounts(
        executor::Request {
            log: context.log.clone(),
            image: context.image,
            workspace: &workspace,
            output: context.destination,
            cwd: "/workspace",
            argv,
            env,
            stdout: &stdout,
            stderr: &stderr,
            timeout: Duration::from_secs(600),
            name: context.execution_name,
        },
        &[
            executor::Mount {
                source: &runtime,
                destination: "/oyzu",
                readonly: true,
            },
            executor::Mount {
                source: &spool,
                destination: "/broker",
                readonly: false,
            },
        ],
    )?;
    drop(session);
    if result.code != 0 {
        let logs = format!(
            "{}\n{}",
            fs::read_to_string(stdout)?,
            fs::read_to_string(stderr)?
        );
        let tail: String = logs
            .chars()
            .rev()
            .take(12000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        bail!(
            "{} dependency preparation failed ({}): {tail}",
            context.target.manager,
            result.code
        );
    }
    snapshot::capture_prepared(context.destination, &control.path().join("frozen"))
}
