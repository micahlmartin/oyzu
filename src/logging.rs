//! Invocation-owned progress and multiplexed output. No global logger or builder rules.
mod output;
mod render;
#[cfg(test)]
mod tests;

pub(crate) use output::Follow;
use serde::Serialize;
use std::{
    fs::File,
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone, Copy)]
pub enum Format {
    Text,
    Json,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Event<'a> {
    Progress {
        message: &'a str,
    },
    Command {
        argv: &'a [String],
        cwd: &'a str,
    },
    Output {
        stream: &'a str,
        text: &'a str,
        continued: bool,
    },
    Finished {
        status: &'a str,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
    },
}

struct State {
    sequence: u64,
    console: Option<(Format, Box<dyn Write + Send>)>,
    journal: Option<File>,
}
struct Bus {
    start: Instant,
    state: Mutex<State>,
}

/// Clones share one ordered sink; each scope retains its own target/action label.
/// Callers supply the logger explicitly through lifecycle and execution contracts.
#[derive(Clone)]
pub struct Log {
    bus: Arc<Bus>,
    scope: String,
}

impl Default for Log {
    fn default() -> Self {
        Self::new(None)
    }
}
impl Log {
    fn new(console: Option<(Format, Box<dyn Write + Send>)>) -> Self {
        Self {
            bus: Arc::new(Bus {
                start: Instant::now(),
                state: Mutex::new(State {
                    sequence: 0,
                    console,
                    journal: None,
                }),
            }),
            scope: "build".into(),
        }
    }
    /// Live logs use stderr; stdout is reserved for the final human or JSON result.
    pub fn console(format: Format) -> Self {
        Self::new(Some((format, Box::new(std::io::stderr()))))
    }
    pub fn scope(&self, scope: impl Into<String>) -> Self {
        Self {
            bus: self.bus.clone(),
            scope: scope.into(),
        }
    }
    pub(crate) fn journal(&self, path: &Path) -> std::io::Result<()> {
        self.bus.state.lock().unwrap().journal = Some(File::create(path)?);
        Ok(())
    }
    pub(crate) fn close_journal(&self) {
        self.bus.state.lock().unwrap().journal = None;
    }
    pub fn progress(&self, message: &str) {
        self.emit(Event::Progress { message });
    }
    pub(crate) fn command(&self, argv: &[String], cwd: &str) {
        self.emit(Event::Command { argv, cwd });
    }
    pub(crate) fn finished(&self, status: &str, exit_code: Option<i32>, duration_ms: Option<u64>) {
        self.emit(Event::Finished {
            status,
            exit_code,
            duration_ms,
        });
    }
    pub(crate) fn active(&self) -> bool {
        let state = self.bus.state.lock().unwrap();
        state.console.is_some() || state.journal.is_some()
    }
    pub fn emit(&self, event: Event<'_>) {
        let mut state = self.bus.state.lock().unwrap();
        state.sequence += 1;
        let elapsed = self.bus.start.elapsed().as_millis() as u64;
        let record = serde_json::json!({"schemaVersion":"v1alpha1", "sequence":state.sequence, "elapsedMs":elapsed, "scope":self.scope, "event":event});
        let json = serde_json::to_string(&record).unwrap();
        if let Some(file) = &mut state.journal {
            let _ = writeln!(file, "{json}");
        }
        if let Some((format, writer)) = &mut state.console {
            let line = match format {
                Format::Json => json,
                Format::Text => render::line(elapsed, &self.scope, &event),
            };
            let _ = writeln!(writer, "{line}");
            let _ = writer.flush();
        }
    }
}
