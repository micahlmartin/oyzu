use super::{fixtures::*, *};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, io::Write};
mod assembly;

#[test]
fn verifies_image_and_complete_platform_index_without_extracting_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("image.tar");
    let mut files = BTreeMap::new();
    let first = image(&mut files, "amd64", true, false);
    layout(&mut files, &first);
    archive(&files, &path, false);
    let verified = verify(&path).unwrap();
    assert_eq!(verified.kind, "oci-image");
    assert_eq!(verified.digest, first["digest"]);
    assert_eq!(verified.platforms.len(), 1);
    let inspection = crate::build::inspect(&path).unwrap();
    assert_eq!(inspection["ociDigest"], first["digest"]);
    assert_eq!(inspection["verification"], "content-integrity");
    let second = image(&mut files, "arm64", false, false);
    let index = blob(
        &mut files,
        INDEX,
        serde_json::to_vec(
            &json!({"schemaVersion":2,"mediaType":INDEX,"manifests":[first,second]}),
        )
        .unwrap(),
    );
    layout(&mut files, &index);
    archive(&files, &path, false);
    let verified = verify(&path).unwrap();
    assert_eq!(verified.kind, "oci-index");
    assert_eq!(verified.platforms.len(), 2);
    assert_eq!(verified.digest, index["digest"]);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    let removed = format!(
        "blobs/sha256/{}",
        second["digest"]
            .as_str()
            .unwrap()
            .strip_prefix("sha256:")
            .unwrap()
    );
    files.remove(&removed);
    archive(&files, &path, false);
    assert!(verify(&path)
        .unwrap_err()
        .to_string()
        .contains("missing a referenced blob"));
}

#[test]
fn runtime_configuration_types_preserve_optional_nulls_and_extensions() {
    validate_runtime(&json!({"Entrypoint":null,"Cmd":["hello"],"Env":["NAME=value"],"Labels":{"example":"value"},"vendor":42})).unwrap();
    for value in [
        json!([]),
        json!({"Entrypoint":"/server"}),
        json!({"Env":[42]}),
        json!({"User":1}),
        json!({"Labels":{"bad":false}}),
    ] {
        assert!(validate_runtime(&value).is_err());
    }
}

#[test]
fn engine_image_assertions_record_integrity_platform_failures_and_refuse_stale_reports() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("image.tar");
    let mut files = BTreeMap::new();
    let root = image(&mut files, "amd64", true, false);
    layout(&mut files, &root);
    archive(&files, &path, false);
    let verify = |arch: &str, report: &str| {
        let image = crate::executor::Image {
            reference: "fixture".into(),
            digest: format!("sha256:{}", "1".repeat(64)),
            os: "linux".into(),
            arch: "arm64".into(),
        };
        let mode = crate::executor::Mode::OciValidation {
            input: "image.tar".into(),
            report: report.into(),
        };
        let argv = mode.argv(&format!("linux/{arch}")).unwrap();
        crate::executor::execute_mode(
            crate::executor::Request {
                image: &image,
                workspace: temp.path(),
                output: temp.path(),
                cwd: "/workspace",
                argv: &argv,
                env: &BTreeMap::new(),
                stdout: &temp.path().join("stdout"),
                stderr: &temp.path().join("stderr"),
                timeout: std::time::Duration::from_secs(10),
                name: "validation-fixture",
            },
            &[],
            &mode,
            &[],
            &format!("linux/{arch}").parse().unwrap(),
        )
    };
    assert_eq!(verify("amd64", "passed.xml").unwrap().code, 0);
    assert_eq!(
        crate::reports::junit_summary(&temp.path().join("passed.xml")).unwrap()["passed"],
        2
    );
    assert_eq!(verify("arm64", "platform.xml").unwrap().code, 1);
    assert_eq!(
        crate::reports::junit_summary(&temp.path().join("platform.xml")).unwrap()["failed"],
        1
    );
    assert!(verify("amd64", "passed.xml").is_err());
    assert!(verify("amd64", "../escape.xml").is_err());
    fs::write(&path, b"corrupt archive").unwrap();
    assert_eq!(verify("amd64", "failed.xml").unwrap().code, 1);
    let summary = crate::reports::junit_summary(&temp.path().join("failed.xml")).unwrap();
    assert_eq!(summary["failed"], 1);
    assert_eq!(summary["skipped"], 1);
}

