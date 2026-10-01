//! Read-only observations and order-independent resolution of exclusive roles.
//! Native detectors live with their ecosystem; this module knows no tool names.
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) use super::source::Source;

pub(crate) trait Detector<C>: Sync {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str {
        "1"
    }
    fn detect(&self, context: &C) -> Result<Vec<Finding>>;
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Strength {
    Native,
    Fallback,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct Evidence {
    pub(super) path: String,
    pub(super) digest: String,
    pub(super) location: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct Finding {
    candidate: String,
    strength: Strength,
    evidence: Vec<Evidence>,
}

impl Finding {
    pub fn native(candidate: &str, evidence: Vec<Evidence>) -> Self {
        Self {
            candidate: candidate.into(),
            strength: Strength::Native,
            evidence,
        }
    }
    pub fn fallback(candidate: &str) -> Self {
        Self {
            candidate: candidate.into(),
            strength: Strength::Fallback,
            evidence: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct Observation {
    detector: String,
    finding: Finding,
}

/// Persisted discovery evidence, not an external detector/plugin interface.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    selected: String,
    registry: BTreeMap<String, String>,
    observations: Vec<Observation>,
}

impl Resolution {
    pub fn selected(&self) -> &str {
        &self.selected
    }
}

pub(crate) fn exclusive<C>(
    role: &str,
    context: &C,
    detectors: &[&dyn Detector<C>],
) -> Result<Resolution> {
    // Validate and normalize registration before executing any detector.
    let mut registry = BTreeMap::new();
    let mut ordered = BTreeMap::new();
    for detector in detectors {
        if registry
            .insert(detector.id().to_string(), detector.version().to_string())
            .is_some()
        {
            bail!("duplicate detector {}", detector.id());
        }
        ordered.insert(detector.id(), *detector);
    }
    let mut observations = Vec::new();
    for (id, detector) in ordered {
        for mut finding in detector
            .detect(context)
            .with_context(|| format!("{id}: {role} detection failed"))?
        {
            if finding.candidate.is_empty()
                || (finding.strength == Strength::Native && finding.evidence.is_empty())
            {
                bail!("{id}: invalid detection evidence");
            }
            finding.evidence.sort();
            finding.evidence.dedup();
            observations.push(Observation {
                detector: id.into(),
                finding,
            });
        }
    }
    observations.sort();
    observations.dedup();
    let native = observations
        .iter()
        .any(|o| o.finding.strength == Strength::Native);
    let candidates: BTreeSet<_> = observations
        .iter()
        .filter(|o| !native || o.finding.strength == Strength::Native)
        .map(|o| o.finding.candidate.as_str())
        .collect();
    if candidates.len() != 1 {
        let evidence: Vec<_> = observations
            .iter()
            .flat_map(|o| {
                o.finding
                    .evidence
                    .iter()
                    .map(|e| format!("{} via {} {}", o.finding.candidate, e.path, e.location))
            })
            .collect();
        bail!("conflicting or missing {role}: candidates {candidates:?}; evidence {evidence:?}");
    }
    let selected = (*candidates.first().unwrap()).to_string();
    Ok(Resolution {
        selected,
        registry,
        observations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    struct Probe {
        id: &'static str,
        findings: Vec<Finding>,
    }
    impl Detector<()> for Probe {
        fn id(&self) -> &'static str {
            self.id
        }
        fn detect(&self, _: &()) -> Result<Vec<Finding>> {
            Ok(self.findings.clone())
        }
    }
    fn native(name: &str) -> Finding {
        Finding::native(
            name,
            vec![Evidence {
                path: format!("{name}.lock"),
                digest: format!("sha256:{}", "a".repeat(64)),
                location: "lockfile".into(),
            }],
        )
    }
    #[test]
    fn exclusive_resolution_is_order_independent_and_never_hides_native_conflicts() {
        let first = Probe {
            id: "manager/lock",
            findings: vec![native("native")],
        };
        let corroborating = Probe {
            id: "manager/declared",
            findings: vec![native("native")],
        };
        let default = Probe {
            id: "manager/default",
            findings: vec![Finding::fallback("default")],
        };
        let a = exclusive("manager", &(), &[&first, &corroborating, &default]).unwrap();
        let b = exclusive("manager", &(), &[&default, &corroborating, &first]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.selected(), "native");
        assert_eq!(a.observations.len(), 3);
        assert_eq!(
            exclusive("manager", &(), &[&default]).unwrap().selected(),
            "default"
        );
        let conflicting = Probe {
            id: "manager/other",
            findings: vec![native("other")],
        };
        let a = exclusive("manager", &(), &[&first, &conflicting, &default])
            .unwrap_err()
            .to_string();
        let b = exclusive("manager", &(), &[&default, &conflicting, &first])
            .unwrap_err()
            .to_string();
        assert_eq!(a, b);
        assert!(a.contains("native.lock") && a.contains("other.lock"));
        assert!(exclusive("manager", &(), &[&first, &first]).is_err());
        assert!(exclusive::<()>("manager", &(), &[]).is_err());
    }
    #[test]
    fn source_is_bounded_read_once_and_rejects_unsafe_metadata() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("package.json"), "{}").unwrap();
        let source = Source::read(root.path(), &["package.json"]).unwrap();
        let first = source.evidence("package.json", "/").unwrap();
        fs::write(root.path().join("package.json"), "changed after read").unwrap();
        assert_eq!(source.text("package.json"), Some("{}"));
        assert_eq!(source.evidence("package.json", "/").unwrap(), first);
        assert_ne!(
            Source::read(root.path(), &["package.json"])
                .unwrap()
                .evidence("package.json", "/")
                .unwrap(),
            first
        );
        assert!(Source::read(root.path(), &["../secret"]).is_err());
        fs::create_dir(root.path().join("directory")).unwrap();
        assert!(Source::read(root.path(), &["directory"]).is_err());
        fs::write(root.path().join("large"), vec![b'x'; 4 * 1024 * 1024 + 1]).unwrap();
        assert!(Source::read(root.path(), &["large"]).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("package.json", root.path().join("link")).unwrap();
            assert!(Source::read(root.path(), &["link"]).is_err());
        }
    }
}
