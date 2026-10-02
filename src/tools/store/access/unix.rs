use super::{component, FileIdentity, Kind};
use anyhow::{bail, ensure, Context, Result};
use std::{
    ffi::{CStr, CString, OsString},
    fs::File,
    io,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStringExt, fs::MetadataExt},
    },
    path::{Component, Path},
};

pub(in crate::tools::store) struct Directory {
    handle: File,
}

impl Directory {
    pub fn open(path: &Path) -> Result<Self> {
        ensure!(path.is_absolute(), "store root must be absolute");
        let fd = unsafe {
            libc::open(
                c"/".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        let mut directory = Self { handle: owned(fd)? };
        for part in path.components() {
            match part {
                Component::RootDir => {}
                Component::Normal(name) => {
                    directory =
                        directory.child(name.to_str().context("store root must be UTF-8")?)?
                }
                _ => bail!("store root must have normalized components"),
            }
        }
        Ok(directory)
    }

    pub fn child(&self, name: &str) -> Result<Self> {
        // Ancestors of the trusted root need Unix-native spelling, while tree
        // members themselves are separately checked for portable names.
        let name = native_name(name)?;
        let fd = unsafe {
            libc::openat(
                self.handle.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        Ok(Self { handle: owned(fd)? })
    }

    pub fn executable(&self) -> Result<u32> {
        Ok(self.handle.metadata()?.mode() & 0o111)
    }

    pub fn file(&self, name: &str) -> Result<File> {
        component(name)?;
        let name = native_name(name)?;
        let fd = unsafe {
            libc::openat(
                self.handle.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        let file = owned(fd)?;
        ensure!(file.metadata()?.is_file(), "payload is not a regular file");
        Ok(file)
    }

    pub fn entries(&self) -> Result<Vec<(String, Kind)>> {
        // openat(".") gives readdir its own cursor; dup would share offsets with
        // the directory handle and make repeated scans depend on earlier scans.
        let fd = unsafe {
            libc::openat(
                self.handle.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error().into());
        }
        let pointer = unsafe { libc::fdopendir(fd) };
        if pointer.is_null() {
            let error = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(error.into());
        }
        struct Stream(*mut libc::DIR);
        impl Drop for Stream {
            fn drop(&mut self) {
                unsafe {
                    libc::closedir(self.0);
                }
            }
        }
        let stream = Stream(pointer);
        let mut result = Vec::new();
        loop {
            set_errno(0);
            let entry = unsafe { libc::readdir(stream.0) };
            if entry.is_null() {
                let error = io::Error::last_os_error();
                ensure!(
                    error.raw_os_error() == Some(0),
                    "payload directory enumeration failed: {error}"
                );
                break;
            }
            let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            let name = std::str::from_utf8(bytes).context("payload name must be UTF-8")?;
            component(name)?;
            let cname = native_name(name)?;
            let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
            let rc = unsafe {
                libc::fstatat(
                    self.handle.as_raw_fd(),
                    cname.as_ptr(),
                    stat.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            };
            if rc != 0 {
                return Err(io::Error::last_os_error().into());
            }
            let mode = unsafe { stat.assume_init() }.st_mode & libc::S_IFMT;
            let kind = match mode {
                libc::S_IFREG => Kind::File,
                libc::S_IFDIR => Kind::Directory,
                libc::S_IFLNK => Kind::Symlink,
                _ => bail!("special files are forbidden in payloads"),
            };
            result.push((name.to_owned(), kind));
            ensure!(result.len() <= 200_000, "payload entry limit exceeded");
        }
        result.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(result)
    }

    pub fn link(&self, name: &str) -> Result<OsString> {
        component(name)?;
        let name = native_name(name)?;
        let mut bytes = vec![0u8; 16 * 1024];
        let length = unsafe {
            libc::readlinkat(
                self.handle.as_raw_fd(),
                name.as_ptr(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
            )
        };
        if length < 0 {
            return Err(io::Error::last_os_error().into());
        }
        ensure!(
            (length as usize) < bytes.len(),
            "payload symlink target is too long"
        );
        bytes.truncate(length as usize);
        Ok(OsString::from_vec(bytes))
    }
}

fn native_name(name: &str) -> Result<CString> {
    ensure!(
        !name.is_empty() && name != "." && name != ".." && !name.contains('/'),
        "invalid directory component"
    );
    Ok(CString::new(name)?)
}

fn owned(fd: i32) -> Result<File> {
    if fd < 0 {
        return Err(io::Error::last_os_error().into());
    }
    // Successful native open transfers one descriptor, owned exclusively here.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn set_errno(value: i32) {
    #[cfg(target_os = "linux")]
    unsafe {
        *libc::__errno_location() = value;
    }
    #[cfg(target_os = "macos")]
    unsafe {
        *libc::__error() = value;
    }
}

pub(in crate::tools::store) fn identity(file: &File) -> Result<FileIdentity> {
    let metadata = file.metadata()?;
    ensure!(metadata.is_file(), "payload is not a regular file");
    Ok(FileIdentity {
        object: (metadata.dev(), metadata.ino()),
        links: metadata.nlink(),
        size: metadata.len(),
        executable: metadata.mode() & 0o111,
    })
}
