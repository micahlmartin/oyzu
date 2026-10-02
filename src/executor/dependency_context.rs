//! One captured package-store context for custom Dockerfiles. Adapters own its
//! credential-free layout and native compatibility; execution owns containment.
use crate::{platform::Platform, snapshot};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DependencyContext {
    pub store: String,
    pub tree_digest: String,
    pub platform: Platform,
}

impl DependencyContext {
    pub(super) fn validate(&self) -> Result<()> {
        if !self.store.starts_with("contexts/")
            || !snapshot::portable(&self.store)
            || self.tree_digest.len() != 71
            || !self.tree_digest.starts_with("sha256:")
            || !self.tree_digest[7..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            bail!("invalid captured dependency context");
        }
        Ok(())
    }

    /// Copy only the declared store before native tooling can read it. The
    /// original prepared tree is never mounted into the BuildKit worker.
    pub(super) fn capture(&self, root: &Path, destination: &Path, target: &Platform) -> Result<()> {
        self.validate()?;
        if &self.platform != target {
            bail!("dependency context platform differs from image target");
        }
        let source = super::files::input(root, &self.store)?;
        if !source.is_dir() {
            bail!("dependency context requires a captured directory");
        }
        // BuildKit applies a named local context's own ignore file. A prepared
        // store must be exact; silently filtering its contents breaks replay.
        if source.join(".dockerignore").symlink_metadata().is_ok() {
            bail!("dependency context must not contain a root .dockerignore");
        }
        let copied = snapshot::capture_prepared(&source, destination)?;
        if copied.digest != self.tree_digest {
            bail!("dependency context changed after planning");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn contexts_bind_exact_subtrees_platforms_and_digest_without_exposing_siblings() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("contexts/packages");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("package.whl"), "prepared package bytes").unwrap();
        fs::write(root.path().join("session-private"), "not a context input").unwrap();
        let before = snapshot::inspect_tree(root.path()).unwrap().digest;
        let context = DependencyContext {
            store: "contexts/packages".into(),
            tree_digest: snapshot::inspect_tree(&source).unwrap().digest,
            platform: "linux/amd64".parse().unwrap(),
        };
        let private = tempfile::tempdir().unwrap();
        let copy = private.path().join("copy");
        context
            .capture(root.path(), &copy, &context.platform)
            .unwrap();
        assert_eq!(
            snapshot::inspect_tree(&copy).unwrap().digest,
            context.tree_digest
        );
        assert!(!copy.join("session-private").exists());
        assert_eq!(snapshot::inspect_tree(root.path()).unwrap().digest, before);
        assert!(context
            .capture(
                root.path(),
                &private.path().join("wrong-platform"),
                &"linux/arm64".parse().unwrap()
            )
            .is_err());
        assert!(!private.path().join("wrong-platform").exists());
        fs::write(source.join("package.whl"), "tampered").unwrap();
        assert!(context
            .capture(
                root.path(),
                &private.path().join("tampered"),
                &context.platform
            )
            .unwrap_err()
            .to_string()
            .contains("changed after planning"));
        fs::write(source.join(".dockerignore"), "package.whl\n").unwrap();
        assert!(context
            .capture(
                root.path(),
                &private.path().join("ignored"),
                &context.platform
            )
            .is_err());
        for store in [
            "../escape",
            "contexts/../escape",
            "images/base-0",
            "contexts/",
        ] {
            let mut invalid = context.clone();
            invalid.store = store.into();
            assert!(invalid.validate().is_err());
        }
    }
}
