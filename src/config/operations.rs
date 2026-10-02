//! Non-executing configuration inspection and lossless edits.
use super::{
    edit::Edit,
    locations::{protected_read, Locations},
    registry::{Kind, Registry, Scope, CAPABILITIES},
    session::{Options, Session},
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

/// Configuration operations shared by headless clients, independent of CLI parsing.
pub enum Command {
    Show,
    Get {
        key: String,
    },
    Explain {
        key: Option<String>,
    },
    Profiles,
    Validate {
        strict: bool,
        all_profiles: bool,
    },
    Status,
    Refresh,
    Set {
        key: String,
        value: String,
        json_value: bool,
        scope: WriteScope,
    },
    Unset {
        key: String,
        scope: WriteScope,
    },
}
#[derive(Clone, Copy)]
pub enum WriteScope {
    User,
    Project,
    Local,
}

pub(crate) fn execute(command: &Command, directory: &Path, options: &Options) -> Result<Value> {
    if matches!(command, Command::Status) {
        return status(directory, options);
    }
    if matches!(command, Command::Refresh) {
        let locations = Locations::native()?;
        let bytes = protected_read(&locations.machine.join("management.json"))?
            .context("POLICY_UNAVAILABLE: no protected enrollment")?;
        let bootstrap = super::managed::Bootstrap::parse(&bytes)?;
        let root = super::session::workspace_root(directory, options.root.as_deref())?;
        let acquired =
            super::agent::Agent::new(bootstrap, &locations, &root, super::sources::detected_ci())?
                .acquire(true)?;
        return Ok(
            json!({"refreshed":true,"revision":acquired.snapshot.revision(),"offlineDeadline":acquired.snapshot.deadline()}),
        );
    }
    if let Command::Set { key, scope, .. } | Command::Unset { key, scope } = command {
        // Ordinary edits remain available while execution is blocked. They never
        // write enrollment or activate a requested value as effective policy.
        let registry = Registry::default();
        let locations = Locations::native()?;
        let root = super::session::workspace_root(directory, options.root.as_deref())?;
        let (path, kind) = if matches!(scope, WriteScope::User) {
            (locations.user.clone(), Scope::User)
        } else {
            let directory = directory.canonicalize()?;
            if !directory.starts_with(&root) {
                bail!("CONFIG_SCOPE: edit directory escapes workspace");
            }
            (
                directory.join(if matches!(scope, WriteScope::Project) {
                    "oyzu.toml"
                } else {
                    "oyzu.local.toml"
                }),
                if matches!(scope, WriteScope::Project) {
                    Scope::Project
                } else {
                    Scope::Local
                },
            )
        };
        let value = if let Command::Set {
            value, json_value, ..
        } = command
        {
            Some(if *json_value {
                super::policy::strict_json(value.as_bytes()).map_err(|_| {
                    anyhow::anyhow!(
                        "CONFIG_INVALID_VALUE: malformed, duplicate-key or oversized JSON value"
                    )
                })?
            } else {
                let def = registry
                    .definition(key)
                    .context("CONFIG_INVALID_VALUE: cannot set an unknown key")?;
                match def.kind {
                    Kind::Boolean | Kind::Jobs | Kind::Percent => serde_json::from_str(value)
                        .map_err(|_| anyhow::anyhow!("CONFIG_INVALID_VALUE: invalid scalar"))?,
                    _ => json!(value),
                }
            })
        } else {
            None
        };
        let mut edit = Edit::read(&path)?;
        edit.change(key, value, options.profile.as_deref(), &registry)?;
        edit.commit(
            &path,
            kind,
            kind != Scope::User && path.parent() != Some(root.as_path()),
            &registry,
        )?;
        return Ok(json!({"destination":path,"updated":key}));
    }
    bail!("CONFIG_INVALID_VALUE: inspection requires captured target selection")
}
pub(crate) fn inspect(
    command: &Command,
    directory: &Path,
    mut session: Session,
    selected: Vec<std::path::PathBuf>,
    inventory_diagnostics: Vec<super::sources::Diagnostic>,
) -> Result<Value> {
    session.select_for_targets(&selected)?;
    let all = matches!(
        command,
        Command::Validate {
            all_profiles: true,
            ..
        }
    );
    let mut effective = session.resolve(&directory.canonicalize()?, all)?;
    effective.diagnostics.extend(inventory_diagnostics);
    if matches!(command, Command::Validate { .. }) {
        for target in selected
            .iter()
            .filter(|target| **target != directory.canonicalize().unwrap_or_default())
        {
            effective
                .diagnostics
                .extend(session.resolve(target, all)?.diagnostics);
        }
    }
    match command {
        Command::Show => Ok(
            json!({"values":effective.public_values(&session.registry),"profile":effective.profile,"digest":effective.digest,"diagnostics":effective.diagnostics,"status":"resolved"}),
        ),
        Command::Get { key } => effective
            .public_values(&session.registry)
            .remove(key)
            .context("CONFIG_INVALID_VALUE: setting has no effective value"),
        Command::Explain { key } => {
            let mut result = effective.explain(&session.registry);
            if let Some(key) = key {
                for section in ["values", "origins", "constraintSources", "constraints"] {
                    let v = result[section].get(key).cloned().unwrap_or(Value::Null);
                    result[section] = json!({key:v});
                }
            }
            Ok(result)
        }
        Command::Profiles => Ok(
            json!({"profiles":effective.profiles,"selected":effective.profile,"reason":effective.selection_reason}),
        ),
        Command::Validate { strict, .. } => {
            if *strict && !effective.diagnostics.is_empty() {
                bail!(
                    "CONFIG_UNKNOWN_OPTIONAL: strict validation rejects {} warnings",
                    effective.diagnostics.len()
                );
            }
            Ok(
                json!({"valid":true,"diagnostics":effective.diagnostics,"capabilities":CAPABILITIES}),
            )
        }
        _ => unreachable!(),
    }
}
fn status(directory: &Path, options: &Options) -> Result<Value> {
    let locations = Locations::native()?;
    match protected_read(&locations.machine.join("management.json")) {
        Ok(None) => Ok(json!({"mode":"standalone","enrolled":false})),
        Ok(Some(bytes)) => match super::managed::Bootstrap::parse(&bytes) {
            Ok(b) => {
                let root = super::session::workspace_root(directory, options.root.as_deref())?;
                match super::agent::Agent::new(b, &locations, &root, super::sources::detected_ci())?
                    .status()
                {
                    Ok(status) => Ok(status),
                    Err(_) => Ok(
                        json!({"mode":"managed","enrolled":true,"status":"POLICY_UNAVAILABLE","offlineAvailable":false}),
                    ),
                }
            }
            Err(_) => Ok(json!({"mode":"managed","enrolled":true,"status":"POLICY_INVALID"})),
        },
        Err(_) => Ok(json!({"mode":"managed","enrolled":true,"status":"POLICY_INVALID"})),
    }
}
