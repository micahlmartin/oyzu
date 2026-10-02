use super::*;
use std::fs;

fn setup() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    (temp, root)
}

#[test]
fn content_identity_includes_directories_and_ignores_creation_order() {
    let (_first, root) = setup();
    fs::create_dir(root.join("bin")).unwrap();
    fs::write(root.join("bin/tool"), b"not executed").unwrap();
    fs::create_dir(root.join("empty")).unwrap();
    let first = inspect(&root).unwrap();
    let (_second, other) = setup();
    fs::create_dir(other.join("empty")).unwrap();
    fs::create_dir(other.join("bin")).unwrap();
    fs::write(other.join("bin/tool"), b"not executed").unwrap();
    assert_eq!(first.digest, inspect(&other).unwrap().digest);
    assert_eq!(first.entries.len(), 3);
    assert_eq!(first.bytes, 12);
    fs::write(other.join("bin/tool"), b"changed bytes").unwrap();
    assert_ne!(first.digest, inspect(&other).unwrap().digest);
}

#[test]
fn internal_hardlinks_are_explicit_but_external_links_fail() {
    let (_temp, root) = setup();
    fs::create_dir(root.join("payload")).unwrap();
    fs::write(root.join("outside"), b"shared").unwrap();
    fs::hard_link(root.join("outside"), root.join("payload/one")).unwrap();
    assert!(inspect(&root.join("payload"))
        .unwrap_err()
        .to_string()
        .contains("hardlink"));
    fs::hard_link(root.join("payload/one"), root.join("payload/two")).unwrap();
    fs::remove_file(root.join("outside")).unwrap();
    assert_eq!(inspect(&root.join("payload")).unwrap().entries.len(), 2);
}

#[test]
fn portable_names_reject_aliases_and_devices() {
    for name in [
        "..",
        "a:b",
        "NUL",
        "Con.txt",
        "lpt1.log",
        "COM¹",
        "trailing.",
        "trailing ",
        "a\\b",
        "a/b",
    ] {
        assert!(access::component(name).is_err(), "accepted {name}");
    }
    for name in ["工具", "node.exe", "console", "lpt10", "lib.node"] {
        access::component(name).unwrap();
    }
}

fn record(path: &str, target: Option<&str>, kind: &'static str) -> TreeEntry {
    TreeEntry {
        path: path.into(),
        kind,
        executable: 0,
        size: None,
        digest: None,
        target: target.map(str::to_owned),
    }
}

#[test]
fn link_resolution_checks_parents_after_expansion_and_detects_cycles() {
    let escape = vec![
        record("root", Some("."), "symlink"),
        record("escape", Some("root/../outside"), "symlink"),
    ];
    assert!(validate_links(&escape)
        .unwrap_err()
        .to_string()
        .contains("escapes"));
    let cycle = vec![
        record("one", Some("two"), "symlink"),
        record("two", Some("one"), "symlink"),
    ];
    assert!(validate_links(&cycle)
        .unwrap_err()
        .to_string()
        .contains("cycle"));
    assert!(validate_links(&[record("one", Some("missing"), "symlink")]).is_err());
    let valid = vec![
        record("dir", None, "directory"),
        record("file", None, "file"),
        record("dir/link", Some("../file"), "symlink"),
    ];
    validate_links(&valid).unwrap();
    for target in ["/etc/passwd", "C:/Windows", "a//b", "\\server\\share", ""] {
        assert!(validate_target(target).is_err());
    }
}

#[cfg(unix)]
#[test]
fn native_symlinks_are_recorded_without_following_and_escaping_links_fail() {
    use std::os::unix::fs::symlink;
    let (_temp, root) = setup();
    fs::create_dir(root.join("bin")).unwrap();
    fs::write(root.join("bin/tool"), b"tool").unwrap();
    symlink("bin/tool", root.join("tool")).unwrap();
    assert_eq!(
        inspect(&root).unwrap().entries.last().unwrap().kind,
        "symlink"
    );
    symlink("../outside", root.join("escape")).unwrap();
    assert!(inspect(&root).is_err());
}

#[cfg(unix)]
#[test]
fn replaced_root_never_redirects_open_handle_reads() {
    use std::os::unix::fs::symlink;
    let (_temp, root) = setup();
    fs::create_dir(root.join("payload")).unwrap();
    fs::create_dir(root.join("outside")).unwrap();
    fs::write(root.join("payload/file"), b"original").unwrap();
    fs::write(root.join("outside/file"), b"outside").unwrap();
    let directory = Directory::open(&root.join("payload")).unwrap();
    fs::rename(root.join("payload"), root.join("held")).unwrap();
    symlink("outside", root.join("payload")).unwrap();
    let mut bytes = String::new();
    directory
        .file("file")
        .unwrap()
        .read_to_string(&mut bytes)
        .unwrap();
    assert_eq!(bytes, "original");
    assert!(Directory::open(&root.join("payload")).is_err());
}

#[cfg(windows)]
#[test]
fn held_windows_ancestors_cannot_be_replaced() {
    let (_temp, root) = setup();
    fs::create_dir(root.join("payload")).unwrap();
    let held = Directory::open(&root.join("payload")).unwrap();
    assert!(fs::rename(root.join("payload"), root.join("moved")).is_err());
    drop(held);
    fs::rename(root.join("payload"), root.join("moved")).unwrap();
}

#[cfg(unix)]
#[test]
fn executable_permissions_affect_identity_and_special_files_fail() {
    use std::os::unix::fs::PermissionsExt;
    let (_temp, root) = setup();
    fs::write(root.join("tool"), b"bytes").unwrap();
    let original = inspect(&root).unwrap().digest;
    fs::set_permissions(root.join("tool"), fs::Permissions::from_mode(0o755)).unwrap();
    assert_ne!(original, inspect(&root).unwrap().digest);
    let socket = std::os::unix::net::UnixListener::bind(root.join("socket")).unwrap();
    assert!(inspect(&root).is_err());
    drop(socket);
}

#[cfg(unix)]
#[test]
fn case_collisions_and_nonportable_names_fail_on_unix_too() {
    let (_temp, root) = setup();
    fs::write(root.join("Tool"), b"one").unwrap();
    fs::write(root.join("tool"), b"two").unwrap();
    // Some Darwin volumes are case-insensitive: do not confuse overwriting the
    // same file with a two-entry collision test.
    if fs::read_dir(&root).unwrap().count() == 2 {
        assert!(inspect(&root)
            .unwrap_err()
            .to_string()
            .contains("case-colliding"));
    }
    fs::write(root.join("NUL.txt"), b"forbidden on Windows").unwrap();
    assert!(inspect(&root)
        .unwrap_err()
        .to_string()
        .contains("device alias"));
}
