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
    pub fn remove_file(&self, name: &str) -> Result<()> {
        component(name)?;
        // All ancestors remain held without FILE_SHARE_DELETE. This removes a
        // single owned temporary entry and never traverses a child directory.
        fs::remove_file(self.path.join(name))?;
        Ok(())
    }

    pub fn lock_file(&self, name: &str) -> Result<File> {
        component(name)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(self.path.join(name))?;
        ensure!(
            file.metadata()?.is_file()
                && file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
                && identity(&file)?.links == 1,
            "store lock must be a regular file without links"
        );
        Ok(file)
    }

    pub fn sync_file(&self, name: &str) -> Result<()> {
        component(name)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(self.path.join(name))?;
        ensure!(
            file.metadata()?.is_file()
                && file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0,
            "cannot sync redirected payload file"
        );
        file.sync_all()?;
        Ok(())
    }
    // Windows has no portable directory fsync; publication requests write-through
    // and all regular payload/receipt files are flushed before moving.
    pub fn sync(&self) -> Result<()> {
        Ok(())
    }

    pub fn publish(&self, name: &str, destination: &Self, target: &str) -> Result<()> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
        component(name)?;
        component(target)?;
        let from: Vec<u16> = self
            .path
            .join(name)
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let to: Vec<u16> = destination
            .path
            .join(target)
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // No REPLACE_EXISTING or COPY_ALLOWED: collisions and cross-volume moves fail.
        let result = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) };
        ensure!(
            result != 0,
            "atomic store publication failed: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }

    pub fn duplicate(&self) -> Result<Self> {
        Ok(Self {
            path: self.path.clone(),
            handles: self
                .handles
                .iter()
                .map(File::try_clone)
                .collect::<std::io::Result<_>>()?,
        })
    }

    pub fn create_directory(&self, name: &str) -> Result<Self> {
        component(name)?;
        if let Err(error) = fs::create_dir(self.path.join(name)) {
            ensure!(
                error.kind() == std::io::ErrorKind::AlreadyExists,
                "create payload directory: {error}"
            );
        }
        self.child(name)
    }

    pub fn create_file(&self, name: &str) -> Result<File> {
        component(name)?;
        Ok(OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(self.path.join(name))?)
    }

    pub fn create_link(&self, _name: &str, _target: &str) -> Result<()> {
        anyhow::bail!("archive symlink layout is not yet admitted on Windows")
    }

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
