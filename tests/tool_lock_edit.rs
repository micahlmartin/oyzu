mod tool_store_fixture;
use oyzu::tools::{self, ToolLockEdit};
use std::fs;

const FIXTURE: &str = include_str!("fixtures/tool-lock/valid.toml");
const EMPTY: &[u8] = b"format = 2\nenvironment = []\ntool = []\n";

fn updated() -> String {
    FIXTURE.replace("\"core:node\" = \"22\"", "\"core:node\" = \"22.1\"")
}

#[test]
fn creates_previews_commits_and_preserves_unaffected_records() {
    let root = tool_store_fixture::directory().unwrap();
    let path = root.path().join("oyzu.lock");
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(FIXTURE.as_bytes())
        .unwrap();
    assert!(!path.exists());
    assert_eq!(proposal.changes().len(), 2);
    proposal.commit().unwrap();
    assert_eq!(fs::read(&path).unwrap(), FIXTURE.as_bytes());
    let old = FIXTURE.replace(
        "version = \"22.1.0\"",
        "version = '22.1.0' # keep this spelling",
    ) + "\n# tail remains\n";
    fs::write(&path, &old).unwrap();
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(updated().as_bytes())
        .unwrap();
    assert_eq!(proposal.changes().len(), 1);
    let diff = serde_json::to_value(proposal.changes()).unwrap();
    assert_eq!(
        diff[0],
        serde_json::json!({"record":"environment", "scope":".", "profile":"default", "change":"changed"})
    );
    let preview = String::from_utf8(proposal.preview().to_vec()).unwrap();
    assert!(preview.contains("version = '22.1.0' # keep this spelling"));
    assert!(preview.contains("# tail remains"));
    assert_eq!(fs::read_to_string(&path).unwrap(), old);
    proposal.commit().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), preview);
    tools::inspect_lock(&path).unwrap();
}

#[cfg(windows)]
#[test]
fn readonly_source_fails_without_leaking_a_readonly_temporary() {
    let root = tool_store_fixture::directory().unwrap();
    let path = root.path().join("oyzu.lock");
    fs::write(&path, FIXTURE).unwrap();
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(updated().as_bytes())
        .unwrap();
    let original_permissions = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = proposal.commit();
    fs::set_permissions(&path, original_permissions).unwrap();
    assert!(result.unwrap_err().to_string().contains("read-only"));
    assert_eq!(fs::read_to_string(&path).unwrap(), FIXTURE);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn semantic_noop_is_byte_identical_and_explicit_removal_can_empty_the_graph() {
    let root = tool_store_fixture::directory().unwrap();
    let path = root.path().join("oyzu.lock");
    let original = FIXTURE.replace("format = 2", "format=2 # preserve\n");
    fs::write(&path, &original).unwrap();
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(FIXTURE.as_bytes())
        .unwrap();
    assert!(proposal.changes().is_empty());
    assert_eq!(proposal.preview(), original.as_bytes());
    proposal.commit().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(EMPTY)
        .unwrap();
    assert_eq!(proposal.changes().len(), 2);
    proposal.commit().unwrap();
    assert_eq!(tools::inspect_lock(&path).unwrap().tools, 0);
    ToolLockEdit::capture(&path)
        .unwrap()
        .propose(FIXTURE.as_bytes())
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(tools::inspect_lock(&path).unwrap().tools, 1);
}

#[test]
fn stale_source_missing_file_race_and_active_writer_leave_original_untouched() {
    let root = tool_store_fixture::directory().unwrap();
    let path = root.path().join("oyzu.lock");
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(FIXTURE.as_bytes())
        .unwrap();
    fs::write(&path, EMPTY).unwrap();
    assert!(proposal
        .commit()
        .unwrap_err()
        .to_string()
        .contains("TOOL_LOCK_EDIT_CONFLICT"));
    assert_eq!(fs::read(&path).unwrap(), EMPTY);
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(FIXTURE.as_bytes())
        .unwrap();
    fs::write(&path, b"external edit").unwrap();
    assert!(proposal
        .commit()
        .unwrap_err()
        .to_string()
        .contains("TOOL_LOCK_EDIT_CONFLICT"));
    assert_eq!(fs::read(&path).unwrap(), b"external edit");
    fs::write(&path, FIXTURE).unwrap();
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(updated().as_bytes())
        .unwrap();
    let mutex = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.path().join(".oyzu-tool-lock-edit.lock"))
        .unwrap();
    mutex.lock().unwrap();
    assert!(proposal
        .commit()
        .unwrap_err()
        .to_string()
        .contains("TOOL_LOCK_EDIT_CONFLICT"));
    mutex.unlock().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), FIXTURE);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn invalid_whole_graph_and_old_format_are_not_published_or_migrated() {
    let root = tool_store_fixture::directory().unwrap();
    let path = root.path().join("oyzu.lock");
    fs::write(&path, FIXTURE).unwrap();
    let invalid = FIXTURE.replace("version = \"22.1.0\"", "version = \"24.0.0\"");
    assert!(ToolLockEdit::capture(&path)
        .unwrap()
        .propose(invalid.as_bytes())
        .is_err());
    assert!(ToolLockEdit::capture(&path)
        .unwrap()
        .propose(&vec![b' '; 8 * 1024 * 1024 + 1])
        .is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), FIXTURE);
    fs::write(&path, "format = 1\nenvironment=[]\ntool=[]").unwrap();
    assert!(ToolLockEdit::capture(&path).is_err());
    assert!(fs::read_to_string(&path).unwrap().starts_with("format = 1"));
}

