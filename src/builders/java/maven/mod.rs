use crate::builders::task::insert;
use crate::builders::{Builder, Descriptor};
use crate::model::Target;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub(in crate::builders) struct Maven;

impl Builder for Maven {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            ids: &["java/maven"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        ["pom.xml"]
            .iter()
            .any(|file| path.join(file).is_file())
            .then_some("java/maven")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        let path = target.path.clone();
        target.manager = "maven".into();
        let text = fs::read_to_string(path.join("pom.xml"))?;
        let doc = roxmltree::Document::parse(&text)?;
        if let Some(version) = doc
            .root_element()
            .children()
            .find(|v| v.has_tag_name("version"))
            .and_then(|v| v.text())
        {
            target.version = version.into();
        }
        let executable = if path.join("mvnw").is_file() {
            if cfg!(windows) {
                "mvnw.cmd"
            } else {
                "./mvnw"
            }
        } else {
            "mvn"
        };
        insert(
            target,
            "install",
            &[executable, "-B", "dependency:go-offline"],
            false,
        );
        insert(target, "build", &[executable, "-B", "verify"], true);
        insert(target, "test", &[executable, "-B", "test"], false);

        Ok(())
    }
}
