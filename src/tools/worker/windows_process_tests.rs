use super::*;
use crate::tools::{native_tool_worker_channel, split_tool_worker_channel, NativeToolWorkerIo};
use std::{io::Write, os::windows::io::IntoRawHandle, thread, time::Duration};
use windows_sys::Win32::Foundation::GetLastError;

const FIXTURE: &str = "tools::worker::windows_process::tests::native_process_fixture";
const ARGUMENT: &str = "--skip=quoted-value=a b\"c\\";

fn digest() -> String {
    let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn event() -> OwnedHandle {
    let handle = unsafe { CreateEventW(null(), 1, 0, null()) };
    assert!(!handle.is_null());
    unsafe { OwnedHandle::from_raw_handle(handle) }
}
fn fixture_arguments(role: &str) -> Vec<OsString> {
    [
        "--exact",
        FIXTURE,
        "--ignored",
        "--nocapture",
        &format!("--skip=role={role}"),
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}
fn parameter(name: &str, handle: HANDLE) -> OsString {
    format!("--skip={name}={}", handle as usize).into()
}
fn handle_argument(name: &str) -> HANDLE {
    let prefix = format!("--skip={name}=");
    std::env::args()
        .find_map(|arg| {
            arg.strip_prefix(&prefix)
                .map(|value| value.parse::<usize>().unwrap())
        })
        .unwrap() as HANDLE
}

#[test]
fn real_spawn_restricts_handles_environment_and_cleans_descendant_held_control_pipes() {
    let temp = tempfile::tempdir().unwrap();
    let unrelated = event();
    assert_ne!(
        unsafe {
            SetHandleInformation(
                unrelated.as_raw_handle(),
                HANDLE_FLAG_INHERIT,
                HANDLE_FLAG_INHERIT,
            )
        },
        0
    );
    let allowed = event();
    let observer = allowed.try_clone().unwrap();
    let (parent, child) = native_tool_worker_channel().unwrap();
    let (source_reader, source_writer) = child.split();
    // Explicitly duplicate the endpoint halves; source halves stay non-inheritable
    // and are dropped before launch. Each duplicate has exactly one Rust owner.
    let reader = source_reader.as_handle().try_clone_to_owned().unwrap();
    let writer = source_writer.as_handle().try_clone_to_owned().unwrap();
    drop(source_reader);
    drop(source_writer);
    let mut arguments = fixture_arguments("parent");
    arguments.extend([
        parameter("read", reader.as_raw_handle()),
        parameter("write", writer.as_raw_handle()),
        parameter("allowed", allowed.as_raw_handle()),
        parameter("unrelated", unrelated.as_raw_handle()),
        OsString::from(ARGUMENT),
    ]);
    let mut process = WindowsToolWorkerProcess::spawn_current(
        &digest(),
        &arguments,
        &BTreeMap::from([("OYZU_PROBE".into(), "explicit".into())]),
        temp.path(),
        vec![reader, writer, allowed],
    )
    .unwrap();
    let mut io = NativeToolWorkerIo::new(parent).unwrap();
    io.start_receive().unwrap();
    io.start_send(br#"{"request":true}"#.to_vec()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !io.try_send().unwrap() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    let response = loop {
        if let Some(value) = io.try_receive().unwrap() {
            break value;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    };
    let descendant_pid = response["descendant"].as_u64().unwrap() as u32;
    let descendant = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, descendant_pid) };
    assert!(!descendant.is_null());
    let descendant = unsafe { OwnedHandle::from_raw_handle(descendant) };
    assert_eq!(
        unsafe { WaitForSingleObject(observer.as_raw_handle(), 0) },
        WAIT_OBJECT_0
    );
    assert_eq!(
        unsafe { WaitForSingleObject(unrelated.as_raw_handle(), 0) },
        WAIT_TIMEOUT
    );
    assert!(process.try_wait().unwrap().is_none());
    // Both root and descendant retain the control handles; neither drains data.
    io.start_receive().unwrap();
    io.start_send(
        serde_json::to_vec(&serde_json::json!({"data":"x".repeat(2*1024*1024)})).unwrap(),
    )
    .unwrap();
    thread::sleep(Duration::from_millis(50));
    assert!(io.try_receive().unwrap().is_none());
    assert!(!io.try_send().unwrap());
    process
        .terminate(Instant::now() + Duration::from_secs(10))
        .unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(descendant.as_raw_handle(), 0) },
        WAIT_OBJECT_0
    );
    io.shutdown(Instant::now() + Duration::from_secs(10))
        .unwrap();
}

