//! ZIP32 admission before decoder allocation; all writes use anchored paths.
//! Record layouts follow PKWARE APPNOTE 6.3.10 sections 4.3.7–4.3.16.
use super::{paths::Paths, Bounds, Directory};
use anyhow::{ensure, Context, Result};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

const METADATA_LIMIT: u64 = 32 * 1024 * 1024;

struct Entry {
    name: String,
    local: u64,
    compressed: u32,
    size: u32,
    crc: u32,
    flags: u16,
    method: u16,
    mode: u32,
    directory: bool,
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn fixed<const N: usize>(file: &mut File) -> Result<[u8; N]> {
    let mut bytes = [0; N];
    file.read_exact(&mut bytes)
        .context("truncated ZIP record")?;
    Ok(bytes)
}
fn variable(file: &mut File, length: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![0; length];
    file.read_exact(&mut bytes)
        .context("truncated ZIP metadata")?;
    Ok(bytes)
}
fn extras(mut bytes: &[u8]) -> Result<()> {
    while !bytes.is_empty() {
        ensure!(bytes.len() >= 4, "truncated ZIP extra field");
        let tag = u16_at(bytes, 0);
        let size = usize::from(u16_at(bytes, 2));
        // These fields can change sizes, names, or encryption interpretation.
        ensure!(
            !matches!(tag, 1 | 0x7075 | 0x6375 | 0x9901),
            "ZIP extension is not admitted"
        );
        bytes = bytes
            .get(4..4 + size)
            .and_then(|_| bytes.get(4 + size..))
            .context("truncated ZIP extra field")?;
    }
    Ok(())
}

fn preflight(file: &mut File, bounds: &Bounds) -> Result<Vec<Entry>> {
    let length = file.seek(SeekFrom::End(0))?;
    ensure!(length >= 22, "missing ZIP directory");
    let tail_size = length.min(22 + 65535) as usize;
    file.seek(SeekFrom::End(-(tail_size as i64)))?;
    let tail = variable(file, tail_size)?;
    let endings: Vec<_> = (0..=tail.len() - 22)
        .filter(|&i| {
            u32_at(&tail, i) == 0x06054b50
                && i + 22 + usize::from(u16_at(&tail, i + 20)) == tail.len()
        })
        .collect();
    ensure!(endings.len() == 1, "missing or ambiguous ZIP directory");
    let index = endings[0];
    let end = &tail[index..];
    let count = u16_at(end, 10);
    ensure!(
        u16_at(end, 4) == 0 && u16_at(end, 6) == 0 && u16_at(end, 8) == count,
        "multi-volume ZIP is not admitted"
    );
    ensure!(
        count != u16::MAX && u32::from(count) <= bounds.max_entries,
        "ZIP entry limit exceeded"
    );
    let central_size = u64::from(u32_at(end, 12));
    let central = u64::from(u32_at(end, 16));
    ensure!(
        central_size <= METADATA_LIMIT
            && central != u64::from(u32::MAX)
            && central + central_size == length - tail_size as u64 + index as u64,
        "invalid or oversized ZIP directory"
    );
    file.seek(SeekFrom::Start(central))?;
    let mut entries = Vec::with_capacity(usize::from(count));
    let mut names = BTreeSet::new();
    let mut total = 0u64;
    for _ in 0..count {
        ensure!(
            file.stream_position()? + 46 <= central + central_size,
            "truncated ZIP directory"
        );
        let header = fixed::<46>(file)?;
        ensure!(
            u32_at(&header, 0) == 0x02014b50,
            "invalid ZIP directory entry"
        );
        let flags = u16_at(&header, 8);
        let method = u16_at(&header, 10);
        ensure!(
            u16_at(&header, 6) <= 20
                && flags & !0x080e == 0
                && matches!(method, 0 | 8)
                && (method == 8 || flags & 6 == 0),
            "ZIP encoding is not admitted"
        );
        ensure!(
            u16_at(&header, 34) == 0,
            "multi-volume ZIP entry is not admitted"
        );
        let compressed = u32_at(&header, 20);
        let size = u32_at(&header, 24);
        let local = u32_at(&header, 42);
        ensure!(
            ![compressed, size, local].contains(&u32::MAX),
            "ZIP64 is not admitted"
        );
        ensure!(
            u64::from(size) <= bounds.max_file_bytes
                && u64::from(size)
                    <= u64::from(compressed).saturating_mul(u64::from(bounds.max_expansion_ratio)),
            "ZIP file expansion exceeds limit"
        );
        ensure!(method != 0 || size == compressed, "invalid stored ZIP size");
        total = total
            .checked_add(u64::from(size))
            .context("ZIP size overflow")?;
        ensure!(total <= bounds.max_bytes, "ZIP payload exceeds limit");
        let name_len = usize::from(u16_at(&header, 28));
        let extra_len = usize::from(u16_at(&header, 30));
        let comment_len = usize::from(u16_at(&header, 32));
        ensure!(
            file.stream_position()? + (name_len + extra_len + comment_len) as u64
                <= central + central_size,
            "ZIP metadata exceeds directory"
        );
        let raw = variable(file, name_len)?;
        ensure!(
            flags & 0x0800 != 0 || raw.is_ascii(),
            "ZIP name encoding is not admitted"
        );
        let name = String::from_utf8(raw).context("invalid ZIP UTF-8 name")?;
        super::access::relative(name.trim_end_matches('/'))?;
        ensure!(names.insert(name.clone()), "duplicate ZIP directory name");
        extras(&variable(file, extra_len)?)?;
        file.seek(SeekFrom::Current(comment_len as i64))?;
        let attributes = u32_at(&header, 38);
        let host = header[5];
        ensure!(
            matches!(host, 0 | 3),
            "ZIP originating system is not admitted"
        );
        let mode = if host == 3 { attributes >> 16 } else { 0 };
        let directory = name.ends_with('/');
        let kind = mode & 0o170000;
        ensure!(
            kind == 0 || kind == if directory { 0o040000 } else { 0o100000 },
            "ZIP special entry is not admitted"
        );
        ensure!(
            attributes & 0xffc8 == 0 && (attributes & 0x10 == 0 || directory),
            "inconsistent ZIP attributes"
        );
        ensure!(
            !directory || (size == 0 && compressed == 0),
            "ZIP directory contains data"
        );
        entries.push(Entry {
            name,
            local: u64::from(local),
            compressed,
            size,
            crc: u32_at(&header, 16),
            flags,
            method,
            mode,
            directory,
        });
    }
    ensure!(
        file.stream_position()? == central + central_size,
        "unconsumed ZIP directory data"
    );
    // Physical local records must partition the data region. Reject aliases,
    // overlap, stubs and unexplained gaps before giving offsets to the decoder.
    let mut physical: Vec<_> = entries.iter().collect();
    physical.sort_by_key(|entry| entry.local);
    let mut cursor = 0u64;
    let mut metadata = central_size;
    for (index, entry) in physical.iter().enumerate() {
        ensure!(
            entry.local == cursor && cursor + 30 <= central,
            "overlapping or noncontiguous ZIP records"
        );
        file.seek(SeekFrom::Start(cursor))?;
        let header = fixed::<30>(file)?;
        ensure!(
            u32_at(&header, 0) == 0x04034b50
                && u16_at(&header, 4) <= 20
                && u16_at(&header, 6) == entry.flags
                && u16_at(&header, 8) == entry.method,
            "inconsistent ZIP local header"
        );
        let actual = [
            u32_at(&header, 14),
            u32_at(&header, 18),
            u32_at(&header, 22),
        ];
        let expected = [entry.crc, entry.compressed, entry.size];
        ensure!(
            actual == expected || (entry.flags & 8 != 0 && actual == [0; 3]),
            "inconsistent ZIP local sizes or CRC"
        );
        let name_len = usize::from(u16_at(&header, 26));
        let extra_len = usize::from(u16_at(&header, 28));
        metadata += 30 + (name_len + extra_len) as u64;
        ensure!(metadata <= METADATA_LIMIT, "ZIP metadata exceeds limit");
        ensure!(
            cursor + 30 + (name_len + extra_len) as u64 <= central,
            "ZIP local metadata exceeds data region"
        );
        ensure!(
            variable(file, name_len)? == entry.name.as_bytes(),
            "inconsistent ZIP local name"
        );
        extras(&variable(file, extra_len)?)?;
        cursor = file.stream_position()? + u64::from(entry.compressed);
        let next = physical.get(index + 1).map_or(central, |entry| entry.local);
        ensure!(cursor <= next, "overlapping ZIP data");
        if entry.flags & 8 != 0 {
            let length = next - cursor;
            ensure!(
                matches!(length, 12 | 16),
                "invalid ZIP data descriptor size"
            );
            file.seek(SeekFrom::Start(cursor))?;
            let descriptor = variable(file, length as usize)?;
            let offset = if length == 16 {
                ensure!(
                    u32_at(&descriptor, 0) == 0x08074b50,
                    "invalid ZIP data descriptor signature"
                );
                4
            } else {
                0
            };
            ensure!(
                [
                    u32_at(&descriptor, offset),
                    u32_at(&descriptor, offset + 4),
                    u32_at(&descriptor, offset + 8)
                ] == expected,
                "inconsistent ZIP data descriptor"
            );
            cursor = next;
        }
    }
    ensure!(cursor == central, "unconsumed ZIP local data");
    ensure!(
        total + metadata + (length - central - central_size) <= bounds.max_bytes,
        "ZIP expanded archive exceeds limit"
    );
    file.rewind()?;
    Ok(entries)
}

pub(super) fn extract(
    mut file: File,
    root: &Directory,
    strip: Option<&str>,
    bounds: &Bounds,
) -> Result<()> {
    let entries = preflight(&mut file, bounds)?;
    let mut archive = ::zip::ZipArchive::new(file).context("invalid ZIP archive")?;
    ensure!(
        archive.len() == entries.len(),
        "ZIP decoder entry count differs"
    );
    let mut paths = Paths::new(bounds, strip);
    for (index, entry) in entries.iter().enumerate() {
        let mut source = archive.by_index(index)?;
        ensure!(
            source.name_raw() == entry.name.as_bytes()
                && source.size() == u64::from(entry.size)
                && source.compressed_size() == u64::from(entry.compressed)
                && source.crc32() == entry.crc,
            "ZIP decoder metadata differs"
        );
        let destination = paths.destination(root, &entry.name, entry.directory, entry.size == 0)?;
        if entry.directory {
            ensure!(
                source.read(&mut [0u8; 1])? == 0,
                "ZIP directory contains data"
            );
        } else if let Some((_, parent, name)) = destination {
            let mut target = parent.create_file(&name)?;
            let written = io::copy(
                &mut (&mut source).take(u64::from(entry.size) + 1),
                &mut target,
            )?;
            ensure!(written == u64::from(entry.size), "ZIP decoded size differs");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                target.set_permissions(std::fs::Permissions::from_mode(
                    0o600 | (entry.mode & 0o111),
                ))?;
            }
            #[cfg(not(unix))]
            let _ = entry.mode;
            target.sync_all()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn archive(names: &[&str]) -> Vec<u8> {
        let mut writer = ::zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in names {
            writer
                .start_file(
                    *name,
                    ::zip::write::SimpleFileOptions::default()
                        .compression_method(::zip::CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(b"payload").unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    fn validate(bytes: &[u8], bounds: &Bounds) -> Result<Vec<Entry>> {
        let mut file = tempfile::tempfile()?;
        file.write_all(bytes)?;
        preflight(&mut file, bounds)
    }
    fn central(bytes: &[u8]) -> usize {
        bytes
            .windows(4)
            .position(|bytes| bytes == b"PK\x01\x02")
            .unwrap()
    }

    #[test]
    fn rejects_duplicate_names_before_decoder_collapses_them() {
        let mut bytes = archive(&["a", "b"]);
        let start = central(&bytes);
        let second = start + 47;
        bytes[second + 46] = b'a';
        assert!(validate(&bytes, &Bounds::default())
            .err()
            .unwrap()
            .to_string()
            .contains("duplicate"));
    }

    #[test]
    fn rejects_entry_size_metadata_and_path_violations() {
        let bytes = archive(&["a", "b"]);
        let bounds = Bounds {
            max_entries: 1,
            ..Bounds::default()
        };
        assert!(validate(&bytes, &bounds).is_err());
        let bounds = Bounds {
            max_file_bytes: 1,
            ..Bounds::default()
        };
        assert!(validate(&bytes, &bounds).is_err());
        let mut bytes = archive(&["a"]);
        let end = bytes.len() - 22;
        bytes[end + 12..end + 16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(validate(&bytes, &Bounds::default()).is_err());
        assert!(validate(&archive(&["../escape"]), &Bounds::default()).is_err());
        let bounds = Bounds {
            max_bytes: 20,
            max_file_bytes: 10,
            ..Bounds::default()
        };
        assert!(validate(&archive(&["a"]), &bounds).is_err());
    }

    #[test]
    fn rejects_links_extensions_aliases_and_unconsumed_bytes() {
        for variant in 0..5 {
            let mut bytes = archive(&["a"]);
            let start = central(&bytes);
            match variant {
                0 => {
                    bytes[start + 5] = 3;
                    bytes[start + 38..start + 42]
                        .copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
                }
                1 => bytes[start + 6..start + 8].copy_from_slice(&45u16.to_le_bytes()),
                2 => {
                    bytes.push(0);
                }
                3 => bytes[start + 42..start + 46].copy_from_slice(&1u32.to_le_bytes()),
                _ => bytes[start + 8..start + 10].copy_from_slice(&1u16.to_le_bytes()),
            }
            assert!(
                validate(&bytes, &Bounds::default()).is_err(),
                "variant {variant}"
            );
        }
        for bytes in [
            &[1, 0, 0, 0][..],
            &[0x75, 0x70, 0, 0],
            &[0xff],
            &[0xff, 0xff, 5, 0, 0],
        ] {
            assert!(extras(bytes).is_err());
        }
    }

    #[test]
    fn accepts_and_checks_both_data_descriptor_encodings() {
        for signature in [false, true] {
            let mut bytes = archive(&["a"]);
            let start = central(&bytes);
            let expected = bytes[start + 16..start + 28].to_vec();
            bytes[6] |= 8;
            bytes[14..26].fill(0);
            bytes[start + 8] |= 8;
            let mut descriptor = Vec::new();
            if signature {
                descriptor.extend(0x08074b50u32.to_le_bytes());
            }
            descriptor.extend(expected);
            let extra = descriptor.len();
            bytes.splice(start..start, descriptor);
            let end = bytes.len() - 22;
            bytes[end + 16..end + 20].copy_from_slice(&((start + extra) as u32).to_le_bytes());
            assert!(validate(&bytes, &Bounds::default()).is_ok());
            let temporary = tempfile::tempdir().unwrap();
            let root = Directory::open(&temporary.path().canonicalize().unwrap()).unwrap();
            let mut file = tempfile::tempfile().unwrap();
            file.write_all(&bytes).unwrap();
            extract(file, &root, None, &Bounds::default()).unwrap();
            assert_eq!(
                std::fs::read(temporary.path().join("a")).unwrap(),
                b"payload"
            );
            bytes[start + extra - 1] ^= 1;
            assert!(validate(&bytes, &Bounds::default()).is_err());
        }
    }

    #[test]
    fn shared_paths_reject_case_collisions_without_following_links() {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(&archive(&["Dir/a", "dir/b"])).unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let root = Directory::open(&temporary.path().canonicalize().unwrap()).unwrap();
        assert!(extract(file, &root, None, &Bounds::default()).is_err());
        assert!(!temporary.path().join("dir/b").exists());
    }
}
