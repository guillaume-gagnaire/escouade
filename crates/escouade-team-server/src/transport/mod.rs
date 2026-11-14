//! The transport: what any Escouade server exposed to its clients needs, and nothing of the team.
//!
//! - CBOR in and out (`cbor`), with the error body every route answers (`Problem`);
//! - the handshake of the protocol's versions (`handshake`);
//! - bounded bodies, an announced length too large refused before reading (`body`);
//! - rate limits by address and by device (`rate`);
//! - listening in clear behind a reverse proxy, or in TLS with a given certificate (`listen`).
//!
//! The foundation of the server mode (ESC-451) has no shared transport yet: TS-1 carries its own here
//! (`docs/TEAM-SERVER.md` §16). This module uses nothing of the rest of the crate, to become a common crate later.

pub mod body;
pub mod cbor;
pub mod handshake;
pub mod listen;
pub mod rate;

pub use cbor::{Cbor, Problem};
