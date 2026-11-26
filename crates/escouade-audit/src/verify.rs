//! What `escouade-audit verify` checks of an export (`docs/TEAM-AUDIT.md` §5.4): each line of the model, each signature
//! against the emitters' keys, each chain without a gap nor a fork (a gap an `audit.dropped` says is not one), each
//! checkpoint against the chain, and the place of the cross-check of S1 and S2 (§5.3), which AU-5 fills.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use ed25519_dalek::VerifyingKey;
use serde::Deserialize;
use serde_json::Value;

use crate::event::Fact;
use crate::record::{chain_digest, signature_holds, Record};
use crate::words::{Digest, PublicKey};

/// The emitters' keys (`--keys`): `{"keys": {"<device>": "ed25519:<base64>"}}`, the list the audit machine exports
/// from the members' journal (AU-5).
#[derive(Debug, Clone, Default)]
pub struct Keys(pub(crate) BTreeMap<String, VerifyingKey>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeysFile {
    keys: BTreeMap<String, PublicKey>,
}

impl Keys {
    pub fn parse(json: &str) -> Result<Keys, String> {
        let f: KeysFile = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let mut out = BTreeMap::new();
        for (device, key) in f.keys {
            let k = key
                .key()
                .ok_or_else(|| format!("the key of {device} is not an Ed25519 key"))?;
            out.insert(device, k);
        }
        Ok(Keys(out))
    }
    pub fn insert(&mut self, device: &str, key: VerifyingKey) {
        self.0.insert(device.to_owned(), key);
    }
}

/// Where a line was read: its file and its number (from 1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Place {
    pub file: String,
    pub line: usize,
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// A gap between what an export says and what it should.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Not JSON, or not of the model (an unknown field, event or severity).
    Unreadable(String),
    /// Signed by no key given.
    UnknownEmitter(String),
    /// Not signed, or not by its emitter's key.
    BadSignature { emitter: String, seq: u64 },
    /// Numbers missing from a chain, that no `audit.dropped` says.
    Gap { emitter: String, from: u64, to: u64 },
    /// Two events of one number.
    Fork { emitter: String, seq: u64 },
    /// An event whose `prev` is not the fingerprint of the one before.
    BrokenLink { emitter: String, seq: u64 },
    /// A checkpoint whose head is not the event of its number.
    Checkpoint { emitter: String, seq: u64 },
    /// A checkpoint past the end of the export: its end was taken away.
    EndMissing {
        emitter: String,
        seq: u64,
        last: u64,
    },
    /// Two events of one entry of the server's journal, of different encrypted bytes (§5.3).
    Anchor { stream: String, n: u64 },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Problem::Unreadable(e) => write!(f, "not a record of escouade.audit/v1: {e}"),
            Problem::UnknownEmitter(d) => write!(f, "emitter {d} has no key given (--keys)"),
            Problem::BadSignature { emitter, seq } => {
                write!(f, "{emitter} #{seq}: the signature does not hold")
            }
            Problem::Gap { emitter, from, to } => write!(f, "{emitter}: #{from} to #{to} missing"),
            Problem::Fork { emitter, seq } => {
                write!(f, "{emitter} #{seq}: two different events of one number")
            }
            Problem::BrokenLink { emitter, seq } => {
                write!(
                    f,
                    "{emitter} #{seq}: prev is not the fingerprint of #{}",
                    seq - 1
                )
            }
            Problem::Checkpoint { emitter, seq } => {
                write!(
                    f,
                    "{emitter}: the checkpoint of #{seq} is not the event of that number"
                )
            }
            Problem::EndMissing { emitter, seq, last } => {
                write!(
                    f,
                    "{emitter}: a checkpoint says #{seq}, the export ends at #{last}"
                )
            }
            Problem::Anchor { stream, n } => {
                write!(
                    f,
                    "entry {n} of {stream}: two events of different encrypted bytes"
                )
            }
        }
    }
}

/// A problem, where it is (none for what is of a whole chain).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub place: Option<Place>,
    pub problem: Problem,
}

