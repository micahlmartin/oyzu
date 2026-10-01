//! Native Node test reporting. Only exact known commands are instrumented;
//! arbitrary shell programs and custom runners retain their original semantics.
use super::super::{strings, ReportFormat, ReportSpec, TaskPlan};
use crate::{
    model::{Target, Task},
    reports::ReportSource,
};

fn arguments(id: &str) -> Vec<String> {
    strings(&[
        "--experimental-test-coverage",
        "--test-reporter=junit",
        &format!("--test-reporter-destination=/out/{id}/reports/junit.xml"),
        "--test-reporter=lcov",
        &format!("--test-reporter-destination=/out/{id}/reports/coverage.lcov"),
    ])
}

pub(super) fn test(id: &str) -> TaskPlan {
    let mut argv = strings(&["npm", "run", "test", "--"]);
    argv.extend(arguments(id));
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

pub(super) fn instrument_override(target: &Target, task: &Task) -> Option<Vec<String>> {
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
    command.extend(arguments(&target.name));
    Some(command)
}
