use super::super::{semver_snapshot, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext};
use crate::records;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let target = context.target;
    let id = &target.name;
    let package = records::read(&target.path.join("package.json"))?;
    let captured = super::preparation::required(&target.path, &package)?;
    if captured && context.dependencies.is_none() {
        bail!("{id}: npm dependency capture is required before planning execution");
    }
    let version = semver_snapshot(target, context.source);
    let name = package["name"]
        .as_str()
        .context("npm artifact requires package name")?;
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@/._-".contains(&b))
    {
        bail!("invalid npm package name");
    }
    let filename = format!(
        "{}-{version}.tgz",
        name.trim_start_matches('@').replace('/', "-")
    );
    let mut plan = BuilderPlan::new(
        version,
        CommandSpec::new(
            "package",
            &[
                "npm",
                "pack",
                "--ignore-scripts",
                "--pack-destination",
                &format!("/out/{id}/artifacts"),
            ],
        ),
    );
    plan.env.extend(BTreeMap::from([
        ("npm_config_offline".into(), "true".into()),
        ("npm_config_audit".into(), "false".into()),
        ("npm_config_fund".into(), "false".into()),
        ("npm_config_cache".into(), "/tmp/npm-cache".into()),
    ]));
    let script="const fs=require('node:fs');for(const f of ['package.json','package-lock.json','npm-shrinkwrap.json']){if(!fs.existsSync(f))continue;const p=JSON.parse(fs.readFileSync(f,'utf8'));p.version=process.env.OYZU_VERSION;if(p.packages?.[''])p.packages[''].version=p.version;fs.writeFileSync(f,JSON.stringify(p,null,2)+'\\n');}";
    plan.prepare
        .push(CommandSpec::new("version", &["node", "-e", script]));
    if context.dependencies.is_some() {
        plan.prepare.push(CommandSpec::new(
            "prepare",
            &["node", "/oyzu/npm.mjs", "install", "/dependencies", "."],
        ));
    } else {
        let install = if super::preparation::lockfile(&target.path).is_some() {
            "ci"
        } else {
            "install"
        };
        plan.prepare.push(CommandSpec::new(
            "prepare",
            &["npm", install, "--offline", "--ignore-scripts"],
        ));
    }
    let framework = target
        .discovery
        .get("test-framework")
        .context("missing resolved Node test framework")?
        .selected();
    let script = package["scripts"]["test"].as_str();
    if !["node-test", "jest"].contains(&framework) && script.is_none() {
        bail!("{id}: {framework} test/report integration is not implemented yet; refusing to omit its test operation");
    }
    let command = if framework == "jest" && script.is_none_or(super::jest::recognized) {
        super::jest::wrap(super::super::strings(if script.is_some() {
            &["npm", "run", "test", "--"]
        } else {
            super::jest::DEFAULT
        }))
    } else if script.is_some() {
        super::super::strings(if script == Some("node --test") {
            &["npm", "run", "test", "--"]
        } else {
            &["npm", "run", "test"]
        })
    } else {
        super::super::strings(&["node", "--test"])
    };
    plan.tasks.insert(
        "test".into(),
        super::reporting::test(id, command, framework == "node-test"),
    );
    plan.artifacts.push(ArtifactSpec {
        kind: crate::builders::ArtifactKind::File,
        name: "primary".into(),
        version: None,
        filename,
        media_type: "application/gzip",
    });
    Ok(plan)
}
