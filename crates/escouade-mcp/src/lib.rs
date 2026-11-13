//! The parts of Escouade's MCP server that stand without the app (docs/CRATES.md, § 13): who a request comes from
//! (`Caller`, `Tokens`), the port it listens on (`port`), its HTTP transport and the guards before rmcp (`http`, on a
//! `http::Host` the app's core implements), its activity log (`activity`), the tools' arguments and the schemas the
//! model reads (`args`, `memory::Kind`) and the declaration of the server in a harness (`install`). The tools
//! themselves act on the app's core: they stay in the app (`mcp::tools`), which re-exports each part where it was.

#[macro_use]
extern crate escouade_plugin_cli;

/// A text in the interface's language: `tr!("français", "English", args…)`, as the app's `tr!`.
macro_rules! tr {
    ($($t:tt)+) => {
        tr_in!(escouade_base::i18n::ui(), $($t)+)
    };
}

pub mod activity;
pub mod args;
pub mod http;
pub mod install;
pub mod memory;
pub mod port;

use base64::Engine;
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use subtle::ConstantTimeEq;

/// Where the server's port is chosen, at its first start or when the one saved is taken.
pub const PORTS: RangeInclusive<u16> = 47000..=47999;

/// The path of the MCP endpoint: `http://127.0.0.1:<port>/mcp`.
pub const PATH: &str = "/mcp";

/// Who a request comes from, as its token says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Caller {
    /// Claude Code outside Escouade (a terminal, another tool), with the token its config holds.
    External,
    /// The agent of this id, with the token its process was started with.
    Agent(String),
}

/// The server as the window is told it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub running: bool,
    /// The port it listens on; stopped, the one saved for its next start (0 before the first).
    pub port: u16,
    /// Why it does not run though the settings want it to.
    pub error: Option<String>,
}

/// The tokens a request may bring. Compared in constant time, every one of them each time: the
/// time an answer takes says nothing of how close a token came.
#[derive(Default)]
pub struct Tokens {
    /// Claude outside Escouade's, once read from the keychain (or made).
    pub external: RwLock<Option<String>>,
    /// The agents', by agent id.
    pub agents: RwLock<HashMap<String, String>>,
}

impl Tokens {
    pub fn caller(&self, presented: &str) -> Option<Caller> {
        let presented = presented.as_bytes();
        let mut found = None;
        if let Some(t) = &*self.external.read() {
            if bool::from(t.as_bytes().ct_eq(presented)) {
                found = Some(Caller::External);
            }
        }
        for (id, t) in self.agents.read().iter() {
            if bool::from(t.as_bytes().ct_eq(presented)) {
                found = Some(Caller::Agent(id.clone()));
            }
        }
        found
    }
}

/// 32 bytes from the system's random generator, in base64url (43 characters).
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the system's random generator");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
