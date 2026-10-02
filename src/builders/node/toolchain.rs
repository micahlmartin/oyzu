//! Node runtime constraints shared by native manager adapters. Image families
//! remain manager-owned; lock interpretation and installation remain native.
use crate::model::Target;
use anyhow::{bail, Result};
use std::collections::BTreeMap;

pub(super) fn requested(target: &Target) -> Result<Option<&str>> {
    if target.variant.is_empty() {
        return Ok(None);
    }
    let Some(version) = target
        .variant
        .get("node")
        .filter(|_| target.variant.len() == 1)
    else {
        bail!(
            "{}: Node builder requires one node runtime axis",
            target.name
        );
    };
    let components: Vec<_> = version.split('.').collect();
    if components.len() != 3
        || components.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|b| b.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
                || part.parse::<u32>().is_err()
        })
    {
        bail!(
            "{}: matrix.node requires an exact major.minor.patch runtime",
            target.name
        );
    }
    Ok(Some(version))
}

pub(super) fn preparation_environment(target: &Target) -> Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::from([("HOME".into(), "/tmp/oyzu-home".into())]);
    if let Some(version) = requested(target)? {
        env.insert("OYZU_EXPECT_NODE".into(), version.into());
    }
    Ok(env)
}

pub(super) fn verify(target: &Target, observed: Option<&str>) -> Result<()> {
    if let Some(version) = requested(target)? {
        if observed != Some(version) {
            bail!("{}: runtime variant requires matching captured Node preflight evidence (requested {version}, observed {observed:?})", target.name);
        }
    }
    Ok(())
}
