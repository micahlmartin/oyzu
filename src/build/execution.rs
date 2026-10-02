//! Execute a resolved plan and collect outcome records without ecosystem dispatch.
use super::{bundle::capture_output, directory, materialization};
use crate::{builders, dependencies, executor, records, reports::collection, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Default)]
pub(super) struct ExecutionRecords {
    pub actions: Vec<Value>,
    pub reports: Vec<Value>,
    pub artifacts: Vec<Value>,
    pub diagnostics: Vec<Value>,
    pub evidence: Vec<Value>,
}

impl ExecutionRecords {
    fn collect_due(
        &mut self,
        collector: &mut collection::Collector<'_>,
        boundary: &str,
    ) -> Result<bool> {
        let mut failed = false;
        for (index, reports) in collector.finish(boundary)? {
            for report in reports {
                if report.failed() {
                    failed = true;
                    self.actions[index]["status"] = json!("failed");
                    if self.actions[index]["exitCode"] == 0 {
                        self.actions[index]["exitCode"] = json!(1);
                    }
                }
                if let Some(diagnostic) = report.diagnostic {
                    self.diagnostics.push(diagnostic);
                }
                self.reports.push(report.record);
            }
        }
        if failed {
            // Deferred reports gate the boundary action as well as their producer.
            // Otherwise a successful post-hook could release packaging after a
            // report discovered that the main test action actually failed.
            if let Some(action) = self
                .actions
                .iter_mut()
                .find(|a| a["id"] == boundary && a["status"] == "succeeded")
            {
                action["status"] = json!("failed");
                action["exitCode"] = json!(1);
            }
        }
        Ok(failed)
    }
}

