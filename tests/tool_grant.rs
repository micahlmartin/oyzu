use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use oyzu::tools::{ToolGrantContext, ToolGrantOperation as Op, VerifiedToolGrant};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

const NOW: i64 = 1700000000;
fn payload() -> Value {
    serde_json::from_str(include_str!("fixtures/tool-grant/payload.json")).unwrap()
}
fn context() -> ToolGrantContext {
    ToolGrantContext {
        request_id: "12345678-1234-1234-1234-123456789abc".into(),
        context_id: "synthetic-context".into(),
        policy_revision: "synthetic-revision".into(),
        selection_digest: format!("sha256:{}", "a".repeat(64)),
        issuer: "synthetic-issuer".into(),
        tenant: "synthetic-tenant".into(),
        subject: "synthetic-subject".into(),
        revocation_epoch: 1,
        operation: Op::Exec,
    }
}
fn token(header: &str, payload: &str) -> String {
    let key = SigningKey::from_bytes(&[7; 32]);
    let signed = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(payload)
    );
    format!(
        "{signed}.{}",
        URL_SAFE_NO_PAD.encode(key.sign(signed.as_bytes()).to_bytes())
    )
}
fn signed(value: &Value) -> String {
    token(r#"{"alg":"EdDSA","kid":"test"}"#, &value.to_string())
}
fn verify(
    raw: &str,
    context: &ToolGrantContext,
    now: Instant,
) -> anyhow::Result<VerifiedToolGrant> {
    let keys = BTreeMap::from([(
        "test".to_string(),
        SigningKey::from_bytes(&[7; 32]).verifying_key(),
    )]);
    VerifiedToolGrant::verify(raw, &keys, context, NOW, now)
}
#[test]
fn signed_online_and_offline_lifetimes_are_independently_bounded() {
    let now = Instant::now();
    let ctx = context();
    let mut grant = verify(&signed(&payload()), &ctx, now).unwrap();
    grant
        .check(&ctx, NOW + 59, now + Duration::from_secs(59), false)
        .unwrap();
    assert!(grant
        .check(&ctx, NOW + 59, now + Duration::from_secs(59), true)
        .is_err());
    assert!(grant
        .check(&ctx, NOW + 60, now + Duration::from_secs(60), false)
        .is_err());
    let mut p = payload();
    p["offline_allowed"] = json!(true);
    p["not_after"] = json!(NOW + 900);
    let mut grant = verify(&signed(&p), &ctx, now).unwrap();
    assert!(grant
        .check(&ctx, NOW + 60, now + Duration::from_secs(60), false)
        .is_err());
    grant
        .check(&ctx, NOW + 899, now + Duration::from_secs(899), true)
        .unwrap();
    assert!(grant
        .check(&ctx, NOW + 899, now + Duration::from_secs(900), true)
        .is_err());
    p["not_after"] = json!(NOW + 901);
    assert!(verify(&signed(&p), &ctx, now).is_err());
    p["offline_allowed"] = json!(false);
    p["not_after"] = json!(NOW + 61);
    assert!(verify(&signed(&p), &ctx, now).is_err());
}
#[test]
fn context_clock_revocation_and_operation_checks_cannot_be_bypassed() {
    let now = Instant::now();
    let ctx = context();
    let raw = signed(&payload());
    for field in 0..8 {
        let mut changed = ctx.clone();
        match field {
            0 => changed.request_id = "other".into(),
            1 => changed.context_id = "other".into(),
            2 => changed.policy_revision = "other".into(),
            3 => changed.selection_digest = format!("sha256:{}", "b".repeat(64)),
            4 => changed.issuer = "other".into(),
            5 => changed.tenant = "other".into(),
            6 => changed.subject = "other".into(),
            _ => changed.revocation_epoch = 2,
        }
        assert!(verify(&raw, &changed, now).is_err());
        let mut grant = verify(&raw, &ctx, now).unwrap();
        assert!(grant.check(&changed, NOW, now, false).is_err());
        assert!(grant.check(&ctx, NOW, now, false).is_err());
    }
    let mut grant = verify(&raw, &ctx, now).unwrap();
    let mut other = ctx.clone();
    other.operation = Op::Install;
    assert!(grant.check(&other, NOW, now, false).is_err());
    grant
        .check(&ctx, NOW + 20, now + Duration::from_secs(20), false)
        .unwrap();
    assert!(grant
        .check(&ctx, NOW + 14, now + Duration::from_secs(21), false)
        .is_err());
    let mut grant = verify(&raw, &ctx, now).unwrap();
    grant
        .check(&ctx, NOW + 20, now + Duration::from_secs(20), false)
        .unwrap();
    assert!(grant
        .check(&ctx, NOW + 20, now + Duration::from_secs(19), false)
        .is_err());
    let mut grant = verify(&raw, &ctx, now).unwrap();
    grant.invalidate();
    assert!(grant.check(&ctx, NOW, now, false).is_err());
}
#[test]
fn signature_headers_duplicates_and_shared_invalid_shapes_fail() {
    let now = Instant::now();
    let ctx = context();
    let p = payload();
    for header in [
        r#"{"alg":"Ed25519","kid":"test"}"#,
        r#"{"alg":"EdDSA","kid":"unknown"}"#,
        r#"{"alg":"EdDSA","kid":"test","jku":"https://example.test"}"#,
        r#"{"alg":"EdDSA","kid":"test","kid":"test"}"#,
    ] {
        assert!(verify(&token(header, &p.to_string()), &ctx, now).is_err());
    }
    let raw = signed(&p);
    let mut parts: Vec<_> = raw.split('.').map(str::to_owned).collect();
    parts[2] = URL_SAFE_NO_PAD.encode([0; 64]);
    assert!(verify(&parts.join("."), &ctx, now).is_err());
    let duplicate = p.to_string().replacen('{', "{\"decision\":\"allow\",", 1);
    assert!(verify(
        &token(r#"{"alg":"EdDSA","kid":"test"}"#, &duplicate),
        &ctx,
        now
    )
    .is_err());
    for case in
        serde_json::from_str::<Vec<Value>>(include_str!("fixtures/tool-grant/invalid-shapes.json"))
            .unwrap()
    {
        let mut changed = p.clone();
        let field = case["pointer"].as_str().unwrap().trim_start_matches('/');
        if case["remove"] == true {
            changed.as_object_mut().unwrap().remove(field);
        } else {
            changed[field] = case["value"].clone();
        }
        assert!(
            verify(&signed(&changed), &ctx, now).is_err(),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn shared_valid_and_offline_shapes_agree_with_runtime() {
    let now = Instant::now();
    for case in
        serde_json::from_str::<Vec<Value>>(include_str!("fixtures/tool-grant/valid-shapes.json"))
            .unwrap()
    {
        let mut p = payload();
        p[case["pointer"].as_str().unwrap().trim_start_matches('/')] = case["value"].clone();
        let mut ctx = context();
        if p["operations"] == json!(["resolve"]) {
            ctx.operation = Op::Resolve;
        }
        assert!(verify(&signed(&p), &ctx, now).is_ok(), "{}", case["name"]);
    }
    let p: Value =
        serde_json::from_str(include_str!("fixtures/tool-grant/offline-payload.json")).unwrap();
    verify(&signed(&p), &context(), now).unwrap();
    for case in serde_json::from_str::<Vec<Value>>(include_str!(
        "fixtures/tool-grant/offline-invalid-shapes.json"
    ))
    .unwrap()
    {
        let mut changed = p.clone();
        changed[case["pointer"].as_str().unwrap().trim_start_matches('/')] = case["value"].clone();
        assert!(
            verify(&signed(&changed), &context(), now).is_err(),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn late_receipt_cannot_restart_signed_lifetime() {
    let now = Instant::now();
    let ctx = context();
    let keys = BTreeMap::from([(
        "test".to_string(),
        SigningKey::from_bytes(&[7; 32]).verifying_key(),
    )]);
    let raw = signed(&payload());
    assert!(VerifiedToolGrant::verify(&raw, &keys, &ctx, NOW - 1, now).is_err());
    assert!(VerifiedToolGrant::verify(&raw, &keys, &ctx, NOW + 60, now).is_err());
    let mut grant = VerifiedToolGrant::verify(&raw, &keys, &ctx, NOW + 59, now).unwrap();
    assert!(grant
        .check(&ctx, NOW + 59, now + Duration::from_secs(1), false)
        .is_err());
    let mut p = payload();
    p["offline_allowed"] = json!(true);
    p["not_after"] = json!(NOW + 900);
    assert!(VerifiedToolGrant::verify(&signed(&p), &keys, &ctx, NOW + 60, now).is_err());
    let wrong_keys = BTreeMap::from([(
        "test".to_string(),
        SigningKey::from_bytes(&[8; 32]).verifying_key(),
    )]);
    assert!(VerifiedToolGrant::verify(&raw, &wrong_keys, &ctx, NOW, now).is_err());
}
