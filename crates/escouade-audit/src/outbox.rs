//! The buffer on the disk (`docs/TEAM-AUDIT.md` §4.4): every sealed event is written here first, then sent. When the
//! output does not answer (syslog down), the events wait, 7 days or 1 GiB by default; past it the oldest drop and an
//! `audit.dropped` says how many, of when to when. The emitter's chain is kept beside them, so that it takes up at the
//! position of the last event written, without a gap nor a double of its own.
//!
//! On the disk, in its folder: `spool/<n>.jsonl`, segments of lines in order; `cursor.json`, the first line not sent
//! (its segment, its offset); `chain.json`, the emitter's [`Chain`].

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::{Dropped, Fact, Nothing};
use crate::record::{Chain, Draft, Emitter, SealError, Sealed};
use crate::sink::Sink;
use crate::words::Time;

/// The limits of the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// How long an event not sent waits, in milliseconds (7 days).
    pub max_age_ms: i64,
    /// How many bytes not sent wait (1 GiB).
    pub max_bytes: u64,
    /// The size a segment is closed at (16 MiB).
    pub segment_bytes: u64,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_age_ms: 7 * 24 * 3600 * 1000,
            max_bytes: 1024 * 1024 * 1024,
            segment_bytes: 16 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    segment: u64,
    offset: u64,
}

/// The buffer of an emitter.
pub struct Outbox {
    dir: PathBuf,
    limits: Limits,
    cursor: Cursor,
}

/// Why an event was not emitted.
#[derive(Debug, thiserror::Error)]
pub enum EmitError {
    #[error(transparent)]
    Seal(#[from] SealError),
    #[error("the audit's buffer: {0}")]
    Io(#[from] io::Error),
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_data()?;
    }
    fs::rename(&tmp, path)
}

impl Outbox {
    /// The buffer in `dir`, made if need be.
    pub fn open(dir: impl Into<PathBuf>, limits: Limits) -> io::Result<Outbox> {
        let dir = dir.into();
        fs::create_dir_all(dir.join("spool"))?;
        let cursor = match fs::read(dir.join("cursor.json")) {
            Ok(b) => serde_json::from_slice(&b)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Cursor::default(),
            Err(e) => return Err(e),
        };
        let mut o = Outbox {
            dir,
            limits,
            cursor,
        };
        if let Some(first) = o.segments()?.first() {
            if o.cursor.segment < *first {
                o.cursor = Cursor {
                    segment: *first,
                    offset: 0,
                };
            }
        }
        Ok(o)
    }

    fn segment_path(&self, n: u64) -> PathBuf {
        self.dir.join("spool").join(format!("{n:016}.jsonl"))
    }

