//! `escouade-connector`, the connector of a private network (RP-3, `docs/PRIVATE-NETWORK.md` § 4; the spec page
//! « Escouade à distance › Atteindre un réseau privé »).
//!
//! A program a company runs in its network. It opens a channel **out** to an `escouade-server` (a WebSocket over TLS,
//! Noise inside: the crate `escouade-connector-proto`), through the company's proxy if it has one, and lets through
//! the agents' connections to the hosts of its **ceiling** alone, written by the company in `connector.toml`, that
//! the server cannot widen. It resolves each name with its machine's DNS, checks the address after the resolution and
//! joins that very address (no DNS rebinding); it never joins the loopback, the link-local (a cloud's metadata), its
//! own machine, nor an address out of the ranges of the ceiling. It journals each connection, allowed or refused, in
//! JSON on its standard output, never the content, the pairing's code nor a key. It listens on no port.
//!
//! - [`config`]: `connector.toml`; [`ceiling`]: the rules and ranges, the shape of a host;
//! - [`guard`]: the checks of each `open`, the resolution and the connection;
//! - [`run`]: the service (the channel, the schedule of reconnection, the relay, the reload);
//! - [`commands`]: `pair`, `unpair`, `check`; [`keys`]: what it keeps; [`journal`]: the journal.
//!
//! Nothing here depends on the core of Escouade nor on Tauri (§ 4.6).

pub mod ceiling;
pub mod commands;
pub mod config;
pub mod guard;
pub mod journal;
pub mod keys;
pub mod run;

use escouade_connector_proto::Version;

/// This program's version, which the server judges by its own (the app's version, `scripts/version.mjs`).
pub fn version() -> Version {
    let n = |s: &str| s.parse::<u16>().unwrap_or(0);
    Version::new(
        n(env!("CARGO_PKG_VERSION_MAJOR")),
        n(env!("CARGO_PKG_VERSION_MINOR")),
        n(env!("CARGO_PKG_VERSION_PATCH")),
    )
}
