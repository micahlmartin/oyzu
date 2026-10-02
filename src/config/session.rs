//! Invocation configuration capture shared by CLI and discovery.
use super::{
    constraints::Constraints,
    locations::{protected_read, Locations},
    registry::{Registry, Scope},
    resolve::{self, EffectiveConfig, Selection},
    sources::{self, ConfigSource},
};
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub root: Option<PathBuf>,
    pub profile: Option<String>,
    pub no_profile: bool,
    pub local_overrides: bool,
    pub all_profiles: bool,
}
#[derive(Clone)]
pub struct Session {
    pub root: PathBuf,
    pub registry: Registry,
    pub ci: bool,
    selection: Selection,
    sources: Vec<ConfigSource>,
    constraints: Constraints,
    local: bool,
    pub locations: Locations,
    known_profiles: std::collections::BTreeMap<String, Vec<String>>,
    selection_reason: Option<String>,
    captured: std::collections::BTreeMap<PathBuf, Vec<ConfigSource>>,
    management: Option<serde_json::Value>,
    invocation: Vec<ConfigSource>,
}
impl Session {
    fn validate_capture_limits(&self) -> Result<()> {
        let unique: std::collections::BTreeMap<_, _> = self
            .sources
            .iter()
            .chain(self.captured.values().flatten())
            .map(|source| (&source.identity, (source.syntax.len(), source.entry_count)))
            .collect();
        if unique.len() > 128
            || unique.values().map(|(bytes, _)| bytes).sum::<usize>() > 8 * 1024 * 1024
            || unique.values().map(|(_, entries)| entries).sum::<usize>() > 10000
        {
            bail!("CONFIG_LIMIT: invocation source limit exceeded");
        }
        Ok(())
    }
    pub fn open(directory: &Path, options: &Options) -> Result<Self> {
        let root = workspace_root(directory, options.root.as_deref())?;
        let registry = Registry::default();
        let locations = Locations::native()?;
        let ci = sources::detected_ci();
        let mut sources = Vec::new();
        let mut constraints = Constraints::default();
        // Enrollment inspection precedes every ordinary source. Invalid records are never absence.
        let mut managed = false;
        let mut management = None;
        if let Some(bytes) = protected_read(&locations.machine.join("management.json"))? {
            managed = true;
            let bootstrap = super::managed::Bootstrap::parse(&bytes)?;
            let acquired =
                super::agent::Agent::new(bootstrap, &locations, &root, ci)?.acquire(false)?;
            management = Some(
                serde_json::json!({"revision":acquired.snapshot.revision(),"offlineDeadline":acquired.snapshot.deadline(),"onlineContext":acquired.online}),
            );
            sources.push(acquired.snapshot.policy().source(
                "corporate",
                &locations.machine,
                Scope::Corporate,
                &registry,
                &mut constraints,
            )?);
        }
        if let Some(bytes) = protected_read(&locations.machine.join("admin-settings.json"))? {
            sources.push(super::policy::Policy::parse(&bytes)?.source(
                "local-administration",
                &locations.machine,
                Scope::Admin,
                &registry,
                &mut constraints,
            )?);
        }
        if let Some(source) = ConfigSource::read(
            &locations.machine.join("config.toml"),
            Scope::Machine,
            false,
            &registry,
        )? {
            sources.push(source);
        }
        if let Some(source) = ConfigSource::read(&locations.user, Scope::User, false, &registry)? {
            sources.push(source);
        }
        let mut administrative = registry.defaults();
        for source in sources
            .iter()
            .filter(|source| source.scope.administrative())
        {
            administrative.extend(source.base.values.clone());
        }
        // This gate is administrative-only; ordinary sources cannot affect it.
        let controlled = managed
            || sources.iter().any(|source| {
                source.scope.administrative()
                    && source.base.values.contains_key("config.localOverridesInCi")
            })
            || !constraints.origins("config.localOverridesInCi").is_empty();
        let local = !ci
            || options.local_overrides
                && (!controlled
                    || administrative.get("config.localOverridesInCi")
                        == Some(&serde_json::json!(true)));
        if ci && options.local_overrides && !local {
            bail!("CONFIG_OVERRIDE_DENIED: policy disallows local overrides in CI");
        }
        let directory = directory.canonicalize()?;
        let mut captured = std::collections::BTreeMap::new();
        captured.insert(
            root.clone(),
            sources::project_sources(&root, &root, local, &registry)?,
        );
        if directory != root {
            let target_sources = sources::project_sources(&root, &directory, local, &registry)?;
            validate_shared_capture(&captured, &directory, &target_sources)?;
            captured.insert(directory.clone(), target_sources);
        }
        let inherited = if options.profile.is_none() && !options.no_profile {
            std::env::var("OYZU_INHERITED_PROFILE").ok()
        } else {
            None
        };
        let mut invocation = Vec::new();
        for (key, environment) in registry.environment_aliases() {
            if let Ok(value) = std::env::var(environment) {
                let (namespace, setting) = key.split_once('.').expect("registered setting path");
                let text = toml::to_string(&serde_json::json!({namespace:{setting:value}}))?;
                let mut source = ConfigSource::parse(
                    environment,
                    &root,
                    Scope::Invocation,
                    false,
                    &text,
                    &registry,
                )?;
                source.diagnostics.push(sources::Diagnostic {
                    code: "CONFIG_DEPRECATED".into(),
                    source: environment.into(),
                    key: key.into(),
                    message: format!(
                        "use {key}; legacy environment input remains subject to policy"
                    ),
                    remedy: format!("Set {key} in an ordinary configuration source"),
                    ..sources::Diagnostic::default()
                });
                invocation.push(source);
            }
        }
        Ok(Self {
            root,
            registry,
            ci,
            selection: Selection {
                explicit: options
                    .profile
                    .clone()
                    .or_else(|| inherited.clone().filter(|value| value != "@none")),
                no_profile: options.no_profile || inherited.as_deref() == Some("@none"),
                environment: std::env::var("OYZU_PROFILE").ok(),
            },
            sources,
            constraints,
            local,
            locations,
            known_profiles: Default::default(),
            selection_reason: None,
            captured,
            management,
            invocation,
        })
    }
    pub fn resolve(&self, target: &Path, all_profiles: bool) -> Result<EffectiveConfig> {
        let mut sources = self.sources.clone();
        sources.extend(
            self.captured
                .get(&target.canonicalize()?)
                .ok_or_else(|| {
                    anyhow::anyhow!("CONFIG_SCOPE: target was not captured for this invocation")
                })?
                .clone(),
        );
        if !self.known_profiles.is_empty() {
            let mut marker = ConfigSource::parse(
                "profile-catalogue",
                &self.root,
                Scope::Project,
                false,
                "",
                &self.registry,
            )?;
            for name in self.known_profiles.keys() {
                marker.profiles.insert(name.clone(), Default::default());
            }
            sources.push(marker);
        }
        sources.extend(self.invocation.clone());
        let mut result = resolve::resolve(
            &sources,
            &self.registry,
            self.ci,
            &self.selection,
            self.constraints.clone(),
            all_profiles,
        )?;
        result.management = self.management.clone();
        result
            .diagnostics
            .extend(
                self.locations
                    .diagnostics
                    .iter()
                    .map(|message| sources::Diagnostic {
                        code: "CONFIG_LOCATION".into(),
                        source: "native-locations".into(),
                        key: String::new(),
                        message: message.clone(),
                        remedy: "Use an absolute XDG directory or remove the invalid override"
                            .into(),
                        ..sources::Diagnostic::default()
                    }),
            );
        for diagnostic in &mut result.diagnostics {
            diagnostic.target = Some(
                target
                    .strip_prefix(&self.root)
                    .unwrap_or(target)
                    .display()
                    .to_string(),
            );
        }
        if let Some(reason) = &self.selection_reason {
            result.selection_reason = reason.clone();
            result.profiles = self.known_profiles.clone();
        }
        if !self.local {
            result
                .exclusions
                .push("oyzu.local.toml files excluded in inferred CI before parsing".into());
        }
        if all_profiles {
            for profile in result.profiles.keys() {
                let selection = Selection {
                    explicit: Some(profile.clone()),
                    ..Default::default()
                };
                resolve::resolve(
                    &sources,
                    &self.registry,
                    self.ci,
                    &selection,
                    self.constraints.clone(),
                    false,
                )?;
            }
        }
        Ok(result)
    }
    pub(crate) fn requested_root(&self) -> Result<EffectiveConfig> {
        let mut requested = self.clone();
        requested.constraints = Constraints::default();
        let mut result = requested.resolve(&self.root, false)?;
        result.constraints = self.constraints.clone();
        Ok(result)
    }
    pub fn select_for_targets(&mut self, targets: &[PathBuf]) -> Result<()> {
        let mut sources = self.sources.clone();
        sources.extend(self.captured[&self.root].clone());
        let mut catalogue = ConfigSource::parse(
            "target-profile-catalogue",
            &self.root,
            Scope::Project,
            false,
            "",
            &self.registry,
        )?;
        for target in targets {
            let target = target.canonicalize()?;
            if !self.captured.contains_key(&target) {
                let captured =
                    sources::project_sources(&self.root, &target, self.local, &self.registry)?;
                validate_shared_capture(&self.captured, &target, &captured)?;
                self.captured.insert(target.clone(), captured);
                // Reject an oversized union before reading more targets. Shared
                // ancestor files participate once in the invocation budget.
                self.validate_capture_limits()?;
            }
            for source in &self.captured[&target] {
                for name in source.profiles.keys() {
                    catalogue.profiles.entry(name.clone()).or_default();
                }
            }
        }
        let unique: std::collections::BTreeMap<_, _> = self
            .sources
            .iter()
            .chain(self.captured.values().flatten())
            .map(|source| (&source.identity, source))
            .collect();
        self.validate_capture_limits()?;
        let mut profile_origins =
            std::collections::BTreeMap::from([("ci".to_owned(), vec!["built-in".to_owned()])]);
        for source in unique
            .values()
            .filter(|source| !(self.ci && source.scope == Scope::User))
        {
            for profile in source.profiles.keys() {
                profile_origins
                    .entry(profile.clone())
                    .or_default()
                    .push(source.identity.clone());
            }
        }
        sources.push(catalogue);
        // Selection is invocation-wide; target-specific values and constraints
        // are evaluated only after the target's complete cascade is available.
        for source in &mut sources {
            source.base.values.retain(|key, _| {
                matches!(key.as_str(), "profile.default" | "config.allowedProfiles")
            });
            source.base.remove.clear();
            source.base.requires.clear();
            for overlay in source.profiles.values_mut() {
                *overlay = Default::default();
            }
        }
        let selected = resolve::resolve(
            &sources,
            &self.registry,
            self.ci,
            &self.selection,
            Constraints::default(),
            false,
        )?;
        self.selection_reason = Some(selected.selection_reason);
        self.known_profiles = profile_origins;
        self.selection.explicit = selected.profile;
        self.selection.no_profile = self.selection.explicit.is_none();
        Ok(())
    }
}
pub fn workspace_root(directory: &Path, explicit: Option<&Path>) -> Result<PathBuf> {
    let directory = directory.canonicalize()?;
    if let Some(root) = explicit {
        let root = root.canonicalize()?;
        if !directory.starts_with(&root) {
            bail!("CONFIG_SCOPE: invocation directory is outside --root");
        }
        return Ok(root);
    }
    for parent in directory.ancestors() {
        if parent.join(".git").exists() {
            return Ok(parent.into());
        }
    }
    Ok(directory)
}

