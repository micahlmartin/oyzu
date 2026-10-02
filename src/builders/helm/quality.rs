//! Chart YAML formatting, separate from native Helm lint and template semantics.
use crate::discovery::detectors::{exclusive, Detector, Finding, Source};
use crate::{
    builders::{task::insert, DevelopmentCommand},
    model::{Target, Task},
};
use anyhow::Result;

struct Yamlfmt;
impl Detector<Source> for Yamlfmt {
    fn id(&self) -> &'static str {
        "helm/yamlfmt"
    }
    fn detect(&self, _source: &Source) -> Result<Vec<Finding>> {
        Ok(vec![Finding::fallback("yamlfmt")])
    }
}

pub(super) fn discover(target: &mut Target) -> Result<()> {
    let source = Source::read(&target.path, &[])?;
    target.discovery.insert(
        "formatter".into(),
        exclusive("Helm YAML formatter", &source, &[&Yamlfmt])?,
    );
    insert(target, "format-check", &["yamlfmt", "-lint", "."], true);
    insert(target, "format", &["yamlfmt", "."], false);
    target.tasks.get_mut("format").unwrap().mutates_source = true;
    Ok(())
}

pub(super) fn development(task: &Task) -> Option<DevelopmentCommand> {
    let operation = match task
        .argv
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["yamlfmt", "-lint", "."] => "format-check",
        ["yamlfmt", "."] => "format",
        _ => return None,
    };
    (task.provider == "helm").then(|| DevelopmentCommand {
        argv: vec![
            "python".into(),
            "-I".into(),
            "-c".into(),
            include_str!("runtime/quality.py").into(),
            operation.into(),
            ".".into(),
        ],
        env: Default::default(),
    })
}
