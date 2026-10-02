use super::{metadata, preparation::environment};
use crate::builders::{
    ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext, ReportFormat, ReportSpec, TaskPlan,
};
use anyhow::{Context, Result};
use serde_json::json;

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let prepared = context
        .dependencies
        .context("Maven requires a prepared native repository and reactor")?;
    let projects = metadata::read(&prepared.root.join("metadata.xml"))?;
    let root = projects
        .iter()
        .find(|p| p.path == ".")
        .unwrap_or(&projects[0]);
    let mut plan = BuilderPlan::new(
        root.version.clone(),
        CommandSpec::new("package", &["python3", "-I", "/oyzu/maven.py", "package"]),
    );
    plan.env.extend(environment());
    plan.prepare.push(CommandSpec::new(
        "prepare",
        &["python3", "-I", "/oyzu/maven.py", "install"],
    ));
    let mut build = TaskPlan::command(&["python3", "-I", "/oyzu/maven.py", "build"]);
    let mut exports = Vec::new();
    for (index, project) in projects.iter().enumerate() {
        let coordinate = format!("{}.{}", project.group, project.artifact);
        let module = crate::names::scoped("module", &coordinate);
        let duplicate = projects
            .iter()
            .filter(|p| p.artifact == project.artifact)
            .count()
            > 1;
        let artifact_name = if duplicate {
            coordinate.as_str()
        } else {
            project.artifact.as_str()
        };
        let mut outputs = vec![(
            "pom",
            project.pom.clone(),
            format!("{artifact_name}-{}.pom", project.version),
            "application/xml",
        )];
        if project.packaging != "pom" {
            let file = format!("{}.{}", project.final_name, project.packaging);
            let filename = if duplicate {
                format!("{}-{file}", project.group)
            } else {
                file.clone()
            };
            outputs.push((
                project.packaging.as_str(),
                format!("{}/{}", project.directory, file),
                filename,
                "application/java-archive",
            ));
        }
        for (kind, source, filename, media_type) in outputs {
            exports.push(json!({"source":source,"destination":format!("/out/{}/artifacts/{filename}", context.target.name)}));
            plan.artifacts.push(ArtifactSpec {
                kind: crate::builders::ArtifactKind::File,
                name: crate::names::scoped(kind, &coordinate),
                filename,
                media_type,
                version: Some(project.version.clone()),
            });
        }
        let tests = project.test_roots.iter().any(|path| {
            walkdir::WalkDir::new(context.target.path.join(path))
                .into_iter()
                .flatten()
                .any(|e| e.file_type().is_file())
        });
        if tests {
            if project.test_reports.is_empty() {
                anyhow::bail!(
                    "Maven test sources have no native Surefire/Failsafe execution in verify"
                );
            }
            build.reports.push(ReportSpec {
                format: ReportFormat::Junit,
                filename: "junit.xml",
                source: crate::reports::ReportSource::File,
                name: Some(module.clone()),
                input: Some(format!(".oyzu-maven/reports/{index}/TEST-*.xml")),
            });
            build.reports.push(ReportSpec {
                format: ReportFormat::Jacoco,
                filename: "jacoco.xml",
                source: crate::reports::ReportSource::File,
                name: Some(module),
                input: Some(format!("{}/site/jacoco/jacoco.xml", project.directory)),
            });
        }
    }
    plan.package.argv.push(serde_json::to_string(&exports)?);
    plan.tasks.insert("build".into(), build);
    Ok(plan)
}
