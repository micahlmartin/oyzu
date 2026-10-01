//! Admission and evidence for native managers whose registry capture is pending.
use crate::{builders::PreparationContext, dependencies::Prepared, records, snapshot};
use anyhow::{bail, Result};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};

pub(super) fn validate(root: &Path) -> Result<()> {
    let package = records::read(&root.join("package.json"))?;
    if package.get("workspaces").is_some() || root.join("pnpm-workspace.yaml").exists() {
        bail!("native Node workspace capture is not implemented yet");
    }
    for field in [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ] {
        if package[field]
            .as_object()
            .is_some_and(|entries| !entries.is_empty())
        {
            bail!("this manager's registry dependency capture is not implemented yet");
        }
    }
    Ok(())
}

pub(super) fn prepare(
    context: PreparationContext<'_>,
    runtime: &str,
    lock: &str,
) -> Result<Option<Prepared>> {
    validate(&context.target.path)?;
    let tree = crate::dependencies::preparation::capture(
        &context,
        super::super::RUNTIME,
        &["node".into(), format!("/oyzu/{runtime}"), "acquire".into()],
        &BTreeMap::from([
            ("HOME".into(), "/tmp/oyzu-home".into()),
            ("COREPACK_ENABLE_NETWORK".into(), "0".into()),
            ("YARN_IGNORE_PATH".into(), "1".into()),
        ]),
        vec![],
    )?;
    let inventory = records::read(&context.destination.join("inventory.json"))?;
    let platform = json!({"os":context.image.os,"arch":context.image.arch});
    let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
        "adapter":{"id":format!("node/{}-native",context.target.manager),"digest":snapshot::file_digest(&std::env::current_exe()?)?,"layoutVersion":"1"},
        "manager":{"id":context.target.manager,"version":inventory["version"],"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":[snapshot::file_digest(&context.target.path.join(lock))?],
        "targetPlatform":platform,"packages":[],"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/node-manager":{"nodeVersion":inventory["nodeVersion"],"profile":"dependency-free"}}});
    Ok(Some(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    }))
}
