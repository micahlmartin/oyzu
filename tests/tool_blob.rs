mod tool_store_fixture;

use oyzu::tools::{cache_tool_blob, materialize_tool_blob};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn location(root: &Path, digest: &str) -> std::path::PathBuf {
    root.join("blobs/sha256").join(&digest[7..])
}
struct NeverRead;
impl Read for NeverRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("unexpected acquisition")
    }
}

#[test]
fn publishes_exact_bytes_and_cache_hit_never_acquires_again() {
    let temporary = tool_store_fixture::directory().unwrap();
    let bytes = b"verified archive bytes";
    let digest = digest(bytes);
    let mut blob = cache_tool_blob(
        temporary.path(),
        &mut &bytes[..],
        &digest,
        bytes.len() as u64,
    )
    .unwrap();
    assert_eq!(blob.digest(), digest);
    assert_eq!(blob.size(), bytes.len() as u64);
    let mut observed = Vec::new();
    blob.read_to_end(&mut observed).unwrap();
    assert_eq!(observed, bytes);
    assert_eq!(
        fs::read(location(temporary.path(), &digest)).unwrap(),
        bytes
    );
    assert_eq!(
        fs::read_dir(temporary.path().join("staging"))
            .unwrap()
            .count(),
        0
    );
    let mut cached = cache_tool_blob(
        temporary.path(),
        &mut NeverRead,
        &digest,
        bytes.len() as u64,
    )
    .unwrap();
    let mut observed = Vec::new();
    cached.read_to_end(&mut observed).unwrap();
    assert_eq!(observed, bytes);
}

#[test]
fn corruption_and_external_links_fail_without_fallback() {
    let temporary = tool_store_fixture::directory().unwrap();
    let bytes = b"good";
    let digest = digest(bytes);
    let mut blob = cache_tool_blob(temporary.path(), &mut &bytes[..], &digest, 4).unwrap();
    let path = location(temporary.path(), &digest);
    fs::write(&path, b"evil").unwrap();
    assert!(cache_tool_blob(temporary.path(), &mut NeverRead, &digest, 4).is_err());
    let mut retained = Vec::new();
    blob.read_to_end(&mut retained).unwrap();
    assert_eq!(
        retained, bytes,
        "returned snapshot must not alias the mutable cache"
    );
    fs::write(&path, bytes).unwrap();
    fs::hard_link(&path, temporary.path().join("external")).unwrap();
    assert!(cache_tool_blob(temporary.path(), &mut NeverRead, &digest, 4).is_err());
}

#[test]
fn rejects_size_digest_and_reader_failure_without_publishing() {
    let temporary = tool_store_fixture::directory().unwrap();
    let expected = digest(b"good");
    for bytes in [b"bad!".as_slice(), b"goo", b"good!"] {
        assert!(cache_tool_blob(temporary.path(), &mut &bytes[..], &expected, 4).is_err());
        assert!(!location(temporary.path(), &expected).exists());
    }
    struct Failure;
    impl Read for Failure {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("cancelled transport"))
        }
    }
    assert!(cache_tool_blob(temporary.path(), &mut Failure, &expected, 4).is_err());
    for size in [0, 8 * 1024 * 1024 * 1024 + 1, u64::MAX] {
        assert!(cache_tool_blob(temporary.path(), &mut NeverRead, &expected, size).is_err());
    }
    assert!(cache_tool_blob(temporary.path(), &mut NeverRead, "../bad", 4).is_err());
    assert!(!location(temporary.path(), &expected).exists());
}

#[test]
fn bounds_stream_consumption_and_handles_interrupted_short_reads() {
    let temporary = tool_store_fixture::directory().unwrap();
    let expected = digest(b"good");
    let mut infinite = io::repeat(b'g');
    assert!(cache_tool_blob(temporary.path(), &mut infinite, &expected, 4).is_err());
    struct Chunks {
        remaining: &'static [u8],
        interrupted: bool,
        consumed: usize,
    }
    impl Read for Chunks {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(io::ErrorKind::Interrupted.into());
            }
            let count = bytes.len().min(self.remaining.len()).min(1);
            bytes[..count].copy_from_slice(&self.remaining[..count]);
            self.remaining = &self.remaining[count..];
            self.consumed += count;
            Ok(count)
        }
    }
    let mut extra = Chunks {
        remaining: b"good-more-extra-bytes",
        interrupted: false,
        consumed: 0,
    };
    assert!(cache_tool_blob(temporary.path(), &mut extra, &expected, 4).is_err());
    assert_eq!(extra.consumed, 5);
    let mut exact = Chunks {
        remaining: b"good",
        interrupted: false,
        consumed: 0,
    };
    assert!(cache_tool_blob(temporary.path(), &mut exact, &expected, 4).is_ok());
}

