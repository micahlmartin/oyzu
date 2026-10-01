//! A builder's selected source files, scoped to one target in a captured workspace.
use super::portable;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize)]
#[serde(try_from = "Selection")]
pub(crate) struct Projection {
    root: String,
    files: BTreeSet<String>,
    #[serde(skip)]
    retained: BTreeSet<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    root: String,
    files: Vec<String>,
}

impl TryFrom<Selection> for Projection {
    type Error = anyhow::Error;

    fn try_from(selection: Selection) -> Result<Self> {
        Self::new(&selection.root, &selection.files)
    }
}

impl Projection {
    pub fn new(root: &str, files: &[String]) -> Result<Self> {
        if (root != "." && !portable(root))
            || files.len() > 100_000
            || files.iter().any(|file| !portable(file))
        {
            bail!("invalid builder source selection");
        }
        let mut retained = BTreeSet::new();
        for file in files {
            let full = if root == "." {
                file.clone()
            } else {
                format!("{root}/{file}")
            };
            let mut path = full.as_str();
            loop {
                retained.insert(path.to_string());
                match path.rsplit_once('/') {
                    Some((parent, _)) => path = parent,
                    None => break,
                }
            }
        }
        Ok(Self {
            root: root.into(),
            files: files.iter().cloned().collect(),
            retained,
        })
    }

    /// Other targets and workspace-level files remain available. Selected files'
    /// ancestors must survive directory pruning even for ignore negations.
    pub fn contains(&self, path: &str) -> bool {
        if self.root != "."
            && !path
                .strip_prefix(&self.root)
                .is_some_and(|rest| rest.starts_with('/'))
        {
            return true;
        }
        self.retained.contains(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scoped_capture_keeps_ancestors_and_siblings_but_omits_unselected_files() {
        let input = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        for file in [
            "image/Dockerfile",
            "image/bin/server",
            "image/ignored/keep.txt",
            "image/ignored/secret.txt",
            "image-other/data",
            "app/main.go",
            "build.yaml",
        ] {
            let path = input.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, file).unwrap();
        }
        let projection =
            Projection::new("image", &["Dockerfile".into(), "ignored/keep.txt".into()]).unwrap();
        let wire = serde_json::to_value(&projection).unwrap();
        assert!(wire.get("retained").is_none());
        let projection = serde_json::from_value(wire).unwrap();
        let captured = super::super::capture_projected(
            input.path(),
            &output.path().join("source"),
            &projection,
        )
        .unwrap();
        let paths: Vec<_> = captured.entries.iter().map(|e| e.path.as_str()).collect();
        for expected in [
            "image",
            "image/ignored",
            "image/ignored/keep.txt",
            "app/main.go",
            "image-other/data",
            "build.yaml",
        ] {
            assert!(paths.contains(&expected), "{expected}");
        }
        assert!(!paths.contains(&"image/bin"));
        assert!(!paths.contains(&"image/ignored/secret.txt"));
        assert!(input.path().join("image/bin/server").is_file());
    }

    #[test]
    fn root_selection_and_deserialization_validate_paths() {
        let projection = Projection::new(".", &["src/main.rs".into()]).unwrap();
        assert!(projection.contains("src"));
        assert!(projection.contains("src/main.rs"));
        assert!(!projection.contains("src/secret"));
        for (root, files) in [
            ("../outside", vec![]),
            (".", vec!["../secret"]),
            (".", vec!["C:/secret"]),
        ] {
            assert!(serde_json::from_value::<Projection>(
                serde_json::json!({"root":root,"files":files})
            )
            .is_err());
        }
        assert!(serde_json::from_value::<Projection>(
            serde_json::json!({"root":".","files":[],"unknown":true})
        )
        .is_err());
    }
}
