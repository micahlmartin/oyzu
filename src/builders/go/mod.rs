mod development;
mod metadata;
mod planning;
mod preparation;
use crate::builders::task::insert;
use crate::builders::{Builder, Descriptor};
use crate::model::Target;
use anyhow::Result;
use std::path::Path;

pub(super) struct Go;

static RUNTIME: &[super::RuntimeFile] = &[super::RuntimeFile {
    name: "go-metadata.go",
    contents: include_str!("runtime/metadata.go"),
}];

impl Builder for Go {
    fn development_argv(&self, task: &crate::model::Task) -> Result<Option<Vec<String>>> {
        development::argv(task)
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
        Ok("golang:1.24-bookworm")
    }
    fn plan(&self, context: super::PlanningContext<'_>) -> Result<super::BuilderPlan> {
        planning::plan(context)
    }
    fn descriptor(&self) -> Descriptor {
        Descriptor {
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
