mod contract;
use anyhow::{bail, Result};
pub(crate) use contract::{matches, validate_declarations, Input, Root};
pub use contract::{Declaration, Format};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::Path,
};

/// How a native adapter supplies a report to the common collector.
#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ReportSource {
    #[default]
    File,
    GoTestEvents,
}

pub(crate) fn materialize(source: ReportSource, stdout: &Path, destination: &Path) -> Result<()> {
    match source {
        ReportSource::File => Ok(()),
        ReportSource::GoTestEvents => go_to_junit(stdout, destination).map(|_| ()),
    }
}

pub(crate) const MAX_REPORT_BYTES: u64 = 16 * 1024 * 1024;

fn read_report(path: &Path) -> Result<String> {
    let mut text = String::new();
    fs::File::open(path)?
        .take(MAX_REPORT_BYTES + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > MAX_REPORT_BYTES {
        bail!("report exceeds 16 MiB");
    }
    Ok(text)
}

fn xml_report(text: &str, cobertura: bool) -> Result<roxmltree::Document<'_>> {
    // Native Cobertura generators emit this inert external declaration. roxmltree
    // reads only the supplied string: it never retrieves the referenced DTD.
    // Keep internal subsets, other DTDs and entity definitions disabled.
    let known_doctype = [
        "<!DOCTYPE coverage SYSTEM \"https://cobertura.sourceforge.net/xml/coverage-04.dtd\">",
        "<!DOCTYPE coverage SYSTEM \"http://cobertura.sourceforge.net/xml/coverage-04.dtd\">",
        "<!DOCTYPE coverage SYSTEM 'https://cobertura.sourceforge.net/xml/coverage-04.dtd'>",
        "<!DOCTYPE coverage SYSTEM 'http://cobertura.sourceforge.net/xml/coverage-04.dtd'>",
    ];
    let allow_dtd = cobertura
        && text.matches("<!DOCTYPE").count() == 1
        && known_doctype
            .iter()
            .any(|declaration| text.contains(declaration))
        && !text.contains("<!ENTITY");
    Ok(roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd,
            nodes_limit: 1_000_000,
        },
    )?)
}

pub fn junit_summary(path: &Path) -> Result<Value> {
    let text = read_report(path)?;
    let doc = xml_report(&text, false)?;
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
    let mut packages = BTreeMap::<String, (bool, String)>::new();
    let mut seen = false;
    for line in read_report(log)?.lines().filter(|s| !s.trim().is_empty()) {
        let event: Value = serde_json::from_str(line)?;
        let Some(package) = event["Package"].as_str() else {
            continue;
        };
        seen = true;
        let Some(test) = event["Test"].as_str() else {
            let package = packages.entry(package.into()).or_default();
            if event["Action"] == "fail" {
                package.0 = true;
            }
            if let Some(output) = event["Output"].as_str() {
                package.1.push_str(output);
            }
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
    if !seen {
        bail!("missing Go test events");
    }
    for (package, (failed, output)) in packages {
        if failed
            && !cases
                .iter()
                .any(|((p, _), (status, _))| p == &package && status != "pass" && status != "skip")
        {
            cases.insert((package, "package failure".into()), ("fail".into(), output));
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
        .chars()
        .map(|c| {
            if c == '\t'
                || c == '\n'
                || c == '\r'
                || (c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}')
            {
                c
            } else {
                '\u{fffd}'
            }
        })
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn coverage_summary(path: &Path, format: &str) -> Result<Value> {
    let text = read_report(path)?;
    let (mut total, mut covered) = (0u64, 0u64);
    match format {
        "cobertura" => {
            let doc = xml_report(&text, true)?;
            let root = doc.root_element();
            if !root.has_tag_name("coverage") {
                bail!("invalid coverage root");
            }
            total = root
                .attribute("lines-valid")
                .ok_or_else(|| anyhow::anyhow!("missing coverage denominator"))?
                .parse()?;
            covered = root
                .attribute("lines-covered")
                .ok_or_else(|| anyhow::anyhow!("missing covered lines"))?
                .parse()?;
            if covered > total {
                bail!("covered lines exceed denominator");
            }
        }
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
        json!({"covered":covered,"total":total,"metric":if format=="go-cover" {"statements"} else {"lines"}}),
    )
}
