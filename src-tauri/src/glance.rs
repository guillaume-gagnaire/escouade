//! What waits in a core, at a glance (ESC-539, `docs/SERVER-UX.md` § 5 and 6, direction B « les piles »): what the
//! piles of the projects' rail count and « Partout » shows of a space the window is not on — the agents that wait for
//! an answer, those at work, the requests of the box « À toi » — without its conversations nor its whole state.
//!
//! The window asks it of each other space (`space_glance`, served: through the space's own connection), every few
//! seconds while it is connected. It is small on purpose: what three spaces cost a window is measured.
//!
//! **The policies**: a core whose own policy denies the aggregate view (`servers.aggregateView` to `false`, a work
//! server's) gives its numbers alone, never a text; one asked `counts` (this computer's policy denies the view for this
//! space) the same. `servers.contentlessNotifications` is said, for the window's notifications of this space.

use crate::core::Core;
use crate::locks::{AGGREGATE_VIEW, CONTENTLESS};
use crate::model::{AgentStatus, PendingView};
use crate::remoteapi::proto::Machine;
use serde::Serialize;
use serde_json::Value;

/// The most agents a glance names (the waiting ones first): the others are counted.
pub const MAX_AGENTS: usize = 40;
/// The most requests of the box it names.
pub const MAX_ASKS: usize = 20;
/// The most characters of a request's argument it carries: « Partout » shows two lines of it.
pub const MAX_ARG: usize = 400;

/// What waits in a core.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Glance {
    /// The machine it runs on (« escouade@vps.example.net »).
    pub machine: Option<Machine>,
    pub version: String,
    /// The agents that wait for an answer, and the requests of the box that wait for the user.
    pub waiting: usize,
    /// The agents at work.
    pub running: usize,
    /// Numbers alone: the texts and the gestures are not for another space's view.
    pub counts_only: bool,
    /// Its notifications say that something waits, never what (`servers.contentlessNotifications`).
    pub contentless: bool,
    /// The colours of its first projects, for its pile in the rail.
    pub colors: Vec<String>,
    /// The projects of the agents and requests named.
    pub projects: Vec<GlanceProject>,
    pub agents: Vec<GlanceAgent>,
    pub asks: Vec<GlanceAsk>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlanceProject {
    pub id: String,
    pub name: String,
    pub color: String,
}

/// An agent that waits or works.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlanceAgent {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub status: AgentStatus,
    /// What it does now (« Lance npx vitest run src/sepa »).
    pub activity: Option<String>,
    pub last_activity: i64,
    /// The first request it waits on, its argument cut to `MAX_ARG`.
    pub request: Option<PendingView>,
}

/// A request of the box « À toi » that waits for the user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlanceAsk {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub created_at: i64,
}

/// `text` cut to `max` characters, « … » at its end when it was longer; whether it was.
fn cut(text: &str, max: usize) -> (String, bool) {
    match text.char_indices().nth(max) {
        Some((at, _)) => (format!("{}…", &text[..at]), true),
        None => (text.to_string(), false),
    }
}

impl Core {
    /// What waits here; `counts`: the numbers alone (the window's policy denies the aggregate view for this space).
    pub fn glance(&self, counts: bool) -> Glance {
        let locks = self.locks.read();
        let denied = locks.value(AGGREGATE_VIEW) == Some(&Value::Bool(false));
        let contentless = locks.value(CONTENTLESS) == Some(&Value::Bool(true));
        drop(locks);
        let counts_only = counts || denied;
        let mut agents: Vec<_> = self
            .agent_views()
            .into_iter()
            .filter(|a| !a.meta.archived && a.meta.status.is_active())
            .collect();
        let waiting_agents = agents
            .iter()
            .filter(|a| a.meta.status == AgentStatus::Waiting)
            .count();
        let running = agents.len() - waiting_agents;
        // The waiting first, the longest waiting first; then the latest at work.
        agents.sort_by_key(|a| {
            let waiting = a.meta.status == AgentStatus::Waiting;
            (
                !waiting,
                if waiting {
                    a.meta.last_activity
                } else {
                    -a.meta.last_activity
                },
            )
        });
        let asks: Vec<_> = if self.inbox_enabled() {
            let mut open: Vec<_> = self
                .asks_all()
                .into_iter()
                .filter(|a| a.state.waiting())
                .collect();
            open.sort_by_key(|a| a.created_at);
            open
        } else {
            Vec::new()
        };
        let mut g = Glance {
            machine: self.host.machine(),
            version: self.host.version(),
            waiting: waiting_agents + asks.len(),
            running,
            counts_only,
            contentless,
            colors: self
                .projects
                .read()
                .iter()
                .take(3)
                .map(|p| p.color.clone())
                .collect(),
            ..Glance::default()
        };
        if counts_only {
            return g;
        }
        g.agents = agents
            .into_iter()
            .take(MAX_AGENTS)
            .map(|a| GlanceAgent {
                request: a.requests.first().cloned().map(|mut r| {
                    let (arg, was_cut) = cut(&r.arg, MAX_ARG);
                    r.arg = arg;
                    r.cut |= was_cut;
                    r
                }),
                id: a.meta.id,
                project_id: a.meta.project_id,
                name: a.meta.name,
                status: a.meta.status,
                activity: a.activity,
                last_activity: a.meta.last_activity,
            })
            .collect();
        g.asks = asks
            .into_iter()
            .take(MAX_ASKS)
            .map(|a| GlanceAsk {
                id: a.id,
                project_id: a.project_id,
                title: cut(&a.title, 200).0,
                created_at: a.created_at,
            })
            .collect();
        let named: std::collections::HashSet<&str> = g
            .agents
            .iter()
            .map(|a| a.project_id.as_str())
            .chain(g.asks.iter().map(|a| a.project_id.as_str()))
            .collect();
        g.projects = self
            .projects
            .read()
            .iter()
            .filter(|p| named.contains(p.id.as_str()))
            .map(|p| GlanceProject {
                id: p.id.clone(),
                name: p.name.clone(),
                color: p.color.clone(),
            })
            .collect();
        g
    }
}

#[cfg(test)]
mod tests {
    use super::cut;

    #[test]
    fn a_long_text_is_cut_on_a_character() {
        assert_eq!(cut("abc", 5), ("abc".to_string(), false));
        assert_eq!(cut("éàüöî", 3), ("éàü…".to_string(), true));
    }
}
