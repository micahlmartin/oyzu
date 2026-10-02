//! Owns one Windows worker job and confirmed job-wide termination. The native
//! spawn adapter must create suspended, assign here, then resume; image/channel
//! admission and thread/primary-process handles belong to that adapter.
use anyhow::{ensure, Context, Result};
use std::{
    mem::size_of,
    os::windows::io::{AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    ptr::{null, null_mut},
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::System::JobObjects::*;

/// A private, non-inheritable job for exactly one initial worker and its
/// descendants. No breakaway flags are enabled. Dropping the last job handle
/// requests termination; only terminate() confirms an empty job before return.
/// A job is lifecycle containment, not a sandbox or authorization to execute.
pub struct WindowsToolWorkerJob {
    handle: OwnedHandle,
    assigned: bool,
    stopping: bool,
}

impl WindowsToolWorkerJob {
    pub fn new() -> Result<Self> {
        // Null security attributes create a non-inheritable, unnamed job.
        let handle = unsafe { CreateJobObjectW(null(), null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error()).context("TOOL_WORKER_JOB_CREATE_FAILED");
        }
        // Successful creation transfers this unique handle to the owner.
        let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
        let limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION {
                LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                ..Default::default()
            },
            ..Default::default()
        };
        // The typed structure stays live until the synchronous API returns.
        let success = unsafe {
            SetInformationJobObject(
                handle.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        check(success, "TOOL_WORKER_JOB_CONFIGURE_FAILED")?;
        Ok(Self {
            handle,
            assigned: false,
            stopping: false,
        })
    }

    /// Assign the sole initial process before its primary thread is resumed.
    /// The caller must supply a live process created with CREATE_SUSPENDED and
    /// terminate/reap it if assignment fails. This method does not authenticate
    /// the image, inspect suspension or transfer the process handle's ownership.
    /// Even a failed assignment consumes the attempt; never reuse the job.
    pub fn assign_suspended(&mut self, process: BorrowedHandle<'_>) -> Result<()> {
        ensure!(
            !self.assigned && !self.stopping,
            "TOOL_WORKER_JOB_SEQUENCE_INVALID"
        );
        self.assigned = true;
        // The borrowed process and owned job handles remain live during the call.
        let success = unsafe {
            AssignProcessToJobObject(self.handle.as_raw_handle(), process.as_raw_handle())
        };
        check(success, "TOOL_WORKER_JOB_ASSIGN_FAILED")?;
        Ok(())
    }

    fn active_processes(&self) -> Result<u32> {
        let mut accounting = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        // The fixed-size output is correctly aligned, writable and kept live.
        let success = unsafe {
            QueryInformationJobObject(
                self.handle.as_raw_handle(),
                JobObjectBasicAccountingInformation,
                (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                null_mut(),
            )
        };
        check(success, "TOOL_WORKER_JOB_QUERY_FAILED")?;
        Ok(accounting.ActiveProcesses)
    }

    /// Terminate the entire job and observe zero active processes by an absolute
    /// monotonic deadline. Timeout/error retains the job handle and prevents new
    /// assignment. Retry cleanup on the same owner; never infer exit from the
    /// TerminateJobObject return alone. I/O joining is a separate obligation.
    pub fn terminate(&mut self, deadline: Instant) -> Result<()> {
        self.stopping = true;
        // Termination is idempotent for an empty job. No handles are released by
        // this request; the job remains owned through confirmation or failure.
        let success = unsafe { TerminateJobObject(self.handle.as_raw_handle(), 130) };
        check(success, "TOOL_WORKER_JOB_TERMINATE_FAILED")?;
        loop {
            if self
                .active_processes()
                .context("TOOL_WORKER_JOB_CLEANUP_INCOMPLETE")?
                == 0
            {
                return Ok(());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(!remaining.is_zero(), "TOOL_WORKER_JOB_CLEANUP_TIMEOUT");
            thread::sleep(remaining.min(Duration::from_millis(5)));
        }
    }
}

fn check(success: i32, code: &'static str) -> Result<()> {
    if success == 0 {
        Err(std::io::Error::last_os_error()).context(code)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "windows_job_tests.rs"]
mod tests;
