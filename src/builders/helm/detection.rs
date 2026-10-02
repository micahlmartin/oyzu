//! Native chart suite evidence, without parsing assertions or running Helm.
use crate::discovery::{
    detectors::{exclusive, Detector, Finding, Source},
    Resolution,
};
use anyhow::{bail, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Context {
    source: Source,
    suites: Vec<String>,
    archives: Vec<(String, Vec<String>)>,
}

struct Unittest;
impl Detector<Context> for Unittest {
    fn id(&self) -> &'static str {
        "helm/unittest"
    }
    fn version(&self) -> &'static str {
        "3"
    }

    fn detect(&self, context: &Context) -> Result<Vec<Finding>> {
        let mut evidence = context
            .suites
            .iter()
            .filter_map(|path| context.source.evidence(path, "/"))
            .collect::<Vec<_>>();
        for (path, locations) in &context.archives {
            for location in locations {
                if let Some(item) = context.source.evidence(path, &format!("tar:{location}")) {
                    evidence.push(item);
                }
            }
        }
        Ok(if evidence.is_empty() {
            vec![]
        } else {
            vec![Finding::native("helm-unittest", evidence)]
        })
    }
}

struct Validation;
impl Detector<Context> for Validation {
    fn id(&self) -> &'static str {
        "helm/native-validation"
    }

    fn detect(&self, _: &Context) -> Result<Vec<Finding>> {
        Ok(vec![Finding::fallback("helm-validation")])
    }
}

pub(super) fn detect(chart: &Path) -> Result<Resolution> {
    let (suites, paths) = suite_paths(chart)?;
    let inputs: Vec<_> = suites
        .iter()
        .chain(paths.iter())
        .map(String::as_str)
        .collect();
    let source = Source::read_binary(chart, &inputs)?;
    let mut archives = Vec::new();
    let mut budget = super::archives::Budget::default();
    for suite in &suites {
        if let Some(bytes) = source.bytes(suite) {
            std::str::from_utf8(bytes)?;
        }
    }
    let mut total = suites.len();
    for path in paths {
        let bytes = source
            .bytes(&path)
            .ok_or_else(|| anyhow::anyhow!("Helm archive disappeared during discovery"))?;
        let locations = super::archives::suites(bytes, &mut budget)?;
        total += locations.len();
        if total > 128 {
            bail!("Helm native suite count exceeds 128");
        }
        archives.push((path, locations));
    }
    exclusive(
        "Helm test framework",
        &Context {
            source,
            suites,
            archives,
        },
        &[&Unittest, &Validation],
    )
}

fn directory(path: &Path) -> Result<Option<fs::ReadDir>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!(
                    "Helm suite discovery requires contained directories: {}",
                    path.display()
                );
            }
            Ok(Some(fs::read_dir(path)?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Follow only native unpacked chart locations. No project execution, archive
/// extraction, repository traversal or dependency acquisition during discovery.
/// Packaged subcharts are bounded binary evidence, not filesystem projections.
fn suite_paths(chart: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let mut archives = Vec::new();
    let mut suites = Vec::new();
    let mut pending = vec![(PathBuf::new(), 0)];
    let mut entries = 0;
    while let Some((relative, depth)) = pending.pop() {
        for child in ["tests", "charts"] {
            let location = relative.join(child);
            let Some(directory) = directory(&chart.join(&location))? else {
                continue;
            };
            for entry in directory {
                entries += 1;
                if entries > 4096 {
                    bail!("Helm suite discovery entry count exceeds 4096");
                }
                let entry = entry?;
                let name = entry.file_name();
                let name = name
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("non-UTF8 Helm test path"))?;
                let path = location.join(name);
                if child == "tests" && name.ends_with("_test.yaml") {
                    suites.push(path.to_str().unwrap().replace('\\', "/"));
                    if suites.len() > 128 {
                        bail!("Helm native suite count exceeds 128");
                    }
                } else if child == "charts" && !name.starts_with(['.', '_']) {
                    let kind = entry.file_type()?;
                    if kind.is_symlink() {
                        bail!("Helm suite discovery rejects symlinked subcharts");
                    }
                    if kind.is_file() && name.ends_with(".tgz") {
                        archives.push(path.to_str().unwrap().replace('\\', "/"));
                    }
                    if !kind.is_dir() {
                        continue;
                    }
                    match fs::symlink_metadata(entry.path().join("Chart.yaml")) {
                        Ok(metadata) if metadata.file_type().is_symlink() => {
                            bail!("Helm subchart metadata is a symlink")
                        }
                        Ok(metadata) if metadata.is_file() => {
                            if depth >= 16 {
                                bail!("Helm subchart discovery depth exceeds 16")
                            }
                            pending.push((path, depth + 1));
                        }
                        Ok(_) => bail!("Helm subchart metadata must be a regular file"),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                        Err(error) => return Err(error.into()),
                    }
                }
            }
        }
    }
    suites.sort();
    archives.sort();
    Ok((suites, archives))
}
