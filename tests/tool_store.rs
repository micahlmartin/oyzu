use std::{fs, process::Command};

#[test]
fn cli_observes_real_payload_changes_without_executing_or_mutating_it() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let payload = root.join("工具 payload");
    fs::create_dir(&payload).unwrap();
    fs::create_dir(payload.join("bin")).unwrap();
    fs::write(payload.join("bin/tool"), b"payload bytes, never executed").unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_oyzu"))
            .args(["tools", "inspect-tree", payload.to_str().unwrap()])
            .output()
            .unwrap()
    };
    let first = run();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first["validation"], "payload-observation-only");
    assert_eq!(first["entries"].as_array().unwrap().len(), 2);
    assert_eq!(
        fs::read(payload.join("bin/tool")).unwrap(),
        b"payload bytes, never executed"
    );
    fs::write(payload.join("bin/tool"), b"modified payload").unwrap();
    let second = run();
    assert!(second.status.success());
    let second: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_ne!(first["digest"], second["digest"]);
    fs::hard_link(payload.join("bin/tool"), root.join("outside")).unwrap();
    let failure = run();
    assert_eq!(failure.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&failure.stderr).contains("hardlink"));
    assert_eq!(fs::read(root.join("outside")).unwrap(), b"modified payload");
}
