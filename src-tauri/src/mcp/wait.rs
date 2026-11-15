//! What `wait_ticket` does (`tools`): waits until one of the tickets named changes, then says what
//! changed. It replaces the loop a client wrote with `get_ticket`, which read a ticket every few
//! seconds and missed what came and went between two reads (a request asked and answered, a block
//! put away). What is watched is what a client acts on: the column, the block, an agent that waits
//! for an answer, an agent at rest in « À tester » (the ticket can be approved), the ticket gone.
//! Each answer gives `since`, what it saw of each ticket: the next call compares the tickets with
//! it, so a ticket whose state differs from it is told at once, a change made between two calls
//! included (one that came back to its first state is not seen). Without it, the state at the call
//! is the one compared. The tools of the flow give one too, the ticket as it was before their action:
//! an approval over before the client waits is told all the same. A wait ends when its client gives
//! the request up (its connection closes): there is no one to answer.

use super::read::{answerable_views, json};
use super::resolve;
use super::tools::{ToolError, WaitTicketArgs};
use super::Caller;
use crate::core::Core;
use crate::model::{Column, Ticket};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::{Duration, Instant};

/// The most tickets one wait watches.
const TICKETS_MAX: usize = 20;

/// How long a wait lasts when no timeout is asked, in seconds, and the longest it may: under the
/// 300 seconds an HTTP client such as undici waits for the headers of an answer by default.
const WAIT_DEFAULT: u64 = 120;
const WAIT_MAX: u64 = 240;

/// How often the tickets are looked at.
const POLL: Duration = Duration::from_millis(250);

/// What is watched of a ticket, as `since` keeps it: its block by a hash of its reason (the
/// cursor stays short, and a reason is not copied into it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Mark {
    id: String,
    column: Column,
    blocked: Option<u32>,
    conflict: bool,
    waiting: bool,
    resting: bool,
}

impl Mark {
    /// What differs in `now`, in the order the answer tells it.
    fn what(&self, now: &Mark) -> Vec<&'static str> {
        [
            ("column", self.column != now.column),
            ("blocked", self.blocked != now.blocked),
            ("conflict", self.conflict != now.conflict),
            ("waiting", self.waiting != now.waiting),
            ("resting", self.resting != now.resting),
        ]
        .into_iter()
        .filter_map(|(name, differs)| differs.then_some(name))
        .collect()
    }
}

/// A ticket as it is looked at: what is watched of it, and what the answer says of it.
struct Seen {
    key: String,
    id: String,
    column: Column,
    blocked: Option<String>,
    conflict: bool,
    step: Option<String>,
    /// Its agent has requests that wait for an answer (`list_requests` lists them).
    waiting: bool,
    /// It is in « À tester » and its agent is at rest: it can be approved.
    resting: bool,
}

impl Seen {
    fn of(core: &Core, t: &Ticket) -> Self {
        // Its agent's requests (counted as `list_agents` does) and whether it is at work.
        let agent = t
            .agent_id
            .as_deref()
            .and_then(|id| core.agent(id).ok())
            .map(|h| {
                let rt = h.lock();
                (answerable_views(&rt).count(), rt.meta.status.is_active())
            });
        Self {
            key: t.key.clone(),
            id: t.id.clone(),
            column: t.column,
            blocked: t.blocked.clone(),
            conflict: t.conflict,
            step: t.step.clone(),
            waiting: agent.is_some_and(|(requests, _)| requests > 0),
            resting: t.column == Column::Review && agent.is_some_and(|(_, at_work)| !at_work),
        }
    }

    fn mark(&self) -> Mark {
        Mark {
            id: self.id.clone(),
            column: self.column,
            blocked: self.blocked.as_deref().map(fnv1a),
            conflict: self.conflict,
            waiting: self.waiting,
            resting: self.resting,
        }
    }
}

/// A ticket watched.
struct Watch {
    /// What the client last knew of it: its cursor's, else what it was at the call.
    baseline: Mark,
    /// What it was when last looked at (the last time it was there, once gone).
    seen: Seen,
    /// It was deleted since.
    gone: bool,
}

