//! Runtime boundary for policy distribution. Tests supply clocks and faulting stores.
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{
    io::Read,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
pub(crate) enum Refresh {
    Snapshot(String),
    TransportFailure,
    Denied,
}
pub(crate) trait Runtime: Send + Sync {
    fn now(&self) -> Result<i64>;
    fn state(&self, id: &str) -> Result<Option<Vec<u8>>>;
    fn save(&self, id: &str, value: &[u8]) -> Result<()>;
    fn refresh(&self, endpoint: &str, request: &Value) -> Result<Refresh>;
}
pub(crate) struct Native {
    wall: i64,
    monotonic: Instant,
}
impl Native {
    pub fn new() -> Result<Self> {
        Ok(Self {
            wall: wall()?,
            monotonic: Instant::now(),
        })
    }
}
fn wall() -> Result<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .try_into()?)
}
fn entry(id: &str) -> Result<keyring::Entry> {
    keyring::Entry::new("oyzu-policy-v1", id)
        .map_err(|_| anyhow::anyhow!("POLICY_UNAVAILABLE: OS integrity store is unavailable"))
}
impl Runtime for Native {
    fn now(&self) -> Result<i64> {
        let wall = wall()?;
        let expected = self
            .wall
            .saturating_add(self.monotonic.elapsed().as_secs() as i64);
        if wall < expected.saturating_sub(1) {
            bail!("POLICY_CLOCK_UNCERTAIN: clock moved behind monotonic observation");
        }
        Ok(wall)
    }
    fn state(&self, id: &str) -> Result<Option<Vec<u8>>> {
        match entry(id)?.get_secret() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => bail!("POLICY_UNAVAILABLE: cannot read OS integrity state"),
        }
    }
    fn save(&self, id: &str, value: &[u8]) -> Result<()> {
        entry(id)?
            .set_secret(value)
            .map_err(|_| anyhow::anyhow!("POLICY_UNAVAILABLE: cannot persist OS integrity state"))
    }
    fn refresh(&self, endpoint: &str, request: &Value) -> Result<Refresh> {
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .build()?;
        let response = match client.post(endpoint).json(request).send() {
            Ok(response) if response.status().is_success() => response,
            Ok(response) if response.status().is_server_error() => {
                return Ok(Refresh::TransportFailure)
            }
            Ok(_) => return Ok(Refresh::Denied),
            Err(_) => return Ok(Refresh::TransportFailure),
        };
        let mut bytes = Vec::new();
        if response
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .is_err()
        {
            // An interrupted response supplies no policy. The agent may still
            // authorize against an independently verified offline snapshot.
            return Ok(Refresh::TransportFailure);
        }
        let value = super::policy::strict_json_limit(&bytes, 2 * 1024 * 1024)?;
        Ok(Refresh::Snapshot(
            value["configurationSnapshot"]
                .as_str()
                .context("POLICY_INVALID: missing configuration snapshot")?
                .into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener, thread};

    fn response(status: u16, body: &str, announced_length: usize) -> (Result<Refresh>, Value) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!(
            "http://{}/v1/build-contexts:resolve",
            listener.local_addr().unwrap()
        );
        let reply = format!(
            "HTTP/1.1 {status} Test\r\nContent-Length: {announced_length}\r\nConnection: close\r\n\r\n{body}"
        );
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut header = Vec::new();
            let mut byte = [0];
            while !header.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).unwrap();
                header.push(byte[0]);
                assert!(header.len() < 8192);
            }
            let header = String::from_utf8(header).unwrap();
            assert!(header.starts_with("POST /v1/build-contexts:resolve HTTP/1.1\r\n"));
            let length: usize = header
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().unwrap())
                })
                .unwrap();
            let mut body = vec![0; length];
            socket.read_exact(&mut body).unwrap();
            socket.write_all(reply.as_bytes()).unwrap();
            serde_json::from_slice(&body).unwrap()
        });
        // HTTP is confined to the transport fixture. Production enrollment
        // validation requires HTTPS before constructing a refresh endpoint.
        let request = serde_json::json!({"purpose":"configuration","configurationContext":{"subjectId":null}});
        let result = Native::new().unwrap().refresh(&endpoint, &request);
        let observed = server.join().unwrap();
        assert_eq!(observed, request);
        (result, observed)
    }

    #[test]
    fn refresh_transport_distinguishes_denial_interruption_and_invalid_policy() {
        let body = r#"{"configurationSnapshot":"test-envelope"}"#;
        assert!(
            matches!(response(200, body, body.len()).0.unwrap(), Refresh::Snapshot(value) if value == "test-envelope")
        );
        assert!(matches!(
            response(403, "private denial", 14).0.unwrap(),
            Refresh::Denied
        ));
        assert!(matches!(
            response(503, "", 0).0.unwrap(),
            Refresh::TransportFailure
        ));
        assert!(matches!(
            response(200, body, body.len() + 20).0.unwrap(),
            Refresh::TransportFailure
        ));
        for body in [
            r#"{"configurationSnapshot":"a","configurationSnapshot":"b"}"#,
            r#"{"configurationSnapshot":42}"#,
            "private non-JSON response",
        ] {
            let error = response(200, body, body.len()).0.err().unwrap();
            assert!(!format!("{error:#}").contains(body));
        }
    }

    // Opt in on a provisioned host: ordinary unit tests must not require an
    // unlocked credential store or mutate a developer's native keychain.
    #[test]
    #[ignore = "requires an available native OS credential store"]
    fn native_integrity_store_roundtrip() {
        let unique = tempfile::tempdir().unwrap();
        let id = crate::records::digest(
            "oyzu.policy-store-test.v1",
            &serde_json::json!(unique.path()),
        )
        .unwrap();
        struct Cleanup(keyring::Entry);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete_credential();
            }
        }
        let cleanup = Cleanup(entry(&id).unwrap());
        let first = Native::new().unwrap();
        assert_eq!(first.state(&id).unwrap(), None);
        let original = br#"{"sequence":1,"denied":false}"#;
        first.save(&id, original).unwrap();
        // New runtime and keyring handles must observe persisted bytes.
        let reopened = Native::new().unwrap();
        assert_eq!(reopened.state(&id).unwrap().as_deref(), Some(&original[..]));
        let replacement = br#"{"sequence":2,"denied":true}"#;
        reopened.save(&id, replacement).unwrap();
        assert_eq!(first.state(&id).unwrap().as_deref(), Some(&replacement[..]));
        cleanup.0.delete_credential().unwrap();
        assert_eq!(first.state(&id).unwrap(), None);
    }
}
