//! Private rootless BuildKit worker. Application RUN never receives a host socket.
use super::{docker_path, run, Execution, Mode, Request};
use crate::snapshot;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

struct Worker<'a> {
    name: &'a str,
    active: bool,
}
impl Worker<'_> {
    fn stop(&mut self) -> Result<()> {
        let result = Command::new("docker")
            .args(["rm", "--force", "--volumes", self.name])
            .output()?;
        if !result.status.success() {
            bail!(
                "could not confirm worker shutdown: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        self.active = false;
        Ok(())
    }
}
impl Drop for Worker<'_> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.stop();
        }
    }
}

fn output_file(root: &Path, relative: &str) -> Result<fs::File> {
    if !snapshot::portable(relative) {
        bail!("invalid worker output path");
    }
    let parts: Vec<_> = relative.split('/').collect();
    let mut path = root.to_path_buf();
    for part in &parts[..parts.len() - 1] {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_dir() => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&path)?,
            _ => bail!("unsafe worker output parent"),
        }
    }
    path.push(parts.last().unwrap());
    Ok(fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?)
}

fn file(root: &Path, relative: &str) -> Result<PathBuf> {
    if !snapshot::portable(relative) {
        bail!("invalid captured context path");
    }
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("BuildKit input is a symlink");
        }
    }
    if !fs::metadata(&path)?.is_file() {
        bail!("BuildKit context input is not a regular file");
    }
    Ok(path)
}

fn copy(root: &Path, relative: &str, destination: &Path, total: &mut u64) -> Result<()> {
    let source = file(root, relative)?;
    let size = fs::metadata(&source)?.len();
    *total = total.checked_add(size).context("context size overflow")?;
    if *total > 10 * 1024 * 1024 * 1024 {
        bail!("BuildKit context exceeds 10 GiB");
    }
    let target = destination.join(relative);
    fs::create_dir_all(target.parent().context("missing context parent")?)?;
    if fs::copy(source, target)? != size {
        bail!("context input changed while copied");
    }
    Ok(())
}

fn bind(command: &mut Command, source: &Path, destination: &str, readonly: bool) -> Result<()> {
    let source = source.canonicalize()?;
    if source.to_string_lossy().contains(',') {
        bail!("unsupported comma in worker bind path");
    }
    command.arg("--mount").arg(format!(
        "type=bind,source={},target={}{}",
        docker_path(&source),
        destination,
        if readonly { ",readonly" } else { "" }
    ));
    Ok(())
}

fn normalize_context(root: &Path) -> Result<()> {
    // The source contract records executable intent, not the host user's umask,
    // ownership or setuid bits. Normalize only our private transport copy.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry?;
            let mode = if entry.file_type().is_dir()
                || entry.metadata()?.permissions().mode() & 0o111 != 0
            {
                0o755
            } else {
                0o644
            };
            fs::set_permissions(entry.path(), fs::Permissions::from_mode(mode))?;
        }
    }
    #[cfg(not(unix))]
    let _ = root;
    Ok(())
}

