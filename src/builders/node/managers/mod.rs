//! Native manager ownership within the Node builder, independent of test runners.
mod empty;
mod npm;
mod pnpm;
#[cfg(test)]
mod tests;
mod yarn;

use crate::{
    builders::{BuilderPlan, CommandSpec, PlanningContext, PreparationContext},
    dependencies::Prepared,
};
use anyhow::{bail, Result};

pub(super) trait Manager: Sync {
    fn id(&self) -> &'static str;
    fn image(&self) -> &'static str;
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>>;
    fn configure(&self, context: &PlanningContext<'_>, plan: &mut BuilderPlan) -> Result<()>;
    fn package(&self, target: &str, filename: &str) -> CommandSpec;
    /// Resolve owned native invocation only for explicit development execution.
    /// None leaves the discovered command unchanged; errors must propagate.
    fn development_command(
        &self,
        _task: &crate::model::Task,
    ) -> Result<Option<crate::builders::DevelopmentCommand>> {
        Ok(None)
    }
    /// Translate captured native workspace facts into intent without executing
    /// tools. Return an explicit unsupported error until the manager owns its
    /// module/version/report semantics; never silently pack only the root.
    fn workspace_plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        bail!(
            "{}: {} workspace planning is not implemented yet",
            context.target.name,
            self.id()
        );
    }
    fn script(&self, name: &str, _forward_arguments: bool) -> Vec<String> {
        vec![self.id().into(), "run".into(), name.into()]
    }
}

pub(super) fn get(id: &str) -> Result<&'static dyn Manager> {
    static MANAGERS: &[&dyn Manager] = &[&npm::Npm, &pnpm::Pnpm, &yarn::Yarn];
    MANAGERS
        .iter()
        .copied()
        .find(|manager| manager.id() == id)
        .ok_or_else(|| anyhow::anyhow!("unsupported Node manager {id}"))
}

pub(super) fn require_prepared(context: &PlanningContext<'_>) -> Result<()> {
    if context.dependencies.is_none() {
        bail!(
            "{}: native manager preflight is required before planning",
            context.target.name
        );
    }
    Ok(())
}
