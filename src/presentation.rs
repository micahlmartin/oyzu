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
