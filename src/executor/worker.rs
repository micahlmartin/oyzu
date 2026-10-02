//! Private rootless BuildKit worker. Application RUN never receives a host socket.
#[cfg(test)]
mod native_dependencies;
#[cfg(test)]
mod native_recipe;
use super::files::{file, input, output_file};
use super::{docker_path, run, Execution, Mode, Request};
use crate::snapshot;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
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

/// Assemble already selected source files and explicit artifact inputs. Native
/// ignore rules apply only to source selection, never to artifact descendants.
fn capture_context(
    root: &Path,
    destination: &Path,
    cwd: &str,
    context_files: &[String],
    materialized: &[String],
) -> Result<()> {
    let paths: BTreeSet<_> = context_files.iter().collect();
    let mut entries = paths.len();
    let mut total = 0;
    if entries > 100000 {
        bail!("materialized image context exceeds entry limit");
    }
    for path in paths {
        copy(root, path, destination, &mut total)?;
    }
    for artifact in materialized {
        let relative = if cwd == "." {
            artifact.as_str()
        } else {
            artifact
                .strip_prefix(&format!("{cwd}/"))
                .context("materialization is outside image context")?
        };
        let source = input(root, relative)?;
        let target = destination.join(relative);
        if target.symlink_metadata().is_ok() {
            bail!("materialized image context collides with another input");
        }
        if source.is_dir() {
            let tree = snapshot::inspect_tree(&source)?;
            entries = entries
                .checked_add(tree.entries.len() + 1)
                .context("context entry count overflow")?;
            total = total
                .checked_add(tree.entries.iter().map(|entry| entry.size).sum::<u64>())
                .context("context size overflow")?;
            if entries > 100000 || total > 10 * 1024 * 1024 * 1024 {
                bail!("materialized image context exceeds entry or byte limit");
            }
            fs::create_dir_all(target.parent().context("missing context parent")?)?;
            let copied = snapshot::capture_prepared(&source, &target)?;
            if copied.digest != tree.digest {
                bail!("materialized directory changed while copied");
            }
        } else {
            entries += 1;
            if entries > 100000 {
                bail!("materialized image context exceeds entry limit");
            }
            copy(root, relative, destination, &mut total)?;
        }
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

/// A definition belongs to private executor staging. Generated instructions are
/// bound to their plan identity; a project Dockerfile is never rewritten.
fn write_definition(
    root: &Path,
    definition: &Path,
    digest: &str,
    recipe: Option<&super::recipe::Recipe>,
    images: &[super::ImageInput],
) -> Result<()> {
    let destination = definition.join("Dockerfile");
    if destination.symlink_metadata().is_ok() {
        bail!("private Dockerfile destination already exists");
    }
    if let Some(recipe) = recipe {
        fs::write(&destination, recipe.render(images)?)?;
    } else {
        fs::copy(file(root, "Dockerfile")?, &destination)?;
    }
    // Verify the bytes actually handed to BuildKit, including a source change
    // during copying. Neither an earlier check nor a declared hash suffices.
    if snapshot::file_digest(&destination)? != digest {
        bail!("Dockerfile changed after preflight; replan the captured definition");
    }
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

fn capture_image_store(root: &Path, image: &super::ImageInput, destination: &Path) -> Result<()> {
    image.validate()?;
    let mut source = root.to_path_buf();
    for part in image.store.split('/') {
        source.push(part);
        if fs::symlink_metadata(&source)?.file_type().is_symlink() {
            bail!("captured image store path is a symlink");
        }
    }
    if !source.canonicalize()?.starts_with(root.canonicalize()?) {
        bail!("image store escapes prepared dependencies");
    }
    let captured = snapshot::capture_prepared(&source, destination)?;
    if captured.digest != image.tree_digest {
        bail!("captured image store changed after planning");
    }
    normalize_context(destination)?;
    // buildctl initializes a local content store even for read-only use.
    fs::create_dir(destination.join("ingest"))?;
    Ok(())
}

pub(super) fn execute(
    request: Request<'_>,
    mounts: &[super::Mount<'_>],
    mode: &Mode,
    materialized: &[String],
    target_platform: &crate::platform::Platform,
) -> Result<Execution> {
    let Mode::Buildkit {
        output,
        context_files,
        apparmor_profile,
        dockerfile_digest,
        generated_recipe,
        images,
        dependency_context,
        ..
    } = mode
    else {
        unreachable!()
    };
    if dependency_context
        .as_ref()
        .is_some_and(|context| &context.platform != target_platform)
    {
        bail!("dependency context platform differs from image target");
    }
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
    capture_context(&root, &context, cwd, context_files, materialized)?;
    write_definition(
        &root,
        &definition,
        dockerfile_digest,
        generated_recipe.as_deref(),
        images,
    )?;
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
    if !images.is_empty() {
        let prepared = mounts
            .iter()
            .find(|m| m.destination == "/dependencies" && m.readonly)
            .context("captured image stores require prepared dependencies")?;
        for (index, image) in images.iter().enumerate() {
            let destination = private.path().join(format!("base-{index}"));
            capture_image_store(prepared.source, image, &destination)?;
            bind(
                &mut start,
                &destination,
                &format!("/inputs/base-{index}"),
                true,
            )?;
        }
    }
    if let Some(context) = dependency_context {
        let prepared = mounts
            .iter()
            .find(|m| m.destination == "/dependencies" && m.readonly)
            .context("captured dependency context requires prepared dependencies")?;
        let destination = private.path().join("packages");
        context.capture(prepared.source, &destination, target_platform)?;
        normalize_context(&destination)?;
        bind(&mut start, &destination, "/inputs/dependencies", true)?;
    }
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
    let expected = mode.argv(&target_platform.to_string()).unwrap();
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
    fn generated_definitions_are_private_and_bound_to_their_actual_bytes() {
        use crate::executor::recipe::{Base, Copy, Recipe};
        let source = tempfile::tempdir().unwrap();
        let definition = tempfile::tempdir().unwrap();
        fs::write(source.path().join("Dockerfile"), "source-owned bytes").unwrap();
        let recipe = Recipe {
            base: Base::Scratch,
            copies: vec![Copy {
                source: "app".into(),
                destination: "/app/app".into(),
            }],
            uid: 65532,
            gid: 65532,
            workdir: "/app".into(),
            entrypoint: vec!["/app/app".into()],
        };
        let digest = recipe.digest(&[]).unwrap();
        write_definition(
            source.path(),
            definition.path(),
            &digest,
            Some(&recipe),
            &[],
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(source.path().join("Dockerfile")).unwrap(),
            "source-owned bytes"
        );
        assert_eq!(
            snapshot::file_digest(&definition.path().join("Dockerfile")).unwrap(),
            digest
        );
        assert!(write_definition(
            source.path(),
            definition.path(),
            &digest,
            Some(&recipe),
            &[]
        )
        .is_err());
        let invalid = tempfile::tempdir().unwrap();
        assert!(write_definition(
            source.path(),
            invalid.path(),
            &format!("sha256:{}", "0".repeat(64)),
            Some(&recipe),
            &[]
        )
        .is_err());
        let copied = tempfile::tempdir().unwrap();
        let source_digest = snapshot::file_digest(&source.path().join("Dockerfile")).unwrap();
        write_definition(source.path(), copied.path(), &source_digest, None, &[]).unwrap();
        assert_eq!(
            fs::read_to_string(copied.path().join("Dockerfile")).unwrap(),
            "source-owned bytes"
        );
    }

    #[test]
    fn image_stores_are_contained_copied_and_bound_to_the_plan() {
        let root = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let store = root.path().join("images/base-0");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join("index.json"), b"fixture input identity").unwrap();
        let capture = snapshot::capture_prepared(&store, &output.path().join("initial")).unwrap();
        let mut binding = super::super::ImageInput {
            reference: "example/base:1".into(),
            name: "example/base:1".into(),
            store: "images/base-0".into(),
            manifest: format!("sha256:{}", "1".repeat(64)),
            config: format!("sha256:{}", "2".repeat(64)),
            tree_digest: capture.digest,
        };
        capture_image_store(root.path(), &binding, &output.path().join("worker")).unwrap();
        assert!(output.path().join("worker/ingest").is_dir());
        assert!(!store.join("ingest").exists());
        fs::write(store.join("index.json"), b"tampered").unwrap();
        assert!(
            capture_image_store(root.path(), &binding, &output.path().join("changed"))
                .unwrap_err()
                .to_string()
                .contains("changed after planning")
        );
        binding.store = "images/base-0/../../../outside".into();
        assert!(capture_image_store(root.path(), &binding, &output.path().join("escape")).is_err());
    }

    #[test]
    fn context_includes_complete_directory_artifacts_and_file_inputs() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::create_dir_all(source.path().join("site/dist/empty")).unwrap();
        fs::write(source.path().join("site/dist/index.html"), "frontend").unwrap();
        fs::write(source.path().join("site/.hidden"), "included").unwrap();
        fs::write(source.path().join("Dockerfile"), "FROM scratch").unwrap();
        fs::write(source.path().join("binary"), "executable bytes").unwrap();
        fs::write(source.path().join("unselected"), "not a context input").unwrap();
        capture_context(
            source.path(),
            destination.path(),
            "image",
            &["Dockerfile".into()],
            &["image/site".into(), "image/binary".into()],
        )
        .unwrap();
        assert_eq!(
            fs::read(destination.path().join("site/dist/index.html")).unwrap(),
            b"frontend"
        );
        assert!(destination.path().join("site/dist/empty").is_dir());
        assert!(destination.path().join("site/.hidden").is_file());
        assert!(destination.path().join("binary").is_file());
        assert!(!destination.path().join("unselected").exists());
        assert_eq!(
            snapshot::inspect_tree(&source.path().join("site"))
                .unwrap()
                .digest,
            snapshot::inspect_tree(&destination.path().join("site"))
                .unwrap()
                .digest
        );
        fs::write(
            destination.path().join("site/dist/index.html"),
            "private copy",
        )
        .unwrap();
        assert_eq!(
            fs::read(source.path().join("site/dist/index.html")).unwrap(),
            b"frontend"
        );
        for paths in [
            vec!["elsewhere/site".into()],
            vec!["image/../site".into()],
            vec!["image/site".into(), "image/site".into()],
        ] {
            let output = tempfile::tempdir().unwrap();
            assert!(capture_context(source.path(), output.path(), "image", &[], &paths).is_err());
        }
        let output = tempfile::tempdir().unwrap();
        assert!(capture_context(
            source.path(),
            output.path(),
            ".",
            &["site/dist/index.html".into()],
            &["site".into()]
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn directory_context_rejects_links_and_retains_executable_intent() {
        use std::os::unix::{fs::symlink, fs::PermissionsExt};
        let source = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        fs::create_dir(source.path().join("site")).unwrap();
        fs::write(source.path().join("site/run"), "executable").unwrap();
        fs::set_permissions(
            source.path().join("site/run"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        capture_context(source.path(), output.path(), ".", &[], &["site".into()]).unwrap();
        assert_ne!(
            fs::metadata(output.path().join("site/run"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
        symlink("run", source.path().join("site/link")).unwrap();
        let linked = tempfile::tempdir().unwrap();
        assert!(capture_context(source.path(), linked.path(), ".", &[], &["site".into()]).is_err());
        symlink("site", source.path().join("alias")).unwrap();
        assert!(
            capture_context(source.path(), linked.path(), ".", &[], &["alias".into()]).is_err()
        );
    }

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
