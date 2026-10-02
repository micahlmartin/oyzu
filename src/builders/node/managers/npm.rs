mod preparation;
mod workspace;
use super::Manager;
use crate::{
    builders::{BuilderPlan, CommandSpec, PlanningContext, PreparationContext},
    dependencies::Prepared,
    records,
};
use anyhow::{bail, Result};

pub(super) struct Npm;
impl Manager for Npm {
    fn development_command(
        &self,
        task: &crate::model::Task,
    ) -> Result<Option<crate::builders::DevelopmentCommand>> {
        workspace::development_command(task)
    }
    fn workspace_plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        workspace::plan(context)
    }
    fn id(&self) -> &'static str {
        "npm"
    }
    fn image(&self) -> &'static str {
        "oyzu-toolchain/node:npm11.11.0-node22"
    }
    fn runtime_image(&self, node: &str) -> Result<String> {
        super::runtime_image(self.image(), node)
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        preparation::prepare(context)
    }
    fn configure(&self, context: &PlanningContext<'_>, plan: &mut BuilderPlan) -> Result<()> {
        let package = records::read(&context.target.path.join("package.json"))?;
        let required = preparation::required(&context.target.path, &package)?;
        if required && context.dependencies.is_none() {
            bail!(
                "{}: npm dependency capture is required before planning execution",
                context.target.name
            );
        }
        for (name, value) in [
            ("npm_config_offline", "true"),
            ("npm_config_audit", "false"),
            ("npm_config_fund", "false"),
            ("npm_config_cache", "/tmp/npm-cache"),
        ] {
            plan.env.insert(name.into(), value.into());
        }
        let command = if context.dependencies.is_some() {
            CommandSpec::new(
                "prepare",
                &["node", "/oyzu/npm.mjs", "install", "/dependencies", "."],
            )
        } else {
            let install = if preparation::lockfile(&context.target.path).is_some() {
                "ci"
            } else {
                "install"
            };
            CommandSpec::new(
                "prepare",
                &["npm", install, "--offline", "--ignore-scripts"],
            )
        };
        plan.prepare.push(command);
        Ok(())
    }
    fn package(&self, target: &str, _filename: &str) -> CommandSpec {
        CommandSpec::new(
            "package",
            &[
                "npm",
                "pack",
                "--ignore-scripts",
                "--pack-destination",
                &format!("/out/{target}/artifacts"),
            ],
        )
    }
    fn script(&self, name: &str, forward_arguments: bool) -> Vec<String> {
        let mut argv = vec!["npm".into(), "run".into(), name.into()];
        if forward_arguments {
            argv.push("--".into());
        }
        argv
    }
}
