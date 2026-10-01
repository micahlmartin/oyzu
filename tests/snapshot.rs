use oyzu::snapshot::capture;
use std::fs;

#[test]
fn snapshot_is_content_identified_and_independent_of_checkout_location() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    for root in [&a, &b] {
        fs::create_dir(root).unwrap();
        fs::write(root.join("source.txt"), "original").unwrap();
        fs::create_dir(root.join("dist")).unwrap();
        fs::write(root.join("dist/ignored"), "output").unwrap();
    }
    let first = capture(&a, &temp.path().join("capture-a")).unwrap();
    let second = capture(&b, &temp.path().join("capture-b")).unwrap();
    assert_eq!(first.digest, second.digest);
    assert_eq!(first.entries.len(), 1);
    fs::write(a.join("source.txt"), "changed").unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join("capture-a/source.txt")).unwrap(),
        "original"
    );
    let changed = capture(&a, &temp.path().join("changed")).unwrap();
    assert_ne!(first.digest, changed.digest);
}

#[test]
fn source_nested_destination_and_existing_destination_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    assert!(capture(temp.path(), &temp.path().join("nested")).is_err());
    let other = tempfile::tempdir().unwrap();
    assert!(capture(temp.path(), other.path()).is_err());
}

#[cfg(unix)]
#[test]
fn escaping_symlink_cannot_capture_host_files() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", source.join("escape")).unwrap();
    assert!(capture(&source, &temp.path().join("out"))
        .unwrap_err()
        .to_string()
        .contains("symlink"));
}
