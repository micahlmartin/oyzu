//! Native private transport allocation only. Process inheritance, image identity,
//! OS deadlines and termination belong to the supervisor, not this factory.
use std::io::{self, Read, Write};

#[cfg(unix)]
type Reader = std::os::unix::net::UnixStream;
#[cfg(unix)]
type Writer = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Reader = io::PipeReader;
#[cfg(windows)]
type Writer = io::PipeWriter;

/// One endpoint of a private control transport. No stdin/stdout, filesystem
/// socket name, listening port or environment capability is used. Handles begin
/// non-inheritable; a supervisor must explicitly arrange restricted inheritance.
pub struct NativeToolWorkerEndpoint {
    reader: Reader,
    writer: Writer,
}
/// Separately owned native read half. Blocking I/O has no implicit deadline.
pub struct NativeToolWorkerReader(Reader);
/// Separately owned native write half. Blocking I/O has no implicit deadline.
pub struct NativeToolWorkerWriter(Writer);

/// Allocate two connected endpoints: Unix socketpair or two Windows anonymous
/// pipes. Checks non-inheritance before returning. This does not spawn, authorize
/// or contain a worker, and provides no protection from hostile same-user code.
pub fn native_tool_worker_channel(
) -> io::Result<(NativeToolWorkerEndpoint, NativeToolWorkerEndpoint)> {
    #[cfg(unix)]
    let endpoints = {
        let (parent, worker) = std::os::unix::net::UnixStream::pair()?;
        (
            NativeToolWorkerEndpoint {
                reader: parent.try_clone()?,
                writer: parent,
            },
            NativeToolWorkerEndpoint {
                reader: worker.try_clone()?,
                writer: worker,
            },
        )
    };
    #[cfg(windows)]
    let endpoints = {
        let (worker_reader, parent_writer) = io::pipe()?;
        let (parent_reader, worker_writer) = io::pipe()?;
        (
            NativeToolWorkerEndpoint {
                reader: parent_reader,
                writer: parent_writer,
            },
            NativeToolWorkerEndpoint {
                reader: worker_reader,
                writer: worker_writer,
            },
        )
    };
    for endpoint in [&endpoints.0, &endpoints.1] {
        non_inherited(&endpoint.reader)?;
        non_inherited(&endpoint.writer)?;
    }
    Ok(endpoints)
}

impl NativeToolWorkerEndpoint {
    /// Consume the endpoint without duplicating handles. Pass these halves to
    /// split_tool_worker_channel for shared framing limits and failure state.
    pub fn split(self) -> (NativeToolWorkerReader, NativeToolWorkerWriter) {
        (
            NativeToolWorkerReader(self.reader),
            NativeToolWorkerWriter(self.writer),
        )
    }
}
impl Read for NativeToolWorkerEndpoint {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.reader.read(bytes)
    }
}
impl Write for NativeToolWorkerEndpoint {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writer.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
impl Read for NativeToolWorkerReader {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.read(bytes)
    }
}
impl Write for NativeToolWorkerWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

// Borrowed handles let the future supervisor configure explicit inheritance and
// native cancellation without transferring ownership or reopening a public name.
#[cfg(unix)]
impl std::os::fd::AsFd for NativeToolWorkerReader {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        std::os::fd::AsFd::as_fd(&self.0)
    }
}
#[cfg(unix)]
impl std::os::fd::AsFd for NativeToolWorkerWriter {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        std::os::fd::AsFd::as_fd(&self.0)
    }
}
#[cfg(windows)]
impl std::os::windows::io::AsHandle for NativeToolWorkerReader {
    fn as_handle(&self) -> std::os::windows::io::BorrowedHandle<'_> {
        std::os::windows::io::AsHandle::as_handle(&self.0)
    }
}
#[cfg(windows)]
impl std::os::windows::io::AsHandle for NativeToolWorkerWriter {
    fn as_handle(&self) -> std::os::windows::io::BorrowedHandle<'_> {
        std::os::windows::io::AsHandle::as_handle(&self.0)
    }
}

