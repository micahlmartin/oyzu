//! Finite application-container inventory options, not cascading TOML settings.
//! Capture validates these values once; packaging consumes the frozen inventory.
use anyhow::{bail, Result};
use serde::Deserialize;

#[derive(Clone, Debug)]
pub enum Container {
    Enabled(bool),
    Options(ContainerOptions),
}

impl<'de> Deserialize<'de> for Container {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        match serde_yaml::Value::deserialize(deserializer)? {
            serde_yaml::Value::Bool(enabled) => Ok(Self::Enabled(enabled)),
            value @ serde_yaml::Value::Mapping(_) => serde_yaml::from_value(value)
                .map(Self::Options)
                .map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(
                "container requires a boolean or option mapping",
            )),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerOptions {
    #[serde(default, deserialize_with = "defined")]
    pub base: Option<String>,
    #[serde(default, deserialize_with = "defined")]
    pub user: Option<String>,
    #[serde(default, deserialize_with = "defined")]
    pub workdir: Option<String>,
    #[serde(default, deserialize_with = "defined")]
    pub entrypoint: Option<Vec<String>>,
}

// Missing options use defaults; an explicit null is not a valid override.
fn defined<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

impl Container {
    pub(crate) fn enabled(&self) -> bool {
        !matches!(self, Self::Enabled(false))
    }

    pub(crate) fn options(&self) -> Option<ContainerOptions> {
        match self {
            Self::Enabled(false) => None,
            Self::Enabled(true) => Some(ContainerOptions::default()),
            Self::Options(options) => Some(options.clone()),
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if let Self::Options(options) = self {
            options.validate()?;
        }
        Ok(())
    }
}

impl ContainerOptions {
    /// Numeric image identity; named accounts require unavailable runtime lookup.
    pub(crate) fn identity(&self) -> Result<Option<(u32, u32)>> {
        let Some(user) = &self.user else {
            return Ok(None);
        };
        let (uid, gid) = user.split_once(':').ok_or_else(|| {
            anyhow::anyhow!("CONFIG_INVALID_VALUE: container.user requires numeric UID:GID")
        })?;
        let numeric = |value: &str| -> Result<u32> {
            if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                bail!("CONFIG_INVALID_VALUE: container.user requires numeric UID:GID");
            }
            value.parse().map_err(|_| {
                anyhow::anyhow!(
                    "CONFIG_INVALID_VALUE: container.user exceeds numeric identity range"
                )
            })
        };
        Ok(Some((numeric(uid)?, numeric(gid)?)))
    }

    fn validate(&self) -> Result<()> {
        self.identity()?;
        if self
            .base
            .as_ref()
            .is_some_and(|value| !crate::oci::literal_reference(value))
        {
            bail!("CONFIG_INVALID_VALUE: container.base requires a bounded provisioned image reference");
        }
        if let Some(path) = &self.workdir {
            let relative = path.strip_prefix('/').unwrap_or("").trim_end_matches('/');
            if path.len() > 1024
                || (path != "/"
                    && (!crate::snapshot::portable(relative)
                        || !relative
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))))
            {
                bail!("CONFIG_INVALID_VALUE: container.workdir requires an absolute literal image path");
            }
        }
        if let Some(argv) = &self.entrypoint {
            if argv.is_empty()
                || argv.len() > 64
                || argv.iter().any(|arg| {
                    arg.is_empty() || arg.len() > 4096 || arg.chars().any(char::is_control)
                })
            {
                bail!("CONFIG_INVALID_VALUE: container.entrypoint requires bounded nonempty literal arguments");
            }
        }
        Ok(())
    }
}
