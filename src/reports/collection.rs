//! Capture native report evidence before parsing it. No ecosystem dispatch or execution.
use crate::bundle_store::{capture_bounded_output, safe_report_parent};
use crate::{reports, snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub(crate) struct Locations<'a> {
    pub workspace: &'a Path,
    pub output: &'a Path,
    pub bundle: &'a Path,
}

struct Pending<'a> {
    action: &'a Value,
    prepared: Vec<Result<()>>,
    index: usize,
}

pub(crate) struct Collector<'a> {
    pub locations: Locations<'a>,
    pub source_digest: &'a Value,
    pending: BTreeMap<String, Vec<Pending<'a>>>,
}

impl<'a> Collector<'a> {
    pub fn new(locations: Locations<'a>, source_digest: &'a Value) -> Self {
        Self {
            locations,
            source_digest,
            pending: BTreeMap::new(),
        }
    }
    pub fn defer(&mut self, action: &'a Value, stdout: PathBuf, index: usize) -> Result<()> {
        let intents = action["reports"]
            .as_array()
            .context("missing report intents")?;
        if intents.is_empty() {
            return Ok(());
        }
        let boundary = action["extensions"]["oyzu.dev/collect-after"]
            .as_str()
            .or_else(|| action["id"].as_str())
            .context("missing collection boundary")?;
        self.pending
            .entry(boundary.into())
            .or_default()
            .push(Pending {
                action,
                // Normalize native event streams before hooks can consume the
                // report. Retain failures; a hook cannot repair invalid events
                // by replacing them with an unrelated successful report.
                prepared: intents
                    .iter()
                    .map(|intent| prepare(action, intent, &self.locations, &stdout))
                    .collect(),
                index,
            });
        Ok(())
    }
    pub fn finish(&mut self, boundary: &str) -> Result<Vec<(usize, Vec<CollectedReport>)>> {
        let mut completed = Vec::new();
        for pending in self.pending.remove(boundary).unwrap_or_default() {
            let mut collected = Vec::new();
            for (intent, prepared) in pending.action["reports"]
                .as_array()
                .context("missing report intents")?
                .iter()
                .zip(pending.prepared)
            {
                collected.extend(collect(
                    pending.action,
                    intent,
                    self.source_digest,
                    &self.locations,
                    prepared,
                )?);
            }
            completed.push((pending.index, collected));
        }
        Ok(completed)
    }
    pub fn ensure_finished(&self) -> Result<()> {
        if !self.pending.is_empty() {
            bail!("unreached report collection boundary");
        }
        Ok(())
    }
}

pub(crate) struct CollectedReport {
    pub record: Value,
    pub diagnostic: Option<Value>,
}

impl CollectedReport {
    pub fn failed(&self) -> bool {
        self.diagnostic.is_some()
            || self.record["summary"]["failed"]
                .as_u64()
                .is_some_and(|count| count > 0)
    }
}

fn report_input(action: &Value, id: &str, path: &str) -> Result<reports::Input> {
    Ok(action["extensions"]["oyzu.dev/report-inputs"]
        .get(id)
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()?
        .unwrap_or(reports::Input {
            root: reports::Root::Output,
            path: path.into(),
        }))
}

fn prepare(action: &Value, intent: &Value, locations: &Locations<'_>, stdout: &Path) -> Result<()> {
    let id = intent["id"].as_str().context("missing report id")?;
    let source: reports::ReportSource = action["extensions"]["oyzu.dev/report-sources"]
        .get(id)
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()?
        .unwrap_or_default();
    if source == reports::ReportSource::File {
        return Ok(());
    }
    let path = action["extensions"]["oyzu.dev/report-paths"][id]
        .as_str()
        .context("missing report path")?;
    let input = report_input(action, id, path)?;
    if !matches!(input.root, reports::Root::Output) || input.path != path {
        bail!("native events require their assigned output destination");
    }
    safe_report_parent(locations.output, path)?;
    reports::materialize(source, stdout, &locations.output.join(path))
}

