//! Translate typed builder intent into the versioned execution plan.
use crate::{builders, dependencies, executor, model::Workspace, records, snapshot, tasks};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn relative(root: &Path, path: &Path) -> Result<String> {
    let value = path
        .strip_prefix(root)
        .context("action path outside captured source")?
        .to_str()
        .context("non-UTF8 action path")?
        .replace('\\', "/");
    Ok(if value.is_empty() { ".".into() } else { value })
}

fn platform(image: &executor::Image) -> Value {
    json!({"os":image.os,"arch":image.arch})
}

/// Selection uses frozen declarations; execution must consume source containing
/// the same inventory bytes (including an inventory being added or removed).
pub(super) fn verify_inventory_source(
    workspace: &Workspace,
    source: &snapshot::Snapshot,
) -> Result<()> {
    let captured = source
        .entries
        .iter()
        .find(|entry| entry.path == "build.yaml")
        .and_then(|entry| entry.digest.as_ref());
    if captured != workspace.declarations.source_digest.as_ref() {
        bail!("build.yaml changed between discovery and source capture; rerun the build to select and plan from the same inventory");
    }
    Ok(())
}

pub(super) fn target_order(
    workspace: &Workspace,
    selected: &BTreeSet<String>,
) -> Result<Vec<String>> {
    let configs = &workspace.declarations.targets;
    for (id, c) in configs.iter().filter(|(id, _)| selected.contains(*id)) {
        if !c.matrix.is_empty() || c.container.is_some() || c.bindings.is_some() {
            bail!("{id}: platform expansion and packaging options are not implemented yet");
        }
    }
    let mut pending = selected.clone();
    let mut done = Vec::new();
    while !pending.is_empty() {
        let ready = pending
            .iter()
            .find(|id| {
                configs.get(*id).is_none_or(|c| {
                    c.depends_on
                        .iter()
                        .chain(c.materialize.iter().map(|m| &m.from))
                        .all(|d| done.contains(d))
                })
            })
            .cloned();
        let id = ready.context("target dependency cycle")?;
        pending.remove(&id);
        done.push(id);
    }
    Ok(done)
}

fn action(
    id: &str,
    target: &str,
    operation: &str,
    argv: Vec<String>,
    cwd: &str,
    env: &BTreeMap<String, String>,
    execution: (&executor::Image, &str, &executor::Mode),
) -> Value {
    let (image, source, mode) = execution;
    let argv = mode
        .argv(&format!("{}/{}", image.os, image.arch))
        .unwrap_or(argv);
    let mut action = json!({"id":id,"target":target,"operation":operation,"dependsOn":[],"argv":argv,"cwd":cwd,"env":env,
        "tools":[target],"executionPlatform":platform(image),"targetPlatform":platform(image),
        "inputs":[{"kind":"tree","digest":source,"mount":"workspace"}],"outputs":[],"reports":[],
        "required":true,"cacheable":false,"network":"none",
        "limits":{"timeoutSeconds":600,"cpu":2,"memoryBytes":2147483648u64,"outputBytes":16777216}});
    action["extensions"]["oyzu.dev/executor"] = json!(mode);
    action
}

/// Pure planning after source capture and image resolution; never invokes project code.
pub fn plan(
    workspace: &Workspace,
    source: &snapshot::Snapshot,
    images: &BTreeMap<String, executor::Image>,
) -> Result<Value> {
    plan_with_dependencies(workspace, source, images, &BTreeMap::new())
}

pub(super) fn plan_with_dependencies(
    workspace: &Workspace,
    source: &snapshot::Snapshot,
    images: &BTreeMap<String, executor::Image>,
    dependencies: &BTreeMap<String, dependencies::Prepared>,
) -> Result<Value> {
    verify_inventory_source(workspace, source)?;
    let mut intents = BTreeMap::new();
    for id in workspace.targets.keys() {
        intents.insert(
            id.clone(),
            intent(workspace, id, source, dependencies.get(id))?,
        );
    }
    compile(workspace, source, images, dependencies, &intents)
}

pub(super) fn intent(
    workspace: &Workspace,
    id: &str,
    source: &snapshot::Snapshot,
    dependency: Option<&dependencies::Prepared>,
) -> Result<builders::BuilderPlan> {
    let target = &workspace.targets[id];
    let intent = builders::get(&target.builder)?.plan(builders::PlanningContext {
        target,
        source,
        dependencies: dependency,
    })?;
    intent
        .validate()
        .with_context(|| format!("{id}: invalid builder output contract"))?;
    Ok(intent)
}

