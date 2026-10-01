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
