//! Typed execution capabilities selected by built-in adapters, never shell text.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default)]
pub(crate) enum Profile {
    #[default]
    Process,
    RootlessBuildkit,
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
            Self::Buildkit { image_name, .. } => Some(
                [
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
                .collect(),
            ),
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
        } = self
        {
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
