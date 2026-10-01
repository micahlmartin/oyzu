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
        intent
            .validate()
            .with_context(|| format!("{id}: invalid builder output contract"))?;
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
        let operation_id = |stage: &str| {
            if workspace.targets.len() == 1 && workspace.tasks.contains_key(stage) {
                stage.to_string()
            } else {
                format!("{id}:{stage}")
            }
        };
        let operation_contracts: BTreeMap<_, _> = intent
            .tasks
            .iter()
            .map(|(name, plan)| (operation_id(name), plan))
            .collect();
        for stage in &intent.stages {
            // Match public task lookup for a single-target workspace: explicit
            // root tasks own unqualified operations. Never fan a root override
            // out across multiple targets.
            let root_override =
                workspace.targets.len() == 1 && workspace.tasks.contains_key(*stage);
            let task_id = operation_id(stage);
            let Some(task) = workspace.tasks.get(&task_id) else {
                continue;
            };
            if task.availability.is_some() || (!task.build_stage && !root_override) {
                continue;
            }
            let sequence = tasks::sequence(workspace, &task_id)?;
            for (position, step) in sequence.iter().enumerate() {
                if !emitted.insert(step.clone()) {
                    continue;
                }
                let task = &workspace.tasks[step];
                if task.mutates_source {
                    bail!("{step}: mutating formatter cannot run as a build check");
                }
                if !task.target.is_empty() && task.target != id {
                    bail!("{step}: cross-target task prerequisites require graph integration");
                }
                // The operation owns its required evidence even when TOML
                // replaces its body. Runner-specific adaptation belongs to the
                // builder; the engine never guesses how to modify a command.
                // A native operation can first appear as another operation's
                // prerequisite. Its report contract follows its identity.
                let contract = operation_contracts.get(step).copied();
                let native = contract.filter(|_| task.provider == target.manager);
                let mut bindings = super::reporting::bind(&workspace.root, &id, task, contract)?;
                let mut env = intent.env.clone();
                env.extend(task.env.clone());
                if let Some(owner) = tasks::hook_owner(task).and_then(|id| workspace.tasks.get(&id))
                {
                    let reports = super::reporting::bind(
                        &workspace.root,
                        &id,
                        owner,
                        operation_contracts.get(&owner.id()).copied(),
                    )?;
                    env.extend(reports.env);
                }
                env.extend(bindings.env);
                let instrumented = contract
                    .filter(|_| native.is_none())
                    .and_then(|_| builder.instrument_override(target, task, &env));
                let native_reporting = native.is_some() || instrumented.is_some();
                let argv = native.map_or_else(
                    || instrumented.unwrap_or_else(|| task.argv.clone()),
                    |v| v.argv.clone(),
                );
                if !native_reporting {
                    for source in bindings.sources.values_mut() {
                        *source = crate::reports::ReportSource::File;
                    }
                }
                let post = tasks::post_hook(task);
                let boundary = sequence[position + 1..]
                    .iter()
                    .find(|id| **id == post)
                    .unwrap_or(step);
                let mut a = action(
                    step,
                    &id,
                    &task.name,
                    argv,
                    &relative(&workspace.root, &task.cwd)?,
                    &env,
                    (image, &source.digest),
                );
                a["reports"] = json!(bindings.intents);
                a["extensions"] = json!({"oyzu.dev/report-paths":bindings.paths,"oyzu.dev/stdout-must-be-empty":task.stdout_must_be_empty,"oyzu.dev/report-sources":bindings.sources,"oyzu.dev/report-inputs":bindings.inputs,"oyzu.dev/collect-after":boundary});
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
            artifacts.push(json!({"id":artifact_id,"target":id,"variant":{},"name":artifact.name,"producer":producer,"kind":"file","version":artifact.version.as_ref().unwrap_or(&intent.version),"mediaType":artifact.media_type,"path":format!("{id}/artifacts/{}",artifact.filename)}));
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
