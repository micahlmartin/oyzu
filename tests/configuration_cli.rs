use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};
fn run(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_oyzu"))
        .arg("--root")
        .arg(root)
        .arg("-C")
        .arg(root)
        .args(args)
        .env_remove("OYZU_PROFILE")
        .env_remove("CI")
        .env_remove("GITHUB_ACTIONS")
        .env_remove("GITLAB_CI")
        .output()
        .unwrap()
}
fn value(root: &Path, args: &[&str]) -> Value {
    let result = run(root, args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
#[test]
fn real_cli_inspection_profiles_and_lossless_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("oyzu.toml"),"# retained\n[profile]\ndefault='dev'\n[env]\nTOKEN='redact-me'\n[profiles.dev.build]\njobs=3\n[future]\noption=true\n").unwrap();
    assert_eq!(value(root, &["config", "get", "build.jobs"]), json!(3));
    let result = value(root, &["config", "explain"]);
    assert!(!result.to_string().contains("redact-me"));
    assert_eq!(result["profile"], "dev");
    assert!(!run(root, &["config", "validate", "--strict"])
        .status
        .success());
    value(
        root,
        &[
            "config",
            "set",
            "build.jobs",
            "5",
            "--project",
            "--profile",
            "dev",
        ],
    );
    assert_eq!(value(root, &["config", "get", "build.jobs"]), json!(5));
    assert!(fs::read_to_string(root.join("oyzu.toml"))
        .unwrap()
        .contains("# retained"));
    value(
        root,
        &[
            "config",
            "unset",
            "build.jobs",
            "--project",
            "--profile",
            "dev",
        ],
    );
    assert!(
        value(root, &["config", "get", "build.jobs"])
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(!run(
        root,
        &[
            "config",
            "set",
            "config.allowedProfiles",
            "[]",
            "--project",
            "--json-value"
        ]
    )
    .status
    .success());
    assert!(!run(root, &["config", "set", "build.jobs", "3"])
        .status
        .success());
}
#[test]
fn nested_target_values_do_not_leak_and_selected_profile_is_shared() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("build.yaml"),
        "alpha:\n  uses: node/package\n  path: alpha\nbeta:\n  uses: node/package\n  path: beta\n",
    )
    .unwrap();
    for name in ["alpha", "beta"] {
        fs::create_dir(root.join(name)).unwrap();
        fs::write(root.join(name).join("package.json"),r#"{"name":"example","version":"1.0.0","scripts":{"probe":"node -e \"console.log(process.env.SCOPE)\""}}"#).unwrap();
    }
    fs::write(root.join("oyzu.toml"), "[env]\nSCOPE='root'\n").unwrap();
    fs::write(
        root.join("alpha/oyzu.toml"),
        "[profiles.special.env]\nSCOPE='alpha'\n",
    )
    .unwrap();
    let tasks = value(root, &["--profile", "special", "run", "list", "--json"]);
    assert_eq!(tasks["alpha:probe"]["env"]["SCOPE"], "[REDACTED]");
    assert_eq!(tasks["beta:probe"]["env"]["SCOPE"], "[REDACTED]");
    let workspace = oyzu::discovery::discover_with_options(
        root,
        None,
        &oyzu::config::session::Options {
            root: Some(root.into()),
            profile: Some("special".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(workspace.tasks["alpha:probe"].env["SCOPE"], "alpha");
    assert_eq!(workspace.tasks["beta:probe"].env["SCOPE"], "root");
    fs::write(root.join("oyzu.local.toml"), "[env]\nSCOPE='root-local'\n").unwrap();
    let tasks = value(root, &["--profile", "special", "run", "list", "--json"]);
    assert_eq!(tasks["alpha:probe"]["env"]["SCOPE"], "[REDACTED]");
    let workspace = oyzu::discovery::discover_with_options(
        root,
        None,
        &oyzu::config::session::Options {
            root: Some(root.into()),
            profile: Some("special".into()),
            local_overrides: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(workspace.tasks["alpha:probe"].env["SCOPE"], "root-local");
}
#[test]
fn ci_skips_invalid_local_source_before_parsing() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("oyzu.local.toml"), "invalid [").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_oyzu"))
        .arg("--root")
        .arg(tmp.path())
        .arg("-C")
        .arg(tmp.path())
        .args(["config", "show"])
        .env("CI", "true")
        .env_remove("OYZU_PROFILE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