impl Watch {
    fn look(&mut self, core: &Core) {
        match core.ticket(&self.seen.id) {
            Ok(t) => self.seen = Seen::of(core, &t),
            Err(_) => self.gone = true,
        }
    }

    fn what(&self) -> Vec<&'static str> {
        if self.gone {
            return vec!["gone"];
        }
        self.baseline.what(&self.seen.mark())
    }
}

/// The 32-bit FNV-1a hash of `text`.
fn fnv1a(text: &str) -> u32 {
    text.bytes().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

/// The seconds a wait lasts, as asked: 1 at least, `WAIT_MAX` at most.
fn timeout_of(asked: Option<u64>) -> Duration {
    Duration::from_secs(asked.unwrap_or(WAIT_DEFAULT).clamp(1, WAIT_MAX))
}

/// `marks` as a cursor: the base64url, without padding, of their JSON.
fn cursor(marks: &[Mark]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(marks).unwrap_or_default())
}

/// The cursor of the ticket `t` as it is now, for a tool that changes it to hand back: a wait that
/// begins after the change (a quick approval is over before the client waits) is told of it.
pub(super) fn since(core: &Core, t: &Ticket) -> String {
    cursor(&[Seen::of(core, t).mark()])
}

/// The marks a cursor holds; None for a text no answer gave.
fn marks_of(since: &str) -> Option<Vec<Mark>> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(since)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// What a ticket was before the change.
#[derive(Serialize)]
struct Before {
    column: Column,
    blocked: bool,
    conflict: bool,
    waiting: bool,
    resting: bool,
}

#[derive(Serialize)]
struct Change {
    key: String,
    what: Vec<&'static str>,
    before: Before,
}

/// A ticket as the answer gives it now (as it was last seen, once gone).
#[derive(Serialize)]
struct Now {
    key: String,
    id: String,
    column: Column,
    blocked: Option<String>,
    conflict: bool,
    step: Option<String>,
    waiting: bool,
    resting: bool,
    gone: bool,
}

#[derive(Serialize)]
struct Answer {
    changed: bool,
    changes: Vec<Change>,
    tickets: Vec<Now>,
    /// What the tickets were at the end of the wait, for the next call.
    since: String,
    /// How many seconds it waited.
    waited: u64,
}

