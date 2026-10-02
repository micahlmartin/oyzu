mod tool_store_fixture;
use oyzu::{
    config::{
        constraints::Constraints,
        registry::{Registry, Scope},
        resolve::{resolve, EffectiveConfig, Selection},
        sources::ConfigSource,
    },
    tools,
};
use std::{collections::BTreeMap, fs, path::Path};

fn effective(text: &str) -> EffectiveConfig {
    let registry = Registry::default();
    let source = ConfigSource::parse(
        "fixture",
        Path::new("."),
        Scope::Project,
        false,
        text,
        &registry,
    )
    .unwrap();
    resolve(
        &[source],
        &registry,
        false,
        &Selection::default(),
        Constraints::default(),
        false,
    )
    .unwrap()
}
fn catalog() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("node".into(), "core:node".into()),
        ("nodejs".into(), "core:node".into()),
    ])
}
fn identity(text: &str) -> tools::ToolRequestIdentity {
    tools::project_tool_requests(&effective(text), &catalog(), &BTreeMap::new(), &[]).unwrap()
}

#[test]
fn canonical_requests_exclude_environment_and_preserve_backend_version_syntax() {
    let first = identity("[tools]\nnode='22'\n[env]\nSECRET='first'");
    let second = identity("[tools]\n'core:node'='22'\n[env]\nSECRET='second'");
    assert_eq!(first.digest, second.digest);
    assert_eq!(
        first.digest,
        "sha256:ea7616dfb64b2072eaee960143b4446718664c3de8e967b88deac20947f62fbf"
    );
    assert_eq!(
        first.requests,
        BTreeMap::from([("core:node".into(), "22".into())])
    );
    assert!(!serde_json::to_string(&first).unwrap().contains("SECRET"));
    let range = identity("[tools]\nnode='>=22 <24'");
    assert_eq!(range.requests["core:node"], ">=22 <24");
    assert_ne!(first.digest, range.digest);
    assert_eq!(
        identity("[tools]\nnode='20'\n[profile]\ndefault='dev'\n[profiles.dev.tools]\nnode='22'")
            .digest,
        first.digest
    );
}

#[test]
fn constraints_and_capabilities_bind_identity_with_order_independent_sets() {
    let config = effective("[tools]\nnode='22'");
    let constraints = BTreeMap::from([("core:node".into(), vec![">=20".into(), "<25".into()])]);
    let first = tools::project_tool_requests(
        &config,
        &catalog(),
        &constraints,
        &["prebuilt".into(), "exec".into()],
    )
    .unwrap();
    let reversed = BTreeMap::from([("core:node".into(), vec!["<25".into(), ">=20".into()])]);
    assert_eq!(
        first.digest,
        tools::project_tool_requests(
            &config,
            &catalog(),
            &reversed,
            &["exec".into(), "prebuilt".into()]
        )
        .unwrap()
        .digest
    );
    assert_ne!(
        first.digest,
        tools::project_tool_requests(&config, &catalog(), &constraints, &["exec".into()])
            .unwrap()
            .digest
    );
    assert_ne!(
        first.digest,
        tools::project_tool_requests(
            &config,
            &catalog(),
            &BTreeMap::new(),
            &["exec".into(), "prebuilt".into()]
        )
        .unwrap()
        .digest
    );
}

#[test]
fn rejects_unknown_duplicate_and_rebound_aliases_and_duplicate_sets() {
    for text in [
        "[tools]\nunknown='22'",
        "[tools]\nnode='22'\nnodejs='22'",
        "[tools]\nnode='22'\n'core:node'='24'",
    ] {
        assert!(
            tools::project_tool_requests(&effective(text), &catalog(), &BTreeMap::new(), &[])
                .is_err()
        );
    }
    let config = effective("[tools]\nnode='22'");
    let mut aliases = catalog();
    aliases.insert("core:node".into(), "other:node".into());
    assert!(tools::project_tool_requests(&config, &aliases, &BTreeMap::new(), &[]).is_err());
    let duplicates = BTreeMap::from([("core:node".into(), vec![">=20".into(), ">=20".into()])]);
    assert!(tools::project_tool_requests(&config, &catalog(), &duplicates, &[]).is_err());
    assert!(tools::project_tool_requests(
        &config,
        &catalog(),
        &BTreeMap::new(),
        &["exec".into(), "exec".into()]
    )
    .is_err());
    let oversized = BTreeMap::from([("core:node".into(), vec!["x".repeat(16385)])]);
    assert!(tools::project_tool_requests(&config, &catalog(), &oversized, &[]).is_err());
}

