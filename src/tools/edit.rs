//! Lossless lock proposals and cooperative workspace publication. Resolution,
//! source admission and installation are caller responsibilities; no network or
//! tool execution occurs here. Frozen consumers never use this mutation API.
use super::lock;
use anyhow::{ensure, Context, Result};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use toml_edit::{DocumentMut, Item, Table};

/// Captured source for an explicit lock update. Capture validates an existing
/// format-2 graph; missing files are allowed, but migration is not implicit.
pub struct ToolLockEdit {
    path: PathBuf,
    original: Option<Vec<u8>>,
}

/// Validated proposal bound to the captured bytes. Inspect `preview` before
/// installation; commit only after all caller-required verification/install work
/// succeeds. This object is not a backend or policy authorization.
pub struct ToolLockProposal {
    source: ToolLockEdit,
    preview: Vec<u8>,
    changes: Vec<ToolLockChange>,
}

/// Semantic changes only; formatting and collection ordering are excluded.
#[derive(Debug, serde::Serialize)]
#[serde(tag = "record", rename_all = "kebab-case")]
pub enum ToolLockChange {
    Environment {
        scope: String,
        profile: String,
        change: ToolLockChangeKind,
    },
    Tool {
        key: String,
        change: ToolLockChangeKind,
    },
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolLockChangeKind {
    Added,
    Changed,
    Removed,
}

impl ToolLockEdit {
    pub fn capture(path: &Path) -> Result<Self> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let path = parent
            .canonicalize()?
            .join(path.file_name().context("tool lock needs a file name")?);
        let original = read_source(&path)?;
        if let Some(bytes) = &original {
            lock::parse(bytes)?;
        }
        Ok(Self { path, original })
    }

    /// Candidate is the complete intended lock, not a patch. Validates the whole
    /// graph and retains the original spelling/comments of semantically unchanged
    /// records. Removed records are explicitly absent from the candidate.
    pub fn propose(self, candidate: &[u8]) -> Result<ToolLockProposal> {
        let proposed = lock::parse(candidate)?;
        let expected = serde_json::to_value(&proposed)?;
        let previous = self
            .original
            .as_deref()
            .map(lock::parse)
            .transpose()?
            .map(serde_json::to_value)
            .transpose()?;
        let mut changes = Vec::new();
        for name in ["environment", "tool"] {
            let empty = serde_json::json!([]);
            let before =
                indexed_values(name, previous.as_ref().map(|v| &v[name]).unwrap_or(&empty))?;
            let after = indexed_values(name, &expected[name])?;
            for id in before
                .keys()
                .chain(after.keys())
                .collect::<std::collections::BTreeSet<_>>()
            {
                if before.get(id) == after.get(id) {
                    continue;
                }
                let change = match (before.contains_key(id), after.contains_key(id)) {
                    (false, _) => ToolLockChangeKind::Added,
                    (_, false) => ToolLockChangeKind::Removed,
                    _ => ToolLockChangeKind::Changed,
                };
                changes.push(if name == "tool" {
                    ToolLockChange::Tool {
                        key: id.0.clone(),
                        change,
                    }
                } else {
                    ToolLockChange::Environment {
                        scope: id.0.clone(),
                        profile: id.1.clone(),
                        change,
                    }
                });
            }
        }
        let preview = if changes.is_empty() && self.original.is_some() {
            self.original.clone().unwrap()
        } else if let Some(original) = &self.original {
            let old = lock::parse(original)?;
            let mut document: DocumentMut = std::str::from_utf8(original)?.parse()?;
            let replacement: DocumentMut = std::str::from_utf8(candidate)?.parse()?;
            for (name, old_values, new_values) in [
                (
                    "environment",
                    serde_json::to_value(old.environment)?,
                    serde_json::to_value(proposed.environment)?,
                ),
                (
                    "tool",
                    serde_json::to_value(old.tool)?,
                    serde_json::to_value(proposed.tool)?,
                ),
            ] {
                let old_values = indexed_values(name, &old_values)?;
                let new_values = indexed_values(name, &new_values)?;
                // Preserve even inline-array syntax and all document trivia on
                // a semantic no-op, including collection order.
                if old_values == new_values {
                    continue;
                }
                reconcile(
                    &mut document[name],
                    &replacement[name],
                    name,
                    &old_values,
                    &new_values,
                )?;
            }
            document.to_string().into_bytes()
        } else {
            candidate.to_vec()
        };
        // The lossless editor is not an independent graph validator.
        ensure!(
            serde_json::to_value(lock::parse(&preview)?)? == expected,
            "lock editor changed proposed graph semantics"
        );
        Ok(ToolLockProposal {
            source: self,
            preview,
            changes,
        })
    }
}

