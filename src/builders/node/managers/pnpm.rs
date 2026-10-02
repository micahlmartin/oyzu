use super::Manager;
mod patches;
use crate::{
    broker,
    builders::{BuilderPlan, CommandSpec, PlanningContext, PreparationContext},
    dependencies::Prepared,
    records,
};
use anyhow::{bail, Context, Result};

pub(super) struct Pnpm;
impl Pnpm {
    fn validate(&self, root: &std::path::Path) -> Result<bool> {
        let package = records::read(&root.join("package.json"))?;
        if package.get("workspaces").is_some()
            || [".npmrc", ".pnpmfile.cjs"]
                .iter()
                .any(|name| root.join(name).exists())
        {
            bail!("pnpm workspace/custom configuration capture is not implemented yet");
        }
        let lock: serde_yaml::Value =
            serde_yaml::from_str(&std::fs::read_to_string(root.join("pnpm-lock.yaml"))?)?;
        let importers = lock["importers"]
            .as_mapping()
            .context("pnpm lock missing importers")?;
        if importers.len() != 1
            || importers
                .get(serde_yaml::Value::String(".".into()))
                .is_none_or(|v| v.as_mapping().is_none())
        {
            bail!("pnpm workspace/dependency capture is not implemented yet");
        }
        for field in ["dependencies", "devDependencies", "optionalDependencies"] {
            if let Some(dependencies) = package[field].as_object() {
                for (name, specifier) in dependencies {
                    if lock["importers"]["."][field][name]["specifier"].as_str()
                        != specifier.as_str()
                    {
                        bail!("pnpm capture requires a current frozen lockfile for {name}");
                    }
                }
            }
        }
        Ok(lock
            .get("packages")
            .is_some_and(|packages| packages.as_mapping().is_some_and(|p| !p.is_empty())))
    }
}
impl Manager for Pnpm {
    fn id(&self) -> &'static str {
        "pnpm"
    }
    fn image(&self) -> &'static str {
        "oyzu-toolchain/node:pnpm10.11.0-node22"
    }
    fn runtime_image(&self, node: &str) -> Result<String> {
        super::runtime_image(self.image(), node)
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        let required = self.validate(&context.target.path)?;
        let sources = if required {
            vec![broker::Source::new(
                "npm-public",
                "https://registry.npmjs.org/",
                None,
            )?]
        } else {
            vec![]
        };
        let root = context.target.path.clone();
        let mut prepared = super::registry::prepare(
            context,
            self.id(),
            "pnpm.mjs",
            "pnpm-lock.yaml",
            super::registry::Acquisition::Build,
            sources,
        )?;
        if let Some(prepared) = &mut prepared {
            patches::record(&root, prepared)?;
        }
        Ok(prepared)
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
