//! Resolve symbolic producer outputs and copy verified bytes into private consumers.
use super::bundle::safe_file;
use crate::{config::Materialize, executor::Image, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

pub(super) fn plan(
    mappings: &[Materialize],
    cwd: &str,
    artifacts: &[Value],
    images: &BTreeMap<String, Image>,
    consumer: &str,
    source: &snapshot::Snapshot,
) -> Result<Vec<Value>> {
    if mappings.len() > 256 {
        bail!("target exceeds 256 artifact materializations");
    }
    let mut destinations: Vec<String> = Vec::new();
    let mut inputs = Vec::new();
    for mapping in mappings {
        let relative = mapping
            .to
            .to_str()
            .context("materialization path is not UTF-8")?;
        if !snapshot::portable(relative) {
            bail!("nonportable materialization destination {relative}");
        }
        let destination = if cwd == "." {
            relative.into()
        } else {
            format!("{cwd}/{relative}")
        };
        let folded = destination.to_lowercase();
        for other in &destinations {
            let other_folded = other.to_lowercase();
            for (left, right) in destination.split('/').zip(other.split('/')) {
                if left.to_lowercase() != right.to_lowercase() {
                    break;
                }
                if left != right {
                    bail!("case-aliased materialization parents");
                }
            }
            if folded == other_folded
                || folded.starts_with(&format!("{other_folded}/"))
                || other_folded.starts_with(&format!("{folded}/"))
            {
                bail!("overlapping materialization destinations: {destination}");
            }
        }
        for entry in &source.entries {
            let existing = entry.path.to_lowercase();
            if existing == folded
                || existing.starts_with(&format!("{folded}/"))
                || (folded.starts_with(&format!("{existing}/"))
                    && (entry.kind != "directory"
                        || !destination.starts_with(&format!("{}/", entry.path))))
            {
                bail!("materialization collides with captured source: {destination}");
            }
        }
        destinations.push(destination.clone());
        let candidates: Vec<_> = artifacts
            .iter()
            .filter(|a| a["target"] == mapping.from)
            .collect();
        let artifact = if let Some(name) = &mapping.artifact {
            candidates
                .iter()
                .copied()
                .find(|a| a["name"] == *name)
                .with_context(|| format!("unknown artifact {}:{name}", mapping.from))?
        } else {
            candidates
                .iter()
                .copied()
                .find(|a| a["name"] == "primary")
                .or_else(|| (candidates.len() == 1).then(|| candidates[0]))
                .with_context(|| {
                    format!("{} requires an explicit artifact selection", mapping.from)
                })?
        };
        if artifact["kind"] != "file" {
            bail!("materialization requires a supported file artifact");
        }
        let producer_platform = images
            .get(&mapping.from)
            .context("missing producer platform")?;
        let consumer_platform = images.get(consumer).context("missing consumer platform")?;
        if producer_platform.os != consumer_platform.os
            || producer_platform.arch != consumer_platform.arch
        {
            bail!("incompatible artifact materialization platforms");
        }
        inputs.push(json!({"kind":"artifact","artifact":artifact["id"],"producer":artifact["producer"],"mount":destination}));
    }
    inputs.sort_by(|a, b| a["mount"].as_str().cmp(&b["mount"].as_str()));
    Ok(inputs)
}

/// Producer bundle files are never mounted into a consumer. Every consumer gets
/// its own copy, verified against the successful producer's recorded bytes.
pub(super) fn apply(
    workspace: &Path,
    bundle: &Path,
    inputs: &[Value],
    artifacts: &[Value],
    actions: &[Value],
) -> Result<Vec<Value>> {
    let mut receipts = Vec::new();
    for input in inputs.iter().filter(|i| i["kind"] == "artifact") {
        let artifact = artifacts
            .iter()
            .find(|a| a["id"] == input["artifact"] && a["producer"] == input["producer"])
            .context("materialization producer artifact is unavailable")?;
        if !actions
            .iter()
            .any(|a| a["id"] == input["producer"] && a["status"] == "succeeded")
        {
            bail!("materialization producer did not succeed");
        }
        let source = safe_file(
            bundle,
            artifact["path"]
                .as_str()
                .context("missing producer artifact path")?,
        )?;
        let expected = artifact["digest"]
            .as_str()
            .context("missing producer artifact digest")?;
        if snapshot::file_digest(&source)? != expected
            || Some(fs::metadata(&source)?.len()) != artifact["size"].as_u64()
        {
            bail!("materialization producer artifact failed integrity verification");
        }
        let relative = input["mount"]
            .as_str()
            .context("missing materialization destination")?;
        if !snapshot::portable(relative) {
            bail!("invalid materialization destination");
        }
        let mut destination = workspace.to_path_buf();
        let parts: Vec<_> = relative.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            destination.push(part);
            match fs::symlink_metadata(&destination) {
                Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {}
                Ok(_) => bail!("unsafe materialization parent"),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(&destination)?
                }
                Err(error) => return Err(error.into()),
            }
        }
        destination.push(parts.last().unwrap());
        let mut incoming = fs::File::open(&source)?;
        let mut output = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&destination)?;
        std::io::copy(&mut incoming, &mut output)?;
        output.flush()?;
        output.sync_all()?;
        if snapshot::file_digest(&destination)? != expected {
            bail!("materialized artifact changed while copying");
        }
        fs::set_permissions(&destination, incoming.metadata()?.permissions())?;
        receipts.push(json!({"artifact":input["artifact"],"producer":input["producer"],"path":relative,"digest":expected,"size":artifact["size"]}));
    }
    Ok(receipts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_copies_preserve_producer_and_reject_tampering_and_overwrites() {
        let bundle = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        fs::write(bundle.path().join("binary"), "verified producer bytes").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                bundle.path().join("binary"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        let digest = snapshot::file_digest(&bundle.path().join("binary")).unwrap();
        let artifacts = vec![
            json!({"id":"producer/primary","producer":"producer:package","path":"binary","digest":digest,"size":23}),
        ];
        let actions = vec![json!({"id":"producer:package","status":"succeeded"})];
        let inputs = vec![
            json!({"kind":"artifact","artifact":"producer/primary","producer":"producer:package","mount":"bin/server"}),
        ];
        // The native payload has 23 bytes; both size and digest are required.
        assert_eq!(
            fs::metadata(bundle.path().join("binary")).unwrap().len(),
            23
        );
        let receipts = apply(
            workspace.path(),
            bundle.path(),
            &inputs,
            &artifacts,
            &actions,
        )
        .unwrap();
        assert_eq!(receipts[0]["digest"], digest);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(
                fs::metadata(workspace.path().join("bin/server"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o111,
                0
            );
        }
        assert!(apply(
            workspace.path(),
            bundle.path(),
            &inputs,
            &artifacts,
            &actions
        )
        .is_err());
        fs::write(workspace.path().join("bin/server"), "consumer mutation").unwrap();
        assert_eq!(
            snapshot::file_digest(&bundle.path().join("binary")).unwrap(),
            digest
        );
        let second = tempfile::tempdir().unwrap();
        let failed = vec![json!({"id":"producer:package","status":"failed"})];
        assert!(apply(second.path(), bundle.path(), &inputs, &artifacts, &failed).is_err());
        fs::write(bundle.path().join("binary"), "tampered producer bytes").unwrap();
        assert!(apply(second.path(), bundle.path(), &inputs, &artifacts, &actions).is_err());
    }

    #[test]
    fn destinations_reject_source_collisions_aliases_escapes_and_ambiguous_outputs() {
        let root = tempfile::tempdir().unwrap();
        let captured = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("Inputs")).unwrap();
        fs::write(root.path().join("existing"), "source").unwrap();
        let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
        let image = Image {
            reference: "test".into(),
            digest: "sha256:test".into(),
            os: "linux".into(),
            arch: "amd64".into(),
        };
        let images = BTreeMap::from([
            ("producer".into(), image.clone()),
            ("consumer".into(), image),
        ]);
        let artifacts = vec![
            json!({"id":"producer/primary","target":"producer","name":"primary","producer":"producer:package","kind":"file"}),
        ];
        let mapping = |to: &str| Materialize {
            from: "producer".into(),
            artifact: None,
            to: to.into(),
        };
        for path in [
            "../escape",
            "/absolute",
            "existing",
            "existing/child",
            "inputs/child",
            "NUL",
            "a\\b",
        ] {
            assert!(
                plan(
                    &[mapping(path)],
                    ".",
                    &artifacts,
                    &images,
                    "consumer",
                    &source
                )
                .is_err(),
                "{path}"
            );
        }
        for paths in [["new", "new/child"], ["new/a", "NEW/b"]] {
            assert!(plan(
                &paths.map(mapping),
                ".",
                &artifacts,
                &images,
                "consumer",
                &source
            )
            .is_err());
        }
        assert!(plan(
            &[mapping("Inputs/file")],
            ".",
            &artifacts,
            &images,
            "consumer",
            &source
        )
        .is_ok());
        let ambiguous = vec![
            json!({"target":"producer","name":"a"}),
            json!({"target":"producer","name":"b"}),
        ];
        assert!(plan(
            &[mapping("output")],
            ".",
            &ambiguous,
            &images,
            "consumer",
            &source
        )
        .is_err());
    }
}
