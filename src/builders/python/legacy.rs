//! Legacy setuptools facts are obtained in a broker-free worker before planning.
use crate::{builders::PreparationContext, executor, records, snapshot};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path, time::Duration};

pub(super) const IMAGE: &str = "python:3.12-bookworm";
pub(super) const RUNTIME: &str = include_str!("runtime/legacy.py");

pub(super) fn matches(root: &Path) -> bool {
    root.join("setup.py").is_file() && !root.join("pyproject.toml").exists()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Metadata {
    pub name: String,
    pub version: String,
    pub wheel_tag: String,
    pub compiler: Option<Compiler>,
    pub requirements: Vec<String>,
    pub extensions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Compiler {
    pub command: Vec<String>,
    pub version: String,
    pub include: String,
    pub soabi: String,
}

impl Metadata {
    pub fn read(value: Value) -> Result<Self> {
        let metadata: Self =
            serde_json::from_value(value).context("invalid legacy Python metadata")?;
        for token in [&metadata.name, &metadata.version, &metadata.wheel_tag] {
            if token.is_empty()
                || !token
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.+".contains(&b))
            {
                bail!("invalid legacy artifact identity");
            }
        }
        if !metadata.requirements.is_empty() {
            bail!("legacy runtime dependencies require metadata-driven acquisition");
        }
        if metadata.extensions.is_empty() != metadata.compiler.is_none() {
            bail!("legacy extension compiler facts are missing or inconsistent");
        }
        if let Some(compiler) = &metadata.compiler {
            if compiler.command.is_empty()
                || compiler
                    .command
                    .iter()
                    .any(|s| s.is_empty() || s.contains(char::is_whitespace))
                || compiler.version.is_empty()
                || compiler.include.is_empty()
                || compiler.soabi.is_empty()
            {
                bail!("incomplete legacy compiler capability");
            }
        }
        Ok(metadata)
    }
}

pub(super) fn capture(context: &PreparationContext<'_>, runtime: &Path) -> Result<Value> {
    let control = tempfile::tempdir()?;
    let workspace = control.path().join("workspace");
    snapshot::capture(&context.target.path, &workspace)?;
    let output = control.path().join("output");
    fs::create_dir(&output)?;
    let stdout = control.path().join("stdout");
    let stderr = control.path().join("stderr");
    let env = BTreeMap::from([
        ("OYZU_SOURCE_DIGEST".into(), context.source_digest.into()),
        ("PIP_NO_INDEX".into(), "1".into()),
        ("PIP_CONFIG_FILE".into(), "/dev/null".into()),
        ("HOME".into(), "/tmp/oyzu-home".into()),
        ("SOURCE_DATE_EPOCH".into(), "0".into()),
        ("PYTHONHASHSEED".into(), "0".into()),
    ]);
    let argv = ["python", "-I", "/oyzu/python.py", "legacy-metadata"].map(str::to_string);
    let result = executor::execute_with_mounts(
        executor::Request {
            log: context.log.clone(),
            image: context.image,
            workspace: &workspace,
            output: &output,
            cwd: "/workspace",
            argv: &argv,
            env: &env,
            stdout: &stdout,
            stderr: &stderr,
            timeout: Duration::from_secs(120),
            name: context.execution_name,
        },
        &[
            executor::Mount {
                source: runtime,
                destination: "/oyzu",
                readonly: true,
            },
            executor::Mount {
                source: context.destination,
                destination: "/dependencies",
                readonly: true,
            },
        ],
    )?;
    if result.code != 0 {
        bail!(
            "isolated legacy Python metadata failed: {}",
            fs::read_to_string(stderr)?
                .chars()
                .take(6000)
                .collect::<String>()
        );
    }
    let path = output.join("legacy.json");
    let info = fs::symlink_metadata(&path)?;
    if !info.is_file() || info.file_type().is_symlink() || info.len() > 1024 * 1024 {
        bail!("invalid legacy Python metadata output");
    }
    let value = records::read(&path)?;
    Metadata::read(value.clone())?;
    records::write(&context.destination.join("legacy.json"), &value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        builders::{Builder, PlanningContext},
        dependencies::Prepared,
        discovery,
    };
    use serde_json::json;

    #[test]
    fn legacy_planning_uses_captured_native_identity_without_executing_setup() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("setup.py"),
            "raise AssertionError('metadata must not execute on the host')\n",
        )
        .unwrap();
        let capture = tempfile::tempdir().unwrap();
        let source = snapshot::capture(root.path(), &capture.path().join("source")).unwrap();
        let workspace = discovery::discover(&capture.path().join("source")).unwrap();
        let target = &workspace.targets["project"];
        let builder = super::super::Python;
        assert_eq!(builder.toolchain(target).unwrap(), IMAGE);
        let version = format!("0.1.0.dev0+g{}", &source.digest[7..19]);
        let metadata = json!({"name":"native_demo","version":version,"wheelTag":"cp312-cp312-linux_x86_64","extensions":["native_math"],"requirements":[],"compiler":{"command":["gcc"],"version":"gcc 12.2.0","include":"/usr/local/include/python3.12","soabi":"cpython-312-x86_64-linux-gnu"}});
        let dependencies = Prepared {
            root: capture.path().join("dependencies"),
            digest: format!("sha256:{}", "1".repeat(64)),
            record: json!({"extensions":{"oyzu.dev/python-legacy":metadata}}),
        };
        let plan = builder
            .plan(PlanningContext {
                target,
                source: &source,
                dependencies: Some(&dependencies),
            })
            .unwrap();
        plan.validate().unwrap();
        assert_eq!(plan.version, version);
        assert_eq!(
            plan.artifacts[0].filename,
            format!("native_demo-{version}-cp312-cp312-linux_x86_64.whl")
        );
        assert_eq!(plan.fixed_env["CC"], "gcc");
        assert_eq!(plan.tasks["test"].reports.len(), 2);
        assert!(plan.tasks.contains_key("lint") && plan.tasks.contains_key("format-check"));
        let mut incomplete = metadata.clone();
        incomplete["compiler"] = Value::Null;
        assert!(Metadata::read(incomplete).is_err());
        let mut escaping = metadata;
        escaping["wheelTag"] = json!("../../outside");
        assert!(Metadata::read(escaping).is_err());
    }
}
