//! Bind builder requirements and custom task locations without ecosystem dispatch.
use crate::{
    builders::TaskPlan,
    model::Task,
    reports::{self, Input, Root},
};
use anyhow::{bail, Context, Result};
use std::{collections::BTreeMap, path::Path};

#[derive(Default)]
pub(crate) struct Bindings {
    pub intents: Vec<reports::Intent>,
    pub paths: BTreeMap<String, String>,
    pub inputs: BTreeMap<String, Input>,
    pub sources: BTreeMap<String, reports::ReportSource>,
    pub env: BTreeMap<String, String>,
}

pub(crate) fn bind(
    root: &Path,
    target: &str,
    task: &Task,
    contract: Option<&TaskPlan>,
) -> Result<Bindings> {
    let mut result = Bindings::default();
    let cwd = task
        .cwd
        .strip_prefix(root)
        .context("report cwd outside captured source")?
        .to_str()
        .context("non-UTF8 report cwd")?
        .replace('\\', "/");
    let workspace_path = |path: &str| {
        if cwd.is_empty() {
            path.to_string()
        } else {
            format!("{cwd}/{path}")
        }
    };
    if let Some(contract) = contract {
        for report in &contract.reports {
            let kind = report.format.kind();
            if task.reports.iter().any(|r| r.kind == kind) {
                continue;
            }
            let (id, mut path) = if let Some(name) = &report.name {
                if !crate::names::valid(name) {
                    bail!("invalid report module identity");
                }
                (
                    format!("{target}:{kind}:{name}"),
                    format!("{target}/reports/{name}/{}", report.filename),
                )
            } else {
                (
                    format!("{target}:{kind}"),
                    format!("{target}/reports/{}", report.filename),
                )
            };
            let mut destination = format!("/out/{path}");
            if let Some(input) = &report.input {
                reports::validate_declarations(&[reports::Declaration {
                    kind: kind.into(),
                    format: report.format,
                    path: input.clone(),
                }])?;
                let input = workspace_path(input);
                destination = format!("/workspace/{input}");
                if input.contains(['*', '?']) {
                    path.push_str(".files");
                }
                result.inputs.insert(
                    id.clone(),
                    Input {
                        root: Root::Workspace,
                        path: input,
                    },
                );
            }
            result
                .intents
                .push(reports::Intent::required(&id, target, report.format));
            if !destination.contains(['*', '?'])
                && contract
                    .reports
                    .iter()
                    .filter(|r| r.format.kind() == kind)
                    .count()
                    == 1
            {
                result.env.insert(
                    format!("OYZU_{}_REPORT", kind.to_ascii_uppercase()),
                    destination,
                );
            }
            result.sources.insert(id.clone(), report.source);
            if result.paths.insert(id, path).is_some() {
                bail!("colliding native report identity");
            }
        }
    }
    for (index, report) in task.reports.iter().enumerate() {
        let name = crate::names::scoped("task", &task.id());
        let id = format!("{target}:{name}:{}:{index}", report.kind);
        let path = workspace_path(&report.path);
        let mut destination = format!("{target}/reports/{name}-{}-{index}", report.kind);
        if !path.contains(['*', '?']) {
            destination.push('.');
            destination.push_str(report.format.extension());
        }
        result
            .intents
            .push(reports::Intent::required(&id, target, report.format));
        result.paths.insert(id.clone(), destination);
        result
            .sources
            .insert(id.clone(), reports::ReportSource::File);
        if !path.contains(['*', '?'])
            && task
                .reports
                .iter()
                .filter(|r| r.kind == report.kind)
                .count()
                == 1
        {
            result.env.insert(
                format!("OYZU_{}_REPORT", report.kind.to_ascii_uppercase()),
                format!("/workspace/{path}"),
            );
        }
        result.inputs.insert(
            id,
            Input {
                root: Root::Workspace,
                path,
            },
        );
    }
    Ok(result)
}
