//! Native metadata collection through the shared isolated preparation lifecycle.
use super::metadata::Metadata;
use crate::{broker, builders::PreparationContext, dependencies::Prepared, records, snapshot};
use anyhow::{bail, Result};
use serde_json::json;
use std::{collections::BTreeMap, fs};

pub(super) fn environment() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("HOME".into(), "/tmp/oyzu-home".into()),
        ("GOENV".into(), "off".into()),
        ("GOTOOLCHAIN".into(), "local".into()),
        ("GOPROXY".into(), "off".into()),
        ("GOSUMDB".into(), "off".into()),
        ("GOVCS".into(), "*:off".into()),
        ("GOPRIVATE".into(), "".into()),
        ("GONOPROXY".into(), "".into()),
        ("CGO_ENABLED".into(), "1".into()),
        ("CC".into(), "gcc".into()),
        ("GOFLAGS".into(), "-mod=readonly -p=2".into()),
        ("GOMAXPROCS".into(), "2".into()),
        ("GOPATH".into(), "/tmp/oyzu-go".into()),
        ("GOCACHE".into(), "/tmp/oyzu-go-cache".into()),
    ])
}

pub(super) fn prepare(context: PreparationContext<'_>) -> Result<Option<Prepared>> {
    capture(context, Purpose::Build).map(Some)
}

pub(super) fn prepare_context(context: PreparationContext<'_>) -> Result<Prepared> {
    capture(context, Purpose::DependencyContext)
}

enum Purpose {
    Build,
    DependencyContext,
}

fn capture(context: PreparationContext<'_>, purpose: Purpose) -> Result<Prepared> {
    // Module stores use the consumer's native Go runtime, but do not need the
    // separate application/library publication adapter or projected versions.
    let mut script = "CGO_ENABLED=0 GOWORK=off GO111MODULE=off GOFLAGS=-p=2 go build -trimpath -buildvcs=false -o /tmp/oyzu-go-metadata /oyzu/go-metadata.go /oyzu/go-acquisition.go /oyzu/broker-transport.go\n/tmp/oyzu-go-metadata /out/metadata.json /out/modules /broker".to_string();
    if matches!(purpose, Purpose::Build) {
        script
            .push_str("\noyzu-go-modulezip /out/metadata.json \"$1\" \"$2\" /out/module-artifacts");
    }
    let mut argv = vec!["sh".into(), "-ec".into(), script, "oyzu-go-prepare".into()];
    if matches!(purpose, Purpose::Build) {
        argv.extend([
            crate::builders::semver_snapshot_digest(context.target, context.source_digest),
            context.target.builder.clone(),
        ]);
    }
    let tree = crate::dependencies::preparation::capture(
        &context,
        super::RUNTIME,
        &argv,
        &environment(),
        vec![broker::Source::new(
            "go-public",
            "https://proxy.golang.org/",
            None,
        )?],
    )?;
    let metadata: Metadata =
        serde_json::from_slice(&fs::read(context.destination.join("metadata.json"))?)?;
    metadata.validate()?;
    let module_artifacts = if matches!(purpose, Purpose::Build)
        && (context.target.builder == "go/library" || metadata.binaries.is_empty())
    {
        serde_json::to_value(super::packaging::read(
            context.destination,
            &metadata.modules,
        )?)?
    } else {
        json!([])
    };
    if metadata.os != context.image.os || metadata.arch != context.image.arch {
        bail!("native Go platform does not match the resolved toolchain image");
    }
    let platform = json!({"os":context.image.os,"arch":context.image.arch});
    let mut packages = Vec::new();
    for (index, dependency) in metadata.dependencies.iter().enumerate() {
        let path = context.destination.join("modules").join(&dependency.file);
        let info = fs::symlink_metadata(&path)?;
        if !info.is_file()
            || info.file_type().is_symlink()
            || info.len() != dependency.size
            || snapshot::file_digest(&path)? != dependency.digest
        {
            bail!("captured Go module archive does not match native inventory");
        }
        packages.push(json!({"id":format!("go/module-{index}"),"name":dependency.name,"version":dependency.version,
            "sourceId":"go-public","digest":dependency.digest,"size":dependency.size,"purpose":"build",
            "dependencies":[],"verification":"digest-only"}));
    }
    let mut locks = Vec::new();
    for member in &metadata.modules {
        for file in ["go.mod", "go.sum"] {
            let path = context.target.path.join(member).join(file);
            if path.is_file() {
                locks.push(snapshot::file_digest(&path)?);
            }
        }
    }
    for file in ["go.work", "go.work.sum"] {
        let path = context.target.path.join(file);
        if path.is_file() {
            locks.push(snapshot::file_digest(&path)?);
        }
    }
    let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
        "adapter":{"id":"go/native","digest":snapshot::file_digest(&std::env::current_exe()?)?,"layoutVersion":"3"},
        "manager":{"id":"go","version":metadata.version,"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":locks,"targetPlatform":platform,
        "packages":packages,"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/go-metadata":metadata,"oyzu.dev/go-module-artifacts":module_artifacts,"oyzu.dev/go-acquisition":{
            "inventory":"used-module-archives","dependencyEdges":"not-modeled","checksumVerification":"native-go.sum","sourceProfile":"public-proxy-only"}}});
    Ok(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    })
}
