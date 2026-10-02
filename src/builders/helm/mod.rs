use crate::builders::task::insert;
mod archives;
mod detection;
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
pub(super) const RUNTIME: &[RuntimeFile] = &[
    RuntimeFile {
        name: "helm-charts.py",
        contents: include_str!("runtime/charts.py"),
    },
    RuntimeFile {
        name: "helm-archive.py",
        contents: include_str!("runtime/archive.py"),
    },
    RuntimeFile {
        name: "helm-test.py",
        contents: include_str!("runtime/testing.py"),
    },
];

impl Builder for Helm {
    fn acquisition_requires_network(&self) -> bool {
        false
    }
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

    fn development_command(
        &self,
        task: &crate::model::Task,
    ) -> Result<Option<crate::builders::DevelopmentCommand>> {
        let argv: Vec<_> = task.argv.iter().map(String::as_str).collect();
        if task.provider == "helm" {
            if let ["helm", "unittest", "--strict", chart] = argv.as_slice() {
                return Ok(Some(crate::builders::DevelopmentCommand {
                    argv: vec![
                        "python".into(),
                        "-I".into(),
                        "-c".into(),
                        include_str!("runtime/charts.py").into(),
                        "test".into(),
                        chart.to_string(),
                    ],
                    env: Default::default(),
                }));
            }
        }
        Ok(None)
    }
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            tools: &["helm"],
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
        let metadata = metadata::read(&path)?;
        target.version = metadata.version;
        let framework = detection::detect(&path)?;
        let unittest = framework.selected() == "helm-unittest";
        target.discovery.insert("test-framework".into(), framework);
        insert(
            target,
            "install",
            &["helm", "dependency", "build", chart],
            false,
        );
        insert(target, "build", &["helm", "package", chart], true);
        insert(target, "lint", &["helm", "lint", chart], true);
        if unittest {
            insert(
                target,
                "test",
                &["helm", "unittest", "--strict", chart],
                true,
            );
        } else if metadata.kind != "library" {
            insert(
                target,
                "test",
                &["helm", "template", "oyzu-check", chart],
                true,
            );
        } else {
            insert(
                target,
                "test",
                &["helm", "lint", chart, "--strict", "--with-subcharts"],
                true,
            );
        }

        Ok(())
    }
}