// Compare the complete shared scope, including absence. Comparing only matching
// identities would miss a file created or deleted after an earlier capture.
fn validate_shared_capture(
    prior: &std::collections::BTreeMap<PathBuf, Vec<ConfigSource>>,
    target: &Path,
    captured: &[ConfigSource],
) -> Result<()> {
    for (prior_target, prior_sources) in prior {
        let shared = |source: &&ConfigSource| {
            target.starts_with(&source.directory) && prior_target.starts_with(&source.directory)
        };
        let previous: std::collections::BTreeMap<_, _> = prior_sources
            .iter()
            .filter(shared)
            .map(|s| (&s.identity, &s.digest))
            .collect();
        let current: std::collections::BTreeMap<_, _> = captured
            .iter()
            .filter(shared)
            .map(|s| (&s.identity, &s.digest))
            .collect();
        if previous != current {
            bail!("CONFIG_EDIT_CONFLICT: configuration changed during capture");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_capture_detects_creation_deletion_and_changes_but_allows_new_target_scope() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let child = root.join("child");
        std::fs::create_dir(&child).unwrap();
        let registry = Registry::default();
        let read = || sources::project_sources(&root, &child, true, &registry).unwrap();
        let absent = std::collections::BTreeMap::from([(root.clone(), Vec::new())]);
        std::fs::write(root.join("oyzu.local.toml"), "[build]\njobs=2\n").unwrap();
        let present = read();
        assert!(validate_shared_capture(&absent, &child, &present).is_err());
        let prior = std::collections::BTreeMap::from([(root.clone(), present)]);
        std::fs::remove_file(root.join("oyzu.local.toml")).unwrap();
        assert!(validate_shared_capture(&prior, &child, &read()).is_err());
        std::fs::write(root.join("oyzu.local.toml"), "[build]\njobs=3\n").unwrap();
        assert!(validate_shared_capture(&prior, &child, &read()).is_err());
        std::fs::write(root.join("oyzu.local.toml"), "[build]\njobs=2\n").unwrap();
        std::fs::write(child.join("oyzu.toml"), "[build]\njobs=4\n").unwrap();
        assert!(validate_shared_capture(&prior, &child, &read()).is_ok());
    }
}
