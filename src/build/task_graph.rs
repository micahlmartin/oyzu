//! Compile task prerequisites, hook boundaries and builder stage order.
//! Tasks retain their target owner; execution, native adaptation and reports
//! remain with their existing subsystems. No project command runs here.
use crate::{builders::BuilderPlan, model::Workspace, tasks};
use anyhow::{bail, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct TaskGraph {
    pub ordered: Vec<String>,
    pub owners: BTreeMap<String, String>,
    pub dependencies: BTreeMap<String, BTreeSet<String>>,
}

pub(super) fn operation(workspace: &Workspace, target: &str, name: &str) -> String {
    if workspace.targets.len() == 1 && workspace.tasks.contains_key(name) {
        name.into()
    } else {
        format!("{target}:{name}")
    }
}

fn entry(workspace: &Workspace, id: &str) -> String {
    let task = &workspace.tasks[id];
    let pre = tasks::pre_hook(task);
    if !tasks::is_hook(task) && workspace.tasks.contains_key(&pre) {
        pre
    } else {
        id.into()
    }
}

pub(super) fn completion(workspace: &Workspace, id: &str) -> String {
    let task = &workspace.tasks[id];
    let post = tasks::post_hook(task);
    if !tasks::is_hook(task) && workspace.tasks.contains_key(&post) {
        post
    } else {
        id.into()
    }
}

impl TaskGraph {
    fn assign_owner(&mut self, workspace: &Workspace, id: &str, inherited: &str) -> Result<()> {
        let task = &workspace.tasks[id];
        let owner = if task.target.is_empty() {
            inherited
        } else {
            &task.target
        };
        if let Some(existing) = self.owners.get(id) {
            if existing != owner {
                bail!("{id}: shared root task has ambiguous build ownership; use a qualified target task");
            }
            return Ok(());
        }
        self.owners.insert(id.into(), owner.into());
        for prerequisite in &task.depends_on {
            self.assign_owner(workspace, &tasks::resolve(workspace, prerequisite)?, owner)?;
        }
        for hook in [entry(workspace, id), completion(workspace, id)] {
            if hook != id {
                self.assign_owner(workspace, &hook, owner)?;
            }
        }
        Ok(())
    }

    pub fn new(workspace: &Workspace, plans: &BTreeMap<String, BuilderPlan>) -> Result<Self> {
        let native: BTreeSet<_> = plans
            .iter()
            .flat_map(|(target, plan)| {
                plan.tasks
                    .keys()
                    .map(|name| operation(workspace, target, name))
                    .filter(|id| {
                        workspace
                            .tasks
                            .get(id)
                            .is_some_and(|task| task.provider == workspace.targets[target].manager)
                    })
            })
            .collect();
        let mut graph = Self {
            ordered: Vec::new(),
            owners: BTreeMap::new(),
            dependencies: BTreeMap::new(),
        };
        let mut emitted = BTreeSet::new();
        for (target, plan) in plans {
            let mut seen = BTreeSet::new();
            let mut previous = None;
            for stage in &plan.stages {
                let id = operation(workspace, target, stage);
                let Some(task) = workspace.tasks.get(&id) else {
                    continue;
                };
                let root_override =
                    workspace.targets.len() == 1 && workspace.tasks.contains_key(*stage);
                if (task.availability.is_some() && !native.contains(&id))
                    || (!task.build_stage && !root_override && !plan.tasks.contains_key(*stage))
                {
                    continue;
                }
                if seen.contains(&id) {
                    continue;
                }
                let sequence = tasks::sequence_for_build(workspace, &id, &native)?;
                graph.assign_owner(workspace, &id, target)?;
                for step in sequence {
                    // Preserve the entire local mutation sequence, including
                    // sibling prerequisites. Only foreign-owned work follows
                    // another pipeline. Shared tasks remain single actions.
                    if !seen.contains(&step) && graph.owners[&step] == *target {
                        if let Some(previous) = previous.replace(step.clone()) {
                            graph
                                .dependencies
                                .entry(step.clone())
                                .or_default()
                                .insert(previous);
                        }
                    }
                    seen.insert(step.clone());
                    if emitted.insert(step.clone()) {
                        graph.ordered.push(step);
                    }
                }
            }
        }
        for id in &graph.ordered {
            let task = &workspace.tasks[id];
            for prerequisite in &task.depends_on {
                let prerequisite = tasks::resolve(workspace, prerequisite)?;
                graph
                    .dependencies
                    .entry(entry(workspace, id))
                    .or_default()
                    .insert(completion(workspace, &prerequisite));
            }
            let pre = entry(workspace, id);
            if pre != *id {
                graph
                    .dependencies
                    .entry(id.clone())
                    .or_default()
                    .insert(pre);
            }
            let post = completion(workspace, id);
            if post != *id {
                graph
                    .dependencies
                    .entry(post)
                    .or_default()
                    .insert(id.clone());
            }
        }
        Ok(graph)
    }
}
