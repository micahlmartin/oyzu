//! Translate typed builder intent into the versioned execution plan.
use crate::{builders, config, dependencies, executor, model::Workspace, records, snapshot, tasks};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn relative(root: &Path, path: &Path) -> Result<String> {
    let value = path
        .strip_prefix(root)
        .context("action path outside captured source")?
        .to_str()
        .context("non-UTF8 action path")?
        .replace('\\', "/");
    Ok(if value.is_empty() { ".".into() } else { value })
}

fn platform(image: &executor::Image) -> Value {
    json!({"os":image.os,"arch":image.arch})
}

fn target_order(workspace: &Workspace) -> Result<Vec<String>> {
    let configs = config::targets(&workspace.root)?.unwrap_or_default();
    for (id, c) in &configs {
        if !c.materialize.is_empty()
            || !c.matrix.is_empty()
            || c.platform.is_some()
            || c.container.is_some()
            || c.bindings.is_some()
        {
            bail!("{id}: platform expansion, materialization and packaging options are not implemented yet");
        }
    }
    let mut pending: BTreeSet<_> = workspace.targets.keys().cloned().collect();
    let mut done = Vec::new();
    while !pending.is_empty() {
        let ready = pending
            .iter()
            .find(|id| {
                configs
                    .get(*id)
                    .is_none_or(|c| c.depends_on.iter().all(|d| done.contains(d)))
            })
            .cloned();
        let id = ready.context("target dependency cycle")?;
        pending.remove(&id);
        done.push(id);
    }
    Ok(done)
}

fn action(
    id: &str,
    target: &str,
    operation: &str,
    argv: Vec<String>,
    cwd: &str,
    env: &BTreeMap<String, String>,
    execution: (&executor::Image, &str),
) -> Value {
    let (image, source) = execution;
    json!({"id":id,"target":target,"operation":operation,"dependsOn":[],"argv":argv,"cwd":cwd,"env":env,
        "tools":[target],"executionPlatform":platform(image),"targetPlatform":platform(image),
        "inputs":[{"kind":"tree","digest":source,"mount":"workspace"}],"outputs":[],"reports":[],
        "required":true,"cacheable":false,"network":"none",
        "limits":{"timeoutSeconds":600,"cpu":2,"memoryBytes":2147483648u64,"outputBytes":16777216}})
}

/// Pure planning after source capture and image resolution; never invokes project code.
pub fn plan(
    workspace: &Workspace,
    source: &snapshot::Snapshot,
    images: &BTreeMap<String, executor::Image>,
) -> Result<Value> {
    plan_with_dependencies(workspace, source, images, &BTreeMap::new())
}

