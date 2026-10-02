mod acquisition;
mod application;
mod dependency_context;
mod detection;
mod discovery;
mod distribution_app;
mod legacy;
mod planning;
mod quality;
mod testing;

use super::{Builder, BuilderPlan, Descriptor, PlanningContext, PreparationContext, RuntimeFile};
use crate::{dependencies::Prepared, model::Target};
use anyhow::{bail, Result};
use std::path::Path;

pub(super) struct Python;

impl Builder for Python {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            tools: &["python"],
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
    fn development_test(
        &self,
        target: &Target,
        task: &crate::model::Task,
    ) -> Result<Option<super::TaskPlan>> {
        Ok(testing::development(target, task))
    }
    fn toolchain(&self, target: &Target) -> Result<&'static str> {
        match target.manager.as_str() {
            "pip" if legacy::matches(&target.path) => Ok(legacy::IMAGE),
            "pip" => Ok(acquisition::PYTHON_IMAGE),
            "uv" => Ok(acquisition::UV_IMAGE),
            "poetry" => Ok(acquisition::POETRY_IMAGE),
            manager => bail!("unsupported Python manager {manager}"),
        }
    }
    fn prepare(&self, context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        acquisition::prepare(context).map(Some)
    }
    fn dependency_providers(
        &self,
    ) -> &'static [&'static dyn crate::dependencies::context::Provider] {
        &[
            &dependency_context::PythonStore::Pip,
            &dependency_context::PythonStore::Uv,
            &dependency_context::PythonStore::Poetry,
        ]
    }
    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        planning::plan(context)
    }
    fn runtime_files(&self) -> &'static [RuntimeFile] {
        &[
            RuntimeFile {
                name: "python-quality.py",
                contents: include_str!("runtime/quality.py"),
            },
            RuntimeFile {
                name: "python.py",
                contents: acquisition::PYTHON_HELPER,
            },
            RuntimeFile {
                name: "broker_transport.py",
                contents: crate::broker::RUNTIME,
            },
            RuntimeFile {
                name: "python-reporting.py",
                contents: include_str!("runtime/reporting.py"),
            },
            RuntimeFile {
                name: "python-app.py",
                contents: include_str!("runtime/application.py"),
            },
            RuntimeFile {
                name: "python-distribution-app.py",
                contents: include_str!("runtime/distribution_app.py"),
            },
            RuntimeFile {
                name: "python-legacy.py",
                contents: legacy::RUNTIME,
            },
        ]
    }

    fn container_profile(
        &self,
        target: &Target,
        prepared: Option<&Prepared>,
    ) -> Result<Option<super::ContainerProfile>> {
        if target.builder != "python/app"
            || prepared.is_some_and(|p| {
                p.record["extensions"]
                    .get("oyzu.dev/python-legacy")
                    .is_some()
            })
        {
            return Ok(None);
        }
        let prepared = prepared.ok_or_else(|| {
            anyhow::anyhow!("Python container requires captured interpreter identity")
        })?;
        let version = prepared.record["manager"]["platform"]["runtime"]
            .as_str()
            .ok_or_else(|| {
                anyhow::anyhow!("Python container requires captured interpreter identity")
            })?;
        let minor = version.split('.').take(2).collect::<Vec<_>>().join(".");
        if prepared.record["extensions"]["oyzu.dev/python-runtime"]["implementation"] != "cpython" {
            bail!("Python container profile requires captured CPython implementation identity");
        }
        if minor != "3.12" {
            bail!("no registered Python container runtime profile for {minor}");
        }
        Ok(Some(super::ContainerProfile {
            id: "python-pure-zip-3.12-v1",
            artifact: "application",
            base: acquisition::PYTHON_IMAGE,
            payload: "application.pyz",
            entrypoint: vec!["python".into(), "/app/application.pyz".into()],
            probe: vec![
                "python".into(),
                "-I".into(),
                "-c".into(),
                "import sys; print(f'{sys.implementation.name}:{sys.version_info.major}.{sys.version_info.minor}')".into(),
            ],
            expected: format!("cpython:{minor}"),
        }))
    }
}

#[cfg(test)]
mod container_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn runtime_profile_uses_captured_interpreter_and_only_application_outputs() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("requirements.txt"), "").unwrap();
        std::fs::write(root.path().join("app.py"), "print('ok')\n").unwrap();
        let mut target =
            crate::discovery::discover_target("app", root.path(), Some("python/app")).unwrap();
        let mut prepared = Prepared {
            root: root.path().into(),
            digest: "sha256:fixture".into(),
            record: json!({"manager":{"platform":{"runtime":"3.12.13"}},"extensions":{"oyzu.dev/python-runtime":{"implementation":"cpython"}}}),
        };
        let profile = Python
            .container_profile(&target, Some(&prepared))
            .unwrap()
            .unwrap();
        assert_eq!(profile.artifact, "application");
        assert_eq!(profile.expected, "cpython:3.12");
        assert_eq!(profile.entrypoint, ["python", "/app/application.pyz"]);
        assert!(Python.container_profile(&target, None).is_err());
        prepared.record["extensions"]["oyzu.dev/python-runtime"]["implementation"] = json!("pypy");
        assert!(Python.container_profile(&target, Some(&prepared)).is_err());
        prepared.record["extensions"]["oyzu.dev/python-runtime"]["implementation"] =
            json!("cpython");
        prepared.record["manager"]["platform"]["runtime"] = json!("3.13.0");
        assert!(Python.container_profile(&target, Some(&prepared)).is_err());
        prepared.record["extensions"]["oyzu.dev/python-legacy"] = json!({});
        assert!(Python
            .container_profile(&target, Some(&prepared))
            .unwrap()
            .is_none());
        target.builder = "python/package".into();
        assert!(Python.container_profile(&target, None).unwrap().is_none());
    }
}
