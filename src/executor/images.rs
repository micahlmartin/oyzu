//! Host-side export of explicitly provisioned image inputs. Never pulls or runs them.
use super::{resolve_for, run, Image, Profile, Request};
use anyhow::{bail, Result};
use std::{fs, path::Path, process::Command};

pub(crate) fn export_image(
    reference: &str,
    destination: &Path,
    request: &Request<'_>,
    target: &crate::platform::Platform,
) -> Result<Image> {
    if !crate::oci::literal_reference(reference) {
        bail!("invalid provisioned image reference");
    }
    if destination.exists() {
        bail!("image export destination already exists");
    }
    let image = resolve_for(reference, Profile::ImageInput)?;
    if image.platform()? != *target {
        bail!("provisioned base image platform differs from the selected build platform");
    }
    let mut command = Command::new("docker");
    command
        .args(["image", "save", "--output"])
        .arg(destination)
        .arg(&image.digest);
    let result = run(command, request)?;
    if result.code != 0 {
        bail!("provisioned image export failed; inspect acquisition logs");
    }
    if fs::metadata(destination)?.len() > 10 * 1024 * 1024 * 1024 {
        bail!("base image export exceeds 10 GiB");
    }
    Ok(image)
}
