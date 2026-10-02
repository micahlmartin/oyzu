//! Exercise production Fetcher limits with actual HTTP responses.
use anyhow::{ensure, Context, Result};
use oyzu::broker::{Fetcher, Source};
use serde_json::json;

fn main() -> Result<()> {
    let base = std::env::args()
        .nth(1)
        .context("controlled source URL required")?;
    let new_fetcher = || Fetcher::new(vec![Source::new("limit-fixture", &base, None)?]);
    let mut results = Vec::new();
    let mut requests = new_fetcher()?;
    for _ in 0..4096 {
        ensure!(requests.fetch(&format!("{base}empty"))?.body.is_empty());
    }
    let error = requests
        .fetch(&format!("{base}empty"))
        .err()
        .context("request limit not enforced")?;
    ensure!(error.to_string().contains("request limit exceeded"));
    results.push(json!({"case":"production-request-limit","allowed":4096,"denied":4097}));

    const LIMIT: usize = 128 * 1024 * 1024;
    let mut response = new_fetcher()?;
    ensure!(response.fetch(&format!("{base}bytes/{LIMIT}"))?.body.len() == LIMIT);
    let error = response
        .fetch(&format!("{base}bytes/{}", LIMIT + 1))
        .err()
        .context("response limit not enforced")?;
    ensure!(error.to_string().contains("byte limit exceeded"));
    results.push(json!({"case":"production-response-byte-limit","allowed_bytes":LIMIT,"denied_bytes":LIMIT+1}));

    let mut session = new_fetcher()?;
    for _ in 0..8 {
        ensure!(session.fetch(&format!("{base}bytes/{LIMIT}"))?.body.len() == LIMIT);
    }
    let error = session
        .fetch(&format!("{base}bytes/1"))
        .err()
        .context("session byte limit not enforced")?;
    ensure!(error.to_string().contains("byte limit exceeded"));
    results.push(json!({"case":"production-session-byte-limit","allowed_bytes":8*LIMIT,"denied_total_bytes":8*LIMIT+1}));

    let mut redirects = new_fetcher()?;
    let error = redirects
        .fetch(&format!("{base}redirect"))
        .err()
        .context("redirect limit not enforced")?;
    ensure!(error.to_string().contains("redirect limit exceeded"));
    results.push(json!({"case":"production-redirect-limit","maximum_attempts":5}));
    println!("{}", serde_json::to_string(&results)?);
    Ok(())
}