#[test]
fn startup_rejects_mismatched_image_and_invalid_inputs_without_resuming_any_child() {
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(
        WindowsToolWorkerProcess::spawn_current(
            &format!("sha256:{}", "0".repeat(64)),
            &[],
            &BTreeMap::new(),
            temp.path(),
            vec![]
        )
        .err()
        .unwrap()
        .to_string(),
        "TOOL_WORKER_IMAGE_DIGEST_MISMATCH"
    );
    for arguments in [
        vec![OsString::from("bad\0argument")],
        vec![OsString::from("x".repeat(WIDE_LIMIT))],
    ] {
        assert!(WindowsToolWorkerProcess::spawn_current(
            "unused",
            &arguments,
            &BTreeMap::new(),
            temp.path(),
            vec![]
        )
        .is_err());
    }
    assert!(environment_block(&BTreeMap::from([
        ("Path".into(), "a".into()),
        ("PATH".into(), "b".into())
    ]))
    .is_err());
    assert!(environment_block(&BTreeMap::from([("A".into(), "a\0b".into())])).is_err());
    assert_eq!(environment_block(&BTreeMap::new()).unwrap(), [0, 0]);
    let (mut reader, writer) = std::io::pipe().unwrap();
    let handle = unsafe { OwnedHandle::from_raw_handle(writer.into_raw_handle()) };
    assert_eq!(
        WindowsToolWorkerProcess::spawn_current(
            &digest(),
            &[],
            &BTreeMap::new(),
            &temp.path().join("missing"),
            vec![handle]
        )
        .err()
        .unwrap()
        .to_string(),
        "TOOL_WORKER_SPAWN_FAILED"
    );
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    assert!(
        bytes.is_empty(),
        "failed launch must close supplied handle copies"
    );
}

#[test]
fn quoting_roundtrips_through_native_windows_argument_parser() {
    use windows_sys::Win32::{Foundation::LocalFree, UI::Shell::CommandLineToArgvW};
    let expected: Vec<OsString> = ["", "a b", "a\"b", "a\\", "a\\\"b", "日本語", "\\\\"]
        .into_iter()
        .map(OsString::from)
        .collect();
    let line = command_line(&expected).unwrap();
    let mut count = 0;
    let arguments = unsafe { CommandLineToArgvW(line.as_ptr(), &mut count) };
    assert!(!arguments.is_null());
    assert_eq!(count as usize, expected.len() + 1);
    for (index, value) in expected.iter().enumerate() {
        let pointer = unsafe { *arguments.add(index + 1) };
        let mut length = 0;
        while unsafe { *pointer.add(length) } != 0 {
            length += 1;
        }
        assert_eq!(
            unsafe { std::slice::from_raw_parts(pointer, length) },
            value.encode_wide().collect::<Vec<_>>()
        );
    }
    unsafe {
        LocalFree(arguments.cast());
    }
}

#[test]
fn child_exit_releases_incomplete_frame_and_preserves_native_dword() {
    let temp = tempfile::tempdir().unwrap();
    let (parent, child) = native_tool_worker_channel().unwrap();
    let (reader, source_writer) = child.split();
    let writer = source_writer.as_handle().try_clone_to_owned().unwrap();
    drop(reader);
    drop(source_writer);
    let mut arguments = fixture_arguments("partial");
    arguments.push(parameter("write", writer.as_raw_handle()));
    let mut process = WindowsToolWorkerProcess::spawn_current(
        &digest(),
        &arguments,
        &BTreeMap::new(),
        temp.path(),
        vec![writer],
    )
    .unwrap();
    let mut io = NativeToolWorkerIo::new(parent).unwrap();
    io.start_receive().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match io.try_receive() {
            Err(_) => break,
            Ok(None) => {}
            Ok(Some(_)) => panic!("incomplete frame was accepted"),
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(process.terminate(deadline).unwrap(), 0xd00d0013);
    io.shutdown(deadline).unwrap();
}

#[test]
#[ignore = "same-binary subprocess fixture invoked by Windows process tests"]
fn native_process_fixture() {
    if std::env::args().any(|arg| arg == "--skip=role=partial") {
        let mut writer = unsafe { File::from_raw_handle(handle_argument("write")) };
        writer.write_all(&[0, 0]).unwrap();
        std::process::exit(0xd00d0013u32 as i32);
    }
    if std::env::args().any(|arg| arg == "--skip=role=leaf") {
        loop {
            thread::sleep(Duration::from_secs(60));
        }
    }
    assert!(std::env::args().any(|arg| arg == ARGUMENT));
    assert_eq!(std::env::var("OYZU_PROBE").unwrap(), "explicit");
    assert!(std::env::var_os("PATH").is_none());
    unsafe {
        SetEvent(handle_argument("unrelated"));
        assert_ne!(
            SetEvent(handle_argument("allowed")),
            0,
            "{}",
            GetLastError()
        );
    }
    let reader = unsafe { File::from_raw_handle(handle_argument("read")) };
    let writer = unsafe { File::from_raw_handle(handle_argument("write")) };
    let descendant_handles = vec![
        reader.as_handle().try_clone_to_owned().unwrap(),
        writer.as_handle().try_clone_to_owned().unwrap(),
    ];
    let child = WindowsToolWorkerProcess::spawn_current(
        &digest(),
        &fixture_arguments("leaf"),
        &BTreeMap::new(),
        &std::env::current_dir().unwrap(),
        descendant_handles,
    )
    .unwrap();
    let (mut receive, mut send) = split_tool_worker_channel(reader, writer);
    assert_eq!(
        receive.receive().unwrap(),
        serde_json::json!({"request":true})
    );
    let pid = unsafe { GetProcessId(child.process.as_raw_handle()) };
    send.send(&serde_json::to_vec(&serde_json::json!({"descendant":pid})).unwrap())
        .unwrap();
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
