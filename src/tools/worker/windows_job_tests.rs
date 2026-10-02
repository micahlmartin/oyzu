use super::*;
use std::{
    ffi::OsStr,
    os::windows::{ffi::OsStrExt, io::AsHandle, process::CommandExt},
    path::Path,
    process::Command,
};
use windows_sys::Win32::{
    Foundation::{GetHandleInformation, HANDLE_FLAG_INHERIT, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{
        CreateProcessW, OpenProcess, ResumeThread, TerminateProcess, WaitForSingleObject,
        CREATE_BREAKAWAY_FROM_JOB, CREATE_NO_WINDOW, CREATE_SUSPENDED, INFINITE,
        PROCESS_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, STARTUPINFOW,
    },
};

const CHILD_TEST: &str = "tools::worker::windows_job::tests::native_job_child_fixture";

struct Child {
    process: OwnedHandle,
    primary_thread: OwnedHandle,
}
impl Drop for Child {
    fn drop(&mut self) {
        // Test failure cleanup: never strand a suspended/running initial child.
        unsafe {
            TerminateProcess(self.process.as_raw_handle(), 130);
            WaitForSingleObject(self.process.as_raw_handle(), INFINITE);
        }
    }
}

fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(Some(0)).collect()
}

fn spawn(job: &mut WindowsToolWorkerJob, marker: &Path, role: &str) -> Child {
    let image = std::env::current_exe().unwrap();
    let image_wide = wide(image.as_os_str());
    // Native Windows paths cannot contain a quote. Both quoted path arguments
    // end in a filename, not a backslash. Remaining tokens are static fixture IDs.
    let command = format!(
        "\"{}\" --exact {CHILD_TEST} --ignored --nocapture --skip=oyzu-role-{role} \"--skip=oyzu-marker={}\"",
        image.display(), marker.display()
    );
    let mut command = wide(OsStr::new(&command));
    let startup = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();
    // This test adapter creates only this test executable, suspended and hidden.
    // It does not implement production image verification or channel inheritance.
    let success = unsafe {
        CreateProcessW(
            image_wide.as_ptr(),
            command.as_mut_ptr(),
            null(),
            null(),
            0,
            CREATE_SUSPENDED | CREATE_NO_WINDOW,
            null(),
            null(),
            &startup,
            &mut process,
        )
    };
    assert_ne!(success, 0, "{}", std::io::Error::last_os_error());
    let child = Child {
        process: unsafe { OwnedHandle::from_raw_handle(process.hProcess) },
        primary_thread: unsafe { OwnedHandle::from_raw_handle(process.hThread) },
    };
    job.assign_suspended(child.process.as_handle()).unwrap();
    assert_ne!(
        unsafe { ResumeThread(child.primary_thread.as_raw_handle()) },
        u32::MAX
    );
    child
}

fn wait_exit(process: &OwnedHandle) {
    assert_eq!(
        unsafe { WaitForSingleObject(process.as_raw_handle(), 10_000) },
        WAIT_OBJECT_0
    );
}

fn descendant(marker: &Path) -> OwnedHandle {
    let deadline = Instant::now() + Duration::from_secs(10);
    let pid = loop {
        if let Ok(text) = std::fs::read_to_string(marker) {
            if let Ok(pid) = text.parse::<u32>() {
                break pid;
            }
        }
        assert!(Instant::now() < deadline, "child did not report descendant");
        thread::sleep(Duration::from_millis(5));
    };
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        )
    };
    assert!(!handle.is_null());
    let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
    assert_eq!(
        unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) },
        WAIT_TIMEOUT
    );
    handle
}

