//! Shared semantic record encoding (OEP-0019).
use anyhow::Result;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub fn digest(domain: &str, value: &Value) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(domain.as_bytes());
    hash.update([0]);
    hash.update(serde_json_canonicalizer::to_vec(value)?);
    Ok(format!("sha256:{:x}", hash.finalize()))
}

pub fn write(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, serde_json_canonicalizer::to_vec(value)?)?;
    Ok(())
}

pub fn read(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
