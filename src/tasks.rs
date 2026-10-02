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
    native: &BTreeSet<String>,
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
    if let Some(reason) = task.availability.as_ref().filter(|_| !native.contains(id)) {
        bail!("task {id} unavailable: {reason}");
    }
    active.insert(id.into());
    for dep in &task.depends_on {
        visit(
            workspace,
            &resolve(workspace, dep)?,
            true,
            native,
            active,
            emitted,
            order,
        )?;
    }
    let is_hook = task.name.starts_with("pre_") || task.name.starts_with("post_");
    if include_hooks && !is_hook {
        let pre = hook(task, "pre");
        if workspace.tasks.contains_key(&pre) {
            visit(workspace, &pre, false, native, active, emitted, order)?;
        }
    }
    order.push(id.into());
    emitted.insert(id.into());
    if include_hooks && !is_hook {
        let post = hook(task, "post");
        if workspace.tasks.contains_key(&post) {
            visit(workspace, &post, false, native, active, emitted, order)?;
        }
    }
    active.remove(id);
    Ok(())
}

pub fn sequence(workspace: &Workspace, id: &str) -> Result<Vec<String>> {
    sequence_for_build(workspace, id, &BTreeSet::new())
}

/// Captured plans can supply native operations unavailable in host development.
/// This only changes availability admission; hooks and cycle checks are shared.
pub(crate) fn sequence_for_build(
    workspace: &Workspace,
    id: &str,
    native: &BTreeSet<String>,
) -> Result<Vec<String>> {
    let id = resolve(workspace, id)?;
    let mut order = vec![];
    visit(
        workspace,
        &id,
        true,
        native,
        &mut BTreeSet::new(),
        &mut BTreeSet::new(),
        &mut order,
    )?;
    Ok(order)
}

pub fn execute(task: &Task, args: &[String]) -> Result<Outcome> {
    execute_with_unsets(task, args, &BTreeSet::new(), None)
}
fn execute_with_unsets(
    task: &Task,
    args: &[String],
    removed: &BTreeSet<String>,
    config: Option<&crate::config::resolve::EffectiveConfig>,
) -> Result<Outcome> {
    if task.argv.is_empty() {
        bail!("empty command for {}", task.id());
    }
    // Development task execution is explicit; it is never represented as a hermetic build.
    let mut argv = task.argv.clone();
    let mut env = task.env.clone();
    for builder in crate::builders::all() {
        if let Some(native) = builder.development_command(task)? {
            argv = native.argv;
            env.extend(native.env);
            break;
        }
    }
    if let Some(config) = config {
        config.validate_environment(&env)?;
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
    let path = env
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| std::ffi::OsStr::new(value));
    let mut command = Command::new(crate::launch::program(&argv[0], path));
    command.args(&argv[1..]);
    if let Some(profile) = config.map(|config| &config.profile) {
        command.env(
            "OYZU_INHERITED_PROFILE",
            profile.as_deref().unwrap_or("@none"),
        );
    }
    for key in removed.iter().filter_map(|key| key.strip_prefix("env.")) {
        command.env_remove(key);
    }
    let output = command
        .current_dir(&task.cwd)
        .envs(&env)
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
    let sequence = sequence(workspace, &primary)?;
    let configuration = |id: &str| {
        let target = workspace
            .targets
            .get(&workspace.tasks[id].target)
            .or_else(|| {
                (workspace.targets.len() == 1)
                    .then(|| workspace.targets.values().next())
                    .flatten()
            });
        let config = target
            .and_then(|target| workspace.configuration.get(&target.name))
            .or(workspace.root_configuration.as_ref());
        (target, config)
    };
    // Admit every prerequisite and hook before executing any native command.
    for id in &sequence {
        let (target, config) = configuration(id);
        if let Some(config) = config {
            config
                .constraints
                .apply(&mut config.values().clone(), &config.removed)?;
            let builder = target
                .map(|target| crate::builders::get(&target.builder))
                .transpose()?;
            if builder.is_none() && config.get("tools.allowed").is_some() {
                bail!("CONFIG_OVERRIDE_DENIED: root task has no admitted native tool identity");
            }
            crate::config::enforcement::execution_preflight(
                config,
                builder.map_or(&[], |builder| builder.descriptor().tools),
            )?;
            if config.management.is_some() {
                bail!("CONFIG_OVERRIDE_DENIED: managed host tasks require approved development execution and connector bindings");
            }
            config.validate_environment(&workspace.tasks[id].env)?;
        }
    }
    let mut outcomes = vec![];
    for id in sequence {
        let (_, config) = configuration(&id);
        let removed = config
            .map(|config| config.removed.clone())
            .unwrap_or_default();
        let outcome = execute_with_unsets(
            &workspace.tasks[&id],
            if id == primary { args } else { &[] },
            &removed,
            config,
        )?;
        let failed = outcome.exit_code != 0;
        outcomes.push(outcome);
        if failed {
            break;
        }
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_build_availability_keeps_hooks_cycles_and_host_limits() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("Dockerfile"), "FROM scratch\n").unwrap();
        std::fs::write(root.path().join("oyzu.toml"), "[tasks.\"project:pre_test\"]\nargv=['echo','before']\n[tasks.\"project:post_test\"]\nargv=['echo','after']\n").unwrap();
        let workspace = crate::discovery::discover(root.path()).unwrap();
        assert!(sequence(&workspace, "project:test").is_err());
        let native = BTreeSet::from(["project:test".into()]);
        assert_eq!(
            sequence_for_build(&workspace, "project:test", &native).unwrap(),
            ["project:pre_test", "project:test", "project:post_test"]
        );
        assert_eq!(
            sequence_for_build(&workspace, "project:lint", &native).unwrap(),
            ["project:lint"]
        );
        let mut workspace = workspace;
        workspace
            .tasks
            .get_mut("project:lint")
            .unwrap()
            .availability = Some("test profile has no linter".into());
        assert!(sequence_for_build(&workspace, "project:lint", &native).is_err());
        workspace
            .tasks
            .get_mut("project:pre_test")
            .unwrap()
            .depends_on
            .push("project:test".into());
        assert!(sequence_for_build(&workspace, "project:test", &native)
            .unwrap_err()
            .to_string()
            .contains("cycle"));
    }
}
