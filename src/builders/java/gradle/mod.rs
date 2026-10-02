use crate::builders::task::insert;
mod metadata;
mod planning;
mod preparation;
mod testing;
#[cfg(test)]
mod tests;
use crate::builders::{
    Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile,
};
use crate::model::Target;
use anyhow::Result;
use std::path::Path;

pub(in crate::builders) struct Gradle;

const RUNTIME: &[RuntimeFile] = &[
    super::quality::RUNTIME,
    super::reporting::RUNTIME,
    RuntimeFile {
        name: "gradle-host.py",
        contents: include_str!("runtime/host.py"),
    },
    RuntimeFile {
        name: "host.gradle",
        contents: include_str!("runtime/host.gradle"),
    },
    RuntimeFile {
        name: "reporting.gradle",
        contents: include_str!("runtime/reporting.gradle"),
    },
    RuntimeFile {
        name: "gradle.py",
        contents: include_str!("runtime/adapter.py"),
    },
    RuntimeFile {
        name: "metadata.gradle",
        contents: include_str!("runtime/metadata.gradle"),
    },
    RuntimeFile {
        name: "integration.gradle",
        contents: include_str!("runtime/integration.gradle"),
    },
    RuntimeFile {
        name: "broker_transport.py",
        contents: crate::broker::RUNTIME,
    },
];

impl Builder for Gradle {
    fn development_test(
        &self,
        target: &Target,
        task: &crate::model::Task,
    ) -> Result<Option<crate::builders::TaskPlan>> {
        testing::development(target, task)
    }
    fn toolchain(&self, _target: &Target) -> Result<&'static str> {
        Ok("oyzu-toolchain/gradle:8.14.3-jdk17")
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
    fn development_command(
        &self,
        task: &crate::model::Task,
    ) -> Result<Option<crate::builders::DevelopmentCommand>> {
        super::quality::development(task)
    }
    fn runtime_files(&self) -> &'static [RuntimeFile] {
        RUNTIME
    }
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            tools: &["java"],
            ids: &["java/gradle"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        ["build.gradle", "build.gradle.kts"]
            .iter()
            .any(|file| path.join(file).is_file())
            .then_some("java/gradle")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        let path = target.path.clone();
        target.manager = "gradle".into();
        let executable = if path.join("gradlew").is_file() {
            if cfg!(windows) {
                "gradlew.bat"
            } else {
                "./gradlew"
            }
        } else {
            "gradle"
        };
        insert(target, "build", &[executable, "--no-daemon", "build"], true);
        insert(target, "test", &[executable, "--no-daemon", "test"], false);

        super::quality::discover(target)?;
        Ok(())
    }
}
