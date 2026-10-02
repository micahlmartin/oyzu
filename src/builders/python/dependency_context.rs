//! Native Python wheel stores resolved in the consumer's captured image.
use crate::{
    builders::PreparationContext,
    dependencies::{context::Provider, Prepared},
};
use anyhow::Result;
use std::path::Path;

pub(super) enum PythonStore {
    Pip,
    Uv,
    Poetry,
}

impl PythonStore {
    pub(super) fn manager(&self) -> &'static str {
        match self {
            Self::Pip => "pip",
            Self::Uv => "uv",
            Self::Poetry => "poetry",
        }
    }
    pub(super) fn lock(&self) -> &'static str {
        match self {
            Self::Pip => "requirements.txt",
            Self::Uv => "uv.lock",
            Self::Poetry => "poetry.lock",
        }
    }
}

impl Provider for PythonStore {
    fn id(&self) -> &'static str {
        match self {
            Self::Pip => "python/pip",
            Self::Uv => "python/uv",
            Self::Poetry => "python/poetry",
        }
    }
    fn tools(&self) -> &'static [&'static str] {
        match self {
            Self::Pip => &["python"],
            Self::Uv => &["python", "uv"],
            Self::Poetry => &["python", "poetry"],
        }
    }
    fn detect(&self, source: &Path) -> bool {
        source.join(self.lock()).is_file()
            && (matches!(self, Self::Pip) || source.join("pyproject.toml").is_file())
    }
    fn store(&self) -> &'static str {
        "wheels"
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Prepared> {
        super::acquisition::prepare_runtime(context, self)
    }
}
