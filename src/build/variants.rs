//! Expand captured runtime/platform declarations into isolated target/task instances.
//! Owns identity and edge matching, not native version checks or execution.
mod platforms;
use crate::{
    model::{Target, Task, Workspace},
    names, tasks,
};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Mapping = BTreeMap<String, Vec<String>>;
const MAX_INSTANCES: usize = 256;
const MAX_TASKS: usize = 16_384;

fn root_name(name: &str) -> String {
    let mut prefix = String::new();
    let mut rest = name;
    while let Some((hook, tail)) = rest
        .split_once('_')
        .filter(|(hook, _)| matches!(*hook, "pre" | "post"))
    {
        prefix.push_str(hook);
        prefix.push('_');
        rest = tail;
    }
    format!("{prefix}root-{rest}")
}

fn compatible(consumer: &Target, producer: &Target, match_platform: bool) -> bool {
    consumer.variant.iter().all(|(axis, value)| {
        if axis == "platform" && !match_platform {
            return true;
        }
        producer
            .variant
            .get(axis)
            .is_none_or(|other| other == value)
    })
}

fn matches(
    mapping: &Mapping,
    targets: &BTreeMap<String, Target>,
    owner: &Target,
    name: &str,
    match_platform: bool,
) -> Result<Vec<String>> {
    let candidates = mapping
        .get(name)
        .with_context(|| format!("unknown target {name}"))?;
    let requested_platform = match_platform
        .then(|| owner.variant.get("platform"))
        .flatten();
    let selected: Vec<_> = platform_candidates(candidates, targets, requested_platform)
        .into_iter()
        .filter(|id| compatible(owner, &targets[id], match_platform))
        .collect();
    if selected.is_empty() {
        bail!("{}: no compatible variant of {name}", owner.name);
    }
    Ok(selected)
}

fn platform_candidates(
    candidates: &[String],
    targets: &BTreeMap<String, Target>,
    requested: Option<&String>,
) -> Vec<String> {
    // Ordering edges and unconstrained consumers prefer standalone instances;
    // explicit artifact consumers require the matching demanded platform.
    let has_default = candidates
        .iter()
        .any(|id| !targets[id].variant.contains_key("platform"));
    candidates
        .iter()
        .filter(|id| match requested {
            Some(platform) => targets[*id].variant.get("platform") == Some(platform),
            None => !has_default || !targets[*id].variant.contains_key("platform"),
        })
        .cloned()
        .collect()
}

fn task_dependencies(
    original: &Workspace,
    mapping: &Mapping,
    targets: &BTreeMap<String, Target>,
    owner: Option<&Target>,
    task: &mut Task,
    single: bool,
) -> Result<()> {
    let mut dependencies = Vec::new();
    for request in &task.depends_on {
        // Preserve unresolved declarations for ordinary selected-task admission;
        // an unused custom task must not fail an unrelated build.
        let Ok(id) = tasks::resolve(original, request) else {
            dependencies.push(request.clone());
            continue;
        };
        let dependency = &original.tasks[&id];
        if dependency.target.is_empty() {
            if single {
                dependencies.push(format!(
                    "{}:{}",
                    owner.context("missing variant task owner")?.name,
                    root_name(&dependency.name)
                ));
            } else {
                dependencies.push(id);
            }
        } else {
            let candidates = if let Some(owner) = owner {
                matches(
                    mapping,
                    targets,
                    owner,
                    &dependency.target,
                    mapping[&dependency.target].contains(&owner.name),
                )?
            } else {
                platform_candidates(&mapping[&dependency.target], targets, None)
            };
            dependencies.extend(
                candidates
                    .into_iter()
                    .map(|target| format!("{target}:{}", dependency.name)),
            );
        }
    }
    task.depends_on = dependencies;
    Ok(())
}

