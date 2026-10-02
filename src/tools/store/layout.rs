//! Finite archive layouts; no scripts, network, expressions or executable hooks.
use super::{
    access::{self, Directory},
    archive::{self, Bounds},
    tree::TreeInspection,
};
use crate::tools::{lock, ToolCandidateRequest, VerifiedBlob};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    format: u32,
    backend_digest: String,
    platform: String,
    input_blob_digests: Vec<String>,
    archive_kind: String,
    strip_prefix: Option<String>,
    payload_subtree: String,
    required_paths: Vec<RequiredPath>,
    entrypoints: BTreeMap<String, Launch>,
    environment: BTreeMap<String, Environment>,
    extraction_bounds: Bounds,
    executable_paths: Vec<String>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RequiredPath {
    path: String,
    kind: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    owner: String,
    relative_path: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Launch {
    kind: String,
    payload_relative_path: String,
    interpreter_tool_key: Option<String>,
    interpreter_relative_path: Option<String>,
    prefix_args: Vec<Argument>,
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Argument {
    Literal { value: String },
    Path { path: Reference },
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Environment {
    Literal { value: String },
    Paths { paths: Vec<Reference> },
}

pub(in crate::tools) fn stage(
    request: ToolCandidateRequest<'_>,
    bytes: &[u8],
    blob: VerifiedBlob,
    lock: &lock::Lock,
) -> Result<String> {
    lock::digest(request.installer_release_digest)?;
    lock::digest(request.admitted_layout_digest)?;
    let value = crate::config::policy::strict_json_limit(bytes, 2 * 1024 * 1024)?;
    ensure!(
        value.get("strip_prefix").is_some(),
        "layout requires explicit strip-prefix field"
    );
    for launch in value
        .get("entrypoints")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|m| m.values())
    {
        ensure!(
            launch.get("interpreter_tool_key").is_some()
                && launch.get("interpreter_relative_path").is_some(),
            "layout requires explicit interpreter fields"
        );
    }
    let plan: Plan = serde_json::from_value(value).context("invalid archive layout plan")?;
    ensure!(plan.format == 1, "unsupported archive layout format");
    let layout_digest =
        crate::records::digest("oyzu.archive-layout.v1", &serde_json::to_value(&plan)?)?;
    ensure!(
        layout_digest == request.admitted_layout_digest,
        "layout differs from admitted descriptor"
    );
    let selection = lock
        .selections
        .iter()
        .find(|s| {
            s.scope == request.scope
                && s.profile == request.profile
                && s.platform == request.platform
        })
        .context("locked selection is unavailable")?;
    let installation = selection
        .installation_keys
        .get(request.tool_key)
        .context("tool is not in selected closure")?;
    let tool = lock
        .tool
        .iter()
        .find(|t| t.key == request.tool_key)
        .context("missing locked tool")?;
    let distribution = tool
        .distribution
        .iter()
        .find(|d| d.platform == request.platform)
        .context("missing locked distribution")?;
    ensure!(
        plan.backend_digest == tool.backend_digest
            && plan.platform == request.platform
            && layout_digest == distribution.layout_digest
            && plan.input_blob_digests == [distribution.digest.clone()]
            && blob.digest() == distribution.digest
            && blob.size() == distribution.size
            && distribution.package_closure_digest.is_none(),
        "archive layout differs from locked distribution"
    );
    // Unsupported transforms are errors, never silently ignored.
    ensure!(
        matches!(plan.archive_kind.as_str(), "tar" | "tar.gz")
            && plan.payload_subtree == "."
            && plan.executable_paths.is_empty(),
        "archive layout transform is not implemented"
    );
    plan.extraction_bounds.validate()?;
    if let Some(prefix) = &plan.strip_prefix {
        access::relative(prefix)?;
    }
    validate_required(&plan.required_paths)?;
    let (entrypoints, environment) = resolve_templates(
        &plan,
        installation,
        &distribution.dependencies,
        &selection.installation_keys,
    )?;
    let root = Directory::open(&std::path::absolute(request.staging)?)?;
    ensure!(
        root.entries()?.is_empty(),
        "candidate staging must be empty"
    );
    let tree = archive::unpack_layout(
        blob,
        root.create_directory("payload")?,
        plan.archive_kind == "tar.gz",
        plan.strip_prefix.as_deref(),
        &plan.extraction_bounds,
    )?;
    check_payload(&plan, &tree)?;
    let mut dependencies: Vec<_> = distribution
        .dependencies
        .iter()
        .map(|key| selection.installation_keys[key].clone())
        .collect();
    dependencies.sort();
    let receipt = json!({"format":1,"installation_key":installation,"tool_key":tool.key,
        "tool_id":tool.id,"version":tool.version,"platform":request.platform,"backend_digest":tool.backend_digest,
        "distribution_digest":distribution.digest,"distribution_size":distribution.size,"layout_digest":layout_digest,
        "verification":distribution.verification,"package_closure_digest":null,"dependency_installation_keys":dependencies,
        "tree_digest":tree.digest,"tree_manifest_digest":tree.manifest_digest,"entrypoints":entrypoints,
        "environment":environment,"installer_release_digest":request.installer_release_digest});
    let bytes = serde_json_canonicalizer::to_vec(&receipt)?;
    ensure!(
        bytes.len() <= 2 * 1024 * 1024,
        "candidate receipt exceeds limit"
    );
    // The candidate is still uncommitted. Publication revalidates the complete
    // closure, including interpreter/environment references into dependencies.
    let mut file = root.create_file("receipt.json")?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    root.sync()?;
    Ok(installation.clone())
}

fn validate_required(paths: &[RequiredPath]) -> Result<()> {
    ensure!(
        !paths.is_empty() && paths.len() <= 4096,
        "invalid required path count"
    );
    let mut previous = None;
    for path in paths {
        access::relative(&path.path)?;
        ensure!(
            matches!(path.kind.as_str(), "file" | "directory" | "symlink"),
            "invalid required path type"
        );
        ensure!(
            previous.is_none_or(|p| p < path.path.as_str()),
            "required paths must be sorted and unique"
        );
        previous = Some(path.path.as_str());
    }
    Ok(())
}
fn literal(value: &str) -> Result<()> {
    ensure!(
        value.len() <= 16 * 1024 && !value.contains('\0'),
        "invalid layout literal"
    );
    Ok(())
}
fn reference(
    value: &Reference,
    installation: &str,
    dependencies: &[String],
    keys: &BTreeMap<String, String>,
) -> Result<Value> {
    if value.relative_path != "." {
        access::relative(&value.relative_path)?;
    }
    let key = if value.owner == "self" {
        installation
    } else {
        ensure!(
            dependencies.contains(&value.owner),
            "layout references an undeclared dependency"
        );
        keys.get(&value.owner)
            .context("layout dependency is absent")?
    };
    Ok(json!({"installation_key":key,"relative_path":value.relative_path}))
}
fn resolve_templates(
    plan: &Plan,
    installation: &str,
    dependencies: &[String],
    keys: &BTreeMap<String, String>,
) -> Result<(Value, Value)> {
    ensure!(
        !plan.entrypoints.is_empty()
            && plan.entrypoints.len() <= 1024
            && plan.environment.len() <= 1024,
        "layout entrypoint/environment limit exceeded"
    );
    let resolve = |r: &Reference| reference(r, installation, dependencies, keys);
    let mut commands = BTreeSet::new();
    let mut entrypoints = serde_json::Map::new();
    for (command, launch) in &plan.entrypoints {
        access::component(command)?;
        ensure!(
            commands.insert(command.to_uppercase()),
            "layout command case collision"
        );
        access::relative(&launch.payload_relative_path)?;
        ensure!(
            launch.prefix_args.len() <= 256,
            "layout argument limit exceeded"
        );
        let args = launch
            .prefix_args
            .iter()
            .map(|a| -> Result<Value> {
                Ok(match a {
                    Argument::Literal { value } => {
                        literal(value)?;
                        json!({"kind":"literal","value":value})
                    }
                    Argument::Path { path } => json!({"kind":"path","path":resolve(path)?}),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let launch = match (
            launch.kind.as_str(),
            &launch.interpreter_tool_key,
            &launch.interpreter_relative_path,
        ) {
            ("native", None, None) => {
                json!({"kind":"native","payload_relative_path":launch.payload_relative_path,"prefix_args":args})
            }
            ("interpreter", Some(owner), Some(path)) => {
                access::relative(path)?;
                json!({"kind":"interpreter","payload_relative_path":launch.payload_relative_path,
                    "prefix_args":args,"interpreter":resolve(&Reference { owner:owner.clone(), relative_path:path.clone() })?})
            }
            _ => anyhow::bail!("invalid layout interpreter fields"),
        };
        entrypoints.insert(command.clone(), launch);
    }
    let mut names = BTreeSet::new();
    let mut environment = serde_json::Map::new();
    for (name, value) in &plan.environment {
        ensure!(
            !name.is_empty()
                && name.len() <= 256
                && name.bytes().enumerate().all(|(i, c)| c == b'_'
                    || c.is_ascii_alphabetic()
                    || (i > 0 && c.is_ascii_digit())),
            "invalid layout environment name"
        );
        ensure!(
            names.insert(name.to_ascii_uppercase()),
            "layout environment case collision"
        );
        let value = match value {
            Environment::Literal { value } => {
                ensure!(
                    !name.eq_ignore_ascii_case("PATH"),
                    "layout PATH requires typed references"
                );
                literal(value)?;
                json!({"kind":"literal","value":value})
            }
            Environment::Paths { paths } => {
                ensure!(paths.len() <= 256, "layout environment path limit exceeded");
                json!({"kind":"paths","paths":paths.iter().map(resolve).collect::<Result<Vec<_>>>()?})
            }
        };
        environment.insert(name.clone(), value);
    }
    Ok((Value::Object(entrypoints), Value::Object(environment)))
}

fn check_payload(plan: &Plan, tree: &TreeInspection) -> Result<()> {
    let index: BTreeMap<_, _> = tree.entries.iter().map(|e| (e.path.as_str(), e)).collect();
    for path in &plan.required_paths {
        ensure!(
            index
                .get(path.path.as_str())
                .is_some_and(|e| e.kind == path.kind),
            "required layout path is missing or has wrong type"
        );
    }
    let own_path =
        |reference: &Reference, directory: Option<bool>, executable: bool| -> Result<()> {
            if reference.owner != "self" {
                return Ok(());
            }
            if reference.relative_path == "." && directory != Some(false) {
                return Ok(());
            }
            let entry = super::receipt::resolve(&index, &reference.relative_path)?;
            ensure!(
                match directory {
                    Some(true) => entry.kind == "directory",
                    Some(false) => entry.kind == "file",
                    None => matches!(entry.kind, "file" | "directory"),
                },
                "layout reference has wrong payload type"
            );
            ensure!(
                !executable || plan.platform.starts_with("windows/") || entry.executable != 0,
                "layout interpreter is not executable"
            );
            Ok(())
        };
    for launch in plan.entrypoints.values() {
        let entry = super::receipt::resolve(&index, &launch.payload_relative_path)?;
        ensure!(entry.kind == "file", "layout entrypoint is not a file");
        if launch.kind == "native" {
            ensure!(
                plan.platform.starts_with("windows/") || entry.executable != 0,
                "layout entrypoint is not executable"
            );
        }
        if let (Some(owner), Some(path)) = (
            &launch.interpreter_tool_key,
            &launch.interpreter_relative_path,
        ) {
            own_path(
                &Reference {
                    owner: owner.clone(),
                    relative_path: path.clone(),
                },
                Some(false),
                true,
            )?;
        }
        for arg in &launch.prefix_args {
            if let Argument::Path { path } = arg {
                own_path(path, None, false)?;
            }
        }
    }
    for value in plan.environment.values() {
        if let Environment::Paths { paths } = value {
            for path in paths {
                own_path(path, Some(true), false)?;
            }
        }
    }
    Ok(())
}
