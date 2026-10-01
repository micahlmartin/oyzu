use super::{metadata::Metadata, preparation::environment};
use crate::builders::{
    ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext, ReportFormat, ReportSpec, TaskPlan,
};
use anyhow::{bail, Context, Result};
use std::{collections::BTreeSet, fs};

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let prepared = context
        .dependencies
        .context("Cargo planning requires captured native metadata")?;
    let metadata: Metadata =
        serde_json::from_slice(&fs::read(prepared.root.join("metadata.json"))?)?;
    metadata.validate()?;
    let host = fs::read_to_string(prepared.root.join("host.txt"))?;
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("invalid Rust compiler host triple");
    }
    let id = &context.target.name;
    // Fixed shell program; native names and output paths are positional arguments.
    let mut package = CommandSpec::new(
        "package",
        &[
            "sh",
            "-c",
            "while [ \"$#\" -gt 0 ]; do cp -- \"$1\" \"$2\" || exit $?; shift 2; done",
            "oyzu-package",
        ],
    );
    let mut artifacts = vec![];
    let mut names = BTreeSet::new();
    let mut archive =
        TaskPlan::command(&["cargo", "package", "--locked", "--offline", "--allow-dirty"]);
    for package_metadata in &metadata.packages {
        archive
            .argv
            .extend(["--package".into(), package_metadata.name.clone()]);
        let archive_name = format!(
            "{}-{}.crate",
            package_metadata.name, package_metadata.version
        );
        package.argv.extend([
            format!(".oyzu-build/target/package/{archive_name}"),
            format!("/out/{id}/artifacts/{archive_name}"),
        ]);
        artifacts.push(ArtifactSpec {
            name: crate::names::scoped("crate", &package_metadata.name),
            filename: archive_name,
            version: Some(package_metadata.version.clone()),
            media_type: "application/gzip",
        });
        for target in &package_metadata.targets {
            if !target.kind.iter().any(|k| k == "bin") {
                continue;
            }
            if !names.insert(target.name.clone()) {
                bail!("duplicate Cargo binary name {}", target.name);
            }
            let version = &package_metadata.version;
            let filename = format!("{}-{version}-{host}", target.name);
            package.argv.extend([
                format!(".oyzu-build/target/{host}/release/{}", target.name),
                format!("/out/{id}/artifacts/{filename}"),
            ]);
            artifacts.push(ArtifactSpec {
                name: crate::names::scoped("bin", &target.name),
                filename,
                version: Some(version.clone()),
                media_type: "application/octet-stream",
            });
        }
    }
    let mut plan = BuilderPlan::new(metadata.packages[0].version.clone(), package);
    plan.stages.push("archive");
    plan.tasks.insert("archive".into(), archive);
    plan.env.extend(environment());
    plan.env.insert("CARGO_BUILD_TARGET".into(), host);
    plan.prepare.push(CommandSpec::new(
        "prepare",
        &["cp", "-R", "/dependencies/overlay/.", "."],
    ));
    plan.tasks.insert(
        "build".into(),
        TaskPlan::command(&[
            "cargo",
            "build",
            "--workspace",
            "--release",
            "--locked",
            "--offline",
        ]),
    );
    let mut test = TaskPlan::command(&[
        "sh",
        "/oyzu/rust-test.sh",
        &format!("/out/{id}/reports/coverage.xml"),
    ]);
    test.reports.push(ReportSpec {
        format: ReportFormat::Junit,
        filename: "junit.xml",
        source: crate::reports::ReportSource::File,
    });
    test.reports.push(ReportSpec {
        format: ReportFormat::Cobertura,
        filename: "coverage.xml",
        source: crate::reports::ReportSource::File,
    });
    plan.tasks.insert("test".into(), test);
    plan.tasks.insert(
        "lint".into(),
        TaskPlan::command(&[
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--offline",
            "--",
            "-D",
            "warnings",
        ]),
    );
    plan.artifacts = artifacts;
    Ok(plan)
}
