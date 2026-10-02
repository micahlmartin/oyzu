use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub target: String,
    pub provider: String,
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    #[serde(serialize_with = "redact_environment")]
    pub env: BTreeMap<String, String>,
    pub depends_on: Vec<String>,
    pub availability: Option<String>,
    pub build_stage: bool,
    pub mutates_source: bool,
    pub stdout_must_be_empty: bool,
    #[serde(default)]
    pub reports: Vec<crate::reports::Declaration>,
}

impl Task {
    pub fn id(&self) -> String {
        if self.target.is_empty() {
            self.name.clone()
        } else {
            format!("{}:{}", self.target, self.name)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Target {
    pub name: String,
    pub builder: String,
    #[serde(default)]
    pub builder_selection: BuilderSelection,
    pub manager: String,
    pub path: PathBuf,
    pub version: String,
    pub tasks: BTreeMap<String, Task>,
    #[serde(default)]
    pub discovery: BTreeMap<String, crate::discovery::Resolution>,
}

/// Whether a target's builder is selected by project intent or inferred from
/// native files. Adapters may use explicit intent to resolve output ambiguity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuilderSelection {
    #[default]
    Inferred,
    Explicit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub root: PathBuf,
    pub targets: BTreeMap<String, Target>,
    pub tasks: BTreeMap<String, Task>,
    #[serde(skip)]
    pub configuration: BTreeMap<String, crate::config::resolve::EffectiveConfig>,
    #[serde(skip)]
    pub root_configuration: Option<crate::config::resolve::EffectiveConfig>,
}

fn redact_environment<S: serde::Serializer>(
    env: &BTreeMap<String, String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let redacted: BTreeMap<_, _> = env.keys().map(|key| (key, "[REDACTED]")).collect();
    redacted.serialize(serializer)
}