pub(super) fn execute_plan(
    plan: &Value,
    work: &Path,
    out: &Path,
    bundle: &Path,
    images: &BTreeMap<String, executor::Image>,
    dependencies: &BTreeMap<String, dependencies::Prepared>,
    run_id: &str,
) -> Result<ExecutionRecords> {
    let runtime = tempfile::tempdir()?;
    let mut runtime_paths = BTreeMap::new();
    for target in plan["targets"].as_array().context("missing targets")? {
        let id = target["id"].as_str().context("missing target id")?;
        let builder = builders::get(target["builder"].as_str().context("missing builder id")?)?;
        if !builder.runtime_files().is_empty() {
            let directory = runtime.path().join(id);
            fs::create_dir(&directory)?;
            for file in builder.runtime_files() {
                if file.name.contains(['/', '\\', ':']) || file.name.starts_with('.') {
                    bail!("invalid built-in runtime filename");
                }
                fs::write(directory.join(file.name), file.contents)?;
            }
            runtime_paths.insert(id.to_string(), directory);
        }
    }
    let contexts = tempfile::tempdir()?;
    let mut workspaces = BTreeMap::new();
    let targets = plan["targets"].as_array().context("missing targets")?;
    for target in targets {
        let id = target["id"].as_str().context("missing target id")?;
        let selection = &target["extensions"]["oyzu.dev/source-projection"];
        let path = if !selection.is_null() {
            let projection: snapshot::Projection = serde_json::from_value(selection.clone())?;
            let path = contexts.path().join(id);
            snapshot::capture_projected(work, &path, &projection)?;
            path
        } else if targets.len() == 1 {
            work.to_path_buf()
        } else {
            let path = contexts.path().join(id);
            snapshot::capture(work, &path)?;
            path
        };
        workspaces.insert(id.to_string(), path);
    }
    // Each target receives a private /out root as well as a private workspace.
    // Concurrent project code cannot overwrite another target's pending outputs.
    let mut outputs = BTreeMap::new();
    for target in targets {
        let id = target["id"].as_str().context("missing target id")?;
        let root = out.join(id);
        fs::create_dir_all(root.join(id).join("reports"))?;
        fs::create_dir_all(root.join(id).join("artifacts"))?;
        outputs.insert(id.to_string(), root);
    }
    let mut collectors: BTreeMap<_, _> = workspaces
        .iter()
        .map(|(id, path)| {
            (
                id.clone(),
                collection::Collector::new(
                    collection::Locations {
                        workspace: path,
                        output: &outputs[id],
                        bundle,
                    },
                    &plan["source"]["treeDigest"],
                ),
            )
        })
        .collect();
    let mut initialized = BTreeSet::new();
    let mut materialization_evidence = BTreeMap::new();
    let mut records = ExecutionRecords::default();
    fs::create_dir(bundle.join("logs"))?;
    let actions = plan["actions"].as_array().context("missing actions")?;
    let jobs = plan["extensions"]["oyzu.dev/execution"]["jobs"]
        .as_u64()
        .unwrap_or(1);
    let schedule = super::scheduling::Schedule::new(actions, jobs)?;
    records.actions = actions.iter().map(|a| json!({"id":a["id"],"target":a["target"],"required":true,"status":"pending","producerEvidence":[],"enforced":[]})).collect();
    let context = LaunchContext {
        images,
        workspaces: &workspaces,
        dependencies,
        runtime_paths: &runtime_paths,
        outputs: &outputs,
        bundle,
        run_id,
    };
    while records.actions.iter().any(|a| a["status"] == "pending") {
        let ready = schedule.ready(&records.actions)?;
        let mut runnable = Vec::new();
        for index in ready {
            let a = &actions[index];
            let id = a["id"].as_str().context("missing action id")?;
            let target = a["target"].as_str().context("missing action target")?;
            let mut outcome = records.actions[index].clone();
            if !schedule.permitted(index, &records.actions) {
                outcome["status"] = json!("blocked");
                outcome["reason"] = json!("a prerequisite failed");
                records.actions[index] = outcome;
                records.collect_due(
                    collectors
                        .get_mut(target)
                        .context("missing report collector")?,
                    id,
                )?;
                continue;
            }
            if initialized.insert(target.to_string()) {
                let binding = (|| -> Result<()> {
                    let mut receipts = materialization::apply(
                        &workspaces[target],
                        bundle,
                        a["inputs"].as_array().context("missing action inputs")?,
                        &records.artifacts,
                        &records.actions,
                    )?;
                    materialization::reference_reports(
                        &mut receipts,
                        bundle,
                        &records.artifacts,
                        &records.actions,
                        &records.reports,
                    )?;
                    if !receipts.is_empty() {
                        fs::create_dir_all(bundle.join("inputs"))?;
                        let path = format!("inputs/{target}.json");
                        let record = json!({"schemaVersion":"v1alpha1","kind":"artifact-materialization","target":target,"inputs":receipts});
                        records::write(&bundle.join(&path), &record)?;
                        let evidence_id = format!("{target}/materialization");
                        records.evidence.push(json!({"id":evidence_id,"kind":"artifact-materialization","subjectDigest":records::digest("oyzu.materialization.v1alpha1", &record)?,"producer":"oyzu-executor","verification":"local","path":path,"digest":snapshot::file_digest(&bundle.join(path))?}));
                        materialization_evidence.insert(target.to_string(), evidence_id);
                    }
                    Ok(())
                })();
                if let Err(error) = binding {
                    outcome["status"] = json!("failed");
                    outcome["exitCode"] = json!(1);
                    records.diagnostics.push(json!({"code":"materialization-failed","phase":"execute","severity":"error","message":error.to_string(),"action":id,"target":target}));
                    records.actions[index] = outcome;
                    continue;
                }
            }
            runnable.push(index);
        }
        let completed = super::scheduling::parallel(&runnable, |index| {
            launch(&actions[index], index, &context)
        })?;
        for (index, executed) in completed {
            let executed = executed?;
            let a = &actions[index];
            let id = a["id"].as_str().context("missing action id")?;
            let target = a["target"].as_str().context("missing action target")?;
            let collector = collectors
                .get_mut(target)
                .context("missing report collector")?;
            let mut outcome = records.actions[index].clone();
            if let Some(evidence) = materialization_evidence.get(target) {
                outcome["producerEvidence"] = json!([evidence]);
            }
            if let Some(duration) = executed.duration {
                outcome["durationMs"] = json!(duration);
            }
            if let Some(error) = executed.failure {
                records.diagnostics.push(json!({"code":"executor-failed","phase":"execute","severity":"error","message":error,"action":id,"target":target}));
            }
            let mut code = executed.code;
            collector.defer(a, executed.stdout, index)?;
            if code == 0 {
                for intent in plan["artifacts"]
                    .as_array()
                    .context("missing artifacts")?
                    .iter()
                    .filter(|v| v["producer"] == id)
                {
                    let capture = (|| -> Result<Value> {
                        if intent["kind"] == "directory" {
                            return directory::capture(&outputs[target], bundle, intent);
                        }
                        let path = intent["path"].as_str().context("missing artifact path")?;
                        let file = capture_output(&outputs[target], bundle, path)?;
                        let mut artifact = intent.clone();
                        artifact["digest"] = json!(snapshot::file_digest(&file)?);
                        artifact["size"] = json!(fs::metadata(&file)?.len());
                        if matches!(intent["kind"].as_str(), Some("oci-image" | "oci-index")) {
                            let verified = crate::oci::verify(&file)?;
                            if intent["kind"] != verified.kind {
                                bail!("OCI output does not match planned image/index kind");
                            }
                            if verified.kind == "oci-image" {
                                verified.require_target(&serde_json::from_value(
                                    a["targetPlatform"].clone(),
                                )?)?;
                            }
                            artifact["ociDigest"] = json!(verified.digest);
                        }
                        Ok(artifact)
                    })();
                    match capture {
                        Ok(artifact) => records.artifacts.push(artifact),
                        Err(error) => {
                            code = 1;
                            records.diagnostics.push(json!({"code":"artifact-invalid","phase":"collect","severity":"error","message":error.to_string(),"action":id,"target":target}));
                        }
                    }
                }
            }
            outcome["status"] = json!(if code == 0 { "succeeded" } else { "failed" });
            outcome["exitCode"] = json!(code);
            // A Docker startup failure is not evidence that a sandbox ran.
            if code == 0 {
                outcome["enforced"] = json!(executed.mode.enforced());
            }
            records.actions[index] = outcome;
            records.collect_due(collector, id)?;
        }
    }
    for collector in collectors.values() {
        collector.ensure_finished()?;
    }
    Ok(records)
}

