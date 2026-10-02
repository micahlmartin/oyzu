mod tool_store_fixture;

use sha2::{Digest, Sha256};
use std::{fs, io::Write};

fn tar(entries: &[(&str, &[u8], u8, Option<&str>)]) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    for (path, bytes, kind, target) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        header.set_size(bytes.len() as u64);
        header.set_entry_type(tar::EntryType::new(*kind));
        // Write raw hostile names; Builder::append_data would sanitize them.
        header.as_mut_bytes()[..path.len()].copy_from_slice(path.as_bytes());
        if let Some(target) = target {
            header.as_mut_bytes()[157..157 + target.len()].copy_from_slice(target.as_bytes());
        }
        header.set_cksum();
        archive.append(&header, *bytes).unwrap();
    }
    archive.into_inner().unwrap()
}

fn materialize(bytes: &[u8], gzip: bool) -> anyhow::Result<oyzu::tools::TreeInspection> {
    let directory = tool_store_fixture::directory()?;
    let source = directory.path().join("source");
    let staging = directory.path().join("stage");
    fs::write(&source, bytes)?;
    fs::create_dir(&staging)?;
    oyzu::tools::materialize_archive(
        &source,
        &staging,
        &format!("sha256:{:x}", Sha256::digest(bytes)),
        bytes.len() as u64,
        gzip,
    )
}

#[test]
fn verifies_exact_bytes_before_materializing_and_never_overwrites() {
    let directory = tool_store_fixture::directory().unwrap();
    let source = directory.path().join("source");
    let stage = directory.path().join("stage");
    fs::create_dir(&stage).unwrap();
    let bytes = tar(&[("bin/tool", b"payload", b'0', None)]);
    fs::write(&source, &bytes).unwrap();
    let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    let bad = format!("sha256:{}", "0".repeat(64));
    assert!(
        oyzu::tools::materialize_archive(&source, &stage, &bad, bytes.len() as u64, false).is_err()
    );
    assert_eq!(fs::read_dir(&stage).unwrap().count(), 0);
    assert!(oyzu::tools::materialize_archive(
        &source,
        &stage,
        &digest,
        bytes.len() as u64 - 1,
        false
    )
    .is_err());
    assert_eq!(fs::read_dir(&stage).unwrap().count(), 0);
    let report =
        oyzu::tools::materialize_archive(&source, &stage, &digest, bytes.len() as u64, false)
            .unwrap();
    assert_eq!(report.bytes, 7);
    assert_eq!(fs::read(stage.join("bin/tool")).unwrap(), b"payload");
    assert!(
        oyzu::tools::materialize_archive(&source, &stage, &digest, bytes.len() as u64, false)
            .is_err()
    );
}

#[test]
fn rejects_traversal_aliases_collisions_special_files_and_sparse_extensions() {
    for path in [
        "../outside",
        "/outside",
        "C:/outside",
        "bin/../outside",
        "bin\\outside",
        "NUL.txt",
        "bin/tool:stream",
        "bin/tool.",
    ] {
        assert!(
            materialize(&tar(&[(path, b"bad", b'0', None)]), false).is_err(),
            "{path}"
        );
    }
    for entries in [
        vec![
            ("bin/tool", b"a".as_slice(), b'0', None),
            ("bin/tool", b"b", b'0', None),
        ],
        vec![("Bin/a", b"a", b'0', None), ("bin/b", b"b", b'0', None)],
        vec![("bin", b"a", b'0', None), ("bin/tool", b"b", b'0', None)],
    ] {
        assert!(materialize(&tar(&entries), false).is_err());
    }
    for kind in [b'1', b'3', b'4', b'6', b'S', b'x', b'g'] {
        assert!(materialize(&tar(&[("special", b"", kind, None)]), false).is_err());
    }
}

#[test]
fn rejects_expansion_bombs_and_corrupt_gzip_trailers() {
    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut writer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        writer.write_all(bytes).unwrap();
        writer.finish().unwrap()
    }
    let ordinary = tar(&[("tool", b"payload", b'0', None)]);
    let mut compressed = gzip(&ordinary);
    let report = materialize(&compressed, true).unwrap();
    assert_eq!(report.digest, materialize(&ordinary, false).unwrap().digest);
    let trailer = compressed.len() - 8;
    compressed[trailer] ^= 1;
    assert!(materialize(&compressed, true).is_err());
    let bomb = tar(&[("tool", &vec![0u8; 1024 * 1024], b'0', None)]);
    assert!(materialize(&gzip(&bomb), true).is_err());
}

#[cfg(unix)]
#[test]
fn delays_links_until_writes_finish_and_checks_complete_link_graph() {
    let valid = tar(&[
        ("bin/alias", b"", b'2', Some("tool")),
        ("bin/tool", b"payload", b'0', None),
    ]);
    assert_eq!(materialize(&valid, false).unwrap().entries.len(), 3);
    for entries in [
        vec![("alias", b"".as_slice(), b'2', Some("../outside"))],
        vec![("alias", b"", b'2', Some("absent"))],
        vec![("a", b"", b'2', Some("b")), ("b", b"", b'2', Some("a"))],
        vec![
            ("bin", b"", b'2', Some("../outside")),
            ("bin/tool", b"bad", b'0', None),
        ],
    ] {
        assert!(materialize(&tar(&entries), false).is_err());
    }
}

#[test]
fn bounds_gnu_extension_bodies_before_allocating_them() {
    let extension = vec![b'a'; 16 * 1024 + 1];
    assert!(materialize(&tar(&[("././@LongLink", &extension, b'L', None)]), false).is_err());
    assert!(materialize(&tar(&[("././@LongLink", b"bin/tool\0", b'L', None)]), false).is_err());
    let valid = tar(&[
        ("././@LongLink", b"bin/tool\0", b'L', None),
        ("ignored", b"data", b'0', None),
    ]);
    assert!(materialize(&valid, false)
        .unwrap()
        .entries
        .iter()
        .any(|entry| entry.path == "bin/tool"));
}

#[cfg(unix)]
#[test]
fn rejects_fifo_and_symlink_sources_without_following_or_waiting() {
    use std::{ffi::CString, os::unix::fs::symlink};
    let directory = tool_store_fixture::directory().unwrap();
    let stage = directory.path().join("stage");
    fs::create_dir(&stage).unwrap();
    let fifo = directory.path().join("fifo");
    let name = CString::new(fifo.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let link = directory.path().join("link");
    symlink(&fifo, &link).unwrap();
    for source in [fifo, link] {
        assert!(oyzu::tools::materialize_archive(
            &source,
            &stage,
            &format!("sha256:{}", "0".repeat(64)),
            1,
            false
        )
        .is_err());
    }
    assert_eq!(fs::read_dir(stage).unwrap().count(), 0);
}
