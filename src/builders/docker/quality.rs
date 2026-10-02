//! Native Dockerfile quality defaults; configuration is interpreted by the tools.
use crate::{
    builders::task::insert,
    discovery::detectors::{exclusive, Detector, Finding, Source},
    model::Target,
};
use anyhow::Result;

pub(super) const CONTROL_FILES: &[&str] = &[".hadolint.yaml", ".hadolint.yml", ".editorconfig"];

struct Hadolint;
impl Detector<Source> for Hadolint {
    fn id(&self) -> &'static str {
        "docker/hadolint"
    }
    fn detect(&self, source: &Source) -> Result<Vec<Finding>> {
        let evidence = [".hadolint.yaml", ".hadolint.yml"]
            .iter()
            .filter_map(|path| source.evidence(path, "/"))
            .collect::<Vec<_>>();
        Ok(vec![if evidence.is_empty() {
            Finding::fallback("hadolint")
        } else {
            Finding::native("hadolint", evidence)
        }])
    }
}

struct Dockerfmt;
impl Detector<Source> for Dockerfmt {
    fn id(&self) -> &'static str {
        "docker/dockerfmt"
    }
    fn detect(&self, source: &Source) -> Result<Vec<Finding>> {
        Ok(vec![match source.evidence(".editorconfig", "/") {
            Some(evidence) => Finding::native("dockerfmt", vec![evidence]),
            None => Finding::fallback("dockerfmt"),
        }])
    }
}

pub(super) fn discover(target: &mut Target) -> Result<()> {
    let source = Source::read(&target.path, CONTROL_FILES)?;
    target.discovery.insert(
        "linter".into(),
        exclusive("Dockerfile linter", &source, &[&Hadolint])?,
    );
    target.discovery.insert(
        "formatter".into(),
        exclusive("Dockerfile formatter", &source, &[&Dockerfmt])?,
    );
    insert(
        target,
        "lint",
        &["hadolint", "--no-color", "Dockerfile"],
        true,
    );
    let mut formatter = vec!["dockerfmt"];
    if source.text(".editorconfig").is_none() {
        formatter.push("--newline");
    }
    for (name, flag, stage) in [
        ("format-check", "--check", true),
        ("format", "--write", false),
    ] {
        let mut argv = formatter.clone();
        argv.extend([flag, "Dockerfile"]);
        insert(target, name, &argv, stage);
    }
    target.tasks.get_mut("format").unwrap().mutates_source = true;
    Ok(())
}