fn collect(
    action: &Value,
    intent: &Value,
    source_digest: &Value,
    locations: &Locations<'_>,
    prepared: Result<()>,
) -> Result<Vec<CollectedReport>> {
    let id = intent["id"].as_str().context("missing report id")?;
    let path = action["extensions"]["oyzu.dev/report-paths"][id]
        .as_str()
        .context("missing report path")?;
    let format = intent["format"].as_str().context("missing report format")?;
    let record = json!({"id":id,"action":action["id"],"target":action["target"],
        "kind":intent["kind"],"format":format,"status":"invalid","summary":{}});
    let selection = (|| -> Result<(reports::Input, Vec<String>)> {
        prepared?;
        let input = report_input(action, id, path)?;
        let root = match input.root {
            reports::Root::Output => locations.output,
            reports::Root::Workspace => locations.workspace,
        };
        let files = reports::matches(root, &input.path)?;
        Ok((input, files))
    })();
    let (input, files) = match selection {
        Ok(value) => value,
        Err(error) => return Ok(vec![parsed_report(action, record, Err(error))]),
    };
    let root = match input.root {
        reports::Root::Output => locations.output,
        reports::Root::Workspace => locations.workspace,
    };
    let glob = input.path.contains(['*', '?']);
    let mut collected = Vec::new();
    let mut remaining = 64 * 1024 * 1024u64;
    for file in files {
        let mut record = record.clone();
        let destination = if glob {
            let suffix = crate::names::scoped("file", &file);
            record["id"] = json!(format!("{id}/{suffix}"));
            let format: reports::Format = serde_json::from_value(json!(format))?;
            format!("{path}/{suffix}.{}", format.extension())
        } else {
            path.into()
        };
        let parsed = (|| -> Result<Value> {
            let captured = capture_bounded_output(
                root,
                &file,
                locations.bundle,
                &destination,
                reports::MAX_REPORT_BYTES.min(remaining),
            )?;
            remaining -= std::fs::metadata(&captured)?.len();
            record["path"] = json!(destination);
            record["digest"] = json!(snapshot::file_digest(&captured)?);
            record["subjectDigest"] = source_digest.clone();
            // Summary and recorded digest must describe the same retained bytes.
            if format == "junit" {
                reports::junit_summary(&captured)
            } else {
                reports::coverage_summary(&captured, format)
            }
        })();
        collected.push(parsed_report(action, record, parsed));
    }
    Ok(collected)
}

