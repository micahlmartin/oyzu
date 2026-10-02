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
}

struct Unittest;
impl Detector<Context> for Unittest {
    fn id(&self) -> &'static str {
        "helm/unittest"
    }
    fn version(&self) -> &'static str {
        "2"
    }

    fn detect(&self, context: &Context) -> Result<Vec<Finding>> {
        let evidence = context
            .suites
            .iter()
            .filter_map(|path| context.source.evidence(path, "/"))
            .collect::<Vec<_>>();
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
    let suites = suite_paths(chart)?;
    let source = Source::read(
        chart,
        &suites.iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    exclusive(
        "Helm test framework",
        &Context { source, suites },
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
fn suite_paths(chart: &Path) -> Result<Vec<String>> {
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
    Ok(suites)
}