pub(super) fn compile(
    workspace: &Workspace,
    source: &snapshot::Snapshot,
    images: &BTreeMap<String, executor::Image>,
    dependencies: &BTreeMap<String, dependencies::Prepared>,
    intents: &BTreeMap<String, builders::BuilderPlan>,
) -> Result<Value> {
    verify_inventory_source(workspace, source)?;
    let selected = intents.keys().cloned().collect();
    let order = target_order(workspace, &selected)?;
    let mut planned = Vec::new();
    let mut target_records = Vec::new();
    let mut artifacts = Vec::new();
    let mut tools = Vec::new();
    let builder_digest = snapshot::file_digest(&std::env::current_exe()?)?;
    let configs = &workspace.declarations.targets;
    let mut materialized = BTreeMap::new();
    let task_graph = super::task_graph::TaskGraph::new(workspace, intents)?;
    for id in order {
        let target = &workspace.targets[&id];
        let image = images
            .get(&id)
            .with_context(|| format!("{id}: no resolved toolchain image"))?;
        if let Some(required) = configs.get(&id).and_then(|c| c.platform.as_ref()) {
            let actual = format!("{}/{}", image.os, image.arch);
            if required != &actual {
                bail!("{id}: required platform {required} does not match resolved toolchain {actual}; cross-platform execution is not implemented yet");
            }
        }
        let builder = builders::get(&target.builder)?;
        if let Some(config) = workspace.configuration.get(&id) {
            crate::config::enforcement::execution_preflight(config, builder.descriptor().tools)?;
        }
        let intent = &intents[&id];
        let cwd = relative(&workspace.root, &target.path)?;
        let projection = intent
            .source_files
            .as_ref()
            .map(|files| snapshot::Projection::new(&cwd, files))
            .transpose()?;
        if let Some(config) = configs.get(&id) {
            materialized.insert(
                id.clone(),
                super::materialization::plan(
                    &config.materialize,
                    &cwd,
                    &artifacts,
                    images,
                    &id,
                    source,
                    projection.as_ref(),
                )?,
            );
        }
        let mut record = json!({"id":id,"builder":target.builder,"builderDigest":builder_digest,"path":cwd,"variant":{},"platform":platform(image)});
        if !target.discovery.is_empty() {
            record["extensions"]["oyzu.dev/discovery"] = json!(target.discovery);
        }
        if let Some(coverage) = &intent.coverage {
            record["extensions"]["oyzu.dev/coverage-applicability"] = json!(coverage);
        }
        if let Some(projection) = projection {
            record["extensions"]["oyzu.dev/source-projection"] = json!(projection);
        }
        if let Some(config) = workspace.configuration.get(&id) {
            record["extensions"]["oyzu.dev/configuration"] =
                json!({"digest": config.digest, "profile": config.profile});
        }
        target_records.push(record);
        tools.push(json!({"id":id,"version":image.reference,"digest":image.digest,"platform":platform(image)}));
        for command in &intent.prepare {
            planned.push(action(
                &format!("{id}:{}", command.operation),
                &id,
                command.operation,
                command.argv.clone(),
                &cwd,
                &intent.env,
                (image, &source.digest, &command.execution),
            ));
        }
        let operation_contracts: BTreeMap<_, _> = intent
            .tasks
            .iter()
            .map(|(name, plan)| (super::task_graph::operation(workspace, &id, name), plan))
            .collect();
        for step in task_graph
            .ordered
            .iter()
            .filter(|step| task_graph.owners[*step] == id)
        {
            let task = &workspace.tasks[step];
            if task.mutates_source {
                bail!("{step}: mutating formatter cannot run as a build check");
            }
            // The operation owns its required evidence even when TOML
            // replaces its body. Runner-specific adaptation belongs to the
            // builder; the engine never guesses how to modify a command.
            // A native operation can first appear as another operation's
            // prerequisite. Its report contract follows its identity.
            let contract = operation_contracts.get(step).copied();
            let native = contract.filter(|_| task.provider == target.manager);
            let mut bindings =
                crate::reports::bindings::bind(&workspace.root, &id, task, contract)?;
            let mut env = intent.env.clone();
            env.extend(task.env.clone());
            if let Some(owner) = tasks::hook_owner(task).and_then(|id| workspace.tasks.get(&id)) {
                let reports = crate::reports::bindings::bind(
                    &workspace.root,
                    &id,
                    owner,
                    operation_contracts.get(&owner.id()).copied(),
                )?;
                env.extend(reports.env);
            }
            env.extend(bindings.env);
            if let Some(config) = workspace.configuration.get(&id) {
                config.validate_environment(&env)?;
            }
            for (name, value) in &intent.fixed_env {
                if env.get(name) != Some(value) {
                    bail!("{step}: {name} must remain {value} for the captured builder capability");
                }
            }
            let instrumented = contract
                .filter(|_| native.is_none())
                .and_then(|_| builder.instrument_override(target, task, &env));
            let native_reporting = native.is_some() || instrumented.is_some();
            let argv = native.map_or_else(
                || instrumented.unwrap_or_else(|| task.argv.clone()),
                |v| v.argv.clone(),
            );
            if !native_reporting {
                for source in bindings.sources.values_mut() {
                    *source = crate::reports::ReportSource::File;
                }
            }
            let boundary = super::task_graph::completion(workspace, step);
            let mut a = action(
                step,
                &id,
                &task.name,
                argv,
                &relative(&workspace.root, &task.cwd)?,
                &env,
                (
                    image,
                    &source.digest,
                    native.map_or(&executor::Mode::Process, |v| &v.execution),
                ),
            );
            a["reports"] = json!(bindings.intents);
            a["extensions"]["oyzu.dev/report-paths"] = json!(bindings.paths);
            a["extensions"]["oyzu.dev/stdout-must-be-empty"] = json!(task.stdout_must_be_empty);
            a["extensions"]["oyzu.dev/report-sources"] = json!(bindings.sources);
            a["extensions"]["oyzu.dev/report-inputs"] = json!(bindings.inputs);
            a["extensions"]["oyzu.dev/collect-after"] = json!(boundary);
            planned.push(a);
        }
        let producer = format!("{id}:{}", intent.package.operation);
        let mut package = action(
            &producer,
            &id,
            intent.package.operation,
            intent.package.argv.clone(),
            &cwd,
            &intent.env,
            (image, &source.digest, &intent.package.execution),
        );
        let mut outputs = Vec::new();
        for artifact in &intent.artifacts {
            let artifact_id = format!("{id}/{}", artifact.name);
            outputs.push(artifact_id.clone());
            artifacts.push(json!({"id":artifact_id,"target":id,"variant":{},"name":artifact.name,"producer":producer,"kind":artifact.kind,"version":artifact.version.as_ref().unwrap_or(&intent.version),"mediaType":artifact.media_type,"path":format!("{id}/artifacts/{}",artifact.filename)}));
        }
        package["outputs"] = json!(outputs);
        planned.push(package);
    }
    // Targets have private workspaces. Preserve their internal mutation/hook
    // sequence without inventing dependencies between unrelated targets.
    let final_actions: BTreeMap<String, String> = planned
        .iter()
        .map(|action| {
            Ok((
                action["target"]
                    .as_str()
                    .context("missing action target")?
                    .into(),
                action["id"].as_str().context("missing action id")?.into(),
            ))
        })
        .collect::<Result<_>>()?;
    let preparation: BTreeMap<_, _> = planned
        .iter()
        .filter(|a| {
            !task_graph.owners.contains_key(a["id"].as_str().unwrap())
                && final_actions[a["target"].as_str().unwrap()] != a["id"].as_str().unwrap()
        })
        .map(|a| {
            (
                a["target"].as_str().unwrap().to_owned(),
                a["id"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let mut previous: BTreeMap<String, String> = BTreeMap::new();
    for a in &mut planned {
        if let Some(inputs) = a["target"].as_str().and_then(|id| materialized.get(id)) {
            a["inputs"].as_array_mut().unwrap().extend(inputs.clone());
        }
        if let Some(dependency) = a["target"].as_str().and_then(|id| dependencies.get(id)) {
            a["inputs"].as_array_mut().unwrap().push(
                json!({"kind":"dependency","digest":dependency.digest,"mount":"dependencies"}),
            );
        }
        let mut prerequisites: BTreeSet<String> = a["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|input| input["kind"] == "artifact")
            .filter_map(|input| input["producer"].as_str().map(str::to_owned))
            .collect();
        let target = a["target"]
            .as_str()
            .context("missing action target")?
            .to_owned();
        let action_id = a["id"].as_str().context("missing action id")?;
        if task_graph.owners.contains_key(action_id) {
            let edges = task_graph
                .dependencies
                .get(action_id)
                .cloned()
                .unwrap_or_default();
            if !edges
                .iter()
                .any(|id| task_graph.owners.get(id) == Some(&target))
            {
                prerequisites.extend(preparation.get(&target).cloned());
            }
            prerequisites.extend(edges);
        } else if final_actions[&target] == action_id {
            // Package only after every selected task owned by this target,
            // including custom tasks requested by another target, has finished.
            let owned: BTreeSet<_> = task_graph
                .owners
                .iter()
                .filter(|(_, owner)| **owner == target)
                .map(|(id, _)| id.clone())
                .collect();
            if owned.is_empty() {
                prerequisites.extend(preparation.get(&target).cloned());
            } else {
                prerequisites.extend(
                    owned
                        .iter()
                        .filter(|id| {
                            !task_graph.dependencies.iter().any(|(consumer, edges)| {
                                owned.contains(consumer) && edges.contains(*id)
                            })
                        })
                        .cloned(),
                );
            }
        } else {
            prerequisites.extend(previous.get(&target).cloned());
            previous.insert(target.clone(), action_id.into());
        }
        if let Some(config) = configs.get(&target) {
            for dependency in &config.depends_on {
                prerequisites.insert(
                    final_actions
                        .get(dependency)
                        .context("target dependency has no final action")?
                        .clone(),
                );
            }
        }
        if let Some(config) = a["target"]
            .as_str()
            .and_then(|id| workspace.configuration.get(id))
        {
            a["extensions"]["oyzu.dev/configuration-digest"] = json!(config.digest);
            a["extensions"]["oyzu.dev/coverage-minimum"] = config
                .get("checks.coverageMinimum")
                .cloned()
                .unwrap_or(json!(0));
        }
        a["dependsOn"] = json!(prerequisites);
    }
    // Validate combined stage, hook, task and target edges before returning a
    // runnable plan. Task-only cycle checks cannot see all these relationships.
    super::scheduling::Schedule::new(&planned, 1)?;
    validate_required_checks(workspace, &selected, &planned)?;
    let managed = workspace
        .configuration
        .iter()
        .filter(|(id, _)| selected.contains(*id))
        .map(|(_, config)| config)
        .find_map(|config| config.management.as_ref());
    let required: BTreeSet<_> = workspace
        .configuration
        .iter()
        .filter(|(id, _)| selected.contains(*id))
        .map(|(_, config)| config)
        .filter_map(|config| config.get("checks.required").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let digests: BTreeMap<_, _> = workspace
        .configuration
        .iter()
        .filter(|(id, _)| selected.contains(*id))
        .map(|(id, config)| (id, &config.digest))
        .collect();
    let mut policy = json!({"mode":if managed.is_some(){"managed"}else{"standalone"},"enforcementDigest":records::digest("oyzu.policy.v1alpha1",&json!({"configuration":digests,"productionEligible":false,"executor":"docker-v1"}))?,"requiredChecks":required});
    if let Some(management) = managed {
        policy["extensions"]["oyzu.dev/configuration-policy"] = management.clone();
    }
    // A nested target cannot raise the invocation's shared concurrency ceiling.
    let jobs = workspace
        .root_configuration
        .iter()
        .chain(
            workspace
                .configuration
                .iter()
                .filter(|(id, _)| selected.contains(*id))
                .map(|(_, config)| config),
        )
        .filter_map(|config| config.get("build.jobs").and_then(Value::as_u64))
        .min()
        .unwrap_or(1);
    Ok(
        json!({"schemaVersion":"v1alpha1","kind":"build-plan","extensions":{"oyzu.dev/execution":{"jobs":jobs}},"source":{"treeDigest":source.digest,"commit":null,"dirty":true},"policy":policy,"tools":tools,"targets":target_records,"actions":planned,"artifacts":artifacts}),
    )
}

pub(super) fn resolve_images(
    workspace: &Workspace,
    overrides: &[String],
    selected: &BTreeSet<String>,
) -> Result<BTreeMap<String, executor::Image>> {
    let mut refs = BTreeMap::new();
    for value in overrides {
        let (manager, reference) = value
            .split_once('=')
            .context("--image requires manager=image")?;
        if reference.is_empty() || !workspace.targets.values().any(|t| t.manager == manager) {
            bail!("invalid or unused image override {value}");
        }
        if refs.insert(manager, reference).is_some() {
            bail!("duplicate image override for {manager}");
        }
    }
    let mut images = BTreeMap::new();
    for (id, target) in workspace
        .targets
        .iter()
        .filter(|(id, _)| selected.contains(*id))
    {
        let builder = builders::get(&target.builder)?;
        let default = builder.toolchain(target)?;
        let reference = refs
            .get(target.manager.as_str())
            .copied()
            .unwrap_or(default);
        images.insert(
            id.clone(),
            executor::resolve_for(reference, builder.executor_profile())?,
        );
    }
    Ok(images)
}

fn validate_required_checks(
    workspace: &Workspace,
    selected: &BTreeSet<String>,
    actions: &[Value],
) -> Result<()> {
    for (id, config) in workspace
        .configuration
        .iter()
        .filter(|(id, _)| selected.contains(*id))
    {
        let checks = config
            .get("checks.required")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let coverage = config
            .get("checks.coverageMinimum")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        for check in checks
            .iter()
            .filter_map(Value::as_str)
            .chain((coverage > 0).then_some("coverage"))
        {
            let present = actions.iter().filter(|a| a["target"] == *id).any(|a| {
                if matches!(check, "tests" | "coverage") {
                    let kind = if check == "tests" { "test" } else { "coverage" };
                    a["reports"]
                        .as_array()
                        .is_some_and(|reports| reports.iter().any(|r| r["kind"] == kind))
                } else {
                    a["operation"] == check
                }
            });
            if !present {
                bail!("CONFIG_OVERRIDE_DENIED: {id} cannot satisfy required {check} check");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn helm_native_suite_output_matches_named_report_collection() {
        let root = tempfile::tempdir().unwrap();
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/builds/helm-chart/project");
        let input = root.path().join("input");
        snapshot::capture(&fixture, &input).unwrap();
        fs::create_dir(input.join("chart/tests")).unwrap();
        fs::write(
            input.join("chart/tests/example_test.yaml"),
            "suite: example\n",
        )
        .unwrap();
        let capture = root.path().join("source");
        let source = snapshot::capture(&input, &capture).unwrap();
        let workspace = crate::discovery::discover(&capture).unwrap();
        let image = executor::Image {
            reference: "helm:test".into(),
            digest: format!("sha256:{}", "1".repeat(64)),
            os: "linux".into(),
            arch: "amd64".into(),
        };
        let dependency = dependencies::Prepared {
            root: capture,
            digest: format!("sha256:{}", "2".repeat(64)),
            record: json!({}),
        };
        let plan = plan_with_dependencies(
            &workspace,
            &source,
            &BTreeMap::from([("project".into(), image)]),
            &BTreeMap::from([("project".into(), dependency)]),
        )
        .unwrap();
        let test = plan["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == "project:test")
            .unwrap();
        let paths = &test["extensions"]["oyzu.dev/report-paths"];
        for id in ["project:test", "project:test:unittest"] {
            let destination = format!("/out/{}", paths[id].as_str().unwrap());
            assert!(test["argv"]
                .as_array()
                .unwrap()
                .contains(&json!(destination)));
            assert_eq!(test["extensions"]["oyzu.dev/report-sources"][id], "file");
        }
        assert_eq!(test["reports"].as_array().unwrap().len(), 2);
        assert!(test["reports"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["required"] == true));
    }

    #[test]
    fn npm_workspace_plans_keep_each_artifact_and_required_report_under_override() {
        for (custom, format_stage, public_root) in [
            (false, "format-check", false),
            (true, "format-check", false),
            (false, "format:check", false),
            (false, "format-check", true),
            (true, "format-check", true),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("examples/builds/node-workspace/project");
            let source = snapshot::capture(&fixture, &temp.path().join("source")).unwrap();
            if public_root {
                let manifest = temp.path().join("source/package.json");
                let mut package = records::read(&manifest).unwrap();
                package["private"] = json!(false);
                records::write(&manifest, &package).unwrap();
            }
            if format_stage == "format:check" {
                let manifest = temp.path().join("source/package.json");
                let mut package = records::read(&manifest).unwrap();
                package["scripts"]["format:check"] = json!("native-root-format-check");
                records::write(&manifest, &package).unwrap();
            }
            if custom {
                fs::write(
                    temp.path().join("source/oyzu.toml"),
                    "[tasks.\"project:test\"]\nargv=['custom-test']\n",
                )
                .unwrap();
            }
            let workspace = crate::discovery::discover(&temp.path().join("source")).unwrap();
            let image = executor::Image {
                reference: "node:test".into(),
                digest: format!("sha256:{}", "1".repeat(64)),
                os: "linux".into(),
                arch: "amd64".into(),
            };
            let mut members = Vec::new();
            for name in ["app", "shared"] {
                let pkg =
                    records::read(&workspace.root.join(format!("packages/{name}/package.json")))
                        .unwrap();
                let edges = if name == "app" {
                    json!([{"name":"@oyzu-example/shared","target":"@oyzu-example/shared","kind":"prod","spec":"0.1.0"}])
                } else {
                    json!([])
                };
                members.push(json!({"name":pkg["name"],"path":format!("packages/{name}"),"version":pkg["version"],"private":pkg["private"].as_bool().unwrap_or(false),"scripts":pkg["scripts"],"dependencies":edges}));
            }
            let dependency = dependencies::Prepared {
                root: temp.path().into(),
                digest: format!("sha256:{}", "2".repeat(64)),
                record: json!({"extensions":{"oyzu.dev/npm":{"workspaces":{"schemaVersion":1,"members":members,"rootDependencies":[]}}}}),
            };
            let plan = plan_with_dependencies(
                &workspace,
                &source,
                &BTreeMap::from([("project".into(), image)]),
                &BTreeMap::from([("project".into(), dependency)]),
            )
            .unwrap();
            let actions = plan["actions"].as_array().unwrap();
            assert!(actions.iter().any(|a| a["id"] == "project:build"));
            for stage in ["lint", format_stage] {
                let operation = actions
                    .iter()
                    .find(|a| a["id"] == format!("project:{stage}"))
                    .unwrap();
                assert_eq!(
                    operation["argv"],
                    json!(["node", "/oyzu/npm-workspace-build.mjs", stage])
                );
            }
            assert_eq!(
                actions
                    .iter()
                    .filter(
                        |a| a["id"] == "project:format:check" || a["id"] == "project:format-check"
                    )
                    .count(),
                1
            );
            let test = actions.iter().find(|a| a["id"] == "project:test").unwrap();
            assert_eq!(
                test["reports"].as_array().unwrap().len(),
                if public_root { 6 } else { 4 }
            );
            assert!(test["reports"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["required"] == true));
            if custom {
                assert_eq!(test["argv"], json!(["custom-test"]));
            }
            let package = actions
                .iter()
                .find(|a| a["id"] == "project:package")
                .unwrap();
            assert_eq!(
                package["outputs"].as_array().unwrap().len(),
                if public_root { 3 } else { 2 }
            );
        }
    }

    #[test]
    fn qualified_docker_test_override_keeps_build_stage_and_required_evidence() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("Dockerfile"), "FROM scratch\n").unwrap();
        fs::write(
            root.path().join("oyzu.toml"),
            "[tasks.\"project:pre_test\"]\nargv=['prepare-test']\n[tasks.\"project:test\"]\nargv=['custom-test']\n[tasks.\"project:post_test\"]\nargv=['finish-test']\n",
        )
        .unwrap();
        let capture = tempfile::tempdir().unwrap();
        let source = snapshot::capture(root.path(), &capture.path().join("source")).unwrap();
        let workspace =
            crate::discovery::discover_with_shell(&capture.path().join("source"), Some("sh"))
                .unwrap();
        // The development task is not a stage until the captured builder supplies
        // its contract. An explicit body must keep that contract and its hooks.
        assert!(!workspace.tasks["project:test"].build_stage);
        let image = executor::Image {
            reference: "buildkit:test".into(),
            digest: format!("sha256:{}", "1".repeat(64)),
            os: "linux".into(),
            arch: "amd64".into(),
        };
        let metadata = json!({"schemaVersion":"v1alpha1","frontend":"dockerfile.v0","stages":[{"name":"","base":"scratch"}],"requirements":[],"context":{"files":["Dockerfile","oyzu.toml"]},"selection":{"targetPlatform":"linux/amd64","sourceDateEpoch":crate::executor::BUILDKIT_SOURCE_DATE_EPOCH}});
        let dependency = dependencies::Prepared {
            root: capture.path().into(),
            digest: format!("sha256:{}", "2".repeat(64)),
            record: json!({"targetPlatform":{"os":"linux","arch":"amd64"},"extensions":{"oyzu.dev/docker":{"metadata":metadata,"apparmorProfile":"oyzu-buildkit","dockerfileDigest":snapshot::file_digest(&workspace.root.join("Dockerfile")).unwrap()}}}),
        };
        let plan = plan_with_dependencies(
            &workspace,
            &source,
            &BTreeMap::from([("project".into(), image)]),
            &BTreeMap::from([("project".into(), dependency)]),
        )
        .unwrap();
        let actions = plan["actions"].as_array().unwrap();
        assert_eq!(
            actions
                .iter()
                .map(|a| a["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "project:build",
                "project:pre_test",
                "project:test",
                "project:post_test",
                "project:lint",
                "project:format-check",
                "project:package"
            ]
        );
        let test = &actions[2];
        assert_eq!(test["argv"], json!(["custom-test"]));
        assert_eq!(test["reports"].as_array().unwrap().len(), 1);
        assert_eq!(test["reports"][0]["required"], true);
        assert_eq!(
            test["extensions"]["oyzu.dev/report-sources"]["project:test"],
            "file"
        );
    }

    #[test]
    fn unknown_go_override_requires_files_instead_of_interpreting_arbitrary_stdout() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("go.mod"),
            "module example.test/demo\n\ngo 1.24\n",
        )
        .unwrap();
        fs::write(
            root.path().join("main.go"),
            "package main\nfunc main() {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("oyzu.toml"),
            "[tasks.\"project:test\"]\nargv=['custom-test']\n",
        )
        .unwrap();
        let capture = tempfile::tempdir().unwrap();
        let source = snapshot::capture(root.path(), &capture.path().join("source")).unwrap();
        let workspace =
            crate::discovery::discover_with_shell(&capture.path().join("source"), Some("sh"))
                .unwrap();
        let image = executor::Image {
            reference: "go:test".into(),
            digest: format!("sha256:{}", "1".repeat(64)),
            os: "linux".into(),
            arch: "amd64".into(),
        };
        let metadata = json!({"version":"go1.24","os":"linux","arch":"amd64","patterns":["./..."],"modules":["."],"binaries":[{"name":"demo","package":"example.test/demo","directory":"."}],"cgo":false});
        let mut dependency = dependencies::Prepared {
            root: capture.path().into(),
            digest: format!("sha256:{}", "2".repeat(64)),
            record: json!({"extensions":{"oyzu.dev/go-metadata":metadata}}),
        };
        let plan = plan_with_dependencies(
            &workspace,
            &source,
            &BTreeMap::from([("project".into(), image.clone())]),
            &BTreeMap::from([(
                "project".into(),
                dependencies::Prepared {
                    root: dependency.root.clone(),
                    digest: dependency.digest.clone(),
                    record: dependency.record.clone(),
                },
            )]),
        )
        .unwrap();
        let task = plan["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == "project:test")
            .unwrap();
        assert_eq!(task["argv"], json!(["custom-test"]));
        assert_eq!(task["reports"].as_array().unwrap().len(), 2);
        assert_eq!(
            task["extensions"]["oyzu.dev/report-sources"]["project:test"],
            "file"
        );
        // Captured native compiler facts survive root TOML environment overrides.
        dependency.record["extensions"]["oyzu.dev/go-metadata"]["cgo"] = json!(true);
        dependency.record["extensions"]["oyzu.dev/go-metadata"]["compiler"] = json!("gcc 12");
        dependency.record["extensions"]["oyzu.dev/go-metadata"]["compilerTarget"] =
            json!("x86_64-linux-gnu");
        fs::write(
            capture.path().join("source/oyzu.toml"),
            "[env]\nCGO_ENABLED='0'\n",
        )
        .unwrap();
        let workspace =
            crate::discovery::discover_with_shell(&capture.path().join("source"), Some("sh"))
                .unwrap();
        let failure = plan_with_dependencies(
            &workspace,
            &source,
            &BTreeMap::from([("project".into(), image)]),
            &BTreeMap::from([("project".into(), dependency)]),
        );
        assert!(failure
            .unwrap_err()
            .to_string()
            .contains("CGO_ENABLED must remain 1"));
    }
}
