//! Shared archive path admission and anchored destination traversal.
use super::{access, Bounds, Directory};
use anyhow::{ensure, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Paths<'a> {
    bounds: &'a Bounds,
    strip: Option<&'a str>,
    expanded: BTreeMap<String, (String, bool)>,
    explicit: BTreeSet<String>,
    name_bytes: usize,
}

impl<'a> Paths<'a> {
    pub(super) fn new(bounds: &'a Bounds, strip: Option<&'a str>) -> Self {
        Self {
            bounds,
            strip,
            expanded: BTreeMap::new(),
            explicit: BTreeSet::new(),
            name_bytes: 0,
        }
    }

    pub(super) fn charge(&mut self, length: usize) -> Result<()> {
        self.name_bytes = self
            .name_bytes
            .checked_add(length)
            .context("archive path size overflow")?;
        ensure!(
            self.name_bytes <= 32 * 1024 * 1024,
            "archive path bytes exceed limit"
        );
        Ok(())
    }

    /// None denotes an admitted empty strip-prefix ancestor. No file/link is
    /// created here; destination parents are opened/created without following links.
    pub(super) fn destination(
        &mut self,
        root: &Directory,
        raw: &str,
        is_directory: bool,
        empty_metadata: bool,
    ) -> Result<Option<(String, Directory, String)>> {
        let raw = if is_directory {
            raw.trim_end_matches('/')
        } else {
            raw
        };
        access::relative(raw)?;
        // Raw names remain in the duplicate set even when stripped or skipped.
        self.charge(raw.len())?;
        ensure!(
            self.explicit.insert(raw.to_owned()),
            "duplicate archive path"
        );
        ensure!(
            self.explicit.len() <= self.bounds.max_entries as usize,
            "archive entry limit exceeded"
        );
        let path = if let Some(strip) = self.strip {
            if raw == strip || strip.starts_with(&format!("{raw}/")) {
                ensure!(
                    is_directory && empty_metadata,
                    "strip-prefix ancestor is not an ordinary directory"
                );
                return Ok(None);
            }
            raw.strip_prefix(&format!("{strip}/"))
                .context("archive entry is outside strip-prefix")?
        } else {
            raw
        };
        let components: Vec<_> = path.split('/').collect();
        ensure!(
            components.len() <= self.bounds.max_depth as usize,
            "archive depth limit exceeded"
        );
        let mut parent = root.duplicate()?;
        let mut prefix = String::new();
        for (index, name) in components.iter().enumerate() {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(name);
            let directory = index + 1 < components.len() || is_directory;
            let folded = prefix.to_uppercase();
            if let Some((spelling, was_directory)) = self.expanded.get(&folded) {
                ensure!(
                    spelling == &prefix && *was_directory && directory,
                    "archive path collision"
                );
            } else {
                self.charge(prefix.len())?;
                self.expanded.insert(folded, (prefix.clone(), directory));
                ensure!(
                    self.expanded.len() <= self.bounds.max_entries as usize,
                    "expanded archive entry limit exceeded"
                );
            }
            if directory {
                parent = parent.create_directory(name)?;
            }
        }
        Ok(Some((
            path.to_owned(),
            parent,
            components.last().unwrap().to_string(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripped_ancestor_names_consume_the_shared_name_budget() {
        let temporary = tempfile::tempdir().unwrap();
        let root = Directory::open(&temporary.path().canonicalize().unwrap()).unwrap();
        let bounds = Bounds::default();
        let mut paths = Paths::new(&bounds, Some("prefix"));
        paths.charge(32 * 1024 * 1024).unwrap();
        assert!(paths.destination(&root, "prefix", true, true).is_err());
        assert!(root.entries().unwrap().is_empty());
    }
}