/// What `verify` found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub events: usize,
    pub emitters: usize,
    /// Events of a number seen twice, the same (a SIEM dedups by `id`).
    pub doubles: usize,
    /// Events an `audit.dropped` says were dropped.
    pub dropped: u64,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.findings.is_empty()
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for x in &self.findings {
            match &x.place {
                Some(p) => writeln!(f, "{p}: {}", x.problem)?,
                None => writeln!(f, "{}", x.problem)?,
            }
        }
        write!(
            f,
            "{} events, {} emitters, {}, {} problem{}",
            self.events,
            self.emitters,
            if self.ok() {
                "whole chains"
            } else {
                "chains not whole"
            },
            self.findings.len(),
            if self.findings.len() == 1 { "" } else { "s" },
        )?;
        if self.doubles > 0 {
            write!(f, ", {} doubles", self.doubles)?;
        }
        if self.dropped > 0 {
            write!(
                f,
                ", {} dropped by the buffer (audit.dropped)",
                self.dropped
            )?;
        }
        Ok(())
    }
}

/// An event read.
struct Read {
    place: Place,
    value: Value,
    record: Record,
    digest: Digest,
}

/// Checks the lines of an export (each its place), against `keys` (none: the signatures are not checked) and the
/// checkpoints read from the team's stream (`--checkpoints`).
pub fn verify(
    lines: Vec<(Place, String)>,
    keys: Option<&Keys>,
    checkpoints: Vec<(Place, String)>,
) -> Report {
    let mut report = Report::default();
    let mut read: Vec<Read> = Vec::new();
    for (place, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                report.findings.push(Finding {
                    place: Some(place),
                    problem: Problem::Unreadable(e.to_string()),
                });
                continue;
            }
        };
        let record = match Record::from_value(value.clone()) {
            Ok(r) => r,
            Err(e) => {
                report.findings.push(Finding {
                    place: Some(place),
                    problem: Problem::Unreadable(e.to_string()),
                });
                continue;
            }
        };
        let digest = match chain_digest(&value) {
            Ok(d) => d,
            Err(e) => {
                report.findings.push(Finding {
                    place: Some(place),
                    problem: Problem::Unreadable(e.to_string()),
                });
                continue;
            }
        };
        read.push(Read {
            place,
            value,
            record,
            digest,
        });
    }
    report.events = read.len();

    // The signatures.
    if let Some(keys) = keys {
        let mut unknown = BTreeSet::new();
        for r in &read {
            let device = r.record.emitter.device.as_str();
            match keys.0.get(device) {
                None => {
                    if unknown.insert(device.to_owned()) {
                        report.findings.push(Finding {
                            place: Some(r.place.clone()),
                            problem: Problem::UnknownEmitter(device.to_owned()),
                        });
                    }
                }
                Some(k) if !signature_holds(&r.value, k) => report.findings.push(Finding {
                    place: Some(r.place.clone()),
                    problem: Problem::BadSignature {
                        emitter: device.to_owned(),
                        seq: r.record.emitter.seq,
                    },
                }),
                Some(_) => {}
            }
        }
    }

    // The chains, emitter by emitter.
    let mut by: BTreeMap<String, BTreeMap<u64, &Read>> = BTreeMap::new();
    for r in &read {
        let chain = by
            .entry(r.record.emitter.device.as_str().to_owned())
            .or_default();
        match chain.get(&r.record.emitter.seq) {
            Some(other) if other.digest == r.digest => report.doubles += 1,
            Some(_) => report.findings.push(Finding {
                place: Some(r.place.clone()),
                problem: Problem::Fork {
                    emitter: r.record.emitter.device.as_str().to_owned(),
                    seq: r.record.emitter.seq,
                },
            }),
            None => {
                chain.insert(r.record.emitter.seq, r);
            }
        }
    }
    report.emitters = by.len();
    let mut checks: Vec<(Place, Record)> = read
        .iter()
        .filter(|r| matches!(r.record.fact, Fact::AuditCheckpoint { .. }))
        .map(|r| (r.place.clone(), r.record.clone()))
        .collect();
    for (place, line) in checkpoints {
        match Record::from_line(&line) {
            Ok(r) if matches!(r.fact, Fact::AuditCheckpoint { .. }) => checks.push((place, r)),
            Ok(_) => report.findings.push(Finding {
                place: Some(place),
                problem: Problem::Unreadable("not an audit.checkpoint".to_owned()),
            }),
            Err(e) => report.findings.push(Finding {
                place: Some(place),
                problem: Problem::Unreadable(e.to_string()),
            }),
        }
    }
    for (emitter, chain) in &by {
        // The numbers an `audit.dropped` of this emitter says.
        let said: Vec<(u64, u64)> = chain
            .values()
            .filter_map(|r| match &r.record.fact {
                Fact::AuditDropped { detail, .. } => Some((detail.first, detail.last)),
                _ => None,
            })
            .collect();
        report.dropped += said.iter().map(|(a, b)| b - a + 1).sum::<u64>();
        let covered = |n: u64| said.iter().any(|(a, b)| (*a..=*b).contains(&n));
        let mut before: Option<&Read> = None;
        for (seq, r) in chain {
            if let Some(b) = before {
                let expected = b.record.emitter.seq + 1;
                if *seq > expected {
                    let missing: Vec<u64> = (expected..*seq).filter(|n| !covered(*n)).collect();
                    if let (Some(from), Some(to)) = (missing.first(), missing.last()) {
                        report.findings.push(Finding {
                            place: Some(r.place.clone()),
                            problem: Problem::Gap {
                                emitter: emitter.clone(),
                                from: *from,
                                to: *to,
                            },
                        });
                    }
                } else if r.record.emitter.prev.as_ref() != Some(&b.digest) {
                    report.findings.push(Finding {
                        place: Some(r.place.clone()),
                        problem: Problem::BrokenLink {
                            emitter: emitter.clone(),
                            seq: *seq,
                        },
                    });
                }
            } else if *seq == 1 && r.record.emitter.prev.is_some() {
                report.findings.push(Finding {
                    place: Some(r.place.clone()),
                    problem: Problem::BrokenLink {
                        emitter: emitter.clone(),
                        seq: 1,
                    },
                });
            }
            before = Some(r);
        }
        let last = chain.keys().next_back().copied().unwrap_or(0);
        for (place, c) in &checks {
            if c.emitter.device.as_str() != emitter {
                continue;
            }
            let Fact::AuditCheckpoint { detail, .. } = &c.fact else {
                continue;
            };
            match chain.get(&detail.seq) {
                Some(r) if r.digest == detail.head => {}
                Some(_) => report.findings.push(Finding {
                    place: Some(place.clone()),
                    problem: Problem::Checkpoint {
                        emitter: emitter.clone(),
                        seq: detail.seq,
                    },
                }),
                None if detail.seq > last => report.findings.push(Finding {
                    place: Some(place.clone()),
                    problem: Problem::EndMissing {
                        emitter: emitter.clone(),
                        seq: detail.seq,
                        last,
                    },
                }),
                // Before the export's start, or in a gap already said.
                None => {}
            }
        }
    }

    report.findings.extend(crosscheck(
        &read.iter().map(|r| &r.record).collect::<Vec<_>>(),
    ));
    report
}

/// The cross-check of S1 and S2 (§5.3), by `(anchor.stream, anchor.n)`: here the first rule, a fingerprint of the
/// encrypted bytes that differs between two events of one entry. AU-5 adds the two others (an event of S2 without its
/// entry of S1; an entry of S1 without its event of S2 for more than an hour), once both sources emit.
pub fn crosscheck(records: &[&Record]) -> Vec<Finding> {
    let mut seen: BTreeMap<(String, u64), &Digest> = BTreeMap::new();
    let mut out = Vec::new();
    let mut said = BTreeSet::new();
    for r in records {
        let Some(a) = &r.anchor else { continue };
        let key = (a.stream.as_str().to_owned(), a.n);
        match seen.get(&key) {
            Some(ct) if **ct != a.ct => {
                if said.insert(key.clone()) {
                    out.push(Finding {
                        place: None,
                        problem: Problem::Anchor {
                            stream: key.0,
                            n: key.1,
                        },
                    });
                }
            }
            Some(_) => {}
            None => {
                seen.insert(key, &a.ct);
            }
        }
    }
    out
}
