use super::super::{semver_snapshot, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext};
use crate::records;
use anyhow::{bail, Context, Result};

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let target = context.target;
    let id = &target.name;
    let package = records::read(&target.path.join("package.json"))?;
    let manager = super::managers::get(&target.manager)?;
    if package.get("workspaces").is_some() {
        return manager.workspace_plan(context);
    }
    let version = semver_snapshot(target, context.source);
    let name = package["name"]
        .as_str()
        .context("Node package artifact requires package name")?;
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@/._-".contains(&b))
    {
        bail!("invalid Node package name");
    }
    let filename = format!(
        "{}-{version}.tgz",
        name.trim_start_matches('@').replace('/', "-")
    );
    let mut plan = BuilderPlan::new(version, manager.package(id, &filename));
    let script="const fs=require('node:fs');for(const f of ['package.json','package-lock.json','npm-shrinkwrap.json']){if(!fs.existsSync(f))continue;const p=JSON.parse(fs.readFileSync(f,'utf8'));p.version=process.env.OYZU_VERSION;if(p.packages?.[''])p.packages[''].version=p.version;fs.writeFileSync(f,JSON.stringify(p,null,2)+'\\n');}";
    plan.prepare
        .push(CommandSpec::new("version", &["node", "-e", script]));
    manager.configure(&context, &mut plan)?;
    let framework = target
        .discovery
        .get("test-framework")
        .context("missing resolved Node test framework")?
        .selected();
    let script = package["scripts"]["test"].as_str();
    if framework == "vitest"
        && script.is_none_or(super::vitest::recognized)
        && !["dependencies", "devDependencies", "optionalDependencies"]
            .iter()
            .any(|field| package[*field].get("vitest").is_some())
    {
        bail!("{id}: vitest requires a declared and captured native framework dependency");
    }
    if !["node-test", "jest", "vitest"].contains(&framework) && script.is_none() {
        bail!("{id}: {framework} test/report integration is not implemented yet; refusing to omit its test operation");
    }
    let command = if framework == "jest" && script.is_none_or(super::jest::recognized) {
        super::jest::wrap(if script.is_some() {
            manager.script("test", true)
        } else {
            super::super::strings(super::jest::DEFAULT)
        })
    } else if framework == "vitest" && script.is_none_or(super::vitest::recognized) {
        super::vitest::wrap(if script.is_some() {
            manager.script("test", true)
        } else {
            super::super::strings(super::vitest::DEFAULT)
        })
    } else if script.is_some() {
        manager.script("test", script == Some("node --test"))
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
    super::quality::plan(target, &mut plan)?;
    Ok(plan)
}
