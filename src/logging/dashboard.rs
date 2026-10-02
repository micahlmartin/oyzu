//! Terminal projection of observed facts; no scheduling or builder decisions.
use super::{render, Event, TaskState};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};
use unicode_width::UnicodeWidthChar;

#[derive(Default)]
struct Task {
    target: String,
    state: String,
    dependencies: Vec<String>,
    started: Option<u64>,
    duration: Option<u64>,
    command: String,
    reason: String,
}

/// Output lives in an invocation-local spool. Only offsets are held in memory;
/// scrolling reads the requested lines, not the whole build output.
struct History {
    file: File,
    end: u64,
    all: Vec<(u64, usize)>,
    scopes: BTreeMap<String, Vec<(u64, usize)>>,
}
impl History {
    fn new() -> std::io::Result<Self> {
        Ok(Self {
            file: tempfile::tempfile()?,
            end: 0,
            all: Vec::new(),
            scopes: BTreeMap::new(),
        })
    }
    fn push(&mut self, scope: &str, text: &str) -> std::io::Result<()> {
        self.file.seek(SeekFrom::Start(self.end))?;
        self.file.write_all(text.as_bytes())?;
        let entry = (self.end, text.len());
        self.end += text.len() as u64;
        self.all.push(entry);
        self.scopes.entry(scope.into()).or_default().push(entry);
        Ok(())
    }
    fn lines(&mut self, scope: Option<&str>, count: usize, offset: usize) -> Vec<String> {
        let entries = match scope {
            Some(s) => self.scopes.get(s).map(Vec::as_slice).unwrap_or(&[]),
            None => &self.all,
        };
        let end = entries
            .len()
            .saturating_sub(offset.min(entries.len().saturating_sub(count)));
        entries[end.saturating_sub(count)..end]
            .iter()
            .filter_map(|&(position, length)| {
                let mut bytes = vec![0; length];
                self.file.seek(SeekFrom::Start(position)).ok()?;
                self.file.read_exact(&mut bytes).ok()?;
                Some(String::from_utf8_lossy(&bytes).into_owned())
            })
            .collect()
    }
}

