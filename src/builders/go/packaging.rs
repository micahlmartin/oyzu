//! Native module archive records; preparation owns packaging, the engine collects.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Deserialize, Serialize)]
pub(super) struct Module {
    pub path: String,
    pub directory: String,
    pub version: String,
    pub zip: String,
    #[serde(rename = "mod")]
    pub module: String,
    pub info: String,
    pub sum: String,
    #[serde(rename = "goModSum")]
    pub go_mod_sum: String,
}

pub(super) fn read(root: &Path, members: &[String]) -> Result<Vec<Module>> {
    let modules: Vec<Module> =
        serde_json::from_slice(&fs::read(root.join("module-artifacts/inventory.json"))?)?;
    if modules.is_empty() || modules.len() != members.len() {
        bail!("Go module artifact inventory does not cover the captured workspace");
    }
    let mut names = BTreeSet::new();
    for (index, (module, member)) in modules.iter().zip(members).enumerate() {
        if &module.directory != member
            || module.path.is_empty()
            || !names.insert(&module.path)
            || !module.version.starts_with('v')
            || !module.version.contains("-dev.g")
            || !module.sum.starts_with("h1:")
            || !module.go_mod_sum.starts_with("h1:")
        {
            bail!("invalid native Go module identity");
        }
        for (filename, suffix) in [
            (&module.zip, "zip"),
            (&module.module, "mod"),
            (&module.info, "info"),
        ] {
            if filename != &format!("module-{index}-{}.{suffix}", module.version)
                || !crate::snapshot::portable(filename)
                || filename.contains('/')
            {
                bail!("invalid native Go module artifact path");
            }
            let metadata = fs::symlink_metadata(root.join("module-artifacts").join(filename))?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                bail!("native Go module artifact must be a regular file");
            }
        }
    }
    Ok(modules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builders::PlanningContext, dependencies::Prepared, discovery, snapshot};
    use serde_json::json;

    #[test]
    fn library_plans_native_module_outputs_and_preserves_test_and_quality_gates() {
        let root = tempfile::tempdir().unwrap();
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/builds/go-app/variants/library");
        let source = snapshot::capture(&fixture, &root.path().join("source")).unwrap();
        let mut workspace = discovery::discover(&root.path().join("source")).unwrap();
        let prepared_root = root.path().join("prepared");
        fs::create_dir_all(prepared_root.join("module-artifacts")).unwrap();
        let version = "v2.0.0-dev.g0123456789ab";
        for suffix in ["zip", "mod", "info"] {
            fs::write(
                prepared_root.join(format!("module-artifacts/module-0-{version}.{suffix}")),
                "native artifact placeholder",
            )
            .unwrap();
        }
        let mut inventory = json!([{"path":"example.com/oyzu/math/v2","directory":".","version":version,
            "zip":format!("module-0-{version}.zip"),"mod":format!("module-0-{version}.mod"),"info":format!("module-0-{version}.info"),
            "sum":"h1:native","goModSum":"h1:native"}]);
        crate::records::write(
            &prepared_root.join("module-artifacts/inventory.json"),
            &inventory,
        )
        .unwrap();
        let prepared = Prepared {
            root: prepared_root.clone(),
            digest: source.digest.clone(),
            record: json!({"extensions":{"oyzu.dev/go-metadata":{
            "version":"go1.24.13","os":"linux","arch":"amd64","patterns":["./..."],"modules":["."],"binaries":[],"cgo":false}}}),
        };
        for builder in ["go/app", "go/library"] {
            let target = workspace.targets.get_mut("project").unwrap();
            target.builder = builder.into();
            let plan = super::super::planning::plan(PlanningContext {
                target,
                source: &source,
                dependencies: Some(&prepared),
            })
            .unwrap();
            plan.validate().unwrap();
            assert_eq!(plan.artifacts.len(), 3);
            assert!(plan
                .artifacts
                .iter()
                .all(|a| a.version.as_deref() == Some(version)));
            assert!(plan.tasks["build"]
                .argv
                .iter()
                .any(|a| a.contains("go build -trimpath -buildvcs=false './...'")));
            assert_eq!(plan.tasks["test"].reports.len(), 2);
            assert!(plan.tasks.contains_key("lint"));
            assert!(workspace.tasks["project:format-check"].build_stage);
        }
        inventory[0]["zip"] = json!("../escaped.zip");
        crate::records::write(
            &prepared_root.join("module-artifacts/inventory.json"),
            &inventory,
        )
        .unwrap();
        assert!(read(&prepared_root, &[".".into()]).is_err());
    }
}
