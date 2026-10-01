use crate::model::{Task, Workspace};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::{collections::BTreeSet, process::Command};

#[derive(Debug, Serialize)]
pub struct Outcome {
    pub task: String,
    pub status: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn resolve(workspace: &Workspace, requested: &str) -> Result<String> {
    if workspace.tasks.contains_key(requested) {
        return Ok(requested.into());
    }
    let candidates: Vec<_> = workspace
        .tasks
        .values()
        .filter(|t| t.name == requested)
        .map(Task::id)
        .collect();
    match candidates.as_slice() {
        [only] => Ok(only.clone()),
        [] => bail!("unknown task {requested}"),
        _ => bail!(
            "ambiguous task {requested}; choose {}",
            candidates.join(", ")
        ),
    }
}

fn hook(task: &Task, prefix: &str) -> String {
    if task.target.is_empty() {
        format!("{prefix}_{}", task.name)
    } else {
        format!("{}:{prefix}_{}", task.target, task.name)
    }
}

pub(crate) fn post_hook(task: &Task) -> String {
    hook(task, "post")
}

pub(crate) fn hook_owner(task: &Task) -> Option<String> {
    let name = task
        .name
        .strip_prefix("pre_")
        .or_else(|| task.name.strip_prefix("post_"))?;
    Some(if task.target.is_empty() {
        name.into()
    } else {
        format!("{}:{name}", task.target)
    })
}

fn visit(
    workspace: &Workspace,
    id: &str,
    include_hooks: bool,
    active: &mut BTreeSet<String>,
    emitted: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) -> Result<()> {
    if active.contains(id) {
        bail!("task dependency cycle at {id}");
    }
    if emitted.contains(id) {
        return Ok(());
    }
    let task = &workspace.tasks[id];
    if let Some(reason) = &task.availability {
        bail!("task {id} unavailable: {reason}");
    }
    active.insert(id.into());
    for dep in &task.depends_on {
        visit(
            workspace,
            &resolve(workspace, dep)?,
            true,
            active,
            emitted,
            order,
        )?;
    }
    let is_hook = task.name.starts_with("pre_") || task.name.starts_with("post_");
    if include_hooks && !is_hook {
        let pre = hook(task, "pre");
        if workspace.tasks.contains_key(&pre) {
            visit(workspace, &pre, false, active, emitted, order)?;
        }
    }
    order.push(id.into());
    emitted.insert(id.into());
    if include_hooks && !is_hook {
        let post = hook(task, "post");
        if workspace.tasks.contains_key(&post) {
            visit(workspace, &post, false, active, emitted, order)?;
        }
    }
    active.remove(id);
    Ok(())
}

pub fn sequence(workspace: &Workspace, id: &str) -> Result<Vec<String>> {
    let id = resolve(workspace, id)?;
    let mut order = vec![];
    visit(
        workspace,
        &id,
        true,
        &mut BTreeSet::new(),
        &mut BTreeSet::new(),
        &mut order,
    )?;
    Ok(order)
}

pub fn execute(task: &Task, args: &[String]) -> Result<Outcome> {
    if task.argv.is_empty() {
        bail!("empty command for {}", task.id());
    }
    // Development task execution is explicit; it is never represented as a hermetic build.
    let mut argv = task.argv.clone();
    for builder in crate::builders::all() {
        if let Some(native) = builder.development_argv(task)? {
            argv = native;
            break;
        }
    }
    if !args.is_empty() && argv.iter().any(|s| s == "-c" || s == "-Command") {
        bail!("shell task arguments require an argv task definition");
    }
    if !args.is_empty() {
        if matches!(argv[0].as_str(), "npm" | "pnpm") && argv.get(1).is_some_and(|v| v == "run") {
            argv.push("--".into());
        }
        argv.extend_from_slice(args);
    }
    let path = task
        .env
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| std::ffi::OsStr::new(value));
    let mut command = Command::new(crate::launch::program(&argv[0], path));
    command.args(&argv[1..]);
    let output = command
        .current_dir(&task.cwd)
        .envs(&task.env)
        .output()
        .with_context(|| {
            format!(
                "cannot execute {} for {}; install the required native tool outside Oyzu",
                argv[0],
                task.id()
            )
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let code = if output.status.success() && task.stdout_must_be_empty && !stdout.trim().is_empty()
    {
        1
    } else {
        output.status.code().unwrap_or(1)
    };
    Ok(Outcome {
        task: task.id(),
        status: if code == 0 { "succeeded" } else { "failed" }.into(),
        exit_code: code,
        stdout,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

pub fn run(workspace: &Workspace, requested: &str, args: &[String]) -> Result<Vec<Outcome>> {
    let primary = resolve(workspace, requested)?;
    let mut outcomes = vec![];
    for id in sequence(workspace, &primary)? {
        let outcome = execute(
            &workspace.tasks[&id],
            if id == primary { args } else { &[] },
        )?;
        let failed = outcome.exit_code != 0;
        outcomes.push(outcome);
        if failed {
            break;
        }
    }
    Ok(outcomes)
}
