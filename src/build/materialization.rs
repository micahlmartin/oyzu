//! Resolve symbolic producer outputs and copy verified bytes into private consumers.
use crate::bundle_store::safe_file;
use crate::{config::Materialize, platform::Platform, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

pub(super) fn plan(
    mappings: &[Materialize],
    cwd: &str,
    artifacts: &[Value],
    platforms: &BTreeMap<String, Platform>,
    consumer: &str,
    source: &snapshot::Snapshot,
    projection: Option<&snapshot::Projection>,
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
            if projection.is_some_and(|p| !p.contains(&entry.path)) {
                continue;
            }
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
        if !matches!(artifact["kind"].as_str(), Some("file" | "directory")) {
            bail!("materialization requires a supported file or directory artifact");
        }
        let producer_platform = platforms
            .get(&mapping.from)
            .context("missing producer platform")?;
        let consumer_platform = platforms
            .get(consumer)
            .context("missing consumer platform")?;
        if producer_platform != consumer_platform {
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
        let is_directory = artifact["kind"] == "directory";
        let source = if is_directory {
            super::directory::verify(bundle, artifact)?
        } else {
            safe_file(
                bundle,
                artifact["path"]
                    .as_str()
                    .context("missing producer artifact path")?,
            )?
        };
        let expected = artifact["digest"]
            .as_str()
            .context("missing producer artifact digest")?;
        if !is_directory
            && (snapshot::file_digest(&source)? != expected
                || Some(fs::metadata(&source)?.len()) != artifact["size"].as_u64())
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
        if is_directory {
            super::directory::copy(&source, &destination, artifact)?;
        } else {
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
        }
        receipts.push(json!({"artifact":input["artifact"],"producer":input["producer"],"path":relative,"digest":expected,"size":artifact["size"]}));
    }
    Ok(receipts)
}

/// Link original producer evidence; never relabel it as consumer measurements.
pub(super) fn reference_reports(
    receipts: &mut [Value],
    bundle: &Path,
    artifacts: &[Value],
    actions: &[Value],
    reports: &[Value],
) -> Result<()> {
    for receipt in receipts {
        let artifact = artifacts
            .iter()
            .find(|a| a["id"] == receipt["artifact"] && a["digest"] == receipt["digest"])
            .context("missing materialized artifact identity")?;
        let target = artifact["target"]
            .as_str()
            .context("missing producer target")?;
        let mut references = Vec::new();
        for report in reports.iter().filter(|r| {
            r["target"] == target
                && r["status"] == "collected"
                && matches!(r["kind"].as_str(), Some("test" | "coverage"))
        }) {
            if !actions
                .iter()
                .any(|a| a["id"] == report["action"] && a["status"] == "succeeded")
            {
                bail!("materialized producer report action did not succeed");
            }
            let path = report["path"]
                .as_str()
                .context("missing producer report path")?;
            if snapshot::file_digest(&safe_file(bundle, path)?)? != report["digest"] {
                bail!("materialized producer report failed integrity verification");
            }
            references.push(json!({"report":report["id"],"kind":report["kind"],"digest":report["digest"],"subjectDigest":report["subjectDigest"]}));
        }
        receipt["extensions"]["oyzu.dev/producer-reports"] =
            json!({"scope":"producer-target","reports":references});
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn producer_report_links_keep_scope_and_reject_tampered_or_failed_evidence() {
        let bundle = tempfile::tempdir().unwrap();
        fs::write(bundle.path().join("coverage.out"), "mode: set\n").unwrap();
        let digest = snapshot::file_digest(&bundle.path().join("coverage.out")).unwrap();
        let artifacts = vec![json!({"id":"app/primary","target":"app","digest":"artifact-digest"})];
        let actions = vec![json!({"id":"app:test","status":"succeeded"})];
        let reports = vec![
            json!({"id":"app:coverage","target":"app","action":"app:test","kind":"coverage","status":"collected","path":"coverage.out","digest":digest,"subjectDigest":"source-digest"}),
        ];
        let mut receipts = vec![json!({"artifact":"app/primary","digest":"artifact-digest"})];
        reference_reports(&mut receipts, bundle.path(), &artifacts, &actions, &reports).unwrap();
        let linked = &receipts[0]["extensions"]["oyzu.dev/producer-reports"];
        assert_eq!(linked["scope"], "producer-target");
        assert_eq!(linked["reports"][0]["digest"], digest);
        assert_eq!(linked["reports"][0]["subjectDigest"], "source-digest");
        let failed = vec![json!({"id":"app:test","status":"failed"})];
        assert!(
            reference_reports(&mut receipts, bundle.path(), &artifacts, &failed, &reports).is_err()
        );
        fs::write(bundle.path().join("coverage.out"), "tampered evidence").unwrap();
        assert!(
            reference_reports(&mut receipts, bundle.path(), &artifacts, &actions, &reports)
                .is_err()
        );
    }

    #[test]
    fn directory_copies_are_flat_complete_independent_and_verified() {
        let out = tempfile::tempdir().unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        fs::create_dir_all(out.path().join("site/assets/empty")).unwrap();
        fs::write(out.path().join("site/index.html"), "hello").unwrap();
        let artifact = super::super::directory::capture(out.path(), bundle.path(), &json!({
            "id":"frontend/primary", "producer":"frontend:package", "kind":"directory", "path":"site"
        })).unwrap();
        let actions = vec![json!({"id":"frontend:package","status":"succeeded"})];
        let inputs = vec![
            json!({"kind":"artifact","artifact":"frontend/primary","producer":"frontend:package","mount":"context/site"}),
        ];
        let artifacts = vec![artifact];
        let receipts = apply(
            workspace.path(),
            bundle.path(),
            &inputs,
            &artifacts,
            &actions,
        )
        .unwrap();
        assert_eq!(receipts[0]["digest"], artifacts[0]["digest"]);
        assert_eq!(
            fs::read(workspace.path().join("context/site/index.html")).unwrap(),
            b"hello"
        );
        assert!(workspace.path().join("context/site/assets/empty").is_dir());
        assert!(!workspace.path().join("context/site/site").exists());
        assert!(apply(
            workspace.path(),
            bundle.path(),
            &inputs,
            &artifacts,
            &actions
        )
        .is_err());
        fs::write(
            workspace.path().join("context/site/index.html"),
            "consumer changed",
        )
        .unwrap();
        super::super::directory::verify(bundle.path(), &artifacts[0]).unwrap();
        let other = tempfile::tempdir().unwrap();
        let failed = vec![json!({"id":"frontend:package","status":"failed"})];
        assert!(apply(other.path(), bundle.path(), &inputs, &artifacts, &failed).is_err());
        fs::write(bundle.path().join("site/extra"), "not recorded").unwrap();
        assert!(apply(other.path(), bundle.path(), &inputs, &artifacts, &actions).is_err());
        assert!(!other.path().join("context").exists());
    }

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
        let image: Platform = "linux/amd64".parse().unwrap();
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
                    &source,
                    None
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
                &source,
                None
            )
            .is_err());
        }
        assert!(plan(
            &[mapping("Inputs/file")],
            ".",
            &artifacts,
            &images,
            "consumer",
            &source,
            None
        )
        .is_ok());
        let mut incompatible = images.clone();
        incompatible.insert("consumer".into(), "linux/arm64".parse().unwrap());
        assert!(plan(
            &[mapping("Inputs/file")],
            ".",
            &artifacts,
            &incompatible,
            "consumer",
            &source,
            None
        )
        .unwrap_err()
        .to_string()
        .contains("incompatible artifact"));
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
            &source,
            None
        )
        .is_err());
        // A builder's source projection is applied before artifact placement.
        // Only excluded files cease to reserve their original destinations.
        let projection = snapshot::Projection::new(".", &["existing".into()]).unwrap();
        assert!(plan(
            &[mapping("existing")],
            ".",
            &artifacts,
            &images,
            "consumer",
            &source,
            Some(&projection)
        )
        .is_err());
        let projection = snapshot::Projection::new(".", &[]).unwrap();
        assert!(plan(
            &[mapping("existing")],
            ".",
            &artifacts,
            &images,
            "consumer",
            &source,
            Some(&projection)
        )
        .is_ok());
        let projected = captured.path().join("projected");
        snapshot::capture_projected(&captured.path().join("source"), &projected, &projection)
            .unwrap();
        assert!(!projected.join("existing").exists());
        assert!(root.path().join("existing").is_file());
    }
}
