//! Development shell sessions: reversible environment transitions and upstream
//! hook composition. Configuration, frozen selection and rendering stay with
//! their existing owners. No networking, installation or profile editing occurs.
use super::development;
use crate::config::session::Options;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

type Environment = BTreeMap<String, String>;
const TOKEN: &str = "OYZU_SHELL_SESSION";

#[derive(Default, Serialize, Deserialize)]
struct State {
    #[serde(default)]
    shim_directory: Option<PathBuf>,
    store: Option<PathBuf>,
    profile: Option<String>,
    no_profile: bool,
    local_overrides: bool,
    root: Option<PathBuf>,
    identity: String,
    before: BTreeMap<String, Option<String>>,
    after: Environment,
}

impl State {
    fn options(&self) -> Options {
        Options {
            root: self.root.clone(),
            profile: self.profile.clone(),
            no_profile: self.no_profile,
            local_overrides: self.local_overrides,
            ..Default::default()
        }
    }
}

pub(super) fn active_options() -> Result<Option<Options>> {
    let Some(token) = std::env::var_os(TOKEN) else {
        return Ok(None);
    };
    let state: State = serde_json::from_slice(&std::fs::read(state_path(
        token.to_str().context("invalid shell session token")?,
    )?)?)?;
    Ok(Some(state.options()))
}

fn state_root() -> PathBuf {
    std::env::temp_dir().join("oyzu-shell-sessions")
}
fn state_path(token: &str) -> Result<PathBuf> {
    ensure!(
        token.starts_with("session-")
            && token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
        "invalid shell session token"
    );
    Ok(state_root().join(token))
}
fn current() -> Environment {
    std::env::vars_os()
        .filter_map(|(k, v)| {
            Some((
                development::environment_key(&k.into_string().ok()?),
                v.into_string().ok()?,
            ))
        })
        .collect()
}

/// Create a temporary session and print hooks for caller evaluation. Does not
/// edit profiles or execute the script; already active environments are refused.
pub fn activate(
    directory: &Path,
    options: &Options,
    store: Option<&Path>,
    shell: &str,
) -> Result<i32> {
    ensure!(
        std::env::var_os(TOKEN).is_none(),
        "Oyzu is already active; deactivate before starting another session"
    );
    let hooks = development::hooks(shell, true)?;
    std::fs::create_dir_all(state_root())?;
    let file = tempfile::Builder::new()
        .prefix("session-")
        .tempfile_in(state_root())?;
    let state = State {
        shim_directory: Some(super::shims::prepare(
            &directory.join(store.unwrap_or(Path::new(".oyzu/tools"))),
            store.is_none(),
        )?),
        store: store
            .map(|path| std::path::absolute(directory.join(path)))
            .transpose()?,
        profile: options.profile.clone(),
        no_profile: options.no_profile,
        local_overrides: options.local_overrides,
        root: options.root.as_ref().map(std::path::absolute).transpose()?,
        ..State::default()
    };
    serde_json::to_writer(file.as_file(), &state)?;
    let (_, path) = file.keep()?;
    let token = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("invalid session filename")?;
    let prefix = development::render_changes(
        shell,
        current(),
        [(TOKEN.into(), Some(token.into()))].into(),
    )?;
    print!("{prefix}{hooks}");
    Ok(0)
}

fn rollback(state: &State, environment: &Environment) -> Environment {
    let mut restored = environment.clone();
    for (key, applied) in &state.after {
        if environment.get(key) == Some(applied) {
            match state.before.get(key).and_then(Clone::clone) {
                Some(value) => {
                    restored.insert(key.clone(), value);
                }
                None => {
                    restored.remove(key);
                }
            }
        } else if key == "PATH" {
            // Preserve user additions. Remove only an unchanged leading owned
            // insertion; ambiguous rearrangements are retained for now.
            let before = state
                .before
                .get(key)
                .and_then(|v| v.as_deref())
                .unwrap_or("");
            let old: Vec<_> = std::env::split_paths(before).collect();
            let added: Vec<_> = std::env::split_paths(applied).collect();
            let mut paths: Vec<_> =
                std::env::split_paths(environment.get(key).map(String::as_str).unwrap_or(""))
                    .collect();
            if added.len() == old.len() + 1 && added[1..] == old && paths.first() == added.first() {
                paths.remove(0);
                if let Ok(value) = std::env::join_paths(paths) {
                    if let Ok(value) = value.into_string() {
                        restored.insert(key.clone(), value);
                    }
                }
            }
        }
    }
    restored
}

