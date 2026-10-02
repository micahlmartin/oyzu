//! Worker-side single-operation sequencing. Reuses outer envelope validation;
//! backend initialization, typed payload admission and process exit are external.
use super::{ToolWorkerExchange, ToolWorkerOperation, ToolWorkerOutcome, ToolWorkerRequestContext};
use anyhow::{ensure, Result};
use serde_json::Value;

/// One worker operation. This state must not be reused to serve another context.
/// No result confers authority; the supervisor must independently admit it.
pub struct ToolWorkerSession {
    exchange: ToolWorkerExchange,
}

impl ToolWorkerSession {
    /// Bind every request byte to an independently trusted launch commitment.
    /// The digest must originate from the supervisor's admitted request before
    /// transport, never from this received frame or a project-supplied value.
    /// Use exact frame bytes: parse/reserialize changes intentionally fail binding.
    /// Channel authentication and typed payload/capability admission remain external.
    pub fn from_committed_request(request: &[u8], expected_digest: &str) -> Result<Self> {
        super::super::lock::digest(expected_digest)?;
        let exchange = ToolWorkerExchange::new(request)?;
        ensure!(
            exchange.request_digest() == expected_digest,
            "TOOL_WORKER_REQUEST_COMMITMENT_INVALID"
        );
        Ok(Self { exchange })
    }

    /// Bind the initial envelope to independently trusted supervisor state before
    /// backend initialization. Never derive expected values from the received
    /// request. Exact capabilities are operation-specific, not an ambient allowlist.
    /// Channel authentication and typed payload/handle admission remain external.
    pub fn new(
        request: &[u8],
        expected_operation: ToolWorkerOperation,
        expected: ToolWorkerRequestContext<'_>,
    ) -> Result<Self> {
        let exchange = ToolWorkerExchange::new(request)?;
        let actual = exchange.context();
        ensure!(
            exchange.operation() == expected_operation
                && actual.request_id == expected.request_id
                && actual.context_digest == expected.context_digest
                && actual.backend_release_digest == expected.backend_release_digest
                && actual.target_platform == expected.target_platform
                && actual.capabilities == expected.capabilities,
            "TOOL_WORKER_REQUEST_BINDING_INVALID"
        );
        Ok(Self { exchange })
    }

    pub fn operation(&self) -> ToolWorkerOperation {
        self.exchange.operation()
    }

