use crate::builders::task::insert;
mod metadata;
mod planning;
mod preparation;
mod reporting;
#[cfg(test)]
mod tests;

use crate::builders::{
    Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile,
};
use crate::dependencies::Prepared;
use crate::model::Target;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub(super) struct Rust;

impl Builder for Rust {
    fn toolchain(&self, _target: &Target) -> Result<&'static str> {
        Ok("oyzu-toolchain/rust:1.94.0-nextest0.9.146-llvmcov0.9.1")
    }

    fn runtime_files(&self) -> &'static [RuntimeFile] {
        &[
            RuntimeFile {
                name: "rust-test.sh",
                contents: include_str!("runtime/test.sh"),
            },
            RuntimeFile {
                name: "rust-build.py",
                contents: include_str!("runtime/build.py"),
            },
        ]
    }

    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        preparation::prepare(context).map(Some)
    }

    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        planning::plan(context)
    }

    fn descriptor(&self) -> Descriptor {
        Descriptor {
            tools: &["rust"],
            ids: &["rust/app", "rust/library"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        ["Cargo.toml"]
            .iter()
            .any(|file| path.join(file).is_file())
            .then_some("rust/app")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        let path = target.path.clone();
        target.manager = "cargo".into();
        let value: toml::Value = toml::from_str(&fs::read_to_string(path.join("Cargo.toml"))?)?;
        if let Some(version) = value
            .get("package")
            .and_then(|v| v.get("version"))
            .and_then(|v| v.as_str())
        {
            target.version = version.into();
        }
        insert(target, "install", &["cargo", "fetch", "--locked"], false);
        insert(
            target,
            "build",
            &["cargo", "build", "--locked", "--workspace"],
            true,
        );
        insert(
            target,
            "test",
            &["cargo", "test", "--locked", "--workspace"],
            true,
        );
        insert(
            target,
            "lint",
            &[
                "cargo",
                "clippy",
                "--locked",
                "--workspace",
                "--",
                "-D",
                "warnings",
            ],
            true,
        );
        insert(target, "format", &["cargo", "fmt", "--all"], false);
        target.tasks.get_mut("format").unwrap().mutates_source = true;
        insert(
            target,
            "format-check",
            &["cargo", "fmt", "--all", "--check"],
            true,
        );
        insert(
            target,
            "archive",
            &["cargo", "package", "--workspace", "--locked", "--offline"],
            true,
        );

        Ok(())
    }
}
