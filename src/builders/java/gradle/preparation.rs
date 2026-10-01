use super::{metadata, RUNTIME};
use crate::{broker, builders::PreparationContext, dependencies::Prepared, records, snapshot};
use anyhow::Result;
use serde_json::json;
use std::collections::BTreeMap;

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
    let tree = crate::dependencies::preparation::capture(
        &context,
        RUNTIME,
        &[
            "python3".into(),
            "-I".into(),
            "/oyzu/gradle.py".into(),
            "acquire".into(),
            context.source_digest.into(),
        ],
        &environment(),
        vec![broker::Source::new(
            "maven-central",
            "https://repo.maven.apache.org/maven2/",
            None,
        )?],
    )?;
    let builds = metadata::read(&context.destination.join("metadata"))?;
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
