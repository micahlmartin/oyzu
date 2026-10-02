//! Verified content-addressed bytes. Source authorization is a separate gate.
use super::{
    access::{self, Directory},
    transaction,
};
use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MAX_BYTES: u64 = 8 * 1024 * 1024 * 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A private, verified snapshot, independent of later cache-path mutations.
/// No writer or raw file handle is exposed. This proves bytes, not provenance,
/// policy permission or backend admission.
pub struct VerifiedBlob {
    file: File,
    digest: String,
    size: u64,
}

impl VerifiedBlob {
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn size(&self) -> u64 {
        self.size
    }

    pub(super) fn into_file(mut self) -> Result<File> {
        self.file.rewind()?;
        Ok(self.file)
    }
}

impl Read for VerifiedBlob {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.file.read(bytes)
    }
}
impl Seek for VerifiedBlob {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.file.seek(position)
    }
}

pub(super) fn snapshot(source: &mut dyn Read, digest: &str, size: u64) -> Result<VerifiedBlob> {
    validate(digest, size)?;
    let mut file = tempfile::tempfile()?;
    let mut hash = Sha256::new();
    let mut copied = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        // One extra byte detects excess without draining an unbounded source.
        let limit = buffer
            .len()
            .min(usize::try_from(size - copied + 1).unwrap_or(usize::MAX));
        let count = match source.read(&mut buffer[..limit]) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if count == 0 {
            break;
        }
        copied += count as u64;
        ensure!(copied <= size, "blob exceeds locked size");
        hash.update(&buffer[..count]);
        file.write_all(&buffer[..count])?;
    }
    ensure!(
        copied == size && format!("sha256:{:x}", hash.finalize()) == digest,
        "blob differs from locked size/digest"
    );
    file.rewind()?;
    Ok(VerifiedBlob {
        file,
        digest: digest.into(),
        size,
    })
}

fn validate(digest: &str, size: u64) -> Result<()> {
    crate::tools::lock::digest(digest)?;
    ensure!((1..=MAX_BYTES).contains(&size), "blob size exceeds limit");
    Ok(())
}

pub(in crate::tools) fn cache(
    store: &Path,
    source: &mut dyn Read,
    digest: &str,
    size: u64,
) -> Result<VerifiedBlob> {
    validate(digest, size)?;
    let root = Directory::open(store)?;
    let blobs = root.create_directory("blobs")?.create_directory("sha256")?;
    let locks = root.create_directory("locks")?;
    let name = &digest[7..];
    let lock = locks.lock_file(&format!("blob-{name}"))?;
    transaction::acquire(&lock, Instant::now() + Duration::from_secs(30), false)?;
    match blobs.file(name) {
        Ok(mut file) => {
            ensure!(
                access::identity(&file)?.links == 1,
                "cached blob has external hardlinks"
            );
            // Corruption fails without reading the supplied acquisition stream.
            return snapshot(&mut file, digest, size).context("cached blob is invalid");
        }
        Err(error)
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|e| e.kind() == io::ErrorKind::NotFound) => {}
        Err(error) => return Err(error),
    }
    let mut verified = snapshot(source, digest, size)?;
    let staging = root.create_directory("staging")?;
    // This is a collision-resistant temporary name, not an authentication nonce.
    // create_new is authoritative: a collision never adopts or removes a file.
    let temporary = format!(
        "blob-{}-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut output = staging.create_file(&temporary)?;
    let written = (|| -> Result<()> {
        ensure!(
            io::copy(&mut verified, &mut output)? == size,
            "private blob snapshot was truncated"
        );
        output.sync_all()?;
        Ok(())
    })();
    drop(output); // Windows publication requires closing the temporary writer.
    let publication = written.and_then(|()| staging.publish(&temporary, &blobs, name));
    if let Err(error) = publication {
        if let Err(cleanup) = staging.remove_file(&temporary) {
            if cleanup
                .downcast_ref::<io::Error>()
                .is_none_or(|e| e.kind() != io::ErrorKind::NotFound)
            {
                return Err(error.context(format!("blob temporary cleanup failed: {cleanup}")));
            }
        }
        return Err(error);
    }
    verified.rewind()?;
    Ok(verified)
}
