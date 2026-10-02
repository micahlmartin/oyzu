//! Captured-source build lifecycle (OEP-0006/0007/0012).
mod bundle;
mod containers;
mod directory;
mod execution;
mod indices;
mod materialization;
mod planning;
mod scheduling;
mod selection;
mod task_graph;
mod variants;

use crate::{builders, discovery, records, snapshot};
use anyhow::{bail, Result};
use execution::{execute_plan, ExecutionRecords};
pub use planning::plan;
use planning::resolve_images;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

/// Read-only content verification for a build bundle or exported OCI layout tar.
/// This does not authenticate the producer or establish release eligibility.
pub fn inspect(path: &Path) -> Result<Value> {
    if path.is_file() {
        if fs::symlink_metadata(path)?.file_type().is_symlink() {
            bail!("OCI archive must not be a symlink");
        }
        let verified = crate::oci::verify(path)?;
        return Ok(json!({"schemaVersion":"v1alpha1","kind":"oci-verification",
            "artifactKind":verified.kind,"ociDigest":verified.digest,
            "platforms":verified.platforms,"digest":snapshot::file_digest(path)?,
            "size":fs::metadata(path)?.len(),"verification":"content-integrity"}));
    }
    bundle::inspect(path)
}

/// Build an immutable local bundle. Prior bundles are retained under .oyzu/history.
pub fn run(root: &Path, images: &[String], plan_only: bool) -> Result<Value> {
    run_with_options(
        root,
        images,
        plan_only,
        &crate::config::session::Options {
            root: Some(root.into()),
            ..Default::default()
        },
    )
}

pub fn run_with_options(
    root: &Path,
    images: &[String],
    plan_only: bool,
    options: &crate::config::session::Options,
) -> Result<Value> {
    run_selected_with_options(root, images, plan_only, options, &[])
}

