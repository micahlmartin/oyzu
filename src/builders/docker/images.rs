//! Image-input preparation. Registry acquisition can feed the same immutable
//! OCI-store contract later; this profile consumes explicitly provisioned images.
use crate::{builders::PreparationContext, executor, snapshot};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, time::Duration};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    name: String,
    manifest: String,
    config: String,
    os: String,
    architecture: String,
}

pub(super) fn capture(
    context: &PreparationContext<'_>,
    references: &[String],
) -> Result<Vec<executor::ImageInput>> {
    if references.is_empty() {
        return Ok(Vec::new());
    }
    let control = tempfile::tempdir()?;
    let workspace = control.path().join("workspace");
    let transport = control.path().join("transport");
    fs::create_dir(&workspace)?;
    fs::create_dir(&transport)?;
    fs::create_dir(context.destination.join("images"))?;
    let stdout = control.path().join("stdout");
    let stderr = control.path().join("stderr");
    let env = BTreeMap::new();
    let mut images = Vec::new();
    for (index, reference) in references.iter().enumerate() {
        let request = executor::Request {
            image: context.image,
            workspace: &workspace,
            output: context.destination,
            cwd: "/workspace",
            argv: &[],
            env: &env,
            stdout: &stdout,
            stderr: &stderr,
            timeout: Duration::from_secs(600),
            name: context.execution_name,
        };
        let archive = transport.join("image.tar");
        let resolved = executor::export_image(reference, &archive, &request)?;
        let store = format!("images/base-{index}");
        let argv = crate::builders::strings(&[
            "oyzu-docker-images",
            "/dependencies/image.tar",
            &format!("/out/{store}"),
            &resolved.digest,
            &format!("{}/{}", resolved.os, resolved.arch),
            reference,
        ]);
        let result = executor::execute_with_mounts(
            executor::Request {
                argv: &argv,
                ..request
            },
            &[executor::Mount {
                source: &transport,
                destination: "/dependencies",
                readonly: true,
            }],
        )?;
        if result.code != 0 {
            bail!(
                "native base-image capture failed: {}",
                fs::read_to_string(&stderr)?
            );
        }
        if fs::metadata(&stdout)?.len() > 8192 {
            bail!("image identity response exceeds limit");
        }
        let native: Identity = serde_json::from_slice(&fs::read(&stdout)?)?;
        if native.config != resolved.digest
            || native.os != resolved.os
            || native.architecture != resolved.arch
        {
            bail!("native base identity differs from provisioned image");
        }
        let tree = snapshot::capture_prepared(
            &context.destination.join(&store),
            &control.path().join(format!("frozen-{index}")),
        )?;
        let image = executor::ImageInput {
            reference: reference.clone(),
            name: native.name,
            store,
            manifest: native.manifest,
            config: native.config,
            tree_digest: tree.digest,
        };
        image
            .validate()
            .context("invalid native OCI image binding")?;
        images.push(image);
        fs::remove_file(archive)?;
    }
    Ok(images)
}
