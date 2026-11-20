//! The summary of the box « À toi » (ESC-513, docs/INBOX.md § 11): never one alert per request. The
//! first request after a calm waits two minutes, to gather what comes together, then one system
//! notification says how many wait and of what kinds, from whom: never a request's title, which comes
//! from an agent and leaves the app. Then one at most every 30 minutes (or every hour, as
//! « Résumé de la boîte « À toi » » says; never: the counter alone). No chime, no flashing: the
//! requests an agent stopped for keep their own notifications of today.
//!
//! A request counts once: told, it is not told again, unless it comes back from « Plus tard ». One
//! the user saw in the box, one left to an agent, or one put off is not told. While the window is
//! in front, nothing is said: the box and its counter are there; what the user did not see is told
//! once the window is left. The MCP server off, nothing at all.

use crate::asks::{Ask, AskKind, AskState};
use crate::core::Core;
use crate::i18n::{self, Lang};
use crate::model::now_ms;
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

/// The wait after the first request of a calm, to gather what comes with it.
pub const GROUP_MS: i64 = 2 * 60 * 1000;

/// `GROUP_MS`, unless `ESCOUADE_INBOX_GROUP_MS` says otherwise (the end-to-end tests): read by a build of
/// development alone, as every variable of the tests (SEC-13).
fn group_ms() -> i64 {
    std::env::var("ESCOUADE_INBOX_GROUP_MS")
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .and_then(|v| v.parse().ok())
        .unwrap_or(GROUP_MS)
}

/// What the summary keeps between two readings.
#[derive(Debug, Default)]
pub struct Summary {
    /// The requests already told, by `id@<when it last came back>`.
    told: HashSet<String>,
    /// When the first request not told yet was seen: the gathering's start.
    first: Option<i64>,
    /// When the last summary was said.
    last: Option<i64>,
}

/// What to do now.
#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    /// Nothing waits to be told.
    Nothing,
    /// Some wait: their time has not come.
    Wait,
    /// Tell these, now.
    Tell(Vec<String>),
}

/// The key a request is told under: told again only once it came back from « Plus tard ».
fn key(a: &Ask) -> String {
    let back = a
        .history
        .iter()
        .rev()
        .find(|e| e.what == "woke")
        .map(|e| e.at)
        .filter(|_| a.woke)
        .unwrap_or(0);
    format!("{}@{back}", a.id)
}

/// The requests the user may be told of: open (not put off), not left to an agent, not seen in the
/// box (or back from « Plus tard » since).
pub(crate) fn fresh(asks: &[Ask]) -> Vec<&Ask> {
    asks.iter()
        .filter(|a| a.state == AskState::Open && a.delegated_to.is_none() && (!a.seen || a.woke))
        .collect()
}

impl Summary {
    /// Its clock moved `ms` back, for the tests: the gathering and the last summary are that old.
    #[cfg(test)]
    pub fn backdate(&mut self, ms: i64) {
        self.first = self.first.map(|t| t - ms);
        self.last = self.last.map(|t| t - ms);
    }

    /// The decision, at `now`, for these `fresh` keys; `every`: the least minutes between two
    /// summaries (None: never, every request counted as told); `held`: the window is in front.
    pub fn plan(
        &mut self,
        fresh: &[String],
        now: i64,
        every: Option<i64>,
        group: i64,
        held: bool,
    ) -> Plan {
        let new: Vec<String> = fresh
            .iter()
            .filter(|k| !self.told.contains(*k))
            .cloned()
            .collect();
        // What is no longer there is forgotten: the set does not grow with the days.
        let now_there: HashSet<&String> = fresh.iter().collect();
        self.told.retain(|k| now_there.contains(k));
        if new.is_empty() {
            self.first = None;
            return Plan::Nothing;
        }
        let Some(every) = every else {
            self.told.extend(new);
            self.first = None;
            return Plan::Nothing;
        };
        let first = *self.first.get_or_insert(now);
        if held || now < first + group {
            return Plan::Wait;
        }
        if self.last.is_some_and(|l| now < l + every * 60_000) {
            return Plan::Wait;
        }
        self.told.extend(new.iter().cloned());
        self.last = Some(now);
        self.first = None;
        Plan::Tell(new)
    }
}

/// What a kind of request is called in a summary, with its author.
fn phrase(lang: Lang, a: &Ask) -> String {
    let who = a.author.name.as_str();
    match (a.kind, a.gate) {
        (AskKind::Choice, _) => tr_in!(lang, "un choix de {who}", "a choice from {who}", who = who),
        (AskKind::Confirm, true) => {
            tr_in!(
                lang,
                "un feu vert de {who}",
                "a go-ahead from {who}",
                who = who
            )
        }
        (AskKind::Confirm, false) => {
            tr_in!(
                lang,
                "un oui ou non de {who}",
                "a yes or no from {who}",
                who = who
            )
        }
        (AskKind::Answer, _) => tr_in!(
            lang,
            "une réponse de {who}",
            "an answer from {who}",
            who = who
        ),
        (AskKind::Task, _) => tr_in!(lang, "une tâche de {who}", "a task from {who}", who = who),
    }
}

