mod application;
mod detection;
mod discovery;
mod jest;
mod managers;
mod mocha;
mod planning;
mod quality;
mod reporting;
mod toolchain;
mod vitest;

use super::{Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile};
use crate::dependencies::Prepared;
use crate::model::{Target, Task};
use anyhow::Result;
use std::path::Path;

pub(super) struct Node;

static RUNTIME: &[RuntimeFile] = &[
    RuntimeFile {
        name: "node-runtime.mjs",
        contents: include_str!("runtime/node-runtime.mjs"),
    },
    RuntimeFile {
        name: "mocha.mjs",
        contents: include_str!("runtime/mocha.mjs"),
    },
    RuntimeFile {
        name: "mocha-reporter.cjs",
        contents: include_str!("runtime/mocha-reporter.cjs"),
    },
    RuntimeFile {
        name: "node-vite.mjs",
        contents: include_str!("runtime/vite.mjs"),
    },
    RuntimeFile {
        name: "node-application.mjs",
        contents: include_str!("runtime/application.mjs"),
    },
    RuntimeFile {
        name: "pnpm-patches.mjs",
        contents: include_str!("runtime/pnpm-patches.mjs"),
    },
    RuntimeFile {
        name: "registry-archives.mjs",
        contents: include_str!("runtime/registry-archives.mjs"),
    },
    RuntimeFile {
        name: "yarn-registry.mjs",
        contents: include_str!("runtime/yarn-registry.mjs"),
    },
    RuntimeFile {
        name: "integrity.mjs",
        contents: include_str!("runtime/integrity.mjs"),
    },
    RuntimeFile {
        name: "pnpm-registry.mjs",
        contents: include_str!("runtime/pnpm-registry.mjs"),
    },
    RuntimeFile {
        name: "npm-workspace-test-scope.mjs",
        contents: include_str!("runtime/npm-workspace-test-scope.mjs"),
    },
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
    fn dependency_providers(
        &self,
    ) -> &'static [&'static dyn crate::dependencies::context::Provider] {
        managers::dependency_providers()
    }
    fn development_command(
        &self,
        task: &Task,
    ) -> Result<Option<crate::builders::DevelopmentCommand>> {
        if let Ok(manager) = managers::get(&task.provider) {
            if let Some(command) = manager.development_command(task)? {
                return Ok(Some(command));
            }
        }
        Ok(quality::development(task))
    }
    fn development_test(&self, target: &Target, task: &Task) -> Result<Option<super::TaskPlan>> {
        let package = crate::records::read(&target.path.join("package.json"))?;
        if task.name != "test"
            || package.get("workspaces").is_some()
            || target
                .discovery
                .get("test-framework")
                .is_none_or(|p| p.selected() != "node-test")
        {
            return Ok(None);
        }
        let env = std::collections::BTreeMap::from([
            (
                "OYZU_TEST_REPORT".into(),
                format!("/out/{}/reports/junit.xml", target.name),
            ),
            (
                "OYZU_COVERAGE_REPORT".into(),
                format!("/out/{}/reports/coverage.lcov", target.name),
            ),
        ]);
        Ok(Some(reporting::test(
            &target.name,
            self.instrument_override(target, task, &env)
                .unwrap_or_else(|| task.argv.clone()),
            false,
        )))
    }

    fn descriptor(&self) -> Descriptor {
        Descriptor {
            tools: &["node"],
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
    fn variant_toolchain(&self, target: &Target) -> Result<String> {
        let manager = managers::get(&target.manager)?;
        match toolchain::requested(target)? {
            Some(version) => manager.runtime_image(version),
            None => Ok(manager.image().into()),
        }
    }
    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        self.variant_toolchain(context.target)?;
        let namespace = format!("oyzu.dev/{}", context.target.manager);
        let actual = context
            .dependencies
            .and_then(|d| d.record["extensions"][&namespace]["nodeVersion"].as_str());
        toolchain::verify(context.target, actual)?;
        planning::plan(context)
    }

    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        self.variant_toolchain(context.target)?;
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
        let package = crate::records::read(&target.path.join("package.json")).ok();
        let script = package.as_ref().and_then(|p| p["scripts"]["test"].as_str());
        let recognized = |framework: &str, accepts: fn(&str) -> bool| {
            target
                .discovery
                .get("test-framework")
                .is_some_and(|p| p.selected() == framework)
                && script.is_some_and(accepts)
        };
        reporting::instrument_override(task, env, native_script, &target.manager)
            .or_else(|| jest::instrument_override(task, recognized("jest", jest::recognized)))
            .or_else(|| {
                vitest::instrument_override(
                    task,
                    recognized("vitest", vitest::recognized),
                    &target.manager,
                )
            })
            .or_else(|| {
                mocha::instrument_override(
                    task,
                    recognized("mocha", mocha::recognized),
                    &target.manager,
                )
            })
    }
}
