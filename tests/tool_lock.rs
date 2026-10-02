use std::{fs, process::Command};

#[test]
fn inspection_is_offline_read_only_and_rejects_tampering() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("oyzu.lock");
    let original = include_bytes!("fixtures/tool-lock/valid.toml");
    fs::write(&path, original).unwrap();
    // Inspection must not execute or parse unrelated project/mise input.
    fs::write(temp.path().join("mise.toml"), "invalid = [").unwrap();
    fs::write(temp.path().join("oyzu.toml"), "invalid = [").unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_oyzu"))
            .args(["-C", temp.path().to_str().unwrap(), "tools", "inspect-lock"])
            .env("MISE_CONFIG_FILE", "does-not-exist")
            .env("HTTP_PROXY", "http://127.0.0.1:1")
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .output()
            .unwrap()
    };
    let result = run();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(output["validation"], "structure-and-identity-only");
    assert_eq!(output["tools"], 1);
    assert_eq!(
        output["selections"][0]["digest"],
        include_str!("fixtures/tool-lock/selection.sha256").trim()
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    let changed = String::from_utf8(original.to_vec())
        .unwrap()
        .replace("22.1.0", "24.0.0");
    fs::write(&path, &changed).unwrap();
    let failure = run();
    assert_eq!(failure.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&failure.stderr).contains("tool key does not match"));
    assert_eq!(fs::read_to_string(path).unwrap(), changed);
}
