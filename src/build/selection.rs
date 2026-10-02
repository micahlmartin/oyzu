//! Requested build targets and their declared/native task dependency closure.
//! Discovery remains workspace-wide; only selected targets are prepared. Native
//! plans may add required owners, but cannot remove a user's requested target.
use crate::{builders::BuilderPlan, model::Workspace, tasks};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Selection {
    pub targets: BTreeSet<String>,
    requested: BTreeSet<String>,
    all: BTreeSet<String>,
    explicit: bool,
}

impl Selection {
    pub fn new(workspace: &Workspace, requested: &[String]) -> Result<Self> {
        let explicit = !requested.is_empty();
        let all: BTreeSet<_> = workspace.targets.keys().cloned().collect();
        for id in requested {
            if !all.contains(id) {
                bail!(
                    "unknown build target {id}; available targets: {}",
                    all.iter().cloned().collect::<Vec<_>>().join(", ")
                );
            }
        }
        let requested = if requested.is_empty() {
            all.clone()
        } else {
            requested.iter().cloned().collect()
        };
        let mut selection = Self {
            targets: requested.clone(),
            requested,
            all,
            explicit,
        };
        selection.expand_declared(workspace)?;
        Ok(selection)
    }

    fn expand_declared(&mut self, workspace: &Workspace) -> Result<()> {
        let configuration = &workspace.declarations.targets;
        loop {
            let before = self.targets.len();
            for id in self.targets.clone() {
                if let Some(target) = configuration.get(&id) {
                    for dependency in target
                        .depends_on
                        .iter()
                        .chain(target.materialize.iter().map(|input| &input.from))
                    {
                        if !self.all.contains(dependency) {
                            bail!("{id}: unknown build dependency {dependency}");
                        }
                        self.targets.insert(dependency.clone());
                    }
                }
            }
            if before == self.targets.len() {
                return Ok(());
            }
        }
    }

