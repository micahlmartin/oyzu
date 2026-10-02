use super::{component, FileIdentity, Kind};
use anyhow::{ensure, Context, Result};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Component, Path, PathBuf},
};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

pub(in crate::tools::store) struct Directory {
    path: PathBuf,
    // Keep all ancestors open without FILE_SHARE_DELETE. Path-based Windows
    // opens below cannot be redirected by renaming/replacing a held ancestor.
    handles: Vec<File>,
}

impl Directory {
    pub fn open(path: &Path) -> Result<Self> {
        ensure!(path.is_absolute(), "store root must be absolute");
        let mut directory = Self {
            path: PathBuf::new(),
            handles: Vec::new(),
        };
        for part in path.components() {
            match part {
                Component::Prefix(_) => directory.path.push(part.as_os_str()),
                Component::RootDir => {
                    directory.path.push(part.as_os_str());
                    directory.hold()?;
                }
                Component::Normal(name) => {
                    directory.path.push(name);
                    directory.hold()?;
                }
                _ => anyhow::bail!("store root must have normalized components"),
            }
        }
        Ok(directory)
    }

    fn hold(&mut self) -> Result<()> {
        let handle = open(&self.path)?;
        ensure!(
            handle.metadata()?.is_dir(),
            "store component is not a directory"
        );
        self.handles.push(handle);
        Ok(())
    }

    pub fn child(&self, name: &str) -> Result<Self> {
        component(name)?;
        let mut directory = Self {
            path: self.path.join(name),
            handles: self
                .handles
                .iter()
                .map(File::try_clone)
                .collect::<std::io::Result<_>>()?,
        };
        directory.hold()?;
        Ok(directory)
    }

    pub fn executable(&self) -> Result<u32> {
        Ok(0)
    }

    pub fn file(&self, name: &str) -> Result<File> {
        component(name)?;
        let file = open(&self.path.join(name))?;
        ensure!(file.metadata()?.is_file(), "payload is not a regular file");
        Ok(file)
    }

    pub fn entries(&self) -> Result<Vec<(String, Kind)>> {
        let mut result = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("payload name must be UTF-8"))?;
            component(&name)?;
            let metadata = fs::symlink_metadata(entry.path())?;
            let kind = if metadata.file_type().is_symlink() {
                Kind::Symlink
            } else if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                anyhow::bail!("non-symlink reparse points are forbidden in payloads")
            } else if metadata.is_dir() {
                Kind::Directory
            } else if metadata.is_file() {
                Kind::File
            } else {
                anyhow::bail!("special files are forbidden in payloads")
            };
            result.push((name, kind));
            ensure!(result.len() <= 200_000, "payload entry limit exceeded");
        }
        result.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(result)
    }

    pub fn link(&self, name: &str) -> Result<OsString> {
        component(name)?;
        Ok(fs::read_link(self.path.join(name))?.into_os_string())
    }
}

fn open(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .context("open no-follow store handle")?;
    ensure!(
        file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0,
        "store handle redirects through a reparse point"
    );
    Ok(file)
}

pub(in crate::tools::store) fn identity(file: &File) -> Result<FileIdentity> {
    let mut information = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // File owns a live handle; the API initializes the output only on success.
    let result =
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) };
    ensure!(result != 0, "cannot read payload file identity");
    let info = unsafe { information.assume_init() };
    ensure!(file.metadata()?.is_file(), "payload is not a regular file");
    Ok(FileIdentity {
        object: (
            u64::from(info.dwVolumeSerialNumber),
            (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        ),
        links: u64::from(info.nNumberOfLinks),
        size: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
        executable: 0,
    })
}
