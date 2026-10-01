use super::super::{semver_snapshot, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext};
use crate::records;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let target = context.target;
    let id = &target.name;
    let package = records::read(&target.path.join("package.json"))?;
    if package.get("workspaces").is_some() {
        bail!("{id}: npm workspace build integration is not implemented yet");
    }
    for field in ["dependencies", "devDependencies", "optionalDependencies"] {
        if package[field].as_object().is_some_and(|v| !v.is_empty()) {
            bail!("{id}: dependency acquisition is not implemented; refusing an incomplete or online build");
        }
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
    let script="const fs=require('node:fs');for(const f of ['package.json','package-lock.json']){if(!fs.existsSync(f))continue;const p=JSON.parse(fs.readFileSync(f,'utf8'));p.version=process.env.OYZU_VERSION;if(p.packages?.[''])p.packages[''].version=p.version;fs.writeFileSync(f,JSON.stringify(p,null,2)+'\\n');}";
    plan.prepare
        .push(CommandSpec::new("version", &["node", "-e", script]));
    let install = if target.path.join("package-lock.json").exists() {
        "ci"
    } else {
        "install"
    };
    plan.prepare.push(CommandSpec::new(
        "prepare",
        &["npm", install, "--offline", "--ignore-scripts"],
    ));
    if package["scripts"]["test"].as_str() == Some("node --test") {
        plan.tasks.insert("test".into(), super::reporting::test(id));
    }
    plan.artifacts.push(ArtifactSpec {
        name: "primary".into(),
        version: None,
        filename,
        media_type: "application/gzip",
    });
    Ok(plan)
}
