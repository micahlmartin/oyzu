//! Portable facts from native Go metadata; no manifest syntax interpretation.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Deserialize, Serialize)]
pub(super) struct Metadata {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub patterns: Vec<String>,
    pub modules: Vec<String>,
    pub binaries: Vec<Binary>,
    pub cgo: bool,
    pub compiler: Option<String>,
    #[serde(rename = "compilerTarget")]
    pub compiler_target: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Dependency {
    pub name: String,
    pub version: String,
    pub file: String,
    pub digest: String,
    pub size: u64,
    pub sum: String,
    #[serde(rename = "goModSum")]
    pub go_mod_sum: String,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Binary {
    pub name: String,
    pub package: String,
    pub directory: String,
}

impl Metadata {
    pub fn validate(&self) -> Result<()> {
        if self.modules.is_empty() || self.modules.len() != self.patterns.len() {
            bail!("Go metadata must identify workspace members and their package patterns");
        }
        let mut modules = BTreeSet::new();
        for (module, pattern) in self.modules.iter().zip(&self.patterns) {
            if (module != "." && !crate::snapshot::portable(module))
                || !modules.insert(module.to_lowercase())
                || *pattern
                    != if module == "." {
                        "./...".into()
                    } else {
                        format!("./{module}/...")
                    }
            {
                bail!("invalid or colliding Go workspace member {module}");
            }
        }
        let mut names = BTreeSet::new();
        for binary in &self.binaries {
            if !crate::snapshot::portable(&binary.name)
                || binary.name.contains('/')
                || (binary.directory != "." && !crate::snapshot::portable(&binary.directory))
                || binary.package.is_empty()
                || !names.insert(binary.name.to_lowercase())
            {
                bail!("invalid or colliding Go binary {}", binary.name);
            }
        }
        if self.cgo
            && (self.compiler.as_deref().is_none_or(str::is_empty)
                || self.compiler_target.as_deref().is_none_or(str::is_empty))
        {
            bail!("cgo metadata requires a compiler and target ABI");
        }
        for dependency in &self.dependencies {
            if !crate::snapshot::portable(&dependency.file)
                || !dependency.file.ends_with(".zip")
                || !dependency.sum.starts_with("h1:")
                || !dependency.go_mod_sum.starts_with("h1:")
            {
                bail!("invalid captured Go dependency {}", dependency.name);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn metadata() -> Metadata {
        serde_json::from_value(json!({"version":"go1.24.13","os":"linux","arch":"amd64",
            "modules":["cmd","math"],"patterns":["./cmd/...","./math/..."],
            "binaries":[{"name":"server","package":"example.test/cmd/server","directory":"cmd/server"},
                        {"name":"migrate","package":"example.test/cmd/migrate","directory":"cmd/migrate"}],
            "cgo":true,"compiler":"gcc 12","compilerTarget":"x86_64-linux-gnu"})).unwrap()
    }

    #[test]
    fn portable_go_facts_reject_escapes_collisions_and_incomplete_compiler_evidence() {
        metadata().validate().unwrap();
        let mut value = metadata();
        value.modules[0] = "../cmd".into();
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.patterns[0] = "./other/...".into();
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.binaries[1].name = "SERVER".into();
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.binaries[0].directory = "../outside".into();
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.compiler_target = None;
        assert!(value.validate().is_err());
    }
}
