//! Prepare version-projected Cargo inputs without executing build scripts or project code.
use super::metadata::Metadata;
use crate::{builders::PreparationContext, dependencies::Prepared, executor, records, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path, time::Duration};

pub(super) fn environment() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("HOME".into(), "/tmp/oyzu-home".into()),
        ("CARGO_HOME".into(), "/tmp/oyzu-cargo".into()),
        ("RUSTUP_HOME".into(), "/usr/local/rustup".into()),
        ("RUSTUP_TOOLCHAIN".into(), "1.94.0".into()),
        ("CARGO_NET_OFFLINE".into(), "true".into()),
        ("CARGO_TARGET_DIR".into(), ".oyzu-build/target".into()),
        ("CARGO_BUILD_JOBS".into(), "2".into()),
        ("CARGO_INCREMENTAL".into(), "0".into()),
        (
            "RUSTFLAGS".into(),
            "--remap-path-prefix=/workspace=/src".into(),
        ),
    ])
}

pub(super) fn prepare(context: PreparationContext<'_>) -> Result<Prepared> {
    let root = &context.target.path;
    if !root.join("Cargo.lock").is_file() {
        bail!(
            "{}: a native Cargo.lock is required before building",
            context.target.name
        );
    }
    let control = tempfile::tempdir()?;
    let workspace = control.path().join("workspace");
    snapshot::capture(root, &workspace)?;
    fs::create_dir(context.destination)?;
    let runner = Native {
        context: &context,
        workspace: &workspace,
        logs: control.path(),
    };
    let original: Metadata = serde_json::from_str(&runner.run(&[
        "cargo",
        "metadata",
        "--format-version",
        "1",
        "--offline",
        "--locked",
        "--no-deps",
    ])?)?;
    original.validate()?;
    // Full native resolution checks the existing lock before any projection.
    runner.run(&[
        "cargo",
        "metadata",
        "--format-version",
        "1",
        "--offline",
        "--locked",
    ])?;
    let mut files = original.project_versions(&workspace, context.source_digest)?;
    runner.run(&["cargo", "generate-lockfile", "--offline"])?;
    let projected: Metadata = serde_json::from_str(&runner.run(&[
        "cargo",
        "metadata",
        "--format-version",
        "1",
        "--offline",
        "--locked",
    ])?)?;
    projected.validate()?;
    let rustc = runner.run(&["rustc", "-vV"])?;
    let host = rustc
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .context("rustc host missing")?;
    let cargo = runner.run(&["cargo", "--version"])?;
    let manager_version = cargo
        .split_whitespace()
        .nth(1)
        .context("Cargo version missing")?;
    files.push("Cargo.lock".into());
    let overlay = context.destination.join("overlay");
    fs::create_dir(&overlay)?;
    for file in files {
        let out = overlay.join(&file);
        fs::create_dir_all(out.parent().unwrap())?;
        fs::copy(workspace.join(file), out)?;
    }
    // Preserve native nextest settings, supplying only the required report destination.
    let nextest_path = workspace.join(".config/nextest.toml");
    let mut nextest: toml::Value = if nextest_path.exists() {
        toml::from_str(&fs::read_to_string(nextest_path)?)?
    } else {
        toml::Value::Table(Default::default())
    };
    let mut table = nextest
        .as_table_mut()
        .context("invalid nextest configuration")?;
    for part in ["profile", "default", "junit"] {
        table = table
            .entry(part)
            .or_insert_with(|| toml::Value::Table(Default::default()))
            .as_table_mut()
            .context("invalid nextest profile")?;
    }
    table.insert("path".into(), "junit.xml".into());
    table
        .entry("report-skipped")
        .or_insert_with(|| "ignored".into());
    fs::create_dir_all(overlay.join(".config"))?;
    fs::write(
        overlay.join(".config/nextest.toml"),
        toml::to_string(&nextest)?,
    )?;
    records::write(
        &context.destination.join("metadata.json"),
        &serde_json::to_value(&projected)?,
    )?;
    fs::write(context.destination.join("host.txt"), host)?;
    let tree = snapshot::capture(context.destination, &control.path().join("frozen"))?;
    let platform = json!({"os":context.image.os,"arch":context.image.arch,"abi":host});
    let record = json!({
        "schemaVersion":"v1alpha1", "kind":"dependency-snapshot",
        "adapter":{"id":"rust/cargo-local-workspace", "digest":snapshot::file_digest(&std::env::current_exe()?)?, "layoutVersion":"1"},
        "manager":{"id":"cargo","version":manager_version,"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":[snapshot::file_digest(&root.join("Cargo.lock"))?],
        "targetPlatform":platform,"packages":[],"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/cargo-workspace":{"original":original,"projected":projected}}
    });
    Ok(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    })
}

struct Native<'a, 'b> {
    context: &'a PreparationContext<'b>,
    workspace: &'a Path,
    logs: &'a Path,
}

impl Native<'_, '_> {
    fn run(&self, args: &[&str]) -> Result<String> {
        let stdout = self.logs.join("stdout");
        let stderr = self.logs.join("stderr");
        let result = executor::execute(executor::Request {
            image: self.context.image,
            workspace: self.workspace,
            output: self.context.destination,
            cwd: "/workspace",
            argv: &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            env: &environment(),
            stdout: &stdout,
            stderr: &stderr,
            timeout: Duration::from_secs(120),
            name: self.context.execution_name,
        })?;
        if result.code != 0 {
            bail!(
                "Cargo preparation failed ({}): {}",
                args.join(" "),
                fs::read_to_string(stderr)?
                    .chars()
                    .take(6000)
                    .collect::<String>()
            );
        }
        Ok(fs::read_to_string(stdout)?)
    }
}
