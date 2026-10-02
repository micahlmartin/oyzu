//! Receipt/content matching. A match is not backend admission or authorization.
use super::{
    access::{self, Directory, Kind},
    tree::{self, TreeEntry},
};
use crate::tools::lock::{self, Lock, Verification};
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    io::Read,
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    format: u32,
    installation_key: String,
    tool_key: String,
    tool_id: String,
    version: String,
    platform: String,
    backend_digest: String,
    distribution_digest: String,
    distribution_size: u64,
    layout_digest: String,
    verification: Verification,
    package_closure_digest: Option<String>,
    dependency_installation_keys: Vec<String>,
    tree_digest: String,
    tree_manifest_digest: String,
    entrypoints: BTreeMap<String, ToolLaunch>,
    environment: BTreeMap<String, Environment>,
    installer_release_digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// A symbolic reference inside a verified installation payload, not a host path.
pub struct ToolInstallPath {
    pub installation_key: String,
    pub relative_path: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
/// A literal UTF-8 argument or an installation-relative path; never shell syntax.
pub enum ToolArgument {
    Literal { value: String },
    Path { path: ToolInstallPath },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
/// Receipt launch shape. Deserialization alone does not validate its references
/// or grant permission to execute; leased lookup borrows the verified snapshot.
pub enum ToolLaunch {
    Native {
        payload_relative_path: String,
        prefix_args: Vec<ToolArgument>,
    },
    Interpreter {
        payload_relative_path: String,
        interpreter: ToolInstallPath,
        prefix_args: Vec<ToolArgument>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Environment {
    Literal { value: String },
    Paths { paths: Vec<ToolInstallPath> },
}

/// Symbolic launch metadata borrowed from a verified, leased selection. These
/// paths remain relative to installation payloads; this is not authorization,
/// an absolute executable path or protection against subsequent same-user edits.
pub struct LeasedToolCommand<'a> {
    pub selection_digest: &'a str,
    pub tool_key: &'a str,
    pub installation_key: &'a str,
    pub tool_id: &'a str,
    pub version: &'a str,
    pub platform: &'a str,
    pub launch: &'a ToolLaunch,
}

pub(super) struct VerifiedSelection {
    pub(super) digest: String,
    records: BTreeMap<String, Receipt>,
}

impl VerifiedSelection {
    pub(super) fn command(&self, name: &str) -> Result<LeasedToolCommand<'_>> {
        access::component(name).map_err(|_| anyhow::anyhow!("TOOL_COMMAND_INVALID"))?;
        let mut found = None;
        for receipt in self.records.values() {
            for (command, launch) in &receipt.entrypoints {
                let matches = if receipt.platform.starts_with("windows/") {
                    command.to_uppercase() == name.to_uppercase()
                } else {
                    command == name
                };
                if matches {
                    ensure!(found.is_none(), "TOOL_COMMAND_AMBIGUOUS");
                    found = Some(LeasedToolCommand {
                        selection_digest: &self.digest,
                        tool_key: &receipt.tool_key,
                        installation_key: &receipt.installation_key,
                        tool_id: &receipt.tool_id,
                        version: &receipt.version,
                        platform: &receipt.platform,
                        launch,
                    });
                }
            }
        }
        found.context("TOOL_COMMAND_MISSING")
    }
}

pub(in crate::tools) fn verify(
    lock: &Lock,
    store: &Path,
    scope: &str,
    profile: &str,
    platform: &str,
    installer: &str,
) -> Result<String> {
    let root = Directory::open(store)?.child("installs")?;
    verify_with(lock, scope, profile, platform, installer, |key| {
        root.child(&key[7..])
    })
    .map(|verified| verified.digest)
}

pub(super) fn verify_with(
    lock: &Lock,
    scope: &str,
    profile: &str,
    platform: &str,
    installer: &str,
    directory: impl Fn(&str) -> Result<Directory>,
) -> Result<VerifiedSelection> {
    lock::digest(installer)?;
    let selection = lock
        .selections
        .iter()
        .find(|s| s.scope == scope && s.profile == profile && s.platform == platform)
        .context("locked selection is unavailable")?;
    let mut records = BTreeMap::new();
    let mut total_entries = 0usize;
    let mut total_bytes = 0u64;
    for (key, installation_key) in &selection.installation_keys {
        let directory = directory(installation_key)?;
        ensure!(
            directory.entries()?
                == vec![
                    ("payload".into(), Kind::Directory),
                    ("receipt.json".into(), Kind::File)
                ],
            "installation directory contains unexpected or redirected entries"
        );
        let file = directory.file("receipt.json")?;
        ensure!(
            access::identity(&file)?.links == 1,
            "receipt has external hardlinks"
        );
        let mut bytes = Vec::new();
        file.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        let value = crate::config::policy::strict_json_limit(&bytes, 2 * 1024 * 1024)?;
        ensure!(
            value.get("package_closure_digest").is_some(),
            "receipt requires explicit package closure field"
        );
        let receipt: Receipt =
            serde_json::from_value(value).context("invalid installation receipt")?;
        let tool = lock
            .tool
            .iter()
            .find(|t| &t.key == key)
            .context("missing locked tool")?;
        let distribution = tool
            .distribution
            .iter()
            .find(|d| d.platform == platform)
            .context("missing locked distribution")?;
        let mut dependencies: Vec<_> = distribution
            .dependencies
            .iter()
            .map(|key| selection.installation_keys[key].clone())
            .collect();
        dependencies.sort();
        ensure!(
            receipt.format == 1
                && receipt.installation_key == *installation_key
                && receipt.tool_key == *key
                && receipt.tool_id == tool.id
                && receipt.version == tool.version
                && receipt.platform == platform
                && receipt.backend_digest == tool.backend_digest
                && receipt.distribution_digest == distribution.digest
                && receipt.distribution_size == distribution.size
                && receipt.layout_digest == distribution.layout_digest
                && receipt.verification == distribution.verification
                && receipt.package_closure_digest == distribution.package_closure_digest
                && receipt.dependency_installation_keys == dependencies
                && receipt.installer_release_digest == installer,
            "installation receipt differs from locked identity or installer release"
        );
        let tree = tree::inspect_directory(&directory.child("payload")?)?;
        total_entries = total_entries
            .checked_add(tree.entries.len())
            .context("selection entry count overflow")?;
        total_bytes = total_bytes
            .checked_add(tree.bytes)
            .context("selection byte count overflow")?;
        ensure!(
            total_entries <= 1_000_000 && total_bytes <= 32 * 1024 * 1024 * 1024,
            "selection verification budget exceeded"
        );
        ensure!(
            tree.digest == receipt.tree_digest
                && tree.manifest_digest == receipt.tree_manifest_digest,
            "installed payload differs from receipt"
        );
        records.insert(installation_key.clone(), (receipt, tree));
    }
    let indexes: BTreeMap<_, BTreeMap<_, _>> = records
        .iter()
        .map(|(key, (_, tree))| {
            (
                key.as_str(),
                tree.entries
                    .iter()
                    .map(|entry| (entry.path.as_str(), entry))
                    .collect(),
            )
        })
        .collect();
    for (installation, (receipt, _)) in &records {
        let allowed: BTreeSet<_> = std::iter::once(installation.as_str())
            .chain(
                receipt
                    .dependency_installation_keys
                    .iter()
                    .map(String::as_str),
            )
            .collect();
        let path = |reference: &ToolInstallPath, directory: Option<bool>| -> Result<()> {
            ensure!(
                allowed.contains(reference.installation_key.as_str()),
                "receipt path references an undeclared installation"
            );
            let target = indexes
                .get(reference.installation_key.as_str())
                .context("receipt path installation is absent")?;
            if reference.relative_path == "." && directory != Some(false) {
                return Ok(());
            }
            let entry = resolve(target, &reference.relative_path)?;
            ensure!(
                match directory {
                    Some(true) => entry.kind == "directory",
                    Some(false) => entry.kind == "file",
                    None => matches!(entry.kind, "file" | "directory"),
                },
                "receipt path has wrong payload type"
            );
            Ok(())
        };
        ensure!(
            receipt.entrypoints.len() <= 1024 && receipt.environment.len() <= 1024,
            "receipt launch/environment limit exceeded"
        );
        let mut commands = BTreeSet::new();
        for (command, launch) in &receipt.entrypoints {
            access::component(command)?;
            ensure!(
                commands.insert(command.to_uppercase()),
                "receipt command case collision"
            );
            let (payload, args, native) = match launch {
                ToolLaunch::Native {
                    payload_relative_path,
                    prefix_args,
                } => (payload_relative_path, prefix_args, true),
                ToolLaunch::Interpreter {
                    payload_relative_path,
                    interpreter,
                    prefix_args,
                } => {
                    path(interpreter, Some(false))?;
                    let target = resolve(
                        &indexes[interpreter.installation_key.as_str()],
                        &interpreter.relative_path,
                    )?;
                    executable(target, platform)?;
                    (payload_relative_path, prefix_args, false)
                }
            };
            let entry = resolve(&indexes[installation.as_str()], payload)?;
            ensure!(entry.kind == "file", "receipt entrypoint is not a file");
            if native {
                executable(entry, platform)?;
            }
            ensure!(args.len() <= 256, "receipt prefix argument limit exceeded");
            for arg in args {
                match arg {
                    ToolArgument::Literal { value } => literal(value)?,
                    ToolArgument::Path { path: reference } => {
                        path(reference, None)?;
                    }
                }
            }
        }
        let mut names = BTreeSet::new();
        for (name, value) in &receipt.environment {
            ensure!(
                !name.is_empty()
                    && name.len() <= 256
                    && name.bytes().enumerate().all(|(i, c)| c == b'_'
                        || c.is_ascii_alphabetic()
                        || (i > 0 && c.is_ascii_digit())),
                "invalid receipt environment name"
            );
            ensure!(
                names.insert(name.to_ascii_uppercase()),
                "receipt environment case collision"
            );
            match value {
                Environment::Literal { value } => {
                    ensure!(
                        !name.eq_ignore_ascii_case("PATH"),
                        "PATH must use typed path references"
                    );
                    literal(value)?;
                }
                Environment::Paths { paths } => {
                    ensure!(
                        paths.len() <= 256,
                        "receipt environment path limit exceeded"
                    );
                    for reference in paths {
                        path(reference, Some(true))?;
                    }
                }
            }
        }
    }
    Ok(VerifiedSelection {
        digest: selection.digest.clone(),
        records: records
            .into_iter()
            .map(|(key, (receipt, _))| (key, receipt))
            .collect(),
    })
}

fn literal(value: &str) -> Result<()> {
    ensure!(
        value.len() <= 16 * 1024 && !value.contains('\0'),
        "invalid receipt literal"
    );
    Ok(())
}
fn executable(entry: &TreeEntry, platform: &str) -> Result<()> {
    ensure!(
        platform.starts_with("windows/") || entry.executable != 0,
        "receipt executable lacks executable permission"
    );
    Ok(())
}

pub(super) fn resolve<'a>(
    index: &BTreeMap<&str, &'a TreeEntry>,
    path: &str,
) -> Result<&'a TreeEntry> {
    access::relative(path)?;
    let mut remaining: VecDeque<_> = path.split('/').map(str::to_owned).collect();
    let mut resolved = Vec::new();
    let mut expansions = 0;
    while let Some(component) = remaining.pop_front() {
        if component == "." {
            continue;
        }
        if component == ".." {
            ensure!(resolved.pop().is_some(), "receipt path escapes payload");
            continue;
        }
        resolved.push(component);
        let key = resolved.join("/");
        let entry = index
            .get(key.as_str())
            .context("receipt path is absent from payload")?;
        if let Some(target) = &entry.target {
            expansions += 1;
            ensure!(expansions <= 64, "receipt path link cycle");
            resolved.pop();
            for part in target.split('/').rev() {
                remaining.push_front(part.to_owned());
            }
        } else {
            ensure!(
                remaining.is_empty() || entry.kind == "directory",
                "receipt path traverses a file"
            );
        }
    }
    index
        .get(resolved.join("/").as_str())
        .copied()
        .context("receipt path does not name a payload entry")
}
