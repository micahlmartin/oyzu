use super::*;
use std::{
    io::{self, Cursor},
    sync::mpsc,
    thread,
    time::Duration,
};

struct WaitingReader {
    entered: Option<mpsc::Sender<()>>,
    incoming: mpsc::Receiver<Vec<u8>>,
    bytes: Cursor<Vec<u8>>,
}
impl Read for WaitingReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if let Some(entered) = self.entered.take() {
            entered.send(()).unwrap();
            self.bytes = Cursor::new(
                self.incoming
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))?,
            );
        }
        self.bytes.read(output)
    }
}
fn frame(bytes: &[u8]) -> Vec<u8> {
    [(bytes.len() as u32).to_be_bytes().as_slice(), bytes].concat()
}

#[test]
fn cancellation_write_is_independent_of_blocked_receive_and_abort_is_shared() {
    for abort in [false, true] {
        let (entered, waiting) = mpsc::channel();
        let (release, incoming) = mpsc::channel();
        let (mut receiver, mut sender) = split_tool_worker_channel(
            WaitingReader {
                entered: Some(entered),
                incoming,
                bytes: Cursor::new(vec![]),
            },
            Vec::new(),
        );
        let reading = thread::spawn(move || receiver.receive());
        waiting.recv_timeout(Duration::from_secs(2)).unwrap();
        let (written, completion) = mpsc::channel();
        let writing = thread::spawn(move || {
            written
                .send(sender.send(br#"{"operation":"cancel"}"#))
                .unwrap();
            sender
        });
        // The pending read has received no bytes. It must not block this write.
        completion
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        let sender = writing.join().unwrap();
        assert_eq!(sender.transport, frame(br#"{"operation":"cancel"}"#));
        if abort {
            sender.abort();
        }
        release.send(frame(br#"{"status":"cancelled"}"#)).unwrap();
        let received = reading.join().unwrap();
        if abort {
            assert!(received.unwrap_err().to_string().contains("CHANNEL_CLOSED"));
        } else {
            assert_eq!(received.unwrap()["status"], "cancelled");
        }
    }
}

#[test]
fn split_halves_share_limits_and_terminal_errors() {
    let (mut receiver, mut sender) =
        split_tool_worker_channel(Cursor::new(frame(b"{}")), Vec::new());
    receiver.control.remaining.store(3, Ordering::SeqCst);
    assert!(receiver
        .receive()
        .unwrap_err()
        .to_string()
        .contains("CONTROL_LIMIT"));
    assert_eq!(receiver.transport.position(), 0);
    assert!(sender.send(b"{}").is_err());
    assert!(sender.transport.is_empty());
    let (mut receiver, mut sender) =
        split_tool_worker_channel(Cursor::new(frame(b"{}")), Vec::new());
    receiver.control.remaining.store(11, Ordering::SeqCst);
    receiver.receive().unwrap();
    assert_eq!(sender.control.remaining.load(Ordering::SeqCst), 5);
    assert!(sender
        .send(b"{}")
        .unwrap_err()
        .to_string()
        .contains("CONTROL_LIMIT"));
    assert!(sender.transport.is_empty());
    assert!(receiver
        .receive()
        .unwrap_err()
        .to_string()
        .contains("CHANNEL_CLOSED"));

    let (mut receiver, mut sender) =
        split_tool_worker_channel(Cursor::new(frame(b"{}")), Vec::new());
    assert!(sender.send(b"[]").is_err());
    assert!(receiver.receive().is_err());
    assert_eq!(receiver.transport.position(), 0);

    let (mut receiver, mut sender) = split_tool_worker_channel(Cursor::new(vec![0, 0]), Vec::new());
    assert!(receiver.receive().is_err());
    assert!(sender.send(b"{}").is_err());
    assert!(sender.transport.is_empty());
}

#[cfg(unix)]
#[test]
fn unix_socketpair_transfers_cancel_while_response_read_is_pending() {
    use std::os::{fd::AsRawFd, unix::net::UnixStream};
    let (parent, child) = UnixStream::pair().unwrap();
    for socket in [&parent, &child] {
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        // Rust's socketpair must not become an ambient inherited capability.
        let flags = unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0);
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
    }
    let (mut receiver, mut sender) = split_tool_worker_channel(parent.try_clone().unwrap(), parent);
    let reading = thread::spawn(move || receiver.receive());
    let peer = thread::spawn(move || {
        let mut channel = ToolWorkerChannel::new(child);
        assert_eq!(channel.receive().unwrap()["operation"], "cancel");
        channel.send(br#"{"status":"cancelled"}"#).unwrap();
    });
    sender.send(br#"{"operation":"cancel"}"#).unwrap();
    assert_eq!(reading.join().unwrap().unwrap()["status"], "cancelled");
    peer.join().unwrap();
}

#[test]
fn simultaneous_read_write_cannot_overspend_shared_budget() {
    for _ in 0..8 {
        let (mut receiver, mut sender) =
            split_tool_worker_channel(Cursor::new(frame(b"{}")), Vec::new());
        receiver.control.remaining.store(11, Ordering::SeqCst);
        let ready = Arc::new(std::sync::Barrier::new(2));
        let read_ready = Arc::clone(&ready);
        let read = thread::spawn(move || {
            read_ready.wait();
            let result = receiver.receive();
            (result.is_ok(), receiver.transport.position() as usize)
        });
        let write = thread::spawn(move || {
            ready.wait();
            let result = sender.send(b"{}");
            (result.is_ok(), sender.transport.len())
        });
        let (read_ok, consumed) = read.join().unwrap();
        let (write_ok, written) = write.join().unwrap();
        assert!(!(read_ok && write_ok));
        assert!(consumed + written <= 11);
    }
}
