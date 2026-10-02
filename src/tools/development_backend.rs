//! Closed set of development adapters admitted through the maintained mise fork.
//! Version/catalog/layout facts remain upstream; orchestration uses these IDs.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ffi::OsString, path::Path};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum Tool {
    Node,
    Go,
}

impl Tool {
    /// Add backend-owned development environment from its verified bin location.
    /// Go binds its runtime root; a configured toolchain mode overrides the default.
    pub(super) fn apply_environment(
        self,
        bin: &Path,
        environment: &mut BTreeMap<String, OsString>,
    ) -> Result<()> {
        if self == Self::Go {
            environment.insert(
                "GOROOT".into(),
                bin.parent()
                    .context("Go payload root unavailable")?
                    .as_os_str()
                    .to_owned(),
            );
            environment
                .entry("GOTOOLCHAIN".into())
                .or_insert_with(|| "local".into());
        }
        Ok(())
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Go => "go",
        }
    }
    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Node => "core:node",
            Self::Go => "core:go",
        }
    }
    pub(super) fn source(self) -> &'static str {
        match self {
            Self::Node => "node-releases",
            Self::Go => "go-releases",
        }
    }
}
