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
            .is_none_or(|profile| {
                !["vite-application", "dist-application"].contains(&profile.selected())
            })
    {
        return Ok(());
    }
    let vite = target.discovery["output-profile"].selected() == "vite-application";
    if vite
        && !["dependencies", "devDependencies", "optionalDependencies"]
            .iter()
            .any(|field| package[*field].get("vite").is_some())
    {
        bail!(
            "{}: Vite application output requires a declared, captured vite dependency",
            target.name
        );
    }
    let configured = vite
        && super::detection::output_configuration_files()
            .iter()
            .any(|file| target.path.join(file).exists());
    let output = if vite {
        output_directory(package)?
    } else {
        "dist"
    };
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
    if configured {
        let state = format!("/out/{}/vite-state.json", target.name);
        let metadata = format!("vite-output-{}.json", plan.version);
        let record = format!("/out/{}/artifacts/{metadata}", target.name);
        let script = package["scripts"]["build"].as_str().unwrap();
        let override_output = if script.split_whitespace().count() == 4 {
            output
        } else {
            ""
        };
        plan.prepare.push(CommandSpec::new(
            "configure-vite",
            &[
                "node",
                "/oyzu/node-vite.mjs",
                "prepare",
                &record,
                script,
                override_output,
            ],
        ));
        plan.package = CommandSpec::new(
            "package",
            &[
                "node",
                "/oyzu/node-vite.mjs",
                "package",
                &record,
                &format!(
                    "/out/{}/artifacts/{}",
                    target.name, plan.artifacts[0].filename
                ),
            ],
        );
        plan.artifacts.push(ArtifactSpec {
            kind: ArtifactKind::File,
            name: "build-metadata".into(),
            filename: metadata,
            version: None,
            media_type: "application/vnd.oyzu.vite-output.v1+json",
        });
        for (name, value) in [
            ("OYZU_NODE_VITE_STATE", state),
            ("OYZU_NODE_VITE_RECORD", record),
        ] {
            plan.env.insert(name.into(), value.clone());
            plan.fixed_env.insert(name.into(), value);
        }
    }
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
        configure(target, &package, &mut plan).unwrap();
        assert_eq!(plan.artifacts.len(), 2);
        assert_eq!(plan.artifacts[1].name, "build-metadata");
        assert_eq!(plan.prepare.last().unwrap().operation, "configure-vite");
        assert_eq!(plan.package.argv[1], "/oyzu/node-vite.mjs");
        fs::remove_file(root.path().join("vite.config.ts")).unwrap();
        let mut missing = package.clone();
        missing["devDependencies"] = json!({});
        assert!(configure(target, &missing, &mut plan)
            .unwrap_err()
            .to_string()
            .contains("declared, captured"));
    }

    #[test]
    fn explicit_custom_application_uses_dist_without_claiming_platform_independence() {
        let root = tempfile::tempdir().unwrap();
        let package = json!({"name":"site", "scripts":{"build":"node build.mjs"}});
        fs::write(root.path().join("package.json"), package.to_string()).unwrap();
        fs::write(root.path().join("build.yaml"), "site:\n  uses: node/app\n").unwrap();
        let workspace = discovery::discover(root.path()).unwrap();
        let mut plan = BuilderPlan::new(
            "1.0.0-dev.g1234".into(),
            CommandSpec::new("package", &["npm", "pack"]),
        );
        configure(&workspace.targets["site"], &package, &mut plan).unwrap();
        assert_eq!(plan.package.argv[2], "dist");
        assert_eq!(plan.artifacts[0].name, "primary");
        assert!(matches!(plan.artifacts[0].kind, ArtifactKind::Directory));
        assert_eq!(plan.artifacts[0].filename, "application-1.0.0-dev.g1234");
        assert_eq!(plan.fixed_env["OYZU_NODE_QUALITY_EXCLUDE"], "[\"dist\"]");
        assert!(plan.prepare.is_empty());
        assert!(!plan.env.contains_key("OYZU_NODE_BROWSER"));
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
