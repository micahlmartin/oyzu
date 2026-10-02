//! Conservative target impact from captured source and local baseline evidence.
//! Native workspace members remain one ownership unit until their builder exposes
//! finer input scopes. Selection never grants release authority or cache reuse.
mod git;
#[cfg(test)]
mod tests;
use crate::{model::Workspace, snapshot::Snapshot, tasks};
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Impact {
    pub targets: BTreeSet<String>,
    pub evidence: Value,
}

pub(super) fn resolve(workspace: &Workspace, source: &Snapshot, reference: &str) -> Result<Impact> {
    let all: BTreeSet<_> = workspace.targets.keys().cloned().collect();
    let mut evidence = json!({"reference":reference,"baselineCommit":null,"sourceDigest":source.digest,"changedPaths":[],"reasons":{},"fallback":null});
    let fallback = |reason: &str, mut evidence: Value| {
        evidence["fallback"] = json!(reason);
        Impact {
            targets: all.clone(),
            evidence,
        }
    };
    if workspace
        .configuration
        .values()
        .any(|c| c.management.is_some())
        || workspace.invocation_configuration()?.management.is_some()
    {
        return Ok(fallback("managed-full-build", evidence));
    }
    let (commit, baseline) = match git::baseline(&workspace.root, reference) {
        Ok(value) => value,
        Err(_) => return Ok(fallback("baseline-unavailable", evidence)),
    };
    evidence["baselineCommit"] = json!(commit);
    let current: BTreeMap<_, _> = source
        .entries
        .iter()
        .filter(|e| e.kind == "file")
        .map(|e| {
            (
                e.path.clone(),
                git::File {
                    digest: e.digest.clone().unwrap_or_default(),
                    executable: e.executable,
                },
            )
        })
        .collect();
    let paths: BTreeSet<_> = current.keys().chain(baseline.keys()).cloned().collect();
    let changed: Vec<_> = paths
        .into_iter()
        .filter(|p| current.get(p) != baseline.get(p))
        .collect();
    evidence["changedPaths"] = json!(changed);
    if changed.iter().any(|p| !baseline.contains_key(p)) {
        return Ok(fallback("new-or-untracked-input", evidence));
    }
    let mut targets = BTreeSet::new();
    let mut reasons: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in &changed {
        let name = path.rsplit('/').next().unwrap();
        if name.ends_with(".lock")
            || matches!(
                name,
                "build.yaml"
                    | "oyzu.toml"
                    | "oyzu.local.toml"
                    | "package.json"
                    | "package-lock.json"
                    | "pnpm-lock.yaml"
                    | "pnpm-workspace.yaml"
                    | "pyproject.toml"
                    | "go.mod"
                    | "go.sum"
                    | "Cargo.toml"
                    | "pom.xml"
                    | "build.gradle"
                    | "build.gradle.kts"
                    | "settings.gradle"
                    | "settings.gradle.kts"
                    | "build.xml"
                    | "Chart.yaml"
            )
        {
            return Ok(fallback("configuration-or-lock-change", evidence));
        }
        let owners: Vec<_> = workspace
            .targets
            .iter()
            .filter(|(_, t)| workspace.root.join(path).starts_with(&t.path))
            .map(|(id, _)| id.clone())
            .collect();
        if owners.is_empty() {
            return Ok(fallback("unowned-input", evidence));
        }
        for owner in owners {
            targets.insert(owner.clone());
            reasons
                .entry(owner)
                .or_default()
                .insert(format!("input:{path}"));
        }
    }
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (id, declaration) in &workspace.declarations.targets {
        dependencies.entry(id.clone()).or_default().extend(
            declaration
                .depends_on
                .iter()
                .cloned()
                .chain(declaration.materialize.iter().map(|m| m.from.clone())),
        );
    }
    for task in workspace.tasks.values() {
        for dependency in &task.depends_on {
            let id = tasks::resolve(workspace, dependency)?;
            let owner = &workspace.tasks[&id].target;
            if !owner.is_empty() && owner != &task.target {
                if task.target.is_empty() && !changed.is_empty() {
                    return Ok(fallback("root-task-input-scope", evidence));
                }
                dependencies
                    .entry(task.target.clone())
                    .or_default()
                    .insert(owner.clone());
            }
        }
    }
    loop {
        let before = targets.len();
        for (consumer, inputs) in &dependencies {
            for input in inputs.intersection(&targets.clone()) {
                targets.insert(consumer.clone());
                reasons
                    .entry(consumer.clone())
                    .or_default()
                    .insert(format!("dependency:{input}"));
            }
        }
        if targets.len() == before {
            break;
        }
    }
    evidence["reasons"] = json!(reasons);
    Ok(Impact { targets, evidence })
}
