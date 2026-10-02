//! Host test-only evidence. Native adapters own instrumentation; the shared
//! report collector and bundle store own validation and finalization. These
//! records explicitly disclose ambient host execution and observed source.
use super::{configuration, execute_with_unsets, Outcome};
use crate::{builders, bundle_store, model::Workspace, records, reports, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn relative(root: &Path, path: &Path) -> Result<String> {
    let path = path
        .strip_prefix(root)?
        .to_str()
        .context("non-UTF8 task path")?
        .replace('\\', "/");
    Ok(if path.is_empty() { ".".into() } else { path })
}

fn host_path(value: &str, root: &Path, output: &Path, runtime: &Path) -> String {
    // Only engine-owned mount prefixes are translated. Native shell bodies and
    // other arguments are never parsed or rewritten by this coordinator.
    for (prefix, base) in [
        ("/out/", output),
        ("/workspace/", root),
        ("/oyzu/", runtime),
    ] {
        if let Some(path) = value.strip_prefix(prefix) {
            return base.join(path).to_string_lossy().into_owned();
        }
        if let Some((flag, path)) = value.split_once('=') {
            if let Some(path) = path.strip_prefix(prefix) {
                return format!("{flag}={}", base.join(path).display());
            }
        }
    }
    value.into()
}

pub(super) fn run(
    workspace: &Workspace,
    primary: &str,
    sequence: &[String],
    args: &[String],
) -> Result<Option<Vec<Outcome>>> {
    if workspace.tasks[primary].name != "test" {
        return Ok(None);
    }
    let mut contracts = BTreeMap::new();
    for id in sequence {
        if let (Some(target), _) = configuration(workspace, id) {
            if let Some(contract) =
                builders::get(&target.builder)?.development_test(target, &workspace.tasks[id])?
            {
                contracts.insert(id.clone(), contract);
            }
        }
    }
    // Profiles not yet integrated retain the existing development behavior;
    // no empty evidence bundle is advertised as complete test integration.
    if !contracts.contains_key(primary) {
        return Ok(None);
    }
    for id in sequence {
        if workspace.tasks[id].name == "test" && !contracts.contains_key(id) {
            bail!("{id}: prerequisite test profile has no direct-run report integration yet");
        }
    }
    let mut transaction = bundle_store::Transaction::begin_host(&workspace.root)?;
    let bundle_path = transaction.path().to_owned();
    let bundle = bundle_path.as_path();
    let temporary = tempfile::tempdir()?;
    let output = temporary.path().join("reports");
    fs::create_dir(&output)?;
    fs::create_dir(bundle.join("logs"))?;
    let source = snapshot::capture(&workspace.root, &temporary.path().join("source"))?;
    let source_record = json!({"treeDigest":source.digest,"commit":null,"dirty":true});
    let source_digest = json!(source.digest);
    let primary_target = configuration(workspace, primary)
        .0
        .context("missing test target")?;
    let invocation = json!({"kind":"test","execution":"host","isolation":"none",
        "source":"observed-before-run","toolIdentity":"unverified","requested":primary});
    let platform = json!({"os":if cfg!(target_os="macos") {"darwin"} else {std::env::consts::OS},"arch":std::env::consts::ARCH});
    let builder_digest = snapshot::file_digest(&std::env::current_exe()?)?;
    let mut tasks = Vec::new();
    let mut actions = Vec::new();
    let mut targets = BTreeMap::new();
    let mut runtimes = std::collections::BTreeSet::new();
    for (index, id) in sequence.iter().enumerate() {
        let mut task = workspace.tasks[id].clone();
        let (target, config) = configuration(workspace, id);
        let owner = target.unwrap_or(primary_target);
        let contract = contracts.get(id);
        let runtime = temporary.path().join("runtime").join(&owner.name);
        if contract.is_some() && runtimes.insert(owner.name.clone()) {
            fs::create_dir_all(&runtime)?;
            for file in builders::get(&owner.builder)?.runtime_files() {
                if !snapshot::portable(file.name) {
                    bail!("invalid builder runtime path");
                }
                let destination = runtime.join(file.name);
                fs::create_dir_all(destination.parent().context("missing runtime parent")?)?;
                fs::write(destination, file.contents)?;
            }
        }
        let bindings = reports::bindings::bind(&workspace.root, &owner.name, &task, contract)?;
        for input in bindings.inputs.values() {
            if matches!(input.root, reports::Root::Workspace) {
                reports::ensure_fresh(&workspace.root, &input.path)
                    .with_context(|| format!("{id}: report freshness"))?;
                if !input.path.contains(['*', '?']) {
                    let destination = workspace.root.join(&input.path);
                    fs::create_dir_all(destination.parent().context("missing report parent")?)?;
                }
            }
        }
        fs::create_dir_all(output.join(&owner.name).join("reports"))?;
        if let Some(contract) = contract.filter(|contract| contract.argv != task.argv) {
            task.argv = contract.argv.clone();
            // A declaration redirects the same native obligation to a concrete
            // workspace path. Unknown custom command bodies remain unchanged.
            for arg in &mut task.argv {
                for report in &contract.reports {
                    let kind = report.format.kind().to_ascii_uppercase();
                    let default = format!("/out/{}/reports/{}", owner.name, report.filename);
                    if let Some(destination) = bindings.env.get(&format!("OYZU_{kind}_REPORT")) {
                        if arg == &default {
                            *arg = destination.clone();
                        } else if arg.ends_with(&format!("={default}")) {
                            *arg = format!("{}={destination}", arg.split_once('=').unwrap().0);
                        }
                    }
                }
                *arg = host_path(arg, &workspace.root, &output, &runtime);
            }
        }
        task.env.extend(bindings.env.iter().map(|(key, value)| {
            (
                key.clone(),
                host_path(value, &workspace.root, &output, &runtime),
            )
        }));
        if let Some(parent) = super::hook_owner(&task).and_then(|id| workspace.tasks.get(&id)) {
            let parent_bindings = reports::bindings::bind(
                &workspace.root,
                &owner.name,
                parent,
                contracts.get(&parent.id()),
            )?;
            task.env
                .extend(parent_bindings.env.iter().map(|(key, value)| {
                    (
                        key.clone(),
                        host_path(value, &workspace.root, &output, &runtime),
                    )
                }));
        }
        if let Some(config) = config {
            config.validate_environment(&task.env)?;
            if contract.is_some()
                && config
                    .get("checks.coverageMinimum")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
                    > 0
                && !bindings.intents.iter().any(|r| r.kind() == "coverage")
            {
                bail!("CONFIG_OVERRIDE_DENIED: {id} cannot satisfy required coverage minimum");
            }
        }
        let post = super::post_hook(&task);
        let boundary = if !super::is_hook(&task) && sequence.contains(&post) {
            post
        } else {
            id.clone()
        };
        let environment: BTreeMap<_, _> = task.env.keys().map(|key| (key, "[REDACTED]")).collect();
        actions.push(json!({"id":id,"target":owner.name,"operation":task.name,
            "dependsOn":if index==0 {vec![]} else {vec![sequence[index-1].clone()]},
            "argv":task.argv,"cwd":relative(&workspace.root,&task.cwd)?,"env":environment,
            "tools":[],"executionPlatform":platform,"targetPlatform":platform,"inputs":[],"outputs":[],
            "reports":bindings.intents,"required":true,"cacheable":false,"network":"host",
            "limits":{"timeoutSeconds":0,"cpu":1,"memoryBytes":0,"outputBytes":0},
            "extensions":{"oyzu.dev/invocation":invocation,"oyzu.dev/limits-enforced":false,
                "oyzu.dev/task-arguments":if id==primary {args} else {&[]},
                "oyzu.dev/report-paths":bindings.paths,"oyzu.dev/report-inputs":bindings.inputs,
                "oyzu.dev/report-sources":bindings.sources,"oyzu.dev/collect-after":boundary,
                "oyzu.dev/configuration-digest":config.map(|c| &c.digest),
                "oyzu.dev/coverage-minimum":config.and_then(|c|c.get("checks.coverageMinimum")).cloned().unwrap_or(json!(0))}}));
        let target_path = relative(&workspace.root, &owner.path)?;
        let target_record = targets.entry(owner.name.clone()).or_insert_with(|| {
            json!({"id":owner.name,"builder":owner.builder,"builderDigest":builder_digest,
            "path":target_path,"variant":{},"platform":platform,"extensions":{"oyzu.dev/discovery":owner.discovery}})
        });
        if let Some(coverage) = contract.and_then(|c| c.coverage.as_ref()) {
            target_record["extensions"]["oyzu.dev/coverage-applicability"] = json!(coverage);
        }
        tasks.push(task);
    }
    let targets: Vec<_> = targets.into_values().collect();
    let configurations: BTreeMap<_, _> = sequence
        .iter()
        .filter_map(|id| {
            configuration(workspace, id)
                .1
                .map(|config| (id, &config.digest))
        })
        .collect();
    let managed = sequence.iter().find_map(|id| {
        configuration(workspace, id)
            .1
            .and_then(|c| c.management.as_ref())
    });
    let mut required_checks = vec!["tests"];
    if actions.iter().any(|a| {
        a["reports"]
            .as_array()
            .is_some_and(|reports| reports.iter().any(|r| r["kind"] == "coverage"))
    }) {
        required_checks.push("coverage");
    }
    let mut policy = json!({"mode":if managed.is_some(){"managed"}else{"standalone"},
        "enforcementDigest":records::digest("oyzu.policy.v1alpha1",&json!({"invocation":invocation,"configuration":configurations,"productionEligible":false}))?,
        "requiredChecks":required_checks});
    if let Some(management) = managed {
        policy["extensions"]["oyzu.dev/configuration-policy"] = management.clone();
    }
    let plan = json!({"schemaVersion":"v1alpha1","kind":"build-plan","source":source_record,
        "policy":policy,
        "tools":[],"targets":targets,"actions":actions,"artifacts":[],"extensions":{"oyzu.dev/invocation":invocation}});
    let digest = records::digest("oyzu.plan.v1alpha1", &plan)?;
    records::write(&bundle.join("plan.json"), &plan)?;
    let now = SystemTime::now();
    let run_id = format!(
        "{}-{}",
        std::process::id(),
        now.duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let envelope = json!({"schemaVersion":"v1alpha1","kind":"execution-envelope","runId":run_id,"planDigest":digest,
        "startedAt":chrono::DateTime::<chrono::Utc>::from(now).to_rfc3339(),"host":platform,
        "context":if crate::config::sources::detected_ci(){"ci-unverified"}else{"local"},
        "identity":{"subject":"local-process","verification":"local"},"policy":{"decisionIds":[]},"facts":[],
        "extensions":{"oyzu.dev/invocation":invocation}});
    records::write(&bundle.join("envelope.json"), &envelope)?;
    let mut manifest = json!({"schemaVersion":"v1alpha1","kind":"build-manifest","runId":run_id,"planDigest":digest,"planPath":"plan.json",
        "source":source_record,"status":"failed","targets":targets,"actions":[],"artifacts":[],"reports":[],"evidence":[],"diagnostics":[],
        "envelopePath":"envelope.json","envelopeDigest":snapshot::file_digest(&bundle.join("envelope.json"))?,"extensions":{"oyzu.dev/invocation":invocation}});
    let mut collector = reports::collection::Collector::new(
        reports::collection::Locations {
            workspace: &workspace.root,
            output: &output,
            bundle,
        },
        &source_digest,
    );
    let mut outcomes = Vec::new();
    let mut pending_boundaries = Vec::new();
    for (index, task) in tasks.iter().enumerate() {
        let (_, config) = configuration(workspace, &sequence[index]);
        let removed = config.map(|c| c.removed.clone()).unwrap_or_default();
        let outcome = execute_with_unsets(
            task,
            if task.id() == primary { args } else { &[] },
            &removed,
            config,
        )
        .unwrap_or_else(|error| Outcome {
            task: task.id(),
            status: "failed".into(),
            exit_code: 1,
            stdout: String::new(),
            stderr: format!("{error:#}"),
        });
        let stdout = bundle.join(format!("logs/{index:04}.stdout"));
        fs::write(&stdout, &outcome.stdout)?;
        fs::write(
            bundle.join(format!("logs/{index:04}.stderr")),
            &outcome.stderr,
        )?;
        let failed = outcome.exit_code != 0;
        outcomes.push(outcome);
        collector.defer(&actions[index], stdout, index)?;
        pending_boundaries.push(
            actions[index]["extensions"]["oyzu.dev/collect-after"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
        let boundaries = if failed {
            pending_boundaries.clone()
        } else {
            vec![task.id()]
        };
        for boundary in boundaries {
            for (producer, reports) in collector.finish(&boundary)? {
                for report in reports {
                    if report.failed() {
                        for target in [producer, index] {
                            if outcomes[target].exit_code == 0 {
                                outcomes[target].exit_code = 1;
                                outcomes[target].status = "failed".into();
                            }
                        }
                    }
                    manifest["reports"]
                        .as_array_mut()
                        .unwrap()
                        .push(report.record);
                    if let Some(diagnostic) = report.diagnostic {
                        manifest["diagnostics"]
                            .as_array_mut()
                            .unwrap()
                            .push(diagnostic);
                    }
                }
            }
        }
        if outcomes.last().unwrap().exit_code != 0 {
            break;
        }
    }
    collector.ensure_finished()?;
    manifest["actions"]=json!(actions.iter().enumerate().map(|(i,action)| {
        let mut record=json!({"id":action["id"],"target":action["target"],"required":true,"status":"blocked","enforced":[],"producerEvidence":[]});
        if let Some(outcome)=outcomes.get(i){record["status"]=json!(outcome.status);record["exitCode"]=json!(outcome.exit_code);}
        record
    }).collect::<Vec<_>>());
    if outcomes.len() == actions.len() && outcomes.iter().all(|o| o.exit_code == 0) {
        manifest["status"] = json!("succeeded");
    }
    manifest["extensions"]["oyzu.dev/host-outputs"] = json!(transaction.preserve_host_outputs()?);
    records::write(&bundle.join("manifest.json"), &manifest)?;
    transaction.publish(&run_id)?;
    Ok(Some(outcomes))
}
