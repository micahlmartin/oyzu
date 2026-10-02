//! Engine-owned OCI assertions: no project commands, registry access or extraction.
use super::{files, Execution, Request};
use crate::reports::assertions::{self, Assertion, Outcome};
use anyhow::{bail, Result};
use std::{fs, time::Instant};

pub(super) fn execute(request: Request<'_>, input: &str, report: &str) -> Result<Execution> {
    let start = Instant::now();
    // Exclusive creation refuses stale reports and project-created symlinks.
    let output = files::output_file(request.output, report)?;
    let checked = files::file(request.output, input)
        .and_then(|path| crate::oci::verify(&path))
        .and_then(|value| {
            if value.kind != "oci-image" {
                bail!("expected one OCI image, found {}", value.kind);
            }
            Ok(value)
        });
    let (integrity, platform, failure) = match checked {
        Ok(image) => {
            let matches = image.platforms.len() == 1
                && image
                    .platforms
                    .iter()
                    .all(|p| p.os == request.image.os && p.architecture == request.image.arch);
            if matches {
                (Outcome::Passed, Outcome::Passed, None)
            } else {
                let message = format!(
                    "image platform does not match planned {}/{}",
                    request.image.os, request.image.arch
                );
                (
                    Outcome::Passed,
                    Outcome::Failed(message.clone()),
                    Some(message),
                )
            }
        }
        Err(error) => {
            let message = format!("{error:#}");
            (
                Outcome::Failed(message.clone()),
                Outcome::Skipped("Image integrity validation failed".into()),
                Some(message),
            )
        }
    };
    assertions::write(
        output,
        "oyzu.oci.validation",
        &[
            Assertion {
                name: "OCI content integrity and configuration",
                outcome: integrity,
            },
            Assertion {
                name: "Resolved target platform",
                outcome: platform,
            },
        ],
    )?;
    fs::write(
        request.stdout,
        if failure.is_none() {
            "OCI content and platform assertions passed\n"
        } else {
            "OCI validation failed\n"
        },
    )?;
    fs::write(request.stderr, failure.as_deref().unwrap_or(""))?;
    Ok(Execution {
        code: i32::from(failure.is_some()),
        duration_ms: start.elapsed().as_millis() as u64,
        timed_out: false,
    })
}
