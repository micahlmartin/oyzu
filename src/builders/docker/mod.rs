mod configuration;
use crate::builders::task::insert;
use crate::builders::task::unavailable;
mod metadata;
mod planning;
mod preparation;
mod quality;
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
        // Captured bases are local. Optional package stores are separately
        // admitted by dependencies::context before their broker is started.
        false
    }
    fn executor_profile(&self) -> crate::executor::Profile {
        crate::executor::Profile::RootlessBuildkit
    }
    fn toolchain(&self, _target: &Target) -> Result<&'static str> {
        Ok("oyzu-toolchain/docker:buildkit0.25.0")
    }
    fn execution_platform(
        &self,
        _requested: Option<&str>,
    ) -> Result<Option<crate::platform::Platform>> {
        // The assembly worker need not execute target code. Preparation checks
        // native RUN requirements before admitting actions.
        Ok(None)
    }
    fn target_platform(
        &self,
        requested: Option<&str>,
        image: &crate::executor::Image,
    ) -> Result<crate::platform::Platform> {
        let target = crate::platform::Platform::requested(requested, &image.platform()?)?;
        if target.os() != "linux" || !matches!(target.arch(), "amd64" | "arm64") {
            anyhow::bail!("Docker target platform {target} is not supported; expected linux/amd64 or linux/arm64");
        }
        Ok(target)
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
        unavailable(target, "test", "Default OCI validation runs during oyzu build; direct image test bundles are not implemented yet");
        quality::discover(target)?;

        Ok(())
    }
}
