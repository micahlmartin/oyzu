//! Native configuration locations and protected administrative file access.
use anyhow::{bail, Context, Result};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug)]
pub struct Locations {
    pub machine: PathBuf,
    pub user: PathBuf,
    pub state: PathBuf,
    pub diagnostics: Vec<String>,
}
impl Locations {
    pub fn native() -> Result<Self> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Shell::{
                FOLDERID_LocalAppData, FOLDERID_ProgramData, FOLDERID_RoamingAppData,
            };
            Ok(Self {
                machine: known_folder(&FOLDERID_ProgramData)?.join("Oyzu"),
                user: known_folder(&FOLDERID_RoamingAppData)?.join("Oyzu/config.toml"),
                state: known_folder(&FOLDERID_LocalAppData)?.join("Oyzu/state"),
                diagnostics: vec![],
            })
        }
        #[cfg(not(windows))]
        {
            let home =
                PathBuf::from(std::env::var_os("HOME").context("CONFIG_SCOPE: no home directory")?);
            if !home.is_absolute() {
                bail!("CONFIG_SCOPE: home directory must be absolute");
            }
            #[cfg(target_os = "macos")]
            {
                Ok(Self {
                    machine: "/Library/Application Support/Oyzu".into(),
                    user: home.join("Library/Application Support/Oyzu/config.toml"),
                    state: home.join("Library/Application Support/Oyzu/state"),
                    diagnostics: vec![],
                })
            }
            #[cfg(not(target_os = "macos"))]
            {
                let mut diagnostics = Vec::new();
                let mut xdg = |key: &str, fallback: PathBuf| match std::env::var_os(key) {
                    Some(v) if Path::new(&v).is_absolute() => PathBuf::from(v),
                    Some(_) => {
                        diagnostics.push(format!("{key} must be absolute; using home fallback"));
                        fallback
                    }
                    None => fallback,
                };
                let user = xdg("XDG_CONFIG_HOME", home.join(".config")).join("oyzu/config.toml");
                let state = xdg("XDG_STATE_HOME", home.join(".local/state")).join("oyzu");
                Ok(Self {
                    machine: "/etc/oyzu".into(),
                    user,
                    state,
                    diagnostics,
                })
            }
        }
    }
}
#[cfg(windows)]
fn known_folder(id: &windows_sys::core::GUID) -> Result<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    let mut pointer = std::ptr::null_mut();
    // The API allocates a NUL-terminated UTF-16 path owned by the COM allocator.
    unsafe {
        if windows_sys::Win32::UI::Shell::SHGetKnownFolderPath(
            id,
            0,
            std::ptr::null_mut(),
            &mut pointer,
        ) < 0
        {
            bail!("CONFIG_SCOPE: known folder lookup failed");
        }
        let mut len = 0;
        while *pointer.add(len) != 0 {
            len += 1;
        }
        let path = PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(
            pointer, len,
        )));
        windows_sys::Win32::System::Com::CoTaskMemFree(pointer.cast());
        Ok(path)
    }
}
pub fn protected_read(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => bail!("POLICY_INVALID: administrative record is inaccessible"),
    }
    for ancestor in path.ancestors() {
        check_protection(ancestor)?;
    }
    let mut file =
        fs::File::open(path).context("POLICY_INVALID: administrative record is unreadable")?;
    let before = file.metadata()?;
    if before.len() > 1024 * 1024 {
        bail!("CONFIG_LIMIT: administrative record exceeds 1 MiB");
    }
    check_handle(&file)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        bail!("CONFIG_LIMIT: administrative record exceeds 1 MiB");
    }
    // Re-open and compare identity after reading; checked bytes belong to the checked handle.
    let after = fs::File::open(path)?;
    check_handle(&after)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let after = after.metadata()?;
        if before.dev() != after.dev() || before.ino() != after.ino() {
            bail!("POLICY_INVALID: administrative record was replaced");
        }
    }
    #[cfg(windows)]
    {
        if file_identity(&file)? != file_identity(&after)? {
            bail!("POLICY_INVALID: administrative record was replaced");
        }
    }
    Ok(Some(bytes))
}
fn check_protection(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        bail!("POLICY_INVALID: administrative path redirects through a link");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            bail!("POLICY_INVALID: administrative path is not root protected");
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        if metadata.file_attributes() & 0x400 != 0 {
            bail!("POLICY_INVALID: administrative reparse point");
        }
        let file = fs::OpenOptions::new()
            .access_mode(0x20000)
            .custom_flags(0x02000000 | 0x00200000)
            .open(path)?;
        check_handle(&file)?;
    }
    Ok(())
}
#[cfg(unix)]
fn check_handle(file: &fs::File) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let m = file.metadata()?;
    if m.uid() != 0 || m.mode() & 0o022 != 0 {
        bail!("POLICY_INVALID: insecure administrative file");
    }
    Ok(())
}
#[cfg(windows)]
fn file_identity(file: &fs::File) -> Result<(u32, u32, u32)> {
    use std::os::windows::io::AsRawHandle;
    unsafe {
        let mut info = std::mem::zeroed();
        if windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle(
            file.as_raw_handle(),
            &mut info,
        ) == 0
        {
            bail!("POLICY_INVALID: cannot establish file identity");
        }
        Ok((
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
        ))
    }
}
#[cfg(windows)]
fn check_handle(file: &fs::File) -> Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{Authorization::*, *},
    };
    unsafe {
        let mut owner = std::ptr::null_mut();
        let mut dacl = std::ptr::null_mut();
        let mut descriptor = std::ptr::null_mut();
        if GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            std::ptr::null_mut(),
            &mut dacl,
            std::ptr::null_mut(),
            &mut descriptor,
        ) != 0
        {
            bail!("POLICY_INVALID: cannot inspect administrative ACL");
        }
        let trusted_installer: Vec<u16> =
            "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464"
                .encode_utf16()
                .chain(Some(0))
                .collect();
        let mut installer = std::ptr::null_mut();
        if ConvertStringSidToSidW(trusted_installer.as_ptr(), &mut installer) == 0 {
            LocalFree(descriptor);
            bail!("POLICY_INVALID: cannot construct trusted service identity");
        }
        let trusted = |sid: PSID| {
            IsWellKnownSid(sid, WinLocalSystemSid) != 0
                || IsWellKnownSid(sid, WinBuiltinAdministratorsSid) != 0
                || EqualSid(sid, installer) != 0
        };
        let mask = if file.metadata()?.is_dir() {
            0x500d0152
        } else {
            0x500d0156
        };
        let result = (|| -> Result<()> {
            if owner.is_null() || !trusted(owner) || dacl.is_null() {
                bail!("POLICY_INVALID: administrative owner or ACL is not protected");
            }
            for index in 0..(*dacl).AceCount as u32 {
                let mut ace = std::ptr::null_mut();
                if GetAce(dacl, index, &mut ace) == 0 {
                    bail!("POLICY_INVALID: cannot inspect ACL entry");
                }
                let header = &*(ace as *const ACE_HEADER);
                if header.AceFlags & 0x08 != 0 {
                    continue;
                } // inherit-only ACE does not apply to this object.
                if header.AceType == 0 {
                    let entry = &*(ace as *const ACCESS_ALLOWED_ACE);
                    let sid = std::ptr::addr_of!(entry.SidStart) as PSID;
                    // Write data/append, attributes, delete child, DELETE, WRITE_DAC,
                    // WRITE_OWNER, GENERIC_WRITE and GENERIC_ALL permit replacement.
                    if entry.Mask & mask != 0 && !trusted(sid) {
                        bail!("POLICY_INVALID: ordinary principal can modify administrative path");
                    }
                } else if header.AceType != 1 {
                    bail!("POLICY_INVALID: unsupported administrative ACL entry");
                }
            }
            Ok(())
        })();
        LocalFree(installer);
        LocalFree(descriptor);
        result
    }
}
