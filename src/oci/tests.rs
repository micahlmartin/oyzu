use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Write};

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn blob(files: &mut BTreeMap<String, Vec<u8>>, media: &str, bytes: Vec<u8>) -> Value {
    let id = digest(&bytes);
    let size = bytes.len();
    files.insert(format!("blobs/sha256/{}", &id[7..]), bytes);
    json!({"mediaType":media,"digest":id,"size":size})
}

fn image(files: &mut BTreeMap<String, Vec<u8>>, arch: &str, gzip: bool, bad_diff: bool) -> Value {
    let mut layer = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_ustar();
    header.set_size(5);
    header.set_mode(0o644);
    header.set_cksum();
    layer
        .append_data(&mut header, "greeting", &b"hello"[..])
        .unwrap();
    let plain = layer.into_inner().unwrap();
    let diff = if bad_diff {
        format!("sha256:{}", "0".repeat(64))
    } else {
        digest(&plain)
    };
    let (media, bytes) = if gzip {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&plain).unwrap();
        (
            "application/vnd.oci.image.layer.v1.tar+gzip",
            encoder.finish().unwrap(),
        )
    } else {
        (LAYER, plain)
    };
    let layer = blob(files, media, bytes);
    let config = blob(
        files,
        CONFIG,
        serde_json::to_vec(
            &json!({"architecture":arch,"os":"linux","rootfs":{"type":"layers","diff_ids":[diff]}}),
        )
        .unwrap(),
    );
    blob(
        files,
        MANIFEST,
        serde_json::to_vec(
            &json!({"schemaVersion":2,"mediaType":MANIFEST,"config":config,"layers":[layer]}),
        )
        .unwrap(),
    )
}

fn layout(files: &mut BTreeMap<String, Vec<u8>>, descriptor: &Value) {
    files.insert(
        "oci-layout".into(),
        br#"{"imageLayoutVersion":"1.0.0"}"#.to_vec(),
    );
    files.insert(
        "index.json".into(),
        serde_json::to_vec(&json!({"schemaVersion":2,"manifests":[descriptor]})).unwrap(),
    );
}

fn archive(files: &BTreeMap<String, Vec<u8>>, path: &Path, duplicate: bool) {
    let mut tar = tar::Builder::new(fs::File::create(path).unwrap());
    for (name, body) in files {
        let mut header = tar::Header::new_ustar();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, name, &body[..]).unwrap();
        if duplicate {
            tar.append_data(&mut header, name, &body[..]).unwrap();
        }
    }
    tar.finish().unwrap();
}

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
    crate::records::write(&temp.path().join("manifest.json"), &manifest).unwrap();
    crate::build::inspect(temp.path()).unwrap();
    manifest["artifacts"][0]["ociDigest"] = json!(format!("sha256:{}", "0".repeat(64)));
    crate::records::write(&temp.path().join("manifest.json"), &manifest).unwrap();
    assert!(crate::build::inspect(temp.path()).is_err());
}
