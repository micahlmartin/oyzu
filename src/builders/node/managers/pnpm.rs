use super::{empty, Manager};
use crate::{
    builders::{BuilderPlan, CommandSpec, PlanningContext, PreparationContext},
    dependencies::Prepared,
};
use anyhow::{bail, Context, Result};

pub(super) struct Pnpm;
impl Pnpm {
    fn validate(&self, root: &std::path::Path) -> Result<()> {
        empty::validate(root)?;
        let lock: serde_yaml::Value =
            serde_yaml::from_str(&std::fs::read_to_string(root.join("pnpm-lock.yaml"))?)?;
        for field in ["packages", "snapshots"] {
            if lock
                .get(field)
                .is_some_and(|v| v.as_mapping().is_none_or(|m| !m.is_empty()))
            {
                bail!("pnpm dependency capture is not implemented yet");
            }
        }
        let importers = lock["importers"]
            .as_mapping()
            .context("pnpm lock missing importers")?;
        if importers.len() != 1
            || !importers
                .get(serde_yaml::Value::String(".".into()))
                .is_some_and(|v| v.as_mapping().is_some_and(|m| m.is_empty()))
        {
            bail!("pnpm workspace/dependency capture is not implemented yet");
        }
        Ok(())
    }
}
impl Manager for Pnpm {
    fn id(&self) -> &'static str {
        "pnpm"
    }
    fn image(&self) -> &'static str {
        "oyzu-toolchain/node:pnpm10.11.0-node22"
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        self.validate(&context.target.path)?;
        empty::prepare(context, "pnpm.mjs", "pnpm-lock.yaml")
    }
    fn configure(&self, context: &PlanningContext<'_>, plan: &mut BuilderPlan) -> Result<()> {
        self.validate(&context.target.path)?;
        super::require_prepared(context)?;
        plan.env.insert(
            "npm_config_manage_package_manager_versions".into(),
            "false".into(),
        );
        plan.env
            .insert("COREPACK_ENABLE_NETWORK".into(), "0".into());
        plan.prepare.push(CommandSpec::new(
            "prepare",
            &["node", "/oyzu/pnpm.mjs", "install", "/dependencies", "."],
        ));
        Ok(())
    }
    fn package(&self, target: &str, filename: &str) -> CommandSpec {
        CommandSpec::new(
            "package",
            &[
                "pnpm",
                "--config.manage-package-manager-versions=false",
                "pack",
                "--out",
                &format!("/out/{target}/artifacts/{filename}"),
            ],
        )
    }
}
