mod configuration;
use crate::builders::task::insert;
use crate::builders::task::unavailable;
mod metadata;
mod planning;
mod preparation;
#[cfg(test)]
mod tests;
use crate::builders::{Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext};
use crate::model::Target;
use anyhow::Result;
use std::path::Path;

pub(super) struct Docker;

impl Builder for Docker {
    fn register_settings(&self, registry: &mut crate::config::registry::Registry) -> Result<()> {
        configuration::register(registry)
    }
    fn acquisition_requires_network(&self) -> bool {
        false
    }
    fn executor_profile(&self) -> crate::executor::Profile {
        crate::executor::Profile::RootlessBuildkit
    }
    fn toolchain(&self, _target: &Target) -> Result<&'static str> {
        Ok("oyzu-toolchain/docker:buildkit0.25.0")
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
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            tools: &["docker"],
            ids: &["docker/image"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        ["Dockerfile"]
            .iter()
            .any(|file| path.join(file).is_file())
            .then_some("docker/image")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        target.manager = "docker".into();
        insert(target, "build", &["docker", "build", "."], true);
        unavailable(target, "test", "No image smoke-test contract is configured");
        unavailable(target, "lint", "No Dockerfile linter is configured");
        unavailable(
            target,
            "format-check",
            "No read-only Dockerfile formatter is configured",
        );

        Ok(())
    }
}