/// `wait_ticket`: the tickets named, until one changes or the timeout comes, or until `cancelled`
/// (its client gave up the request): there is then no one to answer, and the wait stops.
pub(super) async fn wait(
    core: &Core,
    caller: &Caller,
    a: WaitTicketArgs,
    cancelled: impl Future<Output = ()>,
) -> Result<String, ToolError> {
    if a.tickets.is_empty() {
        return Err(ToolError::Failed(tr!(
            "Donne au moins un ticket à attendre : sa clé ou son id.",
            "Give at least one ticket to wait for: its key or its id."
        )));
    }
    let n = a.tickets.len();
    if n > TICKETS_MAX {
        return Err(ToolError::Failed(tr!(
            "{n} tickets : {TICKETS_MAX} au plus.",
            "{n} tickets: {TICKETS_MAX} at most."
        )));
    }
    // A ticket named twice is watched once.
    let mut tickets: Vec<Seen> = Vec::new();
    for asked in &a.tickets {
        let t = resolve::ticket(core, caller, asked)?;
        if !tickets.iter().any(|s| s.id == t.id) {
            tickets.push(Seen::of(core, &t));
        }
    }
    let known = match a.since.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(since) => marks_of(since).ok_or_else(|| {
            ToolError::Failed(tr!(
                "since n’est pas un curseur rendu par wait_ticket : redonne-le tel qu’il t’a été donné, ou laisse-le de côté.",
                "since is not one a wait_ticket answered: pass it back as it was given, or leave it out."
            ))
        })?,
        None => Vec::new(),
    };
    let mut watches: Vec<Watch> = tickets
        .into_iter()
        .map(|seen| Watch {
            baseline: known
                .iter()
                .find(|m| m.id == seen.id)
                .cloned()
                .unwrap_or_else(|| seen.mark()),
            seen,
            gone: false,
        })
        .collect();
    let mut cancelled = std::pin::pin!(cancelled);
    let began = Instant::now();
    let deadline = began + timeout_of(a.timeout);
    loop {
        for w in &mut watches {
            w.look(core);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() || watches.iter().any(|w| !w.what().is_empty()) {
            break;
        }
        tokio::select! {
            () = tokio::time::sleep(POLL.min(left)) => {}
            () = &mut cancelled => {
                return Err(ToolError::Failed(tr!(
                    "Attente annulée par le client.",
                    "Wait cancelled by the client."
                )));
            }
        }
    }
    let changes: Vec<Change> = watches
        .iter()
        .filter(|w| !w.what().is_empty())
        .map(|w| Change {
            key: w.seen.key.clone(),
            what: w.what(),
            before: Before {
                column: w.baseline.column,
                blocked: w.baseline.blocked.is_some(),
                conflict: w.baseline.conflict,
                waiting: w.baseline.waiting,
                resting: w.baseline.resting,
            },
        })
        .collect();
    let marks: Vec<Mark> = watches
        .iter()
        .filter(|w| !w.gone)
        .map(|w| w.seen.mark())
        .collect();
    json(&Answer {
        changed: !changes.is_empty(),
        changes,
        tickets: watches
            .into_iter()
            .map(|w| Now {
                key: w.seen.key,
                id: w.seen.id,
                column: w.seen.column,
                blocked: w.seen.blocked,
                conflict: w.seen.conflict,
                step: w.seen.step,
                waiting: w.seen.waiting,
                resting: w.seen.resting,
                gone: w.gone,
            })
            .collect(),
        since: cursor(&marks),
        waited: began.elapsed().as_secs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_of_a_reason_is_its_32_bit_fnv_1a() {
        // The reference values of the algorithm.
        assert_eq!(fnv1a(""), 0x811c_9dc5);
        assert_eq!(fnv1a("a"), 0xe40c_292c);
        assert_eq!(fnv1a("foobar"), 0xbf9c_f968);
        // Bytes, not characters.
        assert_ne!(fnv1a("é"), fnv1a("e"));
    }

    #[test]
    fn the_timeout_is_a_second_at_least_240_at_most_and_120_when_not_asked() {
        let secs = |asked| timeout_of(asked).as_secs();
        assert_eq!(secs(None), 120);
        assert_eq!(secs(Some(0)), 1);
        assert_eq!(secs(Some(1)), 1);
        assert_eq!(secs(Some(60)), 60);
        assert_eq!(secs(Some(240)), 240);
        assert_eq!(secs(Some(241)), 240);
        assert_eq!(secs(Some(u64::MAX)), 240);
    }

    #[test]
    fn a_cursor_comes_back_as_it_was_given_and_nothing_else_is_one() {
        let mark = |id: &str, blocked| Mark {
            id: id.into(),
            column: Column::Review,
            blocked,
            conflict: true,
            waiting: false,
            resting: true,
        };
        let marks = vec![mark("t-1", Some(fnv1a("Conflit"))), mark("t-2", None)];
        let given = cursor(&marks);
        assert!(!given.contains('='), "{given}");
        assert_eq!(marks_of(&given), Some(marks));
        assert_eq!(marks_of(&cursor(&[])), Some(Vec::new()));
        assert_eq!(marks_of("!?"), None);
        assert_eq!(marks_of(""), None);
    }

    #[test]
    fn what_differs_is_told_in_the_order_of_the_answer() {
        let was = Mark {
            id: "t".into(),
            column: Column::Doing,
            blocked: None,
            conflict: false,
            waiting: false,
            resting: false,
        };
        assert!(was.what(&was.clone()).is_empty());
        let now = Mark {
            id: "t".into(),
            column: Column::Review,
            blocked: Some(1),
            conflict: true,
            waiting: true,
            resting: true,
        };
        assert_eq!(
            was.what(&now),
            ["column", "blocked", "conflict", "waiting", "resting"]
        );
    }
}
