use anyhow::{ensure, Result};
use std::fs::File;

#[cfg(unix)]
#[path = "access/unix.rs"]
mod native;
#[cfg(windows)]
#[path = "access/windows.rs"]
mod native;
pub(super) use native::Directory;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Kind {
    File,
    Directory,
    Symlink,
}

pub(super) struct FileIdentity {
    pub object: (u64, u64),
    pub links: u64,
    pub size: u64,
    pub executable: u32,
}

pub(super) fn identity(file: &File) -> Result<FileIdentity> {
    native::identity(file)
}

/// Portable payload names exclude Windows aliases on every host, so a verified
/// Unix archive cannot acquire a different meaning when finalized on Windows.
pub(super) fn component(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name != "." && name != "..",
        "invalid payload path component"
    );
    ensure!(
        !name
            .chars()
            .any(|c| c.is_control() || "\\/:<>\"|?*".contains(c)),
        "nonportable payload path component"
    );
    ensure!(
        !name.ends_with([' ', '.']),
        "payload path has a Windows trailing alias"
    );
    let stem = name.split('.').next().unwrap().to_ascii_uppercase();
    ensure!(
        !matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) && !["COM", "LPT"]
            .iter()
            .any(
                |prefix| stem.strip_prefix(prefix).is_some_and(|suffix| matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                ))
            ),
        "payload path is a Windows device alias"
    );
    Ok(())
}

pub(super) fn relative(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty() && path.split('/').count() <= 64,
        "invalid payload path depth"
    );
    for name in path.split('/') {
        component(name)?;
    }
    Ok(())
}