fn parsed_report(action: &Value, mut record: Value, parsed: Result<Value>) -> CollectedReport {
    let diagnostic = match parsed {
        Ok(summary) => {
            record["status"] = json!("collected");
            let minimum = action["extensions"]["oyzu.dev/coverage-minimum"]
                .as_u64()
                .unwrap_or(0);
            let insufficient = record["kind"] == "coverage"
                && minimum > 0
                && summary["total"].as_u64().is_none_or(|total| {
                    total == 0
                        || summary["covered"].as_u64().is_none_or(|covered| {
                            (covered as u128) * 100 < (total as u128) * (minimum as u128)
                        })
                });
            record["summary"] = summary;
            insufficient.then(|| json!({"code":"CONFIG_OVERRIDE_DENIED","phase":"collect","severity":"error","message":"coverage is below the configured minimum","action":action["id"],"target":action["target"]}))
        }
        Err(error) => Some(
            json!({"code":"report-invalid","phase":"collect","severity":"error",
            "message":error.to_string(),"action":action["id"],"target":action["target"]}),
        ),
    };
    CollectedReport { record, diagnostic }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn collect_file(out: &Path, bundle: &Path, path: &str) -> CollectedReport {
        collect(
            &json!({"id":"app:test","target":"app","extensions":{
                "oyzu.dev/report-paths":{"tests":path}}}),
            &json!({"id":"tests","kind":"test","format":"junit"}),
            &json!("sha256:source"),
            &Locations {
                workspace: out,
                output: out,
                bundle,
            },
            Ok(()),
        )
        .unwrap()
        .remove(0)
    }

    #[test]
    fn invalid_reports_retain_exact_bytes_without_becoming_successes() {
        let out = tempfile::tempdir().unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let bytes = b"<testsuite>truncated\xff";
        fs::write(out.path().join("junit.xml"), bytes).unwrap();
        let report = collect_file(out.path(), bundle.path(), "junit.xml");
        assert!(report.failed());
        assert_eq!(report.record["status"], "invalid");
        assert_eq!(report.record["summary"], json!({}));
        assert_eq!(fs::read(bundle.path().join("junit.xml")).unwrap(), bytes);
        assert_eq!(
            report.record["digest"],
            snapshot::file_digest(&bundle.path().join("junit.xml")).unwrap()
        );
        assert_eq!(report.diagnostic.unwrap()["code"], "report-invalid");
    }

    #[test]
    fn valid_failed_tests_keep_summary_and_fail_the_action() {
        let out = tempfile::tempdir().unwrap();
        let bundle = tempfile::tempdir().unwrap();
        fs::write(
            out.path().join("junit.xml"),
            "<testsuite><testcase><failure/></testcase><testcase/></testsuite>",
        )
        .unwrap();
        let report = collect_file(out.path(), bundle.path(), "junit.xml");
        assert!(report.failed());
        assert!(report.diagnostic.is_none());
        assert_eq!(report.record["status"], "collected");
        assert_eq!(report.record["summary"]["failed"], 1);
        assert_eq!(report.record["summary"]["passed"], 1);
        fs::write(out.path().join("junit.xml"), "changed").unwrap();
        assert_eq!(
            report.record["digest"],
            snapshot::file_digest(&bundle.path().join("junit.xml")).unwrap()
        );
    }

    #[test]
    fn missing_oversized_and_escaping_reports_are_not_captured() {
        let out = tempfile::tempdir().unwrap();
        let bundle = tempfile::tempdir().unwrap();
        fs::File::create(out.path().join("large.xml"))
            .unwrap()
            .set_len(reports::MAX_REPORT_BYTES + 1)
            .unwrap();
        for path in ["missing.xml", "large.xml", "../escape.xml"] {
            let report = collect_file(out.path(), bundle.path(), path);
            assert!(report.failed());
            assert!(report.record.get("path").is_none());
            assert!(report.record.get("digest").is_none());
        }
        assert_eq!(fs::read_dir(bundle.path()).unwrap().count(), 0);
    }

    #[test]
    fn deferred_collection_reads_post_hook_outputs_and_keeps_malformed_glob_members() {
        let work = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let source = json!("sha256:source");
        let action = json!({"id":"app:test","target":"app","reports":[{"id":"tests","kind":"test","format":"junit"}],"extensions":{
            "oyzu.dev/collect-after":"app:post_test",
            "oyzu.dev/report-paths":{"tests":"app/reports/custom"},
            "oyzu.dev/report-inputs":{"tests":{"root":"workspace","path":"api/reports/**/*.xml"}}
        }});
        let mut collector = Collector::new(
            Locations {
                workspace: work.path(),
                output: out.path(),
                bundle: bundle.path(),
            },
            &source,
        );
        collector
            .defer(&action, out.path().join("stdout"), 7)
            .unwrap();
        assert!(collector.finish("app:test").unwrap().is_empty());
        assert!(collector.ensure_finished().is_err());
        fs::create_dir_all(work.path().join("api/reports/nested")).unwrap();
        fs::write(
            work.path().join("api/reports/a.xml"),
            "<testsuite><testcase/></testsuite>",
        )
        .unwrap();
        fs::write(
            work.path().join("api/reports/nested/b.xml"),
            "broken report",
        )
        .unwrap();
        fs::write(work.path().join("api/reports/ignored.txt"), "unrelated").unwrap();
        let mut completed = collector.finish("app:post_test").unwrap();
        assert_eq!(completed.len(), 1);
        let (index, reports) = completed.remove(0);
        assert_eq!(index, 7);
        assert_eq!(reports.len(), 2);
        assert!(!reports[0].failed());
        assert!(reports[1].failed());
        assert_eq!(reports[0].record["summary"]["passed"], 1);
        assert_eq!(
            fs::read_to_string(
                bundle
                    .path()
                    .join(reports[1].record["path"].as_str().unwrap())
            )
            .unwrap(),
            "broken report"
        );
        assert_ne!(reports[0].record["id"], reports[1].record["id"]);
        collector.ensure_finished().unwrap();
    }

    #[test]
    fn native_reports_exist_before_hooks_and_are_not_regenerated_after_them() {
        for failure in [None, Some("invalid-events"), Some("existing-report")] {
            let out = tempfile::tempdir().unwrap();
            let bundle = tempfile::tempdir().unwrap();
            let source = json!("sha256:source");
            let action = json!({"id":"app:test","target":"app","reports":[{"id":"tests","kind":"test","format":"junit"}],"extensions":{
                "oyzu.dev/collect-after":"app:post_test",
                "oyzu.dev/report-paths":{"tests":"junit.xml"},
                "oyzu.dev/report-sources":{"tests":"go-test-events"}
            }});
            let stdout = out.path().join("stdout");
            fs::write(
                &stdout,
                if failure == Some("invalid-events") {
                    "not JSON"
                } else {
                    r#"{"Package":"example/app","Test":"TestGreeting","Action":"pass"}"#
                },
            )
            .unwrap();
            let path = out.path().join("junit.xml");
            if failure == Some("existing-report") {
                fs::write(&path, "preexisting bytes").unwrap();
            }
            let mut collector = Collector::new(
                Locations {
                    workspace: out.path(),
                    output: out.path(),
                    bundle: bundle.path(),
                },
                &source,
            );
            collector.defer(&action, stdout, 0).unwrap();
            if failure.is_none() {
                assert_eq!(reports::junit_summary(&path).unwrap()["passed"], 1);
            } else if failure == Some("existing-report") {
                assert_eq!(fs::read_to_string(&path).unwrap(), "preexisting bytes");
            }
            // A hook may transform a valid normalized report, but cannot erase
            // the failure to obtain valid native evidence in the first place.
            fs::write(
                &path,
                "<testsuite><testcase><skipped/></testcase></testsuite>",
            )
            .unwrap();
            assert!(collector.finish("app:test").unwrap().is_empty());
            let mut completed = collector.finish("app:post_test").unwrap();
            let report = completed.remove(0).1.remove(0);
            assert_eq!(report.failed(), failure.is_some());
            if failure.is_none() {
                assert_eq!(report.record["summary"]["skipped"], 1);
                assert_eq!(
                    report.record["digest"],
                    snapshot::file_digest(&path).unwrap()
                );
            } else {
                assert_eq!(report.record["status"], "invalid");
            }
            collector.ensure_finished().unwrap();
        }
    }

    #[test]
    fn empty_glob_is_missing_evidence_and_matching_is_portable() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("reports/nested")).unwrap();
        fs::write(root.path().join("reports/test-a.xml"), "x").unwrap();
        fs::write(root.path().join("reports/nested/test-b.xml"), "x").unwrap();
        assert_eq!(
            reports::matches(root.path(), "reports/test-?.xml").unwrap(),
            vec!["reports/test-a.xml"]
        );
        assert_eq!(
            reports::matches(root.path(), "reports/**/test-?.xml")
                .unwrap()
                .len(),
            2
        );
        assert!(reports::matches(root.path(), "reports/**/missing.xml").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn declared_report_symlinks_cannot_export_outside_the_workspace() {
        use std::os::unix::fs::symlink;
        let work = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let bundle = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.xml"), "must not be copied").unwrap();
        symlink(outside.path(), work.path().join("reports")).unwrap();
        assert!(reports::matches(work.path(), "reports/*.xml").is_err());
        let report = collect_file(work.path(), bundle.path(), "reports/secret.xml");
        assert!(report.failed());
        assert!(report.record.get("path").is_none());
        assert_eq!(fs::read_dir(bundle.path()).unwrap().count(), 0);
    }
}
