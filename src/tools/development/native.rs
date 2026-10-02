//! Native backend installation bridge. Mise owns installation; Oyzu packages
//! its completed private sysroot for the existing verified store and offline replay.
use super::*;
use sha2::{Digest, Sha256};

pub(super) fn rust_metadata(version: &str, target: &str) -> Metadata {
    Metadata {
        archive: Archive {
            version: version.into(),
            target: target.into(),
            archive_url: format!("oyzu-native:rust-{version}-{target}.tar.gz"),
            archive_kind: "tar.gz".into(),
            strip_prefix: "rust".into(),
            executable_relative_path: if cfg!(windows) {
                "bin/cargo.exe"
            } else {
                "bin/cargo"
            }
            .into(),
            bin_relative_path: "bin".into(),
        },
        declared_sha256: None,
        declared_size: None,
    }
}

pub(super) fn install_rust(
    version: &str,
    target: &str,
    acquisition: &mut Acquisition,
) -> Result<(Metadata, Vec<u8>)> {
    let temporary = tempfile::tempdir()?;
    let archive = temporary.path().canonicalize()?.join("rust.tar.gz");
    worker_call::<()>(
        &WorkerRequest::InstallRust {
            version: version.into(),
            archive: archive.clone(),
        },
        Some(acquisition),
    )?;
    let bytes = std::fs::read(archive)?;
    let mut metadata = rust_metadata(version, target);
    metadata.declared_sha256 = Some(format!("sha256:{:x}", Sha256::digest(&bytes)));
    metadata.declared_size = Some(bytes.len() as u64);
    Ok((metadata, bytes))
}
