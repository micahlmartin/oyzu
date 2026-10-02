//! Resolve provisioned toolchain platforms; never pull images or install emulation.
use super::{Image, Profile};
use crate::platform::Platform;
use anyhow::{bail, Context, Result};

pub(crate) fn resolve_toolchain(
    reference: &str,
    profile: Profile,
    required: Option<&Platform>,
    explicit: bool,
) -> Result<Image> {
    resolve_with(reference, profile, required, explicit, super::resolve_for)
}

fn resolve_with(
    reference: &str,
    profile: Profile,
    required: Option<&Platform>,
    explicit: bool,
    mut inspect: impl FnMut(&str, Profile) -> Result<Image>,
) -> Result<Image> {
    let image = inspect(reference, profile)?;
    let Some(required) = required else {
        return Ok(image);
    };
    let actual = image.platform()?;
    if actual == *required {
        return Ok(image);
    }
    if explicit {
        bail!("required platform {required} differs from explicit toolchain platform {actual}; native target execution requires a matching provisioned image");
    }
    // Defaults are owned tagged references. Digest overrides remain immutable
    // choices and must never be rewritten into guessed tags.
    if reference.contains('@') || !reference.rsplit('/').next().unwrap_or("").contains(':') {
        bail!("platform-specific toolchain selection requires a tagged default image");
    }
    let sibling = format!("{reference}-{}-{}", required.os(), required.arch());
    let selected = inspect(&sibling, profile).with_context(|| format!(
        "required platform {required}: native target execution requires provisioned toolchain {sibling} (default is {actual})"
    ))?;
    if selected.platform()? != *required {
        bail!(
            "platform-specific toolchain {sibling} has platform {}, expected {required}",
            selected.platform()?
        );
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(reference: &str, arch: &str) -> Image {
        Image {
            reference: reference.into(),
            digest: format!("sha256:{}", "1".repeat(64)),
            os: "linux".into(),
            arch: arch.into(),
        }
    }

    #[test]
    fn requirements_choose_verified_siblings_without_changing_explicit_choices() {
        let arm: Platform = "linux/arm64".parse().unwrap();
        let mut seen = Vec::new();
        let selected = resolve_with(
            "example:1",
            Profile::Process,
            Some(&arm),
            false,
            |name, _| {
                seen.push(name.to_owned());
                Ok(image(
                    name,
                    if name.ends_with("-linux-arm64") {
                        "arm64"
                    } else {
                        "amd64"
                    },
                ))
            },
        )
        .unwrap();
        assert_eq!(seen, ["example:1", "example:1-linux-arm64"]);
        assert_eq!(selected.platform().unwrap(), arm);
        for required in [None, Some(&arm)] {
            let mut calls = 0;
            resolve_with("example:1", Profile::Process, required, false, |name, _| {
                calls += 1;
                Ok(image(name, "arm64"))
            })
            .unwrap();
            assert_eq!(calls, 1);
        }
        let mut calls = 0;
        assert!(resolve_with(
            "custom@sha256:fixed",
            Profile::Process,
            Some(&arm),
            true,
            |name, _| {
                calls += 1;
                Ok(image(name, "amd64"))
            }
        )
        .unwrap_err()
        .to_string()
        .contains("explicit toolchain"));
        assert_eq!(calls, 1);
    }

    #[test]
    fn missing_or_mislabeled_siblings_never_fall_back_to_wrong_execution() {
        let arm: Platform = "linux/arm64".parse().unwrap();
        assert!(resolve_with(
            "example:1",
            Profile::Process,
            Some(&arm),
            false,
            |name, _| Ok(image(name, "amd64"))
        )
        .unwrap_err()
        .to_string()
        .contains("expected linux/arm64"));
        let error = resolve_with(
            "example:1",
            Profile::Process,
            Some(&arm),
            false,
            |name, _| {
                if name.ends_with("-arm64") {
                    bail!("missing provisioned image")
                }
                Ok(image(name, "amd64"))
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("native target execution"));
        assert!(format!("{error:#}").contains("missing provisioned image"));
    }
}
