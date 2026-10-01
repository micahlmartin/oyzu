use super::super::{
    semver_snapshot, ArtifactSpec, BuilderPlan, CommandSpec, PlanningContext, ReportFormat,
    ReportSpec, TaskPlan,
};
use anyhow::{bail, Result};
use std::collections::BTreeMap;

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let target = context.target;
    let id = &target.name;
    if target.path.join("go.work").exists() {
        bail!("{id}: Go workspace packaging is not implemented yet");
    }
    if target.builder != "go/app" {
        bail!("{id}: Go library artifact packaging is not implemented yet");
    }
    let version = semver_snapshot(target, context.source);
    let filename = format!("{id}-{version}");
    let mut plan = BuilderPlan::new(
        version,
        CommandSpec::new(
            "package",
            &[
                "cp",
                ".oyzu-build/app",
                &format!("/out/{id}/artifacts/{filename}"),
            ],
        ),
    );
    plan.env.extend(BTreeMap::from([
        ("GOTOOLCHAIN".into(), "local".into()),
        ("GOPROXY".into(), "off".into()),
        ("GOSUMDB".into(), "off".into()),
        ("CGO_ENABLED".into(), "0".into()),
        ("GOFLAGS".into(), "-p=2".into()),
        ("GOMAXPROCS".into(), "2".into()),
        ("GOCACHE".into(), "/workspace/.oyzu-build/go-cache".into()),
    ]));
    plan.prepare
        .push(CommandSpec::new("prepare", &["mkdir", "-p", ".oyzu-build"]));
    plan.tasks.insert(
        "build".into(),
        TaskPlan::command(&["go", "build", "-trimpath", "-o", ".oyzu-build/app", "."]),
    );
    let mut test = TaskPlan::command(&[
        "go",
        "test",
        "-json",
        &format!("-coverprofile=/out/{id}/reports/coverage.out"),
        "./...",
    ]);
    test.reports = vec![
        ReportSpec {
            format: ReportFormat::Junit,
            filename: "junit.xml",
            source: crate::reports::ReportSource::GoTestEvents,
        },
        ReportSpec {
            format: ReportFormat::GoCover,
            filename: "coverage.out",
            source: crate::reports::ReportSource::File,
        },
    ];
    plan.tasks.insert("test".into(), test);
    plan.artifacts.push(ArtifactSpec {
        name: "primary",
        filename,
        media_type: "application/octet-stream",
    });
    Ok(plan)
}
