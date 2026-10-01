//! Chart identity and contained native dependency traversal.
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
pub(super) struct Chart {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
pub(super) struct Dependency {
    name: String,
    #[serde(default)]
    repository: String,
}

pub(super) fn chart_path(root: &Path) -> Result<&'static str> {
    match (
        root.join("Chart.yaml").is_file(),
        root.join("chart/Chart.yaml").is_file(),
    ) {
        (true, false) => Ok("."),
        (false, true) => Ok("chart"),
        (true, true) => bail!("ambiguous root and chart/ Helm charts; select a target path"),
        _ => bail!("missing Helm Chart.yaml"),
    }
}

pub(super) fn read(path: &Path) -> Result<Chart> {
    let chart: Chart = serde_yaml::from_str(&fs::read_to_string(path.join("Chart.yaml"))?)?;
    if chart.name.is_empty()
        || !chart
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        || matches!(chart.name.as_str(), "." | "..")
    {
        bail!("nonportable Helm chart name");
    }
    Ok(chart)
}

/// Native Helm resolves versions. This traversal constrains acquisition to captured
/// local inputs and orders nested dependency preparation before their consumers.
pub(super) fn local_order(root: &Path, chart: &Path) -> Result<Vec<PathBuf>> {
    fn visit(
        root: &Path,
        chart: &Path,
        active: &mut BTreeSet<PathBuf>,
        done: &mut Vec<PathBuf>,
    ) -> Result<()> {
        let chart = chart
            .canonicalize()
            .context("missing local Helm dependency")?;
        if !chart.starts_with(root) {
            bail!("Helm dependency escapes captured target");
        }
        if done.contains(&chart) {
            return Ok(());
        }
        if !active.insert(chart.clone()) {
            bail!("cyclic local Helm dependency");
        }
        for dependency in read(&chart)?.dependencies {
            let path = if let Some(path) = dependency.repository.strip_prefix("file://") {
                if Path::new(path).is_absolute() || path.contains(['\\', ':']) {
                    bail!("Helm dependency must use a relative local path");
                }
                chart.join(path)
            } else if dependency.repository.is_empty() {
                if dependency.name.contains(['/', '\\', ':']) {
                    bail!("invalid local dependency name");
                }
                chart.join("charts").join(dependency.name)
            } else {
                bail!("Helm remote dependency acquisition needs an approved registry adapter");
            };
            visit(root, &path, active, done)?;
        }
        active.remove(&chart);
        done.push(chart);
        Ok(())
    }
    let root = root.canonicalize()?;
    let mut order = Vec::new();
    visit(&root, chart, &mut BTreeSet::new(), &mut order)?;
    Ok(order)
}
