//! What the tools that read answer (`tools`): the projects, agents, tickets and quota as the model
//! reads them, in JSON with keys in camelCase. Never a whole conversation: an agent is summed up
//! by its last message (cut) and the files it edited. The requests that wait for an answer
//! (`requests`) are the agents' own, as their cards in the conversation have them.

use super::files::{self, FileView};
use super::images::{self, ImageView};
use super::resolve::{self, Sight};
use super::tools::ToolError;
use super::workflow::{AgentWorkflow, AgentWorkflowHistory, Tracked};
use super::Caller;
use crate::accounts;
use crate::agent::{AgentRt, Waiting};
use crate::core::Core;
use crate::git;
use crate::model::{
    AgentMeta, AgentStatus, Column, Criterion, EngineId, PauseReason, PendingView, Project,
    QuotaMeter, RateWindow, RemoteSource, Service, Ticket, FIVE_HOURS, MAX_PENDING_FIELD,
    MAX_PENDING_ITEMS, WEEK,
};
use crate::plan::PlanState;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tauri::Runtime;

/// The longest last message an agent's summary gives, in characters.
pub(crate) const LAST_MESSAGE_MAX: usize = 2000;

/// Of a last message longer than that, the most its start keeps: the rest goes to its end, where
/// an agent says what it concluded or what it asks.
pub(crate) const LAST_MESSAGE_START: usize = 500;

/// The most files an agent's summary lists: the latest it began to edit.
pub(crate) const TOUCHED_MAX: usize = 100;

/// `v` as the model reads it.
pub(super) fn json<T: Serialize>(v: &T) -> Result<String, ToolError> {
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
    /// The project's Remote Control setting (« Remote control pour les agents de ce projet »).
    remote_control: bool,
    /// The global one (« Remote control pour tous les agents »), the same on every row. Read
    /// only: no tool changes either.
    remote_control_all: bool,
    /// Of a project of several repositories (ESC-169): each one and its branch.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    repos: Vec<RepoBranch>,
    /// Its harness, and the models and efforts the tools take for it (ESC-248).
    engine: Value,
    models: Vec<String>,
    efforts: Vec<String>,
}

/// A repository of a project of several, and its branch (ESC-169).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoBranch {
    repo: String,
    /// Its branch; for an agent, the one its worktree of it is on.
    branch: String,
    /// For an agent: where its work goes in that repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<String>,
}

/// An agent's repositories, as the tools say them: none for a project of one.
fn agent_repos(wt: Option<&crate::model::Worktree>) -> Vec<RepoBranch> {
    wt.map(|w| {
        w.repos
            .iter()
            .map(|r| RepoBranch {
                repo: r.repo.clone(),
                branch: w.branch.clone(),
                base: Some(r.base_branch.clone()),
            })
            .collect()
    })
    .unwrap_or_default()
}

/// Where the Remote Control of an agent comes from, as the tools say it: the three sources
/// (`RemoteSource`) while it is in force, `Off` for an agent the user set apart by hand.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum RemoteFrom {
    Hand,
    Project,
    Global,
    Off,
}

/// An agent's Remote Control as the window has it, read only (`list_agents` and
/// `get_agent_summary` flatten it into their answers).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteRead {
    /// In force now: the session is reachable from claude.ai.
    remote_control: bool,
    /// What turned it on while it is; `off` when the user set the agent apart (no setting gives it
    /// Remote Control again until they turn it on by hand); None otherwise.
    remote_source: Option<RemoteFrom>,
    /// The link's state Claude Code reports ("ready", "connected", "unavailable"), None without
    /// one.
    remote_state: Option<String>,
    /// In the Claude Code view, its interface runs and was started with Remote Control (so it is
    /// linked to claude.ai until its next launch, whatever the agent's Remote Control says now);
    /// false: it runs without; None: none runs.
    interface_remote: Option<bool>,
}

impl RemoteRead {
    /// From what the window reads of the agent: its saved state (`AgentMeta`), the link state of
    /// its process and whether its interface (if one runs) was started with Remote Control.
    fn of(meta: &AgentMeta, state: Option<&str>, interface: Option<bool>) -> Self {
        let source = if meta.remote_control {
            meta.remote_source.map(|s| match s {
                RemoteSource::Hand => RemoteFrom::Hand,
                RemoteSource::Project => RemoteFrom::Project,
                RemoteSource::Global => RemoteFrom::Global,
            })
        } else {
            meta.remote_off.then_some(RemoteFrom::Off)
        };
        Self {
            remote_control: meta.remote_control,
            remote_source: source,
            remote_state: state.map(str::to_string),
            interface_remote: interface,
        }
    }
}

