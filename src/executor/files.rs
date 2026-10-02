//! Contained input reads and exclusive output creation for engine-owned actions.
use anyhow::{bail, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn output_file(root: &Path, relative: &str) -> Result<fs::File> {
    if !crate::snapshot::portable(relative) {
        bail!("invalid engine output path");
    }
    let parts: Vec<_> = relative.split('/').collect();
    let mut path = root.to_path_buf();
    for part in &parts[..parts.len() - 1] {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if !metadata.file_type().is_symlink() && metadata.is_dir() => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&path)?,
            _ => bail!("unsafe engine output parent"),
        }
    }
    path.push(parts.last().unwrap());
    Ok(fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?)
}

pub(super) fn file(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = input(root, relative)?;
    if !fs::metadata(&path)?.is_file() {
        bail!("engine input is not a regular file");
    }
    Ok(path)
}

/// Resolve an existing contained input without permitting linked parents.
/// The caller owns the allowed terminal kind (file or complete tree).
pub(super) fn input(root: &Path, relative: &str) -> Result<PathBuf> {
    if !crate::snapshot::portable(relative) {
        bail!("invalid captured input path");
    }
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("engine input is a symlink");
        }
    }
    Ok(path)
}
