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
            "CARGO_LLVM_COV_TARGET_DIR".into(),
            ".oyzu-build/target/coverage".into(),
        ),
        (
            "CARGO_LLVM_COV_BUILD_DIR".into(),
            ".oyzu-build/target/coverage".into(),
        ),
        ("CARGO_LLVM_COV_SETUP".into(), "no".into()),
    ])
}

/// Cargo subcommands spawn Cargo again, so source configuration must be in the
/// private native home, not only a top-level --config argument.
pub(super) fn command(argv: Vec<String>) -> Vec<String> {
    let mut command = vec!["sh".into(), "-ec".into(),
        "mkdir -p \"$CARGO_HOME\"; cp /dependencies/cargo-config.toml \"$CARGO_HOME/config.toml\"; exec \"$@\"".into(),
        "oyzu-cargo".into()];
    command.extend(argv);
    command
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
    let packages = super::acquisition::capture(&workspace.join("Cargo.lock"), context.destination)?;
    let native_output = control.path().join("native-output");
    fs::create_dir(&native_output)?;
    let runner = Native {
        context: &context,
        workspace: &workspace,
        logs: control.path(),
        output: &native_output,
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
    records::write(
        &context.destination.join("binaries.json"),
        &json!({"schemaVersion":1,"binaries":projected.binaries()?.iter().map(|(package, target)| {
            json!({"packageId":package.id,"name":target.name})
        }).collect::<Vec<_>>()}),
    )?;
    let rustc = runner.run(&["rustc", "-vV"])?;
    let host = rustc
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .context("rustc host missing")?;
    let cargo = runner.run(&["cargo", "--version"])?;
    let nextest = runner.run(&["cargo", "nextest", "--version"])?;
    let coverage = runner.run(&["cargo", "llvm-cov", "--version"])?;
    runner.run(&["cargo", "llvm-cov", "show-env", "--sh"])?;
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
    fs::write(
        context.destination.join("nextest.toml"),
        super::reporting::configuration(
            &workspace,
            &format!("/out/{}/reports/junit.xml", context.target.name),
        )?,
    )?;
    records::write(
        &context.destination.join("metadata.json"),
        &serde_json::to_value(&projected)?,
    )?;
    fs::write(context.destination.join("host.txt"), host)?;
    let tree = snapshot::capture_prepared(context.destination, &control.path().join("frozen"))?;
    let platform = json!({"os":context.image.os,"arch":context.image.arch,"abi":host});
    let record = json!({
        "schemaVersion":"v1alpha1", "kind":"dependency-snapshot",
        "adapter":{"id":"rust/cargo-workspace", "digest":snapshot::file_digest(&std::env::current_exe()?)?, "layoutVersion":"4"},
        "manager":{"id":"cargo","version":manager_version,"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":[snapshot::file_digest(&root.join("Cargo.lock"))?],
        "targetPlatform":platform,"packages":packages,"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/cargo-workspace":{"original":original,"projected":projected},"oyzu.dev/cargo-tools":{"rustc":rustc.trim(),"nextest":nextest.trim(),"llvmCov":coverage.trim()}}
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
    output: &'a Path,
}

impl Native<'_, '_> {
    fn run(&self, args: &[&str]) -> Result<String> {
        let stdout = self.logs.join("stdout");
        let stderr = self.logs.join("stderr");
        let command = command(args.iter().map(|s| s.to_string()).collect());
        let result = executor::execute_with_mounts(
            executor::Request {
                image: self.context.image,
                workspace: self.workspace,
                output: self.output,
                cwd: "/workspace",
                argv: &command,
                env: &environment(),
                stdout: &stdout,
                stderr: &stderr,
                timeout: Duration::from_secs(120),
                name: self.context.execution_name,
            },
            &[executor::Mount {
                source: self.context.destination,
                destination: "/dependencies",
                readonly: true,
            }],
        )?;
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
