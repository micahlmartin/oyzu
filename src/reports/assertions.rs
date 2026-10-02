//! JUnit encoding for assertions that an engine validator actually executed.
use anyhow::{bail, Result};
use std::io::Write;

pub(crate) enum Outcome {
    Passed,
    Failed(String),
    Skipped(String),
}

pub(crate) struct Assertion {
    pub name: &'static str,
    pub outcome: Outcome,
}

pub(crate) fn write(mut output: impl Write, suite: &str, assertions: &[Assertion]) -> Result<()> {
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><testsuite name=\"{}\">",
        super::escape(suite)
    );
    for assertion in assertions {
        xml.push_str(&format!(
            "<testcase name=\"{}\">",
            super::escape(assertion.name)
        ));
        match &assertion.outcome {
            Outcome::Passed => (),
            Outcome::Failed(reason) => {
                xml.push_str(&format!("<failure>{}</failure>", super::escape(reason)))
            }
            Outcome::Skipped(reason) => {
                xml.push_str(&format!("<skipped message=\"{}\"/>", super::escape(reason)))
            }
        }
        xml.push_str("</testcase>");
    }
    xml.push_str("</testsuite>");
    if xml.len() as u64 > super::MAX_REPORT_BYTES {
        bail!("assertion report exceeds limit");
    }
    output.write_all(xml.as_bytes())?;
    Ok(())
}
