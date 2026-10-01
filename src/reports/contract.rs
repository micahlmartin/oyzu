//! Shared report declarations and contained path selection, independent of builders.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    Junit,
    Cobertura,
    Lcov,
    GoCover,
    Jacoco,
}

impl Format {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Junit => "junit",
            Self::Cobertura => "cobertura",
            Self::Lcov => "lcov",
            Self::GoCover => "go-cover",
            Self::Jacoco => "jacoco",
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Junit => "test",
            _ => "coverage",
        }
    }
    pub(crate) fn extension(&self) -> &'static str {
        match self {
            Self::Junit | Self::Cobertura | Self::Jacoco => "xml",
            Self::Lcov => "lcov",
            Self::GoCover => "out",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    pub kind: String,
    pub format: Format,
    pub path: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Root {
    Output,
    Workspace,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    pub root: Root,
    pub path: String,
}

fn pattern(value: &str) -> Result<globset::GlobMatcher> {
    if value.len() > 4096
        || value.split('/').any(|part| {
            part.is_empty()
                || matches!(part, "." | "..")
                || part.ends_with(['.', ' '])
                || part.chars().any(|c| {
                    c.is_control()
                        || matches!(
                            c,
                            '\\' | ':' | '{' | '}' | '[' | ']' | '"' | '<' | '>' | '|'
                        )
                })
                || (part.contains("**") && part != "**")
        })
    {
        bail!("report path must be a contained relative path or simple *, ?, ** glob: {value}");
    }
    Ok(globset::GlobBuilder::new(value)
        .literal_separator(true)
        .backslash_escape(false)
        .build()?
        .compile_matcher())
}

pub(crate) fn validate_declarations(reports: &[Declaration]) -> Result<()> {
    if reports.len() > 64 {
        bail!("task exceeds 64 report declarations");
    }
    let mut seen = BTreeSet::new();
    for report in reports {
        if report.kind != report.format.kind() {
            bail!(
                "report kind {} is incompatible with format {}",
                report.kind,
                report.format.name()
            );
        }
        pattern(&report.path)?;
        if !seen.insert((&report.kind, report.format.name(), &report.path)) {
            bail!("duplicate report declaration {}", report.path);
        }
    }
    Ok(())
}

/// No symlink traversal. Deterministic ordering and bounded discovery precede reads.
pub(crate) fn matches(root: &Path, value: &str) -> Result<Vec<String>> {
    let matcher = pattern(value)?;
    if !value.contains(['*', '?']) {
        return Ok(vec![value.into()]);
    }
    // Start at the literal prefix, avoiding unrelated build trees and symlinks.
    let prefix: Vec<_> = value
        .split('/')
        .take_while(|p| !p.contains(['*', '?']))
        .collect();
    let mut directory = root.to_path_buf();
    for part in &prefix {
        directory.push(part);
        let metadata =
            std::fs::symlink_metadata(&directory).context("missing report glob directory")?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("unsafe report glob directory");
        }
    }
    let mut found = Vec::new();
    for (index, entry) in walkdir::WalkDir::new(directory)
        .follow_links(false)
        .into_iter()
        .enumerate()
    {
        if index >= 100_000 {
            bail!("report discovery exceeds 100000 entries");
        }
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)?
            .to_str()
            .context("non-UTF8 report path")?
            .replace('\\', "/");
        if matcher.is_match(&relative) {
            if !entry.file_type().is_file() {
                bail!("report match is not a regular file: {relative}");
            }
            if found.len() >= 1024 {
                bail!("report glob exceeds 1024 matches");
            }
            found.push(relative);
        }
    }
    found.sort();
    if found.is_empty() {
        bail!("report glob matched no files: {value}");
    }
    Ok(found)
}
