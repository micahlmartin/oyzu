//! Owns the Windows image and ancestor pin for native worker creation.
//! Production selects only the running executable; path admission stays private.
use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::PathBuf,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_READ, FILE_SHARE_WRITE,
};

pub(super) struct ImagePin {
    pub(super) path: PathBuf,
    _file: File,
    _parents: Vec<File>,
}

impl ImagePin {
    pub(super) fn current(expected: &str) -> Result<Self> {
        crate::tools::lock::digest(expected)?;
        let path = std::env::current_exe()?.canonicalize()?;
        Self::open(path, expected)
    }

    fn open(path: PathBuf, expected: &str) -> Result<Self> {
        let mut parents = Vec::new();
        let ancestors: Vec<_> = path
            .parent()
            .context("TOOL_WORKER_IMAGE_INVALID")?
            .ancestors()
            .collect();
        for directory in ancestors.into_iter().rev() {
            let handle = OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                .open(directory)?;
            let metadata = handle.metadata()?;
            ensure!(
                metadata.is_dir() && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0,
                "TOOL_WORKER_IMAGE_PARENT_INVALID"
            );
            parents.push(handle);
        }
        let mut file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&path)?;
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file()
                && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
                && metadata.len() <= 512 * 1024 * 1024,
            "TOOL_WORKER_IMAGE_INVALID"
        );
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut total = 0;
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            total += count as u64;
            ensure!(total <= metadata.len(), "TOOL_WORKER_IMAGE_CHANGED");
            digest.update(&buffer[..count]);
        }
        ensure!(
            total == metadata.len() && format!("sha256:{:x}", digest.finalize()) == expected,
            "TOOL_WORKER_IMAGE_DIGEST_MISMATCH"
        );
        Ok(Self {
            path,
            _file: file,
            _parents: parents,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn pin_blocks_image_mutation_and_parent_replacement_until_released() {
        let temporary = tempfile::tempdir().unwrap();
        // All attempted file/directory changes target owned disposable fixtures,
        // never the running image, its real release directory or the checkout.
        let directory = temporary.path().join("release");
        fs::create_dir(&directory).unwrap();
        let path = directory.join("worker.exe");
        fs::write(&path, b"admitted image fixture").unwrap();
        let digest = format!("sha256:{:x}", Sha256::digest(b"admitted image fixture"));
        let pinned = ImagePin::open(path.canonicalize().unwrap(), &digest).unwrap();
        let replacement = temporary.path().join("renamed-release");
        assert!(OpenOptions::new().write(true).open(&path).is_err());
        assert!(fs::remove_file(&path).is_err());
        assert!(fs::rename(&path, directory.join("renamed.exe")).is_err());
        assert!(fs::rename(&directory, &replacement).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"admitted image fixture");
        drop(pinned);
        fs::rename(&directory, &replacement).unwrap();
        let path = replacement.join("worker.exe");
        fs::write(&path, b"new release fixture").unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejected_image_releases_file_and_parent_handles() {
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().join("release");
        fs::create_dir(&directory).unwrap();
        let path = directory.join("worker.exe");
        fs::write(&path, b"changed release fixture").unwrap();
        let digest = format!("sha256:{:x}", Sha256::digest(b"expected release fixture"));
        assert_eq!(
            ImagePin::open(path.canonicalize().unwrap(), &digest)
                .err()
                .unwrap()
                .to_string(),
            "TOOL_WORKER_IMAGE_DIGEST_MISMATCH"
        );
        // A failed admission must not leave the update/retry path locked.
        let replacement = temporary.path().join("renamed-release");
        fs::rename(&directory, &replacement).unwrap();
        fs::write(replacement.join("worker.exe"), b"retry fixture").unwrap();
    }
}
