//! Bounded archive materialization into an empty, caller-owned staging directory.
//! Publication, receipt creation and authorization are separate operations.
use super::{
    access::{self, Directory},
    tree::TreeInspection,
};
use anyhow::{ensure, Context, Result};
mod paths;

use std::{
    io::{self, Read},
    path::Path,
};

const MAX_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_FILE: u64 = 1024 * 1024 * 1024;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Bounds {
    pub max_entries: u32,
    pub max_bytes: u64,
    pub max_file_bytes: u64,
    pub max_depth: u32,
    pub max_expansion_ratio: u32,
}
impl Default for Bounds {
    fn default() -> Self {
        Self {
            max_entries: 200_000,
            max_bytes: MAX_BYTES,
            max_file_bytes: MAX_FILE,
            max_depth: 64,
            max_expansion_ratio: 200,
        }
    }
}
impl Bounds {
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            (1..=200_000).contains(&self.max_entries)
                && (1..=MAX_BYTES).contains(&self.max_bytes)
                && (1..=MAX_FILE).contains(&self.max_file_bytes)
                && self.max_file_bytes <= self.max_bytes
                && (1..=64).contains(&self.max_depth)
                && (1..=200).contains(&self.max_expansion_ratio),
            "unsupported extraction bounds"
        );
        Ok(())
    }
}

pub(in crate::tools) fn materialize(
    source: &Path,
    staging: &Path,
    digest: &str,
    size: u64,
    gzip: bool,
) -> Result<TreeInspection> {
    super::super::lock::digest(digest)?;
    ensure!(
        (1..=MAX_BYTES).contains(&size),
        "archive size exceeds limit"
    );
    let root = Directory::open(staging)?;
    ensure!(
        root.entries()?.is_empty(),
        "archive staging directory must be empty"
    );
    // Verify while copying into an unnamed private file. Extraction never
    // reopens the source path or trusts bytes modified after verification.
    let source_parent = Directory::open(source.parent().context("archive source has no parent")?)?;
    let mut source = source_parent.file(
        source
            .file_name()
            .and_then(|name| name.to_str())
            .context("archive source name must be UTF-8")?,
    )?;
    let verified = super::blob::snapshot(&mut source, digest, size)?;
    unpack(verified, root, gzip)
}

pub(in crate::tools) fn materialize_blob(
    verified: super::VerifiedBlob,
    staging: &Path,
    gzip: bool,
) -> Result<TreeInspection> {
    let root = Directory::open(staging)?;
    ensure!(
        root.entries()?.is_empty(),
        "archive staging directory must be empty"
    );
    unpack(verified, root, gzip)
}

fn unpack(verified: super::VerifiedBlob, root: Directory, gzip: bool) -> Result<TreeInspection> {
    unpack_layout(verified, root, gzip, None, &Bounds::default())
}

pub(super) fn unpack_layout(
    verified: super::VerifiedBlob,
    root: Directory,
    gzip: bool,
    strip_prefix: Option<&str>,
    bounds: &Bounds,
) -> Result<TreeInspection> {
    bounds.validate()?;
    if let Some(prefix) = strip_prefix {
        access::relative(prefix)?;
    }
    let size = verified.size();
    let verified = verified.into_file()?;
    let reader: Box<dyn Read> = if gzip {
        Box::new(flate2::read::MultiGzDecoder::new(verified))
    } else {
        Box::new(verified)
    };
    let limit = if gzip {
        size.saturating_mul(u64::from(bounds.max_expansion_ratio))
            .min(bounds.max_bytes)
    } else {
        bounds.max_bytes
    };
    extract(reader, root.duplicate()?, limit, strip_prefix, bounds)?;
    super::tree::inspect_directory(&root)
}

struct Bounded<R> {
    reader: R,
    remaining: u64,
}
impl<R: Read> Read for Bounded<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut probe = [0u8; 1];
            return if self.reader.read(&mut probe)? == 0 {
                Ok(0)
            } else {
                Err(io::Error::other("archive expansion limit exceeded"))
            };
        }
        let length = bytes
            .len()
            .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
        let count = self.reader.read(&mut bytes[..length])?;
        self.remaining -= count as u64;
        Ok(count)
    }
}

