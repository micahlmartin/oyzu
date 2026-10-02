//! Plan and execute aggregate OCI artifacts from complete selected image families.
//! Native builders own image production; OCI owns archive assembly/verification.
use crate::{model::Workspace, platform::Platform, snapshot};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

const KEY: &str = "oyzu.dev/oci-index";
const BUILDER: &str = "oyzu/oci-index";
#[cfg(test)]
mod tests;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    artifact: String,
    producer: String,
    platform: Platform,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Assembly {
    family: String,
    members: Vec<Member>,
}

pub(super) fn is_target(target: &Value) -> bool {
    target["builder"] == BUILDER
}
pub(super) fn is_action(action: &Value) -> bool {
    action["extensions"].get(KEY).is_some()
}

fn host_platform() -> Result<Platform> {
    let os = if std::env::consts::OS == "macos" {
        "darwin"
    } else {
        std::env::consts::OS
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        value => value,
    };
    format!("{os}/{arch}").parse()
}

/// Aggregate only complete selected platform families. Partial dependency
/// selections retain their individual images without inventing a partial index.
pub(super) fn plan(
    workspace: &Workspace,
    mapping: &super::variants::Mapping,
    plan: &mut Value,
) -> Result<()> {
    let selected: BTreeSet<_> = plan["targets"]
        .as_array()
        .context("missing targets")?
        .iter()
        .map(|t| {
            t["id"]
                .as_str()
                .context("missing target id")
                .map(str::to_owned)
        })
        .collect::<Result<_>>()?;
    let outputs = plan["artifacts"]
        .as_array()
        .context("missing artifact intents")?
        .clone();
    let mut used: BTreeSet<_> = workspace
        .targets
        .keys()
        .map(|id| id.to_lowercase())
        .collect();
    for (family, ids) in mapping {
        let mut groups: BTreeMap<BTreeMap<String, String>, Vec<&String>> = BTreeMap::new();
        for id in ids {
            let mut axes = workspace.targets[id].variant.clone();
            if axes.remove("platform").is_some() {
                groups.entry(axes).or_default().push(id);
            }
        }
        for (axes, members) in groups {
            if members.len() < 2 || !members.iter().all(|id| selected.contains(*id)) {
                continue;
            }
            let images: Vec<_> = outputs
                .iter()
                .filter(|a| {
                    a["kind"] == "oci-image" && members.iter().any(|id| a["target"] == **id)
                })
                .collect();
            if images.is_empty() {
                continue;
            }
            let names: BTreeSet<_> = images
                .iter()
                .map(|a| a["name"].as_str().context("missing image name"))
                .collect::<Result<_>>()?;
            for name in names {
                let mut inputs = Vec::new();
                let mut versions = BTreeSet::new();
                for id in &members {
                    let matching: Vec<_> = images
                        .iter()
                        .filter(|a| a["name"] == name && a["target"].as_str() == Some(id.as_str()))
                        .collect();
                    if matching.len() != 1 {
                        bail!("{family}: platform variants must declare the same unique OCI image outputs");
                    }
                    let image = matching[0];
                    versions.insert(image["version"].as_str().context("missing image version")?);
                    inputs.push(Member {
                        artifact: image["id"].as_str().context("missing image id")?.into(),
                        producer: image["producer"]
                            .as_str()
                            .context("missing image producer")?
                            .into(),
                        platform: workspace.targets[*id].variant["platform"].parse()?,
                    });
                }
                if versions.len() != 1 {
                    bail!("{family}: OCI index variants have different versions");
                }
                inputs.sort_by_key(|input| input.platform.to_string());
                let suffix = axes
                    .iter()
                    .map(|(k, v)| format!("-{k}-{v}"))
                    .collect::<String>();
                let id = crate::names::scoped(family, &format!("oci-index-{name}{suffix}"));
                if !used.insert(id.to_lowercase()) {
                    bail!("OCI index target identity collision: {id}");
                }
                let operation = format!("{id}:assemble-index");
                let artifact = format!("{id}/primary");
                let version = versions.into_iter().next().unwrap();
                let assembly = Assembly {
                    family: family.clone(),
                    members: inputs,
                };
                let host = host_platform()?;
                let owner = plan["targets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["id"] == *members[0])
                    .unwrap();
                let builder_digest = owner["builderDigest"].clone();
                let target = json!({"id":id,"builder":BUILDER,"builderDigest":builder_digest,"path":".","variant":axes,"platform":null,"extensions":{KEY:assembly}});
                let inputs: Vec<_> = assembly.members.iter().enumerate().map(|(index, input)| json!({"kind":"artifact","artifact":input.artifact,"producer":input.producer,"mount":format!("images/{index}.oci.tar")})).collect();
                let dependencies: BTreeSet<_> =
                    assembly.members.iter().map(|m| &m.producer).collect();
                let action = json!({"id":operation,"target":id,"operation":"assemble-index","dependsOn":dependencies,
                    "argv":["oyzu-internal","assemble-index"],"cwd":".","env":{},"tools":[id],"executionPlatform":host,"targetPlatform":null,
                    "inputs":inputs,"outputs":[artifact],"reports":[],"required":true,"cacheable":false,"network":"none",
                    "limits":{"timeoutSeconds":0,"cpu":1,"memoryBytes":0,"outputBytes":10737418240u64},"extensions":{KEY:assembly}});
                plan["tools"].as_array_mut().unwrap().push(json!({"id":id,"version":env!("CARGO_PKG_VERSION"),"digest":builder_digest,"platform":host}));
                plan["targets"].as_array_mut().unwrap().push(target);
                plan["actions"].as_array_mut().unwrap().push(action);
                plan["artifacts"].as_array_mut().unwrap().push(json!({"id":artifact,"target":id,"variant":axes,"name":"primary","producer":operation,"kind":"oci-index","version":version,"mediaType":"application/vnd.oci.image.index.v1+json","path":format!("{id}/artifacts/{family}-{version}.oci.tar")}));
            }
        }
    }
    if plan["targets"].as_array().unwrap().len() > 256
        || plan["actions"].as_array().unwrap().len() > 16_384
    {
        bail!("aggregate build plan exceeds target/action limits");
    }
    super::scheduling::Schedule::new(plan["actions"].as_array().unwrap(), 1)?;
    Ok(())
}

fn expected(
    assembly: &Assembly,
    artifacts: &[Value],
    bundle: &Path,
) -> Result<(
    Vec<std::path::PathBuf>,
    BTreeMap<crate::oci::Platform, String>,
)> {
    if assembly.members.is_empty() || assembly.members.len() > 256 {
        bail!("invalid OCI index member count");
    }
    let mut paths = Vec::new();
    let mut images = BTreeMap::new();
    for input in &assembly.members {
        let matches: Vec<_> = artifacts
            .iter()
            .filter(|a| a["id"] == input.artifact)
            .collect();
        if matches.len() != 1 {
            bail!("OCI index is missing a unique required image artifact");
        }
        let artifact = matches[0];
        if artifact["kind"] != "oci-image" || artifact["producer"] != input.producer {
            bail!("OCI index input contract mismatch");
        }
        let path = crate::bundle_store::safe_file(
            bundle,
            artifact["path"]
                .as_str()
                .context("missing OCI input path")?,
        )?;
        if snapshot::file_digest(&path)? != artifact["digest"]
            || Some(fs::metadata(&path)?.len()) != artifact["size"].as_u64()
        {
            bail!("OCI index input content changed");
        }
        let verified = crate::oci::verify(&path)?;
        verified.require_target(&input.platform)?;
        if artifact["ociDigest"] != verified.digest {
            bail!("OCI index input publication identity changed");
        }
        for (platform, digest) in verified.images {
            if images.insert(platform, digest).is_some() {
                bail!("duplicate OCI index input platform");
            }
        }
        paths.push(path);
    }
    Ok((paths, images))
}

pub(super) fn execute(
    action: &Value,
    plan: &Value,
    bundle: &Path,
    artifacts: &[Value],
    actions: &[Value],
) -> Result<Value> {
    let assembly: Assembly = serde_json::from_value(action["extensions"][KEY].clone())?;
    for member in &assembly.members {
        if !actions
            .iter()
            .any(|a| a["id"] == member.producer && a["status"] == "succeeded")
        {
            bail!("OCI index prerequisite did not succeed");
        }
    }
    let (paths, images) = expected(&assembly, artifacts, bundle)?;
    let inputs: Vec<_> = assembly
        .members
        .iter()
        .zip(&paths)
        .map(|(member, path)| {
            let artifact = artifacts
                .iter()
                .find(|a| a["id"] == member.artifact)
                .unwrap();
            Ok(crate::oci::Input {
                path,
                platform: &member.platform,
                digest: artifact["ociDigest"]
                    .as_str()
                    .context("missing OCI digest")?,
            })
        })
        .collect::<Result<_>>()?;
    let mut intent = plan["artifacts"]
        .as_array()
        .context("missing artifacts")?
        .iter()
        .find(|a| a["producer"] == action["id"])
        .context("missing OCI index output")?
        .clone();
    let relative = intent["path"].as_str().context("missing index path")?;
    if !snapshot::portable(relative) {
        bail!("invalid OCI index output path");
    }
    let path = bundle.join(relative);
    fs::create_dir_all(path.parent().context("missing index parent")?)?;
    let verified = crate::oci::assemble(&inputs, &path)?;
    verified.require_index(&images)?;
    intent["digest"] = json!(snapshot::file_digest(&path)?);
    intent["size"] = json!(fs::metadata(&path)?.len());
    intent["ociDigest"] = json!(verified.digest);
    Ok(intent)
}

pub(super) fn inspect(
    manifest: &Value,
    item: &Value,
    verified: &crate::oci::Verified,
    bundle: &Path,
) -> Result<()> {
    if !manifest["actions"]
        .as_array()
        .context("missing actions")?
        .iter()
        .any(|a| {
            a["id"] == item["producer"]
                && a["target"] == item["target"]
                && a["status"] == "succeeded"
        })
    {
        bail!("OCI index assembly did not succeed");
    }
    let target = manifest["targets"]
        .as_array()
        .and_then(|targets| targets.iter().find(|t| t["id"] == item["target"]))
        .context("OCI index target missing")?;
    let assembly: Assembly = serde_json::from_value(target["extensions"][KEY].clone())?;
    let artifacts = manifest["artifacts"]
        .as_array()
        .context("missing artifacts")?;
    let (_, expected) = expected(&assembly, artifacts, bundle)?;
    verified.require_index(&expected)?;
    for member in assembly.members {
        if !manifest["actions"]
            .as_array()
            .context("missing actions")?
            .iter()
            .any(|a| a["id"] == member.producer && a["status"] == "succeeded")
        {
            bail!("OCI index includes an unsuccessful producer");
        }
    }
    Ok(())
}
