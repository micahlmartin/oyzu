use crate::{
    dependencies::Prepared,
    executor::{Image, Mode, Profile},
    model::{Target, Task},
    platform::Platform,
    snapshot::Snapshot,
};
use anyhow::{bail, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub(crate) struct Descriptor {
    pub ids: &'static [&'static str],
    pub tools: &'static [&'static str],
}

/// Discovery and captured-build planning read supplied source only. Captured
/// acquisition uses the scoped broker/executor; development hooks explicitly
/// operate with host tools and do not claim isolation from the host environment.
pub(crate) trait Builder: Sync {
    fn descriptor(&self) -> Descriptor;
    fn register_settings(&self, _registry: &mut crate::config::registry::Registry) -> Result<()> {
        Ok(())
    }
    fn detect(&self, path: &Path) -> Option<&'static str>;
    fn discover(&self, target: &mut Target) -> Result<()>;

    /// Whether public broker acquisition is unavoidable for this builder.
    /// False admits offline preparation only; conditional package providers
    /// must enforce configuration again before any broker access. It does not
    /// grant network authority (see dependencies::context).
    fn acquisition_requires_network(&self) -> bool {
        true
    }

    /// Resolve native arguments/environment only after an explicit development task run.
    /// Static discovery and captured build planning must never call this hook.
    fn development_command(&self, _task: &Task) -> Result<Option<DevelopmentCommand>> {
        Ok(None)
    }

    /// Report contract for explicit host test execution. None means this native
    /// profile still lacks direct-run evidence integration. No acquisition or
    /// sandbox claim is implied; commands use already provisioned host tools.
    /// Commands may reference `/oyzu/<name>` from runtime_files(); host execution
    /// stages those owned assets privately and translates the declared paths.
    fn development_test(&self, _target: &Target, _task: &Task) -> Result<Option<TaskPlan>> {
        Ok(None)
    }

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

    /// Offline stores this ecosystem can prepare for another builder. Providers
    /// own native resolution/layout; shared composition owns policy admission.
    fn dependency_providers(
        &self,
    ) -> &'static [&'static dyn crate::dependencies::context::Provider] {
        &[]
    }

    /// Admit an artifact target before preparation. The default requires native
    /// execution; adapters with a real cross-target packaging capability opt in.
    /// Admission does not establish that target application tests executed.
    fn target_platform(&self, requested: Option<&str>, image: &Image) -> Result<Platform> {
        let execution = image.platform()?;
        let target = Platform::requested(requested, &execution)?;
        if target != execution {
            bail!("required platform {target} differs from execution platform {execution}; native target execution is required by this builder");
        }
        Ok(target)
    }

    /// Required toolchain execution platform for an explicit artifact target.
    /// Packaging adapters may use another worker and return None; target
    /// admission and any target-code execution checks remain mandatory.
    fn execution_platform(&self, requested: Option<&str>) -> Result<Option<Platform>> {
        requested.map(str::parse).transpose()
    }

    /// Select an already provisioned image for concrete runtime axes. Adapters
    /// admitting variants must verify the actual runtime during preparation;
    /// an image reference alone is not runtime-version evidence.
    /// Execution platform selection may resolve a provisioned platform sibling;
    /// target_platform separately admits the artifact against that execution.
    fn variant_toolchain(&self, target: &Target) -> Result<String> {
        if target.variant.keys().any(|axis| axis != "platform") {
            bail!(
                "{}: runtime matrix integration is not implemented for {}",
                target.name,
                target.manager
            );
        }
        Ok(self.toolchain(target)?.into())
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

    /// Runtime and exact already-packaged output for optional container assembly.
    /// Returning None means this native profile is not supported. No I/O or
    /// configuration resolution is permitted; preparation supplies native facts.
    fn container_profile(
        &self,
        _target: &Target,
        _prepared: Option<&Prepared>,
    ) -> Result<Option<super::ContainerProfile>> {
        Ok(None)
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

/// Native invocation context resolved only for explicit development execution.
pub(crate) struct DevelopmentCommand {
    pub argv: Vec<String>,
    pub env: BTreeMap<String, String>,
}

pub(crate) struct PreparationContext<'a> {
    /// The admitted owner's immutable snapshot. Adapters may consume registered
    /// values but must never resolve sources or recompute defaults here.
    pub configuration: &'a crate::config::resolve::EffectiveConfig,
    /// Optional provider selector from the frozen target inventory, not TOML.
    pub dependency_selector: Option<&'a str>,
    pub target: &'a Target,
    pub destination: &'a Path,
    pub image: &'a Image,
    pub target_platform: &'a Platform,
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
    /// If preparation established a target, compilation must verify it against
    /// the admitted request. This is evidence, not permission to change targets.
    pub target_platform: Option<Platform>,
    /// Captured native tool execution identity, when established by preparation.
    pub execution_platform: Option<Platform>,
    pub version: String,
    pub env: BTreeMap<String, String>,
    /// Captured toolchain facts that task overrides cannot silently change.
    pub fixed_env: BTreeMap<String, String>,
    pub prepare: Vec<CommandSpec>,
    pub stages: Vec<&'static str>,
    pub tasks: BTreeMap<String, TaskPlan>,
    pub package: CommandSpec,
    pub artifacts: Vec<ArtifactSpec>,
    /// Explicit application-source coverage facts, independent of report files.
    pub coverage: Option<CoverageApplicability>,
    /// Optional files relative to the target, including required control files.
    /// The engine scopes and applies this selection before materialization.
    pub source_files: Option<Vec<String>>,
}

impl BuilderPlan {
    pub fn validate(&self) -> Result<()> {
        if let Some(CoverageApplicability::Inapplicable { reason }) = &self.coverage {
            if reason.trim().is_empty() {
                bail!("coverage inapplicability requires a reason");
            }
        }
        for (name, value) in &self.fixed_env {
            if self.env.get(name) != Some(value) {
                bail!("builder environment does not supply captured fact {name}");
            }
        }
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
            target_platform: None,
            execution_platform: None,
            env: BTreeMap::from([
                ("HOME".into(), "/tmp/oyzu-home".into()),
                ("CI".into(), "true".into()),
                ("TZ".into(), "UTC".into()),
                ("OYZU_VERSION".into(), version.clone()),
            ]),
            version,
            fixed_env: BTreeMap::new(),
            prepare: vec![],
            stages: vec!["build", "test", "lint", "format-check", "format:check"],
            tasks: BTreeMap::new(),
            package,
            artifacts: vec![],
            coverage: None,
            source_files: None,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub(crate) enum CoverageApplicability {
    Inapplicable { reason: String },
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
    Directory,
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
    semver_snapshot_digest(target, &source.digest)
}

pub(crate) fn semver_snapshot_digest(target: &Target, digest: &str) -> String {
    format!(
        "{}-dev.g{}",
        target.version.split(['-', '+']).next().unwrap_or("0.0.0"),
        &digest[7..19]
    )
}
