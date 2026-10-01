//! Container execution never mounts the live checkout, user home or Docker socket.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Image {
    pub reference: String,
    pub digest: String,
    pub os: String,
    pub arch: String,
}

#[derive(Debug)]
pub struct Execution {
    pub code: i32,
    pub duration_ms: u64,
    pub timed_out: bool,
}

pub fn resolve(reference: &str) -> Result<Image> {
    let result = Command::new("docker")
        .args(["image", "inspect", reference])
        .output()
        .context(
            "Docker executor unavailable: provision Docker and the builder image before building",
        )?;
    if !result.status.success() {
        bail!(
            "Docker image {reference} unavailable: pre-provision it before building ({})",
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }
    let objects: Vec<Value> = serde_json::from_slice(&result.stdout)?;
    let info = objects
        .first()
        .context("Docker image inspection returned no image")?;
    let image = Image {
        reference: reference.into(),
        digest: info["Id"]
            .as_str()
            .context("missing image identity")?
            .into(),
        os: info["Os"].as_str().context("missing image OS")?.into(),
        arch: info["Architecture"]
            .as_str()
            .context("missing image architecture")?
            .into(),
    };
    if image.os != "linux" {
        bail!(
            "executor currently requires a Linux image, found {}",
            image.os
        );
    }
    if !image.digest.starts_with("sha256:") || image.digest.len() != 71 {
        bail!("unverifiable image identity");
    }
    if info["Config"]["Volumes"]
        .as_object()
        .is_some_and(|v| !v.is_empty())
    {
        bail!("builder images with implicit volumes are unsupported");
    }
    Ok(image)
}

pub struct Request<'a> {
    pub image: &'a Image,
    pub workspace: &'a Path,
    pub output: &'a Path,
    pub cwd: &'a str,
    pub argv: &'a [String],
    pub env: &'a BTreeMap<String, String>,
    pub stdout: &'a Path,
    pub stderr: &'a Path,
    pub timeout: Duration,
    pub name: &'a str,
}

pub fn execute(request: Request<'_>) -> Result<Execution> {
    let executable = request.argv.first().context("empty action command")?;
    let mut command = Command::new("docker");
    command.args([
        "run",
        "--rm",
        "--pull=never",
        "--name",
        request.name,
        "--network=none",
        "--read-only",
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges",
        "--pids-limit=256",
        "--memory=2g",
        "--cpus=2",
        "--tmpfs",
        "/tmp:rw,nosuid,size=536870912",
        "--mount",
    ]);
    let workspace = request.workspace.canonicalize()?;
    let output = request.output.canonicalize()?;
    for path in [&workspace, &output] {
        if path.to_string_lossy().contains(',') {
            bail!("Docker bind mount paths containing commas are unsupported");
        }
    }
    command.arg(format!(
        "type=bind,source={},target=/workspace",
        docker_path(&workspace)
    ));
    command.arg("--mount").arg(format!(
        "type=bind,source={},target=/out",
        docker_path(&output)
    ));
    command.args(["--workdir", request.cwd]);
    for (key, value) in request.env {
        command.arg("--env").arg(format!("{key}={value}"));
    }
    command
        .args(["--entrypoint", executable, &request.image.digest])
        .args(&request.argv[1..]);
    let stdout = fs::File::create(request.stdout)?;
    let stderr = fs::File::create(request.stderr)?;
    command.stdin(Stdio::null()).stdout(stdout).stderr(stderr);
    let start = Instant::now();
    let mut child = command
        .spawn()
        .context("cannot launch container executor")?;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Execution {
                code: status.code().unwrap_or(1),
                duration_ms: start.elapsed().as_millis() as u64,
                timed_out: false,
            });
        }
        // Bound logs before they can consume the entire build host filesystem.
        let log_limit = 16 * 1024 * 1024;
        let too_large = fs::metadata(request.stdout)?.len() > log_limit
            || fs::metadata(request.stderr)?.len() > log_limit;
        if start.elapsed() > request.timeout || too_large {
            let _ = Command::new("docker")
                .args(["rm", "--force", request.name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = child.kill();
            let _ = child.wait();
            return Ok(Execution {
                code: if too_large { 125 } else { 124 },
                duration_ms: start.elapsed().as_millis() as u64,
                timed_out: !too_large,
            });
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn docker_path(path: &Path) -> String {
    // Windows canonical paths use the extended-length prefix, which Docker rejects.
    path.to_string_lossy()
        .strip_prefix(r"\\?\")
        .unwrap_or(&path.to_string_lossy())
        .to_string()
}
