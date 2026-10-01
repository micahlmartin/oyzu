use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Build {
    pub schema_version: u32,
    pub gradle_version: String,
    pub directory: String,
    pub projects: Vec<Project>,
}

#[derive(Deserialize)]
pub(super) struct Project {
    pub path: String,
    pub version: String,
    pub archives: Vec<Archive>,
    pub tests: Vec<Test>,
}

#[derive(Deserialize)]
pub(super) struct Archive {
    pub task: String,
    pub file: String,
    pub extension: String,
    pub version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Test {
    pub task: String,
    pub junit: String,
    pub junit_enabled: bool,
    pub sources_known: bool,
    pub sources: Vec<String>,
    pub coverage: Option<String>,
}

fn contained(path: &str, root: bool) -> Result<()> {
    if root && path == "." {
        return Ok(());
    }
    if path.contains(['\\', ':', '*', '?']) || path.split('/').any(|p| matches!(p, "" | "." | ".."))
    {
        bail!("native Gradle path escapes captured source: {path}");
    }
    Ok(())
}

pub(super) fn read(directory: &Path) -> Result<Vec<Build>> {
    let mut paths: Vec<_> = fs::read_dir(directory)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    paths.sort();
    if paths.len() > 1024 {
        bail!("too many native Gradle build models");
    }
    let mut builds = Vec::new();
    let mut identities = BTreeSet::new();
    for path in paths {
        let file = fs::symlink_metadata(&path)?;
        if !file.is_file() || file.file_type().is_symlink() || file.len() > 16 * 1024 * 1024 {
            bail!("invalid native Gradle metadata file");
        }
        let build: Build = serde_json::from_slice(&fs::read(path)?)?;
        contained(&build.directory, true)?;
        if build.schema_version != 1
            || build.projects.is_empty()
            || !identities.insert(build.directory.to_lowercase())
        {
            bail!("invalid or duplicate native Gradle build model");
        }
        let mut projects = BTreeSet::new();
        for project in &build.projects {
            if !project.path.starts_with(':') || !projects.insert(&project.path) {
                bail!("invalid or duplicate native Gradle project identity");
            }
            for archive in &project.archives {
                contained(&archive.file, false)?;
                if archive.version.is_empty() || !archive.task.starts_with(':') {
                    bail!("missing native Gradle archive identity");
                }
            }
            for test in &project.tests {
                contained(&test.junit, false)?;
                if let Some(path) = &test.coverage {
                    contained(path, false)?;
                }
                for path in &test.sources {
                    contained(path, false)?;
                }
            }
        }
        builds.push(build);
    }
    builds
        .iter()
        .find(|b| b.directory == ".")
        .context("missing root Gradle model")?;
    Ok(builds)
}