pub(super) fn plan_with_dependencies(
    workspace: &Workspace,
    source: &snapshot::Snapshot,
    images: &BTreeMap<String, executor::Image>,
    dependencies: &BTreeMap<String, dependencies::Prepared>,
) -> Result<Value> {
    let mut planned = Vec::new();
    let mut target_records = Vec::new();
    let mut artifacts = Vec::new();
    let mut tools = Vec::new();
    let builder_digest = snapshot::file_digest(&std::env::current_exe()?)?;
    let mut emitted = BTreeSet::new();
    for id in target_order(workspace)? {
        let target = &workspace.targets[&id];
        let image = images
            .get(&id)
            .with_context(|| format!("{id}: no resolved toolchain image"))?;
        let builder = builders::get(&target.builder)?;
        let intent = builder.plan(builders::PlanningContext {
            target,
            source,
            dependencies: dependencies.get(&id),
        })?;
        let cwd = relative(&workspace.root, &target.path)?;
        target_records.push(json!({"id":id,"builder":target.builder,"builderDigest":builder_digest,"path":cwd,"variant":{},"platform":platform(image)}));
        tools.push(json!({"id":id,"version":image.reference,"digest":image.digest,"platform":platform(image)}));
        for command in &intent.prepare {
            planned.push(action(
                &format!("{id}:{}", command.operation),
                &id,
                command.operation,
                command.argv.clone(),
                &cwd,
                &intent.env,
                (image, &source.digest),
            ));
        }
        for stage in &intent.stages {
            let task_id = format!("{id}:{stage}");
            let Some(task) = workspace.tasks.get(&task_id) else {
                continue;
            };
            if task.availability.is_some() || !task.build_stage {
                continue;
            }
            for step in tasks::sequence(workspace, &task_id)? {
                if !emitted.insert(step.clone()) {
                    continue;
                }
                let task = &workspace.tasks[&step];
                if task.mutates_source {
                    bail!("{step}: mutating formatter cannot run as a build check");
                }
                if !task.target.is_empty() && task.target != id {
                    bail!("{step}: cross-target task prerequisites require graph integration");
                }
                // TOML replacements and hooks retain their own commands; only a
                // native task is eligible for its builder's specialized execution.
                let native = if step == task_id && task.provider == target.manager {
                    intent.tasks.get(*stage)
                } else {
                    None
                };
                let argv = native.map_or_else(|| task.argv.clone(), |v| v.argv.clone());
                let mut env = intent.env.clone();
                env.extend(task.env.clone());
                let mut report_intents = Vec::new();
                let mut paths = BTreeMap::new();
                let mut report_sources = BTreeMap::new();
                if let Some(native) = native {
                    for report in &native.reports {
                        let kind = report.format.kind();
                        let report_id = format!("{id}:{kind}");
                        report_intents.push(json!({"id":report_id,"kind":kind,"format":report.format.name(),"required":true,"subject":id}));
                        report_sources.insert(report_id.clone(), report.source);
                        paths.insert(report_id, format!("{id}/reports/{}", report.filename));
                    }
                }
                let mut a = action(
                    &step,
                    &id,
                    &task.name,
                    argv,
                    &relative(&workspace.root, &task.cwd)?,
                    &env,
                    (image, &source.digest),
                );
                a["reports"] = json!(report_intents);
                a["extensions"] = json!({"oyzu.dev/report-paths":paths,"oyzu.dev/stdout-must-be-empty":task.stdout_must_be_empty,"oyzu.dev/report-sources":report_sources});
                planned.push(a);
            }
        }
        let producer = format!("{id}:{}", intent.package.operation);
        let mut package = action(
            &producer,
            &id,
            intent.package.operation,
            intent.package.argv.clone(),
            &cwd,
            &intent.env,
            (image, &source.digest),
        );
        let mut outputs = Vec::new();
        for artifact in &intent.artifacts {
            let artifact_id = format!("{id}/{}", artifact.name);
            outputs.push(artifact_id.clone());
            artifacts.push(json!({"id":artifact_id,"target":id,"variant":{},"name":artifact.name,"producer":producer,"kind":"file","version":intent.version,"mediaType":artifact.media_type,"path":format!("{id}/artifacts/{}",artifact.filename)}));
        }
        package["outputs"] = json!(outputs);
        planned.push(package);
    }
    // Initial scheduler is deliberately serial; every actual ordering edge is explicit.
    let mut previous: Option<String> = None;
    for a in &mut planned {
        if let Some(dependency) = a["target"].as_str().and_then(|id| dependencies.get(id)) {
            a["inputs"].as_array_mut().unwrap().push(
                json!({"kind":"dependency","digest":dependency.digest,"mount":"dependencies"}),
            );
        }
        if let Some(p) = &previous {
            a["dependsOn"] = json!([p]);
        }
        previous = a["id"].as_str().map(str::to_string);
    }
    let policy = json!({"mode":"standalone","enforcementDigest":records::digest("oyzu.policy.v1alpha1",&json!({"offline":true,"productionEligible":false,"executor":"docker-v1"}))?,"requiredChecks":[]});
    Ok(
        json!({"schemaVersion":"v1alpha1","kind":"build-plan","source":{"treeDigest":source.digest,"commit":null,"dirty":true},"policy":policy,"tools":tools,"targets":target_records,"actions":planned,"artifacts":artifacts}),
    )
}

pub(super) fn resolve_images(
    workspace: &Workspace,
    overrides: &[String],
) -> Result<BTreeMap<String, executor::Image>> {
    let mut refs = BTreeMap::new();
    for value in overrides {
        let (manager, reference) = value
            .split_once('=')
            .context("--image requires manager=image")?;
        if reference.is_empty() || !workspace.targets.values().any(|t| t.manager == manager) {
            bail!("invalid or unused image override {value}");
        }
        if refs.insert(manager, reference).is_some() {
            bail!("duplicate image override for {manager}");
        }
    }
    let mut images = BTreeMap::new();
    for (id, target) in &workspace.targets {
        let default = builders::get(&target.builder)?.toolchain(target)?;
        let reference = refs
            .get(target.manager.as_str())
            .copied()
            .unwrap_or(default);
        images.insert(id.clone(), executor::resolve(reference)?);
    }
    Ok(images)
}
