use crate::builders::task::insert;
use crate::builders::{Builder, Descriptor};
use crate::model::Target;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub(in crate::builders) struct Ant;

impl Builder for Ant {
    fn descriptor(&self) -> Descriptor {
        Descriptor { ids: &["java/ant"] }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        ["build.xml"]
            .iter()
            .any(|file| path.join(file).is_file())
            .then_some("java/ant")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        let path = target.path.clone();
        target.manager = "ant".into();
        let text = fs::read_to_string(path.join("build.xml"))?;
        let doc = roxmltree::Document::parse(&text)?;
        let default = doc.root_element().attribute("default");
        for element in doc
            .root_element()
            .children()
            .filter(|v| v.has_tag_name("target"))
        {
            if let Some(name) = element.attribute("name") {
                insert(
                    target,
                    name,
                    &["ant", name],
                    Some(name) == default || name == "test",
                );
            }
        }

        Ok(())
    }
}
