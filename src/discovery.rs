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
    discover_with_options(
        path,
        default_shell,
        &config::session::Options {
            root: Some(path.into()),
            ..Default::default()
        },
    )
}

pub fn discover_with_options(
    path: &Path,
    default_shell: Option<&str>,
    options: &config::session::Options,
) -> Result<Workspace> {
    let mut session = config::session::Session::open(path, options)?;
    let root = session.root.clone();
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
        let candidates: Vec<_> = builders::all()
            .iter()
            .filter_map(|builder| builder.detect(&root))
            .collect();
        if !candidates.is_empty() {
            targets.insert("project".into(), discover_target("project", &root, None)?);
        } else {
            let mut owned = Vec::new();
            let walker = walkdir::WalkDir::new(&root)
                .max_depth(32)
                .follow_links(false)
                .into_iter()
                .filter_entry(|entry| {
                    entry.depth() == 0
                        || !matches!(
                            entry.file_name().to_str(),
                            Some(
                                ".git"
                                    | ".oyzu"
                                    | ".cache"
                                    | "dist"
                                    | "target"
                                    | "build"
                                    | "node_modules"
                                    | "vendor"
                                    | ".venv"
                                    | "venv"
                                    | "__pycache__"
                            )
                        )
                });
            for entry in walker {
                let entry = entry?;
                if !entry.file_type().is_dir()
                    || entry.path() == root
                    || owned
                        .iter()
                        .any(|path: &std::path::PathBuf| entry.path().starts_with(path))
                {
                    continue;
                }
                if entry.path().join(".git").exists() {
                    owned.push(entry.path().to_path_buf());
                    continue;
                }
                if builders::all()
                    .iter()
                    .any(|builder| builder.detect(entry.path()).is_some())
                {
                    let relative = entry
                        .path()
                        .strip_prefix(&root)?
                        .to_string_lossy()
                        .replace('\\', "/");
                    let name = crate::names::scoped("project", &relative);
                    targets.insert(name.clone(), discover_target(&name, entry.path(), None)?);
                    owned.push(entry.path().to_path_buf());
                    if targets.len() > 1024 {
                        bail!("CONFIG_LIMIT: too many targets");
                    }
                }
            }
            if targets.is_empty() {
                bail!("no conventional projects discovered");
            }
        }
    }
    let mut native_units = std::collections::BTreeSet::new();
    for target in targets.values() {
        if !native_units.insert((target.path.clone(), target.manager.clone())) {
            bail!("CONFIG_INVALID_VALUE: two targets own the same native build unit");
        }
    }
    let mut tasks = BTreeMap::new();
    for target in targets.values() {
        for task in target.tasks.values() {
            tasks.insert(task.id(), task.clone());
        }
    }
    session.select_for_targets(
        &targets
            .values()
            .map(|target| target.path.clone())
            .collect::<Vec<_>>(),
    )?;
    let root_effective = session.requested_root()?;
    let mut definitions: BTreeMap<_, _> = config::project_from_effective(&root_effective)?
        .tasks
        .into_iter()
        .filter(|(id, _)| !id.contains(':'))
        .collect();
    let mut effective_configs = BTreeMap::new();
    for (id, target) in &targets {
        let effective = session.resolve(&target.path, false)?;
        let config = config::project_from_effective(&effective)?;
        for task in tasks.values_mut().filter(|task| task.target == *id) {
            task.env.extend(config.env.clone());
        }
        for (name, definition) in config.tasks {
            if name.split_once(':').is_some_and(|(group, _)| group == id) {
                definitions.insert(name, definition);
            } else if !name.contains(':') && target.path != root {
                // A nested unqualified definition belongs only to this target.
                let key = format!("tasks.{name}");
                let nested = effective
                    .origins
                    .get(&key)
                    .and_then(|origins| origins.last())
                    .is_some_and(|origin| {
                        Path::new(&origin.source)
                            .parent()
                            .is_some_and(|parent| parent != root)
                    });
                if nested {
                    definitions.insert(format!("{id}:{name}"), definition);
                }
            }
        }
        effective_configs.insert(id.clone(), effective);
    }
    for (id, definition) in definitions {
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
        let effective = effective_configs
            .get(group)
            .or_else(|| {
                (group.is_empty() && targets.len() == 1)
                    .then(|| effective_configs.values().next())
                    .flatten()
            })
            .unwrap_or(&root_effective);
        let cwd = if let Some(relative) = definition.cwd.as_deref() {
            let origin = effective
                .origins
                .get(&format!("tasks.{id}"))
                .or_else(|| effective.origins.get(&format!("tasks.{name}")))
                .and_then(|origins| origins.last());
            let declared = origin
                .and_then(|origin| Path::new(&origin.source).parent())
                .filter(|path| path.is_absolute())
                .unwrap_or(base);
            config::execution_path(&root, declared, relative)?
        } else {
            base.clone()
        };
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
        let effective = effective_configs
            .get(group)
            .or_else(|| {
                (group.is_empty() && targets.len() == 1)
                    .then(|| effective_configs.values().next())
                    .flatten()
            })
            .unwrap_or(&root_effective);
        let mut env = config::project_from_effective(effective)?.env;
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
    let mut warnings = std::collections::BTreeSet::new();
    for diagnostic in effective_configs
        .values()
        .chain(std::iter::once(&root_effective))
        .flat_map(|config| &config.diagnostics)
    {
        if warnings.insert((&diagnostic.source, &diagnostic.key)) {
            eprintln!(
                "{}: {}: {}: {}",
                diagnostic.code, diagnostic.source, diagnostic.key, diagnostic.message
            );
        }
    }
    Ok(Workspace {
        root,
        targets,
        tasks,
        configuration: effective_configs,
        root_configuration: Some(root_effective),
    })
}