/// The notification's title and body for `told`, the requests of these `projects` (by id, their
/// names). Never a request's title nor its text: its kind and its author.
pub fn text(lang: Lang, told: &[&Ask], projects: &BTreeMap<String, String>) -> (String, String) {
    let mut ids: Vec<&str> = told.iter().map(|a| a.project_id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    let n = told.len();
    let count = if n == 1 {
        tr_in!(
            lang,
            "Une demande t’attend",
            "One request is waiting for you"
        )
    } else {
        tr_in!(
            lang,
            "{n} demandes t’attendent",
            "{n} requests are waiting for you",
            n = n
        )
    };
    let mut kinds: Vec<String> = Vec::new();
    for a in told {
        let p = phrase(lang, a);
        if !kinds.contains(&p) {
            kinds.push(p);
        }
    }
    let more = kinds.len() > 3;
    kinds.truncate(3);
    let list = kinds.join(", ");
    let end = if more { "…" } else { "." };
    let title = match ids.as_slice() {
        [one] => format!(
            "{} · {}",
            projects.get(*one).map(String::as_str).unwrap_or("Escouade"),
            tr_in!(lang, "À toi", "Your turn")
        ),
        _ => tr_in!(lang, "Escouade · À toi", "Escouade · Your turn"),
    };
    let body = if ids.len() > 1 {
        tr_in!(
            lang,
            "{count} dans {p} projets : {list}{end}",
            "{count} in {p} projects: {list}{end}",
            count = count,
            p = ids.len(),
            list = list,
            end = end
        )
    } else {
        tr_in!(
            lang,
            "{count} dans À toi : {list}{end}",
            "{count} in Your turn: {list}{end}",
            count = count,
            list = list,
            end = end
        )
    };
    (title, body)
}

/// The range a count falls in, for the statistics: never the count itself.
pub fn bucket(n: usize) -> &'static str {
    match n {
        0 | 1 => "1",
        2..=3 => "2-3",
        _ => "4+",
    }
}

impl Core {
    /// The summary, when requests came and its time did (the box's loop, and two minutes after a
    /// request is left). The MCP server off, or the system notifications off, nothing.
    pub fn inbox_summary_tick(self: &Arc<Self>) {
        let settings = self.settings.read().clone();
        let on = self.inbox_enabled() && settings.os_notifications;
        let asks = self.asks.lock().list.clone();
        let fresh = fresh(&asks);
        let keys: Vec<String> = if on {
            fresh.iter().map(|a| key(a)).collect()
        } else {
            Vec::new()
        };
        let held = !cfg!(test) && self.host.attended();
        let plan = self.inbox_summary.lock().plan(
            &keys,
            now_ms(),
            settings.inbox_summary.minutes(),
            group_ms(),
            held,
        );
        let Plan::Tell(told) = plan else {
            return;
        };
        let told: Vec<&Ask> = fresh
            .into_iter()
            .filter(|a| told.contains(&key(a)))
            .collect();
        let projects: BTreeMap<String, String> = self
            .projects
            .read()
            .iter()
            .map(|p| (p.id.clone(), p.name.clone()))
            .collect();
        let (title, body) = text(i18n::ui(), &told, &projects);
        let mut ids: Vec<&str> = told.iter().map(|a| a.project_id.as_str()).collect();
        ids.dedup();
        let project_id = match ids.as_slice() {
            [one] => Some(one.to_string()),
            _ => None,
        };
        log::info!(
            "inbox summary told ({} requests, {} projects)",
            told.len(),
            ids.len()
        );
        self.telemetry
            .track(crate::telemetry::Event::InboxSummaryShown {
                requests: bucket(told.len()),
            });
        #[cfg(test)]
        {
            self.alerts.lock().push(format!("{title} | {body}"));
            let _ = project_id;
        }
        #[cfg(not(test))]
        self.inbox_toast(title, body, project_id);
    }

    #[cfg(not(test))]
    fn inbox_toast(self: &Arc<Self>, title: String, body: String, project_id: Option<String>) {
        // No chime, no flashing: a toast, which brings the window back on the box.
        // A sandboxed run kept out of the user's way (the end-to-end tests) shows none on their screen.
        if crate::paths::hidden_window() {
            return;
        }
        // The host brings its window forward (a host without one publishes it, for its clients).
        let weak = Arc::downgrade(self);
        self.host.notify(
            &title,
            &body,
            Box::new(move || {
                if let Some(c) = weak.upgrade() {
                    c.hub.emit(crate::model::UiEvent::InboxOpen {
                        project_id: project_id.clone(),
                    });
                }
            }),
        );
    }

    /// Two minutes (`GROUP_MS`) after a request is left, the summary looks again: the box's loop
    /// alone would tell it up to 20 seconds later.
    pub(crate) fn inbox_summary_soon(self: &Arc<Self>) {
        let me = self.clone();
        tauri::async_runtime::spawn(async move {
            let wait = group_ms().max(0) as u64 + 50;
            tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
            me.inbox_summary_tick();
        });
    }
}