#[derive(PartialEq)]
enum Mode {
    Active,
    Selected,
    All,
}
pub(super) struct View {
    title: String,
    phase: String,
    versions: String,
    status: String,
    tasks: BTreeMap<String, Task>,
    order: Vec<String>,
    selected: usize,
    mode: Mode,
    offset: usize,
    horizontal: usize,
    history: History,
    spool_error: bool,
}
impl View {
    pub fn new(title: String) -> std::io::Result<Self> {
        Ok(Self {
            title,
            versions: String::new(),
            phase: "Preflight".into(),
            status: "Running".into(),
            tasks: BTreeMap::new(),
            order: Vec::new(),
            selected: 0,
            mode: Mode::Active,
            offset: 0,
            horizontal: 0,
            history: History::new()?,
            spool_error: false,
        })
    }
    pub fn observe(&mut self, elapsed: u64, scope: &str, event: &Event<'_>) {
        if self.offset > 0
            && (self.mode == Mode::All
                || self.order.get(self.selected).is_some_and(|id| id == scope))
        {
            self.offset = self.offset.saturating_add(1);
        }
        if self
            .history
            .push(scope, &render::line(elapsed, scope, event))
            .is_err()
        {
            self.spool_error = true;
        }
        match event {
            Event::Phase { name } => self.phase = (*name).into(),
            Event::Plan { plan } => {
                self.versions = plan["versions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                for action in plan["actions"].as_array().into_iter().flatten() {
                    let id = action["id"].as_str().unwrap_or("?").to_owned();
                    if !self.tasks.contains_key(&id) {
                        self.order.push(id.clone());
                    }
                    self.tasks.insert(
                        id,
                        Task {
                            target: action["target"].as_str().unwrap_or("?").into(),
                            state: "waiting".into(),
                            dependencies: action["dependsOn"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|v| v.as_str().map(str::to_owned))
                                .collect(),
                            ..Default::default()
                        },
                    );
                }
            }
            Event::Task { state, reason } => {
                if let Some(task) = self.tasks.get_mut(scope) {
                    task.state = match state {
                        TaskState::Running => "running",
                        TaskState::Blocked => "blocked",
                    }
                    .into();
                    if *state == TaskState::Running {
                        task.started = Some(elapsed);
                    }
                    task.reason = reason.unwrap_or_default().into();
                }
            }
            Event::Command { .. } => {
                if let Some(task) = self.tasks.get_mut(scope) {
                    task.command = render::line(elapsed, scope, event);
                }
            }
            Event::Finished {
                status,
                duration_ms,
                ..
            } => {
                if scope == "build" {
                    self.status = status.to_uppercase();
                }
                if let Some(task) = self.tasks.get_mut(scope) {
                    if matches!(*status, "succeeded" | "failed") {
                        task.state = (*status).into();
                        task.duration = *duration_ms;
                    } else if *status == "command-succeeded" && task.state == "running" {
                        task.state = "collecting".into();
                    } else if *status == "command-failed" {
                        task.state = "command-failed".into();
                        task.duration = *duration_ms;
                    }
                    if matches!(*status, "failed" | "command-failed") {
                        self.selected = self.order.iter().position(|s| s == scope).unwrap_or(0);
                        self.mode = Mode::Selected;
                        self.offset = 0;
                    }
                }
            }
            _ => {}
        }
    }
    pub fn key(&mut self, code: crossterm::event::KeyCode) {
        use crossterm::event::KeyCode::*;
        match code {
            Up => {
                self.selected = self.selected.saturating_sub(1);
                self.mode = Mode::Selected;
                self.offset = 0;
            }
            Down => {
                self.selected = (self.selected + 1).min(self.order.len().saturating_sub(1));
                self.mode = Mode::Selected;
                self.offset = 0;
            }
            Enter => {
                self.mode = Mode::Selected;
                self.offset = 0;
            }
            Char('a') => {
                self.mode = Mode::Active;
                self.offset = 0;
            }
            Char('l') => {
                self.mode = Mode::All;
                self.offset = 0;
            }
            Char('f') | End => self.offset = 0,
            PageUp => self.offset = self.offset.saturating_add(5),
            PageDown => self.offset = self.offset.saturating_sub(5),
            Left => self.horizontal = self.horizontal.saturating_sub(20),
            Right => self.horizontal = self.horizontal.saturating_add(20),
            _ => {}
        }
    }
    pub fn frame(&mut self, width: usize, height: usize, elapsed: u64) -> Vec<String> {
        if height < 10 || width < 40 {
            return vec![
                clip(
                    &format!("OYZU {} {:.1}s", self.status, elapsed as f64 / 1000.0),
                    width,
                    0,
                ),
                clip("Resize terminal; q switches to plain logs", width, 0),
            ];
        }
        let mut lines = vec![
            format!("OYZU  {}  |  {:.1}s", self.status, elapsed as f64 / 1000.0),
            format!("{} {}", self.title, self.versions),
        ];
        let phases = ["Preflight", "Dependencies", "Plan", "Execute", "Collect"];
        let current = phases.iter().position(|p| *p == self.phase).unwrap_or(0);
        lines.push(
            phases
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    format!(
                        "{} {p}",
                        if i < current {
                            "✓"
                        } else if i == current {
                            "●"
                        } else {
                            "·"
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("  "),
        );
        let running = self
            .tasks
            .values()
            .filter(|t| matches!(t.state.as_str(), "running" | "collecting"))
            .count();
        let done = self
            .tasks
            .values()
            .filter(|t| t.state == "succeeded")
            .count();
        let failed = self
            .tasks
            .values()
            .filter(|t| matches!(t.state.as_str(), "failed" | "command-failed"))
            .count();
        let blocked = self.tasks.values().filter(|t| t.state == "blocked").count();
        lines.push(format!(
            "{running} active · {done}/{} passed · {failed} failed · {blocked} blocked",
            self.tasks.len()
        ));
        lines.push("TARGET               STATUS       CURRENT TASK              FINISHED".into());
        let mut targets: Vec<String> = Vec::new();
        for id in &self.order {
            let target = &self.tasks[id].target;
            if !targets.contains(target) {
                targets.push(target.clone());
            }
        }
        let selected_target = self
            .order
            .get(self.selected)
            .map(|id| self.tasks[id].target.as_str());
        let selected_row = targets
            .iter()
            .position(|t| Some(t.as_str()) == selected_target)
            .unwrap_or(0);
        let slots = (height / 3).saturating_sub(1).max(1);
        let start = selected_row.saturating_sub(slots - 1);
        for target in targets.iter().skip(start).take(slots) {
            let tasks: Vec<_> = self
                .order
                .iter()
                .filter(|id| self.tasks[*id].target == *target)
                .collect();
            let completed = tasks
                .iter()
                .filter(|id| {
                    matches!(
                        self.tasks[id.as_str()].state.as_str(),
                        "succeeded" | "failed" | "blocked"
                    )
                })
                .count();
            let current = tasks
                .iter()
                .find(|id| {
                    matches!(
                        self.tasks[id.as_str()].state.as_str(),
                        "running" | "collecting"
                    )
                })
                .or_else(|| {
                    tasks.iter().find(|id| {
                        matches!(
                            self.tasks[id.as_str()].state.as_str(),
                            "failed" | "command-failed"
                        )
                    })
                })
                .or_else(|| {
                    tasks
                        .iter()
                        .find(|id| self.tasks[id.as_str()].state == "waiting")
                });
            let (status, action) = current
                .map(|id| {
                    (
                        self.tasks[id.as_str()].state.as_str(),
                        id.strip_prefix(&format!("{target}:")).unwrap_or(id),
                    )
                })
                .unwrap_or_else(|| {
                    (
                        if tasks
                            .iter()
                            .any(|id| self.tasks[id.as_str()].state == "blocked")
                        {
                            "blocked"
                        } else {
                            "succeeded"
                        },
                        "-",
                    )
                });
            lines.push(format!(
                "{} {:<19} {:<12} {:<25} {completed}/{}",
                if Some(target.as_str()) == selected_target {
                    ">"
                } else {
                    " "
                },
                target,
                status,
                action,
                tasks.len()
            ));
        }
        if targets.len() > slots {
            lines.push(format!(
                "Targets {}-{} of {} (↑↓ selects tasks)",
                start + 1,
                (start + slots).min(targets.len()),
                targets.len()
            ));
        }
        let space = height.saturating_sub(lines.len() + 2);
        match self.mode {
            Mode::All => {
                lines.push("── All logs (PgUp/PgDn scroll, ←→ pan, f follow) ──".into());
                lines.extend(
                    self.history
                        .lines(None, space.saturating_sub(1), self.offset)
                        .into_iter()
                        .map(|s| clip(&s, width, self.horizontal)),
                );
            }
            Mode::Selected if !self.order.is_empty() => {
                let id = &self.order[self.selected];
                let task = &self.tasks[id];
                let duration = task
                    .duration
                    .or_else(|| task.started.map(|s| elapsed.saturating_sub(s)));
                lines.push(format!(
                    "── {} / {} · {}{} ──",
                    task.target,
                    id,
                    task.state,
                    duration
                        .map(|d| format!(" {:.1}s", d as f64 / 1000.0))
                        .unwrap_or_default()
                ));
                let detail = if !task.reason.is_empty() {
                    task.reason.clone()
                } else if task.state == "waiting" {
                    format!(
                        "Waiting for: {}",
                        if task.dependencies.is_empty() {
                            "execution slot".into()
                        } else {
                            task.dependencies.join(", ")
                        }
                    )
                } else {
                    task.command.clone()
                };
                lines.push(clip(&detail, width, self.horizontal));
                lines.extend(
                    self.history
                        .lines(Some(id), space.saturating_sub(2), self.offset)
                        .into_iter()
                        .map(|s| clip(&s, width, self.horizontal)),
                );
            }
            _ => {
                let active: Vec<_> = self
                    .order
                    .iter()
                    .filter(|id| matches!(self.tasks[*id].state.as_str(), "running" | "collecting"))
                    .take((space / 3).clamp(1, 3))
                    .cloned()
                    .collect();
                if active.is_empty() {
                    lines.push(format!("── {} ──", self.phase));
                    lines.extend(self.history.lines(None, space.saturating_sub(1), 0));
                } else {
                    let per = space / active.len();
                    for id in &active {
                        lines.push(format!("── {id} ──"));
                        lines.extend(self.history.lines(Some(id), per.saturating_sub(1), 0));
                    }
                    if running > active.len() {
                        lines.push(format!(
                            "{} more active; select a task to inspect",
                            running - active.len()
                        ));
                    }
                }
            }
        }
        lines.truncate(height - 1);
        while lines.len() < height - 1 {
            lines.push(String::new());
        }
        lines.push(if self.spool_error {
            "Log view storage failed; retained build logs may still be available".into()
        } else {
            "↑↓ task  Enter expand  a active  l logs  PgUp/PgDn scroll  ←→ pan  f follow  q hide"
                .into()
        });
        lines.into_iter().map(|s| clip(&s, width, 0)).collect()
    }
}

fn clip(text: &str, width: usize, skip: usize) -> String {
    let safe = render::clean(text).replace('\t', "    ");
    let mut position = 0;
    let mut used = 0;
    safe.chars()
        .filter(|&c| {
            let size = c.width().unwrap_or(0);
            position += size;
            if position <= skip {
                return false;
            }
            used += size;
            used < width
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parallel_tasks_keep_independent_panels_and_failure_focus() {
        let mut view = View::new("fixture".into()).unwrap();
        let plan = serde_json::json!({"actions":[{"id":"a:build","target":"a","dependsOn":[]},{"id":"b:build","target":"b","dependsOn":[]}]});
        view.observe(0, "build", &Event::Plan { plan: &plan });
        for id in ["a:build", "b:build"] {
            view.observe(
                0,
                id,
                &Event::Task {
                    state: TaskState::Running,
                    reason: None,
                },
            );
            view.observe(
                1,
                id,
                &Event::Output {
                    stream: "stdout",
                    text: id,
                    continued: false,
                },
            );
        }
        let frame = view.frame(100, 30, 100).join("\n");
        assert!(frame.contains("2 active"));
        assert!(frame.contains("── a:build ──") && frame.contains("── b:build ──"));
        view.observe(
            200,
            "b:build",
            &Event::Finished {
                status: "failed",
                exit_code: Some(7),
                duration_ms: Some(200),
            },
        );
        assert!(view
            .frame(100, 30, 200)
            .join("\n")
            .contains("── b / b:build · failed"));
        assert!(view.frame(45, 12, 200).len() <= 12);
    }
    #[test]
    fn spool_keeps_early_output_and_terminal_control_text_is_inert() {
        let mut view = View::new("fixture\x1b[2J".into()).unwrap();
        for i in 0..5000 {
            view.observe(
                i,
                "a",
                &Event::Progress {
                    message: &format!("line {i}"),
                },
            );
        }
        let first = view.history.lines(None, 2, 4998);
        assert!(first[0].contains("line 0"));
        assert!(!view.frame(80, 24, 5000).join("\n").contains('\x1b'));
    }
}
