//! Terminal lifecycle and input belong to presentation, never to build execution.
use super::{dashboard::View, Log};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::Print,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, IsTerminal, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// Invocation guard restores the terminal before the final receipt or an error.
pub struct Terminal {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
struct Screen;
impl Screen {
    fn open() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let screen = Self;
        execute!(io::stderr(), EnterAlternateScreen, Hide)?;
        Ok(screen)
    }
}
impl Drop for Screen {
    fn drop(&mut self) {
        let _ = execute!(io::stderr(), Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}
impl Terminal {
    pub fn available() -> bool {
        io::stdin().is_terminal()
            && io::stderr().is_terminal()
            && !std::env::var("TERM").is_ok_and(|v| v == "dumb")
    }
    /// Explicit interactive selection requires a terminal. Auto callers choose a
    /// plain view in CI, redirected streams or unsupported terminals.
    pub fn start(log: &Log, interactive: bool, title: String) -> io::Result<Self> {
        let mut screen = if interactive {
            let view = View::new(title)?;
            let screen = Screen::open()?;
            log.bus.state.lock().unwrap().view = Some(view);
            Some(screen)
        } else {
            None
        };
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let log = log.clone();
        let worker = thread::spawn(move || {
            let mut last_heartbeat = Instant::now();
            let mut previous = Vec::new();
            let mut dimensions = (0, 0);
            while !flag.load(Ordering::Relaxed) {
                if screen.is_some() {
                    let tick_started = Instant::now();
                    let tick = (|| -> io::Result<bool> {
                        if event::poll(Duration::from_millis(100))? {
                            if let Event::Key(key) = event::read()? {
                                if key.kind != KeyEventKind::Release {
                                    if key.code == KeyCode::Char('c')
                                        && key.modifiers.contains(KeyModifiers::CONTROL)
                                    {
                                        // Restore terminal semantics before the same abrupt
                                        // interruption the CLI had before interactive mode.
                                        drop(screen.take());
                                        std::process::exit(130);
                                    }
                                    if key.code == KeyCode::Char('q') {
                                        return Ok(false);
                                    }
                                    if let Some(view) = &mut log.bus.state.lock().unwrap().view {
                                        view.key(key.code);
                                    }
                                }
                            }
                        }
                        let (width, height) = terminal::size()?;
                        let lines = log
                            .bus
                            .state
                            .lock()
                            .unwrap()
                            .view
                            .as_mut()
                            .map(|v| {
                                v.frame(
                                    width as usize,
                                    height as usize,
                                    log.bus.start.elapsed().as_millis() as u64,
                                )
                            })
                            .unwrap_or_default();
                        let mut output = io::stderr();
                        if dimensions != (width, height) {
                            queue!(output, Clear(ClearType::All))?;
                            previous.clear();
                            dimensions = (width, height);
                        }
                        for (row, line) in lines.iter().enumerate() {
                            if previous.get(row) == Some(line) {
                                continue;
                            }
                            queue!(
                                output,
                                MoveTo(0, row as u16),
                                Clear(ClearType::CurrentLine),
                                Print(line)
                            )?;
                        }
                        output.flush()?;
                        previous = lines;
                        Ok(true)
                    })();
                    if !matches!(tick, Ok(true)) {
                        drop(screen.take());
                        log.bus.state.lock().unwrap().view = None;
                        log.progress("Terminal view closed; build continues with plain logs. Full output is retained in dist/logs.");
                    }
                    thread::sleep(
                        Duration::from_millis(100).saturating_sub(tick_started.elapsed()),
                    );
                } else {
                    thread::sleep(Duration::from_millis(100));
                    if last_heartbeat.elapsed() >= Duration::from_secs(10) {
                        let status = log.bus.state.lock().unwrap().timeline.heartbeat();
                        if let Some(status) = status {
                            log.scope("execute").progress(&status);
                        }
                        last_heartbeat = Instant::now();
                    }
                }
            }
            drop(screen.take());
            log.bus.state.lock().unwrap().view = None;
        });
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
