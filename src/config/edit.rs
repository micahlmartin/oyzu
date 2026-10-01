//! Lossless ordinary-source editing with optimistic concurrency detection.
use super::{
    registry::{Registry, Scope},
    sources::ConfigSource,
};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
use toml_edit::{DocumentMut, Item, Table};

pub struct Edit {
    original: Option<Vec<u8>>,
    document: DocumentMut,
}
impl Edit {
    pub fn read(path: &Path) -> Result<Self> {
        if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            bail!("CONFIG_SCOPE: edit destination must not be a symlink");
        }
        let original = match fs::File::open(path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
                if bytes.len() > 1024 * 1024 {
                    bail!("CONFIG_LIMIT: oversized edit source");
                }
                Some(bytes)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let text = std::str::from_utf8(original.as_deref().unwrap_or_default())
            .context("CONFIG_SYNTAX: invalid UTF-8")?;
        let document = text
            .parse()
            .map_err(|_| anyhow::anyhow!("CONFIG_SYNTAX: invalid TOML"))?;
        Ok(Self { original, document })
    }
    pub fn change(
        &mut self,
        key: &str,
        value: Option<Value>,
        profile: Option<&str>,
        registry: &Registry,
    ) -> Result<()> {
        let definition = registry
            .definition(key)
            .context("CONFIG_INVALID_VALUE: cannot edit unknown setting")?;
        if definition.administrative {
            bail!("CONFIG_SCOPE: administrative setting is protected");
        }
        if profile.is_some() && !definition.profile {
            bail!("CONFIG_SCOPE: setting is not profile eligible");
        }
        let (root, entry) = key
            .split_once('.')
            .context("CONFIG_INVALID_VALUE: expected setting path")?;
        let mut table: &mut dyn toml_edit::TableLike = self.document.as_table_mut();
        let mut segments = Vec::new();
        if let Some(name) = profile {
            if !crate::names::valid(name) {
                bail!("CONFIG_INVALID_VALUE: invalid profile name");
            }
            segments.extend(["profiles", name]);
        }
        segments.push(root);
        for segment in segments {
            if !table.contains_key(segment) {
                if value.is_none() {
                    return Ok(());
                }
                table.insert(segment, Item::Table(Table::new()));
            }
            table = table
                .get_mut(segment)
                .and_then(Item::as_table_like_mut)
                .context("CONFIG_INVALID_VALUE: edit requires a table")?;
        }
        if let Some(value) = value {
            let value = registry.validate(key, &value)?;
            let toml_value: toml::Value = serde_json::from_value(value)?;
            let wrapper = BTreeWrapper { value: toml_value };
            let parsed: DocumentMut = toml::to_string(&wrapper)?.parse()?;
            let mut replacement = parsed["value"].clone();
            if let (Some(old), Some(new)) = (
                table.get(entry).and_then(Item::as_value),
                replacement.as_value_mut(),
            ) {
                *new.decor_mut() = old.decor().clone();
            }
            table.insert(entry, replacement);
        } else {
            table.remove(entry);
        }
        Ok(())
    }
    pub fn commit(
        self,
        path: &Path,
        scope: Scope,
        nested: bool,
        registry: &Registry,
    ) -> Result<()> {
        let text = self.document.to_string();
        ConfigSource::parse(
            &path.display().to_string(),
            path.parent().unwrap(),
            scope,
            nested,
            &text,
            registry,
        )?;
        let parent = path.parent().context("CONFIG_SCOPE: no parent directory")?;
        fs::create_dir_all(parent)?;
        let lock_path = parent.join(".oyzu-config-edit.lock");
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)?;
        lock.try_lock()
            .context("CONFIG_EDIT_CONFLICT: another edit is active")?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(text.as_bytes())?;
        temporary.as_file().sync_all()?;
        let current = match fs::read(path) {
            Ok(v) => Some(v),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        if current != self.original {
            bail!("CONFIG_EDIT_CONFLICT: source changed since it was read");
        }
        if let Ok(metadata) = fs::metadata(path) {
            temporary
                .as_file()
                .set_permissions(metadata.permissions())?;
        }
        temporary.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
}
#[derive(serde::Serialize)]
struct BTreeWrapper {
    value: toml::Value,
}