struct LaunchContext<'a> {
    images: &'a BTreeMap<String, executor::Image>,
    workspaces: &'a BTreeMap<String, PathBuf>,
    dependencies: &'a BTreeMap<String, dependencies::Prepared>,
    runtime_paths: &'a BTreeMap<String, PathBuf>,
    outputs: &'a BTreeMap<String, PathBuf>,
    bundle: &'a Path,
    run_id: &'a str,
}
struct Executed {
    code: i32,
    duration: Option<u64>,
    failure: Option<String>,
    stdout: PathBuf,
    mode: executor::Mode,
}
fn launch(a: &Value, index: usize, context: &LaunchContext<'_>) -> Result<Executed> {
    let LaunchContext {
        images,
        workspaces,
        dependencies,
        runtime_paths,
        outputs,
        bundle,
        run_id,
    } = context;
    let id = a["id"].as_str().context("missing action id")?;
    let target = a["target"].as_str().context("missing action target")?;
    let stdout = bundle.join(format!("logs/{index:04}.stdout"));
    let stderr = bundle.join(format!("logs/{index:04}.stderr"));
    let argv: Vec<String> = serde_json::from_value(a["argv"].clone())?;
    let env: BTreeMap<String, String> = serde_json::from_value(a["env"].clone())?;
    let cwd = format!("/workspace/{}", a["cwd"].as_str().context("missing cwd")?);
    eprintln!("{id}");
    let mut mounts = Vec::new();
    if let Some(prepared) = dependencies.get(target) {
        mounts.push(executor::Mount {
            source: &prepared.root,
            destination: "/dependencies",
            readonly: true,
        });
    }
    if let Some(runtime) = runtime_paths.get(target) {
        mounts.push(executor::Mount {
            source: runtime,
            destination: "/oyzu",
            readonly: true,
        });
    }
    let mode: executor::Mode =
        serde_json::from_value(a["extensions"]["oyzu.dev/executor"].clone())?;
    let materialized: Vec<String> = a["inputs"]
        .as_array()
        .context("missing inputs")?
        .iter()
        .filter(|input| input["kind"] == "artifact")
        .filter_map(|input| input["mount"].as_str().map(str::to_owned))
        .collect();
    let result = executor::execute_mode(
        executor::Request {
            image: &images[target],
            workspace: &workspaces[target],
            output: &outputs[target],
            cwd: &cwd,
            argv: &argv,
            env: &env,
            stdout: &stdout,
            stderr: &stderr,
            timeout: Duration::from_secs(600),
            name: &format!("oyzu-{run_id}-{index}"),
        },
        &mounts,
        &mode,
        &materialized,
        &serde_json::from_value(a["targetPlatform"].clone())?,
    );
    let mut duration = None;
    let mut failure = None;
    let mut code = match result {
        Ok(result) => {
            duration = Some(result.duration_ms);
            result.code
        }
        Err(error) => {
            failure = Some(error.to_string());
            1
        }
    };
    if a["extensions"]["oyzu.dev/stdout-must-be-empty"] == true
        && !fs::read_to_string(&stdout)
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        code = 1;
    }
    Ok(Executed {
        code,
        duration,
        failure,
        stdout,
        mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deferred_failed_report_blocks_successors_of_successful_post_hook() {
        let root = tempfile::tempdir().unwrap();
        let source = json!("sha256:source");
        let action = json!({"id":"app:test","target":"app","reports":[{"id":"tests","kind":"test","format":"junit"}],"extensions":{
            "oyzu.dev/collect-after":"app:post_test",
            "oyzu.dev/report-paths":{"tests":"app/reports/custom"},
            "oyzu.dev/report-inputs":{"tests":{"root":"workspace","path":"missing.xml"}}
        }});
        let mut collector = collection::Collector::new(
            collection::Locations {
                workspace: root.path(),
                output: root.path(),
                bundle: root.path(),
            },
            &source,
        );
        collector
            .defer(&action, root.path().join("stdout"), 0)
            .unwrap();
        let mut records = ExecutionRecords {
            actions: vec![
                json!({"id":"app:test","status":"succeeded","exitCode":0}),
                json!({"id":"app:post_test","status":"succeeded","exitCode":0}),
            ],
            ..Default::default()
        };
        assert!(records
            .collect_due(&mut collector, "app:post_test")
            .unwrap());
        assert!(records
            .actions
            .iter()
            .all(|a| a["status"] == "failed" && a["exitCode"] == 1));
        collector.ensure_finished().unwrap();
    }
}
