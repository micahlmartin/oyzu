use crate::builders::task::insert;
use crate::builders::{Builder, Descriptor};
use crate::model::Target;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub(super) struct Helm;

impl Builder for Helm {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            ids: &["helm/chart"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        ["Chart.yaml"]
            .iter()
            .any(|file| path.join(file).is_file())
            .then_some("helm/chart")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        let path = target.path.clone();
        target.manager = "helm".into();
        let value: serde_yaml::Value =
            serde_yaml::from_str(&fs::read_to_string(path.join("Chart.yaml"))?)?;
        if let Some(version) = value.get("version").and_then(|v| v.as_str()) {
            target.version = version.into();
        }
        insert(
            target,
            "install",
            &["helm", "dependency", "build", "."],
            false,
        );
        insert(target, "build", &["helm", "package", "."], true);
        insert(target, "lint", &["helm", "lint", "."], true);
        insert(
            target,
            "test",
            &["helm", "template", "oyzu-check", "."],
            true,
        );

        Ok(())
    }
}
