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

/// A mandatory report obligation in a frozen plan. Native tasks, explicit
/// report declarations and derived packaging actions share this wire contract.
/// Keep all required schema fields together rather than assembling partial JSON.
#[derive(Serialize)]
pub(crate) struct Intent {
    id: String,
    kind: &'static str,
    format: Format,
    required: bool,
    subject: String,
}

impl Intent {
    pub(crate) fn required(id: impl Into<String>, subject: &str, format: Format) -> Self {
        Self {
            id: id.into(),
            kind: format.kind(),
            format,
            required: true,
            subject: subject.into(),
        }
    }
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
    select(root, value, true)
}

/// Direct host tasks must not consume reports left by an earlier invocation.
/// Missing literal prefixes are fresh; existing links or discovery errors fail.
pub(crate) fn ensure_fresh(root: &Path, value: &str) -> Result<()> {
    pattern(value)?;
    let mut path = root.to_path_buf();
    for part in value.split('/').take_while(|p| !p.contains(['*', '?'])) {
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                let redirected = metadata.file_type().is_symlink();
                #[cfg(windows)]
                let redirected = {
                    use std::os::windows::fs::MetadataExt;
                    redirected || metadata.file_attributes() & 0x400 != 0
                };
                if redirected {
                    bail!("unsafe existing report path");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
    if !value.contains(['*', '?']) || !select(root, value, false)?.is_empty() {
        bail!("declared report already exists; preserve or remove it before running tests so stale evidence cannot satisfy this invocation");
    }
    Ok(())
}

fn select(root: &Path, value: &str, required: bool) -> Result<Vec<String>> {
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
    if required && found.is_empty() {
        bail!("report glob matched no files: {value}");
    }
    Ok(found)
}

#[cfg(test)]
mod freshness_tests {
    use super::*;

    #[test]
    fn report_intents_satisfy_required_plan_schema_fields() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../docs/contracts/v1alpha1/plan.schema.json"
        ))
        .unwrap();
        let contract = &schema["$defs"]["reportIntent"];
        for (format, kind) in [
            (Format::Junit, "test"),
            (Format::Cobertura, "coverage"),
            (Format::Lcov, "coverage"),
            (Format::GoCover, "coverage"),
            (Format::Jacoco, "coverage"),
        ] {
            let value = serde_json::to_value(Intent::required(
                "api-container/tests",
                "api-container",
                format,
            ))
            .unwrap();
            for key in contract["required"].as_array().unwrap() {
                assert!(value.get(key.as_str().unwrap()).is_some(), "missing {key}");
            }
            for key in value.as_object().unwrap().keys() {
                assert!(contract["properties"].get(key).is_some(), "unknown {key}");
            }
            assert_eq!(value["subject"], "api-container");
            assert_eq!(value["required"], true);
            assert_eq!(value["kind"], kind);
        }
    }

    #[test]
    fn stale_literals_and_globs_fail_but_missing_prefixes_and_empty_globs_are_fresh() {
        let root = tempfile::tempdir().unwrap();
        for path in ["reports/test.xml", "reports/**/*.xml"] {
            ensure_fresh(root.path(), path).unwrap();
        }
        std::fs::create_dir(root.path().join("reports")).unwrap();
        ensure_fresh(root.path(), "reports/**/*.xml").unwrap();
        std::fs::write(root.path().join("reports/test.xml"), "stale").unwrap();
        assert!(ensure_fresh(root.path(), "reports/test.xml").is_err());
        assert!(ensure_fresh(root.path(), "reports/**/*.xml").is_err());
        assert!(ensure_fresh(root.path(), "../outside.xml").is_err());
        ensure_fresh(root.path(), "reports/new.xml").unwrap();
    }
}
