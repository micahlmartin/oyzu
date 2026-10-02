//! Java quality defaults shared by native managers; task overrides retain authority.
use crate::{
    builders::{task::insert, BuilderPlan, DevelopmentCommand, RuntimeFile, TaskPlan},
    discovery::detectors::{exclusive, Detector, Finding},
    model::{Target, Task},
};
use anyhow::{Context, Result};
use std::path::PathBuf;

pub(super) const RUNTIME: RuntimeFile = RuntimeFile {
    name: "OyzuJavaQuality.java",
    contents: include_str!("runtime/OyzuJavaQuality.java"),
};

struct Checkstyle;
impl Detector<()> for Checkstyle {
    fn id(&self) -> &'static str {
        "java/checkstyle"
    }
    fn detect(&self, _: &()) -> Result<Vec<Finding>> {
        Ok(vec![Finding::fallback("checkstyle")])
    }
}

struct GoogleJavaFormat;
impl Detector<()> for GoogleJavaFormat {
    fn id(&self) -> &'static str {
        "java/google-java-format"
    }
    fn detect(&self, _: &()) -> Result<Vec<Finding>> {
        Ok(vec![Finding::fallback("google-java-format")])
    }
}

pub(super) fn discover(target: &mut Target) -> Result<()> {
    if !target.tasks.contains_key("lint") {
        target.discovery.insert(
            "linter".into(),
            exclusive("Java linter", &(), &[&Checkstyle])?,
        );
        insert(
            target,
            "lint",
            &["java", "OyzuJavaQuality.java", "lint"],
            true,
        );
    }
    if !target.tasks.contains_key("format-check") && !target.tasks.contains_key("format:check") {
        target.discovery.insert(
            "formatter".into(),
            exclusive("Java formatter", &(), &[&GoogleJavaFormat])?,
        );
        insert(
            target,
            "format-check",
            &["java", "OyzuJavaQuality.java", "format-check"],
            true,
        );
    }
    if !target.tasks.contains_key("format") {
        insert(
            target,
            "format",
            &["java", "OyzuJavaQuality.java", "format"],
            false,
        );
    }
    target.tasks.get_mut("format").unwrap().mutates_source = true;
    Ok(())
}

fn mode(task: &Task) -> Option<&str> {
    if !matches!(task.provider.as_str(), "maven" | "gradle" | "ant") {
        return None;
    }
    let [java, source, mode] = task.argv.as_slice() else {
        return None;
    };
    (java == "java"
        && source == "OyzuJavaQuality.java"
        && matches!(mode.as_str(), "lint" | "format-check" | "format"))
    .then_some(mode)
}

pub(super) fn plan(target: &Target, plan: &mut BuilderPlan) {
    for task in target.tasks.values() {
        if let Some(mode) = mode(task) {
            plan.tasks.insert(
                task.name.clone(),
                TaskPlan::command(&["java", "/oyzu/OyzuJavaQuality.java", mode]),
            );
        }
    }
    for env in [&mut plan.env, &mut plan.fixed_env] {
        env.insert(
            "OYZU_JAVA_QUALITY_HOME".into(),
            "/opt/oyzu-java-quality".into(),
        );
    }
}

pub(super) fn development(task: &Task) -> Result<Option<DevelopmentCommand>> {
    let Some(mode) = mode(task) else {
        return Ok(None);
    };
    let home = PathBuf::from(task.env.get("OYZU_JAVA_QUALITY_HOME").map(std::ffi::OsString::from).or_else(|| std::env::var_os("OYZU_JAVA_QUALITY_HOME"))
        .context("Java quality tasks require explicitly provisioned OYZU_JAVA_QUALITY_HOME; see docs/reference/java-quality.md")?);
    anyhow::ensure!(
        home.is_absolute(),
        "OYZU_JAVA_QUALITY_HOME must be an absolute directory"
    );
    let source = home.join("OyzuJavaQuality.java");
    anyhow::ensure!(
        source.is_file(),
        "missing provisioned Java quality adapter: {}",
        source.display()
    );
    Ok(Some(DevelopmentCommand {
        argv: vec![
            "java".into(),
            source.to_string_lossy().into_owned(),
            mode.into(),
        ],
        env: Default::default(),
    }))
}
