//! Cross-process refresh locking with test-only transport and integrity storage.
use super::{envelope, setup, Refresh, Runtime};
use anyhow::Result;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

struct FileRuntime {
    root: PathBuf,
    time: i64,
    envelope: String,
}
impl Runtime for FileRuntime {
    fn now(&self) -> Result<i64> {
        Ok(self.time)
    }
    fn state(&self, _: &str) -> Result<Option<Vec<u8>>> {
        match fs::read(self.root.join("integrity")) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    fn save(&self, _: &str, value: &[u8]) -> Result<()> {
        // Deliberately expose an incomplete write unless the agent's real lock
        // protects this state transition from other acquiring processes.
        let mut file = fs::File::create(self.root.join("integrity"))?;
        let middle = value.len() / 2;
        file.write_all(&value[..middle])?;
        file.flush()?;
        std::thread::sleep(Duration::from_millis(10));
        file.write_all(&value[middle..])?;
        file.sync_all()?;
        Ok(())
    }
    fn refresh(&self, _: &str, _: &serde_json::Value) -> Result<Refresh> {
        let mut calls = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("refreshes"))?;
        writeln!(calls, "refresh")?;
        Ok(Refresh::Snapshot(self.envelope.clone()))
    }
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready() {
        assert!(
            Instant::now() < deadline,
            "policy process fixture timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "worker invoked only by the concurrent-process fixture"]
fn worker() {
    let Some(root) = std::env::var_os("OYZU_TEST_POLICY_PROCESS_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let (mut agent, runtime, key) = setup(&root);
    agent.runtime = Arc::new(FileRuntime {
        root: root.clone(),
        time: runtime.now().unwrap(),
        envelope: envelope(&agent, &key, 1),
    });
    fs::write(root.join(format!("ready-{}", std::process::id())), "ready").unwrap();
    wait_until(|| root.join("start").exists());
    let acquired = agent.acquire(false).unwrap();
    assert_eq!(acquired.snapshot.revision(), "r1");
}

struct Workers(Vec<Child>);
impl Drop for Workers {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn separate_processes_serialize_refresh_and_integrity_commit() {
    let root = tempfile::tempdir().unwrap();
    let mut children = Workers(Vec::new());
    for _ in 0..4 {
        children.0.push(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "config::agent::tests::processes::worker",
                ])
                .env("OYZU_TEST_POLICY_PROCESS_ROOT", root.path())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
    }
    wait_until(|| {
        fs::read_dir(root.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("ready-"))
            .count()
            == 4
    });
    fs::write(root.path().join("start"), "start").unwrap();
    for child in &mut children.0 {
        wait_until(|| child.try_wait().unwrap().is_some());
        assert!(child.wait().unwrap().success());
    }
    assert_eq!(
        fs::read_to_string(root.path().join("refreshes")).unwrap(),
        "refresh\n"
    );
    let (mut agent, runtime, key) = setup(root.path());
    agent.runtime = Arc::new(FileRuntime {
        root: root.path().to_owned(),
        time: runtime.now().unwrap(),
        envelope: envelope(&agent, &key, 1),
    });
    let state = agent.state().unwrap().unwrap();
    assert_eq!(
        agent
            .cached(&state, runtime.now().unwrap())
            .unwrap()
            .revision(),
        "r1"
    );
}
