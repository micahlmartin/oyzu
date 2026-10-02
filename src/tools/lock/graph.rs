use super::{Lock, Tool};
use anyhow::{ensure, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(lock: &Lock) -> Result<Vec<super::super::LockedSelection>> {
    let mut selections = Vec::new();
    let tools: BTreeMap<_, _> = lock
        .tool
        .iter()
        .map(|tool| (tool.key.as_str(), tool))
        .collect();
    let mut reachable = BTreeSet::new();
    // Validate every distribution, including those not available on a root's
    // platform, so invalid dormant graphs cannot become active after an update.
    for tool in &lock.tool {
        for distribution in &tool.distribution {
            let mut state = Walk::default();
            walk(tool, &distribution.platform, &tools, &mut state, 1)?;
        }
    }
    for environment in &lock.environment {
        let mut root_ids = BTreeSet::new();
        let mut platforms: Option<BTreeSet<&str>> = None;
        for key in &environment.roots {
            let root = tools
                .get(key.as_str())
                .context("environment references missing root")?;
            ensure!(root_ids.insert(root.id.as_str()), "ambiguous root tool ID");
            let available = root
                .distribution
                .iter()
                .map(|d| d.platform.as_str())
                .collect::<BTreeSet<_>>();
            platforms = Some(match platforms {
                None => available,
                Some(previous) => previous.intersection(&available).copied().collect(),
            });
        }
        ensure!(
            root_ids == environment.requests.keys().map(String::as_str).collect(),
            "requests must match root tool IDs exactly"
        );
        if let Some(platforms) = platforms {
            ensure!(
                !platforms.is_empty(),
                "environment roots have no common platform"
            );
            for platform in platforms {
                let mut state = Walk::default();
                for key in &environment.roots {
                    walk(tools[key.as_str()], platform, &tools, &mut state, 1)?;
                }
                reachable.extend(state.done.keys().copied());
                let records = state
                    .done
                    .keys()
                    .map(|key| {
                        let tool = tools[key];
                        let distribution = tool
                            .distribution
                            .iter()
                            .find(|d| d.platform == platform)
                            .expect("walk validated the selected platform");
                        serde_json::json!({"key": tool.key, "id": tool.id,
                        "version": tool.version, "backend_digest": tool.backend_digest,
                        "options": tool.options, "distribution": distribution})
                    })
                    .collect::<Vec<_>>();
                selections.push(super::super::LockedSelection {
                    scope: environment.scope.clone(),
                    profile: environment.profile.clone(),
                    platform: platform.into(),
                    digest: crate::records::digest(
                        "oyzu.tool-selection.v2",
                        &serde_json::json!({
                            "platform": platform, "roots": environment.roots, "tools": records
                        }),
                    )?,
                    installation_keys: state
                        .installations
                        .into_iter()
                        .map(|(key, value)| (key.to_owned(), value))
                        .collect(),
                });
            }
        }
    }
    ensure!(
        reachable.len() == tools.len(),
        "unreachable locked tool records"
    );
    Ok(selections)
}

#[derive(Default)]
struct Walk<'a> {
    active: BTreeSet<&'a str>,
    done: BTreeMap<&'a str, usize>,
    ids: BTreeMap<&'a str, &'a str>,
    installations: BTreeMap<&'a str, String>,
}

fn walk<'a>(
    tool: &'a Tool,
    platform: &str,
    tools: &BTreeMap<&str, &'a Tool>,
    state: &mut Walk<'a>,
    depth: usize,
) -> Result<usize> {
    ensure!(depth <= 64, "tool dependency depth exceeds 64");
    if let Some(height) = state.done.get(tool.key.as_str()) {
        ensure!(depth + height - 1 <= 64, "tool dependency depth exceeds 64");
        return Ok(*height);
    }
    ensure!(state.active.insert(&tool.key), "tool dependency cycle");
    if let Some(previous) = state.ids.insert(&tool.id, &tool.key) {
        ensure!(
            previous == tool.key,
            "ambiguous canonical ID in tool closure"
        );
    }
    let distribution = tool
        .distribution
        .iter()
        .find(|d| d.platform == platform)
        .context("dependency has no matching platform")?;
    let mut height = 1;
    for key in &distribution.dependencies {
        let dependency = tools.get(key.as_str()).context("missing tool dependency")?;
        height = height.max(1 + walk(dependency, platform, tools, state, depth + 1)?);
    }
    state.active.remove(tool.key.as_str());
    let dependencies = distribution.dependencies.iter().map(|key|
        serde_json::json!({"key": key, "installation_key": state.installations[key.as_str()]}))
        .collect::<Vec<_>>();
    let installation_key = crate::records::digest(
        "oyzu.installation.v1",
        &serde_json::json!({
            "tool": {"key": tool.key, "id": tool.id, "version": tool.version,
                "backend_digest": tool.backend_digest, "options": tool.options},
            "distribution": distribution, "dependencies": dependencies
        }),
    )?;
    state.installations.insert(&tool.key, installation_key);
    state.done.insert(&tool.key, height);
    Ok(height)
}
