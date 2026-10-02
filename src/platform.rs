//! Canonical OS/architecture identities shared by planning and execution.
//! Native adapters decide which targets they can produce; this type never
//! infers emulation, ABI compatibility or platform-independent artifacts.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Wire")]
pub(crate) struct Platform {
    os: String,
    arch: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    os: String,
    arch: String,
}

impl TryFrom<Wire> for Platform {
    type Error = anyhow::Error;
    fn try_from(value: Wire) -> Result<Self> {
        format!("{}/{}", value.os, value.arch).parse()
    }
}

impl FromStr for Platform {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self> {
        let Some((os, arch)) = value.split_once('/') else {
            bail!("platform requires canonical os/architecture");
        };
        if [os, arch].iter().any(|part| {
            part.is_empty()
                || part.len() > 32
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        }) {
            bail!("platform requires canonical os/architecture; aliases and variant suffixes are unsupported");
        }
        Ok(Self {
            os: os.into(),
            arch: arch.into(),
        })
    }
}

impl Platform {
    pub fn requested(value: Option<&str>, execution: &Self) -> Result<Self> {
        value
            .map(str::parse)
            .unwrap_or_else(|| Ok(execution.clone()))
    }
    pub fn os(&self) -> &str {
        &self.os
    }
    pub fn arch(&self) -> &str {
        &self.arch
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.os, self.arch)
    }
}
