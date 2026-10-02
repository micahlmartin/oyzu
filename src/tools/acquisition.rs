//! Standalone tool-route binding. Administrative configuration selects opaque connector
//! IDs; explicitly supplied host TOML resolves endpoints and credential references.
//! Only this host adapter owns credentials. Managed agent bindings are separate.
use super::development_backend::Tool;
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

pub(super) struct Acquisition {
    tool: Tool,
    connector: Option<String>,
    bindings: Option<PathBuf>,
    transport: Option<(String, broker::Fetcher)>,
}

impl Acquisition {
    pub(super) fn new(
        config: &EffectiveConfig,
        bindings: Option<&Path>,
        tool: Tool,
    ) -> Result<Self> {
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
            .filter(|route| route["scope"] == tool.name() || route["scope"] == tool.id())
            .collect();
        ensure!(exact.len() <= 1, "ambiguous tool connector routes");
        let route = exact
            .first()
            .copied()
            .copied()
            .or_else(|| routes.iter().find(|route| route["scope"] == "*").copied());
        ensure!(
            tool != Tool::Rust || route.is_none(),
            "Rust installer routing is not connected; direct fallback is disabled"
        );
        Ok(Self {
            tool,
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
                && url.port_or_known_default() == Some(443)
                && url.username().is_empty()
                && url.password().is_none()
                && url.fragment().is_none(),
            "TOOL_ROUTE_UNSUPPORTED: unrecognized tool resource"
        );
        let resource = match self.tool {
            Tool::Rust
                if matches!(
                    url.host_str(),
                    Some("sh.rustup.rs" | "win.rustup.rs" | "static.rust-lang.org")
                ) =>
            {
                Some("rustup-bootstrap")
            }
            Tool::Node if url.host_str() == Some("nodejs.org") && url.query().is_none() => {
                url.path().strip_prefix("/dist/")
            }
            Tool::Go if upstream == "https://go.dev/dl/?mode=json&include=all" => {
                Some("index.json")
            }
            Tool::Go if url.host_str() == Some("dl.google.com") && url.query().is_none() => {
                url.path().strip_prefix("/go/")
            }
            _ => None,
        }
        .context("TOOL_ROUTE_UNSUPPORTED: unrecognized tool resource path")?;
        ensure!(
            !resource.is_empty() && !resource.contains('%') && !resource.contains('\\'),
            "TOOL_ROUTE_UNSUPPORTED: unsupported tool resource encoding"
        );
        if self.transport.is_none() {
            let (base, authorization) = if let Some(connector) = &self.connector {
                let path = self.bindings.as_ref().context("configured tool route requires --connector-bindings; public fallback is disabled")?;
                let bytes = super::read_record(path, 1024 * 1024)?;
                let bindings: Bindings = toml::from_str(std::str::from_utf8(&bytes)?)
                    .context("invalid host connector bindings")?;
                ensure!(
                    bindings.format == 1,
                    "unsupported host connector binding format"
                );
                let binding = bindings.connectors.get(connector).context(
                    "configured tool connector has no host binding; public fallback is disabled",
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
                (
                    match self.tool {
                        Tool::Node => PUBLIC,
                        Tool::Go => "https://dl.google.com/go/",
                        Tool::Rust => "https://sh.rustup.rs/",
                    }
                    .into(),
                    None,
                )
            };
            let source = broker::Source::new(self.tool.source(), &base, authorization)?;
            let mut sources = vec![source];
            if self.connector.is_none() && self.tool == Tool::Go {
                sources.push(broker::Source::new(
                    "go-catalog",
                    "https://go.dev/dl/",
                    None,
                )?);
            }
            if self.tool == Tool::Rust {
                sources.push(broker::Source::new(
                    "rustup-windows",
                    "https://win.rustup.rs/",
                    None,
                )?);
                sources.push(broker::Source::new(
                    "rust-distribution",
                    "https://static.rust-lang.org/",
                    None,
                )?);
            }
            self.transport = Some((base, broker::Fetcher::new(sources)?));
        }
        let (base, fetcher) = self.transport.as_mut().unwrap();
        let target = if self.connector.is_some() {
            reqwest::Url::parse(base)?.join(resource)?
        } else {
            url
        };
        fetcher.fetch(target.as_str())
    }
}
