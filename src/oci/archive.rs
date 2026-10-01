use super::{sha256, Descriptor};
use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

const ARCHIVE_LIMIT: u64 = 10 * 1024 * 1024 * 1024;
const JSON_LIMIT: u64 = 16 * 1024 * 1024;
const EXPANDED_LIMIT: u64 = 20 * 1024 * 1024 * 1024;

struct Member {
    offset: u64,
    size: u64,
}

pub(super) struct Layout {
    file: File,
    members: BTreeMap<String, Member>,
    expanded: u64,
}

impl Layout {
    pub fn read(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        if file.metadata()?.len() > ARCHIVE_LIMIT {
            bail!("OCI archive exceeds 10 GiB");
        }
        let mut tar = tar::Archive::new(file);
        let mut members = BTreeMap::new();
        for (index, entry) in tar.entries()?.raw(true).enumerate() {
            if index >= 100000 {
                bail!("OCI archive exceeds entry limit");
            }
            let mut entry = entry?;
            let path = entry.path_bytes();
            let path = std::str::from_utf8(&path).context("non-UTF8 OCI path")?;
            let name = path
                .strip_prefix("./")
                .unwrap_or(path)
                .trim_end_matches('/')
                .to_owned();
            let kind = entry.header().entry_type();
            if kind.is_dir() && ["", ".", "blobs", "blobs/sha256"].contains(&name.as_str()) {
                if entry.size() != 0 {
                    bail!("OCI directory entry has data");
                }
                continue;
            }
            if !kind.is_file() || path.ends_with('/') {
                bail!("OCI layout contains a nonregular entry");
            }
            if name != "oci-layout" && name != "index.json" {
                let hex = name
                    .strip_prefix("blobs/sha256/")
                    .context("unsupported OCI layout path")?;
                sha256(&format!("sha256:{hex}"))?;
            }
            if entry.size() > ARCHIVE_LIMIT {
                bail!("oversized OCI entry");
            }
            if members
                .insert(
                    name.clone(),
                    Member {
                        offset: entry.raw_file_position(),
                        size: entry.size(),
                    },
                )
                .is_some()
            {
                bail!("duplicate OCI archive path {name}");
            }
            let (size, digest) = hash(&mut entry, ARCHIVE_LIMIT)?;
            if size != entry.size() {
                bail!("truncated OCI archive member");
            }
            if let Some(hex) = name.strip_prefix("blobs/sha256/") {
                if digest != format!("sha256:{hex}") {
                    bail!("OCI blob digest mismatch: {name}");
                }
            }
        }
        let mut file = tar.into_inner();
        // Reject concatenated archives or payload following the end marker.
        let mut buffer = [0u8; 65536];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            if buffer[..count].iter().any(|b| *b != 0) {
                bail!("nonzero trailing OCI archive data");
            }
        }
        Ok(Self {
            file,
            members,
            expanded: 0,
        })
    }

    fn bytes(&mut self, name: &str) -> Result<std::io::Take<&mut File>> {
        let member = self
            .members
            .get(name)
            .with_context(|| format!("missing OCI member {name}"))?;
        self.file.seek(SeekFrom::Start(member.offset))?;
        Ok((&mut self.file).take(member.size))
    }

    pub fn json<T: DeserializeOwned>(&mut self, name: &str) -> Result<T> {
        let mut bytes = Vec::new();
        self.bytes(name)?
            .take(JSON_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > JSON_LIMIT {
            bail!("OCI JSON exceeds 16 MiB");
        }
        serde_json::from_slice(&bytes).with_context(|| format!("invalid OCI JSON in {name}"))
    }

    pub fn descriptor(&self, descriptor: &Descriptor) -> Result<String> {
        let hex = sha256(&descriptor.digest)?;
        if descriptor.data.is_some() {
            bail!("inline OCI descriptor data is not supported");
        }
        let name = format!("blobs/sha256/{hex}");
        let member = self
            .members
            .get(&name)
            .context("OCI closure is missing a referenced blob")?;
        if member.size != descriptor.size {
            bail!("OCI descriptor size mismatch");
        }
        Ok(name)
    }

    pub fn layer_digest(&mut self, name: &str, gzip: bool) -> Result<String> {
        let remaining = EXPANDED_LIMIT.saturating_sub(self.expanded);
        let bytes = self.bytes(name)?;
        let (size, digest) = if gzip {
            hash(flate2::read::MultiGzDecoder::new(bytes), remaining)?
        } else {
            hash(bytes, remaining)?
        };
        self.expanded += size;
        Ok(digest)
    }
}

fn hash(mut reader: impl Read, limit: u64) -> Result<(u64, String)> {
    let mut digest = Sha256::new();
    let mut size = 0;
    let mut buffer = [0u8; 65536];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > limit {
            bail!("OCI content exceeds verification byte limit");
        }
        digest.update(&buffer[..count]);
    }
    Ok((size, format!("sha256:{:x}", digest.finalize())))
}