#[test]
fn inline_arrays_retain_unchanged_records_when_adding_an_environment() {
    let root = tool_store_fixture::directory().unwrap();
    let path = root.path().join("oyzu.lock");
    let empty_scope = format!(
        "{{scope='.',profile='default',request_digest='sha256:{}',roots=[],requests={{}}}}",
        "f".repeat(64)
    );
    let original = format!("format=2\nenvironment=[\n  {empty_scope}, # preserve me\n]\ntool=[]\n");
    fs::write(&path, &original).unwrap();
    let addition = empty_scope.replace("scope='.'", "scope='app'");
    let candidate = format!("format=2\nenvironment=[{empty_scope},{addition}]\ntool=[]\n");
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(candidate.as_bytes())
        .unwrap();
    assert!(std::str::from_utf8(proposal.preview())
        .unwrap()
        .contains("# preserve me"));
    proposal.commit().unwrap();
    assert_eq!(tools::inspect_lock(&path).unwrap().environments, 2);
}

#[cfg(unix)]
#[test]
fn rejects_redirected_files_and_fifo_without_opening_their_contents() {
    use std::os::unix::fs::symlink;
    let root = tool_store_fixture::directory().unwrap();
    let path = root.path().join("oyzu.lock");
    let external = root.path().join("external");
    fs::write(&external, FIXTURE).unwrap();
    symlink(&external, &path).unwrap();
    assert!(ToolLockEdit::capture(&path).is_err());
    fs::remove_file(&path).unwrap();
    fs::write(&path, FIXTURE).unwrap();
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(updated().as_bytes())
        .unwrap();
    fs::remove_file(&path).unwrap();
    symlink(&external, &path).unwrap();
    assert!(proposal.commit().is_err());
    assert_eq!(fs::read_to_string(&external).unwrap(), FIXTURE);
    fs::remove_file(&path).unwrap();
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert!(ToolLockEdit::capture(&path).is_err());
}

#[cfg(windows)]
#[test]
fn windows_sharing_denial_is_bounded_preserves_source_and_allows_later_retry() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = tool_store_fixture::directory().unwrap();
    let path = root.path().join("oyzu.lock");
    fs::write(&path, FIXTURE).unwrap();
    let proposal = ToolLockEdit::capture(&path)
        .unwrap()
        .propose(updated().as_bytes())
        .unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ)
        .open(&path)
        .unwrap();
    let started = std::time::Instant::now();
    let error = proposal.commit().unwrap_err();
    assert!(error.to_string().contains("TOOL_LOCK_EDIT_CONFLICT"));
    assert!(
        started.elapsed() >= std::time::Duration::from_secs(2),
        "{error:#}"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    assert_eq!(fs::read_to_string(&path).unwrap(), FIXTURE);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    drop(held);
    ToolLockEdit::capture(&path)
        .unwrap()
        .propose(updated().as_bytes())
        .unwrap()
        .commit()
        .unwrap();
    tools::inspect_lock(&path).unwrap();
}
