use super::Manager;
use crate::{
    broker,
    builders::{BuilderPlan, CommandSpec, PlanningContext, PreparationContext},
    dependencies::Prepared,
    records,
};
use anyhow::{bail, Result};

pub(super) struct Yarn;
impl Yarn {
    fn validate(&self, root: &std::path::Path) -> Result<bool> {
        let package = records::read(&root.join("package.json"))?;
        if package.get("workspaces").is_some()
            || [".yarnrc", ".yarnrc.yml", ".npmrc"]
                .iter()
                .any(|name| root.join(name).exists())
        {
            bail!("Yarn workspace/custom configuration capture is not implemented yet");
        }
        let lock = std::fs::read_to_string(root.join("yarn.lock"))?;
        if !lock.lines().any(|line| line.trim() == "# yarn lockfile v1") {
            bail!("Yarn capture requires a Classic v1 lockfile; modern Yarn integration remains pending");
        }
        let required = lock
            .lines()
            .any(|line| !line.trim().is_empty() && !line.trim().starts_with('#'));
        if !required
            && ["dependencies", "devDependencies", "optionalDependencies"]
                .iter()
                .any(|field| {
                    package[field]
                        .as_object()
                        .is_some_and(|values| !values.is_empty())
                })
        {
            bail!("Yarn dependencies require a current frozen lockfile for capture");
        }
        Ok(required)
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
        let sources = if self.validate(&context.target.path)? {
            vec![
                broker::Source::new("npm-public", "https://registry.npmjs.org/", None)?,
                broker::Source::new("yarn-public", "https://registry.yarnpkg.com/", None)?,
            ]
        } else {
            vec![]
        };
        super::registry::prepare(context, "yarn.mjs", "yarn.lock", sources)
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
