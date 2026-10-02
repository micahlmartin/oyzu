//! Human CLI styling consumes frozen configuration, never rereading sources.
use oyzu::model::{Task, Workspace};

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

/// Render evidence already returned by the build; never infer a successful artifact.
pub(super) fn build_result(result: &serde_json::Value, plan: bool, root: &std::path::Path) {
    println!(
        "\nOyzu {}: {}",
        if plan { "plan" } else { "build" },
        if plan {
            "READY"
        } else {
            result["status"].as_str().unwrap_or("failed")
        }
    );
    if let Some(actions) = result["actions"].as_array() {
        for action in actions {
            let status = action["status"].as_str().unwrap_or("planned");
            let duration = action["durationMs"]
                .as_u64()
                .map(|v| format!("  {:.2}s", v as f64 / 1000.0))
                .unwrap_or_default();
            println!(
                "  {:<12} {}{}",
                status.to_uppercase(),
                action["id"].as_str().unwrap_or("?"),
                duration
            );
            if let Some(reason) = action["reason"].as_str() {
                println!("               {reason}");
            }
        }
    }
    for category in ["artifacts", "reports"] {
        if let Some(items) = result[category].as_array() {
            for item in items {
                if let Some(path) = item["path"].as_str() {
                    println!("  {}: {}", category, root.join("dist").join(path).display());
                }
            }
        }
    }
    if let Some(diagnostics) = result["diagnostics"].as_array() {
        for diagnostic in diagnostics {
            if let Some(message) = diagnostic["message"].as_str() {
                println!("  ERROR: {message}");
            }
        }
    }
    if !plan {
        println!("  Manifest: {}", root.join("dist/manifest.json").display());
        println!("  Logs:     {}", root.join("dist/logs").display());
    }
}
