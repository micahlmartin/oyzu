use anyhow::{bail, Context, Result};
use std::{collections::BTreeSet, fs, path::Path};

pub(super) struct Jar {
    pub target: String,
    pub path: String,
}

pub(super) struct Metadata {
    pub version: String,
    pub manager_version: String,
    pub jars: Vec<Jar>,
}

pub(super) fn read(path: &Path) -> Result<Metadata> {
    let text = fs::read_to_string(path)?;
    let document = roxmltree::Document::parse(&text)?;
    let root = document.root_element();
    let version = root
        .attribute("version")
        .context("missing Ant version")?
        .to_owned();
    if version.is_empty()
        || !version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'))
    {
        bail!("Ant version must be a portable release or snapshot identifier");
    }
    let manager_version = root
        .attribute("antVersion")
        .context("missing Ant tool version")?
        .split_whitespace()
        .skip_while(|part| *part != "version")
        .nth(1)
        .context("invalid Ant tool version")?
        .to_owned();
    let mut jars = Vec::new();
    let mut paths = BTreeSet::new();
    for target in root.children().filter(|n| n.has_tag_name("target")) {
        for jar in target.children().filter(|n| n.has_tag_name("jar")) {
            let path = jar
                .attribute("path")
                .context("missing Ant JAR path")?
                .strip_prefix("/workspace/")
                .context("Ant output outside captured target")?;
            if path.split('/').any(|p| matches!(p, "" | "." | ".."))
                || path.contains(['\\', ':'])
                || !path.ends_with(".jar")
                || !paths.insert(path.to_ascii_lowercase())
            {
                bail!("invalid or ambiguous Ant JAR path {path}");
            }
            jars.push(Jar {
                target: target
                    .attribute("name")
                    .context("missing Ant target")?
                    .into(),
                path: path.into(),
            });
        }
    }
    if jars.is_empty() {
        bail!("no native Ant jar outputs discovered; custom packaging requires an output adapter");
    }
    Ok(Metadata {
        version,
        manager_version,
        jars,
    })
}