fn extract(
    reader: Box<dyn Read>,
    root: Directory,
    limit: u64,
    strip_prefix: Option<&str>,
    bounds: &Bounds,
) -> Result<()> {
    let mut archive = tar::Archive::new(Bounded {
        reader,
        remaining: limit,
    });
    let mut paths = paths::Paths::new(bounds, strip_prefix);
    let mut links = Vec::new();
    let mut long_name = None;
    let mut long_link = None;
    let mut bytes = 0u64;
    let mut entries = 0usize;
    // Raw iteration avoids unbounded allocation of GNU/PAX extension bodies by
    // tar's convenience iterator. Only bounded GNU name/link records are admitted.
    for entry in archive.entries()?.raw(true) {
        let mut entry = entry?;
        entries += 1;
        ensure!(
            entries <= bounds.max_entries as usize,
            "archive entry limit exceeded"
        );
        let kind = entry.header().entry_type();
        if kind.is_gnu_longname() || kind.is_gnu_longlink() {
            ensure!(entry.size() <= 16 * 1024, "archive extension exceeds limit");
            let mut value = String::new();
            entry.read_to_string(&mut value)?;
            let value = value.trim_end_matches('\0').to_owned();
            let slot = if kind.is_gnu_longname() {
                &mut long_name
            } else {
                &mut long_link
            };
            ensure!(slot.replace(value).is_none(), "duplicate archive extension");
            continue;
        }
        ensure!(
            kind.is_file() || kind.is_dir() || kind.is_symlink(),
            "archive entry type is not admitted"
        );
        let path = long_name
            .take()
            .map(Ok)
            .unwrap_or_else(|| String::from_utf8(entry.path_bytes().into_owned()))?;
        let Some((path, parent, name)) = paths.destination(
            &root,
            &path,
            kind.is_dir(),
            entry.size() == 0 && long_link.is_none(),
        )?
        else {
            continue;
        };
        if kind.is_file() {
            ensure!(long_link.is_none(), "link extension on regular file");
            ensure!(
                entry.size() <= bounds.max_file_bytes,
                "archive file exceeds limit"
            );
            bytes = bytes
                .checked_add(entry.size())
                .context("archive payload size overflow")?;
            ensure!(bytes <= bounds.max_bytes, "archive payload exceeds limit");
            let mut file = parent.create_file(&name)?;
            let written = io::copy(&mut entry, &mut file)?;
            ensure!(written == entry.size(), "truncated archive file");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = 0o600 | (entry.header().mode()? & 0o111);
                file.set_permissions(std::fs::Permissions::from_mode(mode))?;
            }
            file.sync_all()?;
        } else if kind.is_symlink() {
            ensure!(entry.size() == 0, "symlink entry contains data");
            let target = long_link.take().map(Ok).unwrap_or_else(|| {
                String::from_utf8(
                    entry
                        .link_name_bytes()
                        .context("missing symlink target")?
                        .into_owned(),
                )
                .map_err(anyhow::Error::from)
            })?;
            ensure!(
                !target.is_empty()
                    && target.len() <= 16 * 1024
                    && !target.starts_with('/')
                    && !target.contains(['\\', ':', '\0']),
                "invalid archive link target"
            );
            paths.charge(target.len())?;
            links.push((path.to_owned(), target));
        } else {
            ensure!(
                long_link.is_none() && entry.size() == 0,
                "invalid directory metadata"
            );
        }
    }
    ensure!(
        long_name.is_none() && long_link.is_none(),
        "orphan archive extension"
    );
    // Finish decoding to validate gzip trailers and apply the expansion bound
    // even to ignored bytes after tar's end marker.
    io::copy(&mut archive.into_inner(), &mut io::sink())?;
    for (path, target) in links {
        let mut directory = root.duplicate()?;
        let mut parts = path.split('/').peekable();
        while let Some(name) = parts.next() {
            if parts.peek().is_some() {
                directory = directory.child(name)?;
            } else {
                directory.create_link(name, &target)?;
            }
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn anchored_writer_does_not_follow_replaced_parent_or_existing_link() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        // macOS temporary paths can contain the /var -> /private/var alias.
        // Resolve the fixture parent before exercising the no-follow boundary.
        let parent = temporary.path().canonicalize().unwrap();
        let root = parent.join("root");
        let moved = parent.join("moved");
        let outside = parent.join("outside");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&outside).unwrap();
        let directory = Directory::open(&root).unwrap();
        std::fs::rename(&root, &moved).unwrap();
        symlink(&outside, &root).unwrap();
        directory
            .create_file("payload")
            .unwrap()
            .write_all(b"owned")
            .unwrap();
        assert_eq!(std::fs::read(moved.join("payload")).unwrap(), b"owned");
        assert!(!outside.join("payload").exists());
        symlink(outside.join("target"), moved.join("link")).unwrap();
        assert!(directory.create_file("link").is_err());
        assert!(directory.create_directory("link").is_err());
        assert!(!outside.join("target").exists());
    }
}
