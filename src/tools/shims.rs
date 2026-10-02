//! Versioned native frontend shims. The manifest identifies the frontend and
//! command, never a selected tool version. Each invocation uses frozen lookup.
use crate::config::session::Options;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: u32,
    release_digest: String,
    command: String,
    store: Option<PathBuf>,
}

fn digest(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        hash.update(&buffer[..size]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

/// Retain the actual frontend image as native Node/Go shims. This runs during
/// install/activation, never during a prompt hook. A shared store is explicit;
/// dynamic mode uses the invocation directory's store on every subsequent call.
pub(super) fn prepare(store: &Path, dynamic: bool) -> Result<PathBuf> {
    let frontend = std::env::current_exe()?;
    let release = digest(&frontend)?;
    let directory = std::path::absolute(store)?
        .join("shims")
        .join(&release)
        .join(if dynamic { "cwd" } else { "shared" });
    std::fs::create_dir_all(&directory)?;
    for name in ["node", "go"] {
        let image = directory.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.into()
        });
        if !image.exists() && std::fs::hard_link(&frontend, &image).is_err() {
            std::fs::copy(&frontend, &image)?;
        }
        ensure!(
            digest(&image)? == release,
            "shim image differs from its frontend release"
        );
        let manifest = Manifest {
            format: 1,
            release_digest: release.clone(),
            command: name.into(),
            store: if dynamic {
                None
            } else {
                Some(std::path::absolute(store)?)
            },
        };
        std::fs::write(
            directory.join(format!("{name}.oyzu-shim.json")),
            serde_json::to_vec(&manifest)?,
        )?;
    }
    Ok(directory)
}

/// Detect a native shim before CLI parsing. Basename, adjacent manifest and
/// image digest must agree. Does not consult PATH or acquire missing tools.
pub fn dispatch() -> Result<Option<i32>> {
    let image = std::env::current_exe()?;
    let name = image
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if !matches!(name, "node" | "go") {
        return Ok(None);
    }
    let manifest: Manifest = serde_json::from_slice(&super::read_record(
        &image
            .parent()
            .context("shim has no directory")?
            .join(format!("{name}.oyzu-shim.json")),
        16384,
    )?)?;
    ensure!(
        manifest.format == 1
            && manifest.command == name
            && manifest.release_digest == digest(&image)?,
        "invalid native shim identity"
    );
    let directory = std::env::current_dir()?;
    let store = manifest
        .store
        .unwrap_or_else(|| directory.join(".oyzu/tools"));
    let options: Options = super::shell::active_options()?.unwrap_or_default();
    let arguments: Vec<OsString> = std::iter::once(OsString::from(name))
        .chain(std::env::args_os().skip(1))
        .collect();
    super::development::exec(&directory, &options, &store, &arguments).map(Some)
}