    pub fn expand(
        &mut self,
        workspace: &Workspace,
        plans: &BTreeMap<String, BuilderPlan>,
    ) -> Result<()> {
        let mut pending: Vec<_> = plans
            .iter()
            .flat_map(|(id, plan)| super::task_graph::roots(workspace, id, plan))
            .collect();
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            let task = &workspace.tasks[&id];
            if !task.target.is_empty() {
                self.targets.insert(task.target.clone());
            }
            for dependency in &task.depends_on {
                pending.push(tasks::resolve(workspace, dependency)?);
            }
            if !tasks::is_hook(task) {
                for hook in [tasks::pre_hook(task), tasks::post_hook(task)] {
                    if workspace.tasks.contains_key(&hook) {
                        pending.push(hook);
                    }
                }
            }
        }
        self.expand_declared(workspace)
    }

    pub fn record(&self) -> Value {
        json!({"mode":if self.explicit {"explicit"} else {"all"},"requested":self.requested,
            "selected":self.targets,"excluded":self.all.difference(&self.targets).map(|id| json!({"target":id,"reason":"outside-selection"})).collect::<Vec<_>>()})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{discovery, executor, snapshot};
    use std::{fs, path::Path};

    fn fixture(root: &Path, yaml: &str, toml: &str) {
        for id in ["api", "schema", "assets", "unused"] {
            fs::create_dir(root.join(id)).unwrap();
            fs::write(
                root.join(id).join("package.json"),
                r#"{"name":"demo","version":"1.0.0","scripts":{"test":"node --test"}}"#,
            )
            .unwrap();
        }
        fs::write(root.join("build.yaml"), yaml).unwrap();
        fs::write(root.join("oyzu.toml"), toml).unwrap();
    }

    const TARGETS: &str = "api: {uses: node/package, path: api}\nschema: {uses: node/package, path: schema}\nassets: {uses: node/package, path: assets}\nunused: {uses: node/package, path: unused}\n";

    #[test]
    fn selection_uses_the_discovered_inventory_after_its_file_changes() {
        let root = tempfile::tempdir().unwrap();
        let yaml = TARGETS.replace(
            "api: {uses: node/package, path: api}",
            "api: {uses: node/package, path: api, depends_on: [schema]}",
        );
        fixture(root.path(), &yaml, "");
        let workspace = discovery::discover(root.path()).unwrap();
        fs::write(root.path().join("build.yaml"), "invalid: [changed").unwrap();
        let selected = Selection::new(&workspace, &["api".into()]).unwrap();
        assert_eq!(
            selected.targets,
            BTreeSet::from(["api".into(), "schema".into()])
        );
        assert_eq!(
            super::super::planning::target_order(&workspace, &selected.targets).unwrap(),
            vec!["schema", "api"]
        );
    }

    #[test]
    fn requests_preserve_explicit_mode_deduplicate_and_reject_unknown_targets() {
        let root = tempfile::tempdir().unwrap();
        fixture(root.path(), TARGETS, "");
        let workspace = discovery::discover(root.path()).unwrap();
        let all = Selection::new(&workspace, &[]).unwrap();
        assert_eq!(all.record()["mode"], "all");
        let names = ["api", "schema", "assets", "unused", "api"].map(str::to_owned);
        let explicit = Selection::new(&workspace, &names).unwrap();
        assert_eq!(explicit.record()["mode"], "explicit");
        assert_eq!(explicit.targets, all.targets);
        assert!(Selection::new(&workspace, &["missing".into()])
            .err()
            .unwrap()
            .to_string()
            .contains("unknown build target"));
    }

    #[test]
    fn task_hooks_and_declared_inputs_expand_to_a_fixed_point() {
        let root = tempfile::tempdir().unwrap();
        let yaml = TARGETS.replace("schema: {uses: node/package, path: schema}",
            "schema: {uses: node/package, path: schema, materialize: [{from: assets, to: inputs/assets.tgz}]}");
        fixture(root.path(), &yaml,
            "[tasks.\"api:pre_test\"]\nargv=['echo','prepare']\ndepends_on=['schema:verify']\n[tasks.\"schema:verify\"]\nargv=['echo','verify']\n[tasks.test]\nargv=['wrong-root-override']\n");
        let snapshot_root = tempfile::tempdir().unwrap();
        let source = snapshot::capture(root.path(), &snapshot_root.path().join("source")).unwrap();
        let workspace = discovery::discover_with_shell(root.path(), Some("sh")).unwrap();
        let mut selection = Selection::new(&workspace, &["api".into()]).unwrap();
        assert_eq!(selection.targets, BTreeSet::from(["api".into()]));
        let mut plans = BTreeMap::new();
        loop {
            for id in selection.targets.clone() {
                if !plans.contains_key(&id) {
                    plans.insert(
                        id.clone(),
                        super::super::planning::intent(&workspace, &id, &source, None).unwrap(),
                    );
                }
            }
            selection.expand(&workspace, &plans).unwrap();
            if plans.len() == selection.targets.len() {
                break;
            }
        }
        assert_eq!(
            selection.targets,
            BTreeSet::from(["api".into(), "schema".into(), "assets".into()])
        );
        let images = selection
            .targets
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    executor::Image {
                        reference: "node:test".into(),
                        digest: format!("sha256:{}", "1".repeat(64)),
                        os: "linux".into(),
                        arch: "amd64".into(),
                    },
                )
            })
            .collect();
        let plan =
            super::super::planning::compile(&workspace, &source, &images, &BTreeMap::new(), &plans)
                .unwrap();
        assert_eq!(plan["targets"].as_array().unwrap().len(), 3);
        let actions = plan["actions"].as_array().unwrap();
        assert!(actions.iter().any(|a| a["id"] == "schema:verify"));
        assert!(actions.iter().any(|a| a["id"] == "api:test"));
        assert!(!actions
            .iter()
            .any(|a| a["id"] == "test" || a["target"] == "unused"));
        assert_eq!(
            selection.record()["excluded"],
            json!([{"target":"unused","reason":"outside-selection"}])
        );
    }

    #[test]
    fn single_selection_keeps_workspace_names_and_selected_configuration_obligations() {
        let root = tempfile::tempdir().unwrap();
        fixture(
            root.path(),
            TARGETS,
            "[build]\njobs=4\n[tasks.test]\nargv=['must-not-run']\n",
        );
        fs::write(
            root.path().join("api/oyzu.toml"),
            "[build]\njobs=2\n[checks]\nrequired=['lint']\n",
        )
        .unwrap();
        fs::write(root.path().join("unused/oyzu.toml"), "[build]\njobs=1\n").unwrap();
        let captured = tempfile::tempdir().unwrap();
        let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
        let workspace = discovery::discover_with_shell(root.path(), Some("sh")).unwrap();
        let mut intents = BTreeMap::from([(
            "api".into(),
            super::super::planning::intent(&workspace, "api", &source, None).unwrap(),
        )]);
        let images = BTreeMap::from([(
            "api".into(),
            executor::Image {
                reference: "node:test".into(),
                digest: format!("sha256:{}", "1".repeat(64)),
                os: "linux".into(),
                arch: "amd64".into(),
            },
        )]);
        let plan = super::super::planning::compile(
            &workspace,
            &source,
            &images,
            &BTreeMap::new(),
            &intents,
        )
        .unwrap();
        assert_eq!(plan["extensions"]["oyzu.dev/execution"]["jobs"], 2);
        assert_eq!(plan["policy"]["requiredChecks"], json!(["lint"]));
        let actions = plan["actions"].as_array().unwrap();
        assert!(actions.iter().any(|a| a["id"] == "api:test"));
        assert!(!actions.iter().any(|a| a["id"] == "test"));
        // A selected builder still must supply the checks its configuration requires.
        intents
            .get_mut("api")
            .unwrap()
            .stages
            .retain(|stage| *stage != "lint");
        assert!(super::super::planning::compile(
            &workspace,
            &source,
            &images,
            &BTreeMap::new(),
            &intents
        )
        .unwrap_err()
        .to_string()
        .contains("cannot satisfy required lint check"));
    }

    #[test]
    fn static_dependencies_are_transitive_and_cycles_still_fail() {
        let root = tempfile::tempdir().unwrap();
        let yaml = TARGETS
            .replace("path: api}", "path: api, depends_on: [schema]}")
            .replace("path: schema}", "path: schema, depends_on: [assets]}");
        fixture(root.path(), &yaml, "");
        let workspace = discovery::discover(root.path()).unwrap();
        let selection = Selection::new(&workspace, &["api".into()]).unwrap();
        assert_eq!(
            super::super::planning::target_order(&workspace, &selection.targets).unwrap(),
            ["assets", "schema", "api"]
        );
        fs::write(
            root.path().join("build.yaml"),
            yaml.replace("path: assets}", "path: assets, depends_on: [api]}"),
        )
        .unwrap();
        assert_eq!(
            super::super::planning::target_order(&workspace, &selection.targets).unwrap(),
            ["assets", "schema", "api"]
        );
        let workspace = discovery::discover(root.path()).unwrap();
        assert!(
            super::super::planning::target_order(&workspace, &selection.targets)
                .unwrap_err()
                .to_string()
                .contains("cycle")
        );
    }
}
