//! What the tools that read answer (`tools`): the projects, agents, tickets and quota as the model
//! reads them, in JSON with keys in camelCase. Never a whole conversation: an agent is summed up
//! by its last message (cut) and the files it edited.

use super::resolve;
use super::tools::ToolError;
use super::Caller;
use crate::accounts;
use crate::core::Core;
use crate::git;
use crate::model::{AgentStatus, Column, Criterion, PauseReason, RateWindow, Service, Ticket};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tauri::Runtime;

/// The longest last message an agent's summary gives, in characters.
pub(crate) const LAST_MESSAGE_MAX: usize = 2000;

/// The most files an agent's summary lists: the latest it began to edit.
pub(crate) const TOUCHED_MAX: usize = 100;

/// `v` as the model reads it.
fn json<T: Serialize>(v: &T) -> Result<String, ToolError> {
    serde_json::to_string(v).map_err(|e| ToolError::Failed(e.to_string()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectRow {
    id: String,
    name: String,
    path: String,
    /// None: not a repository, or a detached HEAD.
    branch: Option<String>,
    /// Not archived.
    agents: usize,
    tickets: ColumnCounts,
}

#[derive(Serialize, Default)]
struct ColumnCounts {
    todo: usize,
    doing: usize,
    review: usize,
    done: usize,
}

impl ColumnCounts {
    fn add(&mut self, column: Column) {
        match column {
            Column::Todo => self.todo += 1,
            Column::Doing => self.doing += 1,
            Column::Review => self.review += 1,
            Column::Done => self.done += 1,
        }
    }
}

/// `list_projects`: every project, in the window's order.
pub(super) async fn projects<R: Runtime>(core: &Core<R>) -> Result<String, ToolError> {
    // One lock at a time (the order is projects before tickets, see `Core`).
    let projects = core.projects.read().clone();
    let mut tickets: HashMap<String, ColumnCounts> = HashMap::new();
    for t in core.tickets.read().iter() {
        tickets
            .entry(t.project_id.clone())
            .or_default()
            .add(t.column);
    }
    let mut agents: HashMap<String, usize> = HashMap::new();
    for h in core.agents.read().values() {
        let rt = h.lock();
        if !rt.meta.archived {
            *agents.entry(rt.meta.project_id.clone()).or_default() += 1;
        }
    }
    let mut rows = Vec::with_capacity(projects.len());
    for p in projects {
        // As the window last read it; a project it has not read yet is asked.
        let cached = core
            .git_cache
            .read()
            .get(&p.id)
            .map(|g| g.is_repo.then(|| g.branch.clone()));
        let branch = match cached {
            Some(branch) => branch,
            None => Some(git::head_branch(&p.path).await),
        };
        rows.push(ProjectRow {
            branch: branch.filter(|b| !b.is_empty() && b != "(detached)"),
            agents: agents.get(&p.id).copied().unwrap_or(0),
            tickets: tickets.remove(&p.id).unwrap_or_default(),
            id: p.id,
            name: p.name,
            path: p.path,
        });
    }
    json(&rows)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentRow {
    id: String,
    name: String,
    /// Its project's name.
    project: String,
    status: AgentStatus,
    /// A question or a permission waits for the user's answer.
    waiting: bool,
    /// Its ticket's key.
    ticket: Option<String>,
    model: String,
    /// Its Claude account's name.
    account: String,
    /// In US dollars, its running turn included.
    cost: f64,
}

/// `list_agents`: the agents not archived, oldest first, of every project or of `project`.
pub(super) fn agents<R: Runtime>(
    core: &Core<R>,
    project: Option<&str>,
) -> Result<String, ToolError> {
    let scope = project.map(|p| resolve::project(core, p)).transpose()?;
    let projects: HashMap<String, String> = core
        .projects
        .read()
        .iter()
        .map(|p| (p.id.clone(), p.name.clone()))
        .collect();
    let keys = ticket_keys(core);
    let settings = core.settings.read().clone();
    let rows: Vec<AgentRow> = core
        .agent_views()
        .into_iter()
        .filter(|v| !v.meta.archived)
        .filter(|v| scope.as_ref().is_none_or(|p| v.meta.project_id == p.id))
        .map(|v| AgentRow {
            project: projects
                .get(&v.meta.project_id)
                .cloned()
                .unwrap_or_default(),
            waiting: !v.pending.is_empty(),
            ticket: v.meta.ticket_id.as_ref().and_then(|t| keys.get(t).cloned()),
            account: accounts::get(&settings, &v.meta.account).name,
            cost: dollars(v.meta.cost + v.live_cost),
            status: v.meta.status,
            model: v.meta.model,
            name: v.meta.name,
            id: v.meta.id,
        })
        .collect();
    json(&rows)
}

/// A cost without the float's noise (0.30000000000000004).
fn dollars(cost: f64) -> f64 {
    (cost * 10_000.0).round() / 10_000.0
}

/// Each ticket's key, by id.
fn ticket_keys<R: Runtime>(core: &Core<R>) -> HashMap<String, String> {
    core.tickets
        .read()
        .iter()
        .map(|t| (t.id.clone(), t.key.clone()))
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TicketRow {
    id: String,
    key: String,
    title: String,
    column: Column,
    /// The keys of the tickets it comes after.
    after: Vec<String>,
    /// Its agent's name.
    agent: Option<String>,
    /// Why it no longer moves on by itself.
    blocked: Option<String>,
}

/// `list_tickets`: the tickets of `project`, of every column or of `column`, in the board's order.
pub(super) fn tickets<R: Runtime>(
    core: &Core<R>,
    project: &str,
    column: Option<Column>,
) -> Result<String, ToolError> {
    let project = resolve::project(core, project)?;
    let names: HashMap<String, String> = core
        .agents
        .read()
        .values()
        .map(|h| {
            let rt = h.lock();
            (rt.meta.id.clone(), rt.meta.name.clone())
        })
        .collect();
    let all = core.tickets.read().clone();
    let keys: HashMap<&str, &str> = all
        .iter()
        .map(|t| (t.id.as_str(), t.key.as_str()))
        .collect();
    let mut list: Vec<&Ticket> = all
        .iter()
        .filter(|t| t.project_id == project.id && column.is_none_or(|c| t.column == c))
        .collect();
    kanban_order(&mut list);
    let rows: Vec<TicketRow> = list
        .into_iter()
        .map(|t| TicketRow {
            id: t.id.clone(),
            key: t.key.clone(),
            title: t.title.clone(),
            column: t.column,
            after: after_keys(t, &keys),
            agent: t.agent_id.as_ref().and_then(|a| names.get(a).cloned()),
            blocked: t.blocked.clone(),
        })
        .collect();
    json(&rows)
}

/// The board's order, as the window shows it: the columns from « À faire » to « Terminé »,
/// « À faire » by priority, « En cours » and « À tester » by arrival, « Terminé » newest first.
fn kanban_order(list: &mut [&Ticket]) {
    list.sort_by(|a, b| {
        a.column.cmp(&b.column).then_with(|| match a.column {
            Column::Todo => (a.rank, a.created_at).cmp(&(b.rank, b.created_at)),
            Column::Doing => a.started_at.unwrap_or(0).cmp(&b.started_at.unwrap_or(0)),
            Column::Review => a.review_at.unwrap_or(0).cmp(&b.review_at.unwrap_or(0)),
            Column::Done => b.done_at.unwrap_or(0).cmp(&a.done_at.unwrap_or(0)),
        })
    });
}

/// The keys of the tickets `t` comes after (one deleted since is left out).
fn after_keys(t: &Ticket, keys: &HashMap<&str, &str>) -> Vec<String> {
    t.after
        .iter()
        .filter_map(|id| keys.get(id.as_str()).map(|k| k.to_string()))
        .collect()
}

/// A ticket whole, as `get_ticket` answers it (and the tools that make or change one, M4).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TicketView {
    id: String,
    key: String,
    title: String,
    description: String,
    criteria: Vec<Criterion>,
    loops: Loops,
    column: Column,
    after: Vec<String>,
    /// The features in place so far, as its agent last listed them.
    progress: Vec<String>,
    external: Option<External>,
}

#[derive(Serialize)]
struct Loops {
    /// n of « Boucle n/max » (0 before the start).
    iteration: u32,
    max: u32,
}

/// The external ticket it was imported from.
#[derive(Serialize)]
struct External {
    service: Service,
    key: String,
    url: String,
    /// Why its last sync failed.
    error: Option<String>,
}

pub(crate) fn ticket_view<R: Runtime>(core: &Core<R>, t: &Ticket) -> TicketView {
    let keys = ticket_keys(core);
    let keys: HashMap<&str, &str> = keys
        .iter()
        .map(|(id, key)| (id.as_str(), key.as_str()))
        .collect();
    TicketView {
        id: t.id.clone(),
        key: t.key.clone(),
        title: t.title.clone(),
        description: t.description.clone(),
        criteria: t.criteria.clone(),
        loops: Loops {
            iteration: t.iteration,
            max: t.max_loops,
        },
        column: t.column,
        after: after_keys(t, &keys),
        progress: t.progress.clone(),
        external: t.external.as_ref().map(|e| External {
            service: e.service,
            key: e.key.clone(),
            url: e.url.clone(),
            error: e.error.clone(),
        }),
    }
}

/// `get_ticket`: the ticket `asked` names, or without one, the caller's own (an agent's).
pub(super) fn ticket<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    asked: Option<&str>,
) -> Result<String, ToolError> {
    let t = match asked.map(str::trim).filter(|a| !a.is_empty()) {
        Some(asked) => resolve::ticket(core, asked)?,
        None => own_ticket(core, caller)?,
    };
    json(&ticket_view(core, &t))
}

/// The ticket the calling agent works on. Claude outside Escouade has none.
fn own_ticket<R: Runtime>(core: &Core<R>, caller: &Caller) -> Result<Ticket, ToolError> {
    let Caller::Agent(id) = caller else {
        return Err(ToolError::Refused(tr!(
            "Donne la clé ou l’id du ticket : seul un agent d’Escouade lit le sien sans le nommer.",
            "Give the ticket’s key or id: only an agent of Escouade reads its own without naming it."
        )));
    };
    let (name, ticket_id) = match core.agent(id) {
        Ok(h) => {
            let rt = h.lock();
            (rt.meta.name.clone(), rt.meta.ticket_id.clone())
        }
        Err(_) => (id.clone(), None),
    };
    ticket_id.and_then(|t| core.ticket(&t).ok()).ok_or_else(|| {
        ToolError::Failed(tr!(
            "L’agent {name} n’a pas de ticket : donne la clé ou l’id de celui à lire.",
            "The agent {name} has no ticket: give the key or the id of the one to read."
        ))
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Usage {
    accounts: Vec<AccountUsage>,
    autopilot_pause: Option<Pause>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountUsage {
    name: String,
    /// New agents go to it now.
    current: bool,
    five_hour: Option<RateWindow>,
    seven_day: Option<RateWindow>,
}

#[derive(Serialize)]
struct Pause {
    reason: PauseReason,
    /// When the tickets start again (epoch milliseconds).
    until: i64,
}

/// `get_usage`: each Claude account's quota windows, and the autopilot's pause.
pub(super) fn usage<R: Runtime>(core: &Core<R>) -> Result<String, ToolError> {
    let mut list = core.settings.read().accounts.clone();
    if list.is_empty() {
        list.push(accounts::principal());
    }
    let usage = core.usage.lock().clone();
    // The account new agents go to: the first active one. The windows read so far are the app's
    // only ones (whichever process answered): they are its, until each account has its own.
    let current = list.iter().position(|a| a.active).unwrap_or(0);
    let rows = list
        .into_iter()
        .enumerate()
        .map(|(i, a)| AccountUsage {
            name: a.name,
            current: i == current,
            five_hour: usage.five_hour.filter(|_| i == current),
            seven_day: usage.seven_day.filter(|_| i == current),
        })
        .collect();
    json(&Usage {
        accounts: rows,
        autopilot_pause: core.autopilot_pause().map(|p| Pause {
            reason: p.reason,
            until: p.until,
        }),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    id: String,
    name: String,
    status: AgentStatus,
    /// Its ticket's key.
    ticket: Option<String>,
    /// Its latest reply, `LAST_MESSAGE_MAX` characters at most.
    last_message: Option<String>,
    /// The latest `TOUCHED_MAX` files it began to edit, relative to its folder.
    touched_files: Vec<String>,
}

/// `get_agent_summary`: the agent `agent` names (in `project` when given), summed up.
pub(super) fn agent_summary<R: Runtime>(
    core: &Core<R>,
    agent: &str,
    project: Option<&str>,
) -> Result<String, ToolError> {
    let scope = project.map(|p| resolve::project(core, p)).transpose()?;
    let found = resolve::agent(core, agent, scope.as_ref())?;
    let h = core.agent(&found.id).map_err(|_| {
        ToolError::Failed(tr!(
            "L’agent {name} n’existe plus.",
            "The agent {name} is gone.",
            name = found.name
        ))
    })?;
    let (mut summary, ticket_id) = {
        let mut rt = h.lock();
        let last_message = rt
            .conv
            .last_where(is_reply)
            .and_then(|v| v["text"].as_str())
            .map(|t| cut_cleanly(t, LAST_MESSAGE_MAX));
        let files = &rt.meta.touched_files;
        let from = files.len().saturating_sub(TOUCHED_MAX);
        let summary = Summary {
            id: rt.meta.id.clone(),
            name: rt.meta.name.clone(),
            status: rt.meta.status,
            ticket: None,
            last_message,
            touched_files: files[from..].to_vec(),
        };
        (summary, rt.meta.ticket_id.clone())
    };
    // The agent's lock released first (the order is agents, then tickets).
    summary.ticket = ticket_id.and_then(|id| {
        core.tickets
            .read()
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.key.clone())
    });
    json(&summary)
}

/// A reply of the agent itself (its main thread's text, not a subagent's) that says something.
fn is_reply(item: &Value) -> bool {
    item["kind"] == "text"
        && item["parent"].is_null()
        && item["text"].as_str().is_some_and(|t| !t.trim().is_empty())
}

/// `text` in `max` characters at most: cut after a whole word (unless that loses a fifth of it: a
/// long word, an address), « … » in place of what follows.
pub(crate) fn cut_cleanly(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max - 1).collect();
    let whole_word = text.chars().nth(max - 1).is_some_and(char::is_whitespace);
    let cut = match head.rfind(char::is_whitespace) {
        _ if whole_word => head.trim_end(),
        Some(at) if head[..at].chars().count() >= (max - 1) * 4 / 5 => head[..at].trim_end(),
        _ => head.as_str(),
    };
    format!("{cut}…")
}

#[cfg(test)]
mod tests {
    use super::cut_cleanly;

    #[test]
    fn a_long_text_is_cut_after_a_whole_word_within_its_limit() {
        assert_eq!(cut_cleanly("  court  ", 10), "court");
        assert_eq!(cut_cleanly("un deux trois quatre", 16), "un deux trois…");
        // The limit falls between two words: the first one kept whole.
        assert_eq!(cut_cleanly("un deux trois", 8), "un deux…");
        // A single long word (an address): cut inside it rather than lose it all.
        let url = format!("voir https://example.com/{}", "a".repeat(50));
        let cut = cut_cleanly(&url, 30);
        assert_eq!(cut.chars().count(), 30);
        assert!(cut.starts_with("voir https://example.com/aaa"), "{cut}");
        assert!(cut.ends_with('…'));
        // Characters, not bytes: never a cut inside one.
        let accents = "é".repeat(30);
        assert_eq!(cut_cleanly(&accents, 10), format!("{}…", "é".repeat(9)));
    }
}
