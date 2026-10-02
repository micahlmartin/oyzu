//! CLI presentation consumes frozen configuration and build evidence.
//! Provider environment hints select presentation only; they grant no authority.
use oyzu::model::{Task, Workspace};

pub(super) fn path_label(path: &std::path::Path) -> String {
    let text = path.display().to_string();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
    }
}

pub(super) fn label(
    workspace: &Workspace,
    task: &Task,
    text: &str,
    success: bool,
    terminal: bool,
) -> String {
    let preference = workspace
        .configuration
        .get(&task.target)
        .or(workspace.root_configuration.as_ref())
        .and_then(|config| config.get("ui.color"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("auto");
    let enabled = match preference {
        "always" => true,
        "never" => false,
        _ => terminal,
    };
    if enabled {
        let color = if success { 32 } else { 31 };
        format!("\x1b[{color}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

/// Render only collected evidence. Durations are summed action time, not target
/// wall time; parallelism means they must not be summed into build elapsed time.
pub(super) fn build_receipt(
    result: &serde_json::Value,
    plan: bool,
    root: &std::path::Path,
    elapsed: std::time::Duration,
) -> String {
    use std::fmt::Write;
    let mut text = String::new();
    let _ = writeln!(
        text,
        "\nOyzu {}: {}  [{:.2}s elapsed]",
        if plan { "plan" } else { "build" },
        if plan {
            "READY"
        } else {
            result["status"].as_str().unwrap_or("failed")
        },
        elapsed.as_secs_f64()
    );
    let actions: Vec<_> = result["actions"].as_array().into_iter().flatten().collect();
    let _ = writeln!(
        text,
        "\nTARGET                   RESULT       TASKS     ACTION TIME"
    );
    for target in result["targets"].as_array().into_iter().flatten() {
        let id = target["id"].as_str().unwrap_or("?");
        let own: Vec<_> = actions.iter().filter(|a| a["target"] == id).collect();
        let passed = own.iter().filter(|a| a["status"] == "succeeded").count();
        let status = if plan {
            "PLANNED"
        } else if own.iter().any(|a| a["status"] == "failed") {
            "FAILED"
        } else if own.iter().any(|a| a["status"] == "blocked") {
            "BLOCKED"
        } else if !own.is_empty() && passed == own.len() {
            "PASSED"
        } else {
            "NO TASKS"
        };
        let duration: u64 = own.iter().filter_map(|a| a["durationMs"].as_u64()).sum();
        let _ = writeln!(
            text,
            "{id:<24} {status:<12} {passed:>2}/{:<4} {:.2}s",
            own.len(),
            duration as f64 / 1000.0
        );
    }
    for action in &actions {
        if matches!(action["status"].as_str(), Some("failed" | "blocked")) {
            let _ = writeln!(
                text,
                "\n{} {}{}",
                action["status"].as_str().unwrap().to_uppercase(),
                action["id"].as_str().unwrap_or("?"),
                action["exitCode"]
                    .as_i64()
                    .map(|c| format!(" (exit {c})"))
                    .unwrap_or_default()
            );
            if let Some(reason) = action["reason"].as_str() {
                let _ = writeln!(text, "  {reason}");
            }
        }
    }
    let reports: Vec<_> = result["reports"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["status"] == "collected")
        .collect();
    let tests: Vec<_> = reports.iter().filter(|r| r["kind"] == "test").collect();
    if !tests.is_empty() {
        let count = |key: &str| {
            tests
                .iter()
                .filter_map(|r| r["summary"][key].as_u64())
                .sum::<u64>()
        };
        let _ = writeln!(
            text,
            "\nTests       {} passed · {} failed · {} skipped ({} collected reports)",
            count("passed"),
            count("failed"),
            count("skipped"),
            tests.len()
        );
    }
    for report in &reports {
        if report["kind"] == "coverage" {
            if let (Some(covered), Some(total)) = (
                report["summary"]["covered"].as_u64(),
                report["summary"]["total"].as_u64(),
            ) {
                let percentage = if total > 0 {
                    format!("{:.1}%", covered as f64 * 100.0 / total as f64)
                } else {
                    "n/a".into()
                };
                let _ = writeln!(
                    text,
                    "Coverage    {}: {percentage} ({covered}/{total} {})",
                    report["target"].as_str().unwrap_or("?"),
                    report["summary"]["metric"].as_str().unwrap_or("units")
                );
            }
        }
    }
    for category in ["artifacts", "reports"] {
        let _ = writeln!(
            text,
            "\n{}{}",
            category.to_uppercase(),
            if plan { " (planned)" } else { "" }
        );
        let mut count = 0;
        for item in result[category].as_array().into_iter().flatten() {
            if let Some(path) = item["path"].as_str() {
                count += 1;
                let _ = writeln!(
                    text,
                    "  {}  {}",
                    item["target"].as_str().unwrap_or(""),
                    path_label(&root.join("dist").join(path))
                );
            }
        }
        if count == 0 {
            let _ = writeln!(text, "  None");
        }
    }
    for diagnostic in result["diagnostics"].as_array().into_iter().flatten() {
        if let Some(message) = diagnostic["message"].as_str() {
            let _ = writeln!(text, "\nERROR  {message}");
        }
    }
    if !plan {
        let _ = writeln!(
            text,
            "\nManifest  {}\nLogs      {}",
            path_label(&root.join("dist/manifest.json")),
            path_label(&root.join("dist/logs"))
        );
    }
    // Project-controlled text cannot emit terminal or CI control sequences.
    text.chars()
        .flat_map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

pub(super) fn environment() -> String {
    for (key, name) in [
        ("GITHUB_ACTIONS", "GitHub Actions"),
        ("GITLAB_CI", "GitLab CI"),
        ("TF_BUILD", "Azure Pipelines"),
        ("BUILDKITE", "Buildkite"),
        ("JENKINS_URL", "Jenkins"),
        ("TEAMCITY_VERSION", "TeamCity"),
    ] {
        if std::env::var(key).is_ok_and(|v| {
            !v.is_empty() && !matches!(v.to_ascii_lowercase().as_str(), "false" | "0")
        }) {
            return format!("CI · {name} (unverified)");
        }
    }
    if oyzu::config::sources::detected_ci() {
        "CI (unverified)".into()
    } else {
        "local".into()
    }
}

/// Provider reporting is best effort and never changes the build result.
pub(super) fn github_summary(receipt: &str) -> std::io::Result<()> {
    use std::io::Write;
    if !std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true") {
        return Ok(());
    }
    if let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        let escaped = receipt
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        writeln!(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?,
            "<pre>{escaped}</pre>"
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receipt_reports_collected_measurements_and_blocked_targets() {
        let result = serde_json::json!({"status":"failed","targets":[{"id":"api"}],"actions":[{"target":"api","id":"api:test","status":"failed","exitCode":7},{"target":"api","id":"api:package","status":"blocked","reason":"test failed"}],"reports":[{"target":"api","kind":"test","status":"collected","summary":{"passed":2,"failed":1,"skipped":3}},{"target":"api","kind":"coverage","status":"invalid","summary":{"covered":1,"total":1}}]});
        let text = build_receipt(
            &result,
            false,
            std::path::Path::new("."),
            std::time::Duration::from_secs(2),
        );
        assert!(text.contains("2 passed · 1 failed · 3 skipped"));
        assert!(text.contains("BLOCKED api:package"));
        assert!(!text.contains("100.0%"));
    }
}
