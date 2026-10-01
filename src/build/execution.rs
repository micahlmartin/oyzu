//! Execute a resolved plan and collect outcome records without ecosystem dispatch.
use super::{bundle::capture_output, collection, materialization};
use crate::{builders, dependencies, executor, records, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
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
        let path = if targets.len() == 1 {
            work.to_path_buf()
        } else {
            let path = contexts.path().join(id);
            snapshot::capture(work, &path)?;
            path
        };
        workspaces.insert(id.to_string(), path);
    }
    let mut collectors: BTreeMap<_, _> = workspaces
        .iter()
        .map(|(id, path)| {
            (
                id.clone(),
                collection::Collector::new(
                    collection::Locations {
                        workspace: path,
                        output: out,
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
    let mut failed = false;
    for target in plan["targets"].as_array().context("missing targets")? {
        let id = target["id"].as_str().context("missing target id")?;
        fs::create_dir_all(out.join(id).join("reports"))?;
        fs::create_dir_all(out.join(id).join("artifacts"))?;
    }
    fs::create_dir(bundle.join("logs"))?;
    for (index, a) in plan["actions"]
        .as_array()
        .context("missing actions")?
        .iter()
        .enumerate()
    {
        let id = a["id"].as_str().context("missing action id")?;
        let target = a["target"].as_str().context("missing action target")?;
        let collector = collectors
            .get_mut(target)
            .context("missing target report collector")?;
        let mut outcome = json!({"id":id,"target":target,"required":true,"status":"blocked","producerEvidence":[],"enforced":[]});
        if failed {
            outcome["reason"] = json!("a prerequisite failed");
            records.actions.push(outcome);
            // A failed main skips post, but its evidence must still be retained.
            failed |= records.collect_due(collector, id)?;
            continue;
        }
        if initialized.insert(target.to_string()) {
            let binding = (|| -> Result<()> {
                let receipts = materialization::apply(
                    &workspaces[target],
                    bundle,
                    a["inputs"].as_array().context("missing action inputs")?,
                    &records.artifacts,
                    &records.actions,
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
                records.actions.push(outcome);
                failed = true;
                continue;
            }
        }
        if let Some(evidence) = materialization_evidence.get(target) {
            outcome["producerEvidence"] = json!([evidence]);
        }
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
        let result = executor::execute_with_mounts(
            executor::Request {
                image: &images[target],
                workspace: &workspaces[target],
                output: out,
                cwd: &cwd,
                argv: &argv,
                env: &env,
                stdout: &stdout,
                stderr: &stderr,
                timeout: Duration::from_secs(600),
                name: &format!("oyzu-{run_id}-{index}"),
            },
            &mounts,
        );
        let mut code = match result {
            Ok(result) => {
                outcome["durationMs"] = json!(result.duration_ms);
                result.code
            }
            Err(error) => {
                records.diagnostics.push(json!({"code":"executor-failed","phase":"execute","severity":"error","message":error.to_string(),"action":id,"target":target}));
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
        collector.defer(a, stdout, records.actions.len())?;
        if code == 0 {
            for intent in plan["artifacts"]
                .as_array()
                .context("missing artifacts")?
                .iter()
                .filter(|v| v["producer"] == id)
            {
                let capture = (|| -> Result<Value> {
                    let path = intent["path"].as_str().context("missing artifact path")?;
                    let file = capture_output(out, bundle, path)?;
                    let mut artifact = intent.clone();
                    artifact["digest"] = json!(snapshot::file_digest(&file)?);
                    artifact["size"] = json!(fs::metadata(file)?.len());
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
            outcome["enforced"] = json!([
                "docker-network-none",
                "docker-read-only-root",
                "docker-cap-drop-all"
            ]);
        }
        failed = code != 0;
        records.actions.push(outcome);
        failed |= records.collect_due(collector, id)?;
    }
    for collector in collectors.values() {
        collector.ensure_finished()?;
    }
    Ok(records)
}
