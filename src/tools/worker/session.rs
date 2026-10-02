//! Worker-side single-operation sequencing. Reuses outer envelope validation;
//! backend initialization, typed payload admission and process exit are external.
use super::{ToolWorkerExchange, ToolWorkerOperation, ToolWorkerOutcome};
use anyhow::{ensure, Result};
use serde_json::Value;

/// One worker operation. This state must not be reused to serve another context.
/// No result confers authority; the supervisor must independently admit it.
pub struct ToolWorkerSession {
    exchange: ToolWorkerExchange,
}

impl ToolWorkerSession {
    /// Validate the one initial request before backend initialization. Capability
    /// IDs and payload contents are still untrusted, not authenticated handles.
    pub fn new(request: &[u8]) -> Result<Self> {
        Ok(Self {
            exchange: ToolWorkerExchange::new(request)?,
        })
    }

    pub fn operation(&self) -> ToolWorkerOperation {
        self.exchange.operation()
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
            let mut worker = ToolWorkerSession::new(REQUEST).unwrap();
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
        ] {
            let mut worker = ToolWorkerSession::new(REQUEST).unwrap();
            assert!(worker.finish(outcome).is_err());
            assert!(worker.finish(success()).is_err());
        }
    }

    #[test]
    fn cancellation_rejects_racing_success_and_poisoned_or_duplicate_input() {
        let cancel = ToolWorkerExchange::new(REQUEST).unwrap().cancel().unwrap();
        let mut worker = ToolWorkerSession::new(REQUEST).unwrap();
        worker.accept_cancel(&cancel).unwrap();
        assert!(worker.finish(success()).is_err());
        let mut worker = ToolWorkerSession::new(REQUEST).unwrap();
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
            let mut worker = ToolWorkerSession::new(REQUEST).unwrap();
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
            let mut worker = ToolWorkerSession::new(REQUEST).unwrap();
            assert!(worker.accept_cancel(invalid).is_err());
            assert!(worker.finish(success()).is_err());
        }
        let mut worker = ToolWorkerSession::new(REQUEST).unwrap();
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
                    let mut session = ToolWorkerSession::new(&request)?;
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
