use super::{metadata, preparation::environment};
use crate::builders::{ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext, TaskPlan};
use anyhow::{Context, Result};
use std::collections::BTreeSet;

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let prepared = context
        .dependencies
        .context("Ant requires native prepared metadata")?;
    let metadata = metadata::read(&prepared.root.join("metadata.xml"))?;
    let mut package = CommandSpec::new(
        "package",
        &["java", "/oyzu/JarPackaging.java", &metadata.version],
    );
    let mut archive = TaskPlan::command(&["ant"]);
    let mut producers = BTreeSet::new();
    let mut artifacts = Vec::new();
    for jar in &metadata.jars {
        if producers.insert(&jar.target) {
            archive.argv.push(jar.target.clone());
        }
        let stem = jar
            .path
            .rsplit('/')
            .next()
            .unwrap()
            .strip_suffix(".jar")
            .unwrap();
        let suffix = format!("-{}", metadata.version);
        let stem = stem
            .strip_suffix(&suffix)
            .filter(|name| !name.is_empty())
            .unwrap_or(stem);
        let filename = format!("{stem}-{}.jar", metadata.version);
        package.argv.extend([
            jar.path.clone(),
            format!("/out/{}/artifacts/{filename}", context.target.name),
        ]);
        artifacts.push(ArtifactSpec {
            kind: crate::builders::ArtifactKind::File,
            name: crate::names::scoped("jar", stem),
            filename,
            media_type: "application/java-archive",
            version: None,
        });
    }
    let mut plan = BuilderPlan::new(metadata.version, package);
    plan.env.extend(environment());
    // Ant's native launcher applies these to custom task bodies and hooks too.
    // Metadata validation restricts versions to non-shell identifier characters.
    plan.env.insert(
        "ANT_ARGS".into(),
        format!("-Dversion={} -Doyzu.version={}", plan.version, plan.version),
    );
    plan.stages.push("archive");
    plan.tasks.insert("archive".into(), archive);
    plan.tasks.insert(
        "test".into(),
        super::reporting::test(&context.target.name, &plan.version),
    );
    plan.fixed_env
        .insert("OYZU_VERSION".into(), plan.version.clone());
    for stage in ["build", "lint", "format-check"] {
        if let Some(task) = context
            .target
            .tasks
            .get(stage)
            .filter(|task| task.availability.is_none() && !task.argv.is_empty())
        {
            let task_plan = TaskPlan {
                argv: task.argv.clone(),
                ..Default::default()
            };
            plan.tasks.insert(stage.into(), task_plan);
        }
    }
    plan.artifacts = artifacts;
    Ok(plan)
}
