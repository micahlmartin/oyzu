mod planning;
use crate::builders::task::insert;
use crate::builders::{Builder, Descriptor};
use crate::model::Target;
use anyhow::Result;
use std::path::Path;

pub(super) struct Go;

impl Builder for Go {
    fn acquisition_requires_network(&self) -> bool {
        false
    }
    fn toolchain(&self, _target: &Target) -> Result<&'static str> {
        Ok("golang:1.24-bookworm")
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
