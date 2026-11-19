//! A plugin's journal (PLG-2, ESC-174): its starts, stops and crashes, what it writes on its
//! standard error, the messages it sends (`log`, its requests), the calls that ran out of time and
//! what it was refused. The last entries are kept in memory for Réglages › Plugins (PLG-4), and
//! all of them in a file of the app's data (`DataDir::plugin_logs_dir`), started again past a size.
//!
//! The secrets it was given (`secrets/get`) never reach the journal: each is masked wherever it
//! shows, its standard error included.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;

/// How many entries stay in memory.
pub const KEPT: usize = 500;
/// Past this size, the file is moved to `<id>.log.1` and started again.
const FILE_MAX: u64 = 1 << 20;
/// How long an entry's text may be, in characters.
const TEXT_MAX: usize = 4000;
/// What stands for a secret in the journal.
pub const MASK: &str = "••••";
/// A secret shorter than this is not masked: it would mask everything.
const MASK_MIN: usize = 4;
/// A line of a secret of several lines longer than this is masked on its own too (§4.4 of the study, ESC-527).
const LINE_MIN: usize = 8;

/// What an entry says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// Its process started.
    Start,
    /// It answered `initialize`.
    Ready,
    /// Its process was stopped, and why.
    Stop,
    /// Its process ended without being asked to.
    Crash,
    /// A line of its standard error.
    Stderr,
    /// A message it sent: a `log`, a request.
    Message,
    /// A call it did not answer in time: the default answer applied.
    Timeout,
    /// Something it asked for and did not get: another's secret, a method Escouade does not serve.
    Refused,
    /// A message dropped from the queue kept while it restarts, or a fact from its full queue.
    Dropped,
    /// A webhook called on a fact, and its answer (PLG-5).
    Webhook,
    /// A command run on a fact, and how it ended (PLG-5).
    Command,
    /// A permission it was asked to decide, and what it answered (PLG-8).
    Decision,
}

/// An entry of the journal.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    /// When, in RFC 3339.
    pub at: String,
    pub kind: Kind,
    pub text: String,
}

/// The journal of a plugin.
pub struct Journal {
    file: PathBuf,
    entries: Mutex<VecDeque<Entry>>,
    masked: Mutex<Vec<String>>,
    /// Its host forgotten: what its last process still says no longer reaches the file.
    closed: std::sync::atomic::AtomicBool,
}

impl Journal {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            entries: Mutex::new(VecDeque::new()),
            masked: Mutex::new(Vec::new()),
            closed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// From now on, `secret` shows as `MASK`.
    /// A secret of several lines (a configuration with its certificates, ESC-527) has each of its lines of more than
    /// `LINE_MIN` characters masked too: a plugin that writes one line of it to its journal does not show it.
    pub fn mask(&self, secret: &str) {
        if secret.chars().count() < MASK_MIN {
            return;
        }
        let mut masked = self.masked.lock();
        let lines = secret
            .contains('\n')
            .then(|| {
                secret
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| l.chars().count() > LINE_MIN)
            })
            .into_iter()
            .flatten();
        for s in std::iter::once(secret).chain(lines) {
            if !masked.iter().any(|m| m == s) {
                masked.push(s.to_string());
            }
        }
        // The longest first: one secret inside another is masked whole.
        masked.sort_by_key(|s| std::cmp::Reverse(s.len()));
    }

    /// Notes `text`, masked and cut to its length.
    pub fn push(&self, kind: Kind, text: impl Into<String>) {
        let mut text: String = text.into();
        for s in self.masked.lock().iter() {
            if text.contains(s.as_str()) {
                text = text.replace(s.as_str(), MASK);
            }
        }
        if text.chars().count() > TEXT_MAX {
            text = text.chars().take(TEXT_MAX).collect::<String>() + "…";
        }
        let entry = Entry {
            at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            kind,
            text,
        };
        self.append(&entry);
        let mut entries = self.entries.lock();
        entries.push_back(entry);
        while entries.len() > KEPT {
            entries.pop_front();
        }
    }

    /// The entries kept in memory, the oldest first.
    pub fn entries(&self) -> Vec<Entry> {
        self.entries.lock().iter().cloned().collect()
    }

    /// From now on, nothing more is written to its file (its plugin uninstalled, or another host
    /// of it writes there).
    pub fn close(&self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Release);
    }

    fn append(&self, e: &Entry) {
        if self.closed.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }
        // Its file: `<at> <kind> <text>`, a line each (`read_file`).
        let write = || -> std::io::Result<()> {
            if let Some(dir) = self.file.parent() {
                std::fs::create_dir_all(dir)?;
            }
            if std::fs::metadata(&self.file).is_ok_and(|m| m.len() > FILE_MAX) {
                let mut old = self.file.clone().into_os_string();
                old.push(".1");
                std::fs::rename(&self.file, old)?;
            }
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file)?;
            let kind = serde_json::to_value(e.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            writeln!(f, "{} {kind} {}", e.at, e.text.replace('\n', " "))
        };
        if let Err(err) = write() {
            log::warn!("plugin journal {}: {err}", self.file.display());
        }
    }
}

