//! Snapshot evidence for native managers sharing the immutable archive inventory.
//! Managers own admission and routes; the engine owns isolation and broker lifetime.
use crate::{broker, builders::PreparationContext, dependencies::Prepared, records, snapshot};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(super) enum Acquisition {
    Build,
    /// Ask the native adapter to export its credential-free consumer layout,
    /// only after the same frozen-lock validation used by application builds.
    DependencyContext,
}

impl Acquisition {
    fn command(&self) -> &'static str {
        match self {
            Self::Build => "acquire",
            Self::DependencyContext => "acquire-context",
        }
    }
}

/// Manager identity comes from the selected adapter, never from the consumer
/// target (which can be a Docker image rather than a Node application).
pub(super) fn prepare(
    context: PreparationContext<'_>,
    manager: &str,
    runtime: &str,
    lock: &str,
    acquisition: Acquisition,
    sources: Vec<broker::Source>,
) -> Result<Option<Prepared>> {
    let tree = crate::dependencies::preparation::capture(
        &context,
        super::super::RUNTIME,
        &[
            "node".into(),
            format!("/oyzu/{runtime}"),
            acquisition.command().into(),
        ],
        &super::super::toolchain::preparation_environment(context.target)?,
        sources,
    )?;
    let inventory = records::read(&context.destination.join("inventory.json"))?;
    super::super::toolchain::verify(context.target, inventory["nodeVersion"].as_str())?;
    ensure!(
        inventory["layoutVersion"] == 2,
        "unsupported native registry capture layout"
    );
    let packages: Vec<Value> = inventory["packages"].as_array().context("missing native registry inventory")?
        .iter().enumerate().map(|(index, p)| json!({
            "id":format!("{manager}/package-{index}"), "name":p["name"], "version":p["version"],
            "sourceId":p["sourceId"], "digest":format!("sha256:{}",p["sha256"].as_str().unwrap_or("")),
            "size":p["size"], "purpose":"build", "dependencies":[], "verification":"digest-only"
        })).collect();
    let platform = json!({"os":context.image.os,"arch":context.image.arch});
    let workspaces =
        crate::builders::node::workspace::model::Metadata::read(inventory["workspaces"].clone())?;
    let extensions = BTreeMap::from([(
        format!("oyzu.dev/{manager}"),
        json!({
            "nodeVersion":inventory["nodeVersion"], "inventory":"all-locked-registry-tarballs",
            "integrity":"lockfile-sha512", "dependencyEdges":"not-modeled", "purposeClassification":"build-inputs", "workspaces":workspaces
        }),
    )]);
    let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
        "adapter":{"id":format!("node/{manager}-registry-tarballs"),"digest":snapshot::file_digest(&std::env::current_exe()?)?,"layoutVersion":"2"},
        "manager":{"id":manager,"version":inventory["version"],"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":[snapshot::file_digest(&context.target.path.join(lock))?],
        "targetPlatform":platform,"packages":packages,"preparedTree":tree.digest,"extensions":extensions});
    Ok(Some(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    }))
}
