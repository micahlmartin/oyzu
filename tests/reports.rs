use oyzu::reports::{coverage_summary, go_to_junit, junit_summary};
use serde_json::json;
use std::fs;

#[test]
fn junit_counts_outcomes_and_rejects_unrelated_or_unsafe_xml() {
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("junit.xml");
    fs::write(&report, r#"<testsuites><testsuite><testcase/><testcase><failure/></testcase><testcase><error/></testcase><testcase><skipped/></testcase></testsuite></testsuites>"#).unwrap();
    assert_eq!(
        junit_summary(&report).unwrap(),
        json!({"passed":1,"failed":2,"skipped":1,"total":4})
    );
    fs::write(&report, "<unrelated/>").unwrap();
    assert!(junit_summary(&report).is_err());
    fs::write(&report, r#"<!DOCTYPE testsuite [<!ENTITY secret SYSTEM "file:///etc/passwd">]><testsuite>&secret;</testsuite>"#).unwrap();
    assert!(junit_summary(&report).is_err());
}

#[test]
fn go_report_preserves_failure_skip_and_incomplete_test_results() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("go.jsonl");
    let report = dir.path().join("junit.xml");
    let events = [
        json!({"Package":"app","Test":"TestPass","Action":"pass"}),
        json!({"Package":"app","Test":"TestFail","Action":"output","Output":"expected <one> & got two"}),
        json!({"Package":"app","Test":"TestFail","Action":"fail"}),
        json!({"Package":"app","Test":"TestSkip","Action":"skip"}),
        json!({"Package":"app","Test":"TestIncomplete","Action":"run"}),
    ];
    fs::write(
        &log,
        events
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    assert_eq!(
        go_to_junit(&log, &report).unwrap(),
        json!({"passed":1,"failed":2,"skipped":1,"total":4})
    );
    let xml = fs::read_to_string(&report).unwrap();
    assert!(xml.contains("expected &lt;one&gt; &amp; got two"));
}

#[test]
fn coverage_retains_metric_and_zero_denominator_and_rejects_bad_records() {
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("coverage.txt");
    fs::write(&report, "SF:app.js\nDA:1,1\nDA:2,0\nend_of_record\n").unwrap();
    assert_eq!(
        coverage_summary(&report, "lcov").unwrap(),
        json!({"covered":1,"total":2,"metric":"lines"})
    );
    fs::write(
        &report,
        "mode: set\napp/main.go:1.1,3.2 3 1\napp/main.go:4.1,5.2 2 0\n",
    )
    .unwrap();
    assert_eq!(
        coverage_summary(&report, "go-cover").unwrap(),
        json!({"covered":3,"total":5,"metric":"statements"})
    );
    fs::write(&report, "mode: set\n").unwrap();
    assert_eq!(coverage_summary(&report, "go-cover").unwrap()["total"], 0);
    fs::write(&report, "DA:1,invalid\n").unwrap();
    assert!(coverage_summary(&report, "lcov").is_err());
    assert!(coverage_summary(&report, "unknown").is_err());
}
