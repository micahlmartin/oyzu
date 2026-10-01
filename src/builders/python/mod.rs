mod acquisition;
mod discovery;
mod planning;

use super::{Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile};
use crate::{dependencies::Prepared, model::Target};
use anyhow::{bail, Result};
use std::path::Path;

pub(super) struct Python;

impl Builder for Python {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            ids: &["python/app", "python/package", "python/library"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        if path.join("pyproject.toml").is_file() || path.join("setup.py").is_file() {
            Some("python/package")
        } else if path.join("requirements.txt").is_file() {
            Some("python/app")
        } else {
            None
        }
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        discovery::discover(target)
    }
    fn toolchain(&self, target: &Target) -> Result<&'static str> {
        match target.manager.as_str() {
            "pip" => Ok(acquisition::PYTHON_IMAGE),
            "uv" => Ok(acquisition::UV_IMAGE),
            "poetry" => Ok(acquisition::POETRY_IMAGE),
            manager => bail!("unsupported Python manager {manager}"),
        }
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        acquisition::prepare(
            &context.target.path,
            context.destination,
            context.image,
            context.source_digest,
            context.execution_name,
            &context.target.manager,
        )
        .map(Some)
    }
    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        planning::plan(context)
    }
    fn runtime_files(&self) -> &'static [RuntimeFile] {
        &[
            RuntimeFile {
                name: "python.py",
                contents: acquisition::PYTHON_HELPER,
            },
            RuntimeFile {
                name: "python-reporting.py",
                contents: include_str!("runtime/reporting.py"),
            },
        ]
    }
}
