//! Verify a self-contained OCI layout tar without extracting it or using a registry.
mod archive;
#[cfg(test)]
mod tests;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

const INDEX: &str = "application/vnd.oci.image.index.v1+json";
const MANIFEST: &str = "application/vnd.oci.image.manifest.v1+json";
const CONFIG: &str = "application/vnd.oci.image.config.v1+json";
const LAYER: &str = "application/vnd.oci.image.layer.v1.tar";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Platform {
    pub os: String,
    pub architecture: String,
    pub variant: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Descriptor {
    media_type: String,
    digest: String,
    size: u64,
    platform: Option<Platform>,
    data: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Index {
    schema_version: u32,
    media_type: Option<String>,
    manifests: Vec<Descriptor>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    media_type: String,
    config: Descriptor,
    layers: Vec<Descriptor>,
}

#[derive(Deserialize)]
struct Configuration {
    #[serde(flatten)]
    platform: Platform,
    rootfs: RootFs,
    config: Option<serde_json::Value>,
}

fn validate_runtime(config: &serde_json::Value) -> Result<()> {
    if !config.is_object() {
        bail!("OCI runtime configuration must be an object");
    }
    // Optional OCI fields may be null. Validate native field types without
    // inventing runtime behavior or rejecting vendor extension fields.
    for field in ["Entrypoint", "Cmd", "Env"] {
        if let Some(value) = config.get(field).filter(|v| !v.is_null()) {
            if !value
                .as_array()
                .is_some_and(|a| a.iter().all(|v| v.is_string()))
            {
                bail!("OCI {field} must be an array of strings");
            }
        }
    }
    for field in ["User", "WorkingDir", "StopSignal"] {
        if config
            .get(field)
            .is_some_and(|v| !v.is_null() && !v.is_string())
        {
            bail!("OCI {field} must be a string");
        }
    }
    if let Some(labels) = config.get("Labels").filter(|v| !v.is_null()) {
        if !labels
            .as_object()
            .is_some_and(|m| m.values().all(|v| v.is_string()))
        {
            bail!("OCI Labels must be a string map");
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct RootFs {
    #[serde(rename = "type")]
    kind: String,
    diff_ids: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct Verified {
    pub digest: String,
    pub kind: &'static str,
    pub platforms: BTreeSet<Platform>,
}

impl Verified {
    /// A platform-specific image must match its planned artifact target, which
    /// need not be the platform of the worker that assembled it.
    pub(crate) fn require_target(&self, target: &crate::platform::Platform) -> Result<()> {
        if self.kind != "oci-image"
            || self.platforms.len() != 1
            || !self
                .platforms
                .iter()
                .all(|p| p.os == target.os() && p.architecture == target.arch())
        {
            bail!("image platform does not match planned {target}");
        }
        Ok(())
    }
}

/// The layout's single root descriptor is the publication identity. An image
/// index may contain multiple platform manifests; a partial closure is invalid.
pub(crate) fn verify(path: &Path) -> Result<Verified> {
    let mut archive = archive::Layout::read(path)?;
    let layout: serde_json::Value = archive.json("oci-layout")?;
    if layout["imageLayoutVersion"] != "1.0.0" {
        bail!("unsupported OCI layout version");
    }
    let index: Index = archive.json("index.json")?;
    if index.schema_version != 2
        || index.media_type.as_deref().is_some_and(|m| m != INDEX)
        || index.manifests.len() != 1
    {
        bail!("OCI archive must have exactly one image or index root");
    }
    let root = &index.manifests[0];
    let kind = match root.media_type.as_str() {
        MANIFEST => "oci-image",
        INDEX => "oci-index",
        _ => bail!("unsupported OCI root media type"),
    };
    let mut budget = 1024;
    let platforms = verify_node(&mut archive, root, 0, &mut budget)?;
    Ok(Verified {
        digest: root.digest.clone(),
        kind,
        platforms,
    })
}

fn verify_node(
    archive: &mut archive::Layout,
    descriptor: &Descriptor,
    depth: usize,
    budget: &mut usize,
) -> Result<BTreeSet<Platform>> {
    if depth > 8 || *budget == 0 {
        bail!("OCI descriptor graph exceeds verification limits");
    }
    *budget -= 1;
    let name = archive.descriptor(descriptor)?;
    match descriptor.media_type.as_str() {
        INDEX => {
            let index: Index = archive.json(&name)?;
            if index.schema_version != 2
                || index.media_type.as_deref() != Some(INDEX)
                || index.manifests.is_empty()
            {
                bail!("invalid or empty OCI image index");
            }
            let mut platforms = BTreeSet::new();
            for child in &index.manifests {
                for platform in verify_node(archive, child, depth + 1, budget)? {
                    if !platforms.insert(platform) {
                        bail!("ambiguous duplicate platform in OCI index");
                    }
                }
            }
            if descriptor.platform.is_some() {
                bail!("platform constraint on nested OCI index is unsupported");
            }
            Ok(platforms)
        }
        MANIFEST => {
            let manifest: Manifest = archive.json(&name)?;
            if manifest.schema_version != 2
                || manifest.media_type != MANIFEST
                || manifest.config.media_type != CONFIG
            {
                bail!("invalid OCI image manifest or configuration type");
            }
            let config_name = archive.descriptor(&manifest.config)?;
            let config: Configuration = archive.json(&config_name)?;
            if let Some(runtime) = &config.config {
                validate_runtime(runtime)?;
            }
            if config.platform.os.is_empty()
                || config.platform.architecture.is_empty()
                || config.rootfs.kind != "layers"
                || config.rootfs.diff_ids.len() != manifest.layers.len()
            {
                bail!("invalid OCI image configuration");
            }
            if descriptor
                .platform
                .as_ref()
                .is_some_and(|p| p != &config.platform)
            {
                bail!("OCI descriptor platform does not match image configuration");
            }
            for (layer, diff_id) in manifest.layers.iter().zip(&config.rootfs.diff_ids) {
                let layer_name = archive.descriptor(layer)?;
                let gzip = match layer.media_type.as_str() {
                    LAYER => false,
                    "application/vnd.oci.image.layer.v1.tar+gzip" => true,
                    _ => bail!("unsupported OCI layer encoding: {}", layer.media_type),
                };
                let actual = archive.layer_digest(&layer_name, gzip)?;
                if &actual != diff_id {
                    bail!("OCI uncompressed layer digest mismatch");
                }
            }
            Ok(BTreeSet::from([config.platform]))
        }
        _ => bail!("unsupported OCI descriptor media type"),
    }
}

fn sha256(value: &str) -> Result<&str> {
    let hex = value
        .strip_prefix("sha256:")
        .context("OCI requires sha256 identities")?;
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        bail!("invalid OCI sha256 identity");
    }
    Ok(hex)
}
