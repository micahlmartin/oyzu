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
    fn capture(
        &self,
        context: PreparationContext<'_>,
        acquisition: super::registry::Acquisition,
    ) -> Result<Option<Prepared>> {
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
            Manager::id(self),
            "pnpm.mjs",
            "pnpm-lock.yaml",
            acquisition,
            sources,
        )?;
        if let Some(prepared) = &mut prepared {
            patches::record(&root, prepared)?;
        }
        Ok(prepared)
    }
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
        if importers
            .get(serde_yaml::Value::String(".".into()))
            .is_none_or(|v| v.as_mapping().is_none())
        {
            bail!("pnpm lock requires a root importer");
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

impl crate::dependencies::context::Provider for Pnpm {
    fn id(&self) -> &'static str {
        "node/pnpm"
    }
    fn tools(&self) -> &'static [&'static str] {
        &["node", "pnpm"]
    }
    fn detect(&self, source: &std::path::Path) -> bool {
        source.join("package.json").is_file() && source.join("pnpm-lock.yaml").is_file()
    }
    fn store(&self) -> &'static str {
        "store"
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Prepared> {
        self.capture(context, super::registry::Acquisition::DependencyContext)?
            .context("pnpm preparation did not produce a captured store")
    }
}

impl Manager for Pnpm {
    fn workspace_build_command(&self) -> Option<Vec<String>> {
        Some(
            ["pnpm", "--recursive", "run", "build"]
                .map(str::to_owned)
                .to_vec(),
        )
    }
    fn workspace_plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        let prepared = context
            .dependencies
            .context("pnpm workspace requires captured metadata")?;
        let metadata = crate::builders::node::workspace::model::Metadata::read(
            prepared.record["extensions"]["oyzu.dev/pnpm"]["workspaces"].clone(),
        )?
        .context("missing captured pnpm workspace model")?;
        crate::builders::node::workspace::planning::plan(
            context,
            self,
            &metadata,
            "pnpm-workspace-build.mjs",
        )
    }
    fn is_workspace(&self, root: &std::path::Path, package: &serde_json::Value) -> bool {
        let config = std::fs::read_to_string(root.join("pnpm-workspace.yaml"))
            .ok()
            .and_then(|text| serde_yaml::from_str::<serde_yaml::Value>(&text).ok());
        package.get("workspaces").is_some()
            || config.is_some_and(|value| {
                value["packages"]
                    .as_sequence()
                    .is_some_and(|p| !p.is_empty())
            })
    }
    fn workspace_test_command(&self) -> Vec<String> {
        ["pnpm", "--recursive", "run", "test"]
            .map(str::to_owned)
            .to_vec()
    }
    fn development_test(
        &self,
        target: &crate::model::Target,
        task: &crate::model::Task,
    ) -> Result<Option<crate::builders::TaskPlan>> {
        let package = crate::records::read(&target.path.join("package.json"))?;
        let expected = if package["scripts"]["test"].is_string() {
            self.script("test", false)
        } else {
            self.workspace_test_command()
        };
        crate::builders::node::workspace::native_plan(
            target,
            task,
            "pnpm-workspace-describe.mjs",
            &expected,
        )
    }
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
        self.capture(context, super::registry::Acquisition::Build)
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
