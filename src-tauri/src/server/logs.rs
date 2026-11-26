//! The log of `escouade-server` (ESC-514): each line on its standard output, for systemd's journal and Docker's logs,
//! and in `server.log` in its data folder, which turns at 5 MB (`server.1.log`, `server.2.log`, `server.3.log`, the
//! oldest dropped). What it is told never holds a code, a token nor a secret (the remote API, the pairing and the
//! devices write none: `remoteapi::server`).

use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The file's name, in the data folder.
pub const FILE: &str = "server.log";

/// The size past which the file turns.
pub const TURN_AT: u64 = 5_000_000;

/// The older files kept.
pub const KEPT: usize = 3;

/// The mode of the log's files under Unix: its owner alone reads and writes them (SR1-13).
#[cfg(unix)]
pub const PRIVATE_FILE_MODE: u32 = 0o600;

struct Sink {
    path: PathBuf,
    file: Option<File>,
    written: u64,
    turn_at: u64,
}

impl Sink {
    /// The file opened to append, readable by its owner alone under Unix (SR1-13: it names devices and projects): made
    /// in mode 0600 whatever the `umask`, and one an earlier version made with it put back in 0600.
    fn open(path: &Path) -> Option<File> {
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            options.mode(PRIVATE_FILE_MODE);
            let file = options.open(path).ok()?;
            let mode = file.metadata().ok()?.permissions().mode() & 0o777;
            if mode != PRIVATE_FILE_MODE {
                let _ = file.set_permissions(std::fs::Permissions::from_mode(PRIVATE_FILE_MODE));
            }
            Some(file)
        }
        #[cfg(not(unix))]
        options.open(path).ok()
    }

    /// `server.log` → `server.1.log` → … → `server.<KEPT>.log`, the oldest dropped; a new one opened.
    fn turn(&mut self) {
        self.file = None;
        let numbered = |n: usize| self.path.with_extension(format!("{n}.log"));
        let _ = std::fs::remove_file(numbered(KEPT));
        for n in (1..KEPT).rev() {
            let _ = std::fs::rename(numbered(n), numbered(n + 1));
        }
        let _ = std::fs::rename(&self.path, numbered(1));
        self.file = Self::open(&self.path);
        self.written = 0;
    }

    fn write(&mut self, line: &str) {
        if self.written + line.len() as u64 > self.turn_at && self.written > 0 {
            self.turn();
        }
        if let Some(f) = self.file.as_mut() {
            if f.write_all(line.as_bytes()).is_ok() {
                self.written += line.len() as u64;
            }
        }
    }
}

/// The server's logger.
pub struct Logger {
    sink: Mutex<Sink>,
    debug: bool,
    stdout: bool,
}

impl Logger {
    /// The log in `<dir>/server.log`, turning at `turn_at` bytes; `debug`: Escouade's own debug lines too; `stdout`:
    /// each line on the standard output as well.
    pub fn new(dir: &Path, turn_at: u64, debug: bool, stdout: bool) -> Self {
        let path = dir.join(FILE);
        // The older files an earlier version made with the `umask`, put back in 0600 (SR1-13).
        #[cfg(unix)]
        for n in 1..=KEPT {
            use std::os::unix::fs::PermissionsExt;
            let old = path.with_extension(format!("{n}.log"));
            if std::fs::symlink_metadata(&old).is_ok_and(|m| m.is_file()) {
                let _ = std::fs::set_permissions(
                    &old,
                    std::fs::Permissions::from_mode(PRIVATE_FILE_MODE),
                );
            }
        }
        let written = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        Self {
            sink: Mutex::new(Sink {
                file: Sink::open(&path),
                path,
                written,
                turn_at,
            }),
            debug,
            stdout,
        }
    }

    /// One line, as it is written: its time, its level, its text.
    pub fn line(level: log::Level, text: &str) -> String {
        format!(
            "{} {:<5} {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            level,
            text
        )
    }

    pub fn write(&self, level: log::Level, text: &str) {
        let line = Self::line(level, text);
        if self.stdout {
            let _ = std::io::stdout().lock().write_all(line.as_bytes());
        }
        self.sink.lock().write(&line);
    }
}

impl log::Log for Logger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        // whisper.cpp tells every step of a model's loading: its warnings and errors alone (as the app's log).
        if m.target().starts_with("whisper_rs") {
            return m.level() <= log::Level::Warn;
        }
        m.level() <= log::Level::Info || (self.debug && m.target().starts_with("escouade"))
    }

    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            self.write(r.level(), &r.args().to_string());
        }
    }

    fn flush(&self) {
        let _ = std::io::stdout().flush();
        if let Some(f) = self.sink.lock().file.as_mut() {
            let _ = f.flush();
        }
    }
}

/// The server's logger made the process's.
pub fn install(dir: &Path, debug: bool) {
    let logger = Box::leak(Box::new(Logger::new(dir, TURN_AT, debug, true)));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(log::LevelFilter::Debug);
    }
}
