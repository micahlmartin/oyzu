use super::{metadata, preparation::environment};
use crate::builders::{
    ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext, ReportFormat, ReportSpec, TaskPlan,
};
use anyhow::{bail, Context, Result};
use serde_json::json;

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let prepared = context
        .dependencies
        .context("Gradle requires prepared native repository files and models")?;
    let builds = metadata::read(&prepared.root.join("metadata"))?;
    let root = builds.iter().find(|b| b.directory == ".").unwrap();
    let version = root
        .projects
        .iter()
        .find(|p| p.path == ":")
        .context("missing native Gradle root project")?
        .version
        .clone();
    let mut plan = BuilderPlan::new(
        version,
        CommandSpec::new("package", &["python3", "-I", "/oyzu/gradle.py", "package"]),
    );
    plan.env.extend(environment());
    let mut build = TaskPlan::command(&[
        "python3",
        "-I",
        "/oyzu/gradle.py",
        "build",
        &context.source.digest,
    ]);
    let mut exports = Vec::new();
    for model in &builds {
        for project in &model.projects {
            for archive in &project.archives {
                let identity = format!("{}:{}", model.directory, archive.task);
                let name = crate::names::scoped("archive", &identity);
                let original = archive.file.rsplit('/').next().unwrap();
                let count = builds
                    .iter()
                    .flat_map(|b| &b.projects)
                    .flat_map(|p| &p.archives)
                    .filter(|a| a.file.rsplit('/').next() == Some(original))
                    .count();
                let filename = if count > 1 {
                    format!("{name}-{original}")
                } else {
                    original.into()
                };
                let media_type = match archive.extension.as_str() {
                    "jar" | "war" | "ear" => "application/java-archive",
                    "zip" => "application/zip",
                    "tar" => "application/x-tar",
                    "gz" | "tgz" => "application/gzip",
                    other => bail!("Gradle archive {other} requires a registered artifact kind"),
                };
                exports.push(json!({"source":archive.file,"destination":format!("/out/{}/artifacts/{filename}", context.target.name)}));
                plan.artifacts.push(ArtifactSpec {
                    name,
                    filename,
                    media_type,
                    version: Some(archive.version.clone()),
                });
            }
            for test in &project.tests {
                if !test.sources_known {
                    bail!(
                        "Gradle test {} requires native source attribution",
                        test.task
                    );
                }
                if test.sources.is_empty() {
                    continue;
                }
                if !test.junit_enabled {
                    bail!("Gradle test {} disabled required JUnit evidence", test.task);
                }
                let name =
                    crate::names::scoped("test", &format!("{}:{}", model.directory, test.task));
                build.reports.push(ReportSpec {
                    format: ReportFormat::Junit,
                    filename: "junit.xml",
                    source: crate::reports::ReportSource::File,
                    name: Some(name.clone()),
                    input: Some(format!("{}/TEST-*.xml", test.junit)),
                });
                build.reports.push(ReportSpec {
                    format: ReportFormat::Jacoco,
                    filename: "jacoco.xml",
                    source: crate::reports::ReportSource::File,
                    name: Some(name),
                    input: Some(
                        test.coverage
                            .clone()
                            .context("missing native Gradle coverage destination")?,
                    ),
                });
            }
        }
    }
    if plan.artifacts.is_empty() {
        bail!("no native Gradle archives discovered");
    }
    plan.tasks.insert("build".into(), build);
    plan.package.argv.push(serde_json::to_string(&exports)?);
    Ok(plan)
}