/// Build requested targets and their transitive artifact/task prerequisites.
/// An empty request selects all discovered targets.
pub fn run_selected_with_options(
    root: &Path,
    images: &[String],
    plan_only: bool,
    options: &crate::config::session::Options,
    requested: &[String],
) -> Result<Value> {
    let root = crate::config::session::workspace_root(root, options.root.as_deref())?;
    // Capture and validate all administrative policy before any build side effects.
    let session = crate::config::session::Session::open(&root, options)?;
    // Native discovery failures still produce the ordinary failed build bundle.
    let workspace = discovery::discover_with_session(session, Some("sh"));
    let transaction = crate::bundle_store::Transaction::begin(&root)?;
    let temp = tempfile::Builder::new().prefix("oyzu-build-").tempdir()?;
    let source_path = temp.path().join("source");
    let bundle = transaction.path();
    let out = temp.path().join("outputs");
    fs::create_dir(&out)?;
    let run_id = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let mut envelope = json!({"schemaVersion":"v1alpha1","kind":"execution-envelope","runId":run_id,"planDigest":null,"startedAt":chrono::DateTime::<chrono::Utc>::from(SystemTime::now()).to_rfc3339(),"host":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH},"context":if crate::config::sources::detected_ci() {"ci-unverified"} else {"local"},"identity":{"subject":"local-process","verification":"local"},"policy":{"decisionIds":[]},"facts":[]});
    if envelope["host"]["os"] == "macos" {
        envelope["host"]["os"] = json!("darwin");
    }
    let mut manifest = json!({"schemaVersion":"v1alpha1","kind":"build-manifest","runId":run_id,"planDigest":null,"planPath":null,"source":null,"status":"failed","targets":[],"actions":[],"artifacts":[],"reports":[],"evidence":[],"diagnostics":[],"envelopePath":"envelope.json","envelopeDigest":null});
    let result = (|| -> Result<Value> {
        let mut workspace = workspace?;
        let mut selection = selection::Selection::new(&workspace, requested)?;
        let source = snapshot::capture(&root, &source_path)?;
        planning::verify_inventory_source(&workspace, &source)?;
        let variants = variants::expand(&mut workspace)?;
        selection.expand_variants(&workspace, &variants)?;
        for target in workspace.targets.values_mut() {
            target.path = source_path.join(target.path.strip_prefix(&root)?);
            for task in target.tasks.values_mut() {
                task.cwd = source_path.join(task.cwd.strip_prefix(&root)?);
            }
        }
        for task in workspace.tasks.values_mut() {
            task.cwd = source_path.join(task.cwd.strip_prefix(&root)?);
        }
        workspace.root = source_path.clone();
        let mut resolved = BTreeMap::new();
        let mut dependencies = BTreeMap::new();
        let mut intents = BTreeMap::new();
        loop {
            planning::target_order(&workspace, &selection.targets)?;
            let pending = selection
                .targets
                .iter()
                .filter(|id| !intents.contains_key(*id))
                .cloned()
                .collect();
            // Admit every new owner before resolving images or acquiring inputs.
            for id in &pending {
                let target = &workspace.targets[id];
                let builder = builders::get(&target.builder)?;
                if let Some(config) = workspace.configuration.get(id) {
                    if containers::requested(
                        workspace
                            .declarations
                            .targets
                            .get(id)
                            .and_then(|d| d.container.as_ref()),
                    )? {
                        crate::config::enforcement::execution_preflight(
                            config,
                            builders::get("docker/image")?.descriptor().tools,
                        )?;
                    }
                    crate::config::enforcement::execution_preflight(
                        config,
                        builder.descriptor().tools,
                    )?;
                    if config.management.is_some() && builder.acquisition_requires_network() {
                        bail!("CONFIG_OVERRIDE_DENIED: managed acquisition requires approved connector bindings; provision approved local dependency inputs before building offline");
                    }
                }
            }
            resolved.extend(resolve_images(&workspace, images, &pending)?);
            for id in pending {
                let target = &workspace.targets[&id];
                let destination = temp.path().join(format!("dependencies-{id}"));
                let execution_name = format!("oyzu-acquire-{run_id}-{id}");
                let builder = builders::get(&target.builder)?;
                let target_platform = builder.target_platform(
                    workspace
                        .declarations
                        .targets
                        .get(&id)
                        .and_then(|c| c.platform.as_deref()),
                    &resolved[&id],
                )?;
                if let Some(prepared) = builder.prepare(builders::PreparationContext {
                    configuration: workspace.configuration.get(&id),
                    target,
                    destination: &destination,
                    image: &resolved[&id],
                    target_platform: &target_platform,
                    source_digest: &source.digest,
                    execution_name: &execution_name,
                })? {
                    dependencies.insert(id.clone(), prepared);
                }
                let intent = planning::intent(&workspace, &id, &source, dependencies.get(&id))?;
                intents.insert(id, intent);
            }
            selection.expand(&workspace, &intents)?;
            if selection.targets.len() == intents.len() {
                break;
            }
        }
        let mut plan = planning::compile(&workspace, &source, &resolved, &dependencies, &intents)?;
        containers::augment(
            &workspace,
            &source,
            &mut plan,
            &mut resolved,
            &mut dependencies,
            temp.path(),
            &run_id,
        )?;
        indices::plan(&workspace, &variants, &mut plan)?;
        plan["extensions"]["oyzu.dev/selection"] = selection.record();
        manifest["extensions"]["oyzu.dev/selection"] = selection.record();
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
        if !dependencies.is_empty() {
            fs::create_dir(bundle.join("dependencies"))?;
        }
        for (id, dependency) in &dependencies {
            let path = format!("dependencies/{id}.json");
            records::write(&bundle.join(&path), &dependency.record)?;
            manifest["evidence"].as_array_mut().unwrap().push(json!({"id":format!("{id}/dependencies"),"kind":"dependency-snapshot","subjectDigest":dependency.digest,"producer":"oyzu-acquisition","verification":"local","path":path,"digest":snapshot::file_digest(&bundle.join(&path))?}));
        }
        let work = temp.path().join("work");
        snapshot::capture(&source_path, &work)?;
        let ExecutionRecords {
            actions,
            reports,
            artifacts,
            diagnostics,
            evidence,
        } = execute_plan(
            &plan,
            &work,
            &out,
            bundle,
            &resolved,
            &dependencies,
            &run_id,
        )?;
        let success = actions.iter().all(|a| a["status"] == "succeeded");
        manifest["actions"] = json!(actions);
        manifest["reports"] = json!(reports);
        manifest["artifacts"] = json!(artifacts);
        manifest["diagnostics"] = json!(diagnostics);
        manifest["evidence"]
            .as_array_mut()
            .unwrap()
            .extend(evidence);
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
    records::write(&bundle.join("envelope.json"), &envelope)?;
    manifest["envelopeDigest"] = json!(snapshot::file_digest(&bundle.join("envelope.json"))?);
    records::write(&bundle.join("manifest.json"), &manifest)?;
    transaction.publish(&run_id)?;
    Ok(manifest)
}
