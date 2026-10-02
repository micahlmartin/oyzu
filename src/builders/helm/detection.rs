//! Native chart suite evidence, without parsing assertions or running Helm.
use crate::discovery::{
    detectors::{exclusive, Detector, Finding, Source},
    Resolution,
};
use anyhow::{bail, Result};
use std::{fs, path::Path};

struct Context {
    source: Source,
    suites: Vec<String>,
}

struct Unittest;
impl Detector<Context> for Unittest {
    fn id(&self) -> &'static str {
        "helm/unittest"
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
    let tests = chart.join("tests");
    let mut suites = Vec::new();
    match fs::symlink_metadata(&tests) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!("Helm tests must be a contained directory");
            }
            for (index, entry) in fs::read_dir(tests)?.enumerate() {
                if index >= 4096 {
                    bail!("Helm test directory entry count exceeds 4096");
                }
                let entry = entry?;
                let name = entry.file_name();
                let name = name
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("non-UTF8 Helm test path"))?;
                if name.ends_with("_test.yaml") {
                    suites.push(format!("tests/{name}"));
                    if suites.len() > 128 {
                        bail!("Helm native suite count exceeds 128");
                    }
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    suites.sort();
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
