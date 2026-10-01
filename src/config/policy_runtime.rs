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
        response.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        let value = super::policy::strict_json_limit(&bytes, 2 * 1024 * 1024)?;
        Ok(Refresh::Snapshot(
            value["configurationSnapshot"]
                .as_str()
                .context("POLICY_INVALID: missing configuration snapshot")?
                .into(),
        ))
    }
}
