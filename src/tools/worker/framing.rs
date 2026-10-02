//! Shared frame budget and terminal state; no lock spans transport I/O.
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::{
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

const FRAME_BYTES: usize = 8 * 1024 * 1024;
const SESSION_BYTES: usize = 32 * 1024 * 1024;
#[cfg(test)]
#[path = "duplex_tests.rs"]
mod duplex_tests;

pub(super) struct Control {
    remaining: AtomicUsize,
    failed: AtomicBool,
}
impl Control {
    fn new() -> Self {
        Self {
            remaining: AtomicUsize::new(SESSION_BYTES),
            failed: AtomicBool::new(false),
        }
    }
    pub(super) fn open(&self) -> Result<()> {
        ensure!(
            !self.failed.load(Ordering::SeqCst),
            "TOOL_WORKER_CHANNEL_CLOSED"
        );
        Ok(())
    }
    fn reserve(&self, bytes: usize) -> Result<()> {
        self.open()?;
        self.remaining
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                remaining.checked_sub(bytes)
            })
            .map_err(|_| anyhow::anyhow!("TOOL_WORKER_CONTROL_LIMIT"))?;
        Ok(())
    }
    pub(super) fn abort(&self) {
        self.failed.store(true, Ordering::SeqCst);
    }
    fn finish<V>(&self, result: Result<V>) -> Result<V> {
        match result {
            Ok(value) => {
                self.open()?;
                Ok(value)
            }
            Err(error) => {
                self.abort();
                Err(error)
            }
        }
    }
}

/// Sequential framing for one private duplex channel. Both directions share
/// the 32 MiB control budget (including prefixes); frame bodies are at most 8 MiB.
/// Use split_tool_worker_channel when sending cancellation concurrently with a
/// blocked receive. Transport authentication, deadlines and shutdown remain the
/// supervisor's responsibility. This is an unstable internal interface.
pub struct ToolWorkerChannel<T> {
    transport: T,
    control: Arc<Control>,
}
impl<T: Read + Write> ToolWorkerChannel<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            control: Arc::new(Control::new()),
        }
    }
    /// Receive an untrusted JSON object; EOF or any other failure is terminal.
    pub fn receive(&mut self) -> Result<Value> {
        receive(&self.control, &mut self.transport)
    }
    /// Validate before writing; partial write/flush failure is terminal.
    pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
        send(&self.control, &mut self.transport, bytes)
    }
    /// Prevent subsequent successful operations; does not interrupt transport I/O.
    pub fn abort(&self) {
        self.control.abort();
    }
}

/// Receive half sharing its budget and failure state with exactly one sender.
/// A blocking read never holds a synchronization lock needed by the sender.
pub struct ToolWorkerReceiver<R> {
    transport: R,
    control: Arc<Control>,
}
/// Send half sharing its budget and failure state with exactly one receiver.
/// Serial ownership prevents interleaved frames from competing writers.
pub struct ToolWorkerSender<W> {
    transport: W,
    control: Arc<Control>,
}

/// Wrap already established read/write handles for the same trusted channel.
/// No handles are opened, duplicated or authenticated here. The caller must
/// enforce OS timeouts/shutdown: abort cannot interrupt an arbitrary Read/Write.
/// In-flight bytes cannot be retracted after a concurrent failure.
pub fn split_tool_worker_channel<R: Read, W: Write>(
    reader: R,
    writer: W,
) -> (ToolWorkerReceiver<R>, ToolWorkerSender<W>) {
    let control = Arc::new(Control::new());
    (
        ToolWorkerReceiver {
            transport: reader,
            control: Arc::clone(&control),
        },
        ToolWorkerSender {
            transport: writer,
            control,
        },
    )
}
impl<R: Read> ToolWorkerReceiver<R> {
    pub(super) fn control(&self) -> Arc<Control> {
        Arc::clone(&self.control)
    }
    pub fn receive(&mut self) -> Result<Value> {
        receive(&self.control, &mut self.transport)
    }
    pub fn abort(&self) {
        self.control.abort();
    }
}
impl<W: Write> ToolWorkerSender<W> {
    pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
        send(&self.control, &mut self.transport, bytes)
    }
    pub fn abort(&self) {
        self.control.abort();
    }
}

