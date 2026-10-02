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

#[test]
fn cobertura_accepts_native_declaration_without_loading_dtds_or_entities() {
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("coverage.xml");
    let native =
        "<!DOCTYPE coverage SYSTEM \"https://cobertura.sourceforge.net/xml/coverage-04.dtd\">";
    let body = "<coverage lines-covered='2' lines-valid='3'/>";
    fs::write(&report, format!("<?xml version='1.0'?>\n{native}\n{body}")).unwrap();
    assert_eq!(
        coverage_summary(&report, "cobertura").unwrap(),
        json!({"covered":2,"total":3,"metric":"lines"})
    );
    for invalid in [
        format!("<!DOCTYPE coverage SYSTEM 'file:///etc/passwd'>{body}"),
        format!("<!DOCTYPE coverage [<!ENTITY secret SYSTEM 'file:///etc/passwd'>]>{body}"),
        format!("<!-- {native} --><!DOCTYPE coverage [<!ENTITY secret '2'>]>{body}"),
        format!("{native}<coverage lines-covered='&secret;' lines-valid='3'/>"),
    ] {
        fs::write(&report, invalid).unwrap();
        assert!(coverage_summary(&report, "cobertura").is_err());
    }
}

#[test]
fn package_level_go_failures_cannot_be_reported_as_success() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("go.jsonl");
    let report = dir.path().join("junit.xml");
    fs::write(&log, "{\"Action\":\"fail\",\"Package\":\"broken\"}\n").unwrap();
    assert_eq!(go_to_junit(&log, &report).unwrap()["failed"], 1);
    assert!(
        go_to_junit(&log, &report).is_err(),
        "must not overwrite a supplied report path"
    );
    fs::write(&log, "invalid events").unwrap();
    assert!(go_to_junit(&log, &dir.path().join("invalid.xml")).is_err());
}
#[test]
fn report_declarations_reject_unknown_formats_mismatched_kinds_and_escaping_globs() {
    let root = tempfile::tempdir().unwrap();
    for declaration in [
        "{kind='test',format='unknown',path='report.xml'}",
        "{kind='coverage',format='junit',path='report.xml'}",
        "{kind='test',format='junit',path='../report.xml'}",
        "{kind='test',format='junit',path='/tmp/report.xml'}",
        "{kind='test',format='junit',path='C:/report.xml'}",
        "{kind='test',format='junit',path='reports/{a,b}.xml'}",
        "{kind='test',format='junit',path='reports/[ab].xml'}",
        "{kind='test',format='junit',path='reports/a**b.xml'}",
    ] {
        std::fs::write(
            root.path().join("oyzu.toml"),
            format!("[tasks.test]\nargv=['custom-test']\nreports=[{declaration}]\n"),
        )
        .unwrap();
        assert!(oyzu::config::project(root.path()).is_err(), "{declaration}");
    }
    std::fs::write(root.path().join("oyzu.toml"), "[tasks.test]\nargv=['custom-test']\nreports=[{kind='test',format='junit',path='reports/**/test-?.xml'}]\n").unwrap();
    assert_eq!(
        oyzu::config::project(root.path()).unwrap().tasks["test"]
            .reports
            .len(),
        1
    );
}
#[test]
fn jacoco_uses_native_aggregate_lines_without_double_counting_nested_counters() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("jacoco.xml");
    let document = "<!DOCTYPE report PUBLIC \"-//JACOCO//DTD Report 1.1//EN\" \"report.dtd\"><report name=\"app\"><package name=\"example\"><counter type=\"LINE\" missed=\"1\" covered=\"2\"/></package><counter type=\"LINE\" missed=\"1\" covered=\"2\"/></report>";
    std::fs::write(&path, document).unwrap();
    assert_eq!(
        oyzu::reports::coverage_summary(&path, "jacoco").unwrap(),
        serde_json::json!({"covered":2,"total":3,"metric":"lines"})
    );
    for invalid in [
        document.replace("report.dtd", "https://example.invalid/foreign.dtd"),
        document.replace("covered=\"2\"", "covered=\"-2\""),
        "<!DOCTYPE report [<!ENTITY secret SYSTEM 'file:///secret'>]><report>&secret;</report>"
            .into(),
    ] {
        std::fs::write(&path, invalid).unwrap();
        assert!(oyzu::reports::coverage_summary(&path, "jacoco").is_err());
    }
}

#[test]
fn jacoco_without_line_debug_data_preserves_bytecode_measurements() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("jacoco.xml");
    // Native JaCoCo output retained from Linux CI run 36953677315.
    std::fs::write(&path, include_str!("fixtures/reports/ant-no-lines.xml")).unwrap();
    assert_eq!(
        oyzu::reports::coverage_summary(&path, "jacoco").unwrap(),
        json!({"covered":2,"total":5,"metric":"instructions"})
    );
    let instruction = "<counter type='INSTRUCTION' missed='3' covered='7'/>";
    let line = "<counter type='LINE' missed='1' covered='2'/>";
    let document = format!("<report><package>{instruction}</package>{instruction}</report>");
    std::fs::write(&path, &document).unwrap();
    assert_eq!(
        oyzu::reports::coverage_summary(&path, "jacoco").unwrap(),
        json!({"covered":7,"total":10,"metric":"instructions"})
    );
    std::fs::write(&path, format!("<report>{instruction}{line}</report>")).unwrap();
    assert_eq!(
        oyzu::reports::coverage_summary(&path, "jacoco").unwrap(),
        json!({"covered":2,"total":3,"metric":"lines"})
    );
    std::fs::write(
        &path,
        "<report><counter type='INSTRUCTION' missed='0' covered='0'/></report>",
    )
    .unwrap();
    assert_eq!(
        oyzu::reports::coverage_summary(&path, "jacoco").unwrap(),
        json!({"covered":0,"total":0,"metric":"instructions"})
    );
    for invalid in [
        format!("<report><package>{instruction}</package></report>"),
        format!("<report>{instruction}{instruction}</report>"),
        format!("<report>{instruction}{line}{line}</report>"),
        format!("<report>{instruction}<counter type='LINE' missed='x' covered='2'/></report>"),
        document.replace("covered='7'", "covered='-7'"),
        document.replace("missed='3'", "missed='18446744073709551615'"),
    ] {
        std::fs::write(&path, invalid).unwrap();
        assert!(oyzu::reports::coverage_summary(&path, "jacoco").is_err());
    }
}