#[test]
fn rejects_tampered_blobs_sizes_layers_and_platform_claims() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("image.tar");
    for case in ["blob", "size", "diff", "platform"] {
        let mut files = BTreeMap::new();
        let mut root = image(&mut files, "amd64", true, case == "diff");
        match case {
            "blob" => files.values_mut().next().unwrap().push(0),
            "size" => root["size"] = json!(root["size"].as_u64().unwrap() + 1),
            "platform" => root["platform"] = json!({"os":"linux","architecture":"arm64"}),
            _ => (),
        }
        layout(&mut files, &root);
        archive(&files, &path, false);
        assert!(verify(&path).is_err(), "accepted {case}");
    }
}

#[test]
fn rejects_duplicate_paths_unknown_entries_and_trailing_archives() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("image.tar");
    let mut files = BTreeMap::new();
    let root = image(&mut files, "amd64", false, false);
    layout(&mut files, &root);
    archive(&files, &path, true);
    assert!(verify(&path).is_err());
    files.insert("unexpected".into(), vec![]);
    archive(&files, &path, false);
    assert!(verify(&path).is_err());
    files.remove("unexpected");
    archive(&files, &path, false);
    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"hidden second archive")
        .unwrap();
    assert!(verify(&path).is_err());
}

#[test]
fn rejects_ambiguous_platform_indices_and_links_in_layout() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("image.tar");
    let mut files = BTreeMap::new();
    let image = image(&mut files, "amd64", false, false);
    let index = blob(
        &mut files,
        INDEX,
        serde_json::to_vec(&json!({"schemaVersion":2,"mediaType":INDEX,"manifests":[image,image]}))
            .unwrap(),
    );
    layout(&mut files, &index);
    archive(&files, &path, false);
    assert!(verify(&path)
        .unwrap_err()
        .to_string()
        .contains("duplicate platform"));
    let mut tar = tar::Builder::new(fs::File::create(&path).unwrap());
    let mut header = tar::Header::new_ustar();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    header.set_link_name("/host/private").unwrap();
    header.set_cksum();
    tar.append_data(&mut header, "oci-layout", std::io::empty())
        .unwrap();
    tar.finish().unwrap();
    assert!(verify(&path).is_err());
}

#[test]
fn bundle_inspection_rejects_wrong_publication_identity_even_with_valid_archive_hash() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("image.tar");
    let mut files = BTreeMap::new();
    let root = image(&mut files, "amd64", true, false);
    layout(&mut files, &root);
    archive(&files, &path, false);
    fs::write(temp.path().join("envelope.json"), b"{}").unwrap();
    let mut manifest = json!({"kind":"build-manifest","status":"failed","envelopePath":"envelope.json","envelopeDigest":crate::snapshot::file_digest(&temp.path().join("envelope.json")).unwrap(),"artifacts":[{"kind":"oci-image","path":"image.tar","size":fs::metadata(&path).unwrap().len(),"digest":crate::snapshot::file_digest(&path).unwrap(),"ociDigest":root["digest"]}],"reports":[]});
    manifest["targets"] = json!([{"id":"image", "platform":{"os":"linux", "arch":"amd64"}}]);
    manifest["artifacts"][0]["target"] = json!("image");
    crate::records::write(&temp.path().join("manifest.json"), &manifest).unwrap();
    crate::build::inspect(temp.path()).unwrap();
    manifest["targets"][0]["platform"]["arch"] = json!("arm64");
    crate::records::write(&temp.path().join("manifest.json"), &manifest).unwrap();
    assert!(crate::build::inspect(temp.path())
        .unwrap_err()
        .to_string()
        .contains("image platform"));
    manifest["targets"][0]["platform"]["arch"] = json!("amd64");
    manifest["artifacts"][0]["ociDigest"] = json!(format!("sha256:{}", "0".repeat(64)));
    crate::records::write(&temp.path().join("manifest.json"), &manifest).unwrap();
    assert!(crate::build::inspect(temp.path()).is_err());
}
