//! Capture native report evidence before parsing it. No ecosystem dispatch or execution.
use super::bundle::{capture_bounded_output, safe_report_parent};
use crate::{reports, snapshot};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub(super) struct CollectedReport {
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

pub(super) fn collect(
    action: &Value,
    intent: &Value,
    source_digest: &Value,
    out: &Path,
    bundle: &Path,
    stdout: &Path,
) -> Result<CollectedReport> {
    let id = intent["id"].as_str().context("missing report id")?;
    let path = action["extensions"]["oyzu.dev/report-paths"][id]
        .as_str()
        .context("missing report path")?;
    let format = intent["format"].as_str().context("missing report format")?;
    let mut record = json!({"id":id,"action":action["id"],"target":action["target"],
        "kind":intent["kind"],"format":format,"status":"invalid","summary":{}});
    let parsed = (|| -> Result<Value> {
        let source: reports::ReportSource = action["extensions"]["oyzu.dev/report-sources"]
            .get(id)
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()?
            .unwrap_or_default();
        if source != reports::ReportSource::File {
            safe_report_parent(out, path)?;
            reports::materialize(source, stdout, &out.join(path))?;
        }
        let captured = capture_bounded_output(out, bundle, path, reports::MAX_REPORT_BYTES)?;
        record["path"] = json!(path);
        record["digest"] = json!(snapshot::file_digest(&captured)?);
        record["subjectDigest"] = source_digest.clone();
        // Summary and recorded digest must describe the same retained bytes.
        if format == "junit" {
            reports::junit_summary(&captured)
        } else {
            reports::coverage_summary(&captured, format)
        }
    })();
    let diagnostic = match parsed {
        Ok(summary) => {
            record["status"] = json!("collected");
            record["summary"] = summary;
            None
        }
        Err(error) => Some(
            json!({"code":"report-invalid","phase":"collect","severity":"error",
            "message":error.to_string(),"action":action["id"],"target":action["target"]}),
        ),
    };
    Ok(CollectedReport { record, diagnostic })
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
            out,
            bundle,
            &out.join("stdout"),
        )
        .unwrap()
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
}
