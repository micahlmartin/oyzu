//! Follow retained raw files without changing subprocess pipe/buffering semantics.
use super::{Event, Log};
use std::{
    fs::File,
    io::{Read, Result},
    path::Path,
};

pub(crate) struct Follow {
    file: File,
    pending: Vec<u8>,
    stream: &'static str,
    log: Log,
}
impl Follow {
    pub fn open(path: &Path, stream: &'static str, log: &Log) -> Result<Self> {
        Ok(Self {
            file: File::open(path)?,
            pending: Vec::new(),
            stream,
            log: log.clone(),
        })
    }
    pub fn drain(&mut self, finish: bool) -> Result<()> {
        let mut bytes = [0; 8192];
        let mut consumed = 0;
        loop {
            if !finish && consumed >= 65536 {
                break;
            }
            let n = self.file.read(&mut bytes)?;
            if n == 0 {
                break;
            }
            consumed += n;
            self.pending.extend_from_slice(&bytes[..n]);
            while let Some(index) = self.pending.iter().position(|b| *b == b'\n') {
                let line: Vec<_> = self.pending.drain(..=index).collect();
                let text = line.strip_suffix(b"\n").unwrap();
                self.send(text.strip_suffix(b"\r").unwrap_or(text), false);
            }
            if self.pending.len() >= 16384 {
                let line = std::mem::take(&mut self.pending);
                self.send(&line, true);
            }
        }
        if finish && !self.pending.is_empty() {
            let line = std::mem::take(&mut self.pending);
            self.send(&line, false);
        }
        Ok(())
    }
    fn send(&self, bytes: &[u8], continued: bool) {
        self.log.emit(Event::Output {
            stream: self.stream,
            text: &String::from_utf8_lossy(bytes),
            continued,
        });
    }
}
