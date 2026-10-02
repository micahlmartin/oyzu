mod development;
mod metadata;
mod packaging;
mod planning;
mod preparation;
mod testing;
use crate::builders::task::insert;
use crate::builders::{Builder, Descriptor};
use crate::model::Target;
use anyhow::Result;
use std::path::Path;

pub(super) struct Go;

impl crate::dependencies::context::Provider for Go {
    fn id(&self) -> &'static str {
        "go/modules"
    }
    fn tools(&self) -> &'static [&'static str] {
        &["go"]
    }
    fn detect(&self, source: &Path) -> bool {
        source.join("go.mod").is_file() || source.join("go.work").is_file()
    }
    fn store(&self) -> &'static str {
        "modules"
    }
    fn prepare(
        &self,
        context: super::PreparationContext<'_>,
    ) -> Result<crate::dependencies::Prepared> {
        preparation::prepare_context(context)
    }
}

static RUNTIME: &[super::RuntimeFile] = &[
    super::RuntimeFile {
        name: "go-metadata.go",
        contents: include_str!("runtime/metadata.go"),
    },
    super::RuntimeFile {
        name: "go-acquisition.go",
        contents: include_str!("runtime/acquisition.go"),
    },
    super::RuntimeFile {
        name: "broker-transport.go",
        contents: include_str!("../../broker/runtime/transport.go"),
    },
];

impl Builder for Go {
    fn dependency_providers(
        &self,
    ) -> &'static [&'static dyn crate::dependencies::context::Provider] {
        &[&Go]
    }
    fn development_test(
        &self,
        target: &Target,
        task: &crate::model::Task,
    ) -> Result<Option<super::TaskPlan>> {
        Ok(testing::development(target, task))
    }
    fn development_command(
        &self,
        task: &crate::model::Task,
    ) -> Result<Option<super::DevelopmentCommand>> {
        development::command(task)
    }
    fn prepare(
        &self,
        context: super::PreparationContext<'_>,
    ) -> Result<Option<crate::dependencies::Prepared>> {
        preparation::prepare(context)
    }

    fn runtime_files(&self) -> &'static [super::RuntimeFile] {
        RUNTIME
    }
    fn toolchain(&self, _target: &Target) -> Result<&'static str> {
        Ok("oyzu-toolchain/go:1.24-mod0.25.0")
    }
    fn plan(&self, context: super::PlanningContext<'_>) -> Result<super::BuilderPlan> {
        planning::plan(context)
    }
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            tools: &["go"],
            ids: &["go/app", "go/library"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        ["go.mod", "go.work"]
            .iter()
            .any(|file| path.join(file).is_file())
            .then_some("go/app")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        target.manager = "go".into();
        insert(target, "install", &["go", "mod", "download"], false);
        insert(target, "build", &["go", "build", "./..."], true);
        insert(target, "test", &["go", "test", "./..."], true);
        insert(target, "lint", &["go", "vet", "./..."], true);
        insert(target, "format", &["gofmt", "-w", "."], false);
        target.tasks.get_mut("format").unwrap().mutates_source = true;
        insert(target, "format-check", &["gofmt", "-l", "."], true);
        target
            .tasks
            .get_mut("format-check")
            .unwrap()
            .stdout_must_be_empty = true;

        Ok(())
    }
}
