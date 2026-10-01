//! Execute a resolved plan and collect outcome records without ecosystem dispatch.
use super::bundle::{capture_output, safe_file, safe_report_parent};
use crate::{builders, dependencies, executor, reports, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path, time::Duration};

#[derive(Default)]
pub(super) struct ExecutionRecords {
    pub actions: Vec<Value>,
    pub reports: Vec<Value>,
    pub artifacts: Vec<Value>,
    pub diagnostics: Vec<Value>,
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
    let mut outcomes = vec![];
    let mut collected = vec![];
    let mut artifacts = vec![];
    let mut diagnostics = vec![];
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
        let mut outcome = json!({"id":id,"target":target,"required":true,"status":"blocked","producerEvidence":[],"enforced":[]});
        if failed {
            outcome["reason"] = json!("a prerequisite failed");
            outcomes.push(outcome);
            continue;
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
                workspace: work,
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
                diagnostics.push(json!({"code":"executor-failed","phase":"execute","severity":"error","message":error.to_string(),"action":id,"target":target}));
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
        for intent in a["reports"].as_array().context("missing report intents")? {
            let report_id = intent["id"].as_str().context("missing report id")?;
            let path = a["extensions"]["oyzu.dev/report-paths"][report_id]
                .as_str()
                .context("missing report path")?;
            let format = intent["format"].as_str().context("missing report format")?;
            let mut report = json!({"id":report_id,"action":id,"target":target,"kind":intent["kind"],"format":format,"status":"missing","summary":{}});
            let summary = (|| -> Result<Value> {
                let source: reports::ReportSource = a["extensions"]["oyzu.dev/report-sources"]
                    .get(report_id)
                    .map(|v| serde_json::from_value(v.clone()))
                    .transpose()?
                    .unwrap_or_default();
                if source != reports::ReportSource::File {
                    safe_report_parent(out, path)?;
                    reports::materialize(source, &stdout, &out.join(path))?;
                }
                let file = safe_file(out, path)?;
                if format == "junit" {
                    reports::junit_summary(&file)
                } else {
                    reports::coverage_summary(&file, format)
                }
            })();
            match summary {
                Ok(summary) => {
                    if summary["failed"].as_u64().is_some_and(|n| n > 0) {
                        code = 1;
                    }
                    report["status"] = json!("collected");
                    report["summary"] = summary;
                    report["path"] = json!(path);
                    let captured = capture_output(out, bundle, path)?;
                    report["digest"] = json!(snapshot::file_digest(&captured)?);
                    report["subjectDigest"] = plan["source"]["treeDigest"].clone();
                }
                Err(error) => {
                    code = 1;
                    report["status"] = json!("invalid");
                    diagnostics.push(json!({"code":"report-invalid","phase":"collect","severity":"error","message":error.to_string(),"action":id,"target":target}));
                }
            }
            collected.push(report);
        }
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
                    Ok(artifact) => artifacts.push(artifact),
                    Err(error) => {
                        code = 1;
                        diagnostics.push(json!({"code":"artifact-invalid","phase":"collect","severity":"error","message":error.to_string(),"action":id,"target":target}));
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
        outcomes.push(outcome);
    }
    Ok(ExecutionRecords {
        actions: outcomes,
        reports: collected,
        artifacts,
        diagnostics,
    })
}
