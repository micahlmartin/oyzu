//! Typed execution capabilities selected by built-in adapters, never shell text.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Fixed native export epoch shared with Docker input selection.
pub(crate) const BUILDKIT_SOURCE_DATE_EPOCH: &str = "315532800";

#[derive(Clone, Copy, Default)]
pub(crate) enum Profile {
    #[default]
    Process,
    RootlessBuildkit,
    ImageInput,
}

/// Verified content store binding supplied by preparation, never a host path.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImageInput {
    pub reference: String,
    pub name: String,
    pub store: String,
    pub manifest: String,
    pub config: String,
    pub tree_digest: String,
}

impl ImageInput {
    pub fn validate(&self) -> Result<()> {
        let reference = |s: &str| {
            !s.is_empty()
                && s.len() <= 512
                && s.as_bytes()[0].is_ascii_alphanumeric()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/._:@-".contains(&b))
        };
        let digest = |s: &str| {
            s.len() == 71
                && s.starts_with("sha256:")
                && s[7..].bytes().all(|b| b.is_ascii_hexdigit())
        };
        if !reference(&self.reference)
            || !reference(&self.name)
            || !self.store.starts_with("images/base-")
            || !crate::snapshot::portable(&self.store)
            || !digest(&self.manifest)
            || !digest(&self.config)
            || !digest(&self.tree_digest)
        {
            bail!("invalid captured image binding");
        }
        Ok(())
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum Mode {
    #[default]
    Process,
    OciValidation {
        input: String,
        report: String,
    },
    Buildkit {
        output: String,
        image_name: String,
        context_files: Vec<String>,
        apparmor_profile: String,
        dockerfile_digest: String,
        #[serde(default)]
        images: Vec<ImageInput>,
    },
}

impl Mode {
    pub fn argv(&self, platform: &str) -> Option<Vec<String>> {
        match self {
            Self::Process => None,
            Self::OciValidation { input, report } => Some(vec![
                "oyzu:validate-oci".into(),
                format!("/out/{input}"),
                "--platform".into(),
                platform.into(),
                "--junit".into(),
                format!("/out/{report}"),
            ]),
            Self::Buildkit {
                image_name, images, ..
            } => {
                let mut args: Vec<String> = [
                    "buildctl",
                    "build",
                    "--progress=plain",
                    "--no-cache",
                    "--frontend",
                    "dockerfile.v0",
                    "--local",
                    "context=/workspace",
                    "--local",
                    "dockerfile=/definition",
                    "--opt",
                    &format!("platform={platform}"),
                    "--opt",
                    "force-network-mode=none",
                    "--opt",
                    &format!("build-arg:SOURCE_DATE_EPOCH={BUILDKIT_SOURCE_DATE_EPOCH}"),
                    "--output",
                    &format!(
                        "type=oci,dest=/output/image.tar,name={image_name},rewrite-timestamp=true"
                    ),
                ]
                .into_iter()
                .map(str::to_owned)
                .collect();
                // Validation requires aliases of one native context to bind
                // identical content. Emit its first deterministic store once.
                let mut contexts = std::collections::BTreeSet::new();
                for (index, image) in images.iter().enumerate() {
                    if !contexts.insert(&image.name) {
                        continue;
                    }
                    args.extend([
                        "--oci-layout".into(),
                        format!("base-{index}=/inputs/base-{index}"),
                        "--opt".into(),
                        format!(
                            "context:{}=oci-layout://base-{index}@{}",
                            image.name, image.manifest
                        ),
                    ]);
                }
                Some(args)
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        if let Self::OciValidation { input, report } = self {
            if !crate::snapshot::portable(input)
                || !crate::snapshot::portable(report)
                || input.eq_ignore_ascii_case(report)
            {
                bail!("invalid OCI validation input/report paths");
            }
        }
        if let Self::Buildkit {
            output,
            image_name,
            context_files,
            apparmor_profile,
            dockerfile_digest,
            images,
        } = self
        {
            let mut names = std::collections::BTreeMap::new();
            if images.len() > 64 {
                bail!("too many captured base images");
            }
            for image in images {
                image.validate()?;
                let identity = (&image.manifest, &image.config, &image.tree_digest);
                if names
                    .insert(&image.name, identity)
                    .is_some_and(|previous| previous != identity)
                {
                    bail!(
                        "conflicting captured identities for image context {}",
                        image.name
                    );
                }
            }
            if dockerfile_digest.len() != 71
                || !dockerfile_digest.starts_with("sha256:")
                || !dockerfile_digest[7..]
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit())
            {
                bail!("invalid captured Dockerfile identity");
            }
            if !crate::snapshot::portable(output)
                || output.contains('\\')
                || output.starts_with('/')
            {
                bail!("invalid BuildKit output destination");
            }
            if image_name.is_empty()
                || !image_name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/._:-".contains(&b))
            {
                bail!("invalid planned image name");
            }
            if apparmor_profile.is_empty()
                || !apparmor_profile
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            {
                bail!("invalid BuildKit AppArmor profile");
            }
            if context_files.len() > 100000
                || context_files.iter().any(|p| {
                    !crate::snapshot::portable(p) || p.contains('\\') || p.starts_with('/')
                })
            {
                bail!("invalid BuildKit context paths");
            }
        }
        Ok(())
    }

    pub fn enforced(&self) -> &'static [&'static str] {
        match self {
            Self::Process => &[
                "docker-network-none",
                "docker-read-only-root",
                "docker-cap-drop-all",
            ],
            Self::OciValidation { .. } => &[
                "engine-bounded-oci-validation",
                "no-project-code-execution",
                "contained-report-write",
            ],
            Self::Buildkit { .. } => &[
                "docker-network-none",
                "buildkit-rootless-worker",
                "buildkit-process-sandbox",
                "buildkit-private-store",
                "buildkit-no-result-cache-import",
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equivalent_image_references_share_a_context_but_conflicts_fail() {
        let image = ImageInput {
            reference: "alpine:3.22".into(),
            name: "alpine:3.22".into(),
            store: "images/base-0".into(),
            manifest: format!("sha256:{}", "1".repeat(64)),
            config: format!("sha256:{}", "2".repeat(64)),
            tree_digest: format!("sha256:{}", "3".repeat(64)),
        };
        let mut alias = image.clone();
        alias.reference = "docker.io/library/alpine:3.22".into();
        alias.store = "images/base-1".into();
        let mode = Mode::Buildkit {
            output: "image.tar".into(),
            image_name: "example/app:1".into(),
            context_files: vec![],
            apparmor_profile: "unconfined".into(),
            dockerfile_digest: format!("sha256:{}", "4".repeat(64)),
            images: vec![image, alias],
        };
        mode.validate().unwrap();
        let args = mode.argv("linux/amd64").unwrap();
        assert_eq!(args.iter().filter(|a| *a == "--oci-layout").count(), 1);
        assert_eq!(
            args.iter()
                .filter(|a| a.starts_with("context:alpine:"))
                .count(),
            1
        );
        assert!(args.contains(&"base-0=/inputs/base-0".into()));
        for field in ["manifest", "config", "tree_digest"] {
            let mut record = serde_json::to_value(&mode).unwrap();
            record["images"][1][field] = format!("sha256:{}", "5".repeat(64)).into();
            let conflict: Mode = serde_json::from_value(record).unwrap();
            assert!(conflict
                .validate()
                .unwrap_err()
                .to_string()
                .contains("conflicting captured identities"));
        }
    }
}
