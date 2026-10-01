//! Captured-source builds and local evidence bundles (OEP-0006/0007/0012).
use crate::{config, discovery, executor, model::Workspace, records, reports, snapshot, tasks};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const NODE_IMAGE: &str = "node:22-bookworm-slim";
pub const GO_IMAGE: &str = "golang:1.24-bookworm";

fn relative(root: &Path, path: &Path) -> Result<String> {
    let value = path
        .strip_prefix(root)
        .context("action path outside captured source")?
        .to_str()
        .context("non-UTF8 action path")?
        .replace('\\', "/");
    Ok(if value.is_empty() { ".".into() } else { value })
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}

fn platform(image: &executor::Image) -> Value {
    json!({"os":image.os,"arch":image.arch})
}

fn target_order(workspace: &Workspace) -> Result<Vec<String>> {
    let configs = config::targets(&workspace.root)?.unwrap_or_default();
    for (id, c) in &configs {
        if !c.materialize.is_empty()
            || !c.matrix.is_empty()
            || c.platform.is_some()
            || c.container.is_some()
            || c.bindings.is_some()
        {
            bail!("{id}: platform expansion, materialization and packaging options are not implemented yet");
        }
    }
    let mut pending: BTreeSet<_> = workspace.targets.keys().cloned().collect();
    let mut done = Vec::new();
    while !pending.is_empty() {
        let ready = pending
            .iter()
            .find(|id| {
                configs
                    .get(*id)
                    .is_none_or(|c| c.depends_on.iter().all(|d| done.contains(d)))
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
    execution: (&executor::Image, &str),
) -> Value {
    let (image, source) = execution;
    json!({"id":id,"target":target,"operation":operation,"dependsOn":[],"argv":argv,"cwd":cwd,"env":env,
        "tools":[target],"executionPlatform":platform(image),"targetPlatform":platform(image),
        "inputs":[{"kind":"tree","digest":source,"mount":"workspace"}],"outputs":[],"reports":[],
        "required":true,"cacheable":false,"network":"none",
        "limits":{"timeoutSeconds":600,"cpu":2,"memoryBytes":2147483648u64,"outputBytes":16777216}})
}

/// Pure planning after source capture and image resolution; never invokes project code.
pub fn plan(
    workspace: &Workspace,
    source: &snapshot::Snapshot,
    images: &BTreeMap<String, executor::Image>,
) -> Result<Value> {
    let mut planned = Vec::new();
    let mut target_records = Vec::new();
    let mut artifacts = Vec::new();
    let mut tools = Vec::new();
    let builder_digest = snapshot::file_digest(&std::env::current_exe()?)?;
    let mut emitted = BTreeSet::new();
    for id in target_order(workspace)? {
        let target = &workspace.targets[&id];
        if !matches!(target.manager.as_str(), "npm" | "go") {
            bail!(
                "{id}: {} build integration is not yet implemented",
                target.manager
            );
        }
        let image = images
            .get(&id)
            .with_context(|| format!("{id}: no resolved toolchain image"))?;
        let cwd = relative(&workspace.root, &target.path)?;
        let version = format!(
            "{}-dev.g{}",
            target.version.split(['-', '+']).next().unwrap_or("0.0.0"),
            &source.digest[7..19]
        );
        target_records.push(json!({"id":id,"builder":target.builder,"builderDigest":builder_digest,"path":cwd,"variant":{},"platform":platform(image)}));
        tools.push(json!({"id":id,"version":image.reference,"digest":image.digest,"platform":platform(image)}));
        let mut env = BTreeMap::from([
            ("HOME".into(), "/tmp/oyzu-home".into()),
            ("CI".into(), "true".into()),
            ("TZ".into(), "UTC".into()),
            ("OYZU_VERSION".into(), version.clone()),
        ]);
        let package = if target.manager == "npm" {
            Some(records::read(&target.path.join("package.json"))?)
        } else {
            None
        };
        if let Some(p) = &package {
            if p.get("workspaces").is_some() {
                bail!("{id}: npm workspace build integration is not implemented yet");
            }
            for field in ["dependencies", "devDependencies", "optionalDependencies"] {
                if p[field].as_object().is_some_and(|v| !v.is_empty()) {
                    bail!("{id}: dependency acquisition is not implemented; refusing an incomplete or online build");
                }
            }
            env.insert("npm_config_offline".into(), "true".into());
            env.insert("npm_config_audit".into(), "false".into());
            env.insert("npm_config_fund".into(), "false".into());
            env.insert("npm_config_cache".into(), "/tmp/npm-cache".into());
            let script = "const fs=require('node:fs');for(const f of ['package.json','package-lock.json']){if(!fs.existsSync(f))continue;const p=JSON.parse(fs.readFileSync(f,'utf8'));p.version=process.env.OYZU_VERSION;if(p.packages?.[''])p.packages[''].version=p.version;fs.writeFileSync(f,JSON.stringify(p,null,2)+'\\n');}";
            planned.push(action(
                &format!("{id}:version"),
                &id,
                "version",
                strings(&["node", "-e", script]),
                &cwd,
                &env,
                (image, &source.digest),
            ));
            let argv = if target.path.join("package-lock.json").exists() {
                strings(&["npm", "ci", "--offline", "--ignore-scripts"])
            } else {
                strings(&["npm", "install", "--offline", "--ignore-scripts"])
            };
            planned.push(action(
                &format!("{id}:prepare"),
                &id,
                "prepare",
                argv,
                &cwd,
                &env,
                (image, &source.digest),
            ));
        } else {
            env.extend(BTreeMap::from([
                ("GOTOOLCHAIN".into(), "local".into()),
                ("GOPROXY".into(), "off".into()),
                ("GOSUMDB".into(), "off".into()),
                ("CGO_ENABLED".into(), "0".into()),
                ("GOFLAGS".into(), "-p=2".into()),
                ("GOMAXPROCS".into(), "2".into()),
                ("GOCACHE".into(), "/workspace/.oyzu-build/go-cache".into()),
            ]));
            if target.path.join("go.work").exists() {
                bail!("{id}: Go workspace packaging is not implemented yet");
            }
            planned.push(action(
                &format!("{id}:prepare"),
                &id,
                "prepare",
                strings(&["mkdir", "-p", ".oyzu-build"]),
                &cwd,
                &env,
                (image, &source.digest),
            ));
        }
        for stage in ["build", "test", "lint", "format-check", "format:check"] {
            let task_id = format!("{id}:{stage}");
            let Some(task) = workspace.tasks.get(&task_id) else {
                continue;
            };
            if task.availability.is_some() || !task.build_stage {
                continue;
            }
            for step in tasks::sequence(workspace, &task_id)? {
                if !emitted.insert(step.clone()) {
                    continue;
                }
                let task = &workspace.tasks[&step];
                if task.mutates_source {
                    bail!("{step}: mutating formatter cannot run as a build check");
                }
                if !task.target.is_empty() && task.target != id {
                    bail!("{step}: cross-target task prerequisites require graph integration");
                }
                let mut argv = task.argv.clone();
                let mut task_env = env.clone();
                task_env.extend(task.env.clone());
                let mut intents = vec![];
                let mut paths = BTreeMap::<String, String>::new();
                if task.provider == "go" && stage == "build" && step == task_id {
                    argv = strings(&["go", "build", "-trimpath", "-o", ".oyzu-build/app", "."]);
                }
                if task.provider == "go" && stage == "test" && step == task_id {
                    argv = strings(&[
                        "go",
                        "test",
                        "-json",
                        &format!("-coverprofile=/out/{id}/reports/coverage.out"),
                        "./...",
                    ]);
                }
                let node_test = package
                    .as_ref()
                    .is_some_and(|p| p["scripts"]["test"].as_str() == Some("node --test"))
                    && task.provider == "npm";
                if stage == "test" && step == task_id && (task.provider == "go" || node_test) {
                    let coverage_format = if node_test { "lcov" } else { "go-cover" };
                    let coverage_file = if node_test {
                        "coverage.lcov"
                    } else {
                        "coverage.out"
                    };
                    for (kind, format, file) in [
                        ("test", "junit", "junit.xml"),
                        ("coverage", coverage_format, coverage_file),
                    ] {
                        let report_id = format!("{id}:{kind}");
                        intents.push(json!({"id":report_id,"kind":kind,"format":format,"required":true,"subject":id}));
                        paths.insert(report_id, format!("{id}/reports/{file}"));
                    }
                    if node_test {
                        argv.extend(strings(&[
                            "--",
                            "--experimental-test-coverage",
                            "--test-reporter=junit",
                            &format!("--test-reporter-destination=/out/{id}/reports/junit.xml"),
                            "--test-reporter=lcov",
                            &format!("--test-reporter-destination=/out/{id}/reports/coverage.lcov"),
                        ]));
                    }
                }
                let mut a = action(
                    &step,
                    &id,
                    &task.name,
                    argv,
                    &relative(&workspace.root, &task.cwd)?,
                    &task_env,
                    (image, &source.digest),
                );
                a["reports"] = json!(intents);
                a["extensions"] = json!({"oyzu.dev/report-paths":paths,"oyzu.dev/stdout-must-be-empty":task.stdout_must_be_empty,"oyzu.dev/go-test-events": task.provider == "go" && stage == "test" && step == task_id});
                planned.push(a);
            }
        }
        let (argv, filename, media_type) = if let Some(p) = &package {
            let name = p["name"]
                .as_str()
                .context("npm artifact requires package name")?;
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"@/._-".contains(&b))
            {
                bail!("invalid npm package name");
            }
            let filename = format!(
                "{}-{version}.tgz",
                name.trim_start_matches('@').replace('/', "-")
            );
            (
                strings(&[
                    "npm",
                    "pack",
                    "--ignore-scripts",
                    "--pack-destination",
                    &format!("/out/{id}/artifacts"),
                ]),
                filename,
                "application/gzip",
            )
        } else {
            if target.builder != "go/app" {
                bail!("{id}: Go library artifact packaging is not implemented yet");
            }
            let filename = format!("{id}-{version}");
            (
                strings(&[
                    "cp",
                    ".oyzu-build/app",
                    &format!("/out/{id}/artifacts/{filename}"),
                ]),
                filename,
                "application/octet-stream",
            )
        };
        let artifact_id = format!("{id}/primary");
        let producer = format!("{id}:package");
        let mut pack = action(
            &producer,
            &id,
            "package",
            argv,
            &cwd,
            &env,
            (image, &source.digest),
        );
        pack["outputs"] = json!([artifact_id]);
        planned.push(pack);
        artifacts.push(json!({"id":artifact_id,"target":id,"variant":{},"name":"primary","producer":producer,"kind":"file","version":version,"mediaType":media_type,"path":format!("{id}/artifacts/{filename}")}));
    }
    // Initial scheduler is deliberately serial; every actual ordering edge is explicit.
    let mut previous: Option<String> = None;
    for a in &mut planned {
        if let Some(p) = &previous {
            a["dependsOn"] = json!([p]);
        }
        previous = a["id"].as_str().map(str::to_string);
    }
    let policy = json!({"mode":"standalone","enforcementDigest":records::digest("oyzu.policy.v1alpha1",&json!({"offline":true,"productionEligible":false,"executor":"docker-v1"}))?,"requiredChecks":[]});
    Ok(
        json!({"schemaVersion":"v1alpha1","kind":"build-plan","source":{"treeDigest":source.digest,"commit":null,"dirty":true},"policy":policy,"tools":tools,"targets":target_records,"actions":planned,"artifacts":artifacts}),
    )
}

fn resolve_images(
    workspace: &Workspace,
    overrides: &[String],
) -> Result<BTreeMap<String, executor::Image>> {
    let mut refs = BTreeMap::from([
        ("npm".to_string(), NODE_IMAGE.to_string()),
        ("go".to_string(), GO_IMAGE.to_string()),
    ]);
    for value in overrides {
        let (manager, reference) = value
            .split_once('=')
            .context("--image requires manager=image")?;
        if !refs.contains_key(manager) || reference.is_empty() {
            bail!("invalid image override {value}");
        }
        refs.insert(manager.into(), reference.into());
    }
    let mut images = BTreeMap::new();
    for (id, target) in &workspace.targets {
        let reference = refs.get(&target.manager).with_context(|| {
            format!(
                "{id}: {} build integration is not implemented yet",
                target.manager
            )
        })?;
        images.insert(id.clone(), executor::resolve(reference)?);
    }
    Ok(images)
}

fn safe_file(root: &Path, relative: &str) -> Result<PathBuf> {
    let mut path = root.to_path_buf();
    if relative.is_empty() || relative.contains('\\') || relative.contains(':') {
        bail!("invalid bundle path");
    }
    for part in relative.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            bail!("invalid bundle path");
        }
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("bundle path is a symlink");
        }
    }
    if !path.is_file() {
        bail!("bundle output is not a regular file");
    }
    Ok(path)
}

