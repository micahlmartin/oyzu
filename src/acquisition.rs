//! Native Python wheel acquisition through the scoped broker (OEP-0017).
use crate::{broker, executor, records, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub const PYTHON_IMAGE: &str = "python:3.12-slim-bookworm";
pub const UV_IMAGE: &str = "ghcr.io/astral-sh/uv:0.12.21-python3.12-bookworm-slim";
pub const PYTHON_HELPER: &str = include_str!("helpers/python.py");

pub struct Prepared {
    pub root: PathBuf,
    pub digest: String,
    pub record: Value,
}

pub fn python(
    root: &Path,
    destination: &Path,
    image: &executor::Image,
    source_digest: &str,
    name: &str,
    manager: &str,
) -> Result<Prepared> {
    fs::create_dir(destination)?;
    let control = tempfile::tempdir()?;
    let helper = control.path().join("helper");
    fs::create_dir(&helper)?;
    fs::write(helper.join("python.py"), PYTHON_HELPER)?;
    let spool = control.path().join("spool");
    fs::create_dir(&spool)?;
    let private = control.path().join("private");
    fs::create_dir(&private)?;
    let _session = broker::Session::start(&spool, &private, broker::python_sources()?)?;
    let env = BTreeMap::from([
        ("OYZU_PYTHON_MANAGER".into(), manager.into()),
        ("UV_CACHE_DIR".into(), "/tmp/uv-cache".into()),
        ("HOME".into(), "/tmp/oyzu-home".into()),
        ("PYTHONNOUSERSITE".into(), "1".into()),
        ("PIP_CONFIG_FILE".into(), "/dev/null".into()),
        ("PIP_DISABLE_PIP_VERSION_CHECK".into(), "1".into()),
    ]);
    let argv = vec![
        "python".into(),
        "-I".into(),
        "/oyzu/python.py".into(),
        "acquire".into(),
    ];
    let stdout = control.path().join("stdout");
    let stderr = control.path().join("stderr");
    let execution = executor::execute_with_mounts(
        executor::Request {
            image,
            workspace: root,
            output: destination,
            cwd: "/workspace",
            argv: &argv,
            env: &env,
            stdout: &stdout,
            stderr: &stderr,
            timeout: Duration::from_secs(600),
            name,
        },
        &[
            executor::Mount {
                source: &helper,
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
    if execution.code != 0 {
        // Native logs refer only to the scoped loopback endpoint; no upstream tokens are supplied.
        bail!(
            "Python dependency preparation failed: {}",
            fs::read_to_string(stderr)?
                .chars()
                .take(6000)
                .collect::<String>()
        );
    }
    let metadata = records::read(&destination.join("packages.json"))?;
    let tree = snapshot::capture(destination, &control.path().join("frozen"))?;
    let packages: Vec<Value> = metadata["packages"].as_array().context("missing acquired package graph")?.iter().map(|p| -> Result<Value> {
        let file = p["file"].as_str().context("missing wheel filename")?;
        if file.contains(['/', '\\', ':']) || !file.ends_with(".whl") {bail!("invalid wheel filename");}
        let path = destination.join("wheels").join(file);
        let info = fs::symlink_metadata(&path)?;
        if !info.is_file() || info.file_type().is_symlink() {bail!("wheel is not a regular file");}
        Ok(json!({"id":p["id"],"name":p["name"],"version":p["version"],"sourceId":"pypi","digest":snapshot::file_digest(&path)?,"size":info.len(),"purpose":p["purpose"],"dependencies":p["dependencies"],"verification":"digest-only"}))
    }).collect::<Result<_>>()?;
    let platform = json!({"os":image.os,"arch":image.arch,"runtime":metadata["python"]});
    let lock_digests = ["uv.lock", "requirements.txt"]
        .iter()
        .filter_map(|file| root.join(file).is_file().then_some(root.join(file)))
        .map(|file| snapshot::file_digest(&file))
        .collect::<Result<Vec<_>>>()?;
    let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot","adapter":{"id":format!("python/{manager}-wheels"),"digest":records::digest("oyzu.adapter.v1alpha1",&json!(PYTHON_HELPER))?,"layoutVersion":"1"},"manager":{"id":manager,"version":metadata["managerVersion"],"digest":image.digest,"platform":platform},"sourceDigest":source_digest,"lockDigests":lock_digests,"targetPlatform":platform,"packages":packages,"preparedTree":tree.digest});
    let digest = records::digest("oyzu.dependencies.v1alpha1", &record)?;
    Ok(Prepared {
        root: destination.into(),
        digest,
        record,
    })
}