/// The raw inventory digest remains unchanged: these are derived declarations,
/// not a second parse or a modification of the user's build.yaml.
pub(super) fn expand(workspace: &mut Workspace) -> Result<Mapping> {
    if !workspace
        .declarations
        .targets
        .values()
        .any(|c| !c.matrix.is_empty() || c.platform.is_some())
        || workspace.targets.values().any(|t| !t.variant.is_empty())
    {
        return Ok(workspace
            .targets
            .keys()
            .map(|id| (id.clone(), vec![id.clone()]))
            .collect());
    }
    let original = workspace.clone();
    let platforms = platforms::requirements(&original)?;
    let mut mapping = Mapping::new();
    let mut targets = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for (name, target) in &original.targets {
        let config = original.declarations.targets.get(name);
        let mut axes = config.map(|c| c.matrix.clone()).unwrap_or_default();
        axes.remove("platform");
        let explicit_platform =
            config.is_some_and(|c| c.platform.is_some() || c.matrix.contains_key("platform"));
        let renamed = config.is_some_and(|c| !c.matrix.is_empty());
        let mut variants = vec![target.clone()];
        for (axis, values) in axes {
            if values.is_empty()
                || variants.len().saturating_mul(values.len()) + targets.len() > MAX_INSTANCES
            {
                bail!("build matrix exceeds {MAX_INSTANCES} target instances");
            }
            let mut expanded = Vec::new();
            for variant in variants {
                for value in &values {
                    let mut variant = variant.clone();
                    variant.variant.insert(axis.clone(), value.clone());
                    expanded.push(variant);
                }
            }
            variants = expanded;
        }
        let mut concrete = Vec::new();
        for variant in variants {
            if !explicit_platform {
                concrete.push(variant.clone());
            }
            for platform in &platforms[name] {
                let mut variant = variant.clone();
                variant.variant.insert("platform".into(), platform.clone());
                concrete.push(variant);
                if targets.len() + concrete.len() > MAX_INSTANCES {
                    bail!("build matrix exceeds {MAX_INSTANCES} target instances");
                }
            }
        }
        let variants = &mut concrete;
        for variant in variants.iter_mut() {
            if renamed || (!explicit_platform && variant.variant.contains_key("platform")) {
                let suffix = variant
                    .variant
                    .iter()
                    .map(|(axis, value)| format!("{axis}-{value}"))
                    .collect::<Vec<_>>()
                    .join("-");
                variant.name = names::scoped(name, &suffix);
            }
        }
        if targets.len() + variants.len() > MAX_INSTANCES {
            bail!("runtime matrix exceeds {MAX_INSTANCES} target instances");
        }
        let mut ids = Vec::new();
        for mut target in concrete {
            if !identities.insert(target.name.to_lowercase()) {
                bail!("runtime matrix target identity collision: {}", target.name);
            }
            for task in target.tasks.values_mut() {
                task.target = target.name.clone();
            }
            ids.push(target.name.clone());
            targets.insert(target.name.clone(), target);
        }
        ids.sort();
        mapping.insert(name.clone(), ids);
    }
    let single = original.targets.len() == 1;
    let mut declarations = BTreeMap::new();
    let mut configuration = BTreeMap::new();
    let mut expanded_tasks = BTreeMap::new();
    let mut root_overrides = BTreeMap::new();
    let mut variant_errors = BTreeMap::new();
    for (logical, ids) in &mapping {
        for id in ids {
            let target = &targets[id];
            if let Some(config) = original.configuration.get(logical) {
                configuration.insert(id.clone(), config.clone());
            }
            if let Some(config) = original.declarations.targets.get(logical) {
                let mut config = config.clone();
                config.matrix.clear();
                config.platform = target.variant.get("platform").cloned().or(config.platform);
                config.depends_on = config
                    .depends_on
                    .iter()
                    .map(|name| matches(&mapping, &targets, target, name, false))
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect();
                for input in &mut config.materialize {
                    let candidates = matches(&mapping, &targets, target, &input.from, true)?;
                    if candidates.len() != 1 {
                        let message = format!(
                            "{id}: materialization from {} has ambiguous runtime/platform variants",
                            input.from
                        );
                        // Retaining a standalone instance must not prevent
                        // valid demanded instances from being built. Do not
                        // invent a binding; reject this instance if selected.
                        if !target.variant.contains_key("platform")
                            && !platforms[logical].is_empty()
                        {
                            variant_errors.insert(id.clone(), message);
                            continue;
                        }
                        bail!(message);
                    }
                    input.from = candidates[0].clone();
                }
                declarations.insert(id.clone(), config);
            }
            // Preserve qualified identities. Root overrides have separate task
            // names and stage aliases, so explicit qualified dependencies still
            // refer to their original command and hook family.
            for root in [false, true] {
                for task in original.tasks.values().filter(|task| {
                    if root {
                        single && task.target.is_empty()
                    } else {
                        task.target == *logical
                    }
                }) {
                    if expanded_tasks.len() >= MAX_TASKS {
                        bail!("runtime matrix exceeds {MAX_TASKS} tasks");
                    }
                    let mut task = task.clone();
                    task.target = id.clone();
                    if root {
                        root_overrides.insert(task.id(), format!("{id}:{}", root_name(&task.name)));
                        task.name = root_name(&task.name);
                    }
                    task_dependencies(
                        &original,
                        &mapping,
                        &targets,
                        Some(target),
                        &mut task,
                        single,
                    )?;
                    if expanded_tasks.insert(task.id(), task).is_some() {
                        bail!("{id}: runtime matrix root task identity collision");
                    }
                }
            }
        }
    }
    if !single {
        for task in original
            .tasks
            .values()
            .filter(|task| task.target.is_empty())
        {
            if expanded_tasks.len() >= MAX_TASKS {
                bail!("runtime matrix exceeds {MAX_TASKS} tasks");
            }
            let mut task = task.clone();
            task_dependencies(&original, &mapping, &targets, None, &mut task, false)?;
            expanded_tasks.insert(task.id(), task);
        }
    }
    workspace.targets = targets;
    workspace.tasks = expanded_tasks;
    workspace.configuration = configuration;
    workspace.build_root_overrides = root_overrides;
    workspace.build_variant_errors = variant_errors;
    workspace.declarations.targets = declarations;
    Ok(mapping)
}

#[cfg(test)]
mod tests;
