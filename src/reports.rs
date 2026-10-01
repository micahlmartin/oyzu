use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

pub fn junit_summary(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path)?;
    if text.len() > 16 * 1024 * 1024 {
        bail!("JUnit report exceeds 16 MiB");
    }
    let doc = roxmltree::Document::parse(&text)?;
    if !matches!(
        doc.root_element().tag_name().name(),
        "testsuite" | "testsuites"
    ) {
        bail!("invalid JUnit root element");
    }
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    for case in doc.descendants().filter(|n| n.has_tag_name("testcase")) {
        if case
            .children()
            .any(|n| n.has_tag_name("failure") || n.has_tag_name("error"))
        {
            failed += 1;
        } else if case.children().any(|n| n.has_tag_name("skipped")) {
            skipped += 1;
        } else {
            passed += 1;
        }
    }
    Ok(json!({"passed":passed,"failed":failed,"skipped":skipped,"total":passed+failed+skipped}))
}

pub fn go_to_junit(log: &Path, report: &Path) -> Result<Value> {
    let mut cases = BTreeMap::<(String, String), (String, String)>::new();
    for line in fs::read_to_string(log)?.lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let (Some(package), Some(test)) = (event["Package"].as_str(), event["Test"].as_str())
        else {
            continue;
        };
        let record = cases.entry((package.into(), test.into())).or_default();
        if let Some(output) = event["Output"].as_str() {
            record.1.push_str(output);
        }
        if matches!(event["Action"].as_str(), Some("pass" | "fail" | "skip")) {
            record.0 = event["Action"].as_str().unwrap().into();
        }
    }
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuites><testsuite name=\"go\">",
    );
    for ((package, test), (status, output)) in cases {
        xml.push_str(&format!(
            "<testcase classname=\"{}\" name=\"{}\">",
            escape(&package),
            escape(&test)
        ));
        match status.as_str() {
            "pass" => {}
            "skip" => xml.push_str("<skipped/>"),
            _ => xml.push_str(&format!("<failure>{}</failure>", escape(&output))),
        }
        xml.push_str("</testcase>");
    }
    xml.push_str("</testsuite></testsuites>");
    // Never follow an output created by project code when normalizing test events.
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(report)?;
    output.write_all(xml.as_bytes())?;
    junit_summary(report)
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn coverage_summary(path: &Path, format: &str) -> Result<Value> {
    let text = fs::read_to_string(path)?;
    let (mut total, mut covered) = (0u64, 0u64);
    match format {
        "lcov" => {
            for line in text.lines().filter_map(|line| line.strip_prefix("DA:")) {
                let fields: Vec<_> = line.split(',').collect();
                if fields.len() < 2 {
                    bail!("invalid LCOV line record");
                }
                total += 1;
                if fields[1].parse::<u64>()? > 0 {
                    covered += 1;
                }
            }
        }
        "go-cover" => {
            if !text.starts_with("mode:") {
                bail!("invalid Go coverage header");
            }
            for line in text.lines().skip(1) {
                let fields: Vec<_> = line.split_whitespace().collect();
                if fields.len() != 3 {
                    bail!("invalid Go coverage record");
                }
                let statements = fields[1].parse::<u64>()?;
                total += statements;
                if fields[2].parse::<u64>()? > 0 {
                    covered += statements;
                }
            }
        }
        _ => bail!("unsupported coverage format {format}"),
    }
    Ok(
        json!({"covered":covered,"total":total,"metric":if format=="lcov" {"lines"} else {"statements"}}),
    )
}
