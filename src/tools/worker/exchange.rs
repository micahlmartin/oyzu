//! Closed envelope and single-operation correlation, independent of transport.
//! Payload values remain untrusted until operation-specific admission exists.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const PROTOCOL: &str = "oyzu.tool-worker/1";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolWorkerOperation {
    Resolve,
    Prepare,
    Environment,
    Executable,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    request_id: String,
    operation: ToolWorkerOperation,
    context_digest: String,
    backend_release_digest: String,
    target_platform: String,
    capabilities: Vec<String>,
    payload: Value,
}

/// Validated envelope identity borrowed from the original request. Syntax and
/// correlation are checked, but these values do not authenticate the caller or
/// authorize a backend, target or capability. Admission must compare them with
/// independently trusted supervisor state before dispatching the payload.
#[derive(Clone, Copy, Debug)]
pub struct ToolWorkerRequestContext<'a> {
    pub request_id: &'a str,
    pub context_digest: &'a str,
    pub backend_release_digest: &'a str,
    pub target_platform: &'a str,
    pub capabilities: &'a [String],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Diagnostic {
    code: String,
}

/// An envelope outcome, not a validated backend result or execution grant.
pub enum ToolWorkerOutcome {
    UntrustedResult(Value),
    Error { code: String },
    Cancelled { code: String },
}

/// Correlates one request, at most one cancel and one terminal response. A
/// malformed or mismatched response permanently ends the exchange. The caller
/// still owns deadlines, transport failure handling and typed payload admission.
pub struct ToolWorkerExchange {
    request: Request,
    cancel_sent: bool,
    terminal: bool,
}

