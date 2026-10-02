//! Built-in ecosystem adapters. The engine consumes plans, never manager-specific commands.
//!
//! These are crate-private implementation contracts, not a stable plugin ABI.
mod container;
mod contract;
mod docker;
mod go;
mod helm;
mod java;
mod node;
mod python;
mod rust;
mod task;
#[cfg(test)]
mod tests;

use anyhow::{bail, Result};
pub(crate) use container::ContainerProfile;
pub(crate) use contract::*;
pub(crate) use task::unavailable;

static BUILDERS: &[&dyn Builder] = &[
    &node::Node,
    &python::Python,
    &go::Go,
    &rust::Rust,
    &java::maven::Maven,
    &java::gradle::Gradle,
    &java::ant::Ant,
    &helm::Helm,
    &docker::Docker,
];

pub(crate) fn all() -> &'static [&'static dyn Builder] {
    BUILDERS
}

pub(crate) fn get(id: &str) -> Result<&'static dyn Builder> {
    for builder in BUILDERS {
        if builder.descriptor().ids.contains(&id) {
            return Ok(*builder);
        }
    }
    bail!("unsupported builder {id}")
}
