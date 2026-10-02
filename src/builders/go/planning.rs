use super::{metadata::Metadata, preparation};
use crate::builders::{
    semver_snapshot, ArtifactKind, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext,
    ReportFormat, ReportSpec, TaskPlan,
};
use anyhow::{Context, Result};

// Paths remain individual shell words; native names never become shell syntax.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let target = context.target;
    let id = &target.name;
    let prepared = context
        .dependencies
        .context("Go planning requires captured native metadata")?;
    let metadata: Metadata =
        serde_json::from_value(prepared.record["extensions"]["oyzu.dev/go-metadata"].clone())?;
    metadata.validate()?;
    let library = target.builder == "go/library" || metadata.binaries.is_empty();
    let version = semver_snapshot(target, context.source);
    let mut plan = BuilderPlan::new(version.clone(), CommandSpec::new("package", &[]));
    plan.env.extend(preparation::environment());
    plan.fixed_env
        .insert("GOMODCACHE".into(), "/dependencies/modules".into());
    plan.fixed_env.insert("GOOS".into(), metadata.os);
    plan.fixed_env.insert("GOARCH".into(), metadata.arch);
    if metadata.cgo {
        plan.fixed_env.insert("CGO_ENABLED".into(), "1".into());
        plan.fixed_env.insert("CC".into(), "gcc".into());
    }
    plan.env.extend(plan.fixed_env.clone());
    plan.env
        .insert("GOCACHE".into(), "/workspace/.oyzu-build/go-cache".into());
    let workspace = if target.path.join("go.work").is_file() {
        "auto"
    } else {
        "off"
    };
    plan.env.insert("GOWORK".into(), workspace.into());
    let mut builds = vec!["mkdir -p .oyzu-build/bin".to_string()];
    let mut copies = Vec::new();
    for (index, binary) in metadata.binaries.iter().filter(|_| !library).enumerate() {
        let primary = metadata.binaries.len() == 1;
        let name = if primary {
            "primary".into()
        } else if crate::names::valid(&binary.name) {
            binary.name.clone()
        } else {
            crate::names::scoped("bin", &binary.name)
        };
        let filename = if primary {
            format!("{id}-{version}")
        } else {
            format!("{}-{version}", binary.name)
        };
        let input = format!(".oyzu-build/bin/{index}");
        // -buildvcs=false avoids ambient checkout metadata outside the captured tree.
        builds.push(format!(
            "go build -trimpath -buildvcs=false -o {} {}",
            quote(&input),
            quote(&binary.package)
        ));
        copies.push(format!(
            "cp {} {}",
            quote(&input),
            quote(&format!("/out/{id}/artifacts/{filename}"))
        ));
        plan.artifacts.push(ArtifactSpec {
            kind: ArtifactKind::File,
            name,
            filename,
            version: None,
            media_type: "application/octet-stream",
        });
    }
    if library {
        builds = vec![format!(
            "go build -trimpath -buildvcs=false {}",
            metadata
                .patterns
                .iter()
                .map(|pattern| quote(pattern))
                .collect::<Vec<_>>()
                .join(" ")
        )];
        let modules = super::packaging::read(&prepared.root, &metadata.modules)?;
        for (index, module) in modules.iter().enumerate() {
            for (kind, filename, media_type) in [
                ("zip", &module.zip, "application/zip"),
                ("mod", &module.module, "text/plain"),
                ("info", &module.info, "application/json"),
            ] {
                copies.push(format!(
                    "cp -- {} {}",
                    quote(&format!("/dependencies/module-artifacts/{filename}")),
                    quote(&format!("/out/{id}/artifacts/{filename}"))
                ));
                plan.artifacts.push(ArtifactSpec {
                    kind: ArtifactKind::File,
                    name: format!("module-{index}-{kind}"),
                    filename: filename.clone(),
                    version: Some(module.version.clone()),
                    media_type,
                });
            }
        }
    }
    plan.package = CommandSpec::new("package", &["sh", "-ec", &copies.join("\n")]);
    plan.tasks.insert(
        "build".into(),
        TaskPlan::command(&["sh", "-ec", &builds.join("\n")]),
    );
    let mut test = TaskPlan::command(&[
        "go",
        "test",
        "-json",
        &format!("-coverprofile=/out/{id}/reports/coverage.out"),
    ]);
    test.argv.extend(metadata.patterns.clone());
    test.reports = vec![
        ReportSpec {
            format: ReportFormat::Junit,
            filename: "junit.xml",
            source: crate::reports::ReportSource::GoTestEvents,
            name: None,
            input: None,
        },
        ReportSpec {
            format: ReportFormat::GoCover,
            filename: "coverage.out",
            source: crate::reports::ReportSource::File,
            name: None,
            input: None,
        },
    ];
    plan.tasks.insert("test".into(), test);
    let mut lint = TaskPlan::command(&["go", "vet"]);
    lint.argv.extend(metadata.patterns);
    plan.tasks.insert("lint".into(), lint);
    Ok(plan)
}
