//! Select target directories without native metadata resolution or project execution.
use crate::{builders, config};
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
pub(crate) struct Candidate {
    pub name: String,
    pub path: PathBuf,
    pub builder: Option<String>,
}
pub(crate) struct Inventory {
    pub targets: Vec<Candidate>,
    pub diagnostics: Vec<config::sources::Diagnostic>,
    pub declarations: config::BuildInventory,
}
pub(crate) fn select(root: &Path) -> Result<Inventory> {
    let mut targets = Vec::new();
    let (declarations, diagnostics) = config::capture_targets(root)?;
    if declarations.source_digest.is_some() {
        for (name, config) in &declarations.targets {
            let builder = builders::get(&config.uses).map_err(|_| {
                anyhow::anyhow!("CONFIG_INVALID_VALUE: target {name} names an unregistered builder")
            })?;
            if config.matrix.keys().any(|axis| {
                axis != "platform" && !builder.descriptor().tools.contains(&axis.as_str())
            }) {
                bail!(
                    "CONFIG_INVALID_VALUE: target {name} has an incompatible language matrix axis"
                );
            }
            let dir = config::contained(root, config.path.as_deref().unwrap_or(Path::new(".")))?;
            targets.push(Candidate {
                name: name.clone(),
                path: dir,
                builder: Some(config.uses.clone()),
            });
        }
    } else {
        let candidates: Vec<_> = builders::all()
            .iter()
            .filter_map(|builder| builder.detect(root))
            .collect();
        if !candidates.is_empty() {
            targets.push(Candidate {
                name: "project".into(),
                path: root.to_path_buf(),
                builder: None,
            });
        } else {
            let mut owned = Vec::new();
            let walker = walkdir::WalkDir::new(root)
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
                        .strip_prefix(root)?
                        .to_string_lossy()
                        .replace('\\', "/");
                    let name = crate::names::scoped("project", &relative);
                    targets.push(Candidate {
                        name,
                        path: entry.path().to_path_buf(),
                        builder: None,
                    });
                    owned.push(entry.path().to_path_buf());
                    if targets.len() > 1024 {
                        bail!("CONFIG_LIMIT: too many targets");
                    }
                }
            }
        }
    }
    Ok(Inventory {
        targets,
        diagnostics,
        declarations,
    })
}
