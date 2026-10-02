//! Typed native workspace facts; no scheduling or package execution here.
mod development;
mod planning;
mod scope;

pub(super) use development::command as development_command;
pub(super) use development::report_plan;

pub(super) fn plan(
    context: crate::builders::PlanningContext<'_>,
) -> anyhow::Result<crate::builders::BuilderPlan> {
    planning::plan(context)
}
pub(super) use crate::builders::node::workspace::model::Metadata;
