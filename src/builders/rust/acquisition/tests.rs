use super::*;
use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use std::process::Command;

fn fixture() -> (Vec<u8>, String, Vec<u8>) {
    let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    for (name, contents) in [
        (
            "Cargo.toml",
            "[package]\nname='fixture-dep'\nversion='1.0.0'\nedition='2021'\n",
        ),
        ("src/lib.rs", "pub fn answer() -> u32 { 42 }\n"),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(
                &mut header,
                format!("fixture-dep-1.0.0/{name}"),
                contents.as_bytes(),
            )
            .unwrap();
    }
    let archive = archive.into_inner().unwrap().finish().unwrap();
    let checksum = format!("{:x}", Sha256::digest(&archive));
    let index = serde_json::to_vec(&json!({"name":"fixture-dep","vers":"1.0.0","deps":[],
        "cksum":checksum,"features":{},"yanked":false}))
    .unwrap();
    (archive, checksum, index)
}

fn write_lock(path: &Path, checksum: &str) {
    fs::write(path, format!("version=4\n[[package]]\nname='consumer'\nversion='0.1.0'\ndependencies=['fixture-dep']\n[[package]]\nname='fixture-dep'\nversion='1.0.0'\nsource='{CRATES_IO}'\nchecksum='{checksum}'\n")).unwrap();
}

