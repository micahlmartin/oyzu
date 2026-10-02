//! Native frontend output contracts. Collection and materialization belong to
//! the engine; selecting the native output directory belongs to this adapter.
use crate::{
    builders::{ArtifactKind, ArtifactSpec, BuilderPlan, CommandSpec},
    model::Target,
    snapshot,
};
use anyhow::{bail, Result};
use serde_json::Value;

pub(super) fn configure(target: &Target, package: &Value, plan: &mut BuilderPlan) -> Result<()> {
    if target.builder != "node/app"
        || target
            .discovery
            .get("output-profile")
            .is_none_or(|profile| profile.selected() != "vite-application")
    {
        return Ok(());
    }
    if !["dependencies", "devDependencies", "optionalDependencies"]
        .iter()
        .any(|field| package[*field].get("vite").is_some())
    {
        bail!(
            "{}: Vite application output requires a declared, captured vite dependency",
            target.name
        );
    }
    for file in super::detection::output_configuration_files() {
        if target.path.join(file).exists() {
            bail!("{}: executable Vite configuration requires native output metadata integration; refusing to guess its output directory", target.name);
        }
    }
    let output = output_directory(package)?;
    // The ordinary package operation is replaced by a directory output, while
    // native build/test/quality tasks and their required reports stay intact.
    let filename = format!("application-{}", plan.version);
    plan.package = CommandSpec::new(
        "package",
        &[
            "node",
            "/oyzu/node-application.mjs",
            output,
            &format!("/out/{}/artifacts/{filename}", target.name),
        ],
    );
    plan.artifacts = vec![ArtifactSpec {
        kind: ArtifactKind::Directory,
        name: "primary".into(),
        filename,
        version: None,
        media_type: "application/vnd.oyzu.tree.v1alpha1",
    }];
    let exclusions = serde_json::to_string(&[output])?;
    plan.env
        .insert("OYZU_NODE_QUALITY_EXCLUDE".into(), exclusions.clone());
    plan.fixed_env
        .insert("OYZU_NODE_QUALITY_EXCLUDE".into(), exclusions);
    Ok(())
}

pub(super) fn output_directory(package: &Value) -> Result<&str> {
    let words: Vec<_> = package["scripts"]["build"]
        .as_str()
        .unwrap_or("")
        .split_whitespace()
        .collect();
    let output = match words.as_slice() {
        ["vite", "build"] => "dist",
        ["vite", "build", "--outDir", output] => *output,
        _ => bail!("Vite output inference supports vite build with an optional literal --outDir; custom build flags require native metadata integration"),
    };
    if !snapshot::portable(output)
        || output.starts_with('-')
        || !output
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b))
    {
        bail!(
            "Vite output directory must be a portable contained literal path without shell syntax"
        );
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery;
    use serde_json::json;
    use std::fs;

    #[test]
    fn application_output_is_explicit_and_preserves_other_plan_obligations() {
        let root = tempfile::tempdir().unwrap();
        let package = json!({"name":"site", "scripts":{"build":"vite build"}, "devDependencies":{"vite":"8.3.2"}});
        fs::write(root.path().join("package.json"), package.to_string()).unwrap();
        let workspace = discovery::discover(root.path()).unwrap();
        let target = &workspace.targets["project"];
        let mut plan = BuilderPlan::new(
            "1.0.0-dev.g1234".into(),
            CommandSpec::new("package", &["npm", "pack"]),
        );
        let stages = plan.stages.clone();
        configure(target, &package, &mut plan).unwrap();
        assert_eq!(plan.stages, stages);
        assert_eq!(plan.artifacts.len(), 1);
        assert!(matches!(plan.artifacts[0].kind, ArtifactKind::Directory));
        assert_eq!(plan.artifacts[0].filename, "application-1.0.0-dev.g1234");
        assert_eq!(plan.package.argv[2], "dist");
        assert!(target.tasks["test"].build_stage);
        assert!(target.tasks["lint"].build_stage);
        assert!(target.tasks["format-check"].build_stage);
        assert!(!target.tasks["format"].build_stage);
        let mut native_package = target.clone();
        native_package.builder = "node/package".into();
        let mut unchanged = BuilderPlan::new(
            "1.0.0".into(),
            CommandSpec::new("package", &["npm", "pack"]),
        );
        configure(&native_package, &package, &mut unchanged).unwrap();
        assert_eq!(unchanged.package.argv, ["npm", "pack"]);
        fs::write(
            root.path().join("vite.config.ts"),
            "throw Error('never execute in planning');",
        )
        .unwrap();
        assert!(configure(target, &package, &mut plan)
            .unwrap_err()
            .to_string()
            .contains("native output metadata"));
        fs::remove_file(root.path().join("vite.config.ts")).unwrap();
        let mut missing = package.clone();
        missing["devDependencies"] = json!({});
        assert!(configure(target, &missing, &mut plan)
            .unwrap_err()
            .to_string()
            .contains("declared, captured"));
    }

    #[test]
    fn native_output_arguments_cannot_be_confused_with_shell_syntax() {
        assert_eq!(
            output_directory(
                &json!({"scripts":{"build":"vite build --outDir public-site/assets"}})
            )
            .unwrap(),
            "public-site/assets"
        );
        for command in [
            "vite build --ssr",
            "vite build --outDir ../outside",
            "vite build --outDir /outside",
            "vite build --outDir site;echo",
            "vite build --outDir $OUTPUT",
            "vite build --outDir site&&echo",
            "vite build --outDir 'site'",
            "vite build --outDir ./site",
            "vite build --outDir C:/site",
        ] {
            assert!(
                output_directory(&json!({"scripts":{"build":command}})).is_err(),
                "{command}"
            );
        }
    }
}
