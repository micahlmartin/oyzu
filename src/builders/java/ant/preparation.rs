use super::{metadata, RUNTIME};
use crate::{builders::PreparationContext, dependencies::Prepared, executor, records, snapshot};
use anyhow::{bail, Result};
use serde_json::json;
use std::{collections::BTreeMap, fs, time::Duration};

pub(super) fn environment() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("HOME".into(), "/tmp/oyzu-home".into()),
        (
            "JAVA_TOOL_OPTIONS".into(),
            "-Xmx512m -Duser.home=/tmp/oyzu-home -Dfile.encoding=UTF-8".into(),
        ),
    ])
}

pub(super) fn prepare(context: PreparationContext<'_>) -> Result<Prepared> {
    if context.target.path.join("ivy.xml").exists() {
        bail!("Ant/Ivy dependency acquisition requires the approved resolver adapter");
    }
    let control = tempfile::tempdir()?;
    let workspace = control.path().join("workspace");
    snapshot::capture(&context.target.path, &workspace)?;
    fs::create_dir(context.destination)?;
    let runtime = control.path().join("runtime");
    fs::create_dir(&runtime)?;
    for file in RUNTIME {
        fs::write(runtime.join(file.name), file.contents)?;
    }
    let query = |version: Option<&str>| -> Result<()> {
        let stdout = control.path().join("stdout");
        let stderr = control.path().join("stderr");
        let mut argv: Vec<String> = [
            "java",
            "--class-path",
            "/opt/ant/lib/ant.jar:/opt/ant/lib/ant-launcher.jar",
            "/oyzu/AntMetadata.java",
            "/workspace/build.xml",
            "/out/metadata.xml",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        if let Some(version) = version {
            argv.push(version.into());
        }
        let result = executor::execute_with_mounts(
            executor::Request {
                image: context.image,
                workspace: &workspace,
                output: context.destination,
                cwd: "/workspace",
                argv: &argv,
                env: &environment(),
                stdout: &stdout,
                stderr: &stderr,
                timeout: Duration::from_secs(120),
                name: context.execution_name,
            },
            &[executor::Mount {
                source: &runtime,
                destination: "/oyzu",
                readonly: true,
            }],
        )?;
        if result.code != 0 {
            bail!(
                "Ant metadata evaluation failed: {}",
                fs::read_to_string(stderr)?
            );
        }
        Ok(())
    };
    query(None)?;
    let original = metadata::read(&context.destination.join("metadata.xml"))?;
    let version = format!(
        "{}-dev.g{}",
        original.version.split(['-', '+']).next().unwrap(),
        &context.source_digest[7..19]
    );
    query(Some(&version))?;
    metadata::read(&context.destination.join("metadata.xml"))?;
    let tree = snapshot::capture(context.destination, &control.path().join("frozen"))?;
    let platform = json!({"os":context.image.os,"arch":context.image.arch});
    let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
        "adapter":{"id":"java/ant-native-metadata","digest":snapshot::file_digest(&std::env::current_exe()?)?,"layoutVersion":"1"},
        "manager":{"id":"ant","version":original.manager_version,"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":[],"targetPlatform":platform,"packages":[],"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/ant":{"originalVersion":original.version,"version":version}}
    });
    Ok(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    })
}
