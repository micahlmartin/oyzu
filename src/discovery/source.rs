//! Bounded, read-once metadata inputs for static ecosystem discovery.
use super::detectors::Evidence;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

struct Document {
    bytes: Vec<u8>,
    digest: String,
}

/// Reads only explicitly declared small metadata files, once per discovery scope.
pub(crate) struct Source {
    documents: BTreeMap<String, Document>,
}

impl Source {
    pub fn read(root: &Path, paths: &[&str]) -> Result<Self> {
        let source = Self::read_binary(root, paths)?;
        for (path, document) in &source.documents {
            std::str::from_utf8(&document.bytes)
                .with_context(|| format!("non-UTF8 discovery metadata {path}"))?;
        }
        Ok(source)
    }
    /// Same bounded, contained capture as text metadata, without decoding bytes.
    /// Format interpretation remains with the requesting detector.
    pub fn read_binary(root: &Path, paths: &[&str]) -> Result<Self> {
        if paths.len() > 128 {
            bail!("too many discovery metadata inputs");
        }
        let mut documents = BTreeMap::new();
        for relative in paths {
            if !crate::snapshot::portable(relative) {
                bail!("invalid discovery input path");
            }
            let mut path = root.to_path_buf();
            let mut missing = false;
            for part in relative.split('/') {
                path.push(part);
                match fs::symlink_metadata(&path) {
                    Ok(meta) if meta.file_type().is_symlink() => {
                        bail!("discovery input is a symlink: {relative}")
                    }
                    Ok(_) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        missing = true;
                        break;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            if missing {
                continue;
            }
            if !fs::symlink_metadata(&path)?.is_file() {
                bail!("discovery input is not a file: {relative}");
            }
            let input = fs::File::open(&path)?;
            if !input.metadata()?.is_file() {
                bail!("discovery input is not a file: {relative}");
            }
            let mut bytes = Vec::new();
            input.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
            if bytes.len() > 4 * 1024 * 1024 {
                bail!("discovery metadata exceeds 4 MiB: {relative}");
            }
            let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
            documents.insert(relative.to_string(), Document { bytes, digest });
        }
        Ok(Self { documents })
    }
    pub fn text(&self, path: &str) -> Option<&str> {
        self.bytes(path)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
    }
    pub fn bytes(&self, path: &str) -> Option<&[u8]> {
        self.documents.get(path).map(|d| d.bytes.as_slice())
    }
    pub fn evidence(&self, path: &str, location: &str) -> Option<Evidence> {
        self.documents.get(path).map(|d| Evidence {
            path: path.into(),
            digest: d.digest.clone(),
            location: location.into(),
        })
    }
}