#[test]
fn materializes_private_snapshot_after_cache_tampering() {
    let temporary = tool_store_fixture::directory().unwrap();
    let mut archive = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o755);
    header.set_size(4);
    header.set_cksum();
    archive
        .append_data(&mut header, "bin/tool", &b"good"[..])
        .unwrap();
    let bytes = archive.into_inner().unwrap();
    let digest = digest(&bytes);
    let blob = cache_tool_blob(
        temporary.path(),
        &mut &bytes[..],
        &digest,
        bytes.len() as u64,
    )
    .unwrap();
    fs::write(location(temporary.path(), &digest), b"bad").unwrap();
    let staging = temporary.path().join("payload");
    fs::create_dir(&staging).unwrap();
    let tree = materialize_tool_blob(blob, &staging, false).unwrap();
    assert_eq!(tree.bytes, 4);
    assert_eq!(fs::read(staging.join("bin/tool")).unwrap(), b"good");
}

#[test]
fn publication_collision_preserves_destination_and_removes_own_temporary() {
    struct Race {
        target: std::path::PathBuf,
        bytes: &'static [u8],
        fired: bool,
    }
    impl Read for Race {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if !self.fired {
                fs::write(&self.target, b"existing")?;
                self.fired = true;
            }
            self.bytes.read(output)
        }
    }
    let temporary = tool_store_fixture::directory().unwrap();
    let expected = digest(b"good");
    let target = location(temporary.path(), &expected);
    let mut source = Race {
        target: target.clone(),
        bytes: b"good",
        fired: false,
    };
    assert!(cache_tool_blob(temporary.path(), &mut source, &expected, 4).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"existing");
    assert_eq!(
        fs::read_dir(temporary.path().join("staging"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
#[ignore = "fixture invoked by separate-process cache test"]
fn blob_child() {
    let root = std::env::var_os("OYZU_TEST_BLOB_STORE").unwrap();
    let bytes = b"parallel-cache-bytes";
    if let Some(ready) = std::env::var_os("OYZU_TEST_BLOB_READY") {
        struct InterruptedOwner {
            ready: std::path::PathBuf,
            wrote: bool,
        }
        impl Read for InterruptedOwner {
            fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
                if !self.wrote {
                    output[0] = b'p';
                    self.wrote = true;
                    return Ok(1);
                }
                fs::write(&self.ready, b"ready")?;
                loop {
                    std::thread::park();
                }
            }
        }
        let mut source = InterruptedOwner {
            ready: ready.into(),
            wrote: false,
        };
        cache_tool_blob(
            Path::new(&root),
            &mut source,
            &digest(bytes),
            bytes.len() as u64,
        )
        .unwrap();
        panic!("interrupted owner should have been terminated");
    }
    cache_tool_blob(
        Path::new(&root),
        &mut &bytes[..],
        &digest(bytes),
        bytes.len() as u64,
    )
    .unwrap();
}

#[test]
fn killed_stream_owner_publishes_nothing_and_releases_blob_lock() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    struct KillOnDrop(std::process::Child);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let temporary = tool_store_fixture::directory().unwrap();
    let ready = temporary.path().join("ready");
    let mut child = KillOnDrop(
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "blob_child"])
            .env("OYZU_TEST_BLOB_STORE", temporary.path())
            .env("OYZU_TEST_BLOB_READY", &ready)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() {
        assert!(
            Instant::now() < deadline && child.0.try_wait().unwrap().is_none(),
            "blob owner did not reach partial-copy boundary"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let bytes = b"parallel-cache-bytes";
    assert!(!location(temporary.path(), &digest(bytes)).exists());
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    assert!(cache_tool_blob(
        temporary.path(),
        &mut &bytes[..],
        &digest(bytes),
        bytes.len() as u64
    )
    .is_ok());
}

#[test]
fn separate_processes_converge_on_one_verified_blob() {
    let temporary = tool_store_fixture::directory().unwrap();
    let mut children = Vec::new();
    for _ in 0..2 {
        children.push(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--ignored", "--exact", "blob_child"])
                .env("OYZU_TEST_BLOB_STORE", temporary.path())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
    }
    // Reap both before reporting a failure, so fixture cleanup cannot erase the
    // store while the other publisher is still producing its diagnostic.
    let outputs: Vec<_> = children
        .into_iter()
        .map(|child| child.wait_with_output().unwrap())
        .collect();
    for output in outputs {
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let bytes = b"parallel-cache-bytes";
    assert!(cache_tool_blob(
        temporary.path(),
        &mut NeverRead,
        &digest(bytes),
        bytes.len() as u64
    )
    .is_ok());
    assert_eq!(
        fs::read_dir(temporary.path().join("blobs/sha256"))
            .unwrap()
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn rejects_redirected_cache_roots_and_blob_symlinks() {
    use std::os::unix::fs::symlink;
    let temporary = tool_store_fixture::directory().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let outside = root.join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, root.join("blobs")).unwrap();
    let expected = digest(b"good");
    assert!(cache_tool_blob(&root, &mut NeverRead, &expected, 4).is_err());
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    fs::remove_file(root.join("blobs")).unwrap();
    fs::create_dir_all(root.join("blobs/sha256")).unwrap();
    fs::write(outside.join("bytes"), b"good").unwrap();
    symlink(outside.join("bytes"), location(&root, &expected)).unwrap();
    assert!(cache_tool_blob(&root, &mut NeverRead, &expected, 4).is_err());
    assert_eq!(fs::read(outside.join("bytes")).unwrap(), b"good");
}