#[test]
fn computed_request_identity_drives_frozen_selection_and_stale_config_denial() {
    let temp = tool_store_fixture::directory().unwrap();
    let requests = identity("[tools]\nnode='22'");
    let mut lock: toml::Value =
        toml::from_str(include_str!("fixtures/tool-lock/valid.toml")).unwrap();
    lock["environment"][0]["request_digest"] = requests.digest.clone().into();
    let bytes = toml::to_string(&lock).unwrap();
    let path = temp.path().join("oyzu.lock");
    fs::write(&path, &bytes).unwrap();
    tools::select_for_tool_requests(
        temp.path(),
        temp.path(),
        "default",
        &requests,
        "linux/amd64/gnu",
    )
    .unwrap();
    let changed = identity("[tools]\nnode='24'");
    assert!(tools::select_for_tool_requests(
        temp.path(),
        temp.path(),
        "default",
        &changed,
        "linux/amd64/gnu"
    )
    .unwrap_err()
    .to_string()
    .contains("TOOL_LOCK_STALE"));
    assert_eq!(fs::read_to_string(&path).unwrap(), bytes);
    lock["environment"][0]["requests"]["core:node"] = "24".into();
    fs::write(&path, toml::to_string(&lock).unwrap()).unwrap();
    assert!(tools::select_for_tool_requests(
        temp.path(),
        temp.path(),
        "default",
        &requests,
        "linux/amd64/gnu"
    )
    .unwrap_err()
    .to_string()
    .contains("locked request map"));
}

#[test]
fn normalized_request_record_roundtrips_projection_and_rejects_wire_drift() {
    use serde_json::json;
    use std::collections::BTreeSet;
    let projected = tools::project_tool_requests(
        &effective("[tools]\nnode='>=22 <24'"),
        &catalog(),
        &BTreeMap::from([("core:node".into(), vec![">=20".into(), "<25".into()])]),
        &["prebuilt".into(), "exec".into()],
    )
    .unwrap();
    let admitted = BTreeSet::from(["core:node".to_owned()]);
    let encoded = serde_json::to_vec(&projected).unwrap();
    let parsed = tools::ToolRequestIdentity::parse(&encoded, &admitted).unwrap();
    assert_eq!(parsed.digest, projected.digest);
    assert_eq!(parsed.requests["core:node"], ">=22 <24");
    assert_eq!(parsed.native_constraints, projected.native_constraints);
    assert_eq!(
        parsed.required_capabilities,
        projected.required_capabilities
    );
    assert!(tools::ToolRequestIdentity::parse(&encoded, &BTreeSet::new()).is_err());
    let original = serde_json::to_value(&projected).unwrap();
    for (pointer, value) in [
        ("/requests/core:node", json!("24")),
        ("/native_constraints/core:node", json!([">=20", "<25"])),
        ("/native_constraints/core:node", json!(["<25", "<25"])),
        ("/required_capabilities", json!(["prebuilt", "exec"])),
        ("/required_capabilities", json!(["exec", "exec"])),
        ("/digest", json!(format!("sha256:{}", "0".repeat(64)))),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert!(
            tools::ToolRequestIdentity::parse(&serde_json::to_vec(&changed).unwrap(), &admitted)
                .is_err(),
            "{pointer}"
        );
    }
    let mut unknown = original;
    unknown["command"] = json!("not a worker command");
    assert!(
        tools::ToolRequestIdentity::parse(&serde_json::to_vec(&unknown).unwrap(), &admitted)
            .is_err()
    );
    let duplicate = format!(
        "{{\"digest\":\"{}\",{}",
        projected.digest,
        std::str::from_utf8(&encoded)
            .unwrap()
            .trim_start_matches('{')
    );
    assert!(tools::ToolRequestIdentity::parse(duplicate.as_bytes(), &admitted).is_err());
}
