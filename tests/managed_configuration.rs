use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use oyzu::config::managed::{Bootstrap, ConfigurationContext, Operation, VerifiedPolicySnapshot};
use serde_json::{json, Value};
fn fixture() -> (SigningKey, Bootstrap, ConfigurationContext, Value) {
    // Deterministic test-only signing key. Never used as a production trust root.
    let key = SigningKey::from_bytes(&[7; 32]);
    let bootstrap=Bootstrap::parse(&serde_json::to_vec(&json!({"schemaVersion":1,"kind":"management-bootstrap","organizationId":"example-org","enrollmentId":"example-device","platformUrl":"https://policy.example.test","policyKeys":[{"kid":"test","kty":"OKP","crv":"Ed25519","x":URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes())}]})).unwrap()).unwrap();
    let context = ConfigurationContext {
        organization_id: "example-org".into(),
        enrollment_id: "example-device".into(),
        subject_id: None,
        workspace_id: Some("test-workspace".into()),
        os: "linux".into(),
        architecture: "x86_64".into(),
        execution_class: "local".into(),
    };
    let payload = json!({"schemaVersion":1,"kind":"managed-policy","snapshotId":"snapshot-1","revision":"revision-1","organizationId":"example-org","enrollmentId":"example-device","audience":"oyzu-config","contextDigest":context.digest().unwrap(),"sequence":1,"issuedAt":"2026-10-01T00:00:00Z","refreshAfter":"2026-10-01T00:15:00Z","expiresAt":"2026-10-03T00:00:00Z","offline":{"localBuilds":true,"maxAgeSeconds":86400},"requiredCapabilities":["policy.constraints/v1"],"settings":{},"profiles":{}});
    (key, bootstrap, context, payload)
}
fn sign(key: &SigningKey, payload: &Value) -> String {
    let header =
        URL_SAFE_NO_PAD.encode(br#"{"alg":"Ed25519","kid":"test","typ":"oyzu-policy+jws"}"#);
    let body = URL_SAFE_NO_PAD.encode(serde_json_canonicalizer::to_vec(payload).unwrap());
    let input = format!("{header}.{body}");
    let signature = key.sign(input.as_bytes());
    format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
}
fn issued() -> i64 {
    chrono::DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")
        .unwrap()
        .timestamp()
}
#[test]
fn verifies_signature_and_exact_offline_deadline() {
    let (key, bootstrap, context, payload) = fixture();
    let envelope = sign(&key, &payload);
    let snapshot =
        VerifiedPolicySnapshot::verify(&envelope, &bootstrap, &context, issued(), None).unwrap();
    assert!(snapshot
        .authorize(Operation::LocalBuild, issued() + 86399, false, true)
        .is_ok());
    assert!(snapshot
        .authorize(Operation::LocalBuild, issued() + 86400, false, true)
        .is_err());
    assert!(snapshot
        .authorize(Operation::CiBuild, issued() + 10, false, true)
        .is_err());
    assert!(snapshot
        .authorize(Operation::Publish, issued() + 10, true, true)
        .is_err());
    assert!(snapshot
        .authorize(Operation::Sign, issued() + 10, false, true)
        .is_err());
    assert!(snapshot
        .authorize(Operation::LocalBuild, issued() + 10, false, false)
        .is_err());
}
#[test]
fn rejects_replay_rollback_changed_sequence_and_clock_rollback() {
    let (key, bootstrap, mut context, mut payload) = fixture();
    let envelope = sign(&key, &payload);
    let snapshot =
        VerifiedPolicySnapshot::verify(&envelope, &bootstrap, &context, issued(), None).unwrap();
    let high = snapshot.high_water(&bootstrap, issued() + 50).unwrap();
    assert!(VerifiedPolicySnapshot::verify(
        &envelope,
        &bootstrap,
        &context,
        issued() + 49,
        Some(&high)
    )
    .is_err());
    payload["revision"] = json!("changed");
    assert!(VerifiedPolicySnapshot::verify(
        &sign(&key, &payload),
        &bootstrap,
        &context,
        issued() + 60,
        Some(&high)
    )
    .is_err());
    payload["sequence"] = json!(2);
    assert!(VerifiedPolicySnapshot::verify(
        &sign(&key, &payload),
        &bootstrap,
        &context,
        issued() + 60,
        Some(&high)
    )
    .is_ok());
    context.subject_id = Some("different-user".into());
    assert!(
        VerifiedPolicySnapshot::verify(&envelope, &bootstrap, &context, issued() + 60, None)
            .is_err()
    );
}
#[test]
fn rejects_unknown_mandatory_semantics_and_missing_offline_fields() {
    let (key, bootstrap, context, mut payload) = fixture();
    payload["settings"] = json!({"future.setting":{"locked":true,"value":true}});
    assert!(VerifiedPolicySnapshot::verify(
        &sign(&key, &payload),
        &bootstrap,
        &context,
        issued(),
        None
    )
    .is_err());
    payload["settings"] = json!({});
    payload.as_object_mut().unwrap().remove("offline");
    assert!(VerifiedPolicySnapshot::verify(
        &sign(&key, &payload),
        &bootstrap,
        &context,
        issued(),
        None
    )
    .is_err());
}
#[test]
fn rejects_tampering_and_wrong_key() {
    let (key, bootstrap, context, payload) = fixture();
    let mut envelope = sign(&key, &payload);
    envelope.pop();
    envelope.push('x');
    assert!(
        VerifiedPolicySnapshot::verify(&envelope, &bootstrap, &context, issued(), None).is_err()
    );
    let wrong = SigningKey::from_bytes(&[8; 32]);
    assert!(VerifiedPolicySnapshot::verify(
        &sign(&wrong, &payload),
        &bootstrap,
        &context,
        issued(),
        None
    )
    .is_err());
}
