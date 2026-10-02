mod tool_store_fixture;
use oyzu::tools;
use std::{fs, path::Path};

const ROOT_DIGEST: &str = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const NESTED_DIGEST: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const PLATFORM: &str = "linux/amd64/gnu";

fn fixture(root: &Path) -> Vec<u8> {
    let mut lock: toml::Value =
        toml::from_str(include_str!("fixtures/tool-lock/valid.toml")).unwrap();
    let mut nested = lock["environment"][0].clone();
    nested["scope"] = "app".into();
    nested["request_digest"] = NESTED_DIGEST.into();
    let mut profile = lock["environment"][0].clone();
    profile["profile"] = "ci".into();
    lock["environment"]
        .as_array_mut()
        .unwrap()
        .extend([nested, profile]);
    fs::create_dir_all(root.join("app/deep")).unwrap();
    fs::create_dir(root.join("application")).unwrap();
    let bytes = toml::to_string(&lock).unwrap().into_bytes();
    fs::write(root.join("oyzu.lock"), &bytes).unwrap();
    fs::write(root.join("mise.toml"), "invalid = [").unwrap();
    fs::write(root.join("oyzu.toml"), "invalid = [").unwrap();
    bytes
}

#[test]
fn selects_physical_nearest_scope_and_profile_without_discovery_or_mutation() {
    let temp = tool_store_fixture::directory().unwrap();
    let root = temp.path();
    let bytes = fixture(root);
    for (directory, profile, digest, scope) in [
        (".", "default", ROOT_DIGEST, "."),
        ("app", "default", NESTED_DIGEST, "app"),
        ("app/deep", "default", NESTED_DIGEST, "app"),
        ("application", "default", ROOT_DIGEST, "."),
        ("app/deep", "ci", ROOT_DIGEST, "."),
    ] {
        let selected = tools::select_locked_environment(
            root,
            &root.join(directory),
            profile,
            digest,
            PLATFORM,
        )
        .unwrap();
        assert_eq!(selected.scope, scope);
        assert_eq!(selected.profile, profile);
        assert_eq!(selected.platform, PLATFORM);
    }
    assert_eq!(fs::read(root.join("oyzu.lock")).unwrap(), bytes);
    assert_eq!(
        fs::read_to_string(root.join("mise.toml")).unwrap(),
        "invalid = ["
    );
}

#[test]
fn stale_nearest_scope_missing_profile_and_missing_platform_never_fall_back() {
    let temp = tool_store_fixture::directory().unwrap();
    let root = temp.path();
    fixture(root);
    let error = tools::select_locked_environment(
        root,
        &root.join("app/deep"),
        "default",
        ROOT_DIGEST,
        PLATFORM,
    )
    .unwrap_err();
    assert!(error.to_string().contains("TOOL_LOCK_STALE"));
    assert!(
        tools::select_locked_environment(root, root, "missing", ROOT_DIGEST, PLATFORM).is_err()
    );
    assert!(tools::select_locked_environment(
        root,
        &root.join("app"),
        "default",
        NESTED_DIGEST,
        "windows/amd64/msvc"
    )
    .is_err());
    let outside = tool_store_fixture::directory().unwrap();
    assert!(tools::select_locked_environment(
        root,
        outside.path(),
        "default",
        ROOT_DIGEST,
        PLATFORM
    )
    .is_err());
    assert!(tools::select_locked_environment(
        root,
        &root.join("oyzu.lock"),
        "default",
        ROOT_DIGEST,
        PLATFORM
    )
    .is_err());
}

#[test]
#[cfg(unix)]
fn directory_symlinks_use_physical_scope_and_cannot_escape_workspace() {
    let temp = tool_store_fixture::directory().unwrap();
    let root = temp.path();
    fixture(root);
    std::os::unix::fs::symlink(root.join("app"), root.join("alias")).unwrap();
    let selected = tools::select_locked_environment(
        root,
        &root.join("alias/deep"),
        "default",
        NESTED_DIGEST,
        PLATFORM,
    )
    .unwrap();
    assert_eq!(selected.scope, "app");
    let lock_path = root.join("oyzu.lock");
    let mut lock: toml::Value = toml::from_str(&fs::read_to_string(&lock_path).unwrap()).unwrap();
    lock["environment"][1]["scope"] = "alias".into();
    fs::write(&lock_path, toml::to_string(&lock).unwrap()).unwrap();
    assert_eq!(
        tools::select_locked_environment(
            root,
            &root.join("app/deep"),
            "default",
            NESTED_DIGEST,
            PLATFORM
        )
        .unwrap()
        .scope,
        "alias"
    );
    let mut duplicate = lock["environment"][1].clone();
    duplicate["scope"] = "app".into();
    lock["environment"].as_array_mut().unwrap().push(duplicate);
    fs::write(&lock_path, toml::to_string(&lock).unwrap()).unwrap();
    assert!(tools::select_locked_environment(
        root,
        &root.join("app/deep"),
        "default",
        NESTED_DIGEST,
        PLATFORM
    )
    .unwrap_err()
    .to_string()
    .contains("TOOL_LOCK_AMBIGUOUS"));
    let outside = tool_store_fixture::directory().unwrap();
    std::os::unix::fs::symlink(outside.path(), root.join("escape")).unwrap();
    assert!(tools::select_locked_environment(
        root,
        &root.join("escape"),
        "default",
        ROOT_DIGEST,
        PLATFORM
    )
    .is_err());
}

#[test]
fn missing_nested_platform_does_not_select_available_parent_platform() {
    let temp = tool_store_fixture::directory().unwrap();
    let root = temp.path();
    fixture(root);
    let path = root.join("oyzu.lock");
    let mut lock: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let mut nested_tool = lock["tool"][0].clone();
    nested_tool["version"] = "24.0.0".into();
    let key = oyzu::records::digest(
        "oyzu.tool-record.v2",
        &serde_json::json!({
            "id":nested_tool["id"].as_str().unwrap(), "version":"24.0.0",
            "backend_digest":nested_tool["backend_digest"].as_str().unwrap(), "options":{}
        }),
    )
    .unwrap();
    nested_tool["key"] = key.clone().into();
    lock["environment"][1]["roots"] = vec![key].into();
    let mut windows = lock["tool"][0]["distribution"][0].clone();
    windows["platform"] = "windows/amd64/msvc".into();
    lock["tool"][0]["distribution"]
        .as_array_mut()
        .unwrap()
        .push(windows);
    lock["tool"].as_array_mut().unwrap().push(nested_tool);
    fs::write(&path, toml::to_string(&lock).unwrap()).unwrap();
    assert!(tools::select_locked_environment(
        root,
        root,
        "default",
        ROOT_DIGEST,
        "windows/amd64/msvc"
    )
    .is_ok());
    let error = tools::select_locked_environment(
        root,
        &root.join("app"),
        "default",
        NESTED_DIGEST,
        "windows/amd64/msvc",
    )
    .unwrap_err();
    assert!(error.to_string().contains("TOOL_PLATFORM_UNAVAILABLE"));
}
