//! Bind builder requirements and custom task locations without ecosystem dispatch.
use crate::{
    builders::TaskPlan,
    model::Task,
    reports::{self, Input, Root},
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

#[derive(Default)]
pub(super) struct Bindings {
    pub intents: Vec<Value>,
    pub paths: BTreeMap<String, String>,
    pub inputs: BTreeMap<String, Input>,
    pub sources: BTreeMap<String, reports::ReportSource>,
    pub env: BTreeMap<String, String>,
}

pub(super) fn bind(
    root: &Path,
    target: &str,
    task: &Task,
    contract: Option<&TaskPlan>,
) -> Result<Bindings> {
    let mut result = Bindings::default();
    if let Some(contract) = contract {
        for report in &contract.reports {
            let kind = report.format.kind();
            if task.reports.iter().any(|r| r.kind == kind) {
                continue;
            }
            let id = format!("{target}:{kind}");
            let path = format!("{target}/reports/{}", report.filename);
            result.intents.push(json!({"id":id,"kind":kind,"format":report.format.name(),"required":true,"subject":target}));
            result.env.insert(
                format!("OYZU_{}_REPORT", kind.to_ascii_uppercase()),
                format!("/out/{path}"),
            );
            result.sources.insert(id.clone(), report.source);
            result.paths.insert(id, path);
        }
    }
    let cwd = task
        .cwd
        .strip_prefix(root)
        .context("report cwd outside captured source")?
        .to_str()
        .context("non-UTF8 report cwd")?
        .replace('\\', "/");
    for (index, report) in task.reports.iter().enumerate() {
        let name = crate::names::scoped("task", &task.id());
        let id = format!("{target}:{name}:{}:{index}", report.kind);
        let path = if cwd.is_empty() {
            report.path.clone()
        } else {
            format!("{cwd}/{}", report.path)
        };
        let mut destination = format!("{target}/reports/{name}-{}-{index}", report.kind);
        if !path.contains(['*', '?']) {
            destination.push('.');
            destination.push_str(report.format.extension());
        }
        result.intents.push(json!({"id":id,"kind":report.kind,"format":report.format.name(),"required":true,"subject":target}));
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
