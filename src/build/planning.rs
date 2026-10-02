//! Translate typed builder intent into the versioned execution plan.
use crate::{builders, config, dependencies, executor, model::Workspace, records, snapshot, tasks};
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

pub(super) fn target_order(workspace: &Workspace) -> Result<Vec<String>> {
    let configs = config::targets(&workspace.root)?.unwrap_or_default();
    for (id, c) in &configs {
        if !c.matrix.is_empty() || c.container.is_some() || c.bindings.is_some() {
            bail!("{id}: platform expansion and packaging options are not implemented yet");
        }
    }
    let mut pending: BTreeSet<_> = workspace.targets.keys().cloned().collect();
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
    let mut planned = Vec::new();
    let mut target_records = Vec::new();
    let mut artifacts = Vec::new();
    let mut tools = Vec::new();
    let builder_digest = snapshot::file_digest(&std::env::current_exe()?)?;
    let mut emitted = BTreeSet::new();
    let configs = config::targets(&workspace.root)?.unwrap_or_default();
    let mut materialized = BTreeMap::new();
    for id in target_order(workspace)? {
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
        let intent = builder.plan(builders::PlanningContext {
            target,
            source,
            dependencies: dependencies.get(&id),
        })?;
        intent
            .validate()
            .with_context(|| format!("{id}: invalid builder output contract"))?;
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
        let operation_id = |stage: &str| {
            if workspace.targets.len() == 1 && workspace.tasks.contains_key(stage) {
                stage.to_string()
            } else {
                format!("{id}:{stage}")
            }
        };
        let operation_contracts: BTreeMap<_, _> = intent
            .tasks
            .iter()
            .map(|(name, plan)| (operation_id(name), plan))
            .collect();
        let native_operations: BTreeSet<_> = operation_contracts
            .keys()
            .filter(|id| {
                workspace
                    .tasks
                    .get(*id)
                    .is_some_and(|task| task.provider == target.manager)
            })
            .cloned()
            .collect();
        for stage in &intent.stages {
            // Match public task lookup for a single-target workspace: explicit
            // root tasks own unqualified operations. Never fan a root override
            // out across multiple targets.
            let root_override =
                workspace.targets.len() == 1 && workspace.tasks.contains_key(*stage);
            let task_id = operation_id(stage);
            let Some(task) = workspace.tasks.get(&task_id) else {
                continue;
            };
            let provided = native_operations.contains(&task_id);
            if (task.availability.is_some() && !provided)
                || (!task.build_stage
                    && !root_override
                    && !operation_contracts.contains_key(&task_id))
            {
                continue;
            }
            let sequence = tasks::sequence_for_build(workspace, &task_id, &native_operations)?;
            for (position, step) in sequence.iter().enumerate() {
                if !emitted.insert(step.clone()) {
                    continue;
                }
                let task = &workspace.tasks[step];
                if task.mutates_source {
                    bail!("{step}: mutating formatter cannot run as a build check");
                }
                if !task.target.is_empty() && task.target != id {
                    bail!("{step}: cross-target task prerequisites require graph integration");
                }
                // The operation owns its required evidence even when TOML
                // replaces its body. Runner-specific adaptation belongs to the
                // builder; the engine never guesses how to modify a command.
                // A native operation can first appear as another operation's
                // prerequisite. Its report contract follows its identity.
                let contract = operation_contracts.get(step).copied();
                let native = contract.filter(|_| task.provider == target.manager);
                let mut bindings = super::reporting::bind(&workspace.root, &id, task, contract)?;
                let mut env = intent.env.clone();
                env.extend(task.env.clone());
                if let Some(owner) = tasks::hook_owner(task).and_then(|id| workspace.tasks.get(&id))
                {
                    let reports = super::reporting::bind(
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
                let post = tasks::post_hook(task);
                let boundary = sequence[position + 1..]
                    .iter()
                    .find(|id| **id == post)
                    .unwrap_or(step);
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
        if let Some(p) = previous.get(&target) {
            prerequisites.insert(p.clone());
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
        previous.insert(
            target,
            a["id"].as_str().context("missing action id")?.into(),
        );
    }
    validate_required_checks(workspace, &planned)?;
    let managed = workspace
        .configuration
        .values()
        .find_map(|config| config.management.as_ref());
    let required: BTreeSet<_> = workspace
        .configuration
        .values()
        .filter_map(|config| config.get("checks.required").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let digests: BTreeMap<_, _> = workspace
        .configuration
        .iter()
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
        .chain(workspace.configuration.values())
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
    for (id, target) in &workspace.targets {
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

fn validate_required_checks(workspace: &Workspace, actions: &[Value]) -> Result<()> {
    for (id, config) in &workspace.configuration {
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
        let metadata = json!({"schemaVersion":"v1alpha1","frontend":"dockerfile.v0","stages":[{"name":"","base":"scratch"}],"requirements":[],"context":{"files":["Dockerfile","oyzu.toml"]}});
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
