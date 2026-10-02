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
    let binaries = metadata.binaries()?;
    for package_metadata in &metadata.packages {
        if !metadata.workspace_members.contains(&package_metadata.id) {
            continue;
        }
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
            kind: crate::builders::ArtifactKind::File,
            name: crate::names::scoped("crate", &package_metadata.name),
            filename: archive_name,
            version: Some(package_metadata.version.clone()),
            media_type: "application/gzip",
        });
        for (index, (owner, target)) in binaries.iter().enumerate() {
            if owner.id != package_metadata.id {
                continue;
            }
            if !names.insert(target.name.clone()) {
                bail!("duplicate Cargo binary name {}", target.name);
            }
            let version = &package_metadata.version;
            let filename = format!("{}-{version}-{host}", target.name);
            package.argv.extend([
                format!(".oyzu-build/target/oyzu-binaries/{index}"),
                format!("/out/{id}/artifacts/{filename}"),
            ]);
            artifacts.push(ArtifactSpec {
                kind: crate::builders::ArtifactKind::File,
                name: crate::names::scoped("bin", &target.name),
                filename,
                version: Some(version.clone()),
                media_type: "application/octet-stream",
            });
        }
    }
    if let Some(command) = super::packaging::command(&metadata)? {
        archive.argv = command;
    }
    let primary = metadata
        .packages
        .iter()
        .find(|p| metadata.workspace_members.contains(&p.id))
        .context("Cargo metadata has no workspace package")?;
    let mut plan = BuilderPlan::new(primary.version.clone(), package);
    plan.stages.push("archive");
    plan.tasks.insert("archive".into(), archive);
    plan.env.extend(environment());
    plan.env.insert("CARGO_BUILD_TARGET".into(), host);
    for name in [
        "CARGO_BUILD_TARGET",
        "CARGO_TARGET_DIR",
        "CARGO_HOME",
        "CARGO_NET_OFFLINE",
    ] {
        plan.fixed_env.insert(name.into(), plan.env[name].clone());
    }
    plan.prepare.push(CommandSpec::new(
        "prepare",
        &["cp", "-R", "/dependencies/overlay/.", "."],
    ));
    plan.tasks.insert(
        "build".into(),
        TaskPlan::command(&[
            "python3",
            "-I",
            "/oyzu/rust-build.py",
            "/dependencies/binaries.json",
            "cargo",
            "build",
            "--workspace",
            "--release",
            "--locked",
            "--offline",
            "--message-format=json-render-diagnostics",
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
        name: None,
        input: None,
    });
    test.reports.push(ReportSpec {
        format: ReportFormat::Cobertura,
        filename: "coverage.xml",
        source: crate::reports::ReportSource::File,
        name: None,
        input: None,
    });
    let doctests: Vec<_> = metadata
        .packages
        .iter()
        .filter(|package| {
            metadata.workspace_members.contains(&package.id)
                && package.targets.iter().any(|target| target.doctest)
        })
        .map(|package| package.name.clone())
        .collect();
    if !doctests.is_empty() {
        test.argv
            .push(format!("/out/{id}/reports/doctest/doctest.xml"));
        test.argv.extend(doctests);
        test.reports.push(ReportSpec {
            format: ReportFormat::Junit,
            filename: "doctest.xml",
            source: crate::reports::ReportSource::File,
            name: Some("doctest".into()),
            input: None,
        });
    }
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
    for task in plan.tasks.values_mut() {
        task.argv = super::preparation::command(std::mem::take(&mut task.argv));
    }
    Ok(plan)
}
