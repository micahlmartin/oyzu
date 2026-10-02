use super::*;

fn hash(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

fn tool(id: &str) -> Tool {
    let mut tool = Tool {
        key: String::new(),
        id: id.into(),
        version: "22.1.0".into(),
        backend_digest: hash('a'),
        options: BTreeMap::new(),
        distribution: vec![Distribution {
            platform: "linux/amd64/gnu".into(),
            digest: hash('b'),
            size: 32,
            source_id: "node-releases".into(),
            artifact_id: "node-22-archive".into(),
            layout_digest: hash('c'),
            dependencies: vec![],
            package_closure_digest: None,
            verification: Verification {
                kind: VerificationKind::DigestOnly,
                evidence_digest: hash('d'),
                verifier_digest: hash('e'),
                subject_digest: hash('b'),
            },
        }],
    };
    tool.key = tool_key(&tool).unwrap();
    tool
}

fn fixture() -> Lock {
    let tool = tool("core:node");
    Lock {
        selections: vec![],
        format: 2,
        environment: vec![Environment {
            scope: ".".into(),
            profile: "default".into(),
            request_digest: hash('f'),
            roots: vec![tool.key.clone()],
            requests: BTreeMap::from([(tool.id.clone(), "22".into())]),
        }],
        tool: vec![tool],
    }
}

fn encoded(lock: &Lock) -> Vec<u8> {
    toml::to_string(lock).unwrap().into_bytes()
}
fn rejected(lock: &Lock, message: &str) {
    let error = parse(&encoded(lock)).unwrap_err().to_string();
    assert!(error.contains(message), "expected {message}, got {error}");
}

#[test]
fn parses_and_round_trips_with_omitted_optional_closure() {
    let lock = parse(&encoded(&fixture())).unwrap();
    assert_eq!(encoded(&lock), encoded(&fixture()));
    assert!(lock.tool[0].distribution[0]
        .package_closure_digest
        .is_none());
}

#[test]
fn rejects_unknown_duplicate_and_unsupported_input() {
    let bytes = encoded(&fixture());
    for prefix in ["format = 2\n", "unknown = true\n"] {
        let input = [prefix.as_bytes(), &bytes].concat();
        assert!(parse(&input).is_err());
    }
    let mut lock = fixture();
    lock.format = 1;
    rejected(&lock, "unsupported tool lock format");
    assert!(parse(&[0xff]).is_err());
    assert!(parse(&vec![b' '; MAX_BYTES + 1]).is_err());
}

#[test]
fn rejects_identity_and_verification_tampering() {
    let mut lock = fixture();
    lock.tool[0].version = "24.0.0".into();
    rejected(&lock, "tool key does not match");
    let mut lock = fixture();
    lock.tool[0].distribution[0].digest = hash('0');
    rejected(&lock, "verification subject differs");
    let mut lock = fixture();
    lock.tool[0].distribution[0].size = 9_007_199_254_740_992;
    rejected(&lock, "canonical integer range");
}

#[test]
fn rejects_missing_cycle_and_wrong_platform_dependencies() {
    let mut lock = fixture();
    lock.tool[0].distribution[0].dependencies.push(hash('0'));
    rejected(&lock, "missing tool dependency");
    let mut lock = fixture();
    let key = lock.tool[0].key.clone();
    lock.tool[0].distribution[0].dependencies.push(key);
    rejected(&lock, "dependency cycle");
    let mut lock = fixture();
    let mut dependency = tool("core:python");
    dependency.distribution[0].platform = "windows/amd64/msvc".into();
    lock.tool[0].distribution[0]
        .dependencies
        .push(dependency.key.clone());
    lock.tool.push(dependency);
    rejected(&lock, "no matching platform");
}

#[test]
fn rejects_ambiguous_closure_but_allows_different_environments() {
    let mut lock = fixture();
    let mut second = tool("core:node");
    second.version = "24.0.0".into();
    second.key = tool_key(&second).unwrap();
    lock.tool[0].distribution[0]
        .dependencies
        .push(second.key.clone());
    lock.tool.push(second.clone());
    rejected(&lock, "ambiguous canonical ID");
    lock.tool[0].distribution[0].dependencies.clear();
    let mut environment = lock.environment[0].clone();
    environment.scope = "apps/second".into();
    environment.roots = vec![second.key];
    environment.requests.insert("core:node".into(), "24".into());
    lock.environment.push(environment);
    assert!(parse(&encoded(&lock)).is_ok());
}

#[test]
fn rejects_unreachable_and_duplicate_records() {
    let mut lock = fixture();
    lock.tool.push(tool("core:python"));
    rejected(&lock, "unreachable");
    let mut lock = fixture();
    lock.tool.push(lock.tool[0].clone());
    rejected(&lock, "duplicate tool key");
    let mut lock = fixture();
    lock.environment.push(lock.environment[0].clone());
    rejected(&lock, "duplicate locked scope/profile");
    let mut lock = fixture();
    lock.environment[0].roots.push(lock.tool[0].key.clone());
    rejected(&lock, "duplicate set entry");
}

#[test]
fn rejects_paths_urls_platforms_and_request_mismatch() {
    for scope in [
        "../escape",
        "/root",
        "C:/root",
        "apps//one",
        "apps/./one",
        "apps\\one",
    ] {
        let mut lock = fixture();
        lock.environment[0].scope = scope.into();
        rejected(&lock, "normalized workspace-relative");
    }
    let mut lock = fixture();
    lock.tool[0].distribution[0].source_id = "https://secret.example".into();
    rejected(&lock, "cannot be a URL");
    let mut lock = fixture();
    lock.tool[0].distribution[0].platform = "windows/amd64/gnu".into();
    rejected(&lock, "unsupported tool platform");
    let mut lock = fixture();
    lock.environment[0].requests.clear();
    rejected(&lock, "requests must match");
}

#[test]
fn dependency_depth_is_bounded() {
    let mut lock = fixture();
    for index in 1..65 {
        let next = tool(&format!("core:tool{index}"));
        lock.tool[index - 1].distribution[0]
            .dependencies
            .push(next.key.clone());
        lock.tool.push(next);
    }
    rejected(&lock, "depth exceeds 64");
    lock.tool.pop();
    lock.tool[63].distribution[0].dependencies.clear();
    assert!(parse(&encoded(&lock)).is_ok());
}

#[test]
fn selection_binds_artifacts_and_transitive_dependencies_not_array_order() {
    let mut lock = fixture();
    let dependency = tool("core:python");
    lock.tool[0].distribution[0]
        .dependencies
        .push(dependency.key.clone());
    lock.tool.push(dependency);
    let original = parse(&encoded(&lock)).unwrap().selections[0].digest.clone();
    lock.tool.reverse();
    assert_eq!(
        parse(&encoded(&lock)).unwrap().selections[0].digest,
        original
    );
    // Tool identity is unchanged, but substituting a different archive even
    // with matching verification metadata must change the selection identity.
    lock.tool[0].distribution[0].digest = hash('0');
    lock.tool[0].distribution[0].verification.subject_digest = hash('0');
    assert_ne!(
        parse(&encoded(&lock)).unwrap().selections[0].digest,
        original
    );
}

#[test]
fn cannot_inject_computed_selection_records() {
    let input = [b"selections = []\n".as_slice(), &encoded(&fixture())].concat();
    assert!(parse(&input).is_err());
}

#[test]
fn installation_identity_binds_dependencies_but_ignores_other_platforms() {
    let mut lock = fixture();
    let dependency = tool("core:python");
    let root_key = lock.tool[0].key.clone();
    let dependency_key = dependency.key.clone();
    lock.tool[0].distribution[0]
        .dependencies
        .push(dependency.key.clone());
    lock.tool.push(dependency);
    let original = parse(&encoded(&lock)).unwrap().selections[0]
        .installation_keys
        .clone();
    for tool in &mut lock.tool {
        let mut extra = tool.distribution[0].clone();
        extra.platform = "darwin/arm64/native".into();
        extra.digest = hash('1');
        extra.verification.subject_digest = hash('1');
        tool.distribution.push(extra);
    }
    let expanded = parse(&encoded(&lock)).unwrap();
    assert_eq!(
        expanded
            .selections
            .iter()
            .find(|s| s.platform == "linux/amd64/gnu")
            .unwrap()
            .installation_keys,
        original
    );
    lock.tool[1].distribution[0].digest = hash('2');
    lock.tool[1].distribution[0].verification.subject_digest = hash('2');
    let changed = parse(&encoded(&lock)).unwrap();
    let changed = &changed
        .selections
        .iter()
        .find(|s| s.platform == "linux/amd64/gnu")
        .unwrap()
        .installation_keys;
    assert_ne!(changed[&root_key], original[&root_key]);
    assert_ne!(changed[&dependency_key], original[&dependency_key]);
}
