//! The gate's journal (R8): each connection, allowed or refused (its time, host, port, bytes, length, agent and
//! project), never what went through it nor a token. The last [`KEPT`] are kept for the interface (Réglages de
//! l'espace › Réseau privé › Journal, RP-5); each one is a line of the server's log too.

use serde::Serialize;
use std::collections::VecDeque;

/// How many the gate keeps.
pub const KEPT: usize = 1000;

/// Where a connection went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Route {
    /// Through a connector, to a private network.
    Connector,
    /// Straight out.
    Direct,
    /// Through the space's proxy.
    Proxy,
    /// Nowhere: refused before.
    Refused,
}

/// A connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// When it was asked (milliseconds since 1970).
    pub at: u64,
    pub host: String,
    pub port: u16,
    /// The agent (its id) or what else holds the token (`terminal`, `git`…); empty when no token said it.
    pub agent: String,
    /// The project's id; empty when no token said it.
    pub project: String,
    /// `http` or `socks`.
    pub by: &'static str,
    pub route: Route,
    /// The connector's id, for a flow of a private network.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    pub allowed: bool,
    /// Why it was refused or failed: its kind (`not-in-list`…) and its sentence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Bytes from the agent, and to it.
    pub up: u64,
    pub down: u64,
    /// How long it lasted, in milliseconds.
    pub ms: u64,
}

/// The last connections.
#[derive(Debug, Default)]
pub struct Journal {
    kept: VecDeque<Entry>,
}

impl Journal {
    pub fn push(&mut self, e: Entry) {
        // No token said them: a dash in the log, empty in the entry.
        let said = |s: &str| {
            if s.is_empty() {
                "-".to_string()
            } else {
                s.to_string()
            }
        };
        let (agent, project) = (said(&e.agent), said(&e.project));
        if e.allowed {
            log::info!(
                "gate: {}:{} by {} for {} of {} through {:?}{}: {} B up, {} B down, {} ms",
                e.host,
                e.port,
                e.by,
                agent,
                project,
                e.route,
                e.connector
                    .as_deref()
                    .map(|c| format!(" {c}"))
                    .unwrap_or_default(),
                e.up,
                e.down,
                e.ms
            );
        } else {
            log::info!(
                "gate: {}:{} by {} for {} of {} refused ({}): {}",
                e.host,
                e.port,
                e.by,
                agent,
                project,
                e.refusal.unwrap_or(""),
                e.reason.as_deref().unwrap_or("")
            );
        }
        if self.kept.len() == KEPT {
            self.kept.pop_front();
        }
        self.kept.push_back(e);
    }

    /// The kept ones, the oldest first.
    pub fn entries(&self) -> Vec<Entry> {
        self.kept.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_keeps_the_last_thousand() {
        let mut j = Journal::default();
        for i in 0..(KEPT + 5) {
            j.push(Entry {
                at: i as u64,
                host: "h".into(),
                port: 1,
                agent: String::new(),
                project: String::new(),
                by: "http",
                route: Route::Direct,
                connector: None,
                allowed: true,
                refusal: None,
                reason: None,
                up: 0,
                down: 0,
                ms: 0,
            });
        }
        let e = j.entries();
        assert_eq!(e.len(), KEPT);
        assert_eq!(e[0].at, 5);
    }
}
