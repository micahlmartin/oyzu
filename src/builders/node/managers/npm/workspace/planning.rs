//! npm owns graph acquisition; common planning owns package build intent.
pub(super) use crate::builders::node::workspace::planning::ordered;
use anyhow::{Context, Result};

pub(super) fn plan(
    context: crate::builders::PlanningContext<'_>,
) -> Result<crate::builders::BuilderPlan> {
    let prepared = context
        .dependencies
        .context("npm workspace requires prepared metadata")?;
    let metadata =
        super::Metadata::read(prepared.record["extensions"]["oyzu.dev/npm"]["workspaces"].clone())?
            .context("missing captured npm workspace model")?;
    crate::builders::node::workspace::planning::plan(
        context,
        &super::super::Npm,
        &metadata,
        "npm-workspace-build.mjs",
    )
}