fn safe_report_parent(root: &Path, relative: &str) -> Result<()> {
    let mut path = root.to_path_buf();
    let parts: Vec<_> = relative.split('/').collect();
    for part in &parts[..parts.len().saturating_sub(1)] {
        if part.is_empty() || *part == "." || *part == ".." || part.contains(['\\', ':']) {
            bail!("invalid report directory");
        }
        path.push(part);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("unsafe report directory");
        }
    }
    Ok(())
}

#[derive(Default)]
struct ExecutionRecords {
    actions: Vec<Value>,
    reports: Vec<Value>,
    artifacts: Vec<Value>,
    diagnostics: Vec<Value>,
}

fn execute_plan(
    plan: &Value,
    work: &Path,
    out: &Path,
    bundle: &Path,
    images: &BTreeMap<String, executor::Image>,
    run_id: &str,
) -> Result<ExecutionRecords> {
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
        let result = executor::execute(executor::Request {
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
        });
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
        let go_events = a["extensions"]["oyzu.dev/go-test-events"] == true;
        for intent in a["reports"].as_array().context("missing report intents")? {
            let report_id = intent["id"].as_str().context("missing report id")?;
            let path = a["extensions"]["oyzu.dev/report-paths"][report_id]
                .as_str()
                .context("missing report path")?;
            let format = intent["format"].as_str().context("missing report format")?;
            let mut report = json!({"id":report_id,"action":id,"target":target,"kind":intent["kind"],"format":format,"status":"missing","summary":{}});
            let summary = (|| -> Result<Value> {
                if go_events && format == "junit" {
                    safe_report_parent(out, path)?;
                    reports::go_to_junit(&stdout, &out.join(path))?;
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
                    report["digest"] = json!(snapshot::file_digest(&out.join(path))?);
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
                    let file = safe_file(out, path)?;
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

/// Build an immutable local bundle. Prior bundles are retained under .oyzu/history.
pub fn run(root: &Path, images: &[String], plan_only: bool) -> Result<Value> {
    let root = root.canonicalize()?;
    let state = root.join(".oyzu");
    if state.exists() && fs::symlink_metadata(&state)?.file_type().is_symlink() {
        bail!(".oyzu must not be a symlink");
    }
    fs::create_dir_all(&state)?;
    let lock_path = state.join("build.lock");
    if lock_path.exists() && fs::symlink_metadata(&lock_path)?.file_type().is_symlink() {
        bail!("build lock must not be a symlink");
    }
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.try_lock()
        .context("another build holds the workspace lock")?;
    let dist = root.join("dist");
    if dist.exists()
        && (fs::symlink_metadata(&dist)?.file_type().is_symlink()
            || records::read(&dist.join("manifest.json"))
                .ok()
                .is_none_or(|v| v["kind"] != "build-manifest"))
    {
        bail!("dist exists and is not an Oyzu bundle; preserve or move it before building");
    }
    let temp = tempfile::Builder::new().prefix("oyzu-build-").tempdir()?;
    let source_path = temp.path().join("source");
    let bundle_stage = tempfile::Builder::new()
        .prefix("bundle-")
        .tempdir_in(&state)?;
    let bundle = bundle_stage.path();
    let out = temp.path().join("outputs");
    fs::create_dir(&out)?;
    let run_id = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let mut envelope = json!({"schemaVersion":"v1alpha1","kind":"execution-envelope","runId":run_id,"planDigest":null,"startedAt":chrono::DateTime::<chrono::Utc>::from(SystemTime::now()).to_rfc3339(),"host":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH},"context":if std::env::var_os("CI").is_some() {"ci-unverified"} else {"local"},"identity":{"subject":"local-process","verification":"local"},"policy":{"decisionIds":[]},"facts":[]});
    if envelope["host"]["os"] == "macos" {
        envelope["host"]["os"] = json!("darwin");
    }
    let mut manifest = json!({"schemaVersion":"v1alpha1","kind":"build-manifest","runId":run_id,"planDigest":null,"planPath":null,"source":null,"status":"failed","targets":[],"actions":[],"artifacts":[],"reports":[],"evidence":[],"diagnostics":[],"envelopePath":"envelope.json","envelopeDigest":null});
    let result = (|| -> Result<Value> {
        let source = snapshot::capture(&root, &source_path)?;
        let workspace = discovery::discover_with_shell(&source_path, Some("sh"))?;
        let resolved = resolve_images(&workspace, images)?;
        let plan = plan(&workspace, &source, &resolved)?;
        if plan_only {
            return Ok(plan);
        }
        let digest = records::digest("oyzu.plan.v1alpha1", &plan)?;
        records::write(&bundle.join("plan.json"), &plan)?;
        envelope["planDigest"] = json!(digest);
        manifest["planDigest"] = json!(digest);
        manifest["planPath"] = json!("plan.json");
        manifest["source"] = plan["source"].clone();
        manifest["targets"] = plan["targets"].clone();
        let work = temp.path().join("work");
        snapshot::capture(&source_path, &work)?;
        let ExecutionRecords {
            actions,
            reports,
            artifacts,
            diagnostics,
        } = execute_plan(&plan, &work, &out, bundle, &resolved, &run_id)?;
        let success = actions.iter().all(|a| a["status"] == "succeeded");
        manifest["actions"] = json!(actions);
        manifest["reports"] = json!(reports);
        manifest["artifacts"] = json!(artifacts);
        manifest["diagnostics"] = json!(diagnostics);
        manifest["status"] = json!(if success { "succeeded" } else { "failed" });
        Ok(plan)
    })();
    if plan_only {
        return result;
    }
    if let Err(error) = result {
        manifest["status"] = json!("failed");
        manifest["diagnostics"].as_array_mut().unwrap().push(json!({"code":"build-failed","phase":"build","severity":"error","message":format!("{error:#}")}));
    }
    // Copy only validated declared files. Unlisted output cannot enter the bundle.
    for item in manifest["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .chain(manifest["reports"].as_array().unwrap().iter())
    {
        if let Some(path) = item["path"].as_str() {
            let source = safe_file(&out, path)?;
            let destination = bundle.join(path);
            fs::create_dir_all(destination.parent().unwrap())?;
            fs::copy(source, destination)?;
        }
    }
    records::write(&bundle.join("envelope.json"), &envelope)?;
    manifest["envelopeDigest"] = json!(snapshot::file_digest(&bundle.join("envelope.json"))?);
    records::write(&bundle.join("manifest.json"), &manifest)?;
    if dist.exists() {
        let history = state.join("history");
        if history.exists() && fs::symlink_metadata(&history)?.file_type().is_symlink() {
            bail!("history must not be a symlink");
        }
        fs::create_dir_all(&history)?;
        let previous = history.join(&run_id);
        fs::rename(&dist, &previous)?;
        if let Err(error) = fs::rename(bundle, &dist) {
            fs::rename(previous, &dist)?;
            return Err(error.into());
        }
    } else {
        fs::rename(bundle, &dist)?;
    }
    Ok(manifest)
}

/// Verify recorded content; this is integrity checking, not producer authentication.
pub fn inspect(root: &Path) -> Result<Value> {
    let manifest = records::read(&safe_file(root, "manifest.json")?)?;
    if manifest["kind"] != "build-manifest" {
        bail!("not an Oyzu build bundle");
    }
    {
        let (path_key, digest_key) = ("envelopePath", "envelopeDigest");
        let path = manifest[path_key]
            .as_str()
            .context("missing envelope path")?;
        if snapshot::file_digest(&safe_file(root, path)?)? != manifest[digest_key] {
            bail!("envelope digest mismatch");
        }
    }
    if let Some(path) = manifest["planPath"].as_str() {
        let plan = records::read(&safe_file(root, path)?)?;
        if records::digest("oyzu.plan.v1alpha1", &plan)? != manifest["planDigest"] {
            bail!("plan digest mismatch");
        }
    } else if manifest["status"] == "succeeded" {
        bail!("successful bundle has no plan");
    }
    for field in ["artifacts", "reports"] {
        for item in manifest[field]
            .as_array()
            .context("missing bundle records")?
        {
            if let Some(path) = item["path"].as_str() {
                let file = safe_file(root, path)?;
                if snapshot::file_digest(&file)? != item["digest"] {
                    bail!("{path}: content digest mismatch");
                }
                if field == "artifacts" && Some(fs::metadata(file)?.len()) != item["size"].as_u64()
                {
                    bail!("{path}: size mismatch");
                }
            }
        }
    }
    Ok(manifest)
}
