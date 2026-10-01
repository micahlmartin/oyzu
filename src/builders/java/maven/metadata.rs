use anyhow::{bail, Context, Result};
use std::{collections::BTreeSet, fs, path::Path};

pub(super) struct Project {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub packaging: String,
    pub path: String,
    pub pom: String,
    pub directory: String,
    pub final_name: String,
    pub test_roots: Vec<String>,
}

fn value(node: roxmltree::Node<'_, '_>, name: &str) -> Result<String> {
    Ok(node
        .children()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .with_context(|| format!("missing native Maven {name}"))?
        .into())
}

fn path(value: String, root: bool) -> Result<String> {
    if root && value == "." {
        return Ok(value);
    }
    if value.contains(['\\', ':'])
        || value
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        bail!("Maven path is outside captured target: {value}");
    }
    Ok(value)
}

pub(super) fn read(file: &Path) -> Result<Vec<Project>> {
    let text = fs::read_to_string(file)?;
    if text.len() > 16 * 1024 * 1024 {
        bail!("Maven metadata exceeds 16 MiB");
    }
    let doc = roxmltree::Document::parse(&text)?;
    if !doc.root_element().has_tag_name("reactor") {
        bail!("invalid Maven reactor metadata");
    }
    let mut projects = Vec::new();
    let mut names = BTreeSet::new();
    let mut poms = BTreeSet::new();
    for node in doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("project"))
    {
        let group = value(node, "groupId")?;
        let artifact = value(node, "artifactId")?;
        let version = value(node, "version")?;
        for id in [&group, &artifact, &version] {
            if id.is_empty()
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            {
                bail!("invalid or unresolved Maven coordinate {id}");
            }
        }
        if !names.insert((group.clone(), artifact.clone())) {
            bail!("duplicate Maven reactor coordinate");
        }
        let packaging = value(node, "packaging")?;
        if !matches!(packaging.as_str(), "jar" | "pom" | "war") {
            bail!("Maven packaging {packaging} needs a native artifact adapter");
        }
        let final_name = value(node, "finalName")?;
        if final_name.contains(['/', '\\', ':', '$']) || matches!(final_name.as_str(), "." | "..") {
            bail!("invalid Maven artifact finalName");
        }
        let pom = path(value(node, "pom")?, false)?;
        if !poms.insert(pom.to_ascii_lowercase()) {
            bail!("case-colliding Maven POM paths");
        }
        let test_roots = node
            .children()
            .find(|n| n.has_tag_name("testRoots"))
            .context("missing Maven test roots")?
            .children()
            .filter(|n| n.has_tag_name("path"))
            .map(|n| path(n.text().context("empty Maven test root")?.into(), false))
            .collect::<Result<Vec<_>>>()?;
        projects.push(Project {
            group,
            artifact,
            version,
            packaging,
            path: path(value(node, "path")?, true)?,
            pom,
            directory: path(value(node, "buildDirectory")?, false)?,
            final_name,
            test_roots,
        });
    }
    if projects.is_empty() {
        bail!("empty Maven reactor");
    }
    Ok(projects)
}
