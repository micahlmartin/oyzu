//! Standalone tool-route binding. Administrative configuration selects opaque connector
//! IDs; explicitly supplied host TOML resolves endpoints and credential references.
//! Only this host adapter owns credentials. Managed agent bindings are separate.
use crate::{broker, config::resolve::EffectiveConfig};
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const PUBLIC: &str = "https://nodejs.org/dist/";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bindings {
    format: u32,
    connectors: BTreeMap<String, Binding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    base_url: String,
    authorization_env: Option<String>,
}

pub(super) struct NodeAcquisition {
    connector: Option<String>,
    bindings: Option<PathBuf>,
    transport: Option<(String, broker::Fetcher)>,
}

impl NodeAcquisition {
    pub(super) fn new(config: &EffectiveConfig, bindings: Option<&Path>) -> Result<Self> {
        ensure!(
            config
                .get("tools.catalogs")
                .is_none_or(|value| value == &serde_json::json!(["public"])),
            "configured tool catalog is not available; public fallback is disabled"
        );
        let routes: Vec<_> = config
            .get("registries.routes")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter(|route| route["protocol"] == "tools")
            .collect();
        let exact: Vec<_> = routes
            .iter()
            .filter(|route| matches!(route["scope"].as_str(), Some("node" | "core:node")))
            .collect();
        ensure!(exact.len() <= 1, "ambiguous Node connector routes");
        let route = exact
            .first()
            .copied()
            .copied()
            .or_else(|| routes.iter().find(|route| route["scope"] == "*").copied());
        Ok(Self {
            connector: route.map(|route| route["connectorId"].as_str().unwrap().to_owned()),
            bindings: bindings.map(Path::to_path_buf),
            transport: None,
        })
    }

    /// Map admitted upstream resource keys to the configured route. An error is
    /// terminal for this source; there is never an alternate public fetch.
    pub(super) fn fetch(&mut self, upstream: &str) -> Result<broker::Response> {
        let url = reqwest::Url::parse(upstream)?;
        ensure!(
            url.scheme() == "https"
                && url.host_str() == Some("nodejs.org")
                && url.port_or_known_default() == Some(443)
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "TOOL_ROUTE_UNSUPPORTED: unrecognized Node resource"
        );
        let resource = url
            .path()
            .strip_prefix("/dist/")
            .context("TOOL_ROUTE_UNSUPPORTED: unrecognized Node resource path")?;
        ensure!(
            !resource.is_empty() && !resource.contains('%') && !resource.contains('\\'),
            "TOOL_ROUTE_UNSUPPORTED: unsupported Node resource encoding"
        );
        if self.transport.is_none() {
            let (base, authorization) = if let Some(connector) = &self.connector {
                let path = self.bindings.as_ref().context("configured Node route requires --connector-bindings; public fallback is disabled")?;
                let bytes = super::read_record(path, 1024 * 1024)?;
                let bindings: Bindings = toml::from_str(std::str::from_utf8(&bytes)?)
                    .context("invalid host connector bindings")?;
                ensure!(
                    bindings.format == 1,
                    "unsupported host connector binding format"
                );
                let binding = bindings.connectors.get(connector).context(
                    "configured Node connector has no host binding; public fallback is disabled",
                )?;
                let authorization = binding
                    .authorization_env
                    .as_ref()
                    .map(|name| {
                        std::env::var(name)
                            .context("connector authorization environment variable is unavailable")
                    })
                    .transpose()?;
                (binding.base_url.clone(), authorization)
            } else {
                (PUBLIC.into(), None)
            };
            let source = broker::Source::new("node-releases", &base, authorization)?;
            self.transport = Some((base, broker::Fetcher::new(vec![source])?));
        }
        let (base, fetcher) = self.transport.as_mut().unwrap();
        let target = reqwest::Url::parse(base)?.join(resource)?;
        fetcher.fetch(target.as_str())
    }
}
