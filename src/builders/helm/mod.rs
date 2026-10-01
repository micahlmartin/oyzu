use crate::builders::task::insert;
mod metadata;
mod planning;
mod preparation;
#[cfg(test)]
mod tests;

use crate::builders::{
    Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile,
};
use crate::dependencies::Prepared;
use crate::model::Target;
use anyhow::Result;
use std::path::Path;

pub(super) struct Helm;

pub(super) const IMAGE: &str = "oyzu-toolchain/helm:3.22.0";
pub(super) const RUNTIME: &[RuntimeFile] = &[RuntimeFile {
    name: "helm-archive.py",
    contents: include_str!("runtime/archive.py"),
}];

impl Builder for Helm {
    fn toolchain(&self, _target: &Target) -> Result<&'static str> {
        Ok(IMAGE)
    }
    fn runtime_files(&self) -> &'static [RuntimeFile] {
        RUNTIME
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        preparation::prepare(context).map(Some)
    }
    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        planning::plan(context)
    }

    fn descriptor(&self) -> Descriptor {
        Descriptor {
            ids: &["helm/chart"],
        }
    }
    fn detect(&self, path: &Path) -> Option<&'static str> {
        (path.join("Chart.yaml").is_file() || path.join("chart/Chart.yaml").is_file())
            .then_some("helm/chart")
    }
    fn discover(&self, target: &mut Target) -> Result<()> {
        let chart = metadata::chart_path(&target.path)?;
        let path = target.path.join(chart);
        target.manager = "helm".into();
        target.version = metadata::read(&path)?.version;
        insert(
            target,
            "install",
            &["helm", "dependency", "build", chart],
            false,
        );
        insert(target, "build", &["helm", "package", chart], true);
        insert(target, "lint", &["helm", "lint", chart], true);
        insert(
            target,
            "test",
            &["helm", "template", "oyzu-check", chart],
            true,
        );

        Ok(())
    }
}
