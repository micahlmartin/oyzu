//! Optional packaging of exact application artifacts in private derived targets.
//! Builders supply runtime requirements; this owner composes acquisition/actions.
use crate::{builders, config, dependencies, executor, model::Workspace, records, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path, time::Duration};

/// Add image assembly only after the complete native graph has been planned.
/// Derived targets have empty source projections and consume verified artifacts.
pub(super) fn augment(
    workspace: &Workspace,
    source: &snapshot::Snapshot,
    plan: &mut Value,
    images: &mut BTreeMap<String, executor::Image>,
    prepared: &mut BTreeMap<String, dependencies::Prepared>,
    temp: &Path,
    run_id: &str,
) -> Result<()> {
    let originals = plan["targets"]
        .as_array()
        .context("missing targets")?
        .clone();
    for original in originals {
        let owner = original["id"].as_str().context("missing target identity")?;
        let Some(options) = workspace
            .declarations
            .targets
            .get(owner)
            .and_then(|d| d.container.as_ref())
            .and_then(config::Container::options)
        else {
            continue;
        };
        let native = &workspace.targets[owner];
        if !native.variant.is_empty() {
            bail!("{owner}: application-container matrix aggregation requires runtime-profile expansion");
        }
        let profile = builders::get(&native.builder)?
            .container_profile(native, prepared.get(owner))?
            .with_context(|| {
                format!(
                    "{owner}: container runtime profile is not implemented for {}",
                    native.builder
                )
            })?;
        let base = options.base.as_deref().unwrap_or(profile.base);
        let id = crate::names::scoped(owner, "container");
        if workspace
            .targets
            .keys()
            .any(|name| name.eq_ignore_ascii_case(&id))
        {
            bail!("derived container target {id} collides with a project target");
        }
        let configuration = workspace.target_configuration(owner)?;
        let docker = builders::get("docker/image")?;
        config::enforcement::execution_preflight(configuration, docker.descriptor().tools)?;
        let platform = serde_json::from_value(original["platform"].clone())?;
        let worker = executor::resolve_for(docker.toolchain(native)?, docker.executor_profile())?;
        let runtime = executor::resolve_for(base, executor::Profile::Process)?;
        if runtime.platform()? != platform {
            bail!("{owner}: container runtime platform differs from the tested application");
        }
        let matching: Vec<_> = plan["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| {
                a["target"] == owner && a["name"] == profile.artifact && a["kind"] == "file"
            })
            .cloned()
            .collect();
        if matching.len() != 1 {
            bail!("{owner}: container profile requires its exact application artifact");
        }
        let artifact = &matching[0];
        let destination = temp.join(format!("container-inputs-{id}"));
        fs::create_dir(&destination)?;
        let control = tempfile::tempdir()?;
        let probe_workspace = control.path().join("workspace");
        let probe_output = control.path().join("output");
        fs::create_dir(&probe_workspace)?;
        fs::create_dir(&probe_output)?;
        let stdout = control.path().join("stdout");
        let stderr = control.path().join("stderr");
        let probe = executor::execute(executor::Request {
            image: &runtime,
            workspace: &probe_workspace,
            output: &probe_output,
            cwd: "/workspace",
            argv: &profile.probe,
            env: &BTreeMap::new(),
            stdout: &stdout,
            stderr: &stderr,
            timeout: Duration::from_secs(60),
            name: &format!("oyzu-runtime-{run_id}-{id}"),
        })?;
        if probe.code != 0
            || fs::metadata(&stdout)?.len() > 8192
            || fs::read_to_string(&stdout)?.trim() != profile.expected
        {
            bail!(
                "{owner}: provisioned container runtime does not satisfy {}",
                profile.id
            );
        }
        let bindings = dependencies::images::capture(
            dependencies::images::Capture {
                destination: &destination,
                image: &worker,
                target_platform: &platform,
                execution_name: &format!("oyzu-container-acquire-{run_id}-{id}"),
            },
            &[base.into()],
        )?;
        if bindings[0].config != runtime.digest {
            bail!("{owner}: runtime image changed between compatibility probe and capture");
        }
        let tree = snapshot::capture_prepared(&destination, &control.path().join("frozen"))?;
        let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
            "adapter":{"id":"oyzu/container-inputs","digest":original["builderDigest"],"layoutVersion":"1"},
            "manager":{"id":"buildkit","version":worker.reference,"digest":worker.digest,"platform":{"os":worker.os,"arch":worker.arch}},
            "sourceDigest":source.digest,"lockDigests":[],"targetPlatform":platform,"packages":[],"preparedTree":tree.digest,
            "extensions":{"oyzu.dev/container":{"owner":owner,"profile":profile.id,"runtime":runtime,"probe":profile.expected,"images":bindings}}});
        let dependency = dependencies::Prepared {
            root: destination,
            digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
            record,
        };
        let (uid, gid) = options.identity()?.unwrap_or((65532, 65532));
        let mode = executor::ImageRecipe {
            base: executor::ImageBase::Captured {
                reference: base.into(),
            },
            copies: vec![executor::ImageCopy {
                source: profile.payload.into(),
                destination: format!("/app/{}", profile.payload),
            }],
            uid,
            gid,
            workdir: options.workdir.unwrap_or_else(|| "/app".into()),
            entrypoint: options.entrypoint.unwrap_or(profile.entrypoint),
        };
        let apparmor = configuration
            .get("docker.apparmorProfile")
            .and_then(Value::as_str)
            .context("missing resolved container execution profile")?;
        let output = format!("{id}/container/image.tar");
        // Image tags use the OCI-compatible snapshot form, independently of a
        // native distribution's version syntax (for example PEP 440 '+').
        let version = builders::semver_snapshot(native, source);
        let filename = format!("{owner}-{version}.oci.tar");
        let mut build = super::planning::action(
            &format!("{id}:build"),
            &id,
            "build",
            vec![],
            ".",
            &BTreeMap::new(),
            (
                &worker,
                &source.digest,
                &executor::Mode::Buildkit {
                    output: output.clone(),
                    image_name: format!("oyzu/{}:{version}", owner.to_lowercase()),
                    context_files: vec![],
                    apparmor_profile: apparmor.into(),
                    dockerfile_digest: mode.digest(&bindings)?,
                    generated_recipe: Some(Box::new(mode)),
                    dependency_context: None,
                    images: bindings,
                },
                &platform,
            ),
        );
        let report_id = format!("{id}/tests");
        let report_path = format!("{id}/reports/junit.xml");
        let mut test = super::planning::action(
            &format!("{id}:test"),
            &id,
            "test",
            vec![],
            ".",
            &BTreeMap::new(),
            (
                &worker,
                &source.digest,
                &executor::Mode::OciValidation {
                    input: output.clone(),
                    report: report_path.clone(),
                },
                &platform,
            ),
        );
        test["reports"] = json!([crate::reports::Intent::required(
            &report_id,
            &id,
            crate::reports::Format::Junit
        )]);
        test["extensions"]["oyzu.dev/report-paths"] = json!({report_id.clone():report_path});
        test["extensions"]["oyzu.dev/report-sources"] = json!({report_id:"file"});
        test["extensions"]["oyzu.dev/collect-after"] = json!(format!("{id}:test"));
        let mut package = super::planning::action(
            &format!("{id}:package"),
            &id,
            "package",
            vec![
                "cp".into(),
                format!("/out/{output}"),
                format!("/out/{id}/artifacts/{filename}"),
            ],
            ".",
            &BTreeMap::new(),
            (&worker, &source.digest, &executor::Mode::Process, &platform),
        );
        package["outputs"] = json!([format!("{id}/image")]);
        let input = json!({"kind":"artifact","artifact":artifact["id"],"producer":artifact["producer"],"mount":profile.payload});
        let mut previous = artifact["producer"]
            .as_str()
            .context("missing artifact producer")?
            .to_owned();
        for action in [&mut build, &mut test, &mut package] {
            let mode: executor::Mode =
                serde_json::from_value(action["extensions"]["oyzu.dev/executor"].clone())?;
            mode.validate()?;
            action["dependsOn"] = json!([previous]);
            action["inputs"].as_array_mut().unwrap().push(input.clone());
            action["inputs"].as_array_mut().unwrap().push(
                json!({"kind":"dependency","digest":dependency.digest,"mount":"dependencies"}),
            );
            action["extensions"]["oyzu.dev/configuration-digest"] = json!(configuration.digest);
            previous = action["id"].as_str().unwrap().into();
        }
        let target = json!({"id":id,"builder":"docker/image","builderDigest":original["builderDigest"],"path":".","variant":native.variant,"platform":platform,
            "extensions":{"oyzu.dev/source-projection":snapshot::Projection::new(".", &[])?,
                "oyzu.dev/configuration":{"digest":configuration.digest,"profile":configuration.profile},
                "oyzu.dev/container":{"owner":owner,"profile":profile.id},
                "oyzu.dev/coverage-applicability":{"status":"inapplicable","reason":"Image structure assertions do not measure application lines; materialization references the exact tested application reports."}}});
        plan["targets"].as_array_mut().unwrap().push(target);
        plan["tools"].as_array_mut().unwrap().push(json!({"id":id,"version":worker.reference,"digest":worker.digest,"platform":{"os":worker.os,"arch":worker.arch}}));
        plan["actions"]
            .as_array_mut()
            .unwrap()
            .extend([build, test, package]);
        plan["artifacts"].as_array_mut().unwrap().push(json!({"id":format!("{id}/image"),"target":id,"variant":native.variant,"name":"image","producer":format!("{id}:package"),"kind":"oci-image","version":version,"mediaType":"application/vnd.oci.image.layout.v1+tar","path":format!("{id}/artifacts/{filename}")}));
        images.insert(id.clone(), worker);
        prepared.insert(id, dependency);
    }
    if plan["targets"].as_array().unwrap().len() > 256
        || plan["actions"].as_array().unwrap().len() > 16_384
    {
        bail!("container expansion exceeds target/action limits");
    }
    super::scheduling::Schedule::new(plan["actions"].as_array().unwrap(), 1)?;
    Ok(())
}