impl ToolWorkerExchange {
    /// Validate the outer request, retaining its payload as untrusted data.
    /// This does not admit capabilities, handles, target support or backend use.
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let request: Request = serde_json::from_value(super::parse(bytes)?)
            .map_err(|_| anyhow::anyhow!("TOOL_WORKER_REQUEST_INVALID"))?;
        ensure!(request.protocol == PROTOCOL, "TOOL_WORKER_PROTOCOL_INVALID");
        ensure!(
            super::super::valid_request_id(&request.request_id),
            "TOOL_WORKER_REQUEST_ID_INVALID"
        );
        super::super::lock::digest(&request.context_digest)?;
        super::super::lock::digest(&request.backend_release_digest)?;
        super::super::lock::platform(&request.target_platform)
            .map_err(|_| anyhow::anyhow!("TOOL_WORKER_PLATFORM_INVALID"))?;
        ensure!(
            request.capabilities.len() <= 256
                && request.capabilities.iter().all(|id| !id.is_empty()
                    && id.len() <= 256
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b)))
                && request
                    .capabilities
                    .windows(2)
                    .all(|pair| pair[0] < pair[1]),
            "TOOL_WORKER_CAPABILITIES_INVALID"
        );
        ensure!(request.payload.is_object(), "TOOL_WORKER_PAYLOAD_INVALID");
        Ok(Self {
            request,
            cancel_sent: false,
            terminal: false,
        })
    }

    pub fn operation(&self) -> ToolWorkerOperation {
        self.request.operation
    }

    /// Borrow identity from the same request used for cancellation and terminal
    /// response correlation. This snapshot remains inspectable after termination;
    /// its availability is not permission to continue an ended operation.
    pub fn context(&self) -> ToolWorkerRequestContext<'_> {
        ToolWorkerRequestContext {
            request_id: &self.request.request_id,
            context_digest: &self.request.context_digest,
            backend_release_digest: &self.request.backend_release_digest,
            target_platform: &self.request.target_platform,
            capabilities: &self.request.capabilities,
        }
    }

    pub fn untrusted_payload(&self) -> &Value {
        &self.request.payload
    }

    /// Produce a correlated cancel envelope at most once. Call before sending;
    /// transport failure requires aborting the operation, not retrying a cancel.
    pub fn cancel(&mut self) -> Result<Vec<u8>> {
        ensure!(
            !self.terminal && !self.cancel_sent,
            "TOOL_WORKER_SEQUENCE_INVALID"
        );
        self.cancel_sent = true;
        Ok(serde_json::to_vec(&serde_json::json!({
            "protocol": PROTOCOL, "request_id": self.request.request_id,
            "context_digest": self.request.context_digest, "operation": "cancel"
        }))?)
    }

    /// Mark transport loss, timeout or a supervisor abort as terminal.
    pub fn abort(&mut self) {
        self.terminal = true;
    }

    /// Worker-side encoding reuses exactly the supervisor's result validation
    /// and terminal transition. Invalid output consumes the response attempt.
    pub(super) fn encode_terminal(&mut self, outcome: ToolWorkerOutcome) -> Result<Vec<u8>> {
        ensure!(!self.terminal, "TOOL_WORKER_SEQUENCE_INVALID");
        let mut value = serde_json::json!({
            "protocol": PROTOCOL, "request_id": self.request.request_id,
            "context_digest": self.request.context_digest
        });
        match outcome {
            ToolWorkerOutcome::UntrustedResult(result) => {
                value["status"] = Value::from("ok");
                value["result"] = result;
            }
            ToolWorkerOutcome::Error { code } => {
                value["status"] = Value::from("error");
                value["diagnostic"] = serde_json::json!({"code":code});
            }
            ToolWorkerOutcome::Cancelled { code } => {
                value["status"] = Value::from("cancelled");
                value["diagnostic"] = serde_json::json!({"code":code});
            }
        }
        let bytes = match super::framing::encode(&value) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.abort();
                return Err(error);
            }
        };
        self.finish(&bytes)?;
        Ok(bytes)
    }

    /// Accept one terminal response with exact request/context correlation.
    /// A successful response racing cancellation is discarded as an error, never
    /// returned for publication. A fresh operation is required after any failure.
    pub fn finish(&mut self, bytes: &[u8]) -> Result<ToolWorkerOutcome> {
        ensure!(!self.terminal, "TOOL_WORKER_SEQUENCE_INVALID");
        self.terminal = true;
        let value = super::parse(bytes)?;
        let object = value.as_object().expect("parser requires object");
        ensure!(
            object.len() == 5
                && object.keys().all(|key| matches!(
                    key.as_str(),
                    "protocol"
                        | "request_id"
                        | "context_digest"
                        | "status"
                        | "result"
                        | "diagnostic"
                )),
            "TOOL_WORKER_RESPONSE_INVALID"
        );
        ensure!(
            value["protocol"] == PROTOCOL
                && value["request_id"] == self.request.request_id
                && value["context_digest"] == self.request.context_digest,
            "TOOL_WORKER_RESPONSE_BINDING_INVALID"
        );
        match value["status"].as_str() {
            Some("ok") => {
                ensure!(!self.cancel_sent, "TOOL_WORKER_CANCELLED_RESULT");
                ensure!(
                    value.get("diagnostic").is_none() && value["result"].is_object(),
                    "TOOL_WORKER_RESPONSE_INVALID"
                );
                Ok(ToolWorkerOutcome::UntrustedResult(value["result"].clone()))
            }
            Some(status @ ("error" | "cancelled")) => {
                ensure!(
                    value.get("result").is_none(),
                    "TOOL_WORKER_RESPONSE_INVALID"
                );
                let diagnostic: Diagnostic = serde_json::from_value(value["diagnostic"].clone())
                    .map_err(|_| anyhow::anyhow!("TOOL_WORKER_DIAGNOSTIC_INVALID"))?;
                ensure!(
                    !diagnostic.code.is_empty()
                        && diagnostic.code.len() <= 64
                        && diagnostic
                            .code
                            .bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'),
                    "TOOL_WORKER_DIAGNOSTIC_INVALID"
                );
                if status == "error" {
                    Ok(ToolWorkerOutcome::Error {
                        code: diagnostic.code,
                    })
                } else {
                    Ok(ToolWorkerOutcome::Cancelled {
                        code: diagnostic.code,
                    })
                }
            }
            _ => anyhow::bail!("TOOL_WORKER_RESPONSE_INVALID"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn request() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/tool-worker/request.json"
        ))
        .unwrap()
    }
    fn response() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/tool-worker/response.json"
        ))
        .unwrap()
    }
    fn exchange() -> ToolWorkerExchange {
        ToolWorkerExchange::new(&serde_json::to_vec(&request()).unwrap()).unwrap()
    }
    fn bytes(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }
    #[test]
    fn one_result_and_cancel_races_are_terminal() {
        let mut state = exchange();
        assert_eq!(state.operation(), ToolWorkerOperation::Resolve);
        assert!(state.untrusted_payload().is_object());
        assert!(matches!(
            state.finish(&bytes(&response())).unwrap(),
            ToolWorkerOutcome::UntrustedResult(_)
        ));
        assert!(state.finish(&bytes(&response())).is_err());
        assert!(state.cancel().is_err());
        let mut state = exchange();
        let cancel: Value = serde_json::from_slice(&state.cancel().unwrap()).unwrap();
        assert_eq!(cancel["request_id"], request()["request_id"]);
        assert_eq!(cancel["operation"], "cancel");
        assert!(state.cancel().is_err());
        assert!(state.finish(&bytes(&response())).is_err());
        let mut state = exchange();
        state.abort();
        assert!(state.finish(&bytes(&response())).is_err());
    }
    #[test]
    fn request_unknowns_and_invalid_bindings_are_rejected() {
        for (field, value) in [
            ("protocol", json!("other")),
            ("operation", json!("install")),
            ("request_id", json!("bad")),
            ("context_digest", json!("bad")),
            ("target_platform", json!("other")),
            ("capabilities", json!(["z", "a"])),
            ("capabilities", json!(["a", "a"])),
            ("payload", json!(null)),
            ("command", json!("sh")),
        ] {
            let mut changed = request();
            changed[field] = value;
            assert!(
                ToolWorkerExchange::new(&bytes(&changed)).is_err(),
                "{field}"
            );
        }
    }
    #[test]
    fn bad_responses_poison_exchange_and_diagnostics_are_closed() {
        for (field, value) in [
            ("protocol", json!("other")),
            ("request_id", json!("other")),
            ("context_digest", json!("other")),
            ("status", json!("progress")),
            ("result", json!(null)),
            ("diagnostic", json!({"code":"FAIL"})),
            ("extra", json!(0)),
        ] {
            let mut changed = response();
            changed[field] = value;
            let mut state = exchange();
            assert!(state.finish(&bytes(&changed)).is_err(), "{field}");
            assert!(state.finish(&bytes(&response())).is_err());
        }
        for status in ["error", "cancelled"] {
            let mut value = response();
            value.as_object_mut().unwrap().remove("result");
            value["status"] = json!(status);
            value["diagnostic"] = json!({"code":"TOOL_DENIED"});
            assert!(exchange().finish(&bytes(&value)).is_ok());
            value["diagnostic"]["message"] = json!("private detail");
            assert!(exchange().finish(&bytes(&value)).is_err());
        }
    }

    #[test]
    fn shared_envelope_shapes_agree_with_runtime() {
        for (source, is_request) in [
            (
                include_str!("../../../tests/fixtures/tool-worker/invalid-requests.json"),
                true,
            ),
            (
                include_str!("../../../tests/fixtures/tool-worker/invalid-responses.json"),
                false,
            ),
        ] {
            for case in serde_json::from_str::<Vec<Value>>(source).unwrap() {
                let mut value = if is_request { request() } else { response() };
                value[case["pointer"].as_str().unwrap().trim_start_matches('/')] =
                    case["value"].clone();
                let rejected = if is_request {
                    ToolWorkerExchange::new(&bytes(&value)).is_err()
                } else {
                    exchange().finish(&bytes(&value)).is_err()
                };
                assert!(rejected, "{}", case["name"]);
            }
        }
    }
}
