//! Append-only presentation. Parallel tasks never open overlapping CI groups.
use super::{render, Event, TaskState};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Timeline {
    tasks: BTreeMap<String, &'static str>,
    pub github: bool,
    group: bool,
}
impl Timeline {
    pub fn close(&mut self) -> &'static str {
        if std::mem::take(&mut self.group) {
            "::endgroup::\n"
        } else {
            ""
        }
    }
    pub fn heartbeat(&self) -> Option<String> {
        if self.tasks.is_empty() {
            return None;
        }
        let running = self
            .tasks
            .iter()
            .filter(|(_, state)| **state == "running")
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>();
        let completed = self
            .tasks
            .values()
            .filter(|s| matches!(**s, "passed" | "failed" | "blocked"))
            .count();
        if completed == self.tasks.len() {
            return None;
        }
        Some(format!(
            "RUNNING {} | {completed}/{} tasks finished",
            running.join(", "),
            self.tasks.len()
        ))
    }
    pub fn event(&mut self, elapsed: u64, scope: &str, event: &Event<'_>) -> String {
        let time = format!(
            "{:02}:{:04.1}",
            elapsed / 60000,
            (elapsed % 60000) as f64 / 1000.0
        );
        match event {
            Event::Phase { name } => {
                let close = self.close();
                self.group = self.github && matches!(*name, "Preflight" | "Dependencies" | "Plan");
                format!(
                    "{close}{}\n{time}  PHASE  {}",
                    if self.group {
                        format!("::group::Oyzu {}", render::clean(name))
                    } else {
                        String::new()
                    },
                    render::clean(name)
                )
            }
            Event::Plan { plan } => {
                let mut lines = vec!["\nBUILD PLAN\nTARGET / BUILDER / TASKS".to_string()];
                if let Some(targets) = plan["targets"].as_array() {
                    for target in targets {
                        let id = target["id"].as_str().unwrap_or("?");
                        let actions: Vec<_> = plan["actions"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|a| a["target"] == id)
                            .collect();
                        for a in &actions {
                            self.tasks
                                .insert(a["id"].as_str().unwrap_or("?").into(), "waiting");
                        }
                        lines.push(render::clean(&format!(
                            "{} / {} / {}",
                            id,
                            target["builder"].as_str().unwrap_or("?"),
                            actions
                                .iter()
                                .map(|a| a["id"].as_str().unwrap_or("?"))
                                .collect::<Vec<_>>()
                                .join(" -> ")
                        )));
                    }
                }
                lines.join("\n")
            }
            Event::Task { state, reason } => {
                let label = match state {
                    TaskState::Running => "START",
                    TaskState::Blocked => "BLOCKED",
                };
                self.tasks.insert(
                    scope.into(),
                    if *state == TaskState::Running {
                        "running"
                    } else {
                        "blocked"
                    },
                );
                format!(
                    "\n{time}  {label:<7} {}{}",
                    render::clean(scope),
                    reason
                        .map(|r| format!("\n         {}", render::clean(r)))
                        .unwrap_or_default()
                )
            }
            Event::Finished {
                status,
                exit_code,
                duration_ms,
            } if self.tasks.contains_key(scope) && matches!(*status, "succeeded" | "failed") => {
                self.tasks.insert(
                    scope.into(),
                    if *status == "succeeded" {
                        "passed"
                    } else {
                        "failed"
                    },
                );
                format!(
                    "{time}  {:<7} {}{}{}",
                    if *status == "succeeded" {
                        "PASS"
                    } else {
                        "FAIL"
                    },
                    render::clean(scope),
                    duration_ms
                        .map(|d| format!("  {:.2}s", d as f64 / 1000.0))
                        .unwrap_or_default(),
                    exit_code
                        .filter(|c| *c != 0)
                        .map(|c| format!(" (exit {c})"))
                        .unwrap_or_default()
                )
            }
            _ => render::line(elapsed, scope, event),
        }
    }
}
