//! Backend identity inspection, independent of source admission or execution.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Descriptor {
    source_pin: String,
    patch_set_digest: String,
    embedding_abi: u64,
    adapter_revision: u64,
    registry_digest: Option<String>,
    plugin_digest: Option<String>,
    verifier_digest: String,
    features: Vec<String>,
}

#[derive(Serialize)]
pub struct BackendInspection {
    pub digest: String,
    pub source_pin: String,
    pub validation: &'static str,
}

/// Read at most 2 MiB plus an overflow probe; never resolve source references,
/// load configuration, mutate input or grant backend admission.
pub fn inspect(path: &Path) -> Result<BackendInspection> {
    let bytes = super::read_record(path, MAX_BYTES)?;
    parse(&bytes)
}

fn parse(bytes: &[u8]) -> Result<BackendInspection> {
    let value = crate::config::policy::strict_json_limit(bytes, MAX_BYTES)?;
    ensure!(
        value.get("registry_digest").is_some() && value.get("plugin_digest").is_some(),
        "backend descriptor requires explicit nullable identities"
    );
    let descriptor: Descriptor = serde_json::from_value(value)?;
    ensure!(
        descriptor.source_pin.len() == 40
            && descriptor
                .source_pin
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "backend source must be an exact lowercase Git revision"
    );
    ensure!(
        (1..=MAX_INTEGER).contains(&descriptor.embedding_abi)
            && (1..=MAX_INTEGER).contains(&descriptor.adapter_revision),
        "backend revisions must be positive safe integers"
    );
    for digest in [
        Some(&descriptor.patch_set_digest),
        descriptor.registry_digest.as_ref(),
        descriptor.plugin_digest.as_ref(),
        Some(&descriptor.verifier_digest),
    ]
    .into_iter()
    .flatten()
    {
        super::lock::digest(digest)?;
    }
    ensure!(
        descriptor.features.windows(2).all(|pair| pair[0] < pair[1]),
        "backend features must be sorted and unique"
    );
    let digest = crate::records::digest("oyzu.backend.v1", &serde_json::to_value(&descriptor)?)?;
    Ok(BackendInspection {
        digest,
        source_pin: descriptor.source_pin,
        validation: "structure-and-identity-only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    const FIXTURE: &str = include_str!("../../tests/fixtures/tool-backend/descriptor.json");

    #[test]
    fn shared_shapes_and_runtime_semantics() {
        for (cases, accepted) in [
            (
                include_str!("../../tests/fixtures/tool-backend/valid-shapes.json"),
                true,
            ),
            (
                include_str!("../../tests/fixtures/tool-backend/invalid-shapes.json"),
                false,
            ),
        ] {
            for case in serde_json::from_str::<Vec<Value>>(cases).unwrap() {
                let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
                let field = case["pointer"].as_str().unwrap().strip_prefix('/').unwrap();
                if case["remove"] == true {
                    value.as_object_mut().unwrap().remove(field);
                } else {
                    value[field] = case["value"].clone();
                }
                assert_eq!(
                    parse(&serde_json::to_vec(&value).unwrap()).is_ok(),
                    accepted,
                    "{}",
                    case["name"]
                );
            }
        }
        let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
        value["features"] = serde_json::json!(["z", "a"]);
        assert!(parse(&serde_json::to_vec(&value).unwrap()).is_err());
        assert!(parse(
            FIXTURE
                .replacen('{', "{\"source_pin\":\"duplicate\",", 1)
                .as_bytes()
        )
        .is_err());
        assert!(parse(&vec![b' '; MAX_BYTES + 1]).is_err());
    }

    #[test]
    fn canonical_identity_matches_golden_and_binds_every_field() {
        let original = parse(FIXTURE.as_bytes()).unwrap();
        assert_eq!(
            original.digest,
            include_str!("../../tests/fixtures/tool-backend/descriptor.sha256").trim()
        );
        let compact: Value = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(
            parse(&serde_json::to_vec(&compact).unwrap())
                .unwrap()
                .digest,
            original.digest
        );
        for (field, replacement) in [
            ("source_pin", serde_json::json!("d".repeat(40))),
            (
                "patch_set_digest",
                serde_json::json!(format!("sha256:{}", "d".repeat(64))),
            ),
            ("embedding_abi", serde_json::json!(2)),
            ("adapter_revision", serde_json::json!(2)),
            (
                "registry_digest",
                serde_json::json!(format!("sha256:{}", "d".repeat(64))),
            ),
            (
                "plugin_digest",
                serde_json::json!(format!("sha256:{}", "d".repeat(64))),
            ),
            (
                "verifier_digest",
                serde_json::json!(format!("sha256:{}", "d".repeat(64))),
            ),
            ("features", serde_json::json!([])),
        ] {
            let mut changed = compact.clone();
            changed[field] = replacement;
            assert_ne!(
                parse(&serde_json::to_vec(&changed).unwrap())
                    .unwrap()
                    .digest,
                original.digest,
                "{field}"
            );
        }
    }
}
