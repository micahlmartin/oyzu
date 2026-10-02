//! Expand captured runtime declarations into isolated target/task instances.
//! Owns identity and edge matching, not native version checks or execution.
use crate::{
    model::{Target, Task, Workspace},
    names, tasks,
};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Mapping = BTreeMap<String, Vec<String>>;
const MAX_INSTANCES: usize = 256;
const MAX_TASKS: usize = 16_384;

fn root_name(name: &str) -> String {
    let mut prefix = String::new();
    let mut rest = name;
    while let Some((hook, tail)) = rest
        .split_once('_')
        .filter(|(hook, _)| matches!(*hook, "pre" | "post"))
    {
        prefix.push_str(hook);
        prefix.push('_');
        rest = tail;
    }
    format!("{prefix}root-{rest}")
}

fn compatible(consumer: &Target, producer: &Target) -> bool {
    consumer.variant.iter().all(|(axis, value)| {
        producer
            .variant
            .get(axis)
            .is_none_or(|other| other == value)
    })
}

fn matches(
    mapping: &Mapping,
    targets: &BTreeMap<String, Target>,
    owner: &Target,
    name: &str,
) -> Result<Vec<String>> {
    let candidates = mapping
        .get(name)
        .with_context(|| format!("unknown target {name}"))?;
    let selected: Vec<_> = candidates
        .iter()
        .filter(|id| compatible(owner, &targets[*id]))
        .cloned()
        .collect();
    if selected.is_empty() {
        bail!("{}: no compatible runtime variant of {name}", owner.name);
    }
    Ok(selected)
}

fn task_dependencies(
    original: &Workspace,
    mapping: &Mapping,
    targets: &BTreeMap<String, Target>,
    owner: Option<&Target>,
    task: &mut Task,
    single: bool,
) -> Result<()> {
    let mut dependencies = Vec::new();
    for request in &task.depends_on {
        // Preserve unresolved declarations for ordinary selected-task admission;
        // an unused custom task must not fail an unrelated build.
        let Ok(id) = tasks::resolve(original, request) else {
            dependencies.push(request.clone());
            continue;
        };
        let dependency = &original.tasks[&id];
        if dependency.target.is_empty() {
            if single {
                dependencies.push(format!(
                    "{}:{}",
                    owner.context("missing variant task owner")?.name,
                    root_name(&dependency.name)
                ));
            } else {
                dependencies.push(id);
            }
        } else {
            let candidates = if let Some(owner) = owner {
                matches(mapping, targets, owner, &dependency.target)?
            } else {
                mapping[&dependency.target].clone()
            };
            dependencies.extend(
                candidates
                    .into_iter()
                    .map(|target| format!("{target}:{}", dependency.name)),
            );
        }
    }
    task.depends_on = dependencies;
    Ok(())
}

