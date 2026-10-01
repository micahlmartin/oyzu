mod detection;
mod discovery;
mod jest;
mod planning;
mod preparation;
mod reporting;

use super::{Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile};
use crate::dependencies::Prepared;
use crate::model::{Target, Task};
use anyhow::{bail, Result};
use std::path::Path;

pub(super) struct Node;

static RUNTIME: &[RuntimeFile] = &[
    RuntimeFile {
        name: "jest.mjs",
        contents: include_str!("runtime/jest.mjs"),
    },
    RuntimeFile {
        name: "jest-results.mjs",
        contents: include_str!("runtime/jest-results.mjs"),
    },
    RuntimeFile {
        name: "npm.mjs",
        contents: include_str!("runtime/npm.mjs"),
    },
    RuntimeFile {
        name: "npm_lock.mjs",
        contents: include_str!("runtime/lock.mjs"),
    },
    RuntimeFile {
        name: "broker_transport.mjs",
        contents: include_str!("../../broker/runtime/transport.mjs"),
    },
];

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

    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        self.toolchain(context.target)?;
        preparation::prepare(context)
    }

    fn runtime_files(&self) -> &'static [RuntimeFile] {
        RUNTIME
    }

    fn instrument_override(
        &self,
        target: &Target,
        task: &Task,
        env: &std::collections::BTreeMap<String, String>,
    ) -> Option<Vec<String>> {
        let native_script = target
            .discovery
            .get("test-framework")
            .is_some_and(|p| p.selected() == "node-test")
            && target
                .tasks
                .get("test")
                .is_some_and(|t| t.argv == ["npm", "run", "test"]);
        let jest_script = target
            .discovery
            .get("test-framework")
            .is_some_and(|p| p.selected() == "jest")
            && crate::records::read(&target.path.join("package.json"))
                .ok()
                .and_then(|p| p["scripts"]["test"].as_str().map(jest::recognized))
                .unwrap_or(false);
        reporting::instrument_override(task, env, native_script)
            .or_else(|| jest::instrument_override(task, jest_script))
    }
}
