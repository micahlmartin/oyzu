//! Assemble verified image archives into one complete, deterministic OCI index.
//! No extraction, image execution, registry lookup or project code is involved.
use super::{archive::Layout, Index, Manifest, Verified, INDEX};
use anyhow::{bail, Context, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};

pub(crate) struct Input<'a> {
    pub path: &'a Path,
    pub platform: &'a crate::platform::Platform,
    pub digest: &'a str,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn append<W: Write>(
    tar: &mut tar::Builder<W>,
    name: &str,
    size: u64,
    bytes: impl Read,
) -> Result<()> {
    let mut header = tar::Header::new_ustar();
    header.set_size(size);
    header.set_mode(0o644);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    tar.append_data(&mut header, name, bytes)?;
    Ok(())
}

/// Every caller-supplied platform/digest must be present exactly once. Write a
/// private temporary archive, verify its complete closure, then publish without
/// overwriting an existing destination. Inputs and their native blobs stay intact.
pub(crate) fn assemble(inputs: &[Input<'_>], destination: &Path) -> Result<Verified> {
    if inputs.is_empty() || inputs.len() > 256 {
        bail!("OCI index assembly requires 1 to 256 platform images");
    }
    let mut expected = BTreeMap::new();
    let mut descriptors = BTreeMap::new();
    let mut layouts = Vec::new();
    let mut blobs = BTreeMap::new();
    for (index, input) in inputs.iter().enumerate() {
        let verified = super::verify(input.path)?;
        verified.require_target(input.platform)?;
        if verified.digest != input.digest {
            bail!("OCI index input publication digest changed");
        }
        let platform = verified.platforms.into_iter().next().unwrap();
        if expected
            .insert(platform.clone(), input.digest.to_owned())
            .is_some()
        {
            bail!("duplicate platform in OCI index assembly inputs");
        }
        let mut layout = Layout::read(input.path)?;
        let root: Index = layout.json("index.json")?;
        let mut descriptor = root
            .manifests
            .into_iter()
            .next()
            .context("missing OCI image root")?;
        if descriptor.digest != input.digest {
            bail!("OCI image changed during index assembly");
        }
        let manifest: Manifest = layout.json(&layout.descriptor(&descriptor)?)?;
        for blob in std::iter::once(&descriptor)
            .chain(std::iter::once(&manifest.config))
            .chain(&manifest.layers)
        {
            let name = layout.descriptor(blob)?;
            if let Some((_, size)) = blobs.insert(name, (index, blob.size)) {
                if size != blob.size {
                    bail!("conflicting OCI blob sizes");
                }
            }
        }
        descriptor.platform = Some(platform.clone());
        descriptors.insert(platform, descriptor);
        layouts.push(layout);
    }
    let members: Vec<_> = descriptors.into_values().collect();
    let index =
        serde_json::to_vec(&json!({"schemaVersion":2,"mediaType":INDEX,"manifests":members}))?;
    let index_digest = digest(&index);
    let root = serde_json::to_vec(
        &json!({"schemaVersion":2,"mediaType":INDEX,"manifests":[{"mediaType":INDEX,"digest":index_digest,"size":index.len()}]}),
    )?;
    let layout = br#"{"imageLayoutVersion":"1.0.0"}"#;
    let tar_size = |size: u64| 512 + size.div_ceil(512) * 512;
    let total = blobs.values().map(|(_, size)| tar_size(*size)).sum::<u64>()
        + tar_size(index.len() as u64)
        + tar_size(root.len() as u64)
        + tar_size(layout.len() as u64)
        + 1024;
    if total > super::archive::ARCHIVE_LIMIT {
        bail!("assembled OCI index exceeds 10 GiB");
    }
    let parent = destination
        .parent()
        .context("missing OCI output directory")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut tar = tar::Builder::new(temporary.as_file_mut());
        for (name, (source, size)) in blobs {
            append(&mut tar, &name, size, layouts[source].bytes(&name)?)?;
        }
        append(
            &mut tar,
            &format!("blobs/sha256/{}", &index_digest[7..]),
            index.len() as u64,
            index.as_slice(),
        )?;
        append(&mut tar, "index.json", root.len() as u64, root.as_slice())?;
        append(&mut tar, "oci-layout", layout.len() as u64, &layout[..])?;
        tar.finish()?;
    }
    temporary.as_file().sync_all()?;
    let verified = super::verify(temporary.path())?;
    verified.require_index(&expected)?;
    temporary
        .persist_noclobber(destination)
        .map_err(|error| error.error)?;
    Ok(verified)
}
