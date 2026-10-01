use super::RUNTIME;
use crate::{broker, builders::PreparationContext, dependencies::Prepared, records, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

pub(super) fn lockfile(root: &Path) -> Option<&'static str> {
    ["npm-shrinkwrap.json", "package-lock.json"]
        .into_iter()
        .find(|name| root.join(name).is_file())
}

/// Native acquisition is needed for all locked entries, including transitive
/// and optional inputs; a dependency-free unlocked project needs no broker.
pub(super) fn required(root: &Path, package: &Value) -> Result<bool> {
    if package.get("workspaces").is_some() {
        bail!("npm workspace build integration is not implemented yet");
    }
    let declared = [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ]
    .iter()
    .any(|field| package[field].as_object().is_some_and(|v| !v.is_empty()));
    let Some(lock) = lockfile(root) else {
        if declared {
            bail!("npm dependencies require a captured package-lock.json or npm-shrinkwrap.json; unlocked resolution is not implemented yet");
        }
        return Ok(false);
    };
    let value = records::read(&root.join(lock))?;
    if !matches!(value["lockfileVersion"].as_u64(), Some(2 | 3)) {
        bail!("npm acquisition requires a v2/v3 lockfile");
    }
    let packages = value["packages"]
        .as_object()
        .context("npm lockfile missing packages")?;
    Ok(declared || packages.keys().any(|p| !p.is_empty()))
}

pub(super) fn prepare(context: PreparationContext<'_>) -> Result<Option<Prepared>> {
    let package = records::read(&context.target.path.join("package.json"))?;
    // Validate the admitted input profile, including dependency-free projects.
    // Every native build must verify its provisioned manager and runtime first.
    let needs_registry = required(&context.target.path, &package)?;
    let sources = if needs_registry {
        vec![broker::Source::new(
            "npm-public",
            "https://registry.npmjs.org/",
            None,
        )?]
    } else {
        vec![]
    };
    let tree = crate::dependencies::preparation::capture(
        &context,
        RUNTIME,
        &["node".into(), "/oyzu/npm.mjs".into(), "acquire".into()],
        &BTreeMap::from([("HOME".into(), "/tmp/oyzu-home".into())]),
        sources,
    )?;
    let inventory = records::read(&context.destination.join("inventory.json"))?;
    let packages: Vec<Value> = inventory["packages"].as_array().context("missing npm inventory")?
        .iter().enumerate().map(|(index, p)| json!({
            "id":format!("npm/package-{index}"), "name":p["name"], "version":p["version"],
            "sourceId":p["sourceId"], "digest":format!("sha256:{}",p["sha256"].as_str().unwrap_or("")),
            "size":p["size"], "purpose":p["purpose"], "dependencies":[], "verification":"digest-only"
        })).collect();
    let lock = lockfile(&context.target.path);
    let lock_digests: Vec<String> = lock
        .map(|name| snapshot::file_digest(&context.target.path.join(name)))
        .transpose()?
        .into_iter()
        .collect();
    let platform = json!({"os":context.image.os,"arch":context.image.arch});
    let record = json!({"schemaVersion":"v1alpha1","kind":"dependency-snapshot",
        "adapter":{"id":"node/npm-registry-tarballs","digest":snapshot::file_digest(&std::env::current_exe()?)?,"layoutVersion":"1"},
        "manager":{"id":"npm","version":inventory["version"],"digest":context.image.digest,"platform":platform},
        "sourceDigest":context.source_digest,"lockDigests":lock_digests,
        "targetPlatform":platform,"packages":packages,"preparedTree":tree.digest,
        "extensions":{"oyzu.dev/npm":{"lockfile":lock,"nodeVersion":inventory["nodeVersion"],"inventory":"all-locked-registry-tarballs","dependencyEdges":"not-modeled","integrity":"lockfile-sha512"}}});
    Ok(Some(Prepared {
        root: context.destination.into(),
        digest: records::digest("oyzu.dependencies.v1alpha1", &record)?,
        record,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn npm_admission_requires_native_lock_and_observes_shrinkwrap_precedence() {
        let root = tempfile::tempdir().unwrap();
        assert!(!required(root.path(), &json!({})).unwrap());
        let package = json!({"dependencies":{"example":"1.0.0"}});
        assert!(required(root.path(), &package).is_err());
        fs::write(
            root.path().join("package-lock.json"),
            r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
        )
        .unwrap();
        assert!(required(root.path(), &package).unwrap());
        assert!(!required(root.path(), &json!({})).unwrap());
        fs::write(
            root.path().join("npm-shrinkwrap.json"),
            r#"{"lockfileVersion":1}"#,
        )
        .unwrap();
        assert_eq!(lockfile(root.path()), Some("npm-shrinkwrap.json"));
        assert!(required(root.path(), &package).is_err());
        assert!(required(root.path(), &json!({"workspaces":[]})).is_err());
    }
}