    /// Borrow the shared validated envelope identity. The dispatcher must still
    /// authenticate the channel and admit it against trusted supervisor state.
    pub fn context(&self) -> ToolWorkerRequestContext<'_> {
        self.exchange.context()
    }

    pub fn untrusted_payload(&self) -> &Value {
        self.exchange.untrusted_payload()
    }

    /// Admit only the exact correlated cancel envelope, at most once. Malformed,
    /// duplicate or foreign input makes the session terminal. This method does
    /// not interrupt backend work; the worker dispatcher must arrange that.
    pub fn accept_cancel(&mut self, bytes: &[u8]) -> Result<()> {
        let result = (|| {
            let expected = self.exchange.cancel()?;
            ensure!(
                super::parse(bytes)? == super::parse(&expected)?,
                "TOOL_WORKER_CANCEL_INVALID"
            );
            Ok(())
        })();
        if result.is_err() {
            self.exchange.abort();
        }
        result
    }

    /// Encode one terminal response, consuming the attempt even on invalid
    /// output. Success after cancellation is rejected. Send failure requires
    /// process teardown, never another response or a fresh context in this worker.
    pub fn finish(&mut self, outcome: ToolWorkerOutcome) -> Result<Vec<u8>> {
        self.exchange.encode_terminal(outcome)
    }

    /// Permanently end this operation on transport loss or dispatcher failure.
    pub fn abort(&mut self) {
        self.exchange.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const REQUEST: &[u8] = include_bytes!("../../../tests/fixtures/tool-worker/request.json");
    fn bound_session(request: &[u8]) -> Result<ToolWorkerSession> {
        // Fixture represents independently selected supervisor state; mutated
        // requests below must not be allowed to redefine this expectation.
        let expected = ToolWorkerExchange::new(REQUEST)?;
        ToolWorkerSession::new(request, expected.operation(), expected.context())
    }

    #[test]
    fn committed_request_binds_payload_and_exact_json_spelling() {
        let expected = ToolWorkerExchange::new(REQUEST).unwrap();
        let commitment = expected.request_digest();
        assert!(ToolWorkerSession::from_committed_request(REQUEST, commitment).is_ok());
        let mut changed: Value = serde_json::from_slice(REQUEST).unwrap();
        changed["payload"] = json!({"substituted": true});
        let changed = serde_json::to_vec(&changed).unwrap();
        // Header-only binding deliberately leaves payload admission external.
        assert!(bound_session(&changed).is_ok());
        assert_eq!(
            ToolWorkerSession::from_committed_request(&changed, commitment)
                .err()
                .unwrap()
                .to_string(),
            "TOOL_WORKER_REQUEST_COMMITMENT_INVALID"
        );
        let reparsed: Value = serde_json::from_slice(REQUEST).unwrap();
        let reserialized = serde_json::to_vec(&reparsed).unwrap();
        assert_ne!(reserialized, REQUEST);
        assert!(ToolWorkerSession::from_committed_request(&reserialized, commitment).is_err());
        assert!(ToolWorkerSession::from_committed_request(REQUEST, "invalid").is_err());
        assert!(ToolWorkerSession::from_committed_request(b"{", commitment).is_err());
    }

    #[test]
    fn native_bootstrap_preserves_bytes_and_checks_preselected_commitment() {
        use super::super::{native_tool_worker_channel, NativeToolWorkerIo, ToolWorkerChannel};
        use std::{
            thread,
            time::{Duration, Instant},
        };
        let expected = ToolWorkerExchange::new(REQUEST).unwrap();
        let commitment = expected.request_digest().to_owned();
        let (parent, child) = native_tool_worker_channel().unwrap();
        let peer = thread::spawn(move || {
            let mut wire = ToolWorkerChannel::new(parent);
            wire.send(REQUEST).unwrap();
            let result = wire.receive().unwrap();
            assert_eq!(result["status"], "ok");
        });
        let mut io = NativeToolWorkerIo::new(child).unwrap();
        io.start_receive().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let frame = loop {
            if let Some(frame) = io.try_receive_frame().unwrap() {
                break frame;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(frame.bytes(), REQUEST);
        assert_eq!(
            frame.untrusted_value(),
            &serde_json::from_slice::<Value>(REQUEST).unwrap()
        );
        assert!(
            io.try_receive().is_err(),
            "a frame can only be collected once"
        );
        let mut worker =
            ToolWorkerSession::from_committed_request(frame.bytes(), &commitment).unwrap();
        io.start_send(worker.finish(success()).unwrap()).unwrap();
        while !io.try_send().unwrap() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(1));
        }
        peer.join().unwrap();
        io.shutdown(deadline).unwrap();
    }

    #[test]
    fn session_requires_exact_independent_supervisor_binding() {
        assert!(bound_session(REQUEST).is_ok());
        for (field, replacement) in [
            ("request_id", json!("22345678-1234-1234-1234-123456789abc")),
            (
                "context_digest",
                json!(format!("sha256:{}", "c".repeat(64))),
            ),
            (
                "backend_release_digest",
                json!(format!("sha256:{}", "c".repeat(64))),
            ),
            ("target_platform", json!("windows/amd64/msvc")),
            ("capabilities", json!([])),
            ("capabilities", json!(["install", "metadata"])),
            ("operation", json!("prepare")),
        ] {
            let mut value: Value = serde_json::from_slice(REQUEST).unwrap();
            value[field] = replacement;
            let bytes = serde_json::to_vec(&value).unwrap();
            assert!(ToolWorkerExchange::new(&bytes).is_ok(), "{field}");
            assert_eq!(
                bound_session(&bytes).err().unwrap().to_string(),
                "TOOL_WORKER_REQUEST_BINDING_INVALID",
                "{field}"
            );
        }
    }
    fn success() -> ToolWorkerOutcome {
        ToolWorkerOutcome::UntrustedResult(json!({}))
    }

    #[test]
    fn worker_terminal_output_obeys_supervisor_validation() {
        for outcome in [
            success(),
            ToolWorkerOutcome::Error {
                code: "TOOL_DENIED".into(),
            },
            ToolWorkerOutcome::Cancelled {
                code: "TOOL_STOPPED".into(),
            },
        ] {
            let mut worker = bound_session(REQUEST).unwrap();
            assert_eq!(worker.operation(), ToolWorkerOperation::Resolve);
            assert!(worker.untrusted_payload().is_object());
            let encoded = worker.finish(outcome).unwrap();
            ToolWorkerExchange::new(REQUEST)
                .unwrap()
                .finish(&encoded)
                .unwrap();
            assert!(worker.finish(success()).is_err());
        }
        for outcome in [
            ToolWorkerOutcome::UntrustedResult(Value::Null),
            ToolWorkerOutcome::Error {
                code: "private details".into(),
            },
            ToolWorkerOutcome::UntrustedResult(json!({"too_large":"x".repeat(8*1024*1024)})),
            ToolWorkerOutcome::UntrustedResult(json!({"escaped":"\0".repeat(2*1024*1024)})),
        ] {
            let mut worker = bound_session(REQUEST).unwrap();
            assert!(worker.finish(outcome).is_err());
            assert!(worker.finish(success()).is_err());
        }
    }

    #[test]
    fn cancellation_rejects_racing_success_and_poisoned_or_duplicate_input() {
        let cancel = ToolWorkerExchange::new(REQUEST).unwrap().cancel().unwrap();
        let mut worker = bound_session(REQUEST).unwrap();
        worker.accept_cancel(&cancel).unwrap();
        assert!(worker.finish(success()).is_err());
        let mut worker = bound_session(REQUEST).unwrap();
        worker.accept_cancel(&cancel).unwrap();
        assert!(worker.accept_cancel(&cancel).is_err());
        assert!(worker
            .finish(ToolWorkerOutcome::Cancelled {
                code: "TOOL_CANCELLED".into()
            })
            .is_err());
        let base: Value = serde_json::from_slice(&cancel).unwrap();
        for (field, value) in [
            ("request_id", json!("foreign")),
            ("context_digest", json!("foreign")),
            ("protocol", json!("other")),
            ("operation", json!("resolve")),
            ("payload", json!({})),
        ] {
            let mut changed = base.clone();
            changed[field] = value;
            let mut worker = bound_session(REQUEST).unwrap();
            assert!(worker
                .accept_cancel(&serde_json::to_vec(&changed).unwrap())
                .is_err());
            assert!(worker.finish(success()).is_err());
        }
        for invalid in [
            REQUEST,
            b"{",
            b"{\"operation\":\"cancel\",\"operation\":\"cancel\"}",
        ] {
            let mut worker = bound_session(REQUEST).unwrap();
            assert!(worker.accept_cancel(invalid).is_err());
            assert!(worker.finish(success()).is_err());
        }
        let mut worker = bound_session(REQUEST).unwrap();
        worker.abort();
        assert!(worker.finish(success()).is_err());
    }

    #[test]
    fn native_channel_correlates_worker_cancellation_and_terminal_response() {
        use super::super::{native_tool_worker_channel, ToolWorkerChannel};
        use std::{sync::mpsc, thread, time::Duration};
        let (done, completion) = mpsc::channel();
        thread::spawn(move || {
            let result = (|| -> Result<()> {
                let (parent, child) = native_tool_worker_channel()?;
                let worker = thread::spawn(move || -> Result<()> {
                    let mut wire = ToolWorkerChannel::new(child);
                    let request = serde_json::to_vec(&wire.receive()?)?;
                    let mut session = bound_session(&request)?;
                    // Dispatcher admission needs the actual envelope identity,
                    // not another parse of an untrusted payload or ambient state.
                    let context = session.context();
                    assert_eq!(context.request_id, "12345678-1234-1234-1234-123456789abc");
                    assert_eq!(context.context_digest, format!("sha256:{}", "a".repeat(64)));
                    assert_eq!(
                        context.backend_release_digest,
                        format!("sha256:{}", "b".repeat(64))
                    );
                    assert_eq!(context.target_platform, "linux/amd64/gnu");
                    assert_eq!(context.capabilities, ["metadata"]);
                    session.accept_cancel(&serde_json::to_vec(&wire.receive()?)?)?;
                    wire.send(&session.finish(ToolWorkerOutcome::Cancelled {
                        code: "TOOL_CANCELLED".into(),
                    })?)?;
                    assert!(session.finish(success()).is_err());
                    Ok(())
                });
                let mut wire = ToolWorkerChannel::new(parent);
                let mut exchange = ToolWorkerExchange::new(REQUEST)?;
                wire.send(REQUEST)?;
                wire.send(&exchange.cancel()?)?;
                let response = serde_json::to_vec(&wire.receive()?)?;
                assert!(
                    matches!(exchange.finish(&response)?, ToolWorkerOutcome::Cancelled { code } if code == "TOOL_CANCELLED")
                );
                worker.join().unwrap()?;
                assert!(wire.receive().is_err());
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