fn receive(control: &Control, transport: &mut impl Read) -> Result<Value> {
    control.open()?;
    let result = (|| {
        let mut prefix = [0; 4];
        control.reserve(prefix.len())?;
        transport
            .read_exact(&mut prefix)
            .map_err(|_| anyhow::anyhow!("TOOL_WORKER_CHANNEL_LOST"))?;
        let length = u32::from_be_bytes(prefix) as usize;
        frame_length(length)?;
        control.reserve(length)?;
        let mut bytes = vec![0; length];
        transport
            .read_exact(&mut bytes)
            .map_err(|_| anyhow::anyhow!("TOOL_WORKER_CHANNEL_LOST"))?;
        parse(&bytes)
    })();
    control.finish(result)
}
fn send(control: &Control, transport: &mut impl Write, bytes: &[u8]) -> Result<()> {
    control.open()?;
    let result = (|| {
        frame_length(bytes.len())?;
        control.reserve(bytes.len() + 4)?;
        parse(bytes)?;
        control.open()?;
        transport
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .and_then(|()| transport.write_all(bytes))
            .and_then(|()| transport.flush())
            .map_err(|_| anyhow::anyhow!("TOOL_WORKER_CHANNEL_LOST"))
    })();
    control.finish(result)
}

fn frame_length(length: usize) -> Result<()> {
    ensure!(
        length > 0 && length <= FRAME_BYTES,
        "TOOL_WORKER_FRAME_LIMIT"
    );
    Ok(())
}

pub(super) fn parse(bytes: &[u8]) -> Result<Value> {
    let value = crate::config::policy::strict_json_limit(bytes, FRAME_BYTES)
        .map_err(|_| anyhow::anyhow!("TOOL_WORKER_INVALID_JSON"))?;
    value.as_object().context("TOOL_WORKER_OBJECT_REQUIRED")?;
    Ok(value)
}

