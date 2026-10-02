use super::metadata::Metadata;
use crate::{builders::PreparationContext, dependencies::Prepared, executor, records, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::{collections::BTreeMap, fs, time::Duration};

pub(super) fn prepare(context: PreparationContext<'_>) -> Result<Prepared> {
    let apparmor = context
        .configuration
        .get("docker.apparmorProfile")
        .and_then(serde_json::Value::as_str)
        .context("CONFIG_INVALID_VALUE: missing resolved Docker execution profile")?;
    let control = tempfile::tempdir()?;
    let workspace = control.path().join("workspace");
    snapshot::capture(&context.target.path, &workspace)?;
    fs::create_dir(context.destination)?;
    let stdout = control.path().join("stdout");
    let stderr = control.path().join("stderr");
    let platform_name = context.target_platform.to_string();
    let argv = crate::builders::strings(&["sh", "-ec", "oyzu-docker-metadata /workspace \"$1\" \"$2\" > /out/metadata.json\nbuildctl --version > /out/manager-version.txt\nhadolint --version > /out/linter-version.txt\ndockerfmt version > /out/formatter-version.txt", "oyzu-docker-preparation", &platform_name, executor::BUILDKIT_SOURCE_DATE_EPOCH]);
    let result = executor::execute(executor::Request {
        log: context.log.clone(),
        image: context.image,
        workspace: &workspace,
        output: context.destination,
        cwd: "/workspace",
        argv: &argv,
        env: &BTreeMap::new(),
        stdout: &stdout,
        stderr: &stderr,
        timeout: Duration::from_secs(60),
        name: context.execution_name,
    })?;
    if result.code != 0 {
        bail!(
            "Docker metadata preparation failed: {}",
            fs::read_to_string(stderr)?
        );
    }
    let path = context.destination.join("metadata.json");
    if fs::metadata(&path)?.len() > 16 * 1024 * 1024 {
        bail!("Docker metadata exceeds 16 MiB");
    }
    let metadata: Metadata = serde_json::from_slice(&fs::read(&path)?)?;
    metadata.validate(&platform_name)?;
    metadata.validate_execution(&context.image.platform()?, context.target_platform)?;
    let images = crate::dependencies::images::capture(
        crate::dependencies::images::Capture {
            log: context.log.clone(),
            destination: context.destination,
            image: context.image,
            target_platform: context.target_platform,
            execution_name: context.execution_name,
        },
        &metadata.image_references()?,
    )?;
    let dependency_context = if let Some(base) = metadata.dependency_base()? {
        let captured_base = images
            .iter()
            .find(|image| image.reference == base)
            .context("missing captured dependency consumer base")?;
        // Resolve the already captured immutable config, never a tag that may
        // have changed since base acquisition. Retain the requested name as evidence.
        let mut runtime = executor::resolve_for(&captured_base.config, executor::Profile::Process)?;
        runtime.reference = base.clone();
        let captured = crate::dependencies::context::prepare(&context, &runtime)?;
        captured.validate(
            &base,
            &images,
            context.target_platform,
            context.source_digest,
        )?;
        Some(captured)
    } else {
        if context.dependency_selector.is_some() {
            bail!(
                "dependencies selector requires an external Dockerfile context named dependencies"
            );
        }
        None
    };
    // Host provisioning is explicit, and the selected boundary is part of the
    // prepared record/plan. Never change host security settings during a build.
    let tree = snapshot::capture_prepared(context.destination, &control.path().join("frozen"))?;
    let version = fs::read_to_string(context.destination.join("manager-version.txt"))?;
    let platform = json!({"os":context.image.os,"arch":context.image.arch});
    let mut record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
        "adapter":{"id":"docker/local-context","digest":snapshot::file_digest(&std::env::current_exe()?)?,"layoutVersion":"5"},
        "manager":{"id":"buildkit","version":version.trim(),"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":[],"targetPlatform":context.target_platform,"packages":[],"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/docker":{"metadata":metadata,"images":images,"imageSource":"provisioned-daemon","apparmorProfile":apparmor,"dockerfileDigest":snapshot::file_digest(&context.target.path.join("Dockerfile"))?,
            "quality":{"hadolint":fs::read_to_string(context.destination.join("linter-version.txt"))?.trim(),
                "dockerfmt":fs::read_to_string(context.destination.join("formatter-version.txt"))?.trim()}}}});
    if let Some(captured) = dependency_context {
        record["packages"] = captured.snapshot["packages"].clone();
        record["lockDigests"] = captured.snapshot["lockDigests"].clone();
        record["extensions"]["oyzu.dev/docker"]["dependencyContext"] =
            serde_json::to_value(captured)?;
    }
    Ok(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    })
}
