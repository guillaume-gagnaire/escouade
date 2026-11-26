//! The outputs of JSON lines (`docs/TEAM-AUDIT.md` §4.2): a file turned by size and by day, or the standard output
//! (Docker, journald). syslog is the third (`syslog.rs`).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::words::Time;

/// Where an event's line is written. A failure leaves the line to the caller (the outbox keeps it).
pub trait Sink {
    fn send(&mut self, line: &str) -> io::Result<()>;
}

/// JSON lines on a stream (the standard output), a line flushed at a time.
pub struct Lines<W: Write> {
    out: W,
}

impl<W: Write> Lines<W> {
    pub fn new(out: W) -> Lines<W> {
        Lines { out }
    }
    pub fn into_inner(self) -> W {
        self.out
    }
}

impl<W: Write> Sink for Lines<W> {
    fn send(&mut self, line: &str) -> io::Result<()> {
        self.out.write_all(line.as_bytes())?;
        self.out.write_all(b"\n")?;
        self.out.flush()
    }
}

/// JSON lines in a file, `<stem>.jsonl`, turned when it would pass its size or when the day changes (UTC, the day an
/// event was seen by its emitter, `seen`: the same after a restart or a buffer sent late): the file is renamed `<stem>-<day>.<n>.jsonl` (its day, the first free number) and a new one begins. An agent of the SIEM
/// follows `<stem>*.jsonl`.
pub struct RotatingFile {
    dir: PathBuf,
    stem: String,
    max_bytes: u64,
    file: Option<(File, u64, String)>,
}

/// When an event's line says it was seen.
fn seen_of(line: &str) -> Option<Time> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    Time::parse(v["seen"].as_str()?).ok()
}

/// 100 MiB, the size a file is turned at by default.
pub const DEFAULT_FILE_BYTES: u64 = 100 * 1024 * 1024;

impl RotatingFile {
    /// The file `<dir>/<stem>.jsonl`, turned past `max_bytes`.
    pub fn new(dir: impl Into<PathBuf>, stem: &str, max_bytes: u64) -> RotatingFile {
        RotatingFile {
            dir: dir.into(),
            stem: stem.to_owned(),
            max_bytes: max_bytes.max(1),
            file: None,
        }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(format!("{}.jsonl", self.stem))
    }

    /// The day of a file found at the start: its last event's, else its last change's.
    fn day_of(path: &Path) -> Option<String> {
        let text = fs::read_to_string(path).ok()?;
        if let Some(seen) = text
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .and_then(seen_of)
        {
            return Some(seen.day());
        }
        let modified = fs::metadata(path).ok()?.modified().ok()?;
        let ms = modified
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as i64;
        Some(Time::from_millis(ms).day())
    }

    fn turn(&mut self, day: &str) -> io::Result<()> {
        self.file = None;
        let current = self.path();
        if !current.exists() {
            return Ok(());
        }
        let mut n = 1;
        loop {
            let to = self.dir.join(format!("{}-{day}.{n}.jsonl", self.stem));
            if !to.exists() {
                return fs::rename(&current, &to);
            }
            n += 1;
        }
    }

    fn open(&mut self, today: &str) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.path();
        if let Some(day) = Self::day_of(&path) {
            if day != today && fs::metadata(&path)?.len() > 0 {
                self.turn(&day)?;
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let size = file.metadata()?.len();
        self.file = Some((file, size, today.to_owned()));
        Ok(())
    }
}

impl Sink for RotatingFile {
    fn send(&mut self, line: &str) -> io::Result<()> {
        let today = seen_of(line).unwrap_or_else(Time::now).day();
        let bytes = line.len() as u64 + 1;
        if let Some((_, size, day)) = &self.file {
            if *day != today || (*size > 0 && size + bytes > self.max_bytes) {
                let day = day.clone();
                self.turn(&day)?;
            }
        }
        if self.file.is_none() {
            self.open(&today)?;
            if let Some((_, size, _)) = &self.file {
                if *size > 0 && size + bytes > self.max_bytes {
                    self.turn(&today)?;
                    self.open(&today)?;
                }
            }
        }
        let (file, size, _) = self.file.as_mut().expect("opened");
        let mut out = Vec::with_capacity(line.len() + 1);
        out.extend_from_slice(line.as_bytes());
        out.push(b'\n');
        file.write_all(&out)?;
        file.flush()?;
        *size += bytes;
        Ok(())
    }
}
