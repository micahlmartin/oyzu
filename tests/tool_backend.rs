use std::{fs, process::Command};

#[test]
fn descriptor_cli_is_read_only_and_ignores_ambient_configuration() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("descriptor.json");
    let original = include_bytes!("fixtures/tool-backend/descriptor.json");
    fs::write(&path, original).unwrap();
    for name in ["mise.toml", "oyzu.toml"] {
        fs::write(temp.path().join(name), "invalid = [").unwrap();
    }
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_oyzu"))
            .arg("-C")
            .arg(temp.path())
            .args(["tools", "inspect-backend", "descriptor.json"])
            .env("MISE_CONFIG_FILE", "missing")
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .output()
            .unwrap()
    };
    let output = run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["digest"],
        include_str!("fixtures/tool-backend/descriptor.sha256").trim()
    );
    assert_eq!(value["validation"], "structure-and-identity-only");
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::write(&path, b"{\"source_pin\":\"main\"}").unwrap();
    assert_eq!(run().status.code(), Some(2));
    assert_eq!(fs::read(&path).unwrap(), b"{\"source_pin\":\"main\"}");
}
