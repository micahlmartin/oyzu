//! Propagate artifact target requirements through materialization edges only.
//! No execution image, host default or platform independence is inferred here.
use crate::{model::Workspace, platform::Platform};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn requirements(workspace: &Workspace) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut required = BTreeMap::new();
    let mut constrained = BTreeSet::new();
    for id in workspace.targets.keys() {
        let mut values = BTreeSet::new();
        if let Some(config) = workspace.declarations.targets.get(id) {
            for value in config
                .platform
                .iter()
                .chain(config.matrix.get("platform").into_iter().flatten())
            {
                let parsed: Platform = value
                    .parse()
                    .with_context(|| format!("{id}: invalid artifact platform"))?;
                values.insert(parsed.to_string());
            }
        }
        if !values.is_empty() {
            constrained.insert(id.clone());
        }
        required.insert(id.clone(), values);
    }
    loop {
        let before: usize = required.values().map(BTreeSet::len).sum();
        if before > super::MAX_INSTANCES {
            bail!(
                "platform expansion exceeds {} target instances",
                super::MAX_INSTANCES
            );
        }
        for (consumer, config) in &workspace.declarations.targets {
            let platforms = required[consumer].clone();
            for input in &config.materialize {
                let producer = required
                    .get_mut(&input.from)
                    .context("missing materialization producer")?;
                if constrained.contains(&input.from) {
                    if !platforms.is_subset(producer) {
                        bail!("{consumer}: materialization platform requirement conflicts with explicit producer {}; consumer requires {platforms:?}, producer allows {producer:?}", input.from);
                    }
                } else {
                    producer.extend(platforms.iter().cloned());
                }
            }
        }
        if required.values().map(BTreeSet::len).sum::<usize>() == before {
            return Ok(required);
        }
    }
}
