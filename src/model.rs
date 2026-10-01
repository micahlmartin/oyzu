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
    pub env: BTreeMap<String, String>,
    pub depends_on: Vec<String>,
    pub availability: Option<String>,
    pub build_stage: bool,
    pub mutates_source: bool,
    pub stdout_must_be_empty: bool,
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
    pub manager: String,
    pub path: PathBuf,
    pub version: String,
    pub tasks: BTreeMap<String, Task>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub root: PathBuf,
    pub targets: BTreeMap<String, Target>,
    pub tasks: BTreeMap<String, Task>,
}
