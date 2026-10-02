//! Typed execution capabilities selected by built-in adapters, never shell text.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

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
                    "build-arg:SOURCE_DATE_EPOCH=315532800",
                    "--output",
                    &format!(
                        "type=oci,dest=/output/image.tar,name={image_name},rewrite-timestamp=true"
                    ),
                ]
                .into_iter()
                .map(str::to_owned)
                .collect();
                for (index, image) in images.iter().enumerate() {
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
            let mut names = std::collections::BTreeSet::new();
            if images.len() > 64 {
                bail!("too many captured base images");
            }
            for image in images {
                image.validate()?;
                if !names.insert(&image.name) {
                    bail!("duplicate captured image context");
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
