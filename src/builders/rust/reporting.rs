//! Native nextest configuration stays outside the source and packaged archives.
use anyhow::{Context, Result};
use std::{fs, path::Path};

pub(super) fn configuration(root: &Path, report: &str) -> Result<String> {
    let path = root.join(".config/nextest.toml");
    let mut config: toml::Value = if path.exists() {
        toml::from_str(&fs::read_to_string(path)?)?
    } else {
        toml::Value::Table(Default::default())
    };
    let mut table = config
        .as_table_mut()
        .context("invalid nextest configuration")?;
    table
        .entry("store")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .context("invalid nextest store")?
        .entry("dir")
        .or_insert_with(|| ".oyzu-build/target/nextest".into());
    for part in ["profile", "default", "junit"] {
        table = table
            .entry(part)
            .or_insert_with(|| toml::Value::Table(Default::default()))
            .as_table_mut()
            .context("invalid nextest profile")?;
    }
    table.insert("path".into(), report.into());
    table
        .entry("report-skipped")
        .or_insert_with(|| "ignored".into());
    Ok(toml::to_string(&config)?)
}
