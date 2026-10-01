//! Darwin extended ACL validation, in addition to root ownership and POSIX modes.
//! Calls the native ACL API; no upstream implementation is incorporated.
use anyhow::{bail, Result};
use std::{
    ffi::{c_int, c_void},
    fs::File,
    os::fd::AsRawFd,
    ptr,
};

extern "C" {
    fn acl_get_fd_np(fd: c_int, kind: c_int) -> *mut c_void;
    fn acl_valid(acl: *mut c_void) -> c_int;
    fn acl_free(value: *mut c_void) -> c_int;
    fn acl_get_entry(acl: *mut c_void, index: c_int, entry: *mut *mut c_void) -> c_int;
    fn acl_get_tag_type(entry: *mut c_void, tag: *mut c_int) -> c_int;
    fn acl_get_permset_mask_np(entry: *mut c_void, mask: *mut u64) -> c_int;
    fn acl_get_flagset_np(entry: *mut c_void, flags: *mut *mut c_void) -> c_int;
    fn acl_get_flag_np(flags: *mut c_void, flag: c_int) -> c_int;
    fn acl_get_qualifier(entry: *mut c_void) -> *mut c_void;
    fn mbr_uid_to_uuid(uid: u32, uuid: *mut u8) -> c_int;
}
struct Allocation(*mut c_void);
impl Drop for Allocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                acl_free(self.0);
            }
        }
    }
}
pub(super) fn check_acl(file: &File) -> Result<()> {
    // ACL_TYPE_EXTENDED; the descriptor pins the object being checked.
    let acl = Allocation(unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) });
    if acl.0.is_null() {
        bail!("POLICY_INVALID: cannot inspect administrative extended ACL");
    }
    if unsafe { acl_valid(acl.0) } != 0 {
        bail!("POLICY_INVALID: malformed extended ACL");
    }
    let mut root_uuid = [0u8; 16];
    if unsafe { mbr_uid_to_uuid(0, root_uuid.as_mut_ptr()) } != 0 {
        bail!("POLICY_INVALID: cannot resolve root ACL identity");
    }
    // Write/append/delete, delete-child, attributes, security and ownership.
    let mutation = 0x3574u64;
    for index in 0..=128 {
        let mut entry = ptr::null_mut();
        if unsafe { acl_get_entry(acl.0, index, &mut entry) } != 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(22) {
                return Ok(());
            }
            bail!("POLICY_INVALID: cannot enumerate extended ACL");
        }
        if index == 128 {
            bail!("POLICY_INVALID: extended ACL exceeds supported limit");
        }
        let mut tag = 0;
        let mut mask = 0;
        let mut flags = ptr::null_mut();
        if unsafe { acl_get_tag_type(entry, &mut tag) } != 0
            || unsafe { acl_get_permset_mask_np(entry, &mut mask) } != 0
            || unsafe { acl_get_flagset_np(entry, &mut flags) } != 0
        {
            bail!("POLICY_INVALID: cannot inspect extended ACL entry");
        }
        let inherit_only = unsafe { acl_get_flag_np(flags, 0x100) };
        if inherit_only < 0 {
            bail!("POLICY_INVALID: cannot inspect ACL inheritance");
        }
        if inherit_only == 1 || tag == 2 {
            continue;
        }
        if tag != 1 || mask & !0x103ffe != 0 {
            bail!("POLICY_INVALID: unsupported extended ACL entry");
        }
        if mask & mutation != 0 {
            let principal = Allocation(unsafe { acl_get_qualifier(entry) });
            if principal.0.is_null() {
                bail!("POLICY_INVALID: missing extended ACL identity");
            }
            // Native ACL qualifiers are UUIDs owned by acl_free.
            let uuid = unsafe { std::slice::from_raw_parts(principal.0.cast::<u8>(), 16) };
            if uuid != root_uuid {
                bail!("POLICY_INVALID: non-root extended ACL permits administrative mutation");
            }
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    #[test]
    fn extended_grants_are_checked_even_with_private_posix_modes() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let chmod = |args: &[&str]| {
            assert!(Command::new("/bin/chmod")
                .args(args)
                .arg(file.path())
                .status()
                .unwrap()
                .success());
        };
        chmod(&["-N"]);
        chmod(&["600"]);
        check_acl(file.as_file()).unwrap();
        chmod(&["+a", "everyone allow read"]);
        check_acl(file.as_file()).unwrap();
        chmod(&["+a", "everyone allow write"]);
        assert!(check_acl(file.as_file()).is_err());
        chmod(&["-N"]);
        chmod(&["+a", "everyone deny delete"]);
        check_acl(file.as_file()).unwrap();
    }
}
