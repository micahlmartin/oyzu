use crate::builders::{
    ArtifactSpec, BuilderPlan, CommandSpec, CoverageApplicability, PlanningContext, ReportFormat,
    ReportSpec, TaskPlan,
};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

pub(super) fn environment() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("HOME".into(), "/tmp/oyzu-home".into()),
        ("HELM_CACHE_HOME".into(), "/tmp/helm/cache".into()),
        ("HELM_CONFIG_HOME".into(), "/tmp/helm/config".into()),
        ("HELM_DATA_HOME".into(), "/tmp/helm/data".into()),
        ("HELM_PLUGINS".into(), "/opt/oyzu-helm-plugins".into()),
        ("KUBECONFIG".into(), "/dev/null".into()),
    ])
}

pub(super) fn plan(context: PlanningContext<'_>) -> Result<BuilderPlan> {
    let prepared = context
        .dependencies
        .context("Helm planning requires captured dependencies")?;
    let chart = super::metadata::read(&prepared.root.join("chart"))?;
    let id = &context.target.name;
    let filename = format!("{}-{}.tgz", chart.name, chart.version);
    let library = chart.kind == "library";
    let package = if library {
        CommandSpec::new(
            "package",
            &[
                "cp",
                "--",
                &format!(".oyzu-build/package/{filename}"),
                &format!("/out/{id}/artifacts/{filename}"),
            ],
        )
    } else {
        CommandSpec::new(
            "package",
            &[
                "sh",
                "-c",
                "cp -- \"$1\" \"$2\" && cp -- .oyzu-build/rendered.yaml \"$3\"",
                "oyzu-package",
                &format!(".oyzu-build/package/{filename}"),
                &format!("/out/{id}/artifacts/{filename}"),
                &format!("/out/{id}/artifacts/rendered.yaml"),
            ],
        )
    };
    let mut plan = BuilderPlan::new(chart.version, package);
    plan.coverage = Some(CoverageApplicability::Inapplicable {
        reason: "Chart packaging has no application-source coverage denominator; native chart assertions do not measure application code.".into(),
    });
    plan.env.extend(environment());
    plan.fixed_env
        .insert("HELM_PLUGINS".into(), plan.env["HELM_PLUGINS"].clone());
    plan.prepare.push(CommandSpec::new(
        "prepare",
        &[
            "sh",
            "-c",
            "mkdir -p .oyzu-build/chart && cp -R /dependencies/chart/. .oyzu-build/chart/",
        ],
    ));
    plan.tasks.insert("build".into(), TaskPlan::command(&[
        "sh", "-c", "helm package .oyzu-build/chart --destination .oyzu-build/package && python -I /oyzu/helm-archive.py \"$1\"", "oyzu-helm-package",
        &format!(".oyzu-build/package/{filename}"),
    ]));
    let mut test = TaskPlan::command(&[
        "python",
        "-I",
        "/oyzu/helm-test.py",
        ".oyzu-build/chart",
        if library { "library" } else { "application" },
        &format!("/out/{id}/reports/junit.xml"),
        ".oyzu-build/rendered.yaml",
    ]);
    test.reports.push(ReportSpec {
        format: ReportFormat::Junit,
        filename: "junit.xml",
        source: crate::reports::ReportSource::File,
        name: None,
        input: None,
    });
    if context.target.discovery["test-framework"].selected() == "helm-unittest" {
        test.argv.extend([
            "--unittest-report".into(),
            format!("/out/{id}/reports/unittest/unittest.xml"),
        ]);
        test.reports.push(ReportSpec {
            format: ReportFormat::Junit,
            filename: "unittest.xml",
            source: crate::reports::ReportSource::File,
            name: Some("unittest".into()),
            input: None,
        });
    }
    plan.tasks.insert("test".into(), test);
    plan.tasks.insert(
        "lint".into(),
        TaskPlan::command(&[
            "helm",
            "lint",
            ".oyzu-build/chart",
            "--strict",
            "--with-subcharts",
        ]),
    );
    plan.artifacts = vec![ArtifactSpec {
        kind: crate::builders::ArtifactKind::File,
        name: "chart".into(),
        filename,
        media_type: "application/gzip",
        version: None,
    }];
    if !library {
        plan.artifacts.push(ArtifactSpec {
            kind: crate::builders::ArtifactKind::File,
            name: "rendered".into(),
            filename: "rendered.yaml".into(),
            media_type: "application/yaml",
            version: None,
        });
    }
    Ok(plan)
}
