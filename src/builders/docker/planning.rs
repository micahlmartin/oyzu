use crate::{
    builders::{
        semver_snapshot, ArtifactKind, ArtifactSpec, BuilderPlan, CommandSpec,
        CoverageApplicability, PlanningContext, ReportFormat, ReportSpec, TaskPlan,
    },
    executor::Mode,
};
use anyhow::{Context, Result};

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let dependency = context
        .dependencies
        .context("Docker planning requires captured native metadata")?;
    let data = &dependency.record["extensions"]["oyzu.dev/docker"];
    let metadata: super::metadata::Metadata = serde_json::from_value(data["metadata"].clone())?;
    let platform = &dependency.record["targetPlatform"];
    metadata.validate(&format!(
        "{}/{}",
        platform["os"].as_str().context("missing Docker OS")?,
        platform["arch"]
            .as_str()
            .context("missing Docker architecture")?
    ))?;
    let id = &context.target.name;
    let version = semver_snapshot(context.target, context.source);
    let filename = format!("{id}-{version}.oci.tar");
    let output = format!("{id}/container/image.tar");
    let mut plan = BuilderPlan::new(
        version.clone(),
        CommandSpec::new(
            "package",
            &[
                "cp",
                &format!("/out/{output}"),
                &format!("/out/{id}/artifacts/{filename}"),
            ],
        ),
    );
    let mut selected = metadata.context.files.clone();
    selected.push("Dockerfile".into());
    if let Some(ignore_file) = &metadata.context.ignore_file {
        selected.push(ignore_file.clone());
    }
    selected.sort();
    selected.dedup();
    plan.source_files = Some(selected);
    plan.coverage = Some(CoverageApplicability::Inapplicable {
        reason: "OCI packaging assertions do not measure application-source coverage; packaged producer coverage remains separate evidence.".into(),
    });
    let mut build = TaskPlan::command(&["buildctl", "build"]);
    build.execution = Mode::Buildkit {
        output: output.clone(),
        image_name: format!("oyzu/{}:{version}", id.to_ascii_lowercase()),
        context_files: metadata.context.files,
        apparmor_profile: data["apparmorProfile"]
            .as_str()
            .context("missing BuildKit security profile")?
            .into(),
        dockerfile_digest: data["dockerfileDigest"]
            .as_str()
            .context("missing captured Dockerfile identity")?
            .into(),
    };
    plan.tasks.insert("build".into(), build);
    let mut test = TaskPlan::command(&[]);
    test.execution = Mode::OciValidation {
        input: output,
        report: format!("{id}/reports/junit.xml"),
    };
    test.reports.push(ReportSpec {
        format: ReportFormat::Junit,
        filename: "junit.xml",
        source: crate::reports::ReportSource::File,
        name: None,
        input: None,
    });
    plan.tasks.insert("test".into(), test);
    plan.artifacts.push(ArtifactSpec {
        kind: ArtifactKind::OciImage,
        name: "image".into(),
        filename,
        media_type: "application/vnd.oci.image.layout.v1+tar",
        version: None,
    });
    Ok(plan)
}