/// The raw inventory digest remains unchanged: these are derived declarations,
/// not a second parse or a modification of the user's build.yaml.
pub(super) fn expand(workspace: &mut Workspace) -> Result<Mapping> {
    if !workspace
        .declarations
        .targets
        .values()
        .any(|c| c.matrix.len() == 1 && !c.matrix.contains_key("platform"))
    {
        return Ok(workspace
            .targets
            .keys()
            .map(|id| (id.clone(), vec![id.clone()]))
            .collect());
    }
    let original = workspace.clone();
    let mut mapping = Mapping::new();
    let mut targets = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for (name, target) in &original.targets {
        let matrix = original.declarations.targets.get(name).map(|c| &c.matrix);
        // Platform propagation is a separate capability; retain its declaration
        // so selected-target admission rejects it rather than dropping an axis.
        let runtime = matrix.filter(|m| m.len() == 1 && !m.contains_key("platform"));
        let variants = if let Some(matrix) = runtime {
            let (axis, values) = matrix.iter().next().unwrap();
            if targets.len() + values.len() > MAX_INSTANCES {
                bail!("runtime matrix exceeds {MAX_INSTANCES} target instances");
            }
            values
                .iter()
                .map(|value| {
                    let mut variant = target.clone();
                    variant.name = names::scoped(name, &format!("{axis}-{value}"));
                    variant.variant.insert(axis.clone(), value.clone());
                    variant
                })
                .collect::<Vec<_>>()
        } else {
            vec![target.clone()]
        };
        if targets.len() + variants.len() > MAX_INSTANCES {
            bail!("runtime matrix exceeds {MAX_INSTANCES} target instances");
        }
        let mut ids = Vec::new();
        for mut target in variants {
            if !identities.insert(target.name.to_lowercase()) {
                bail!("runtime matrix target identity collision: {}", target.name);
            }
            for task in target.tasks.values_mut() {
                task.target = target.name.clone();
            }
            ids.push(target.name.clone());
            targets.insert(target.name.clone(), target);
        }
        ids.sort();
        mapping.insert(name.clone(), ids);
    }
    let single = original.targets.len() == 1;
    let mut declarations = BTreeMap::new();
    let mut configuration = BTreeMap::new();
    let mut expanded_tasks = BTreeMap::new();
    let mut root_overrides = BTreeMap::new();
    for (logical, ids) in &mapping {
        for id in ids {
            let target = &targets[id];
            if let Some(config) = original.configuration.get(logical) {
                configuration.insert(id.clone(), config.clone());
            }
            if let Some(config) = original.declarations.targets.get(logical) {
                let mut config = config.clone();
                if !target.variant.is_empty() {
                    config.matrix.clear();
                }
                config.depends_on = config
                    .depends_on
                    .iter()
                    .map(|name| matches(&mapping, &targets, target, name))
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect();
                for input in &mut config.materialize {
                    let candidates = matches(&mapping, &targets, target, &input.from)?;
                    if candidates.len() != 1 {
                        bail!(
                            "{id}: materialization from {} has ambiguous runtime variants",
                            input.from
                        );
                    }
                    input.from = candidates[0].clone();
                }
                declarations.insert(id.clone(), config);
            }
            // Preserve qualified identities. Root overrides have separate task
            // names and stage aliases, so explicit qualified dependencies still
            // refer to their original command and hook family.
            for root in [false, true] {
                for task in original.tasks.values().filter(|task| {
                    if root {
                        single && task.target.is_empty()
                    } else {
                        task.target == *logical
                    }
                }) {
                    if expanded_tasks.len() >= MAX_TASKS {
                        bail!("runtime matrix exceeds {MAX_TASKS} tasks");
                    }
                    let mut task = task.clone();
                    task.target = id.clone();
                    if root {
                        root_overrides.insert(task.id(), format!("{id}:{}", root_name(&task.name)));
                        task.name = root_name(&task.name);
                    }
                    task_dependencies(
                        &original,
                        &mapping,
                        &targets,
                        Some(target),
                        &mut task,
                        single,
                    )?;
                    if expanded_tasks.insert(task.id(), task).is_some() {
                        bail!("{id}: runtime matrix root task identity collision");
                    }
                }
            }
        }
    }
    if !single {
        for task in original
            .tasks
            .values()
            .filter(|task| task.target.is_empty())
        {
            if expanded_tasks.len() >= MAX_TASKS {
                bail!("runtime matrix exceeds {MAX_TASKS} tasks");
            }
            let mut task = task.clone();
            task_dependencies(&original, &mapping, &targets, None, &mut task, false)?;
            expanded_tasks.insert(task.id(), task);
        }
    }
    workspace.targets = targets;
    workspace.tasks = expanded_tasks;
    workspace.configuration = configuration;
    workspace.build_root_overrides = root_overrides;
    workspace.declarations.targets = declarations;
    Ok(mapping)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build::{planning, selection::Selection, task_graph},
        dependencies, discovery, executor, snapshot,
    };
    use serde_json::json;
    use std::fs;

    fn fixture(yaml: &str, toml: &str, paths: &[&str]) -> (tempfile::TempDir, Workspace) {
        let root = tempfile::tempdir().unwrap();
        for path in paths {
            let path = root.path().join(path);
            fs::create_dir_all(&path).unwrap();
            fs::write(
                path.join("package.json"),
                r#"{"name":"fixture","version":"1.0.0","scripts":{"test":"node --test"}}"#,
            )
            .unwrap();
        }
        fs::write(root.path().join("build.yaml"), yaml).unwrap();
        fs::write(root.path().join("oyzu.toml"), toml).unwrap();
        let workspace = discovery::discover_with_shell(root.path(), Some("sh")).unwrap();
        (root, workspace)
    }

    const MATRIX: &str = "app: {uses: node/package, matrix: {node: ['22.14.0', '24.14.1']}}\n";

    #[test]
    fn expanded_plan_preserves_root_and_qualified_tasks_and_separate_outputs() {
        let (root, mut workspace) = fixture(MATRIX,
            "[tasks.test]\nargv=['node','--test']\n[tasks.pre_test]\nargv=['echo','root-before']\n[tasks.post_test]\nargv=['echo','root-after']\n[tasks.\"app:pre_test\"]\nargv=['echo','qualified-before']\n[tasks.\"app:verify\"]\nargv=['echo','verify']\ndepends_on=['app:test']\n", &["."]);
        let captured = tempfile::tempdir().unwrap();
        let source = snapshot::capture(root.path(), &captured.path().join("source")).unwrap();
        let mut selection = Selection::new(&workspace, &["app".into()]).unwrap();
        let mapping = expand(&mut workspace).unwrap();
        selection.expand_variants(&workspace, &mapping).unwrap();
        assert_eq!(selection.record()["requested"], json!(["app"]));
        assert_eq!(selection.record()["variants"]["app"], json!(mapping["app"]));
        assert_eq!(mapping["app"].len(), 2);
        let mut intents = BTreeMap::new();
        let mut dependencies = BTreeMap::new();
        let mut images = BTreeMap::new();
        for id in &mapping["app"] {
            let target = &workspace.targets[id];
            assert_eq!(
                workspace.tasks[&format!("{id}:verify")].depends_on,
                [format!("{id}:test")]
            );
            assert_eq!(
                task_graph::operation(&workspace, id, "test"),
                format!("{id}:root-test")
            );
            assert_eq!(
                task_graph::operation_name(&workspace, &format!("{id}:root-test")),
                "test"
            );
            assert!(planning::intent(&workspace, id, &source, None)
                .err()
                .unwrap()
                .to_string()
                .contains("preflight evidence"));
            // Synthetic preparation is only a pure-planner fixture; native
            // runtime/engine evidence is independently tested through the CLI.
            let dependency = dependencies::Prepared {
                root: captured.path().into(),
                digest: format!("sha256:{}", "2".repeat(64)),
                record: json!({"extensions":{"oyzu.dev/npm":{"nodeVersion":target.variant["node"]}}}),
            };
            intents.insert(
                id.clone(),
                planning::intent(&workspace, id, &source, Some(&dependency)).unwrap(),
            );
            dependencies.insert(id.clone(), dependency);
            images.insert(
                id.clone(),
                executor::Image {
                    reference: format!("node:{}", target.variant["node"]),
                    digest: format!("sha256:{}", "1".repeat(64)),
                    os: "linux".into(),
                    arch: "amd64".into(),
                },
            );
        }
        let plan =
            planning::compile(&workspace, &source, &images, &dependencies, &intents).unwrap();
        assert_eq!(plan["targets"].as_array().unwrap().len(), 2);
        let artifacts = plan["artifacts"].as_array().unwrap();
        assert_ne!(artifacts[0]["path"], artifacts[1]["path"]);
        assert_ne!(artifacts[0]["variant"], artifacts[1]["variant"]);
        let actions = plan["actions"].as_array().unwrap();
        for id in &mapping["app"] {
            let test = actions
                .iter()
                .find(|a| a["id"] == format!("{id}:root-test"))
                .unwrap();
            assert_eq!(test["operation"], "test");
            assert!(test["argv"]
                .as_array()
                .unwrap()
                .iter()
                .any(|arg| arg == "--test-reporter=junit"));
            assert!(actions
                .iter()
                .any(|a| a["id"] == format!("{id}:pre_root-test")));
            assert!(actions
                .iter()
                .any(|a| a["id"] == format!("{id}:post_root-test")));
            assert!(!actions.iter().any(|a| a["id"] == format!("{id}:pre_test")));
        }
    }

    #[test]
    fn matching_variants_own_dependency_edges_and_shared_producers_stay_single() {
        let yaml = "app: {uses: node/package, path: app, matrix: {node: ['22.14.0','24.14.1']}, depends_on: [shared], materialize: [{from: lib, to: inputs/lib.tgz}]}\nlib: {uses: node/package, path: lib, matrix: {node: ['24.14.1','22.14.0']}}\nshared: {uses: node/package, path: shared}\n";
        let (_root, mut workspace) = fixture(
            yaml,
            "[tasks.\"app:pre_test\"]\nargv=['echo','before']\ndepends_on=['lib:test']\n",
            &["app", "lib", "shared"],
        );
        let mapping = expand(&mut workspace).unwrap();
        assert_eq!(mapping["shared"], ["shared"]);
        for id in &mapping["app"] {
            let config = &workspace.declarations.targets[id];
            assert_eq!(config.depends_on, ["shared"]);
            let producer = &config.materialize[0].from;
            assert_eq!(
                workspace.targets[id].variant,
                workspace.targets[producer].variant
            );
            assert_eq!(
                workspace.tasks[&format!("{id}:pre_test")].depends_on,
                [format!("{producer}:test")]
            );
        }
        let mut again = workspace.clone();
        expand(&mut again).unwrap();
        assert_eq!(
            again.targets.keys().collect::<Vec<_>>(),
            workspace.targets.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn ambiguous_artifact_variants_and_identity_collisions_are_rejected() {
        let yaml = "app: {uses: node/package, path: app, materialize: [{from: lib, to: inputs/lib.tgz}]}\nlib: {uses: node/package, path: lib, matrix: {node: ['22.14.0','24.14.1']}}\n";
        let (_root, mut workspace) = fixture(yaml, "", &["app", "lib"]);
        assert!(expand(&mut workspace)
            .unwrap_err()
            .to_string()
            .contains("ambiguous runtime variants"));
        let collision = names::scoped("app", "node-22.14.0");
        let yaml = format!("app: {{uses: node/package, path: app, matrix: {{node: ['22.14.0']}}}}\n{collision}: {{uses: node/package, path: other}}\n");
        let (_root, mut workspace) = fixture(&yaml, "", &["app", "other"]);
        assert!(expand(&mut workspace)
            .unwrap_err()
            .to_string()
            .contains("identity collision"));
    }

    #[test]
    fn expansion_is_bounded_and_never_discards_platform_axes() {
        let (_root, mut workspace) = fixture(MATRIX, "", &["."]);
        workspace
            .declarations
            .targets
            .get_mut("app")
            .unwrap()
            .matrix
            .insert(
                "node".into(),
                (0..257).map(|v| format!("22.0.{v}")).collect(),
            );
        assert!(expand(&mut workspace)
            .unwrap_err()
            .to_string()
            .contains("256 target instances"));
        workspace
            .declarations
            .targets
            .get_mut("app")
            .unwrap()
            .matrix
            .insert("platform".into(), vec!["linux/amd64".into()]);
        expand(&mut workspace).unwrap();
        assert!(workspace.declarations.targets["app"]
            .matrix
            .contains_key("platform"));
        assert!(planning::target_order(&workspace, &BTreeSet::from(["app".into()])).is_err());
    }

    #[test]
    fn selection_recomputes_the_variant_closure_instead_of_selecting_every_producer() {
        let yaml = "app: {uses: node/package, path: app, matrix: {node: ['22.14.0']}, materialize: [{from: lib, to: inputs/lib.tgz}]}\nlib: {uses: node/package, path: lib, matrix: {node: ['22.14.0','24.14.1']}}\n";
        let (_root, mut workspace) = fixture(yaml, "", &["app", "lib"]);
        let mut selection = Selection::new(&workspace, &["app".into()]).unwrap();
        let mapping = expand(&mut workspace).unwrap();
        selection.expand_variants(&workspace, &mapping).unwrap();
        assert_eq!(selection.targets.len(), 2);
        assert!(selection
            .targets
            .iter()
            .all(|id| workspace.targets[id].variant["node"] == "22.14.0"));
        assert_eq!(selection.record()["excluded"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn runtime_admission_does_not_silently_fall_back_to_the_default_image() {
        let (_root, mut workspace) = fixture(MATRIX, "", &["."]);
        let mapping = expand(&mut workspace).unwrap();
        let mut target = workspace.targets[&mapping["app"][0]].clone();
        let builder = crate::builders::get(&target.builder).unwrap();
        assert!(builder
            .variant_toolchain(&target)
            .unwrap()
            .ends_with(&format!("-node{}", target.variant["node"])));
        for invalid in [
            "22",
            "^22.14.0",
            "22.014.0",
            "../../other",
            "999999999999.0.0",
        ] {
            target.variant.insert("node".into(), invalid.into());
            assert!(builder
                .variant_toolchain(&target)
                .unwrap_err()
                .to_string()
                .contains("exact major.minor.patch"));
        }
        target.variant.insert("node".into(), "24.14.1".into());
        target.manager = "unknown".into();
        assert!(builder.variant_toolchain(&target).is_err());
    }
}