#[test]
fn workspace_packaging_preserves_native_unpublished_dependency_resolution() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("workspace");
    for member in ["core", "app"] {
        fs::create_dir_all(project.join(member).join("src")).unwrap();
        fs::write(project.join(member).join("Cargo.toml"), format!(
            "[package]\nname='example-{member}'\nversion='0.1.0-dev.g123456789abc'\nedition='2021'\n{}",
            if member == "app" { "[dependencies]\nexample-core={path='../core',version='=0.1.0-dev.g123456789abc'}\n" } else { "" }
        )).unwrap();
    }
    fs::write(
        project.join("Cargo.toml"),
        "[workspace]\nmembers=['core','app']\nresolver='2'\n",
    )
    .unwrap();
    fs::write(
        project.join("core/src/lib.rs"),
        "pub fn answer() -> u32 { 42 }\n",
    )
    .unwrap();
    fs::write(
        project.join("app/src/main.rs"),
        "fn main() { assert_eq!(example_core::answer(),42); }\n",
    )
    .unwrap();
    let home = root.path().join("cargo-home");
    fs::create_dir(&home).unwrap();
    let cargo = |args: &[&str]| {
        let result = Command::new(env!("CARGO"))
            .args(args)
            .current_dir(&project)
            .env("CARGO_HOME", &home)
            .env("CARGO_TARGET_DIR", root.path().join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    cargo(&["generate-lockfile", "--offline"]);
    let lock = project.join("Cargo.lock");
    let original_lock = fs::read(&lock).unwrap();
    let prepared = root.path().join("prepared");
    fs::create_dir(&prepared).unwrap();
    let inventory = capture_with(&lock, &prepared, |_| {
        panic!("local workspace must not fetch")
    })
    .unwrap();
    assert!(inventory.is_empty());
    fs::copy(prepared.join("cargo-config.toml"), home.join("config.toml")).unwrap();
    cargo(&[
        "package",
        "--workspace",
        "--locked",
        "--offline",
        "--allow-dirty",
    ]);
    for member in ["core", "app"] {
        assert!(root
            .path()
            .join(format!(
                "target/package/example-{member}-0.1.0-dev.g123456789abc.crate"
            ))
            .is_file());
    }
    assert_eq!(fs::read(lock).unwrap(), original_lock);
}

#[test]
fn local_registry_is_consumed_and_verified_by_native_cargo() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("consumer");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(project.join("Cargo.toml"), "[package]\nname='consumer'\nversion='0.1.0'\nedition='2021'\n[dependencies]\nfixture-dep='=1.0.0'\n").unwrap();
    fs::write(project.join("src/lib.rs"), "pub fn value() -> u32 { fixture_dep::answer() }\n#[test] fn works() { assert_eq!(value(),42); }\n").unwrap();
    let (archive, checksum, index) = fixture();
    write_lock(&project.join("Cargo.lock"), &checksum);
    let original_lock = fs::read(project.join("Cargo.lock")).unwrap();
    let prepared = root.path().join("prepared");
    fs::create_dir(&prepared).unwrap();
    let inventory = capture_with(&project.join("Cargo.lock"), &prepared, |url| match url {
        "https://index.crates.io/fi/xt/fixture-dep" => Ok(index.clone()),
        "https://static.crates.io/crates/fixture-dep/fixture-dep-1.0.0.crate" => {
            Ok(archive.clone())
        }
        _ => panic!("unexpected acquisition URL: {url}"),
    })
    .unwrap();
    assert_eq!(inventory.len(), 1);
    assert_eq!(inventory[0]["digest"], format!("sha256:{checksum}"));
    let registry = prepared.join("registry");
    let config = prepared.join("cargo-config.toml");
    // The production config uses the sandbox mount; this native host probe
    // points that same Cargo source at the temporary captured registry.
    fs::write(&config, format!("[source.crates-io]\nreplace-with='oyzu-captured'\n[source.oyzu-captured]\nlocal-registry={}\n", serde_json::to_string(&registry.to_string_lossy()).unwrap())).unwrap();
    let cargo = |args: &[&str], home: &str| {
        let home = root.path().join(home);
        fs::create_dir_all(&home).unwrap();
        fs::copy(&config, home.join("config.toml")).unwrap();
        Command::new(env!("CARGO"))
            .args(args)
            .current_dir(&project)
            .env("CARGO_HOME", home)
            .env("CARGO_TARGET_DIR", root.path().join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .unwrap()
    };
    for args in [
        vec!["test", "--locked", "--offline"],
        vec!["clippy", "--locked", "--offline", "--", "-D", "warnings"],
        vec!["package", "--locked", "--offline", "--allow-dirty"],
    ] {
        let result = cargo(&args, "cargo-home");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert_eq!(fs::read(project.join("Cargo.lock")).unwrap(), original_lock);
    // A fresh native cache must reject altered archives rather than trust a
    // previously unpacked copy. This probes Cargo's independent checksum gate.
    fs::write(registry.join("fixture-dep-1.0.0.crate"), b"altered archive").unwrap();
    let result = cargo(
        &["metadata", "--format-version", "1", "--locked", "--offline"],
        "fresh-cargo-home",
    );
    assert!(!result.status.success());
}

#[test]
fn acquisition_rejects_unapproved_sources_and_checksum_mismatches() {
    let root = tempfile::tempdir().unwrap();
    let lock = root.path().join("Cargo.lock");
    let (archive, checksum, index) = fixture();
    write_lock(&lock, &checksum);
    fs::write(
        &lock,
        fs::read_to_string(&lock)
            .unwrap()
            .replace(CRATES_IO, "git+https://example.invalid/repository"),
    )
    .unwrap();
    assert!(capture_with(&lock, root.path(), |_| panic!("unapproved source fetched")).is_err());
    write_lock(&lock, &checksum);
    let wrong_index = String::from_utf8(index.clone())
        .unwrap()
        .replace(&checksum, &"0".repeat(64));
    assert!(
        capture_with(&lock, root.path(), |_| Ok(wrong_index.as_bytes().to_vec()))
            .unwrap_err()
            .to_string()
            .contains("index checksum")
    );
    assert!(capture_with(&lock, root.path(), |url| Ok(
        if url.contains("index.crates.io") {
            index.clone()
        } else {
            b"wrong archive".to_vec()
        }
    ))
    .unwrap_err()
    .to_string()
    .contains("archive checksum"));
    assert!(!archive.is_empty());
}