#[cfg(unix)]
fn non_inherited(handle: &impl std::os::fd::AsRawFd) -> io::Result<()> {
    // F_GETFD observes a live borrowed descriptor and transfers no ownership.
    let flags = unsafe { libc::fcntl(handle.as_raw_fd(), libc::F_GETFD) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    if flags & libc::FD_CLOEXEC == 0 {
        return Err(io::Error::other("worker channel descriptor is inheritable"));
    }
    Ok(())
}
#[cfg(windows)]
fn non_inherited(handle: &impl std::os::windows::io::AsRawHandle) -> io::Result<()> {
    use windows_sys::Win32::Foundation::{GetHandleInformation, HANDLE_FLAG_INHERIT};
    let mut flags = 0;
    // The handle remains owned by the endpoint for the duration of this query.
    if unsafe { GetHandleInformation(handle.as_raw_handle(), &mut flags) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if flags & HANDLE_FLAG_INHERIT != 0 {
        return Err(io::Error::other("worker channel handle is inheritable"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{split_tool_worker_channel, ToolWorkerChannel};
    use std::{sync::mpsc, thread, time::Duration};

    #[test]
    fn peer_close_interrupts_incomplete_native_frame_read() {
        let (parent, mut peer) = native_tool_worker_channel().unwrap();
        // Leave a valid length prefix incomplete. No complete frame can be
        // returned until the peer writes more bytes or closes its endpoint.
        peer.write_all(&[0, 0]).unwrap();
        let (reader, writer) = parent.split();
        let (mut receiver, mut sender) = split_tool_worker_channel(reader, writer);
        let (done, completion) = mpsc::channel();
        let read = thread::spawn(move || {
            let _ = done.send(receiver.receive());
        });
        assert!(matches!(
            completion.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        drop(peer);
        assert!(completion
            .recv_timeout(Duration::from_secs(15))
            .unwrap()
            .is_err());
        read.join().unwrap();
        // Read failure must also close the framing state in the other direction.
        assert!(sender.send(b"{}").is_err());
    }

    #[test]
    fn peer_close_interrupts_backpressured_native_frame_write() {
        let (parent, peer) = native_tool_worker_channel().unwrap();
        let (reader, writer) = parent.split();
        let (mut receiver, mut sender) = split_tool_worker_channel(reader, writer);
        let payload = serde_json::to_vec(&serde_json::json!({
            "data": "x".repeat(2 * 1024 * 1024)
        }))
        .unwrap();
        let (done, completion) = mpsc::channel();
        let write = thread::spawn(move || {
            let _ = done.send(sender.send(&payload));
        });
        // The peer never drains the native transport. This exceeds the default
        // socket/anonymous-pipe capacity on the qualified hosts.
        assert!(matches!(
            completion.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        drop(peer);
        assert!(completion
            .recv_timeout(Duration::from_secs(15))
            .unwrap()
            .is_err());
        write.join().unwrap();
        assert!(receiver.receive().is_err());
    }

    #[test]
    fn private_native_channel_transfers_large_frames_and_observes_peer_close() {
        // Bound the test even if a transport regression blocks an I/O thread.
        // This watchdog is not the missing production supervisor deadline.
        let (done, completion) = mpsc::channel();
        thread::spawn(move || {
            let result = (|| -> anyhow::Result<()> {
                let (parent, worker) = native_tool_worker_channel()?;
                let payload = serde_json::json!({"data":"x".repeat(2 * 1024 * 1024)});
                let encoded = serde_json::to_vec(&payload)?;
                let expected = payload.clone();
                let peer = thread::spawn(move || -> anyhow::Result<()> {
                    let mut channel = ToolWorkerChannel::new(worker);
                    let request = channel.receive()?;
                    assert_eq!(request, expected);
                    channel.send(&serde_json::to_vec(&request)?)?;
                    Ok(())
                });
                let (reader, writer) = parent.split();
                let (mut receiver, mut sender) = split_tool_worker_channel(reader, writer);
                let receive = thread::spawn(move || {
                    let result = receiver.receive();
                    (receiver, result)
                });
                sender.send(&encoded)?;
                let (mut receiver, result) = receive.join().unwrap();
                assert_eq!(result?, payload);
                peer.join().unwrap()?;
                assert!(receiver.receive().is_err());
                assert!(sender.send(b"{}").is_err());
                Ok(())
            })();
            let _ = done.send(result);
        });
        completion
            .recv_timeout(Duration::from_secs(15))
            .unwrap()
            .unwrap();
    }
}
