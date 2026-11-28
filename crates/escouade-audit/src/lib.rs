//! The team's audit (AU-1, `docs/TEAM-AUDIT.md`): the model `escouade.audit/v1` and its closed list of events (§1),
//! what never goes out (§2), the outputs (JSON lines in a turned file or on the standard output, syslog RFC 5424 over
//! TLS, CEF; §4.2), the buffer on the disk and its taking up (§4.4), the signature and the chain (§5.2), and what
//! `escouade-audit verify` checks of an export (§5.4).
//!
//! The team server emits by it (AU-2, `escouade-team-server/src/audit.rs`); the stations' facts (AU-4) and the audit machine
//! (AU-5) will. The crate stands apart from the app's core: neither Tauri nor the network but syslog's socket.

pub mod event;
pub mod jcs;
pub mod never;
pub mod outbox;
pub mod record;
pub mod sink;
pub mod syslog;
pub mod verify;
pub mod words;

#[cfg(test)]
mod fill;
#[cfg(test)]
mod tests;

pub use event::{Actor, Fact, List, Nothing};
pub use outbox::{Audit, EmitError, Limits, Outbox};
pub use record::{Anchor, Chain, Draft, Emitter, EmitterRef, Record, Sealed, Source, Team, SCHEMA};
pub use sink::{Lines, RotatingFile, Sink};
pub use words::{CommandPrint, Digest, Id, Name, Reason, Refused, Time, Token, Ulid};
