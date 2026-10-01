use super::metadata::Metadata;
use crate::{builders::PreparationContext, dependencies::Prepared, executor, records, snapshot};
use anyhow::{bail, Result};
use serde_json::json;
use std::{collections::BTreeMap, fs, time::Duration};

pub(super) fn prepare(context: PreparationContext<'_>) -> Result<Prepared> {
    let control = tempfile::tempdir()?;
    let workspace = control.path().join("workspace");
    snapshot::capture(&context.target.path, &workspace)?;
    fs::create_dir(context.destination)?;
    let stdout = control.path().join("stdout");
    let stderr = control.path().join("stderr");
    let argv = crate::builders::strings(&["sh", "-c", "oyzu-docker-metadata /workspace > /out/metadata.json && buildctl --version > /out/manager-version.txt"]);
    let result = executor::execute(executor::Request {
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
    metadata.validate(&format!("{}/{}", context.image.os, context.image.arch))?;
    // Host provisioning is explicit, and the selected boundary is part of the
    // prepared record/plan. Never change host security settings during a build.
    let apparmor = std::env::var("OYZU_BUILDKIT_APPARMOR_PROFILE").unwrap_or_else(|_| {
        if fs::read_to_string("/proc/sys/kernel/apparmor_restrict_unprivileged_userns")
            .is_ok_and(|v| v.trim() == "1")
        {
            "oyzu-buildkit".into()
        } else {
            "unconfined".into()
        }
    });
    let tree = snapshot::capture_prepared(context.destination, &control.path().join("frozen"))?;
    let version = fs::read_to_string(context.destination.join("manager-version.txt"))?;
    let platform = json!({"os":context.image.os,"arch":context.image.arch});
    let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
        "adapter":{"id":"docker/local-context","digest":snapshot::file_digest(&std::env::current_exe()?)?,"layoutVersion":"1"},
        "manager":{"id":"buildkit","version":version.trim(),"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":[],"targetPlatform":platform,"packages":[],"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/docker":{"metadata":metadata,"apparmorProfile":apparmor,"dockerfileDigest":snapshot::file_digest(&context.target.path.join("Dockerfile"))?}}});
    Ok(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    })
}
