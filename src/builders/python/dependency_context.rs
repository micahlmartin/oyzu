//! Native pip wheel stores resolved in the consumer's captured Python image.
use crate::{
    builders::PreparationContext,
    dependencies::{context::Provider, Prepared},
};
use anyhow::Result;
use std::path::Path;

pub(super) struct Pip;
impl Provider for Pip {
    fn id(&self) -> &'static str {
        "python/pip"
    }
    fn tools(&self) -> &'static [&'static str] {
        &["python"]
    }
    fn detect(&self, source: &Path) -> bool {
        source.join("requirements.txt").is_file()
    }
    fn store(&self) -> &'static str {
        "wheels"
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Prepared> {
        super::acquisition::prepare_runtime(context)
    }
}