    /// The segments, in order.
    fn segments(&self) -> io::Result<Vec<u64>> {
        let mut out: Vec<u64> = fs::read_dir(self.dir.join("spool"))?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                e.file_name()
                    .to_str()
                    .and_then(|n| n.strip_suffix(".jsonl"))
                    .and_then(|n| n.parse().ok())
            })
            .collect();
        out.sort_unstable();
        Ok(out)
    }

    /// The emitter's chain as the disk keeps it: `chain.json`, or later the last line of the buffer when the process
    /// stopped between the two writes.
    pub fn chain(&self) -> io::Result<Chain> {
        let mut chain: Chain = match fs::read(self.dir.join("chain.json")) {
            Ok(b) => serde_json::from_slice(&b)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Chain::default(),
            Err(e) => return Err(e),
        };
        if let Some(last) = self.segments()?.last() {
            let mut text = String::new();
            File::open(self.segment_path(*last))?.read_to_string(&mut text)?;
            if let Some(line) = text.lines().rev().find(|l| !l.trim().is_empty()) {
                if let Ok(v) = serde_json::from_str::<Value>(line) {
                    if v.pointer("/emitter/seq").and_then(Value::as_u64) > Some(chain.seq) {
                        if let Some(after) = chain.after(&v) {
                            chain = after;
                        }
                    }
                }
            }
        }
        Ok(chain)
    }

    /// Writes a sealed event at the end of the buffer (on the disk), then the emitter's chain after it.
    pub fn push(&mut self, sealed: &Sealed, chain: &Chain) -> io::Result<()> {
        let segments = self.segments()?;
        let mut n = segments.last().copied().unwrap_or(self.cursor.segment);
        let path = self.segment_path(n);
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size > 0 && size + sealed.line.len() as u64 + 1 > self.limits.segment_bytes {
            n += 1;
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.segment_path(n))?;
        let mut bytes = sealed.line.clone().into_bytes();
        bytes.push(b'\n');
        f.write_all(&bytes)?;
        f.sync_data()?;
        write_atomic(
            &self.dir.join("chain.json"),
            &serde_json::to_vec(chain).expect("a chain is JSON"),
        )
    }

    /// The lines not sent, in order, each with the cursor after it.
    fn pending(&self) -> io::Result<Vec<(String, Cursor)>> {
        let mut out = Vec::new();
        for n in self.segments()? {
            if n < self.cursor.segment {
                continue;
            }
            let mut f = File::open(self.segment_path(n))?;
            let mut offset = if n == self.cursor.segment {
                self.cursor.offset
            } else {
                0
            };
            f.seek(SeekFrom::Start(offset))?;
            let mut reader = BufReader::new(f);
            let mut line = String::new();
            loop {
                line.clear();
                let read = reader.read_line(&mut line)?;
                if read == 0 || !line.ends_with('\n') {
                    break;
                }
                offset += read as u64;
                out.push((line.trim_end().to_owned(), Cursor { segment: n, offset }));
            }
        }
        Ok(out)
    }

    /// How many events wait.
    pub fn waiting(&self) -> io::Result<usize> {
        Ok(self.pending()?.len())
    }

    fn advance(&mut self, to: Cursor) -> io::Result<()> {
        self.cursor = to;
        write_atomic(
            &self.dir.join("cursor.json"),
            &serde_json::to_vec(&self.cursor).expect("a cursor is JSON"),
        )?;
        // The segments wholly sent go; the last stays for the next lines.
        let segments = self.segments()?;
        let last = segments.last().copied();
        for n in segments {
            let whole = n < self.cursor.segment
                || (n == self.cursor.segment
                    && Some(n) != last
                    && fs::metadata(self.segment_path(n))?.len() <= self.cursor.offset);
            if whole {
                fs::remove_file(self.segment_path(n))?;
            }
        }
        if let Some(next) = self.segments()?.first() {
            if *next > self.cursor.segment {
                self.cursor = Cursor {
                    segment: *next,
                    offset: 0,
                };
            }
        }
        Ok(())
    }

    /// Sends what waits to `sink`, in order, each line marked sent once written; stops at the first failure (the
    /// rest waits). Answers how many went.
    pub fn drain(&mut self, sink: &mut dyn Sink) -> io::Result<usize> {
        let mut sent = 0;
        for (line, after) in self.pending()? {
            sink.send(&line)?;
            self.advance(after)?;
            sent += 1;
        }
        Ok(sent)
    }

    /// Drops the oldest events that wait past the limits at `now`: older than its age, or past its bytes. Says what it
    /// dropped (for `audit.dropped`), none when nothing.
    pub fn trim(&mut self, now: Time) -> io::Result<Option<Dropped>> {
        let pending = self.pending()?;
        let mut bytes: u64 = pending.iter().map(|(l, _)| l.len() as u64 + 1).sum();
        let mut dropped: Option<Dropped> = None;
        let mut to = None;
        for (line, after) in &pending {
            let v: Value = serde_json::from_str(line).unwrap_or(Value::Null);
            let seen = v["seen"]
                .as_str()
                .and_then(|s| Time::parse(s).ok())
                .unwrap_or(Time::from_millis(i64::MIN / 2));
            let old = now.millis() - seen.millis() > self.limits.max_age_ms;
            if !old && bytes <= self.limits.max_bytes {
                break;
            }
            bytes -= line.len() as u64 + 1;
            let at = v["at"]
                .as_str()
                .and_then(|s| Time::parse(s).ok())
                .unwrap_or(seen);
            let seq = v
                .pointer("/emitter/seq")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let mut this = Dropped {
                count: 1,
                from: at,
                to: at,
                first: seq,
                last: seq,
            };
            // An `audit.dropped` that drops in turn: what it said goes on in the next.
            if v["event"] == "audit.dropped" {
                if let Ok(said) = serde_json::from_value::<Dropped>(v["detail"].clone()) {
                    this = merge(this, said);
                }
            }
            dropped = Some(match dropped {
                None => this,
                Some(d) => merge(d, this),
            });
            to = Some(*after);
        }
        if let Some(after) = to {
            self.advance(after)?;
        }
        Ok(dropped)
    }
}

fn merge(a: Dropped, b: Dropped) -> Dropped {
    Dropped {
        count: a.count + b.count,
        from: a.from.min(b.from),
        to: a.to.max(b.to),
        first: a.first.min(b.first),
        last: a.last.max(b.last),
    }
}

/// An emitter and its buffer, writing to one output.
pub struct Audit<S: Sink> {
    pub emitter: Emitter,
    pub outbox: Outbox,
    pub sink: S,
}

impl<S: Sink> Audit<S> {
    /// Seals `draft`, keeps it on the disk, and sends what waits. When the output does not answer, the buffer is
    /// trimmed to its limits, and what it dropped is said by an `audit.dropped`, kept to go first when the output
    /// answers again. Answers the sealed event.
    pub fn emit(&mut self, draft: Draft, now: Time) -> Result<Sealed, EmitError> {
        let sealed = self.emitter.seal(draft, now)?;
        self.outbox.push(&sealed, self.emitter.chain())?;
        if self.outbox.drain(&mut self.sink).is_err() {
            if let Some(d) = self.outbox.trim(now)? {
                let said = self.emitter.seal(
                    Draft {
                        fact: Fact::AuditDropped {
                            actor: Default::default(),
                            target: Nothing {},
                            detail: d,
                        },
                        at: now,
                        anchor: None,
                    },
                    now,
                )?;
                self.outbox.push(&said, self.emitter.chain())?;
            }
        }
        Ok(sealed)
    }

    /// Sends what waits (when the output is back), answers how many went.
    pub fn flush(&mut self) -> io::Result<usize> {
        self.outbox.drain(&mut self.sink)
    }
}
