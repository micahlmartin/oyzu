use super::{empty, Manager};
use crate::{
    builders::{BuilderPlan, CommandSpec, PlanningContext, PreparationContext},
    dependencies::Prepared,
};
use anyhow::{bail, Result};

pub(super) struct Yarn;
impl Yarn {
    fn validate(&self, root: &std::path::Path) -> Result<()> {
        empty::validate(root)?;
        let lock = std::fs::read_to_string(root.join("yarn.lock"))?;
        if !lock.lines().any(|line| line.trim() == "# yarn lockfile v1")
            || lock
                .lines()
                .any(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        {
            bail!("only dependency-free Yarn Classic locks are implemented; modern Yarn and registry capture remain pending");
        }
        Ok(())
    }
}
impl Manager for Yarn {
    fn id(&self) -> &'static str {
        "yarn"
    }
    fn image(&self) -> &'static str {
        "oyzu-toolchain/node:yarn1.22.22-node22"
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        self.validate(&context.target.path)?;
        empty::prepare(context, "yarn.mjs", "yarn.lock")
    }
    fn configure(&self, context: &PlanningContext<'_>, plan: &mut BuilderPlan) -> Result<()> {
        self.validate(&context.target.path)?;
        super::require_prepared(context)?;
        plan.env.insert("YARN_IGNORE_PATH".into(), "1".into());
        plan.env
            .insert("COREPACK_ENABLE_NETWORK".into(), "0".into());
        plan.prepare.push(CommandSpec::new(
            "prepare",
            &["node", "/oyzu/yarn.mjs", "install", "/dependencies", "."],
        ));
        Ok(())
    }
    fn package(&self, target: &str, filename: &str) -> CommandSpec {
        CommandSpec::new(
            "package",
            &[
                "sh",
                "-c",
                "yarn --offline --non-interactive pack --filename \"$1\" && node /oyzu/node-archive.mjs \"$1\" /opt/oyzu-yarn/node_modules/tar-stream",
                "oyzu-yarn-pack",
                &format!("/out/{target}/artifacts/{filename}"),
            ],
        )
    }
}
