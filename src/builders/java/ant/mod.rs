use crate::builders::task::insert;
mod metadata;
mod planning;
mod preparation;
mod reporting;
#[cfg(test)]
mod tests;
use crate::builders::{
    Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile,
};
use crate::dependencies::Prepared;
use crate::model::Target;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub(in crate::builders) struct Ant;

const RUNTIME: &[RuntimeFile] = &[
    RuntimeFile {
        name: "AntMetadata.java",
        contents: include_str!("runtime/AntMetadata.java"),
    },
    RuntimeFile {
        name: "JarPackaging.java",
        contents: include_str!("runtime/JarPackaging.java"),
    },
    RuntimeFile {
        name: "AntTesting.java",
        contents: include_str!("runtime/AntTesting.java"),
    },
    RuntimeFile {
        name: "AntCoverage.java",
        contents: include_str!("runtime/AntCoverage.java"),
    },
    RuntimeFile {
        name: "ant-test.sh",
        contents: include_str!("runtime/testing.sh"),
    },
];

impl Builder for Ant {
    fn toolchain(&self, _target: &Target) -> Result<&'static str> {
        Ok("oyzu-toolchain/ant:1.10.18-jdk17")
    }
    fn runtime_files(&self) -> &'static [RuntimeFile] {
        RUNTIME
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        preparation::prepare(context).map(Some)
    }
    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        planning::plan(context)
    }
    fn instrument_override(
        &self,
        _target: &Target,
        task: &crate::model::Task,
        env: &std::collections::BTreeMap<String, String>,
    ) -> Option<Vec<String>> {
        reporting::instrument(task, env)
    }
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
                    matches!(
                        name,
                        "build" | "compile" | "test" | "lint" | "format-check" | "jar"
                    ),
                );
            }
        }

        if !target.tasks.contains_key("build") {
            let command = if target.tasks.contains_key("compile") {
                "compile"
            } else {
                default.unwrap_or("")
            };
            if !command.is_empty() {
                insert(target, "build", &["ant", command], true);
            }
        }
        let package = if target.tasks.contains_key("jar") {
            "jar"
        } else {
            default.unwrap_or("")
        };
        if !package.is_empty() {
            insert(target, "archive", &["ant", package], true);
        }
        Ok(())
    }
}
