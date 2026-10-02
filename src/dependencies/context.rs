//! Composition of native package preparation and an exact offline consumer store.
//! Consumers supply their captured runtime; providers own layout and resolution.
use crate::{builders, executor, records, snapshot};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[cfg(test)]
mod tests;

pub(crate) trait Provider: Sync {
    fn id(&self) -> &'static str;
    fn tools(&self) -> &'static [&'static str];
    fn detect(&self, source: &Path) -> bool;
    /// Prepare native inputs using the consumer's exact runtime and broker.
    /// Return a credential-free store within Prepared.root at store().
    fn prepare(&self, context: builders::PreparationContext<'_>) -> Result<super::Prepared>;
    fn store(&self) -> &'static str;
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Captured {
    pub binding: executor::DependencyContext,
    pub provider: String,
    pub runtime: executor::Image,
    pub snapshot_digest: String,
    pub snapshot: serde_json::Value,
}

fn select(context: &builders::PreparationContext<'_>) -> Result<&'static dyn Provider> {
    let providers: Vec<_> = builders::all()
        .iter()
        .flat_map(|b| b.dependency_providers())
        .collect();
    if let Some(selector) = context.dependency_selector {
        let provider = providers
            .into_iter()
            .find(|p| p.id() == selector)
            .with_context(|| format!("dependency provider {selector} is not implemented"))?;
        if !provider.detect(&context.target.path) {
            bail!("dependency provider {selector} requires its native manifest");
        }
        return Ok(*provider);
    }
    // Unsupported ecosystems still participate in ambiguity detection; installing
    // one supported closure must not silently ignore another native manifest.
    let ecosystems: Vec<_> = builders::all()
        .iter()
        .filter(|b| {
            !b.descriptor()
                .ids
                .contains(&context.target.builder.as_str())
        })
        .filter(|b| b.detect(&context.target.path).is_some())
        .collect();
    if ecosystems.len() != 1 {
        bail!("dependency context requires one unambiguous ecosystem; select a dependencies provider in build.yaml");
    }
    let matches: Vec<_> = ecosystems[0]
        .dependency_providers()
        .iter()
        .filter(|p| p.detect(&context.target.path))
        .collect();
    if matches.len() != 1 {
        bail!("dependency context has no unambiguous implemented native provider");
    }
    Ok(*matches[0])
}

/// Capture only the provider's offline store. Broker/control files stay outside
/// it. No tag is pulled, credentials are not passed to the runtime, and managed
/// preparation fails until an approved connector binding exists.
pub(crate) fn prepare(
    context: &builders::PreparationContext<'_>,
    runtime: &executor::Image,
) -> Result<Captured> {
    let provider = select(context)?;
    crate::config::enforcement::execution_preflight(context.configuration, provider.tools())?;
    if context.configuration.management.is_some() {
        bail!("CONFIG_OVERRIDE_DENIED: managed dependency contexts require approved connector bindings");
    }
    if runtime.platform()? != *context.target_platform {
        bail!("dependency preparation runtime differs from the consumer target platform");
    }
    if context.image.platform()? != *context.target_platform {
        bail!("dependency preparation requires matching native worker and target platforms");
    }
    let control = tempfile::tempdir()?;
    let prepared = provider.prepare(builders::PreparationContext {
        log: context.log.clone(),
        configuration: context.configuration,
        dependency_selector: None,
        target: context.target,
        destination: &control.path().join("prepared"),
        image: runtime,
        target_platform: context.target_platform,
        source_digest: context.source_digest,
        execution_name: &format!("{}-packages", context.execution_name),
    })?;
    if !snapshot::portable(provider.store()) {
        bail!("invalid provider store path");
    }
    let store = "contexts/packages";
    fs::create_dir(context.destination.join("contexts"))?;
    let tree = snapshot::capture_prepared(
        &prepared.root.join(provider.store()),
        &context.destination.join(store),
    )?;
    Ok(Captured {
        binding: executor::DependencyContext {
            store: store.into(),
            tree_digest: tree.digest,
            platform: context.target_platform.clone(),
        },
        provider: provider.id().into(),
        runtime: runtime.clone(),
        snapshot_digest: prepared.digest,
        snapshot: prepared.record,
    })
}

impl Captured {
    pub fn validate(
        &self,
        base: &str,
        images: &[executor::ImageInput],
        platform: &crate::platform::Platform,
        source: &str,
    ) -> Result<()> {
        if self.binding.platform != *platform
            || self.runtime.platform()? != *platform
            || self.runtime.reference != base
            || self.snapshot["manager"]["digest"] != self.runtime.digest
            || !images
                .iter()
                .any(|i| i.reference == base && i.config == self.runtime.digest)
            || self.snapshot["sourceDigest"] != source
            || records::digest("oyzu.dependencies.v1alpha1", &self.snapshot)?
                != self.snapshot_digest
        {
            bail!("dependency context does not match captured runtime, source or preparation identity");
        }
        Ok(())
    }
}
