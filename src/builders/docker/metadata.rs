use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Metadata {
    pub schema_version: String,
    pub frontend: String,
    pub stages: Vec<Stage>,
    pub requirements: Vec<Requirement>,
    pub context: ContextFiles,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Stage {
    name: String,
    base: String,
    platform: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Requirement {
    kind: String,
    reference: Option<String>,
    stage: i64,
    line: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ContextFiles {
    pub ignore_file: Option<String>,
    pub files: Vec<String>,
}

impl Metadata {
    pub fn image_references(&self) -> Result<Vec<String>> {
        let mut references = std::collections::BTreeSet::new();
        for requirement in &self.requirements {
            if matches!(requirement.kind.as_str(), "image" | "image-or-context") {
                let value = requirement.reference.as_deref().unwrap_or("");
                if value.is_empty()
                    || value.len() > 512
                    || !value.as_bytes()[0].is_ascii_alphanumeric()
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"/._:@-".contains(&b))
                {
                    bail!("invalid static Docker image reference");
                }
                references.insert(value.to_string());
            }
        }
        if references.len() > 64 {
            bail!("Docker image input count exceeds limit");
        }
        Ok(references.into_iter().collect())
    }

    pub fn validate(&self, platform: &str) -> Result<()> {
        self.image_references()?;
        if self.schema_version != "v1alpha1"
            || self.frontend != "dockerfile.v0"
            || self.stages.is_empty()
        {
            bail!("Docker build requires the captured built-in frontend and native stages");
        }
        for requirement in &self.requirements {
            let allowed = match requirement.kind.as_str() {
                "image" | "image-or-context" => true, // Must be captured before plan admission.
                "cache-mount" => true, // A new worker owns the cache; no cross-run import/export.
                "platform" => requirement.reference.as_deref() == Some(platform),
                "add-source" => requirement
                    .reference
                    .as_deref()
                    .is_some_and(|p| !p.contains([':', '$', '@']) && !p.starts_with('/')),
                _ => false,
            };
            if !allowed {
                bail!("Dockerfile line {}: {} requires captured-input or capability integration before this profile can build", requirement.line, requirement.kind);
            }
        }
        if self.context.files.len() > 100000
            || self
                .context
                .files
                .iter()
                .any(|p| !crate::snapshot::portable(p))
        {
            bail!("invalid native Docker context inventory");
        }
        Ok(())
    }
}
