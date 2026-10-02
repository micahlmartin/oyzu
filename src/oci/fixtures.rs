use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Write};

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn blob(files: &mut BTreeMap<String, Vec<u8>>, media: &str, bytes: Vec<u8>) -> Value {
    let id = digest(&bytes);
    let size = bytes.len();
    files.insert(format!("blobs/sha256/{}", &id[7..]), bytes);
    json!({"mediaType":media,"digest":id,"size":size})
}

pub(crate) fn image(
    files: &mut BTreeMap<String, Vec<u8>>,
    arch: &str,
    gzip: bool,
    bad_diff: bool,
) -> Value {
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

pub(crate) fn layout(files: &mut BTreeMap<String, Vec<u8>>, descriptor: &Value) {
    files.insert(
        "oci-layout".into(),
        br#"{"imageLayoutVersion":"1.0.0"}"#.to_vec(),
    );
    files.insert(
        "index.json".into(),
        serde_json::to_vec(&json!({"schemaVersion":2,"manifests":[descriptor]})).unwrap(),
    );
}

pub(crate) fn archive(files: &BTreeMap<String, Vec<u8>>, path: &Path, duplicate: bool) {
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
