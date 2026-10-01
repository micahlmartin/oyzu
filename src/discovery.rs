pub(crate) mod detectors;
mod source;
use crate::{
    builders, config,
    model::{Target, Task, Workspace},
};
use anyhow::{bail, Context, Result};
pub use detectors::Resolution;
use std::{collections::BTreeMap, path::Path};

pub fn discover_target(name: &str, path: &Path, explicit: Option<&str>) -> Result<Target> {
    let candidates: Vec<_> = builders::all()
        .iter()
        .filter_map(|builder| builder.detect(path))
        .collect();
    let builder = if let Some(value) = explicit {
        value.to_string()
    } else {
        if candidates.len() != 1 {
            bail!(
                "{}: expected one builder, found {:?}; select uses in build.yaml",
                path.display(),
                candidates
            );
        }
        candidates[0].to_string()
    };
    let mut target = Target {
        name: name.into(),
        builder: builder.clone(),
        manager: builder.clone(),
        path: path.into(),
        version: "0.0.0".into(),
        tasks: BTreeMap::new(),
        discovery: BTreeMap::new(),
    };
    builders::get(&builder)?.discover(&mut target)?;
    for name in ["lint", "format"] {
        builders::unavailable(
            &mut target,
            name,
            "No native configuration or registered integration was discovered",
        );
    }
    Ok(target)
}

pub fn discover(path: &Path) -> Result<Workspace> {
    discover_with_shell(path, None)
}

/// Build discovery selects the executor's default shell, independent of host OS.
pub fn discover_with_shell(path: &Path, default_shell: Option<&str>) -> Result<Workspace> {
    let root = path
        .canonicalize()
        .context("project directory does not exist")?;
    let mut targets = BTreeMap::new();
    if let Some(configs) = config::targets(&root)? {
        for (name, config) in configs {
            let dir = config::contained(&root, config.path.as_deref().unwrap_or(Path::new(".")))?;
            targets.insert(
                name.clone(),
                discover_target(&name, &dir, Some(&config.uses))?,
            );
        }
    } else {
        // Checkout directory spelling must not change logical task identities.
        let name = "project";
        targets.insert(name.into(), discover_target(name, &root, None)?);
    }
    let mut tasks = BTreeMap::new();
    for target in targets.values() {
        for task in target.tasks.values() {
            tasks.insert(task.id(), task.clone());
        }
    }
    let config = config::project(&root)?;
    for task in tasks.values_mut() {
        task.env.extend(config.env.clone());
    }
    for (id, definition) in config.tasks {
        let (group, name) = id.split_once(':').unwrap_or(("", &id));
        let existing = tasks.get(&id);
        let base = if group.is_empty() {
            &root
        } else {
            &targets
                .get(group)
                .with_context(|| format!("unknown task group {group}"))?
                .path
        };
        let cwd = config::contained(base, definition.cwd.as_deref().unwrap_or(Path::new(".")))?;
        if definition.run.is_some() == definition.argv.is_some() {
            bail!("task {id}: specify exactly one of run or argv");
        }
        let argv = if let Some(argv) = definition.argv {
            if argv.is_empty() {
                bail!("task {id}: empty argv");
            }
            argv
        } else {
            let script = definition.run.unwrap();
            let shell = definition.shell.unwrap_or_else(|| {
                if let Some(shell) = default_shell {
                    shell.into()
                } else if cfg!(windows) {
                    "powershell.exe".into()
                } else {
                    "sh".into()
                }
            });
            if shell.to_lowercase().contains("powershell") || shell.to_lowercase().contains("pwsh")
            {
                vec![
                    shell,
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-Command".into(),
                    script,
                ]
            } else {
                vec![shell, "-c".into(), script]
            }
        };
        let mut env = config.env.clone();
        env.extend(definition.env);
        let stage = existing.is_some_and(|t| t.build_stage);
        tasks.insert(
            id.clone(),
            Task {
                name: name.into(),
                target: group.into(),
                provider: "oyzu.toml".into(),
                argv,
                cwd,
                env,
                depends_on: definition.depends_on,
                availability: None,
                build_stage: stage,
                mutates_source: false,
                stdout_must_be_empty: false,
                reports: definition.reports,
            },
        );
    }
    Ok(Workspace {
        root,
        targets,
        tasks,
    })
}
