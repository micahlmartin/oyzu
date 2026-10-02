//! Owns a Windows worker process and its control I/O through joint cleanup.
//! Launch admission, bootstrap, protocol sequencing and deadline policy stay with
//! the supervisor. This owner never treats a termination request as observed exit.
use super::{NativeToolWorkerIo, WindowsToolWorkerProcess};
use anyhow::{ensure, Context, Result};
use std::time::Instant;

/// Lifecycle ownership for an already-created worker and its matching channel.
/// The caller must associate the correct endpoint; construction authenticates
/// neither the process nor its messages. No second operation may reuse this owner.
pub struct WindowsToolWorkerLifecycle {
    // Field order is intentional: closing the job requests tree termination
    // before I/O Drop waits for dedicated threads. Never reverse these fields.
    process: WindowsToolWorkerProcess,
    io: NativeToolWorkerIo,
    stopping: bool,
}

impl WindowsToolWorkerLifecycle {
    pub fn new(process: WindowsToolWorkerProcess, io: NativeToolWorkerIo) -> Self {
        Self {
            process,
            io,
            stopping: false,
        }
    }

    /// Borrow protocol transport only while cleanup has not begun. Process exit
    /// must still be observed separately; transport availability is no liveness proof.
    pub fn control(&mut self) -> Result<&mut NativeToolWorkerIo> {
        ensure!(!self.stopping, "TOOL_WORKER_STOPPING");
        Ok(&mut self.io)
    }

    pub fn try_wait(&self) -> Result<Option<u32>> {
        self.process.try_wait()
    }

    /// Stop both resources under one absolute cleanup deadline. Always attempts
    /// I/O joining even if process termination fails. Success proves job-wide exit,
    /// initial-process exit and joined I/O threads; failure retains both owners for
    /// retry. The deadline is not reset between phases and cannot bound OS syscalls.
    pub fn shutdown(&mut self, deadline: Instant) -> Result<u32> {
        self.stopping = true;
        self.io.begin_shutdown();
        let process = self.process.terminate(deadline);
        let io = self.io.shutdown(deadline);
        match (process, io) {
            (Ok(code), Ok(())) => Ok(code),
            (Err(error), Ok(())) => Err(error).context("TOOL_WORKER_PROCESS_CLEANUP_FAILED"),
            (Ok(_), Err(error)) => Err(error).context("TOOL_WORKER_CONTROL_CLEANUP_FAILED"),
            (Err(process), Err(io)) => Err(io).context(format!(
                "TOOL_WORKER_CLEANUP_FAILED; process: {process:#}; control"
            )),
        }
    }
}

impl Drop for WindowsToolWorkerLifecycle {
    fn drop(&mut self) {
        self.stopping = true;
        self.io.begin_shutdown();
        // Fields then drop in declaration order. Explicit shutdown is required
        // to report confirmed process cleanup; Drop alone makes no such claim.
    }
}
