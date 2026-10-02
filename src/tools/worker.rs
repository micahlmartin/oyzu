//! Internal worker control framing. The supervisor owns channel authentication,
//! deadlines and process lifecycle; typed operation admission belongs above this
//! layer. This codec neither starts a worker nor authorizes any operation.
use anyhow::{ensure, Context, Result};
use serde_json::Value;
use std::io::{Read, Write};

const FRAME_BYTES: usize = 8 * 1024 * 1024;
const SESSION_BYTES: usize = 32 * 1024 * 1024;

/// Bounded framing for one private duplex worker channel. This is an unstable
/// internal protocol, not a plugin API. Both directions share the control budget,
/// including four-byte length prefixes. Any failure permanently closes the codec.
/// The underlying transport must independently enforce deadlines/cancellation;
/// synchronous Read/Write cannot interrupt a blocked system call.
pub struct ToolWorkerChannel<T> {
    transport: T,
    remaining: usize,
    failed: bool,
}

impl<T: Read + Write> ToolWorkerChannel<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            remaining: SESSION_BYTES,
            failed: false,
        }
    }

    fn charge(&mut self, length: usize) -> Result<()> {
        ensure!(
            length > 0 && length <= FRAME_BYTES,
            "TOOL_WORKER_FRAME_LIMIT"
        );
        let charge = length + 4;
        ensure!(charge <= self.remaining, "TOOL_WORKER_CONTROL_LIMIT");
        self.remaining -= charge;
        Ok(())
    }

    /// Receive exactly one length-prefixed JSON object. A clean EOF is still a
    /// channel failure; the protocol requires an explicit terminal response.
    /// Returned data is untrusted until typed envelope/identity validation.
    pub fn receive(&mut self) -> Result<Value> {
        ensure!(!self.failed, "TOOL_WORKER_CHANNEL_CLOSED");
        let result = self.receive_inner();
        self.failed = result.is_err();
        result
    }

    fn receive_inner(&mut self) -> Result<Value> {
        let mut prefix = [0; 4];
        self.transport
            .read_exact(&mut prefix)
            .map_err(|_| anyhow::anyhow!("TOOL_WORKER_CHANNEL_LOST"))?;
        let length = u32::from_be_bytes(prefix) as usize;
        self.charge(length)?;
        let mut bytes = vec![0; length];
        self.transport
            .read_exact(&mut bytes)
            .map_err(|_| anyhow::anyhow!("TOOL_WORKER_CHANNEL_LOST"))?;
        parse(&bytes)
    }

    /// Validate and transmit one already-serialized JSON object. Malformed input
    /// is rejected before any bytes are written. Flush failure closes the codec,
    /// since a peer may have received some or all of the frame already.
    pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
        ensure!(!self.failed, "TOOL_WORKER_CHANNEL_CLOSED");
        let result = self.send_inner(bytes);
        self.failed = result.is_err();
        result
    }

    fn send_inner(&mut self, bytes: &[u8]) -> Result<()> {
        self.charge(bytes.len())?;
        parse(bytes)?;
        self.transport
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .and_then(|()| self.transport.write_all(bytes))
            .and_then(|()| self.transport.flush())
            .map_err(|_| anyhow::anyhow!("TOOL_WORKER_CHANNEL_LOST"))
    }
}

fn parse(bytes: &[u8]) -> Result<Value> {
    let value = crate::config::policy::strict_json_limit(bytes, FRAME_BYTES)
        .map_err(|_| anyhow::anyhow!("TOOL_WORKER_INVALID_JSON"))?;
    value.as_object().context("TOOL_WORKER_OBJECT_REQUIRED")?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor};

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
        assert_eq!(wire.remaining, SESSION_BYTES - 2 * (bytes.len() + 4));
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
        wire.remaining = 11;
        wire.send(b"{}").unwrap();
        assert!(wire
            .receive()
            .unwrap_err()
            .to_string()
            .contains("CONTROL_LIMIT"));
        assert_eq!(wire.transport.input.position(), 4);
        assert_eq!(wire.transport.output.len(), 6);
        let mut wire = channel(vec![]);
        wire.remaining = 5;
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
