mod detection;
mod discovery;
mod planning;
mod reporting;

use super::{Builder, BuilderPlan, Descriptor, PlanningContext};
use crate::model::{Target, Task};
use anyhow::{bail, Result};
use std::path::Path;

pub(super) struct Node;

impl Builder for Node {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            ids: &["node/app", "node/package"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        path.join("package.json").is_file().then_some("node/app")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        discovery::discover(target)
    }
    fn toolchain(&self, target: &Target) -> Result<&'static str> {
        match target.manager.as_str() {
            "npm" => Ok("node:22-bookworm-slim"),
            manager => bail!(
                "{}: {manager} build integration is not implemented yet",
                target.name
            ),
        }
    }
    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        self.toolchain(context.target)?;
        planning::plan(context)
    }

    fn instrument_override(
        &self,
        _target: &Target,
        task: &Task,
        env: &std::collections::BTreeMap<String, String>,
    ) -> Option<Vec<String>> {
        reporting::instrument_override(task, env)
    }
}