/// Bound the encoded wire representation, including JSON escaping, before any
/// transport write. Reuse the incoming tree limits before recursive encoding.
pub(super) fn encode(value: &Value) -> Result<Vec<u8>> {
    crate::config::policy::validate_json_shape(value)
        .map_err(|_| anyhow::anyhow!("TOOL_WORKER_INVALID_JSON"))?;
    value.as_object().context("TOOL_WORKER_OBJECT_REQUIRED")?;
    struct Output(Vec<u8>);
    impl Write for Output {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > FRAME_BYTES - self.0.len() {
                return Err(std::io::Error::other("TOOL_WORKER_FRAME_LIMIT"));
            }
            let required = self.0.len() + bytes.len();
            if required > self.0.capacity() {
                let capacity = required
                    .max(self.0.capacity().saturating_mul(2))
                    .min(FRAME_BYTES);
                self.0
                    .try_reserve_exact(capacity - self.0.len())
                    .map_err(std::io::Error::other)?;
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Output(Vec::new());
    serde_json::to_writer(&mut output, value)
        .map_err(|_| anyhow::anyhow!("TOOL_WORKER_FRAME_LIMIT"))?;
    Ok(output.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor};

    #[test]
    fn encoding_counts_escaped_bytes_and_shares_tree_limits() {
        let exact = serde_json::json!({"x":"a".repeat(FRAME_BYTES - 8)});
        let bytes = encode(&exact).unwrap();
        assert_eq!(bytes.len(), FRAME_BYTES);
        assert_eq!(parse(&bytes).unwrap(), exact);
        let escaped = serde_json::json!({"x":"\0".repeat(FRAME_BYTES / 6)});
        assert!(
            encode(&escaped).is_err(),
            "escaping exceeds the encoded frame limit"
        );
        let mut deep = Value::Null;
        for _ in 0..34 {
            deep = serde_json::json!({"x":deep});
        }
        assert!(encode(&deep).is_err());
        assert!(encode(&serde_json::json!({"x": vec![Value::Null; 10_000]})).is_err());
    }

    #[derive(Default)]
    struct Pipe {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
        fail_flush: bool,
    }
    impl Read for Pipe {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            // Real pipes can return fewer bytes than requested, including within
            // the prefix and within a multibyte UTF-8 character.
            let count = bytes.len().min(1);
            self.input.read(&mut bytes[..count])
        }
    }
    impl Write for Pipe {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let count = bytes.len().min(2);
            self.output.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.fail_flush {
                Err(io::Error::other("synthetic private transport detail"))
            } else {
                Ok(())
            }
        }
    }
    fn frame(bytes: &[u8]) -> Vec<u8> {
        let mut result = (bytes.len() as u32).to_be_bytes().to_vec();
        result.extend(bytes);
        result
    }
    fn channel(bytes: Vec<u8>) -> ToolWorkerChannel<Pipe> {
        ToolWorkerChannel::new(Pipe {
            input: Cursor::new(bytes),
            ..Pipe::default()
        })
    }

    #[test]
    fn fragmented_duplex_frames_use_big_endian_and_one_budget() {
        let bytes = "{\"message\":\"héllo\"}".as_bytes();
        let mut wire = channel(frame(bytes));
        assert_eq!(wire.receive().unwrap()["message"], "héllo");
        wire.send(bytes).unwrap();
        assert_eq!(wire.transport.output, frame(bytes));
        assert_eq!(
            wire.control.remaining.load(Ordering::SeqCst),
            SESSION_BYTES - 2 * (bytes.len() + 4)
        );
    }

    #[test]
    fn invalid_or_truncated_frames_permanently_close_channel() {
        for bytes in [
            vec![],
            vec![0, 0],
            0u32.to_be_bytes().to_vec(),
            ((FRAME_BYTES + 1) as u32).to_be_bytes().to_vec(),
            [2u32.to_be_bytes().as_slice(), b"{"].concat(),
            frame(b"{\"x\":1,\"x\":2}"),
            frame(b"{\"x\":NaN}"),
            frame(b"{\"x\":1e999}"),
            frame(b"[]"),
            frame(b"{}{}"),
            frame(&[b'{', b'"', 0xff, b'"', b':', b'0', b'}']),
        ] {
            let mut wire = channel(bytes);
            assert!(wire.receive().is_err());
            let before = wire.transport.input.position();
            assert!(wire.receive().unwrap_err().to_string().contains("CLOSED"));
            assert_eq!(wire.transport.input.position(), before);
            assert!(wire.send(b"{}").is_err());
            assert!(wire.transport.output.is_empty());
        }
    }

    #[test]
    fn combined_budget_is_enforced_before_body_read_or_write() {
        let mut wire = channel(frame(b"{}"));
        wire.control.remaining.store(11, Ordering::SeqCst);
        wire.send(b"{}").unwrap();
        assert!(wire
            .receive()
            .unwrap_err()
            .to_string()
            .contains("CONTROL_LIMIT"));
        assert_eq!(wire.transport.input.position(), 4);
        assert_eq!(wire.transport.output.len(), 6);
        let mut wire = channel(vec![]);
        wire.control.remaining.store(5, Ordering::SeqCst);
        assert!(wire.send(b"{}").is_err());
        assert!(wire.transport.output.is_empty());
    }

    #[test]
    fn bad_outgoing_json_and_transport_failures_are_terminal() {
        let mut wire = channel(vec![]);
        assert!(wire.send(b"{\"secret\":NaN}").is_err());
        assert!(wire.transport.output.is_empty());
        assert!(wire.send(b"{}").is_err());
        let mut wire = channel(vec![]);
        wire.transport.fail_flush = true;
        assert_eq!(
            wire.send(b"{}").unwrap_err().to_string(),
            "TOOL_WORKER_CHANNEL_LOST"
        );
        assert!(wire.send(b"{}").is_err());
        assert_eq!(wire.transport.output, frame(b"{}"));
    }

    #[test]
    fn exact_frame_limit_is_accepted_but_nested_excess_is_not() {
        let mut bytes = b"{\"padding\":\"".to_vec();
        bytes.resize(FRAME_BYTES - 2, b'x');
        bytes.extend_from_slice(b"\"}");
        let mut wire = ToolWorkerChannel::new(Cursor::new(frame(&bytes)));
        assert!(wire.receive().is_ok());
        bytes.push(b' ');
        let mut wire = ToolWorkerChannel::new(Cursor::new(frame(&bytes)));
        assert!(wire.receive().is_err());
        assert_eq!(wire.transport.position(), 4);
        let nested = format!("{}0{}", "{\"x\":".repeat(40), "}".repeat(40));
        let mut wire = channel(frame(nested.as_bytes()));
        assert_eq!(
            wire.receive().unwrap_err().to_string(),
            "TOOL_WORKER_INVALID_JSON"
        );
    }
}