type RecordId = (String, String);
fn value_id(name: &str, value: &serde_json::Value) -> Result<RecordId> {
    let string = |field: &str| {
        value[field]
            .as_str()
            .map(str::to_owned)
            .context("missing lock record identity")
    };
    if name == "tool" {
        Ok((string("key")?, String::new()))
    } else {
        Ok((string("scope")?, string("profile")?))
    }
}
fn indexed_values(
    name: &str,
    values: &serde_json::Value,
) -> Result<BTreeMap<RecordId, serde_json::Value>> {
    values
        .as_array()
        .context("lock records must be arrays")?
        .iter()
        .map(|v| Ok((value_id(name, v)?, v.clone())))
        .collect()
}
fn table_id(name: &str, table: &Table) -> Result<RecordId> {
    let string = |field: &str| {
        table
            .get(field)
            .and_then(Item::as_str)
            .map(str::to_owned)
            .context("missing lock table identity")
    };
    if name == "tool" {
        Ok((string("key")?, String::new()))
    } else {
        Ok((string("scope")?, string("profile")?))
    }
}
fn tables(item: &Item) -> Result<Vec<Table>> {
    if let Some(array) = item.as_array_of_tables() {
        return Ok(array.iter().cloned().collect());
    }
    item.as_array()
        .context("lock collection must be an array")?
        .iter()
        .map(|v| {
            Ok(v.as_inline_table()
                .context("lock record must be a table")?
                .clone()
                .into_table())
        })
        .collect()
}
fn reconcile(
    original: &mut Item,
    candidate: &Item,
    name: &str,
    old_values: &BTreeMap<RecordId, serde_json::Value>,
    new_values: &BTreeMap<RecordId, serde_json::Value>,
) -> Result<()> {
    let mut incoming = tables(candidate)?
        .into_iter()
        .map(|t| Ok((table_id(name, &t)?, t)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut output = Vec::new();
    for table in tables(original)? {
        let id = table_id(name, &table)?;
        if let Some(replacement) = incoming.remove(&id) {
            output.push(if old_values.get(&id) == new_values.get(&id) {
                table
            } else {
                replacement
            });
        }
    }
    output.extend(incoming.into_values());
    if let Some(array) = original.as_array_mut() {
        // Keep inline collection syntax and its surrounding decoration.
        array.clear();
        for table in output {
            array.push(table.into_inline_table());
        }
    } else if output.is_empty() {
        *original = Item::Value(toml_edit::Value::Array(toml_edit::Array::new()));
    } else {
        let mut array = toml_edit::ArrayOfTables::new();
        for table in output {
            array.push(table);
        }
        *original = Item::ArrayOfTables(array);
    }
    Ok(())
}

impl ToolLockProposal {
    pub fn changes(&self) -> &[ToolLockChange] {
        &self.changes
    }
    pub fn preview(&self) -> &[u8] {
        &self.preview
    }

    /// Serialize cooperating writers with a permanent sibling lock. Recheck exact
    /// captured bytes immediately before atomic replacement. This is optimistic
    /// concurrency, not protection against a malicious same-user process racing
    /// the final comparison or replacing the parent directory.
    pub fn commit(self) -> Result<()> {
        let parent = self
            .source
            .path
            .parent()
            .context("tool lock has no parent")?;
        let mutex = open_regular(&parent.join(".oyzu-tool-lock-edit.lock"), true)?;
        mutex
            .try_lock()
            .context("TOOL_LOCK_EDIT_CONFLICT: another lock edit is active")?;
        let _guard = Unlock(mutex);
        ensure!(
            read_source(&self.source.path)
                .context("TOOL_LOCK_EDIT_CONFLICT: cannot revalidate source")?
                == self.source.original,
            "TOOL_LOCK_EDIT_CONFLICT: lock changed since capture"
        );
        if self.source.original.as_deref() == Some(&self.preview) {
            return Ok(());
        }
        let permissions = fs::metadata(&self.source.path)
            .ok()
            .map(|m| m.permissions());
        #[cfg(windows)]
        ensure!(
            !permissions.as_ref().is_some_and(fs::Permissions::readonly),
            "TOOL_LOCK_EDIT_CONFLICT: source is read-only"
        );
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(&self.preview)?;
        if let Some(permissions) = permissions {
            temporary.as_file().set_permissions(permissions)?;
        }
        temporary.as_file().sync_all()?;
        let started = std::time::Instant::now();
        loop {
            ensure!(
                read_source(&self.source.path)
                    .context("TOOL_LOCK_EDIT_CONFLICT: cannot revalidate source")?
                    == self.source.original,
                "TOOL_LOCK_EDIT_CONFLICT: lock changed before publication"
            );
            let published = if self.source.original.is_none() {
                temporary.persist_noclobber(&self.source.path)
            } else {
                temporary.persist(&self.source.path)
            };
            match published {
                Ok(_) => break,
                Err(error) => {
                    #[cfg(windows)]
                    // MoveFileEx can report ACCESS_DENIED for an open target
                    // without FILE_SHARE_DELETE. Read-only sources were rejected
                    // before staging; other denial still has the same short cap.
                    let retry = matches!(error.error.raw_os_error(), Some(5 | 32 | 33))
                        && started.elapsed() < std::time::Duration::from_secs(2);
                    #[cfg(not(windows))]
                    let retry = {
                        let _ = started;
                        false
                    };
                    if !retry {
                        return Err(error.error)
                            .context("TOOL_LOCK_EDIT_CONFLICT: atomic lock replacement failed");
                    }
                    temporary = error.file;
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
            }
        }
        #[cfg(unix)]
        File::open(parent)?
            .sync_all()
            .context("lock published but directory sync failed")?;
        Ok(())
    }
}
struct Unlock(File);
impl Drop for Unlock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn open_regular(path: &Path, create: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(create)
        .create(create)
        .truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "tool lock edit requires a regular non-symlink file"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            metadata.file_attributes()
                & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
                == 0,
            "tool lock edit rejects reparse points"
        );
    }
    Ok(file)
}
fn read_source(path: &Path) -> Result<Option<Vec<u8>>> {
    let file = match open_regular(path, false) {
        Ok(file) => file,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(None)
        }
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(lock::MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= lock::MAX_BYTES, "tool lock exceeds 8 MiB");
    Ok(Some(bytes))
}
