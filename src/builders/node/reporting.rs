//! Native Node test reporting. Only exact known commands are instrumented;
//! arbitrary shell programs and custom runners retain their original semantics.
use super::super::{strings, ReportFormat, ReportSpec, TaskPlan};
use crate::{model::Task, reports::ReportSource};

fn arguments(test: &str, coverage: &str) -> Vec<String> {
    strings(&[
        "--experimental-test-coverage",
        "--test-reporter=junit",
        &format!("--test-reporter-destination={test}"),
        "--test-reporter=lcov",
        &format!("--test-reporter-destination={coverage}"),
    ])
}

pub(super) fn test(id: &str) -> TaskPlan {
    let mut argv = strings(&["npm", "run", "test", "--"]);
    argv.extend(arguments(
        &format!("/out/{id}/reports/junit.xml"),
        &format!("/out/{id}/reports/coverage.lcov"),
    ));
    TaskPlan {
        argv,
        reports: vec![
            ReportSpec {
                format: ReportFormat::Junit,
                filename: "junit.xml",
                source: ReportSource::File,
            },
            ReportSpec {
                format: ReportFormat::Lcov,
                filename: "coverage.lcov",
                source: ReportSource::File,
            },
        ],
    }
}

pub(super) fn instrument_override(
    task: &Task,
    env: &std::collections::BTreeMap<String, String>,
) -> Option<Vec<String>> {
    if task.name != "test" {
        return None;
    }
    let argv: Vec<_> = task.argv.iter().map(String::as_str).collect();
    let mut command = match argv.as_slice() {
        ["node", "--test"] | ["sh", "-c", "node --test"] => strings(&["node", "--test"]),
        ["npm", "run", "test"] | ["sh", "-c", "npm run test"] => {
            strings(&["npm", "run", "test", "--"])
        }
        _ => return None,
    };
    // The planner resolves declared paths against captured task cwd. Node only
    // owns translating these concrete destinations into its reporter arguments.
    command.extend(arguments(
        env.get("OYZU_TEST_REPORT")?,
        env.get("OYZU_COVERAGE_REPORT")?,
    ));
    Some(command)
}
