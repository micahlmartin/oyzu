//! Development install orchestration for one complete configured tool selection.
//! Uses existing lock transactions and store publication; never edits TOML.
use super::*;

struct Prepared {
    tool: Tool,
    metadata: Metadata,
    acquisition: Acquisition,
    record: lock::Tool,
    layout: serde_json::Value,
    acquired: Option<Vec<u8>>,
}

/// Resolve selected roots, prepare all missing payloads, then publish one lock.
/// Unselected roots retain their exact records; failed preparation leaves the
/// existing lock intact. Scope/platform expansion is a separate lock operation.
pub fn install(
    directory: &Path,
    options: &config::session::Options,
    store: &Path,
    frozen: bool,
    offline: bool,
    update: Option<&[String]>,
    bindings: Option<&Path>,
) -> Result<i32> {
    ensure!(
        update.is_none() || !(frozen || offline),
        "update conflicts with frozen/offline installation"
    );
    let directory = directory.canonicalize()?;
    let Configuration {
        session,
        effective,
        tools: configured,
    } = configuration(&directory, options)?;
    ensure!(
        directory == session.root,
        "initial tool install supports the workspace root scope"
    );
    let profile = effective.profile.as_deref().unwrap_or("default");
    let host = platform()?;
    let backend = identity()?;
    let aliases: BTreeMap<String, String> = worker_call(&WorkerRequest::Aliases, None)?;
    let requests =
        crate::tools::project_tool_requests(&effective, &aliases, &BTreeMap::new(), &[])?;
    let lock_path = session.root.join("oyzu.lock");
    let edit = crate::tools::ToolLockEdit::capture(&lock_path)?;
    let captured_bytes = if lock_path.exists() {
        Some(crate::tools::read_record(&lock_path, lock::MAX_BYTES)?)
    } else {
        None
    };
    let captured = captured_bytes.as_deref().map(lock::parse).transpose()?;
    ensure!(
        !(frozen || offline) || captured.is_some(),
        "frozen/offline install requires an existing oyzu.lock; run oyzu install online first"
    );
    let old_environment = captured.as_ref().and_then(|lock| {
        lock.environment
            .iter()
            .find(|env| env.scope == "." && env.profile == profile)
    });
    if let Some(names) = update {
        ensure!(
            names.iter().all(|name| configured
                .iter()
                .any(|(tool, _)| name == tool.name() || name == tool.id())),
            "update must name configured tools or their canonical IDs"
        );
        if let Some(lock) = &captured {
            ensure!(lock.environment.len() == 1 && old_environment.is_some(), "update currently requires a single matching root/profile; other selections are preserved by refusing this unsupported update");
            ensure!(
                lock.tool
                    .iter()
                    .all(|tool| tool.distribution.len() == 1
                        && tool.distribution[0].platform == host),
                "update currently supports only the current host"
            );
            if !names.is_empty() {
                ensure!(
                    old_environment
                        .unwrap()
                        .requests
                        .keys()
                        .all(|id| configured.iter().any(|(tool, _)| tool.id() == id)),
                    "removing tools requires an all-tool update"
                );
            }
        }
    }
    let mut prepared = Vec::new();
    for (tool, request) in &configured {
        let changed = update.is_some_and(|names| {
            names.is_empty()
                || names
                    .iter()
                    .any(|name| name == tool.name() || name == tool.id())
        });
        let old = captured.as_ref().and_then(|lock| {
            old_environment.and_then(|env| {
                lock.tool
                    .iter()
                    .find(|record| record.id == tool.id() && env.roots.contains(&record.key))
            })
        });
        if captured.is_some() && !changed {
            ensure!(
                old.is_some(),
                "TOOL_LOCK_STALE: missing configured tool; run oyzu install --update"
            );
            ensure!(
                old_environment.and_then(|env| env.requests.get(tool.id()))
                    == requests.requests().get(tool.id()),
                "TOOL_LOCK_STALE: unselected tool request changed; include it in --update"
            );
            ensure!(
                old.unwrap().backend_digest == backend,
                "locked backend differs; explicit relocking required"
            );
        }
        let mut acquisition = Acquisition::new(&effective, bindings, *tool)?;
        let metadata = metadata(
            &Request {
                tool: *tool,
                request: request.clone(),
                exact: if changed {
                    None
                } else {
                    old.map(|record| record.version.clone())
                },
                target: host.into(),
            },
            Some(&mut acquisition),
        )?;
        let old_distribution = old.and_then(|record| {
            record
                .distribution
                .iter()
                .find(|value| value.platform == host)
        });
        let digest = if !changed && old.is_some() {
            old_distribution
                .context("locked host platform unavailable")?
                .digest
                .clone()
        } else {
            metadata
                .declared_sha256
                .clone()
                .context("tool checksum missing")?
        };
        let layout = plan(*tool, &metadata.archive, &digest, &backend);
        let layout_digest = records::digest("oyzu.archive-layout.v1", &layout)?;
        let mut acquired = None;
        let record = if let Some(old) = old.filter(|_| !changed) {
            ensure!(
                old_distribution.unwrap().layout_digest == layout_digest,
                "locked layout differs from current adapter"
            );
            old.clone()
        } else {
            let prior_size = old
                .filter(|record| record.version == metadata.archive.version)
                .and(old_distribution)
                .filter(|distribution| distribution.digest == digest)
                .map(|distribution| distribution.size);
            let size = if let Some(size) = metadata.declared_size.or(prior_size) {
                size
            } else {
                ensure!(!offline, "archive size is unavailable offline");
                let bytes = acquire(&mut acquisition, &metadata)?;
                let size = bytes.len() as u64;
                acquired = Some(bytes);
                size
            };
            lock::Tool {
                key: records::digest(
                    "oyzu.tool-record.v2",
                    &json!({"id":tool.id(),"version":metadata.archive.version,"backend_digest":backend,"options":{}}),
                )?,
                id: tool.id().into(),
                version: metadata.archive.version.clone(),
                backend_digest: backend.clone(),
                options: BTreeMap::new(),
                distribution: vec![lock::Distribution {
                    platform: host.into(),
                    digest: digest.clone(),
                    size,
                    source_id: tool.source().into(),
                    artifact_id: metadata
                        .archive
                        .archive_url
                        .rsplit('/')
                        .next()
                        .context("tool artifact filename missing")?
                        .into(),
                    layout_digest,
                    dependencies: vec![],
                    package_closure_digest: None,
                    verification: lock::Verification {
                        kind: lock::VerificationKind::DigestOnly,
                        evidence_digest: digest.clone(),
                        verifier_digest: backend.clone(),
                        subject_digest: digest,
                    },
                }],
            }
        };
        if changed {
            println!(
                "{}: {} -> {}",
                tool.name(),
                old.map(|record| record.version.as_str())
                    .unwrap_or("(unlocked)"),
                record.version
            );
        }
        prepared.push(Prepared {
            tool: *tool,
            metadata,
            acquisition,
            record,
            layout,
            acquired,
        });
    }
    let bytes = if update.is_none() && captured.is_some() {
        crate::tools::select_for_tool_requests(&session.root, &directory, profile, &requests, host)
            .context(
                "TOOL_LOCK_STALE: run oyzu install --update to resolve changed requirements",
            )?;
        captured_bytes.unwrap()
    } else {
        toml::to_string(&lock::Lock {
            format: 2,
            selections: vec![],
            environment: vec![lock::Environment {
                scope: ".".into(),
                profile: profile.into(),
                request_digest: requests.digest().into(),
                roots: prepared
                    .iter()
                    .map(|item| item.record.key.clone())
                    .collect(),
                requests: requests.requests().clone(),
            }],
            tool: prepared.iter().map(|item| item.record.clone()).collect(),
        })?
        .into_bytes()
    };
    let proposal = if frozen {
        None
    } else {
        Some(edit.propose(&bytes)?)
    };
    let parsed = lock::parse(&bytes)?;
    let selection = parsed
        .selections
        .iter()
        .find(|selection| {
            selection.scope == "." && selection.profile == profile && selection.platform == host
        })
        .context("complete tool selection unavailable")?;
    std::fs::create_dir_all(store)?;
    let staging_root = store.join("staging");
    std::fs::create_dir_all(&staging_root)?;
    let staging = tempfile::tempdir_in(staging_root.canonicalize()?)?;
    std::fs::create_dir_all(staging.path().join("installs"))?;
    let candidate_lock = staging.path().join("oyzu.lock");
    std::fs::write(&candidate_lock, &bytes)?;
    for item in &mut prepared {
        let installation = &selection.installation_keys[&item.record.key];
        if store.join("installs").join(&installation[7..]).exists() {
            continue;
        }
        let distribution = item
            .record
            .distribution
            .iter()
            .find(|value| value.platform == host)
            .context("locked host unavailable")?;
        let blob = match crate::tools::store::cached(
            &std::path::absolute(store)?,
            &distribution.digest,
            distribution.size,
        )? {
            Some(blob) => blob,
            None => {
                ensure!(
                    !offline,
                    "locked tool archive is not cached; run oyzu install online to acquire it"
                );
                let bytes = match item.acquired.take() {
                    Some(bytes) => bytes,
                    None => acquire(&mut item.acquisition, &item.metadata)?,
                };
                crate::tools::cache_tool_blob(
                    store,
                    &mut std::io::Cursor::new(bytes),
                    &distribution.digest,
                    distribution.size,
                )?
            }
        };
        let candidate = staging.path().join("installs").join(&installation[7..]);
        std::fs::create_dir_all(&candidate)?;
        crate::tools::stage_tool_candidate(
            ToolCandidateRequest {
                lock_path: &candidate_lock,
                staging: &candidate,
                scope: ".",
                profile,
                platform: host,
                tool_key: &item.record.key,
                installer_release_digest: &backend,
                admitted_layout_digest: &distribution.layout_digest,
            },
            &serde_json::to_vec(&item.layout)?,
            blob,
        )?;
    }
    let _lease = crate::tools::lease_installation_selection(
        &candidate_lock,
        store,
        Some(staging.path()),
        ".",
        profile,
        host,
        &backend,
    )?;
    if let Some(proposal) = proposal {
        proposal.commit()?;
    }
    for item in prepared {
        println!("Installed {} {}", item.tool.name(), item.record.version);
    }
    crate::tools::shims::prepare(store, false)?;
    Ok(0)
}

fn acquire(acquisition: &mut Acquisition, metadata: &Metadata) -> Result<Vec<u8>> {
    let response = acquisition.fetch(&metadata.archive.archive_url)?;
    ensure!(
        response.status == 200,
        "tool archive acquisition returned {}",
        response.status
    );
    ensure!(
        metadata
            .declared_size
            .is_none_or(|size| size == response.body.len() as u64),
        "archive length differs from catalog"
    );
    Ok(response.body)
}
