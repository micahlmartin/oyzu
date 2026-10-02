use super::Event;

fn clean(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| {
            if c.is_control() && c != '\t' {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn argument(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/._:=+-".contains(&c))
    {
        value.into()
    } else {
        serde_json::to_string(value).unwrap()
    }
}
pub(super) fn line(elapsed: u64, scope: &str, event: &Event<'_>) -> String {
    let prefix = format!(
        "[{:02}:{:02}.{:03}] [{}]",
        elapsed / 60000,
        elapsed / 1000 % 60,
        elapsed % 1000,
        clean(scope)
    );
    let detail = match event {
        Event::Progress { message } => clean(message),
        Event::Command { argv, cwd } => format!(
            "$ {}  (cwd: {})",
            argv.iter()
                .map(|v| argument(v))
                .collect::<Vec<_>>()
                .join(" "),
            clean(cwd)
        ),
        Event::Output {
            stream,
            text,
            continued,
        } => format!(
            "{stream} | {}{}",
            clean(text),
            if *continued { " [continued]" } else { "" }
        ),
        Event::Finished {
            status,
            exit_code,
            duration_ms,
        } => format!(
            "{}{}{}",
            status.to_uppercase(),
            exit_code
                .map(|c| format!(" (exit {c})"))
                .unwrap_or_default(),
            duration_ms
                .map(|d| format!(" [{:.2}s]", d as f64 / 1000.0))
                .unwrap_or_default()
        ),
    };
    format!("{prefix} {detail}")
}
