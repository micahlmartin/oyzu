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

#[test]
fn ordinary_edit_repairs_invalid_value_without_executing_configuration() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("oyzu.toml"),
        "# keep this\n[build]\njobs='invalid'\n",
    )
    .unwrap();
    assert!(!run(temp.path(), &["config", "validate"]).status.success());
    let explanation = value(temp.path(), &["config", "explain"]);
    assert_eq!(explanation["status"], "unresolved");
    assert!(explanation["values"].is_null());
    value(
        temp.path(),
        &["config", "set", "build.jobs", "2", "--project"],
    );
    assert_eq!(
        value(temp.path(), &["config", "get", "build.jobs"]),
        json!(2)
    );
    assert!(fs::read_to_string(temp.path().join("oyzu.toml"))
        .unwrap()
        .contains("# keep this"));
}

#[test]
fn implicit_target_profiles_share_selection_and_report_real_sources() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["alpha", "beta"] {
        fs::create_dir(temp.path().join(name)).unwrap();
        fs::write(temp.path().join(name).join("package.json"), "{}").unwrap();
    }
    fs::write(
        temp.path().join("alpha/oyzu.toml"),
        "[profiles.integration.build]\njobs=3\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("oyzu.toml"),
        "[profile]\ndefault='integration'\n",
    )
    .unwrap();
    let profiles = value(temp.path(), &["config", "profiles"]);
    assert_eq!(profiles["selected"], "integration");
    let origins = profiles["profiles"]["integration"].as_array().unwrap();
    assert_eq!(origins.len(), 1);
    assert!(origins[0].as_str().unwrap().ends_with("oyzu.toml"));
    assert!(!origins[0].as_str().unwrap().contains("catalogue"));
    value(temp.path(), &["config", "validate", "--all-profiles"]);
    fs::write(
        temp.path().join("beta/oyzu.toml"),
        "[profiles.integration.compatibility]\nrequires=['future/v99']\n",
    )
    .unwrap();
    assert!(!run(temp.path(), &["config", "validate"]).status.success());
}

#[test]
fn validation_rejects_unknown_builders_and_incompatible_language_axes() {
    let temp = tempfile::tempdir().unwrap();
    for inventory in [
        "app:\n  uses: future/unknown\n",
        "app:\n  uses: node/package\n  matrix:\n    python: ['3.12']\n",
    ] {
        fs::write(temp.path().join("build.yaml"), inventory).unwrap();
        let result = run(temp.path(), &["config", "validate"]);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("CONFIG_INVALID_VALUE"));
    }
}

#[test]
fn configured_task_color_is_presentation_only_and_json_stays_plain() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("package.json"),
        r#"{"name":"color-fixture","version":"1.0.0","scripts":{"test":"node --version"}}"#,
    )
    .unwrap();
    let mut digest = None;
    for (color, colored) in [("always", true), ("never", false), ("auto", false)] {
        fs::write(root.join("oyzu.toml"), format!("[ui]\ncolor='{color}'\n")).unwrap();
        let output = run(root, &["run", "list"]);
        assert!(output.status.success());
        assert_eq!(output.stdout.contains(&0x1b), colored);
        let json_output = run(root, &["run", "list", "--json"]);
        assert!(json_output.status.success());
        assert!(!json_output.stdout.contains(&0x1b));
        assert!(serde_json::from_slice::<Value>(&json_output.stdout).is_ok());
        let current = value(root, &["config", "explain"])["digest"].clone();
        assert!(current.is_string());
        if let Some(prior) = &digest {
            assert_eq!(prior, &current);
        }
        digest = Some(current);
    }
}
