use super::{metadata, RUNTIME};
use crate::{
    broker, builders::PreparationContext, dependencies::Prepared, executor, records, snapshot,
};
use anyhow::{bail, Result};
use serde_json::json;
use std::{collections::BTreeMap, fs, time::Duration};

pub(super) fn environment() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("HOME".into(), "/tmp/oyzu-home".into()),
        (
            "JAVA_TOOL_OPTIONS".into(),
            "-Xmx768m -Duser.home=/tmp/oyzu-home -Dfile.encoding=UTF-8".into(),
        ),
    ])
}

pub(super) fn prepare(context: PreparationContext<'_>) -> Result<Prepared> {
    let control = tempfile::tempdir()?;
    let workspace = control.path().join("workspace");
    snapshot::capture(&context.target.path, &workspace)?;
    fs::create_dir(context.destination)?;
    let runtime = control.path().join("runtime");
    let spool = control.path().join("spool");
    let private = control.path().join("private");
    for path in [&runtime, &spool, &private] {
        fs::create_dir(path)?;
    }
    for file in RUNTIME {
        fs::write(runtime.join(file.name), file.contents)?;
    }
    let session = broker::Session::start(
        &spool,
        &private,
        vec![broker::Source::new(
            "maven-central",
            "https://repo.maven.apache.org/maven2/",
            None,
        )?],
    )?;
    let stdout = control.path().join("stdout");
    let stderr = control.path().join("stderr");
    let result = executor::execute_with_mounts(
        executor::Request {
            image: context.image,
            workspace: &workspace,
            output: context.destination,
            cwd: "/workspace",
            argv: &[
                "python3".into(),
                "-I".into(),
                "/oyzu/gradle.py".into(),
                "acquire".into(),
                context.source_digest.into(),
            ],
            env: &environment(),
            stdout: &stdout,
            stderr: &stderr,
            timeout: Duration::from_secs(600),
            name: context.execution_name,
        },
        &[
            executor::Mount {
                source: &runtime,
                destination: "/oyzu",
                readonly: true,
            },
            executor::Mount {
                source: &spool,
                destination: "/broker",
                readonly: false,
            },
        ],
    )?;
    drop(session);
    if result.code != 0 {
        let logs = format!(
            "{}\n{}",
            fs::read_to_string(stdout)?,
            fs::read_to_string(stderr)?
        );
        let tail: String = logs
            .chars()
            .rev()
            .take(12000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        bail!(
            "Gradle dependency preparation failed ({}): {tail}",
            result.code
        );
    }
    let builds = metadata::read(&context.destination.join("metadata"))?;
    let tree = snapshot::capture_prepared(context.destination, &control.path().join("frozen"))?;
    let packages = super::super::maven_repository::inventory(&tree, "maven-central")?;
    let platform = json!({"os":context.image.os,"arch":context.image.arch});
    let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
        "adapter":{"id":"java/gradle-native-repository","digest":snapshot::file_digest(&std::env::current_exe()?)?,"layoutVersion":"1"},
        "manager":{"id":"gradle","version":builds[0].gradle_version,"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":[],"targetPlatform":platform,"packages":packages,"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/gradle":{"repositoryInventory":true,"dependencyEdges":"not-modeled"}}});
    Ok(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    })
}
