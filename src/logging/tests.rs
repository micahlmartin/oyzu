use super::*;
use std::{
    fs,
    io::{self, Write},
};

#[derive(Clone)]
struct Buffer(Arc<Mutex<Vec<u8>>>);
impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn concurrent_scopes_produce_ordered_independent_json_records() {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let log = Log::new(Some((Format::Json, Box::new(Buffer(bytes.clone())))));
    std::thread::scope(|scope| {
        for name in ["api:test", "web:build"] {
            let child = log.scope(name);
            scope.spawn(move || {
                for _ in 0..20 {
                    child.progress("working");
                }
            });
        }
    });
    let text = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    let values: Vec<serde_json::Value> = text
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(values.len(), 40);
    for (i, event) in values.iter().enumerate() {
        assert_eq!(event["sequence"], i + 1);
    }
    assert_eq!(
        values.iter().filter(|e| e["scope"] == "api:test").count(),
        20
    );
    assert_eq!(
        values.iter().filter(|e| e["scope"] == "web:build").count(),
        20
    );
}

#[test]
fn output_is_live_keeps_split_utf8_and_flushes_final_partial_line() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("raw");
    let mut writer = File::create(&path).unwrap();
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let log = Log::new(Some((Format::Json, Box::new(Buffer(bytes.clone())))));
    let mut follower = Follow::open(&path, "stderr", &log.scope("api:test")).unwrap();
    writer.write_all(b"first\n\xc3").unwrap();
    follower.drain(false).unwrap();
    assert!(String::from_utf8(bytes.lock().unwrap().clone())
        .unwrap()
        .contains("first"));
    writer.write_all(b"\xa9\nlast").unwrap();
    follower.drain(true).unwrap();
    let text = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    let events: Vec<serde_json::Value> = text
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[1]["event"]["text"], "é");
    assert_eq!(events[2]["event"]["text"], "last");
    assert_eq!(fs::read(path).unwrap(), b"first\n\xc3\xa9\nlast");
}

#[test]
fn plan_events_exclude_execution_environments_and_keep_dependencies() {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let log = Log::new(Some((Format::Json, Box::new(Buffer(bytes.clone())))));
    log.plan(&serde_json::json!({"targets":[{"id":"api","builder":"rust/app"}],"actions":[{"id":"api:build","target":"api","dependsOn":["api:prepare"],"env":{"SECRET":"never-log-this"}}]}));
    let text = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    assert!(!text.contains("never-log-this"));
    assert!(text.contains("api:prepare"));
}