/// How much an agent has running besides its turn, as the window counts it under its card
/// (`list_agents`, `get_agent_summary` and `list_background_work` flatten it into their answers).
/// An agent that finished its turn may still have a command or a subagent going on in the
/// background.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkCounts {
    /// The subagents that run, in the foreground or the background.
    subagents_running: usize,
    /// The tasks Claude Code runs in the background (commands, monitors, remote agents, teammates
    /// and workflows): what the window calls « N en fond ». Never the subagents.
    background_tasks_running: usize,
}

impl WorkCounts {
    /// From the agent's plan: none without one.
    pub(super) fn of(plan: Option<&PlanState>) -> Self {
        Self {
            subagents_running: plan.map_or(0, PlanState::running_subagents),
            background_tasks_running: plan.map_or(0, PlanState::running_background),
        }
    }
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

/// `list_projects`: every project the caller sees (`resolve::Sight`), in the window's order.
pub(super) async fn projects<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
) -> Result<String, ToolError> {
    let sight = Sight::of(core, caller);
    // One lock at a time (the order is projects before tickets, see `Core`).
    let mut projects = core.projects.read().clone();
    projects.retain(|p| sight.sees(&p.id));
    // The settings' lock after the projects' one is released: never two at a time.
    let remote_all = core.settings.read().remote_control_all;
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
        let repos: Vec<RepoBranch> = core
            .git_cache
            .read()
            .get(&p.id)
            .map(|g| {
                g.repos
                    .iter()
                    .map(|r| RepoBranch {
                        repo: r.repo.clone(),
                        branch: r.branch.clone(),
                        base: None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let branch = match cached {
            Some(branch) => branch,
            None => Some(git::head_branch(&p.path).await),
        };
        let (engine, models, efforts) = super::act::harness_choices(core, &p);
        rows.push(ProjectRow {
            engine: serde_json::to_value(engine).unwrap_or_default(),
            models,
            efforts,
            branch: branch.filter(|b| !b.is_empty() && b != "(detached)"),
            agents: agents.get(&p.id).copied().unwrap_or(0),
            tickets: tickets.remove(&p.id).unwrap_or_default(),
            remote_control: p.remote_control,
            remote_control_all: remote_all,
            repos,
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
    /// Its project's name, and its id (two projects may have the same name).
    project: String,
    project_id: String,
    status: AgentStatus,
    /// A question or a permission waits for the user's answer.
    waiting: bool,
    /// How many of them a client can answer (`list_requests` lists them).
    requests: usize,
    /// Its ticket's key.
    ticket: Option<String>,
    /// The harness it runs on (`claude`, `codex`, `copilot`, `cursor`, `opencode`).
    engine: EngineId,
    model: String,
    /// Its Claude account's name.
    account: String,
    /// In US dollars, its running turn included.
    cost: f64,
    #[serde(flatten)]
    work: WorkCounts,
    #[serde(flatten)]
    remote: RemoteRead,
    /// Where it is in its project's workflow; null when it follows none.
    workflow: Option<AgentWorkflow>,
}

/// `list_agents`: the agents not archived, oldest first, of every project the caller sees or of
/// `project`.
pub(super) fn agents<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    project: Option<&str>,
) -> Result<String, ToolError> {
    let sight = Sight::of(core, caller);
    let scope = project
        .map(|p| resolve::project(core, caller, p))
        .transpose()?;
    let projects: HashMap<String, String> = core
        .projects
        .read()
        .iter()
        .map(|p| (p.id.clone(), p.name.clone()))
        .collect();
    let keys = ticket_keys(core);
    let settings = core.settings.read().clone();
    let views: Vec<_> = core
        .agent_views()
        .into_iter()
        .filter(|v| !v.meta.archived && sight.sees(&v.meta.project_id))
        .filter(|v| scope.as_ref().is_none_or(|p| v.meta.project_id == p.id))
        .collect();
    let mut tracked: HashMap<String, Tracked> = HashMap::new();
    for v in &views {
        tracked
            .entry(v.meta.project_id.clone())
            .or_insert_with(|| Tracked::of(core, &v.meta.project_id));
    }
    let rows: Vec<AgentRow> = views
        .into_iter()
        .map(|v| AgentRow {
            workflow: tracked[&v.meta.project_id].of_agent(&v.meta.id),
            work: WorkCounts::of(v.meta.plan.as_ref()),
            remote: RemoteRead::of(&v.meta, v.remote_state.as_deref(), v.interface_remote),
            project: projects
                .get(&v.meta.project_id)
                .cloned()
                .unwrap_or_default(),
            waiting: !v.pending.is_empty(),
            requests: core
                .agent(&v.meta.id)
                .map_or(0, |h| answerable_views(&h.lock()).count()),
            ticket: v.meta.ticket_id.as_ref().and_then(|t| keys.get(t).cloned()),
            account: accounts::get(&settings, &v.meta.account).name,
            cost: dollars(v.meta.cost + v.live_cost),
            project_id: v.meta.project_id,
            status: v.meta.status,
            engine: v.meta.engine,
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

/// A shared ticket, as the tools say it (ESC-190): who shared it, whether it is the user's, who
/// took it, its file, and why its file is not read.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SharedView {
    by: String,
    by_me: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    taken_by: Option<String>,
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    problem: Option<String>,
}

pub(crate) fn shared_view(t: &Ticket) -> Option<SharedView> {
    let s = t.shared.as_ref()?;
    Some(SharedView {
        by: s.author.name.clone(),
        by_me: s.mine,
        taken_by: s.taken_by.as_ref().map(|p| p.name.clone()),
        file: format!("{}/{}.yml", crate::sharing::DIR, s.id),
        problem: s.problem.clone(),
    })
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
    /// The account (its name), the model and the effort it is set to (ESC-153): left out when
    /// « Auto ».
    #[serde(skip_serializing_if = "Option::is_none")]
    account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effort: Option<String>,
    /// Shared with the team (ESC-190): left out when it is not.
    #[serde(skip_serializing_if = "Option::is_none")]
    shared: Option<SharedView>,
}

/// The name of the account `id` a ticket is set to, as the tools say it (by its id when gone).
fn account_name<R: Runtime>(core: &Core<R>, id: &str) -> String {
    accounts::find(&core.settings.read(), id).map_or_else(
        || id.to_string(),
        |a| accounts::name_in(crate::i18n::ui(), a),
    )
}

/// `list_tickets`: the tickets of `project`, of every column or of `column`, in the board's order.
pub(super) fn tickets<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    project: &str,
    column: Option<Column>,
) -> Result<String, ToolError> {
    let project = resolve::project(core, caller, project)?;
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
            account: (!t.account.is_empty()).then(|| account_name(core, &t.account)),
            model: (!t.model.is_empty()).then(|| t.model.clone()),
            effort: (!t.effort.is_empty()).then(|| t.effort.clone()),
            shared: shared_view(t),
        })
        .collect();
    json(&rows)
}

/// The keys of the tickets « À faire » of the project, in the order the board shows them (the one
/// the autopilot starts them in).
pub(super) fn todo_keys<R: Runtime>(core: &Core<R>, project_id: &str) -> Vec<String> {
    let all = core.tickets.read().clone();
    let mut list: Vec<&Ticket> = all
        .iter()
        .filter(|t| t.project_id == project_id && t.column == Column::Todo)
        .collect();
    kanban_order(&mut list);
    list.into_iter().map(|t| t.key.clone()).collect()
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

/// What the tools say of a ticket's account, model or effort left to its default (ESC-153).
const AUTO: &str = "auto";

/// `v`, else `auto`.
fn or_auto(v: &str) -> String {
    if v.is_empty() { AUTO } else { v }.to_string()
}

/// A ticket whole, as `get_ticket` answers it (and the tools that make or change one, M4).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TicketView {
    /// Shared with the team (ESC-190): left out when it is not.
    #[serde(skip_serializing_if = "Option::is_none")]
    shared: Option<SharedView>,
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
    /// Its images, in its order: pasted in its window or attached by the tools; its agent gets
    /// them with its first message, and `get_ticket_image` gives one back.
    images: Vec<ImageView>,
    /// Its other files, of any format, in its order: joined in its window or attached by the tools;
    /// its agent is given their paths with its first message, and `get_ticket_file` reads one.
    files: Vec<FileView>,
    /// The account its agents start on (its name, `auto`: the current account of its harness),
    /// and its id; the model and the effort (`auto`: the Kanban's, else the account's) (ESC-153).
    account: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<String>,
    model: String,
    effort: String,
    /// Why it went back to « Auto » (its account removed, switched off or signed out).
    #[serde(skip_serializing_if = "Option::is_none")]
    account_lost: Option<String>,
    external: Option<External>,
    /// Its agent: None before it starts, and still named once the ticket is done (archived).
    agent: Option<TicketAgent>,
    /// Its agent's branch, else the existing one it takes up; None before its agent starts. Kept
    /// once the ticket is done.
    branch: Option<String>,
    /// Its agent's worktree, kept once the ticket is done even when the cleanup removed it.
    worktree: Option<String>,
    /// Of a project of several repositories (ESC-169): those its agent's worktree has.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    repos: Vec<RepoBranch>,
    /// Why it no longer moves on by itself.
    blocked: Option<String>,
    /// The block is a merge conflict, which `resolve_conflict` hands to its agent.
    conflict: bool,
    /// The validation step running (« Validation… » or « Approving… », the first being stored in
    /// the interface's language of the moment, « Tests… », « Commit… », « Merge… », « Push… »):
    /// None when no approval runs.
    step: Option<String>,
    /// What its approval did, once it is done.
    outcome: Option<Outcome>,
}

/// A ticket's agent, as `get_ticket` sums it up.
#[derive(Serialize)]
struct TicketAgent {
    id: String,
    /// None, with the status, for an agent that no longer exists.
    name: Option<String>,
    status: Option<AgentStatus>,
    /// How many requests wait for an answer, as `list_agents` counts them.
    requests: usize,
}

/// What a ticket's approval did: what the card says, the pull request it opened, the commit it
/// made on the target.
#[derive(Serialize)]
struct Outcome {
    text: String,
    url: Option<String>,
    commit: Option<String>,
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

/// `t` as `get_ticket` answers it. Reads the tickets (the keys of those it comes after) and its
/// agent: never call it with the tickets' lock held (a write one would deadlock, a read one too
/// behind a writer waiting for it), nor with that agent's. It also looks at the size of each
/// image's file (20 at most).
pub(crate) fn ticket_view<R: Runtime>(core: &Core<R>, t: &Ticket) -> TicketView {
    let keys = ticket_keys(core);
    let keys: HashMap<&str, &str> = keys
        .iter()
        .map(|(id, key)| (id.as_str(), key.as_str()))
        .collect();
    let mut worktree = None;
    let agent = t.agent_id.as_ref().map(|id| {
        let Ok(h) = core.agent(id) else {
            return TicketAgent {
                id: id.clone(),
                name: None,
                status: None,
                requests: 0,
            };
        };
        let rt = h.lock();
        worktree = rt.meta.worktree.clone();
        TicketAgent {
            id: id.clone(),
            name: Some(rt.meta.name.clone()),
            status: Some(rt.meta.status),
            requests: answerable_views(&rt).count(),
        }
    });
    TicketView {
        shared: shared_view(t),
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
        images: images::listed(&core.data.ticket_dir(&t.id), &t.images),
        files: files::listed(&core.data.ticket_dir(&t.id), &t.files),
        account: match t.account.as_str() {
            "" => AUTO.into(),
            id => account_name(core, id),
        },
        account_id: (!t.account.is_empty()).then(|| t.account.clone()),
        model: or_auto(&t.model),
        effort: or_auto(&t.effort),
        account_lost: t.account_lost.clone(),
        external: t.external.as_ref().map(|e| External {
            service: e.service,
            key: e.key.clone(),
            url: e.url.clone(),
            error: e.error.clone(),
        }),
        agent,
        branch: worktree
            .as_ref()
            .map(|w| w.branch.clone())
            .or_else(|| Some(t.branch.clone()).filter(|b| !b.is_empty())),
        repos: agent_repos(worktree.as_ref()),
        worktree: worktree.map(|w| w.path),
        blocked: t.blocked.clone(),
        conflict: t.conflict,
        step: t.step.clone(),
        outcome: t.outcome.as_ref().map(|text| Outcome {
            text: text.clone(),
            url: t.outcome_url.clone(),
            commit: t.outcome_commit.clone(),
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
        Some(asked) => resolve::ticket(core, caller, asked)?,
        None => own_ticket(core, caller)?,
    };
    json(&ticket_view(core, &t))
}

/// A project's Kanban settings, as `get_board` answers them (and `set_board` after a change): the
/// names `SetBoardArgs` takes, so that the answer can be given back as it is.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BoardView {
    project: String,
    /// Its id, which names it too (`set_board` takes either, and ignores this key).
    project_id: String,
    /// "merge", "pr", "push" or "keep".
    action: String,
    /// Where tickets merge: the Kanban's, else the branch the project is on (empty on a detached
    /// HEAD).
    target: String,
    /// "merge", "squash" or "rebase".
    strategy: String,
    draft: bool,
    cleanup: bool,
    conventional: bool,
    /// "ask", "agent" or "abort".
    conflict: String,
    max_parallel: u32,
    autopilot: bool,
    /// What the Kanban sets for its tickets' agents; empty: the app's (`defaults`).
    model: String,
    effort: String,
    mode: String,
    defaults: BoardDefaults,
    /// On only with a test command, as the window shows it.
    tests_first: bool,
    test_command: String,
    prefix: String,
}

/// What the tickets' agents go with when the Kanban sets nothing: those of the account a new agent
/// of the project goes to (ESC-128), else its harness's.
#[derive(Serialize)]
struct BoardDefaults {
    model: String,
    effort: String,
    mode: String,
}

/// `project`'s Kanban settings as `get_board` answers them. Async for the branch the project is
/// on, which is the target while the Kanban has none (the first ticket fixes it).
pub(crate) async fn board_view<R: Runtime>(core: &Core<R>, project: &Project) -> BoardView {
    let b = &project.board;
    let target = if b.target.is_empty() {
        git::head_branch(&project.path).await
    } else {
        b.target.clone()
    };
    BoardView {
        project: project.name.clone(),
        project_id: project.id.clone(),
        action: b.action.clone(),
        target,
        strategy: b.strategy.clone(),
        draft: b.draft,
        cleanup: b.cleanup,
        conventional: b.conventional,
        conflict: b.conflict.clone(),
        max_parallel: b.max_parallel.clamp(1, 6),
        autopilot: b.autopilot,
        model: b.model.clone(),
        effort: b.effort.clone(),
        mode: b.mode.clone(),
        defaults: {
            let d = core.new_agent_defaults(&project.id, project.engine());
            BoardDefaults {
                model: d.model,
                effort: d.effort,
                mode: d.mode,
            }
        },
        tests_first: b.tests_first_on(),
        test_command: b.test_command.clone(),
        prefix: b.prefix.clone(),
    }
}

/// `get_board`: the Kanban settings of `project`.
pub(super) async fn board<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    project: &str,
) -> Result<String, ToolError> {
    let project = resolve::project(core, caller, project)?;
    json(&board_view(core, &project).await)
}

/// The ticket the calling agent works on. Claude outside Escouade has none.
pub(super) fn own_ticket<R: Runtime>(core: &Core<R>, caller: &Caller) -> Result<Ticket, ToolError> {
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
    /// Its harness.
    engine: EngineId,
    /// New agents of its harness go to it now.
    current: bool,
    /// Its quota's meters as last read (`QuotaMeter`): windows by their length, the month's
    /// credits; none for a harness without a quota.
    meters: Vec<QuotaMeter>,
    /// Claude Code's two windows, as before 1.8.
    five_hour: Option<RateWindow>,
    seven_day: Option<RateWindow>,
}

#[derive(Serialize)]
struct Pause {
    reason: PauseReason,
    /// The window's length, for a window.
    #[serde(skip_serializing_if = "Option::is_none")]
    minutes: Option<u32>,
    /// When the tickets start again (epoch milliseconds).
    until: i64,
}

/// `get_usage`: each account's quota, of every harness, and the autopilot's pause.
pub(super) fn usage<R: Runtime>(core: &Core<R>) -> Result<String, ToolError> {
    let list = core.settings.read().accounts.clone();
    // Each account's windows as last read, and the account new agents go to as the status bar
    // and the autopilot have it (`UsageSnapshot::settle`: the first good one, `pace`, else the
    // first whose only fault is its pace, else the first active one).
    let usage = core.usage.lock().clone();
    let rows = list
        .into_iter()
        .map(|a| {
            let read = usage.account(&a.id);
            AccountUsage {
                current: usage.current.get(&a.engine) == Some(&a.id),
                meters: read.map(|u| u.meters.clone()).unwrap_or_default(),
                five_hour: read.and_then(|u| u.window(FIVE_HOURS)),
                seven_day: read.and_then(|u| u.window(WEEK)),
                engine: a.engine,
                name: a.name,
            }
        })
        .collect();
    json(&Usage {
        accounts: rows,
        autopilot_pause: core.autopilot_pause().map(|p| Pause {
            reason: p.reason,
            minutes: p.minutes,
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
    /// The harness it runs on, as `list_agents` says it.
    engine: EngineId,
    /// Its ticket's key.
    ticket: Option<String>,
    /// Its latest reply, `LAST_MESSAGE_MAX` characters at most: a longer one by its start and
    /// its end (`start_and_end`). None before its first.
    last_message: Option<String>,
    /// The latest `TOUCHED_MAX` files it began to edit, relative to its folder.
    touched_files: Vec<String>,
    /// How many requests wait for an answer (`list_requests` lists them), and their types in the
    /// order they came.
    requests: usize,
    waiting_for: Vec<&'static str>,
    #[serde(flatten)]
    work: WorkCounts,
    /// Of a project of several repositories (ESC-169): its worktree of each, and their bases.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    repos: Vec<RepoBranch>,
    #[serde(flatten)]
    remote: RemoteRead,
    /// Where it is in its project's workflow and the steps it went through; null when it follows
    /// none.
    workflow: Option<AgentWorkflowHistory>,
}

/// `get_agent_summary`: the agent `agent` names (in `project` when given), summed up.
pub(super) fn agent_summary<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    agent: &str,
    project: Option<&str>,
) -> Result<String, ToolError> {
    let scope = project
        .map(|p| resolve::project(core, caller, p))
        .transpose()?;
    let found = resolve::agent(core, caller, agent, scope.as_ref())?;
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
            .map(|t| start_and_end(t, LAST_MESSAGE_MAX, LAST_MESSAGE_START));
        let files = &rt.meta.touched_files;
        let asked: Vec<&'static str> = answerable_views(&rt).map(request_type).collect();
        let from = files.len().saturating_sub(TOUCHED_MAX);
        let summary = Summary {
            id: rt.meta.id.clone(),
            name: rt.meta.name.clone(),
            status: rt.meta.status,
            engine: rt.meta.engine,
            ticket: None,
            last_message,
            touched_files: files[from..].to_vec(),
            requests: asked.len(),
            waiting_for: asked,
            work: WorkCounts::of(rt.meta.plan.as_ref()),
            repos: agent_repos(rt.meta.worktree.as_ref()),
            remote: RemoteRead::of(
                &rt.meta,
                rt.remote_state.as_deref(),
                rt.tui.as_ref().map(|t| t.remote),
            ),
            workflow: None,
        };
        (summary, rt.meta.ticket_id.clone())
    };
    // The agent's lock released first (the order is agents, then tickets).
    summary.workflow = Tracked::of(core, &found.project_id).history_of(&found.id);
    summary.ticket = ticket_id.and_then(|id| {
        core.tickets
            .read()
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.key.clone())
    });
    json(&summary)
}

// ---------- the requests that wait for an answer ----------

/// The longest plan, command or text of an input `list_requests` gives, in characters: the
/// overview's 2,000 would hide the most of one, and a plan is approved, a command run, on what is
/// read of it, a client having nowhere else to read the request.
const LONG_MAX: usize = 20_000;

/// Of a plan or a command longer than that, the most its start keeps: the rest goes to its end.
const LONG_START: usize = 4_000;

/// The views of the requests of the agent a client may answer (`answerable`), without copying.
pub(super) fn answerable_views(rt: &AgentRt) -> impl Iterator<Item = &PendingView> {
    let open = !rt.meta.terminal && rt.proc.is_some();
    rt.waiting_views().filter(move |_| open)
}

/// The requests of the agent a client may answer: those its process waits for, which are in the
/// conversation's cards. In the Claude Code view the interface asks in its terminal, and a request
/// of a process that is gone is none any more (`on_exit` drops them): either way, none.
pub(super) fn answerable(rt: &AgentRt) -> Vec<Waiting> {
    if rt.meta.terminal || rt.proc.is_none() {
        return Vec::new();
    }
    rt.waiting()
}

/// The tools that run what their `command` says (the keys of their input are the same).
const COMMAND_TOOLS: &[&str] = &["Bash", "PowerShell", "SlashCommand"];

/// The most items of a list, or keys of an object, an input gives.
const INPUT_ITEMS_MAX: usize = 50;

/// `input` as a request gives it: every text capped (`LONG_MAX` characters), a list or an object
/// to `INPUT_ITEMS_MAX` items; `cut` is set when something was.
fn bounded(input: &Value, cut: &mut bool) -> Value {
    match input {
        Value::String(s) => Value::String(capped_to(s, LONG_MAX, cut)),
        Value::Array(items) => {
            *cut |= items.len() > INPUT_ITEMS_MAX;
            Value::Array(
                items
                    .iter()
                    .take(INPUT_ITEMS_MAX)
                    .map(|i| bounded(i, cut))
                    .collect(),
            )
        }
        Value::Object(map) => {
            *cut |= map.len() > INPUT_ITEMS_MAX;
            Value::Object(
                map.iter()
                    .take(INPUT_ITEMS_MAX)
                    .map(|(k, v)| (k.clone(), bounded(v, cut)))
                    .collect(),
            )
        }
        other => other.clone(),
    }
}

/// What a request is, as the tools say it: `permission`, `question`, or `plan` for the plan an
/// agent asks to have approved (a permission of `ExitPlanMode`).
pub(super) fn request_type(view: &PendingView) -> &'static str {
    match (view.kind.as_str(), view.tool.as_str()) {
        ("question", _) => "question",
        (_, "ExitPlanMode") => "plan",
        _ => "permission",
    }
}

/// One question of an `AskUserQuestion`, as Claude Code sent it.
pub(super) struct Asked {
    pub question: String,
    pub header: Option<String>,
    /// Several options may be picked.
    pub multi_select: bool,
    pub options: Vec<AskedOption>,
}

pub(super) struct AskedOption {
    pub label: String,
    pub description: Option<String>,
}

/// The questions of the input of an `AskUserQuestion`, all of them.
pub(super) fn asked_questions(input: &Value) -> Vec<Asked> {
    let text = |v: &Value| v.as_str().filter(|s| !s.is_empty()).map(str::to_string);
    let list = |v: &Value| v.as_array().cloned().unwrap_or_default();
    list(&input["questions"])
        .iter()
        .filter_map(|q| {
            Some(Asked {
                question: q["question"].as_str()?.to_string(),
                header: text(&q["header"]),
                multi_select: q["multiSelect"].as_bool().unwrap_or(false),
                options: list(&q["options"])
                    .iter()
                    .filter_map(|o| {
                        Some(AskedOption {
                            label: o["label"].as_str()?.to_string(),
                            description: text(&o["description"]),
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RequestAgent {
    id: String,
    name: String,
    /// Its project's name, and its id (two projects may have the same name).
    project: String,
    project_id: String,
    /// Its ticket's key.
    ticket: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QuestionRow {
    question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    header: Option<String>,
    multi_select: bool,
    options: Vec<OptionRow>,
}

#[derive(Serialize)]
struct OptionRow {
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestRow {
    /// The request's id, which `answer_permission` and `answer_question` name.
    request: String,
    #[serde(rename = "type")]
    kind: &'static str,
    agent: RequestAgent,
    /// When it came (epoch milliseconds).
    since: i64,
    tool: String,
    /// What the tool is asked to act on (the whole plan, for a plan); none for a question.
    #[serde(skip_serializing_if = "String::is_empty")]
    arg: String,
    /// What Claude Code sent, bounded, for a permission: `arg` sums it up, this is all it asks
    /// (the content to write, the flags of a command…).
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<Value>,
    description: Option<String>,
    /// The permission rule Claude Code met.
    reason: Option<String>,
    /// Claude Code would refuse it by default.
    default_no: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    questions: Vec<QuestionRow>,
    /// A text was cut: the card in the conversation has it whole.
    cut: bool,
}

/// `text` in `max` characters at most; `cut` is set when it was longer.
fn capped_to(text: &str, max: usize, cut: &mut bool) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    *cut = true;
    text.chars().take(max).collect()
}

/// `text` in `MAX_PENDING_FIELD` characters at most, as the agent's view caps what it says.
fn capped(text: &str, cut: &mut bool) -> String {
    capped_to(text, MAX_PENDING_FIELD, cut)
}

/// `w` as `list_requests` gives it: the agent's view of it (all it says besides the questions),
/// the questions with what the view leaves out, a plan or a command up to `LONG_MAX` characters,
/// and the input of a permission.
fn request_row(agent: RequestAgent, w: Waiting) -> RequestRow {
    let kind = request_type(&w.view);
    let PendingView {
        id,
        tool,
        arg,
        description,
        reason,
        cut,
        default_no,
        ..
    } = w.view;
    let (mut arg, mut cut) = (arg, cut);
    let mut questions = Vec::new();
    let mut input = None;
    if kind == "question" {
        let all = asked_questions(&w.input);
        cut |= all.len() > MAX_PENDING_ITEMS;
        for q in all.into_iter().take(MAX_PENDING_ITEMS) {
            cut |= q.options.len() > MAX_PENDING_ITEMS;
            questions.push(QuestionRow {
                question: capped(&q.question, &mut cut),
                header: q.header.map(|h| capped(&h, &mut cut)),
                multi_select: q.multi_select,
                options: q
                    .options
                    .into_iter()
                    .take(MAX_PENDING_ITEMS)
                    .map(|o| OptionRow {
                        label: capped(&o.label, &mut cut),
                        description: o.description.map(|d| capped(&d, &mut cut)),
                    })
                    .collect(),
            });
        }
    } else {
        // The view caps `arg` at what the overview shows. A plan is approved, and a command run,
        // on what is read of it: they are given up to `LONG_MAX`. The view's `cut` held their
        // cut, among others: it stays only when the text fitted the view's cap (something else
        // was cut).
        let whole = match (kind, tool.as_str()) {
            ("plan", _) => w.input["plan"].as_str(),
            (_, t) if COMMAND_TOOLS.contains(&t) => w.input["command"].as_str(),
            _ => None,
        };
        if let Some(text) = whole {
            let shown = start_and_end(text, LONG_MAX, LONG_START);
            let text_cut = shown != text.trim();
            cut = text_cut || (cut && text.chars().count() <= MAX_PENDING_FIELD);
            arg = shown;
        }
        if kind == "permission" {
            input = Some(bounded(&w.input, &mut cut));
        }
    }
    RequestRow {
        request: id,
        kind,
        agent,
        since: w.since,
        tool,
        arg,
        input,
        description,
        reason,
        default_no,
        questions,
        cut,
    }
}

/// `list_requests`: the requests waiting for an answer, in the order they came, of the agent
/// `agent` names (in `project` when given), of the agents of `project`, or of every agent not
/// archived of the projects the caller sees.
pub(super) fn requests<R: Runtime>(
    core: &Core<R>,
    caller: &Caller,
    agent: Option<&str>,
    project: Option<&str>,
) -> Result<String, ToolError> {
    let sight = Sight::of(core, caller);
    let scope = project
        .map(|p| resolve::project(core, caller, p))
        .transpose()?;
    let only = agent
        .map(|a| resolve::agent(core, caller, a, scope.as_ref()))
        .transpose()?;
    let projects: HashMap<String, String> = core
        .projects
        .read()
        .iter()
        .map(|p| (p.id.clone(), p.name.clone()))
        .collect();
    let keys = ticket_keys(core);
    // The agents' handles taken out of the map, to lock them one at a time.
    let handles: Vec<_> = core.agents.read().values().cloned().collect();
    let mut agents: Vec<(i64, String, RequestAgent, Vec<Waiting>)> = Vec::new();
    for h in handles {
        let rt = h.lock();
        let m = &rt.meta;
        let wanted = match &only {
            Some(a) => m.id == a.id,
            None => {
                !m.archived
                    && sight.sees(&m.project_id)
                    && scope.as_ref().is_none_or(|p| m.project_id == p.id)
            }
        };
        if !wanted {
            continue;
        }
        let who = RequestAgent {
            id: m.id.clone(),
            name: m.name.clone(),
            project: projects.get(&m.project_id).cloned().unwrap_or_default(),
            project_id: m.project_id.clone(),
            ticket: m.ticket_id.as_ref().and_then(|t| keys.get(t).cloned()),
        };
        agents.push((m.created_at, m.id.clone(), who, answerable(&rt)));
    }
    agents.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    // Each agent's in its order; across agents, by when they came (the sort keeps the order of
    // those that came at the same millisecond).
    let mut rows: Vec<RequestRow> = agents
        .into_iter()
        .flat_map(|(_, _, who, waiting)| waiting.into_iter().map(move |w| (who.clone(), w)))
        .map(|(who, w)| request_row(who, w))
        .collect();
    rows.sort_by_key(|r| r.since);
    json(&rows)
}

/// A reply of the agent itself (its main thread's text, not a subagent's) that says something.
fn is_reply(item: &Value) -> bool {
    item["kind"] == "text"
        && item["parent"].is_null()
        && item["text"].as_str().is_some_and(|t| !t.trim().is_empty())
}

/// What stands between the start and the end of a text too long to give whole.
const ELIDED: &str = " … ";

/// `text` in `max` characters at most: whole when it fits, else its start (`start_max` characters
/// at most) and its end (the rest), each cut on a whole word, « … » between them.
pub(crate) fn start_and_end(text: &str, max: usize, start_max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let end_max = max.saturating_sub(start_max + ELIDED.chars().count());
    format!(
        "{}{ELIDED}{}",
        start_words(text, start_max),
        end_words(text, end_max)
    )
}

/// The start of `text`, `max` characters at most, ending with a whole word (inside one only when
/// the last space would lose a fifth of it: a long word, an address).
fn start_words(text: &str, max: usize) -> &str {
    let cut = text.char_indices().nth(max).map_or(text.len(), |(i, _)| i);
    let head = &text[..cut];
    if cut == text.len() || text[cut..].starts_with(char::is_whitespace) {
        return head.trim_end();
    }
    match head.rfind(char::is_whitespace) {
        Some(at) if head[..at].chars().count() >= max * 4 / 5 => head[..at].trim_end(),
        _ => head,
    }
}

/// The end of `text`, `max` characters at most, starting with a whole word (inside one only when
/// the first space would lose a fifth of it).
fn end_words(text: &str, max: usize) -> &str {
    let count = text.chars().count();
    let cut = match count.checked_sub(max) {
        Some(skip) if skip > 0 => text.char_indices().nth(skip).map_or(0, |(i, _)| i),
        _ => 0,
    };
    let tail = &text[cut..];
    if cut == 0 || text[..cut].ends_with(char::is_whitespace) {
        return tail.trim_start();
    }
    match tail.find(char::is_whitespace) {
        Some(at) if tail[at..].chars().count() >= max * 4 / 5 => tail[at..].trim_start(),
        _ => tail,
    }
}

#[cfg(test)]
mod tests {
    use super::start_and_end;

    #[test]
    fn a_long_text_keeps_its_start_and_its_end_each_cut_on_a_whole_word() {
        assert_eq!(start_and_end("  court  ", 10, 4), "court");
        let words: Vec<String> = (0..100).map(|i| format!("m{i:02}")).collect();
        let text = words.join(" ");
        assert_eq!(
            start_and_end(&text, 40, 12),
            "m00 m01 m02 … m94 m95 m96 m97 m98 m99"
        );
        // Both limits fall between two words: each one kept whole.
        assert_eq!(
            start_and_end("un deux trois quatre cinq", 14, 7),
            "un deux … cinq"
        );
        // A long word (an address) at the end: cut inside it rather than lose it all.
        let url = format!(
            "{}voir https://example.com/{}",
            "début ".repeat(20),
            "a".repeat(60)
        );
        assert_eq!(
            start_and_end(&url, 60, 12),
            format!("début début … {}", "a".repeat(45))
        );
        // Characters, not bytes: never a cut inside one.
        assert_eq!(
            start_and_end(&"é".repeat(100), 20, 5),
            format!("{} … {}", "é".repeat(5), "é".repeat(12))
        );
    }
}
