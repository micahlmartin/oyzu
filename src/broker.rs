//! Scoped acquisition transport. Only the host broker holds upstream credentials.
//! A private mounted spool connects an offline resolver to this worker; no host
//! listening socket or general HTTP CONNECT proxy is exposed to package code.
use anyhow::{bail, Context, Result};
use reqwest::{blocking::Client, redirect::Policy, Url};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub(crate) const RUNTIME: &str = include_str!("broker/runtime/transport.py");

pub struct Source {
    pub id: String,
    base: Url,
    authorization: Option<String>,
}

impl Source {
    pub fn new(id: &str, base: &str, authorization: Option<String>) -> Result<Self> {
        let base = Url::parse(base)?;
        if !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
            || !base.path().ends_with('/')
        {
            bail!("invalid source route");
        }
        if base.scheme() != "https"
            && !(base.scheme() == "http"
                && matches!(base.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")))
        {
            bail!("source route requires HTTPS");
        }
        Ok(Self {
            id: id.into(),
            base,
            authorization,
        })
    }

    fn permits(&self, url: &Url) -> bool {
        let path = url.path().to_ascii_lowercase();
        !["%2f", "%5c", "%2e", "%25"]
            .iter()
            .any(|v| path.contains(v))
            && url.origin() == self.base.origin()
            && url.path().starts_with(self.base.path())
            && url.username().is_empty()
            && url.password().is_none()
    }
}

pub struct Response {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub source_id: String,
}

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
                .timeout(Duration::from_secs(45))
                .build()?,
            sources,
            requests: 0,
            bytes: 0,
        })
    }

    pub fn fetch(&mut self, request: &str) -> Result<Response> {
        let mut url =
            Url::parse(request).map_err(|_| anyhow::anyhow!("invalid acquisition URL"))?;
        url.set_fragment(None);
        for _ in 0..5 {
            self.requests += 1;
            if self.requests > 4096 {
                bail!("acquisition request limit exceeded");
            }
            let source = self
                .sources
                .iter()
                .find(|s| s.permits(&url))
                .context("SOURCE_DENIED: URL is outside approved source routes")?;
            let mut request = self.client.get(url.clone()).header("Accept", "text/html");
            if let Some(authorization) = &source.authorization {
                request = request.header("Authorization", authorization);
            }
            let response = request
                .send()
                .map_err(|_| anyhow::anyhow!("approved source transport failed"))?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get("Location")
                    .context("redirect has no location")?
                    .to_str()
                    .map_err(|_| anyhow::anyhow!("invalid redirect"))?;
                url = url
                    .join(location)
                    .map_err(|_| anyhow::anyhow!("invalid redirect URL"))?;
                // Reauthorize each destination; a source's token never follows it.
                continue;
            }
            let status = response.status().as_u16();
            let content_type = response
                .headers()
                .get("Content-Type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();
            let mut body = Vec::new();
            response
                .take(128 * 1024 * 1024 + 1)
                .read_to_end(&mut body)
                .map_err(|_| anyhow::anyhow!("source response read failed"))?;
            self.bytes += body.len() as u64;
            if body.len() > 128 * 1024 * 1024 || self.bytes > 1024 * 1024 * 1024 {
                bail!("acquisition byte limit exceeded");
            }
            // Error bodies can contain upstream authentication diagnostics. Do not relay them.
            if status >= 400 {
                body = format!("approved source returned HTTP {status}").into_bytes();
            }
            return Ok(Response {
                status,
                content_type,
                body,
                source_id: source.id.clone(),
            });
        }
        bail!("redirect limit exceeded")
    }
}

pub struct Session {
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Session {
    pub fn start(spool: &Path, private: &Path, sources: Vec<Source>) -> Result<Self> {
        let mut fetcher = Fetcher::new(sources)?;
        let spool = spool.to_path_buf();
        let private = private.to_path_buf();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                if let Ok(entries) = fs::read_dir(&spool) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let Some(id) = name.strip_suffix(".request") else {
                            continue;
                        };
                        if id.len() != 32 || !id.bytes().all(|v| v.is_ascii_hexdigit()) {
                            continue;
                        }
                        // Move out of the writable channel before inspecting or reading.
                        let claimed = private.join(&name);
                        if fs::rename(entry.path(), &claimed).is_err() {
                            continue;
                        }
                        let response = process(&claimed, &mut fetcher);
                        let (metadata, body) = match response {
                            Ok(r) => (
                                json!({"status":r.status,"contentType":r.content_type,"sourceId":r.source_id}),
                                r.body,
                            ),
                            Err(_) => (
                                json!({"status":403,"contentType":"text/plain","sourceId":"denied"}),
                                b"acquisition request denied".to_vec(),
                            ),
                        };
                        let _ = deliver(&spool, &private, id, &metadata, &body);
                        let _ = fs::remove_file(claimed);
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }
}

fn process(path: &Path, fetcher: &mut Fetcher) -> Result<Response> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 8192 {
        bail!("invalid acquisition request file");
    }
    let value: Value = serde_json::from_slice(&fs::read(path)?)?;
    let url = value["url"].as_str().context("missing request URL")?;
    fetcher.fetch(url)
}

fn deliver(spool: &Path, private: &Path, id: &str, metadata: &Value, body: &[u8]) -> Result<()> {
    for (suffix, bytes) in [
        ("body", body.to_vec()),
        ("response", serde_json::to_vec(metadata)?),
    ] {
        let pending: PathBuf = private.join(format!("{id}.{suffix}"));
        fs::write(&pending, bytes)?;
        fs::rename(pending, spool.join(format!("{id}.{suffix}")))?;
    }
    Ok(())
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
