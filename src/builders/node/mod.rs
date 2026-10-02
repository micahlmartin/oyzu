mod detection;
mod discovery;
mod jest;
mod managers;
mod planning;
mod quality;
mod reporting;
mod vitest;

use super::{Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile};
use crate::dependencies::Prepared;
use crate::model::{Target, Task};
use anyhow::Result;
use std::path::Path;

pub(super) struct Node;

static RUNTIME: &[RuntimeFile] = &[
    RuntimeFile {
        name: "npm-workspace-root.mjs",
        contents: include_str!("runtime/npm-workspace-root.mjs"),
    },
    RuntimeFile {
        name: "node-quality.mjs",
        contents: include_str!("runtime/quality.mjs"),
    },
    RuntimeFile {
        name: "npm-workspace-build.mjs",
        contents: include_str!("runtime/npm-workspace-build.mjs"),
    },
    RuntimeFile {
        name: "npm-workspace-plan.mjs",
        contents: include_str!("runtime/npm-workspace-plan.mjs"),
    },
    RuntimeFile {
        name: "npm-native.mjs",
        contents: include_str!("runtime/npm-native.mjs"),
    },
    RuntimeFile {
        name: "npm-workspaces.mjs",
        contents: include_str!("runtime/npm-workspaces.mjs"),
    },
    RuntimeFile {
        name: "node-archive.mjs",
        contents: include_str!("runtime/archive.mjs"),
    },
    RuntimeFile {
        name: "vitest.mjs",
        contents: include_str!("runtime/vitest.mjs"),
    },
    RuntimeFile {
        name: "manager-runtime.mjs",
        contents: include_str!("runtime/manager-runtime.mjs"),
    },
    RuntimeFile {
        name: "pnpm.mjs",
        contents: include_str!("runtime/pnpm.mjs"),
    },
    RuntimeFile {
        name: "yarn.mjs",
        contents: include_str!("runtime/yarn.mjs"),
    },
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
    fn development_command(
        &self,
        task: &Task,
    ) -> Result<Option<crate::builders::DevelopmentCommand>> {
        Ok(quality::development(task))
    }
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
        Ok(managers::get(&target.manager)?.image())
    }
    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        self.toolchain(context.target)?;
        planning::plan(context)
    }

    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        self.toolchain(context.target)?;
        managers::get(&context.target.manager)?.prepare(context)
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
                .is_some_and(|t| t.argv == [&target.manager, "run", "test"]);
        let jest_script = target
            .discovery
            .get("test-framework")
            .is_some_and(|p| p.selected() == "jest")
            && crate::records::read(&target.path.join("package.json"))
                .ok()
                .and_then(|p| p["scripts"]["test"].as_str().map(jest::recognized))
                .unwrap_or(false);
        let vitest_script = target
            .discovery
            .get("test-framework")
            .is_some_and(|p| p.selected() == "vitest")
            && crate::records::read(&target.path.join("package.json"))
                .ok()
                .and_then(|p| p["scripts"]["test"].as_str().map(vitest::recognized))
                .unwrap_or(false);
        reporting::instrument_override(task, env, native_script, &target.manager)
            .or_else(|| jest::instrument_override(task, jest_script))
            .or_else(|| vitest::instrument_override(task, vitest_script, &target.manager))
    }
}
