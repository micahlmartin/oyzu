//! Owns native control I/O threads through interruption and confirmed joining.
//! Framing owns record limits; the supervisor owns deadlines and process cleanup.
use super::{
    framing::Control, split_tool_worker_channel, NativeToolWorkerEndpoint, NativeToolWorkerReader,
    NativeToolWorkerWriter, ToolWorkerFrame, ToolWorkerReceiver, ToolWorkerSender,
};
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

type Receiver = ToolWorkerReceiver<NativeToolWorkerReader>;
type Sender = ToolWorkerSender<NativeToolWorkerWriter>;
type ReadTask = JoinHandle<(Receiver, Result<ToolWorkerFrame>)>;
type WriteTask = JoinHandle<(Sender, Result<()>)>;

/// Dedicated read/write threads for one native private channel. At most one
/// frame per direction is in flight; no unbounded queue or thread pool is used.
/// Results remain untrusted. This neither spawns nor terminates a worker process.
///
/// Call shutdown with the supervisor's absolute monotonic cleanup deadline. A
/// timeout retains ownership for another shutdown attempt. Drop also interrupts
/// and joins; it may wait if the OS fails to complete cancellation, rather than
/// detaching a thread that still owns operation resources.
pub struct NativeToolWorkerIo {
    receiver: Option<Receiver>,
    sender: Option<Sender>,
    read: Option<ReadTask>,
    write: Option<WriteTask>,
    control: Arc<Control>,
    closed: bool,
    #[cfg(unix)]
    shutdown_handle: std::os::unix::net::UnixStream,
}

impl NativeToolWorkerIo {
    pub fn new(endpoint: NativeToolWorkerEndpoint) -> Result<Self> {
        #[cfg(unix)]
        let shutdown_handle = endpoint.shutdown_handle()?;
        let (reader, writer) = endpoint.split();
        let (receiver, sender) = split_tool_worker_channel(reader, writer);
        let control = receiver.control();
        Ok(Self {
            receiver: Some(receiver),
            sender: Some(sender),
            read: None,
            write: None,
            control,
            closed: false,
            #[cfg(unix)]
            shutdown_handle,
        })
    }

    /// Begin one frame read without blocking the caller or the send direction.
    pub fn start_receive(&mut self) -> Result<()> {
        ensure!(!self.closed, "TOOL_WORKER_CHANNEL_CLOSED");
        self.control.open()?;
        ensure!(self.read.is_none(), "TOOL_WORKER_IO_PENDING");
        let mut receiver = self.receiver.take().context("TOOL_WORKER_CHANNEL_CLOSED")?;
        match thread::Builder::new()
            .name("oyzu-tool-read".into())
            .spawn(move || {
                let result = receiver.receive_frame();
                (receiver, result)
            }) {
            Ok(task) => self.read = Some(task),
            Err(error) => {
                self.abort();
                return Err(error).context("TOOL_WORKER_IO_START_FAILED");
            }
        }
        Ok(())
    }

    /// Begin one bounded frame write. Ownership prevents mutation while writing.
    /// Validation precedes thread creation; framing rechecks the shared budget.
    pub fn start_send(&mut self, bytes: Vec<u8>) -> Result<()> {
        ensure!(!self.closed, "TOOL_WORKER_CHANNEL_CLOSED");
        self.control.open()?;
        ensure!(self.write.is_none(), "TOOL_WORKER_IO_PENDING");
        if let Err(error) = super::parse(&bytes) {
            self.abort();
            return Err(error);
        }
        let mut sender = self.sender.take().context("TOOL_WORKER_CHANNEL_CLOSED")?;
        match thread::Builder::new()
            .name("oyzu-tool-write".into())
            .spawn(move || {
                let result = sender.send(&bytes);
                (sender, result)
            }) {
            Ok(task) => self.write = Some(task),
            Err(error) => {
                self.abort();
                return Err(error).context("TOOL_WORKER_IO_START_FAILED");
            }
        }
        Ok(())
    }

    /// None means the single pending read has not completed. A completed read
    /// is joined before exposing its result. Cancellation discards racing output.
    pub fn try_receive(&mut self) -> Result<Option<Value>> {
        self.try_receive_frame()
            .map(|frame| frame.map(ToolWorkerFrame::into_value))
    }

