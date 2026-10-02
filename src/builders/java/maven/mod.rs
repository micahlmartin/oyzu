use crate::builders::task::insert;
mod metadata;
mod planning;
mod preparation;
#[cfg(test)]
mod tests;
use crate::builders::{
    Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile,
};
use crate::model::Target;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub(in crate::builders) struct Maven;

const RUNTIME: &[RuntimeFile] = &[
    RuntimeFile {
        name: "maven_reporting.py",
        contents: include_str!("runtime/reporting.py"),
    },
    RuntimeFile {
        name: "maven.py",
        contents: include_str!("runtime/adapter.py"),
    },
    RuntimeFile {
        name: "broker_transport.py",
        contents: crate::broker::RUNTIME,
    },
];

impl Builder for Maven {
    fn toolchain(&self, _target: &Target) -> Result<&'static str> {
        Ok("oyzu-toolchain/maven:3.9.11-jdk17")
    }
    fn prepare(
        &self,
        context: PreparationContext<'_>,
    ) -> Result<Option<crate::dependencies::Prepared>> {
        preparation::prepare(context).map(Some)
    }
    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        planning::plan(context)
    }
    fn runtime_files(&self) -> &'static [RuntimeFile] {
        RUNTIME
    }
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            tools: &["java"],
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