fn changes(before: &Environment, after: &Environment) -> BTreeMap<String, Option<String>> {
    before
        .keys()
        .chain(after.keys())
        .filter(|key| before.get(*key) != after.get(*key))
        .map(|key| (key.clone(), after.get(key).cloned()))
        .collect()
}

/// Reconcile one active session with frozen local selection and print a shell
/// delta. Missing selection produces cleanup and an unavailable status. Explicit
/// deactivation removes the session record after rendering its restoration.
pub fn transition(directory: &Path, shell: &str, deactivate: bool) -> Result<i32> {
    let token = std::env::var(TOKEN).context("Oyzu shell session is not active")?;
    let path = state_path(&token)?;
    let mut state: State = serde_json::from_slice(&std::fs::read(&path)?)?;
    let environment = current();
    let baseline = rollback(&state, &environment);
    let mut target = baseline.clone();
    let mut identity = String::new();
    let mut unavailable = None;
    if !deactivate {
        let options = state.options();
        let store = state
            .store
            .clone()
            .unwrap_or_else(|| directory.join(".oyzu/tools"));
        let shim_directory = state
            .shim_directory
            .as_ref()
            .context("deactivate and reactivate this older shell session to enable shims")?;
        let mut paths: Vec<_> = baseline
            .get("PATH")
            .map(|path| std::env::split_paths(path).collect())
            .unwrap_or_default();
        paths.insert(0, shim_directory.clone());
        let shim_path = std::env::join_paths(paths)?
            .into_string()
            .map_err(|_| anyhow::anyhow!("shell PATH requires UTF-8"))?;
        match development::installed_command(directory, &options, &store) {
            Ok(selected) => {
                identity = crate::records::digest(
                    "oyzu.shell-selection.v1",
                    &serde_json::json!({"directory":std::path::absolute(directory)?, "executable":selected.executable, "configuration":selected.effective.values()}),
                )?;
                if identity == state.identity {
                    return Ok(0);
                }
                let desired = development::compose_environment(
                    &selected,
                    baseline.get("PATH").map(std::ffi::OsStr::new),
                )?;
                for (key, value) in desired {
                    target.insert(
                        key,
                        value.into_string().map_err(|_| {
                            anyhow::anyhow!("shell environment requires UTF-8 values")
                        })?,
                    );
                }
                target.insert("PATH".into(), shim_path.clone());
                target.insert("OYZU_TOOL_STATUS".into(), "ready".into());
            }
            Err(error) => {
                identity = format!("unavailable:{}:{error:#}", directory.display());
                if identity == state.identity {
                    return Ok(0);
                }
                target.insert("OYZU_TOOL_STATUS".into(), "unavailable".into());
                target.insert("PATH".into(), shim_path);
                unavailable = Some(error);
            }
        }
    } else {
        target.remove(TOKEN);
        target.remove("OYZU_FRONTEND");
        target.remove("__OYZU_ORIG_PATH");
    }
    let mut output =
        development::render_changes(shell, environment.clone(), changes(&environment, &target))?;
    if deactivate {
        output.push_str(&development::hooks(shell, false)?);
        std::fs::remove_file(path)?;
    } else {
        state.before = changes(&baseline, &target)
            .keys()
            .map(|key| (key.clone(), baseline.get(key).cloned()))
            .collect();
        state.after = state
            .before
            .keys()
            .filter_map(|key| target.get(key).map(|value| (key.clone(), value.clone())))
            .collect();
        state.identity = identity;
        std::fs::write(path, serde_json::to_vec(&state)?)?;
    }
    if let Some(error) = unavailable {
        eprintln!("oyzu: selection unavailable: {error:#}");
    }
    print!("{output}");
    Ok(0)
}
