use crate::{
    dependencies::Prepared,
    executor::{Image, Mode, Profile},
    model::{Target, Task},
    snapshot::Snapshot,
};
use anyhow::{bail, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(crate) struct Descriptor {
    pub ids: &'static [&'static str],
}

/// Discovery and planning read captured source only. Acquisition must use the scoped
/// broker and executor; project code never receives host credentials or network access.
pub(crate) trait Builder: Sync {
    fn descriptor(&self) -> Descriptor;
    fn detect(&self, path: &Path) -> Option<&'static str>;
    fn discover(&self, target: &mut Target) -> Result<()>;

    fn executor_profile(&self) -> Profile {
        Profile::Process
    }

    fn toolchain(&self, target: &Target) -> Result<&'static str> {
        bail!(
            "{}: {} build integration is not implemented yet",
            target.name,
            target.manager
        )
    }

    fn prepare(&self, _context: PreparationContext<'_>) -> Result<Option<Prepared>> {
        Ok(None)
    }

    fn plan(&self, context: PlanningContext<'_>) -> Result<BuilderPlan> {
        bail!(
            "{}: {} build integration is not implemented yet",
            context.target.name,
            context.target.manager
        )
    }

    fn runtime_files(&self) -> &'static [RuntimeFile] {
        &[]
    }

    /// Add native reporting to an exactly recognized replacement command. Unknown
    /// bodies stay unchanged and must satisfy the operation's report contract.
    /// This cannot replace or remove required reports, hooks or sandbox constraints.
    fn instrument_override(
        &self,
        _target: &Target,
        _task: &Task,
        _env: &BTreeMap<String, String>,
    ) -> Option<Vec<String>> {
        None
    }
}

pub(crate) struct PreparationContext<'a> {
    pub target: &'a Target,
    pub destination: &'a Path,
    pub image: &'a Image,
    pub source_digest: &'a str,
    pub execution_name: &'a str,
}

pub(crate) struct PlanningContext<'a> {
    pub target: &'a Target,
    pub source: &'a Snapshot,
    pub dependencies: Option<&'a Prepared>,
}

pub(crate) struct RuntimeFile {
    pub name: &'static str,
    pub contents: &'static str,
}

/// Ecosystem intent. The engine expands hooks, adds constraints and serializes
/// the wire contract; adapters cannot change scheduling or sandbox enforcement.
pub(crate) struct BuilderPlan {
    pub version: String,
    pub env: BTreeMap<String, String>,
    pub prepare: Vec<CommandSpec>,
    pub stages: Vec<&'static str>,
    pub tasks: BTreeMap<String, TaskPlan>,
    pub package: CommandSpec,
    pub artifacts: Vec<ArtifactSpec>,
    /// Optional files relative to the target, including required control files.
    /// The engine scopes and applies this selection before materialization.
    pub source_files: Option<Vec<String>>,
}

impl BuilderPlan {
    pub fn validate(&self) -> Result<()> {
        if let Some(files) = &self.source_files {
            crate::snapshot::Projection::new(".", files)?;
        }
        self.package.execution.validate()?;
        for command in &self.prepare {
            command.execution.validate()?;
        }
        for task in self.tasks.values() {
            task.execution.validate()?;
        }
        let mut names = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for artifact in &self.artifacts {
            if !crate::names::valid(&artifact.name)
                || !names.insert(artifact.name.to_ascii_lowercase())
            {
                bail!("invalid or colliding artifact identifier {}", artifact.name);
            }
            let file = &artifact.filename;
            if file.is_empty()
                || matches!(file.as_str(), "." | "..")
                || file.chars().any(|c| {
                    c.is_control()
                        || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
                })
                || file.ends_with(['.', ' '])
                || !paths.insert(file.to_lowercase())
            {
                bail!("invalid or colliding artifact filename {file}");
            }
        }
        Ok(())
    }

    pub fn new(version: String, package: CommandSpec) -> Self {
        Self {
            env: BTreeMap::from([
                ("HOME".into(), "/tmp/oyzu-home".into()),
                ("CI".into(), "true".into()),
                ("TZ".into(), "UTC".into()),
                ("OYZU_VERSION".into(), version.clone()),
            ]),
            version,
            prepare: vec![],
            stages: vec!["build", "test", "lint", "format-check", "format:check"],
            tasks: BTreeMap::new(),
            package,
            artifacts: vec![],
            source_files: None,
        }
    }
}

pub(crate) struct CommandSpec {
    pub operation: &'static str,
    pub argv: Vec<String>,
    pub execution: Mode,
}

impl CommandSpec {
    pub fn new(operation: &'static str, argv: &[&str]) -> Self {
        Self {
            operation,
            argv: strings(argv),
            execution: Mode::Process,
        }
    }
}

pub(crate) struct ArtifactSpec {
    pub kind: ArtifactKind,
    pub name: String,
    pub filename: String,
    pub media_type: &'static str,
    /// Native workspaces can contain packages with independent versions.
    pub version: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ArtifactKind {
    File,
    OciImage,
}

#[derive(Default)]
pub(crate) struct TaskPlan {
    pub execution: Mode,
    pub argv: Vec<String>,
    pub reports: Vec<ReportSpec>,
}

impl TaskPlan {
    pub fn command(argv: &[&str]) -> Self {
        Self {
            argv: strings(argv),
            ..Self::default()
        }
    }
}

pub(crate) use crate::reports::Format as ReportFormat;

pub(crate) struct ReportSpec {
    pub format: ReportFormat,
    pub filename: &'static str,
    pub source: crate::reports::ReportSource,
    /// Optional module identity and native report location relative to task cwd.
    pub name: Option<String>,
    pub input: Option<String>,
}

pub(crate) fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}

pub(crate) fn semver_snapshot(target: &Target, source: &Snapshot) -> String {
    format!(
        "{}-dev.g{}",
        target.version.split(['-', '+']).next().unwrap_or("0.0.0"),
        &source.digest[7..19]
    )
}