/// The last `KEPT` entries of a journal's file (and of the one it was moved to, past its size), the
/// oldest first; a line that does not read is left out.
pub fn read_file(file: &std::path::Path) -> Vec<Entry> {
    let mut old = file.to_path_buf().into_os_string();
    old.push(".1");
    let mut entries: VecDeque<Entry> = VecDeque::new();
    for f in [PathBuf::from(old), file.to_path_buf()] {
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
        for line in text.lines() {
            let mut parts = line.splitn(3, ' ');
            let (Some(at), Some(kind)) = (parts.next(), parts.next()) else {
                continue;
            };
            let Ok(kind) = serde_json::from_value::<Kind>(serde_json::Value::String(kind.into()))
            else {
                continue;
            };
            entries.push_back(Entry {
                at: at.to_string(),
                kind,
                text: parts.next().unwrap_or("").to_string(),
            });
            if entries.len() > KEPT {
                entries.pop_front();
            }
        }
    }
    entries.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal() -> Journal {
        let dir = std::env::temp_dir().join(format!(
            "escouade-journal-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Journal::new(dir.join("journal.jsonl"))
    }

    #[test]
    fn a_secret_of_several_lines_has_each_long_line_masked_too() {
        let j = journal();
        let config =
            "client\r\nremote vpn.acme.dev 1194\n<ca>\nMIIDdzCCAl+gAwIBAgIUP1c2xQ8\n</ca>\n";
        j.mask(config);
        j.push(Kind::Message, format!("whole: {config}"));
        j.push(Kind::Message, "line: MIIDdzCCAl+gAwIBAgIUP1c2xQ8 end");
        j.push(Kind::Message, "line: remote vpn.acme.dev 1194");
        // Eight characters or fewer: left, or everything would be masked.
        j.push(Kind::Message, "short: client </ca>");
        let texts: Vec<String> = j.entries().into_iter().map(|e| e.text).collect();
        assert_eq!(
            texts,
            [
                format!("whole: {MASK}"),
                format!("line: {MASK} end"),
                format!("line: {MASK}"),
                "short: client </ca>".to_string(),
            ]
        );
    }

    #[test]
    fn a_secret_of_one_line_is_masked_whole_only() {
        let j = journal();
        j.mask("s3cr3t-token-value");
        j.push(Kind::Message, "got s3cr3t-token-value and s3cr3t");
        assert_eq!(j.entries()[0].text, format!("got {MASK} and s3cr3t"));
    }

    /// ESC-526: written line by line on the journal, as the page of a plugin's configuration is, each long line masked;
    /// a short line alone stays.
    #[test]
    fn a_configuration_written_line_by_line_shows_none_of_its_long_lines() {
        let j = journal();
        let conf = "client\nremote vpn.acme.example 1194\n  <key>MIIEvQIBADANBg</key>\n";
        j.mask(conf);
        j.push(Kind::Stderr, format!("the secret is {conf}"));
        j.push(Kind::Stderr, "remote vpn.acme.example 1194");
        j.push(Kind::Stderr, "<key>MIIEvQIBADANBg</key>");
        j.push(Kind::Stderr, "client");
        let said: Vec<String> = j.entries().into_iter().map(|e| e.text).collect();
        assert_eq!(
            said,
            [
                format!("the secret is {MASK}"),
                MASK.into(),
                MASK.into(),
                "client".into()
            ]
        );
    }
}