pub(super) fn execute(
    request: Request<'_>,
    mode: &Mode,
    materialized: &[String],
) -> Result<Execution> {
    let Mode::Buildkit {
        output,
        context_files,
        apparmor_profile,
        dockerfile_digest,
        ..
    } = mode
    else {
        unreachable!()
    };
    let cwd = request
        .cwd
        .strip_prefix("/workspace/")
        .context("invalid BuildKit cwd")?;
    if cwd != "." && !snapshot::portable(cwd) {
        bail!("invalid BuildKit context root");
    }
    let root = request.workspace.join(cwd);
    let mut parent = request.workspace.to_path_buf();
    for part in cwd.split('/').filter(|p| *p != ".") {
        parent.push(part);
        if fs::symlink_metadata(&parent)?.file_type().is_symlink() {
            bail!("image context root is a symlink");
        }
    }
    if !root
        .canonicalize()?
        .starts_with(request.workspace.canonicalize()?)
    {
        bail!("BuildKit context escapes workspace");
    }
    // Both directories are private host data, not the live checkout or the sealed bundle.
    let private = tempfile::tempdir()?;
    let context = private.path().join("context");
    let definition = private.path().join("definition");
    let exported = private.path().join("exported");
    for path in [&context, &definition, &exported] {
        fs::create_dir(path)?;
    }
    let mut paths: BTreeSet<_> = context_files.iter().cloned().collect();
    for input in materialized {
        let relative = if cwd == "." {
            input.as_str()
        } else {
            input
                .strip_prefix(&format!("{cwd}/"))
                .context("materialization is outside image context")?
        };
        if !snapshot::portable(relative) {
            bail!("invalid materialized context path");
        }
        paths.insert(relative.to_owned());
    }
    if paths.len() > 100000 {
        bail!("materialized image context exceeds entry limit");
    }
    let mut total = 0;
    for path in paths {
        copy(&root, &path, &context, &mut total)?;
    }
    let dockerfile = file(&root, "Dockerfile")?;
    if snapshot::file_digest(&dockerfile)? != *dockerfile_digest {
        bail!("Dockerfile changed after preflight; replan the captured definition");
    }
    fs::copy(dockerfile, definition.join("Dockerfile"))?;
    // Native ignore evaluation already selected source inputs. A separate
    // Dockerfile-local override keeps explicit artifact inputs present without
    // modifying the source-owned ignore files copied into the context.
    fs::write(
        definition.join("Dockerfile.dockerignore"),
        b"# Context already filtered from captured native ignore rules.\n",
    )?;
    normalize_context(&context)?;
    normalize_context(&definition)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&exported, fs::Permissions::from_mode(0o777))?;
    }
    let mut start = Command::new("docker");
    start.args([
        "run",
        "--detach",
        "--pull=never",
        "--name",
        request.name,
        "--network=none",
        "--memory=2g",
        "--cpus=2",
        "--pids-limit=256",
        "--security-opt=seccomp=unconfined",
        "--security-opt=systempaths=unconfined",
    ]);
    start.arg(format!("--security-opt=apparmor={apparmor_profile}"));
    bind(&mut start, &context, "/workspace", true)?;
    bind(&mut start, &definition, "/definition", true)?;
    bind(&mut start, &exported, "/output", false)?;
    start.args([
        &request.image.digest,
        "--oci-worker-snapshotter=native",
        "--oci-worker-gc=false",
    ]);
    let mut worker = Worker {
        name: request.name,
        active: true,
    };
    let started = Instant::now();
    let launch = run(start, &request)?;
    if launch.code != 0 {
        return Ok(launch);
    }
    let readiness = (|| -> Result<()> {
        loop {
            let ready = Command::new("docker")
                .args(["exec", request.name, "buildctl", "debug", "workers"])
                .output()?;
            if ready.status.success() {
                break;
            }
            let inspect = Command::new("docker")
                .args(["inspect", request.name])
                .output()?;
            let state: Vec<Value> = serde_json::from_slice(&inspect.stdout)?;
            if state.first().is_none_or(|s| s["State"]["Running"] != true)
                || started.elapsed() > Duration::from_secs(45)
            {
                bail!("rootless BuildKit worker unavailable; provision the selected user-namespace/AppArmor profile ({apparmor_profile})");
            }
            thread::sleep(Duration::from_millis(200));
        }
        let inspect = Command::new("docker")
            .args(["inspect", request.name])
            .output()?;
        let states: Vec<Value> = serde_json::from_slice(&inspect.stdout)?;
        let state = states
            .first()
            .context("worker disappeared during startup")?;
        if state["HostConfig"]["NetworkMode"] != "none"
            || state["HostConfig"]["Privileged"] != false
            || state["AppArmorProfile"] != *apparmor_profile
            || state["Args"]
                .as_array()
                .is_some_and(|v| v.iter().any(|a| a == "--oci-worker-no-process-sandbox"))
        {
            bail!("worker boundary differs from the resolved execution profile");
        }
        Ok(())
    })();
    if let Err(error) = readiness {
        let logs = Command::new("docker")
            .args(["logs", "--tail", "200", request.name])
            .output()?;
        fs::write(request.stderr, [logs.stdout, logs.stderr].concat())?;
        return Err(error);
    }
    let expected = mode
        .argv(&format!("{}/{}", request.image.os, request.image.arch))
        .unwrap();
    if request.argv != expected {
        bail!("BuildKit command differs from its typed plan");
    }
    let mut build = Command::new("docker");
    build.args(["exec", request.name]).args(&expected);
    let mut result = run(build, &request)?;
    if result.code == 0 {
        worker.stop()?;
    } // Confirm shutdown before accepting any output.
    if result.code == 0 {
        let image = file(&exported, "image.tar")?;
        if fs::metadata(&image)?.len() > 10 * 1024 * 1024 * 1024 {
            bail!("exported OCI image exceeds 10 GiB");
        }
        let mut input = fs::File::open(image)?;
        let mut destination = output_file(request.output, output)?;
        std::io::copy(&mut input, &mut destination)?;
        destination.sync_all()?;
    }
    result.duration_ms = started.elapsed().as_millis() as u64;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_files_and_output_destinations_cannot_escape_or_overwrite() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(source.path().join("binary"), b"tested artifact").unwrap();
        let mut total = 0;
        copy(source.path(), "binary", destination.path(), &mut total).unwrap();
        assert_eq!(
            fs::read(destination.path().join("binary")).unwrap(),
            b"tested artifact"
        );
        assert!(file(source.path(), "../binary").is_err());
        assert!(output_file(destination.path(), "binary").is_err());
        assert!(output_file(destination.path(), "../escape").is_err());
        assert!(output_file(destination.path(), "binary/child").is_err());
        output_file(destination.path(), "target/container/image.tar").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(source.path(), destination.path().join("redirect")).unwrap();
            assert!(file(destination.path(), "redirect/binary").is_err());
            assert!(output_file(destination.path(), "redirect/escaped").is_err());
            assert!(!source.path().join("escaped").exists());
        }
    }
}