    /// Collect a validated frame retaining exact wire bytes for bootstrap binding.
    /// Shares the pending read with try_receive; either consumes that one result.
    pub fn try_receive_frame(&mut self) -> Result<Option<ToolWorkerFrame>> {
        ensure!(!self.closed, "TOOL_WORKER_CHANNEL_CLOSED");
        self.control.open()?;
        let task = self.read.as_ref().context("TOOL_WORKER_IO_NOT_STARTED")?;
        if !task.is_finished() {
            return Ok(None);
        }
        match self.read.take().unwrap().join() {
            Ok((receiver, Ok(value))) => {
                self.receiver = Some(receiver);
                Ok(Some(value))
            }
            Ok((_, Err(error))) => {
                self.abort();
                Err(error)
            }
            Err(_) => {
                self.abort();
                anyhow::bail!("TOOL_WORKER_IO_PANICKED")
            }
        }
    }

    /// False means the single pending write has not completed. True confirms
    /// its thread was joined; it does not prove that the peer admitted the frame.
    pub fn try_send(&mut self) -> Result<bool> {
        ensure!(!self.closed, "TOOL_WORKER_CHANNEL_CLOSED");
        self.control.open()?;
        let task = self.write.as_ref().context("TOOL_WORKER_IO_NOT_STARTED")?;
        if !task.is_finished() {
            return Ok(false);
        }
        match self.write.take().unwrap().join() {
            Ok((sender, Ok(()))) => {
                self.sender = Some(sender);
                Ok(true)
            }
            Ok((_, Err(error))) => {
                self.abort();
                Err(error)
            }
            Err(_) => {
                self.abort();
                anyhow::bail!("TOOL_WORKER_IO_PANICKED")
            }
        }
    }

    fn abort(&mut self) {
        self.closed = true;
        self.control.abort();
    }

    fn interrupt(&self) {
        #[cfg(unix)]
        {
            // All cloned descriptors name the same socket. Shutdown wakes both
            // directions even while the peer (or its descendant) remains open.
            let _ = self.shutdown_handle.shutdown(std::net::Shutdown::Both);
        }
        #[cfg(windows)]
        {
            fn cancel<T>(task: &JoinHandle<T>) {
                use std::os::windows::io::AsRawHandle;
                // The retained JoinHandle owns this exact dedicated thread until
                // join. It never runs another operation or enters a thread pool.
                // Cancellation may race the next syscall in the same frame;
                // repeat until is_finished, not merely until this call succeeds.
                unsafe {
                    windows_sys::Win32::System::IO::CancelSynchronousIo(task.as_raw_handle());
                }
            }
            if let Some(task) = &self.read {
                cancel(task);
            }
            if let Some(task) = &self.write {
                cancel(task);
            }
        }
    }

    fn join_finished(&mut self) -> bool {
        if self.read.as_ref().is_some_and(JoinHandle::is_finished) {
            let _ = self.read.take().unwrap().join();
        }
        if self.write.as_ref().is_some_and(JoinHandle::is_finished) {
            let _ = self.write.take().unwrap().join();
        }
        self.read.is_none() && self.write.is_none()
    }

    /// Permanently abort, interrupt both native directions and join their
    /// threads. Discards even a successful result racing shutdown. Repeated
    /// calls are permitted; expiry leaves pending handles owned by this value.
    pub fn shutdown(&mut self, deadline: Instant) -> Result<()> {
        self.abort();
        self.receiver.take();
        self.sender.take();
        loop {
            self.interrupt();
            if self.join_finished() {
                return Ok(());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(!remaining.is_zero(), "TOOL_WORKER_IO_SHUTDOWN_TIMEOUT");
            thread::sleep(remaining.min(Duration::from_millis(5)));
        }
    }
}

impl Drop for NativeToolWorkerIo {
    fn drop(&mut self) {
        self.abort();
        self.receiver.take();
        self.sender.take();
        loop {
            self.interrupt();
            if self.join_finished() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}

#[cfg(test)]
#[path = "io_tests.rs"]
mod tests;
