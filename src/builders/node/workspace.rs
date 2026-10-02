//! Shared Node package facts, plans, scopes and evidence. Native managers own
//! membership observation, dependency resolution, projection and packaging.
pub(super) mod model;
pub(super) mod planning;
pub(super) mod reporting;
mod testing;
pub(super) use testing::{native_plan, report_plan};

use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Member {
    pub name: String,
    pub path: String,
}

pub(super) fn validate(members: &[Member]) -> Result<()> {
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();
    if members.is_empty() || members.len() > 1024 {
        bail!("native workspace requires between 1 and 1024 members");
    }
    for member in members {
        if member.name.is_empty()
            || !member
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"@/._-".contains(&b))
            || !names.insert(&member.name)
            || !crate::snapshot::portable(&member.path)
            || member.path.split('/').any(|part| part == "node_modules")
            || !paths.insert(member.path.to_ascii_lowercase())
        {
            bail!("invalid or colliding native workspace member");
        }
    }
    Ok(())
}

pub(super) fn exclusions<'a>(paths: impl Iterator<Item = &'a str>, path: &str) -> Vec<String> {
    let prefix = if path == "." {
        String::new()
    } else {
        format!("{path}/")
    };
    paths
        .filter(|p| *p != path)
        .filter_map(|p| p.strip_prefix(&prefix).map(str::to_owned))
        .collect()
}