fn contains(job: &WindowsToolWorkerJob, process: &OwnedHandle) -> bool {
    let mut member = 0;
    assert_ne!(
        unsafe {
            IsProcessInJob(
                process.as_raw_handle(),
                job.handle.as_raw_handle(),
                &mut member,
            )
        },
        0
    );
    member != 0
}
#[test]
fn native_job_termination_includes_descendants_and_excludes_independent_worker() {
    let temp = tempfile::tempdir().unwrap();
    let mut job = WindowsToolWorkerJob::new().unwrap();
    let mut flags = 0;
    assert_ne!(
        unsafe { GetHandleInformation(job.handle.as_raw_handle(), &mut flags) },
        0
    );
    assert_eq!(flags & HANDLE_FLAG_INHERIT, 0);
    let marker = temp.path().join("descendant.pid");
    let root = spawn(&mut job, &marker, "parent");
    let descendant = descendant(&marker);
    assert!(job.active_processes().unwrap() >= 2);
    assert!(contains(&job, &root.process));
    assert!(contains(&job, &descendant));
    let mut independent = WindowsToolWorkerJob::new().unwrap();
    let other = spawn(&mut independent, &temp.path().join("unused"), "leaf");
    assert!(job.assign_suspended(root.process.as_handle()).is_err());
    job.terminate(Instant::now() + Duration::from_secs(10))
        .unwrap();
    assert_eq!(job.active_processes().unwrap(), 0);
    wait_exit(&root.process);
    wait_exit(&descendant);
    assert_eq!(
        unsafe { WaitForSingleObject(other.process.as_raw_handle(), 0) },
        WAIT_TIMEOUT
    );
    assert!(independent.active_processes().unwrap() >= 1);
    assert!(contains(&independent, &other.process));
    assert!(!contains(&job, &other.process));
    job.terminate(Instant::now() + Duration::from_secs(10))
        .unwrap();
    independent
        .terminate(Instant::now() + Duration::from_secs(10))
        .unwrap();
    wait_exit(&other.process);
}

#[test]
fn last_job_handle_close_terminates_the_real_process_tree() {
    let temp = tempfile::tempdir().unwrap();
    let mut job = WindowsToolWorkerJob::new().unwrap();
    let marker = temp.path().join("descendant.pid");
    let root = spawn(&mut job, &marker, "parent");
    let descendant = descendant(&marker);
    drop(job);
    // Drop requests termination. These independent process waits supply the
    // confirmation; closing the handle alone is not reported as observed exit.
    wait_exit(&root.process);
    wait_exit(&descendant);
}

#[test]
fn terminating_an_empty_job_permanently_closes_assignment() {
    let mut job = WindowsToolWorkerJob::new().unwrap();
    job.terminate(Instant::now()).unwrap();
    assert!(job.stopping);
    assert_eq!(job.active_processes().unwrap(), 0);
}

#[test]
fn failed_assignment_consumes_the_job_attempt() {
    let mut job = WindowsToolWorkerJob::new().unwrap();
    let file = tempfile::tempfile().unwrap();
    assert_eq!(
        job.assign_suspended(file.as_handle())
            .unwrap_err()
            .to_string(),
        "TOOL_WORKER_JOB_ASSIGN_FAILED"
    );
    assert_eq!(
        job.assign_suspended(file.as_handle())
            .unwrap_err()
            .to_string(),
        "TOOL_WORKER_JOB_SEQUENCE_INVALID"
    );
    job.terminate(Instant::now()).unwrap();
}

#[test]
#[ignore = "subprocess fixture invoked only by native job tests"]
fn native_job_child_fixture() {
    let arguments: Vec<_> = std::env::args_os().collect();
    if arguments
        .iter()
        .any(|value| value == "--skip=oyzu-role-parent")
    {
        let marker = arguments
            .iter()
            .find_map(|value| value.to_str()?.strip_prefix("--skip=oyzu-marker="))
            .unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            CHILD_TEST,
            "--ignored",
            "--nocapture",
            "--skip=oyzu-role-leaf",
        ]);
        command.creation_flags(CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB);
        if let Ok(mut escaped) = command.spawn() {
            escaped.kill().unwrap();
            escaped.wait().unwrap();
            panic!("a worker descendant escaped its job");
        }
        command.creation_flags(CREATE_NO_WINDOW);
        let mut child = command.spawn().unwrap();
        std::fs::write(marker, child.id().to_string()).unwrap();
        // Deliberately retain a live descendant. The enclosing job must stop it.
        child.wait().unwrap();
        panic!("descendant exited before its parent was terminated");
    }
    assert!(arguments
        .iter()
        .any(|value| value == "--skip=oyzu-role-leaf"));
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
