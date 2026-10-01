//! Native local chart dependency preparation in the offline executor.
use super::{metadata, planning::environment, RUNTIME};
use crate::{builders::PreparationContext, dependencies::Prepared, executor, records, snapshot};
use anyhow::{bail, Result};
use serde_json::json;
use std::{fs, time::Duration};

pub(super) fn prepare(context: PreparationContext<'_>) -> Result<Prepared> {
    let control = tempfile::tempdir()?;
    let workspace = control.path().join("workspace");
    snapshot::capture(&context.target.path, &workspace)?;
    fs::create_dir(context.destination)?;
    let runtime = control.path().join("runtime");
    fs::create_dir(&runtime)?;
    for file in RUNTIME {
        fs::write(runtime.join(file.name), file.contents)?;
    }
    let run = |args: &[&str]| -> Result<String> {
        let stdout = control.path().join("stdout");
        let stderr = control.path().join("stderr");
        let result = executor::execute_with_mounts(
            executor::Request {
                image: context.image,
                workspace: &workspace,
                output: context.destination,
                cwd: "/workspace",
                argv: &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                env: &environment(),
                stdout: &stdout,
                stderr: &stderr,
                timeout: Duration::from_secs(120),
                name: context.execution_name,
            },
            &[executor::Mount {
                source: &runtime,
                destination: "/oyzu",
                readonly: true,
            }],
        )?;
        if result.code != 0 {
            bail!(
                "Helm preparation failed: {} {}",
                fs::read_to_string(stdout)?,
                fs::read_to_string(stderr)?
            );
        }
        Ok(fs::read_to_string(stdout)?)
    };
    let relative = metadata::chart_path(&workspace)?;
    let chart_root = workspace.join(relative);
    let mut locks = Vec::new();
    for path in metadata::local_order(&workspace, &chart_root)? {
        let lock = path.join("Chart.lock");
        let existing = lock.exists();
        if existing {
            locks.push(snapshot::file_digest(&lock)?);
        }
        let local = path
            .strip_prefix(workspace.canonicalize()?)?
            .to_str()
            .unwrap()
            .replace('\\', "/");
        let local = if local.is_empty() { "." } else { &local };
        run(&["helm", "dependency", "build", local, "--skip-refresh"])?;
        if !existing && lock.exists() {
            // Generated time is informational, excluded from Helm's dependency digest.
            let mut data: serde_yaml::Value = serde_yaml::from_str(&fs::read_to_string(&lock)?)?;
            data["generated"] = serde_yaml::Value::String("1970-01-01T00:00:00Z".into());
            fs::write(&lock, serde_yaml::to_string(&data)?)?;
        }
        if path.join("charts").exists() {
            run(&[
                "python",
                "-I",
                "/oyzu/helm-archive.py",
                &format!("{local}/charts"),
            ])?;
        }
    }
    let chart_file = chart_root.join("Chart.yaml");
    let mut value: serde_yaml::Value = serde_yaml::from_str(&fs::read_to_string(&chart_file)?)?;
    let original = metadata::read(&chart_root)?;
    let version = format!(
        "{}-dev.g{}",
        original.version.split(['-', '+']).next().unwrap(),
        &context.source_digest[7..19]
    );
    value["version"] = serde_yaml::Value::String(version);
    fs::write(chart_file, serde_yaml::to_string(&value)?)?;
    let manager_version = run(&["helm", "version", "--template", "{{.Version}}"])?;
    snapshot::capture(&chart_root, &context.destination.join("chart"))?;
    let tree = snapshot::capture(context.destination, &control.path().join("frozen"))?;
    let platform = json!({"os":context.image.os,"arch":context.image.arch});
    let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
        "adapter":{"id":"helm/local-charts","digest":snapshot::file_digest(&std::env::current_exe()?)?,"layoutVersion":"1"},
        "manager":{"id":"helm","version":manager_version.trim(),"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":locks,"targetPlatform":platform,"packages":[],"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/helm-chart":{"path":relative,"name":original.name,"originalVersion":original.version}}
    });
    Ok(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    })
}
