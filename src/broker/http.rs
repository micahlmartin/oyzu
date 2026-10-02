//! Authorized HTTP acquisition with bounded retries and sanitized failures.
//! Native lock interpretation and integrity verification stay with adapters.
use super::{failure::Failure, Response, Source};
use anyhow::Result;
use reqwest::{blocking::Client, header::HeaderValue, redirect::Policy, Url};
use std::{
    io::Read,
    thread,
    time::{Duration, Instant, SystemTime},
};

pub struct Fetcher {
    client: Client,
    sources: Vec<Source>,
    requests: usize,
    bytes: u64,
}

impl Fetcher {
    pub fn new(sources: Vec<Source>) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .no_proxy()
                .redirect(Policy::none())
                .retry(reqwest::retry::never())
                .timeout(Duration::from_secs(45))
                .build()?,
            sources,
            requests: 0,
            bytes: 0,
        })
    }

    pub fn fetch(&mut self, request: &str) -> Result<Response> {
        let mut url = Url::parse(request).map_err(|_| Failure::InvalidRequest)?;
        url.set_fragment(None);
        let deadline = Instant::now() + Duration::from_secs(45);
        let mut retries = 0;
        let mut redirects = 0;
        loop {
            self.requests += 1;
            if self.requests > 4096 {
                return Err(Failure::RequestLimit.into());
            }
            // Reauthorize every request, including retries and redirects.
            let source = self
                .sources
                .iter()
                .find(|s| s.permits(&url))
                .ok_or(Failure::SourceDenied)?;
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Failure::Timeout.into());
            }
            let mut request = self
                .client
                .get(url.clone())
                .header("Accept", "text/html")
                .timeout(remaining);
            if let Some(authorization) = &source.authorization {
                request = request.header("Authorization", authorization);
            }
            let response = match request.send() {
                Ok(response) => response,
                Err(error) => {
                    let retryable = error.is_timeout()
                        || error.is_connect()
                        || error.is_request()
                        || error.is_body();
                    if retryable && wait_to_retry(&mut retries, None, deadline) {
                        continue;
                    }
                    return Err(if error.is_timeout() {
                        Failure::Timeout
                    } else {
                        Failure::Transport
                    }
                    .into());
                }
            };
            if response.status().is_redirection() {
                redirects += 1;
                if redirects >= 5 {
                    return Err(Failure::RedirectLimit.into());
                }
                let location = response
                    .headers()
                    .get("Location")
                    .ok_or(Failure::InvalidRedirect)?
                    .to_str()
                    .map_err(|_| Failure::InvalidRedirect)?;
                url = url.join(location).map_err(|_| Failure::InvalidRedirect)?;
                continue;
            }
            let status = response.status().as_u16();
            if status >= 400 {
                // Do not read upstream error pages: they can contain secrets,
                // have unbounded bodies or stall after a retryable status.
                let retry_after = response.headers().get("Retry-After").cloned();
                drop(response);
                if matches!(status, 408 | 429 | 502 | 503 | 504)
                    && wait_to_retry(&mut retries, retry_after.as_ref(), deadline)
                {
                    continue;
                }
                return Ok(Response {
                    status,
                    content_type: "text/plain".into(),
                    body: format!("approved source returned HTTP {status}").into_bytes(),
                    source_id: source.id.clone(),
                });
            }
            let content_type = response
                .headers()
                .get("Content-Type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();
            let mut body = Vec::new();
            let result = response.take(128 * 1024 * 1024 + 1).read_to_end(&mut body);
            // Failed and partial attempts still consume the session budget.
            self.bytes += body.len() as u64;
            if body.len() > 128 * 1024 * 1024 || self.bytes > 1024 * 1024 * 1024 {
                return Err(Failure::ByteLimit.into());
            }
            if let Err(error) = result {
                if wait_to_retry(&mut retries, None, deadline) {
                    continue;
                }
                return Err(if error.kind() == std::io::ErrorKind::TimedOut
                    || Instant::now() >= deadline
                {
                    Failure::Timeout
                } else {
                    Failure::Transport
                }
                .into());
            }
            return Ok(Response {
                status,
                content_type,
                body,
                source_id: source.id.clone(),
            });
        }
    }
}

fn wait_to_retry(retries: &mut u32, after: Option<&HeaderValue>, deadline: Instant) -> bool {
    let after = match after.map(HeaderValue::to_str).transpose() {
        Ok(value) => value,
        Err(_) => return false,
    };
    let Some(delay) = retry_delay(
        *retries,
        after,
        deadline.saturating_duration_since(Instant::now()),
        SystemTime::now(),
    ) else {
        return false;
    };
    thread::sleep(delay);
    *retries += 1;
    true
}

fn retry_delay(
    retries: u32,
    after: Option<&str>,
    remaining: Duration,
    now: SystemTime,
) -> Option<Duration> {
    if retries >= 2 {
        return None;
    }
    let backoff = Duration::from_millis(200 * (1 << retries));
    let requested = match after {
        None => Duration::ZERO,
        Some(value) if !value.is_empty() && value.bytes().all(|c| c.is_ascii_digit()) => {
            Duration::from_secs(value.parse().ok()?)
        }
        Some(value) => httpdate::parse_http_date(value)
            .ok()?
            .duration_since(now)
            .unwrap_or_default(),
    };
    let delay = backoff.max(requested);
    (delay < remaining).then_some(delay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_transfer_bytes_consume_the_session_budget_and_limits_are_terminal() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while !request.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\npart",
                )
                .unwrap();
        });
        let mut fetcher =
            Fetcher::new(vec![Source::new("source", &address, None).unwrap()]).unwrap();
        fetcher.bytes = 1024 * 1024 * 1024 - 2;
        let error = fetcher.fetch(&address).err().unwrap();
        assert!(matches!(
            error.downcast_ref::<Failure>(),
            Some(Failure::ByteLimit)
        ));
        assert_eq!(fetcher.requests, 1);
        server.join().unwrap();
        fetcher.requests = 4096;
        assert!(matches!(
            fetcher
                .fetch(&address)
                .err()
                .unwrap()
                .downcast_ref::<Failure>(),
            Some(Failure::RequestLimit)
        ));
    }

    #[test]
    fn retry_after_is_respected_without_exceeding_attempt_or_time_budgets() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let remaining = Duration::from_secs(10);
        assert_eq!(
            retry_delay(0, None, remaining, now),
            Some(Duration::from_millis(200))
        );
        assert_eq!(
            retry_delay(1, Some("0"), remaining, now),
            Some(Duration::from_millis(400))
        );
        assert_eq!(
            retry_delay(0, Some("2"), remaining, now),
            Some(Duration::from_secs(2))
        );
        let date = httpdate::fmt_http_date(now + Duration::from_secs(3));
        assert_eq!(
            retry_delay(0, Some(&date), remaining, now),
            Some(Duration::from_secs(3))
        );
        for header in ["10", "9999999999999999999999999999", "-1", "invalid"] {
            assert!(retry_delay(0, Some(header), remaining, now).is_none());
        }
        assert!(retry_delay(2, None, remaining, now).is_none());
        assert!(retry_delay(0, None, Duration::ZERO, now).is_none());
    }
}
