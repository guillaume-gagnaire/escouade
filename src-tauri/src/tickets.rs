//! The board on the core: tickets saved with the app, the scheduler that starts them with agents
//! of their own, what each end of turn does to them, their validation (tests, commit, merge, pull
//! request, push) and the test launches of the worktrees.

use crate::board::{self, TurnEnd};
use crate::claude::{self, ClaudeProcess};
use crate::core::{AgentOptions, Core};
use crate::git;
use crate::model::*;
use crate::testlaunch;
use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::Runtime;

/// Why a ticket "En cours" stops when its agent's automatic resume after the usage limit is gone
/// (turned off, cancelled, never planned): it would otherwise hold its place forever.
const QUOTA_LOST: &str = "limite d'usage atteinte";

/// The reason a ticket is blocked by an error, on one line (a git error may run over many; the
/// whole error goes to the log): the contexts the app gave, then git's `fatal:` or `error:` line,
/// the one that tells why, when there is one; else the first line, as `board::turn_end` does with
/// an agent's.
pub(crate) fn error_reason(e: &anyhow::Error) -> String {
    format!("Erreur : {}", error_line(e))
}

/// An error on one line, as `error_reason` tells it: a failed validation step is blocked with it
/// as it is (the app's own refusals read as they are written).
pub(crate) fn error_line(e: &anyhow::Error) -> String {
    let layers: Vec<String> = e
        .chain()
        .map(|c| c.to_string())
        .filter(|l| !l.trim().is_empty())
        .collect();
    let says_why = |l: &&str| l.contains("fatal:") || l.contains("error:");
    for (i, layer) in layers.iter().enumerate() {
        if let Some(why) = layer.lines().map(str::trim).find(says_why) {
            let mut parts: Vec<String> = layers[..i].iter().map(|l| board::first_line(l)).collect();
            parts.push(why.to_string());
            return board::first_line(&parts.join(": "));
        }
    }
    board::first_line(&format!("{e:#}"))
}

/// The tests run before a validation stop after this.
const TEST_LIMIT: Duration = Duration::from_secs(20 * 60);

/// A ticket as its form gives it.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TicketDraft {
    pub title: String,
    pub description: String,
    /// One per line of the form; empty lines are dropped.
    pub criteria: Vec<String>,
    pub max_loops: u32,
}

impl<R: Runtime> Core<R> {
    // ---------- tickets ----------

    pub fn ticket(&self, id: &str) -> Result<Ticket> {
        self.tickets
            .read()
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| anyhow!("ticket introuvable"))
    }

    /// Changes a ticket under the lock, then tells the window and saves. `f` runs with the tickets
    /// locked: it must not take the projects' lock (projects come before tickets).
    pub(crate) fn edit_ticket<T>(
        &self,
        id: &str,
        f: impl FnOnce(&mut Ticket) -> Result<T>,
    ) -> Result<T> {
        let (out, ticket) = {
            let mut tickets = self.tickets.write();
            let t = tickets
                .iter_mut()
                .find(|t| t.id == id)
                .ok_or_else(|| anyhow!("ticket introuvable"))?;
            let out = f(t)?;
            (out, t.clone())
        };
        self.hub.emit(UiEvent::Ticket { ticket });
        self.request_save();
        Ok(out)
    }

    fn emit_project(&self, project_id: &str) {
        if let Ok(project) = self.project(project_id) {
            self.hub.emit(UiEvent::Project { project });
        }
    }

    /// A ticket "À faire" at the end of the column. The first one fixes the key's prefix and the
    /// target branch when not set: the one the project is on, even without any commit yet (a new
    /// repository); on a detached HEAD, none, so the ticket is refused until one is chosen.
    pub async fn ticket_create(
        self: &Arc<Self>,
        project_id: &str,
        d: TicketDraft,
    ) -> Result<Ticket> {
        let title = d.title.trim().to_string();
        if title.is_empty() {
            bail!("Un ticket a besoin d'un titre.");
        }
        let project = self.project(project_id)?;
        let current = git::head_branch(&project.path).await;
        let key = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(|| anyhow!("projet introuvable"))?;
            if p.board.target.is_empty() {
                if current.is_empty() {
                    bail!(board::NO_BRANCH);
                }
                p.board.target = current;
            }
            if p.board.prefix.is_empty() {
                p.board.prefix = board::key_prefix(&p.name);
            }
            let n = p.board.next_number.max(1);
            p.board.next_number = n + 1;
            format!("{}-{n}", p.board.prefix)
        };
        let mut ticket = Ticket {
            id: new_id(),
            project_id: project_id.to_string(),
            key,
            title,
            description: d.description.trim().to_string(),
            criteria: board::criteria_from(&d.criteria),
            max_loops: board::max_loops(d.max_loops),
            created_at: now_ms(),
            ..Default::default()
        };
        {
            // Lock order: projects, then tickets. The project is checked again while both are
            // held (the rank and the push in one go): `remove_project`'s write of the projects
            // waits for this, and it takes the project's tickets once it is out.
            let projects = self.projects.read();
            if !projects.iter().any(|p| p.id == project_id) {
                bail!("projet introuvable");
            }
            let mut tickets = self.tickets.write();
            ticket.rank = tickets
                .iter()
                .filter(|t| t.project_id == project_id)
                .map(|t| t.rank)
                .max()
                .unwrap_or(0)
                + 1;
            tickets.push(ticket.clone());
        }
        self.emit_project(project_id);
        self.hub.emit(UiEvent::Ticket {
            ticket: ticket.clone(),
        });
        self.request_save();
        self.schedule();
        Ok(ticket)
    }

    /// Only a ticket "À faire" changes.
    pub fn ticket_update(self: &Arc<Self>, id: &str, d: TicketDraft) -> Result<Ticket> {
        let title = d.title.trim().to_string();
        if title.is_empty() {
            bail!("Un ticket a besoin d'un titre.");
        }
        let ticket = self.edit_ticket(id, |t| {
            if t.column != Column::Todo {
                bail!("Seul un ticket « À faire » se modifie.");
            }
            t.title = title;
            t.description = d.description.trim().to_string();
            t.criteria = board::criteria_from(&d.criteria);
            t.max_loops = board::max_loops(d.max_loops);
            Ok(t.clone())
        })?;
        self.schedule();
        Ok(ticket)
    }

    /// "Passer en tête": only a ticket "À faire" has a rank to change.
    pub fn ticket_prioritize(self: &Arc<Self>, id: &str) -> Result<()> {
        let t = self.ticket(id)?;
        let first = self
            .tickets
            .read()
            .iter()
            .filter(|x| x.project_id == t.project_id && x.column == Column::Todo)
            .map(|x| x.rank)
            .min()
            .unwrap_or(0);
        self.edit_ticket(id, |t| {
            if t.column != Column::Todo {
                bail!("Seul un ticket « À faire » passe en tête.");
            }
            t.rank = first - 1;
            Ok(())
        })?;
        self.schedule();
        Ok(())
    }

    /// "Lancer": a ticket that starts even with the autopilot off.
    pub fn ticket_start(self: &Arc<Self>, id: &str) -> Result<()> {
        self.edit_ticket(id, |t| {
            if t.column != Column::Todo {
                bail!("Ce ticket est déjà parti.");
            }
            t.forced = true;
            Ok(())
        })?;
        self.schedule();
        Ok(())
    }

    /// The ticket goes; one "En cours" or "À tester" has its agent archived (worktree kept). The
    /// card is gone for good: an agent that cannot be archived is logged, not reported.
    pub async fn ticket_delete(self: &Arc<Self>, id: &str) -> Result<()> {
        let t = {
            let mut tickets = self.tickets.write();
            let i = tickets
                .iter()
                .position(|x| x.id == id)
                .ok_or_else(|| anyhow!("ticket introuvable"))?;
            tickets.remove(i)
        };
        self.hub.emit(UiEvent::TicketRemoved {
            id: id.to_string(),
            project_id: t.project_id.clone(),
        });
        self.request_save();
        if matches!(t.column, Column::Doing | Column::Review) {
            if let Some(a) = t.agent_id.as_deref().filter(|a| self.agent(a).is_ok()) {
                if let Err(e) = self.archive_agent(a, true).await {
                    log::warn!("{}: its agent could not be archived: {e:#}", t.key);
                }
            }
        }
        self.schedule();
        Ok(())
    }

    /// The board's settings as the window sends them: the key's prefix and number stay the
    /// backend's, so does the target when the window has none (its copy is older than the first
    /// ticket), a choice it does not know falls back to the default, and the agents in parallel
    /// stay between 1 and 6.
    pub fn board_set(self: &Arc<Self>, project_id: &str, s: BoardSettings) -> Result<Project> {
        let default = BoardSettings::default();
        let one_of = |v: String, known: &[&str], fallback: String| {
            if known.contains(&v.as_str()) {
                v
            } else {
                fallback
            }
        };
        let project = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(|| anyhow!("projet introuvable"))?;
            let target = s.target.trim();
            p.board = BoardSettings {
                prefix: p.board.prefix.clone(),
                next_number: p.board.next_number,
                target: if target.is_empty() {
                    p.board.target.clone()
                } else {
                    target.to_string()
                },
                action: one_of(s.action, &["merge", "pr", "push", "keep"], default.action),
                strategy: one_of(s.strategy, &["merge", "squash", "rebase"], default.strategy),
                conflict: one_of(s.conflict, &["ask", "agent", "abort"], default.conflict),
                max_parallel: s.max_parallel.clamp(1, 6),
                ..s
            };
            p.clone()
        };
        self.hub.emit(UiEvent::Project {
            project: project.clone(),
        });
        self.request_save();
        self.schedule();
        Ok(project)
    }

    pub async fn git_branches(&self, project_id: &str) -> Result<Vec<String>> {
        let p = self.project(project_id)?;
        git::branches(&p.path).await
    }

    /// The project is closed: its tickets go with it, and what its board said.
    pub(crate) fn drop_project_tickets(&self, project_id: &str) {
        self.board_issues.lock().remove(project_id);
        let gone: Vec<String> = {
            let mut tickets = self.tickets.write();
            let ids = tickets
                .iter()
                .filter(|t| t.project_id == project_id)
                .map(|t| t.id.clone())
                .collect();
            tickets.retain(|t| t.project_id != project_id);
            ids
        };
        for id in gone {
            self.hub.emit(UiEvent::TicketRemoved {
                id,
                project_id: project_id.to_string(),
            });
        }
    }

    // ---------- scheduler ----------

    /// Starts what may start, in the background.
    pub fn schedule(self: &Arc<Self>) {
        #[cfg(test)]
        self.passes_queued.fetch_add(1, Ordering::SeqCst);
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            c.schedule_now().await;
            #[cfg(test)]
            c.passes_queued.fetch_sub(1, Ordering::SeqCst);
        });
    }

    /// An agent of the app waits for its quota (usage limit): no ticket starts meanwhile.
    pub(crate) fn quota_paused(&self) -> bool {
        self.agents.read().values().any(|h| {
            let rt = h.lock();
            rt.meta.resume_at.is_some() && !rt.meta.archived
        })
    }

    /// One pass at a time: the tickets "À faire" that fit start, project by project.
    pub async fn schedule_now(self: &Arc<Self>) {
        let _pass = self.board_lock.lock().await;
        // Without Claude Code no agent can work: rather than make every ticket's worktree and
        // agent only to block it, none starts until it is found (saving the settings looks again).
        let claude_path = self.settings.read().claude_path.clone();
        if claude::resolve_binary(&claude_path).is_none() {
            if !self.claude_missing.swap(true, Ordering::AcqRel) {
                log::info!("board: Claude Code not found, no ticket starts until it is");
            }
            return;
        }
        self.claude_missing.store(false, Ordering::Release);
        let paused = self.quota_paused();
        let projects = self.projects.read().clone();
        for p in projects {
            let ids = board::to_start(&self.tickets.read(), &p.id, &p.board, paused);
            // A board that told why nothing starts is looked at again even with nothing to start:
            // its issue goes once its target is there.
            if ids.is_empty() && !self.board_issues.lock().contains_key(&p.id) {
                continue;
            }
            // Every ticket's branch starts from the target: while it has no commit yet (a new
            // repository) or is gone (deleted, renamed), none starts, rather than each make an
            // agent only to block. The window is told why; choosing another target, any change
            // of the board, or the project's next git refresh (a first commit, the branch made
            // again) looks again.
            let target = self.target_of(&p).await;
            let exists = !target.is_empty() && git::branch_exists(&p.path, &target).await;
            let unborn = !exists && git::head_branch(&p.path).await == target;
            let issue = board::target_issue(&target, exists, unborn);
            if self.set_board_issue(&p.id, issue.clone()) {
                if let Some(why) = &issue {
                    log::info!("board of {}: {why}", p.name);
                }
            }
            if issue.is_some() {
                continue;
            }
            for id in ids {
                self.start_ticket(&p, &id).await;
            }
        }
    }

    /// Why the project's board starts nothing, as the window is told (only when it changes):
    /// true when it changed.
    fn set_board_issue(&self, project_id: &str, issue: Option<String>) -> bool {
        let changed = {
            let mut issues = self.board_issues.lock();
            let changed = issues.get(project_id) != issue.as_ref();
            match &issue {
                Some(why) => issues.insert(project_id.to_string(), why.clone()),
                None => issues.remove(project_id),
            };
            changed
        };
        if changed {
            self.hub.emit(UiEvent::BoardIssue {
                project_id: project_id.to_string(),
                issue,
            });
        }
        changed
    }

    /// The board's target branch. A board saved without one (its first ticket made by an earlier
    /// version, on a branch-less HEAD) takes the branch the project is on, for good: it no longer
    /// moves with what is checked out. Empty on a detached HEAD.
    pub(crate) async fn target_of(&self, project: &Project) -> String {
        if !project.board.target.is_empty() {
            return project.board.target.clone();
        }
        let current = git::head_branch(&project.path).await;
        if current.is_empty() {
            return current;
        }
        // (Set meanwhile, by another pass or the settings: that one.)
        let (target, fixed) = {
            let mut projects = self.projects.write();
            match projects.iter_mut().find(|p| p.id == project.id) {
                Some(p) if p.board.target.is_empty() => {
                    p.board.target = current.clone();
                    (current, Some(p.clone()))
                }
                Some(p) => (p.board.target.clone(), None),
                None => (current, None),
            }
        };
        if let Some(project) = fixed {
            self.hub.emit(UiEvent::Project { project });
            self.request_save();
        }
        target
    }

    /// "En cours" at once (no other pass takes it), then its agent and its first message. A start
    /// that fails blocks the ticket, which then holds no place: the next one is tried.
    async fn start_ticket(self: &Arc<Self>, project: &Project, id: &str) {
        let started = self.edit_ticket(id, |t| {
            if t.column != Column::Todo {
                bail!("ce ticket est déjà parti");
            }
            t.column = Column::Doing;
            t.agent_id = None;
            t.iteration = 1;
            t.forced = false;
            t.partial = false;
            t.blocked = None;
            t.conflict = false;
            t.reminded = false;
            t.started_at = Some(now_ms());
            // A new agent from the target branch: nothing of an earlier attempt is in place.
            t.progress.clear();
            for c in &mut t.criteria {
                c.ok = false;
                c.note.clear();
            }
            Ok(t.clone())
        });
        let Ok(t) = started else { return };
        if let Err(e) = self.launch_ticket_agent(project, &t).await {
            log::warn!("ticket {}: start failed: {e:#}", t.key);
            let _ = self.edit_ticket(id, |t| {
                t.blocked = Some(error_reason(&e));
                Ok(())
            });
            self.notify_ticket(id, false);
            self.schedule();
        }
    }

    /// The ticket's agent: its worktree on `ticket/<key>` from the target branch, its block of
    /// ports, the protocol appended to its system prompt; then its first message.
    async fn launch_ticket_agent(self: &Arc<Self>, project: &Project, t: &Ticket) -> Result<()> {
        let s = &project.board;
        let target = self.target_of(project).await;
        let settings = self.settings.read().clone();
        let or = |v: &str, default: &str| Some(if v.is_empty() { default } else { v }.to_string());
        // Reserved at once: no other start or launch preparation gets this block meanwhile.
        let ports = self.reserve_ports();
        let made = self
            .create_agent_with(
                &project.id,
                AgentOptions {
                    model: or(&s.model, &settings.default_model),
                    effort: or(&s.effort, &settings.default_effort),
                    mode: or(&s.mode, &settings.default_mode),
                    name: Some(board::agent_name(&t.key, &t.title)),
                    worktree: Some((board::branch_of(&t.key), target)),
                    append_prompt: Some(board::protocol_prompt(t, ports)),
                    ticket_id: Some(t.id.clone()),
                    port_base: ports,
                    select: false,
                },
            )
            .await;
        // The agent holds the block from now on (or it is free again).
        self.unreserve_ports(ports);
        let agent_id = made?.meta.id;
        if let Err(e) = self.edit_ticket(&t.id, |x| {
            x.agent_id = Some(agent_id.clone());
            Ok(())
        }) {
            // The ticket went meanwhile (deleted): its agent goes with it.
            let _ = self.archive_agent(&agent_id, true).await;
            return Err(e);
        }
        self.send_message(&agent_id, board::first_message(t), vec![])
            .await
    }

    // ---------- ends of turns ----------

    /// Called by `apply` when an agent's turn ended: its recipe is kept, its ticket moves on.
    pub(crate) fn on_turn_end(self: &Arc<Self>, agent_id: &str, end: TurnEnd) {
        let (c, id) = (self.clone(), agent_id.to_string());
        tauri::async_runtime::spawn(async move { c.turn_ended(&id, end).await });
    }

    pub(crate) async fn turn_ended(self: &Arc<Self>, agent_id: &str, end: TurnEnd) {
        let report = match &end {
            TurnEnd::Finished(text) => board::parse_report(text),
            _ => None,
        };
        let Ok(h) = self.agent(agent_id) else { return };
        if let Some(recipe) = report.as_ref().and_then(|r| r.recipe.clone()) {
            h.lock().meta.recipe = Some(recipe);
            self.emit_agent(&h);
            self.request_save();
        }
        let (ticket_id, resumes) = {
            let rt = h.lock();
            (rt.meta.ticket_id.clone(), rt.meta.resume_at.is_some())
        };
        let Some(ticket_id) = ticket_id else { return };
        let limited = end == TurnEnd::Limited;
        // A usage limit with no resume planned (turned off) would leave the ticket waiting forever.
        let end = match end {
            TurnEnd::Limited if !resumes => TurnEnd::Error(QUOTA_LOST.into()),
            end => end,
        };
        let Ok(next) = self.edit_ticket(&ticket_id, |t| {
            // A ticket started again has another agent: the old one's turns never move it.
            if t.agent_id.as_deref() != Some(agent_id) {
                return Ok(board::Next::default());
            }
            // Stopped by the usage limit, a blocked ticket stays as it is: its block already
            // tells why it does not move (its resume lost, cancelled or failed, whether that
            // came before this reading or after), and its agent's next finished turn is read.
            if limited && t.blocked.is_some() {
                return Ok(board::Next::default());
            }
            Ok(board::turn_end(t, &end, report.as_ref(), now_ms()))
        }) else {
            return;
        };
        if let Some(text) = next.send {
            let _ = self.send_or_block(&ticket_id, agent_id, text).await;
        }
        if next.ready {
            self.notify_ticket(&ticket_id, true);
        }
        if next.blocked {
            self.notify_ticket(&ticket_id, false);
        }
        self.schedule();
    }

    /// Sends the ticket's agent a message; when it cannot, the ticket is blocked with the reason.
    pub(crate) async fn send_or_block(
        self: &Arc<Self>,
        ticket_id: &str,
        agent_id: &str,
        text: String,
    ) -> Result<()> {
        if let Err(e) = self.send_message(agent_id, text, vec![]).await {
            log::warn!("agent {agent_id}: message to its ticket's agent not sent: {e:#}");
            let _ = self.edit_ticket(ticket_id, |t| {
                t.blocked = Some(error_reason(&e));
                Ok(())
            });
            self.notify_ticket(ticket_id, false);
            return Err(e);
        }
        Ok(())
    }

    /// The automatic resume its agent waited for after the usage limit is gone (cancelled, turned
    /// off): its ticket "En cours" is blocked rather than hold its place forever.
    pub(crate) fn resume_lost(self: &Arc<Self>, agent_id: &str) {
        let Some(id) = self.doing_ticket_of(agent_id) else {
            return;
        };
        let blocked = self.edit_ticket(&id, |t| {
            let waiting = t.column == Column::Doing
                && t.blocked.is_none()
                && t.agent_id.as_deref() == Some(agent_id);
            if waiting {
                t.blocked = Some(format!("Erreur : {QUOTA_LOST}"));
            }
            Ok(waiting)
        });
        if blocked.unwrap_or(false) {
            self.notify_ticket(&id, false);
            self.schedule();
        }
    }

    /// "ATL-42 prêt à tester" or "ATL-42 bloqué : <raison>"; a click shows the project's board.
    pub(crate) fn notify_ticket(self: &Arc<Self>, ticket_id: &str, ready: bool) {
        let Ok(t) = self.ticket(ticket_id) else {
            return;
        };
        let project = self
            .project(&t.project_id)
            .map(|p| p.name)
            .unwrap_or_default();
        let body = if ready {
            format!("{} prêt à tester", t.key)
        } else {
            format!(
                "{} bloqué : {}",
                t.key,
                t.blocked.clone().unwrap_or_default()
            )
        };
        self.alert(
            project,
            body,
            UiEvent::FocusBoard {
                project_id: t.project_id,
            },
        );
    }

    /// The ticket "En cours" the agent works on.
    pub(crate) fn doing_ticket_of(&self, agent_id: &str) -> Option<String> {
        self.tickets
            .read()
            .iter()
            .find(|t| t.agent_id.as_deref() == Some(agent_id) && t.column == Column::Doing)
            .map(|t| t.id.clone())
    }

    /// The agent's ticket is "En cours": its turns are the board's to tell about.
    pub(crate) fn ticket_doing(&self, agent_id: &str) -> bool {
        self.doing_ticket_of(agent_id).is_some()
    }

    // ---------- recovery and manual management ----------

    /// The agent, unless it is gone or archived.
    fn live_agent(&self, id: &str) -> Option<AgentMeta> {
        let meta = self.agent(id).ok()?.lock().meta.clone();
        (!meta.archived).then_some(meta)
    }

    /// At startup, for each ticket "En cours" and not blocked (`board::recovery`): its agent is
    /// asked to go on (its turn cut by the app's stop, or ended but not read), or waits for its
    /// quota; a ticket whose start the stop cut goes back to "À faire", and the agent made for it,
    /// if any, is archived. Then what may start starts.
    pub fn recover_tickets(self: &Arc<Self>) {
        let cut = std::mem::take(&mut *self.cut_turns.lock());
        let doing: Vec<(String, String, Option<String>)> = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.column == Column::Doing && t.blocked.is_none())
            .map(|t| (t.id.clone(), t.key.clone(), t.agent_id.clone()))
            .collect();
        for (ticket_id, key, agent_id) in doing {
            let agent = agent_id.as_deref().and_then(|a| self.live_agent(a));
            let cut_off = agent_id.as_ref().is_some_and(|a| cut.contains(a));
            match (board::recovery(agent.as_ref(), cut_off), agent) {
                (board::Recovery::GoOn, Some(agent)) => {
                    let c = self.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = c
                            .send_or_block(&ticket_id, &agent.id, board::restart_message(&key))
                            .await;
                    });
                }
                (board::Recovery::Again, _) => self.start_over(&ticket_id, agent_id.as_deref()),
                _ => {}
            }
        }
        self.schedule();
    }

    /// The app stopped while starting the ticket: back to "À faire", and the agent made for it
    /// (never sent its first message), if any, is archived. It was under way: it starts again
    /// even with the autopilot off (one started by "Lancer" would wait otherwise).
    fn start_over(self: &Arc<Self>, ticket_id: &str, agent_id: Option<&str>) {
        let _ = self.edit_ticket(ticket_id, |t| {
            if t.column == Column::Doing && t.blocked.is_none() && t.agent_id.as_deref() == agent_id
            {
                board::back_to_todo(t);
                t.forced = true;
            }
            Ok(())
        });
        let made: Vec<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                let m = &rt.meta;
                (m.ticket_id.as_deref() == Some(ticket_id) && !m.archived && m.prompts == 0)
                    .then(|| m.id.clone())
            })
            .collect();
        for a in made {
            let c = self.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = c.archive_agent(&a, true).await {
                    log::warn!(
                        "agent {a}: made for a ticket that never started, not archived: {e:#}"
                    );
                }
            });
        }
    }

    /// "Reprendre" on a blocked ticket "En cours": its agent goes on (an agent that never got its
    /// first message gets it, with the ticket's description and criteria), or, when it has none
    /// left, the ticket starts again. The block goes as the message is sent.
    pub async fn ticket_resume(self: &Arc<Self>, id: &str) -> Result<()> {
        // Its agent waits for its quota: a message now would meet the limit again (and drop the
        // automatic resume, which takes it up).
        let waiting = self
            .ticket(id)
            .ok()
            .filter(|t| t.column == Column::Doing && t.blocked.is_some())
            .and_then(|t| t.agent_id)
            .and_then(|a| self.live_agent(&a))
            .and_then(|m| m.resume_at);
        if let Some(at) = waiting {
            bail!("En attente du quota — reprise à {}", board::clock(at));
        }
        let t = self.edit_ticket(id, |t| {
            if t.column != Column::Doing {
                bail!("Ce ticket n'est pas en cours.");
            }
            // Taken up already (a second click): its agent is not asked twice.
            if t.blocked.is_none() {
                bail!("Ce ticket n'est pas bloqué.");
            }
            t.blocked = None;
            t.reminded = false;
            Ok(t.clone())
        })?;
        match t.agent_id.as_deref().and_then(|a| self.live_agent(a)) {
            Some(agent) => {
                // Its start failed once it was made: the ticket was never given to it.
                let text = if agent.prompts == 0 {
                    board::first_message(&t)
                } else {
                    board::resume_message(&t.key)
                };
                self.send_or_block(id, &agent.id, text).await
            }
            None => {
                self.edit_ticket(id, |t| {
                    board::back_to_todo(t);
                    t.forced = true;
                    Ok(())
                })?;
                self.schedule();
                Ok(())
            }
        }
    }

    /// An agent was archived or deleted: its ticket "En cours" or "À tester" goes back to
    /// "À faire", from scratch; and what may start starts (its quota wait went with it too).
    pub(crate) fn release_ticket(self: &Arc<Self>, agent_id: &str) {
        self.unlink_ticket(agent_id);
        self.schedule();
    }

    /// The agent's ticket "En cours" or "À tester" goes back to "À faire", from scratch: from
    /// then on, none of the agent's turns moves it.
    pub(crate) fn unlink_ticket(&self, agent_id: &str) {
        let id = self
            .tickets
            .read()
            .iter()
            .find(|t| {
                t.agent_id.as_deref() == Some(agent_id)
                    && matches!(t.column, Column::Doing | Column::Review)
            })
            .map(|t| t.id.clone());
        if let Some(id) = id {
            let _ = self.edit_ticket(&id, |t| {
                // Checked again under the lock: it may have moved since.
                if t.agent_id.as_deref() == Some(agent_id)
                    && matches!(t.column, Column::Doing | Column::Review)
                {
                    board::back_to_todo(t);
                }
                Ok(())
            });
        }
    }

    // ---------- validation ----------

    fn set_step(&self, id: &str, step: &str) {
        let _ = self.edit_ticket(id, |t| {
            if t.column == Column::Review {
                t.step = Some(step.to_string());
            }
            Ok(())
        });
    }

    /// The ticket is still being validated with this agent (archiving or deleting the agent
    /// meanwhile sent it back to do: nothing of its validation goes on), and the agent is not at
    /// work (a message sent during the tests: its files may be half written).
    fn still_validating(&self, id: &str, agent_id: &str) -> Result<()> {
        if !validating(&self.ticket(id)?, agent_id) {
            bail!(CHANGED);
        }
        if self
            .live_agent(agent_id)
            .is_some_and(|m| m.status.is_active())
        {
            bail!(AGENT_BUSY);
        }
        Ok(())
    }

    /// The lock held while merging into the repository at `repo`: two validations never merge
    /// into the same folder at once.
    pub(crate) fn merge_lock(&self, repo: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.merge_locks
            .lock()
            .entry(merge_lock_key(repo))
            .or_default()
            .clone()
    }

    /// "Valider…" on a ticket "À tester" (even partial): tests, commit, then what the board's
    /// settings say, each step shown on the card. A failed step leaves it "À tester", blocked
    /// with the reason ("Réessayer" runs it again). Refused while its agent works (its files would
    /// be committed mid-write) and while a validation of it runs (a second click).
    pub async fn ticket_approve(self: &Arc<Self>, id: &str) -> Result<()> {
        let t = self.ticket(id)?;
        if t.column != Column::Review {
            bail!("Ce ticket n'est pas à tester.");
        }
        let busy = t
            .agent_id
            .as_deref()
            .and_then(|a| self.live_agent(a))
            .is_some_and(|m| m.status.is_active());
        if busy {
            bail!(AGENT_BUSY);
        }
        let t = self.edit_ticket(id, |t| {
            if t.column != Column::Review {
                bail!("Ce ticket n'est pas à tester.");
            }
            if t.step.is_some() {
                bail!("La validation de ce ticket est déjà en cours.");
            }
            t.blocked = None;
            t.conflict = false;
            t.step = Some("Validation…".into());
            Ok(t.clone())
        })?;
        if let Err(e) = self.validate(&t).await {
            log::warn!("ticket {}: validation failed: {e:#}", t.key);
            let reason = error_line(&e);
            // One that left "À tester" meanwhile (back with its agent, or to do, maybe started
            // again with another agent since) stays as it is.
            let blocked = self.edit_ticket(id, |x| {
                let here = x.column == Column::Review && x.agent_id == t.agent_id;
                if here {
                    x.step = None;
                    x.blocked = Some(reason);
                }
                Ok(here)
            });
            if blocked.unwrap_or(false) {
                self.notify_ticket(id, false);
            }
        }
        self.schedule();
        Ok(())
    }

    async fn validate(self: &Arc<Self>, t: &Ticket) -> Result<()> {
        let project = self.project(&t.project_id)?;
        let s = project.board.clone();
        let agent_id = t
            .agent_id
            .clone()
            .ok_or_else(|| anyhow!("Ce ticket n'a pas d'agent."))?;
        let meta = self.agent(&agent_id)?.lock().meta.clone();
        let wt = meta
            .worktree
            .clone()
            .ok_or_else(|| anyhow!("L'agent de ce ticket n'a pas de worktree."))?;
        let target = self.target_of(&project).await;
        // Its test launches stop first: their servers would hold the worktree's files and ports.
        self.stop_test_runs(&agent_id);
        // Tests.
        if s.tests_first && !s.test_command.trim().is_empty() {
            self.set_step(&t.id, "Tests…");
            let settings = self.settings.read().clone();
            let shell = crate::pty::detect_shells(&settings)
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("Aucun shell pour lancer les tests."))?;
            let command = s.test_command.trim();
            let env = testlaunch::port_env(meta.port_base);
            let run = testlaunch::run_tests(&shell, &wt.path, command, &env, TEST_LIMIT).await?;
            if !run.passed {
                // Its agent archived or deleted during the tests: the ticket went back to do (and
                // maybe on with another agent); the old agent is told nothing (it would start again).
                self.edit_ticket(&t.id, |x| {
                    if !validating(x, &agent_id) {
                        bail!(CHANGED);
                    }
                    board::back_to_work(x);
                    Ok(())
                })?;
                // Not a failure of the validation: the ticket is back with its agent.
                let _ = self
                    .send_or_block(
                        &t.id,
                        &agent_id,
                        board::tests_failed_message(command, &run.tail),
                    )
                    .await;
                return Ok(());
            }
        }
        // The files copied from the project into the worktree (`.env`…): never committed, merged
        // or pushed.
        let copied = testlaunch::matching_untracked(&project.path, &project.worktree_copy).await;
        self.still_validating(&t.id, &agent_id)?;
        let mut message = None;
        if s.action != "keep" {
            if !git::branch_exists(&wt.path, &target).await {
                bail!("La branche cible {target} n'existe pas.");
            }
            // Commit what is left in the worktree.
            self.set_step(&t.id, "Commit…");
            let unmerged = git::unmerged(&wt.path).await;
            if !unmerged.is_empty() {
                bail!(
                    "Conflits non résolus dans le worktree sur : {}",
                    unmerged.join(", ")
                );
            }
            // A commit elsewhere (another branch, a detached HEAD) would never reach the target.
            if git::current_branch(&wt.path).await != wt.branch {
                bail!("Le worktree n'est plus sur la branche {}", wt.branch);
            }
            self.still_validating(&t.id, &agent_id)?;
            let keep_out = not_committed(&wt.path, &copied).await;
            if git::stage_all(&wt.path, &keep_out).await? {
                let m = self.commit_message(t, &s, &wt.path, &target).await;
                git::commit_staged(&wt.path, &m).await?;
                message = Some(m);
            }
            // One the agent committed itself goes no further than its branch: in its tree, or
            // only in its history (taken out again since, which a merge commit or a rebase would
            // bring along all the same).
            let in_tree = git::changed_in(&wt.path, &target, &wt.branch, &copied).await?;
            let in_history = git::touched_by(&wt.path, &target, &wt.branch, &copied).await?;
            if let Some(refusal) = board::copied_refusal(&in_tree, &in_history) {
                bail!(refusal);
            }
            // Nothing to merge, to propose or to push.
            if git::ahead_of(&wt.path, &target, &wt.branch).await == 0 {
                let what = match s.action.as_str() {
                    "pr" => "proposer",
                    "push" => "pousser",
                    _ => "merger",
                };
                bail!(
                    "Rien à {what} : {} n'a pas de commit de plus que {target}.",
                    wt.branch
                );
            }
            self.still_validating(&t.id, &agent_id)?;
        }
        let (outcome, url): (String, Option<String>) = match s.action.as_str() {
            "merge" => {
                self.set_step(&t.id, "Merge…");
                match self
                    .merge_ticket(t, &s, &project, &wt, &target, message)
                    .await?
                {
                    Some(done) => done,
                    // Stopped on a conflict, handled as the settings say.
                    None => return Ok(()),
                }
            }
            "pr" => {
                self.set_step(&t.id, "Push…");
                self.open_pr(t, &s, &wt, &target, message).await?
            }
            "push" => {
                self.set_step(&t.id, "Push…");
                git::push_branch(&wt.path, &wt.branch).await?;
                (board::pushed_outcome(&wt.branch), None)
            }
            _ => (board::KEPT_OUTCOME.to_string(), None),
        };
        let cost = self
            .agent(&agent_id)
            .map(|h| h.lock().meta.cost)
            .unwrap_or(meta.cost);
        // Its process, killed for good before its worktree goes (see `remove_worktree_of`).
        let proc = self
            .agent(&agent_id)
            .ok()
            .and_then(|h| h.lock().proc.clone());
        self.edit_ticket(&t.id, |x| {
            // Its agent archived or deleted meanwhile: it went back to do, and stays there.
            if !validating(x, &agent_id) {
                bail!(CHANGED);
            }
            x.column = Column::Done;
            x.step = None;
            x.blocked = None;
            x.conflict = false;
            x.outcome = Some(outcome);
            x.outcome_url = url;
            x.done_at = Some(now_ms());
            x.cost = cost;
            Ok(())
        })?;
        if s.action != "keep" {
            // Done whatever becomes of its agent: its work went where it was sent, and "Réessayer"
            // on it would have nothing left to do.
            if let Err(e) = self.archive_agent(&agent_id, true).await {
                log::warn!("ticket {}: its agent was not archived: {e:#}", t.key);
            }
            if s.action == "merge" && s.cleanup {
                self.remove_worktree_of(t, &project, &wt, proc).await;
            }
        }
        Ok(())
    }

    /// Haiku's commit message in the Conventional Commits form, checked, else
    /// `feat: <title> [<key>]`; without "Message de commit généré", `<key> <title>`.
    async fn commit_message(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        cwd: &str,
        target: &str,
    ) -> String {
        if !s.conventional {
            return board::plain_commit(t);
        }
        let stat = git::staged_stat(cwd, target).await;
        match self
            .one_shot(board::COMMIT_SYSTEM, &board::commit_prompt(t, &stat))
            .await
        {
            Ok(answer) => board::commit_from_answer(&answer, &t.key)
                .unwrap_or_else(|| board::fallback_commit(t)),
            Err(e) => {
                log::info!("ticket {}: no commit message from Haiku: {e:#}", t.key);
                board::fallback_commit(t)
            }
        }
    }

    /// The merge step: in the project's folder when the target is checked out there (refused with
    /// uncommitted changes), refused when an agent's worktree has it, else in a temporary
    /// worktree. None when it stopped on a conflict.
    async fn merge_ticket(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        project: &Project,
        wt: &Worktree,
        target: &str,
        message: Option<String>,
    ) -> Result<Option<(String, Option<String>)>> {
        let repo = self
            .toplevel(&project.path)
            .await
            .ok_or_else(|| anyhow!("pas un dépôt git"))?;
        let message = match message {
            Some(m) => m,
            None => self.commit_message(t, s, &wt.path, target).await,
        };
        // Two validations never merge into the same folder at once: the second one waits.
        let lock = self.merge_lock(&repo);
        let merging = lock.lock().await;
        // Its agent may have been archived or set to work during Haiku's answer and that wait.
        self.still_validating(&t.id, t.agent_id.as_deref().unwrap_or_default())?;
        let tmp = Path::new(&repo)
            .join(".claude")
            .join("worktrees")
            .join(format!(".merge-{}", t.key.to_lowercase()));
        let tmp_s = tmp.to_string_lossy().to_string();
        // One left by a validation the app's stop cut: registered, it would still hold the target
        // and every merge into it would be refused.
        drop_worktree(&repo, &tmp).await;
        let mut checkout = git::checkout_of(&repo, target).await?;
        // Another ticket's, left the same way (no other merge into this repository runs now).
        if let Some(p) = checkout.clone().filter(|p| merge_leftover(&repo, p)) {
            drop_worktree(&repo, Path::new(&p)).await;
            checkout = git::checkout_of(&repo, target).await?;
        }
        let (dir, temporary) = match checkout {
            Some(p) if same_dir(&p, &repo) => {
                if git::has_tracked_changes(&repo).await? {
                    bail!("Le dossier du projet a des modifications non commitées sur {target}");
                }
                (repo.clone(), false)
            }
            Some(p) => {
                let who = self
                    .project_agents(&project.id)
                    .iter()
                    .map(|h| h.lock().meta.clone())
                    .find(|m| m.worktree.as_ref().is_some_and(|w| same_dir(&w.path, &p)))
                    .map(|m| m.name)
                    .unwrap_or(p);
                bail!("{target} est extraite dans le worktree de {who}");
            }
            None => {
                git::ensure_excluded(&repo, ".claude/worktrees/").await?;
                git::run(&repo, &["worktree", "add", &tmp_s, target]).await?;
                (tmp_s.clone(), true)
            }
        };
        let result =
            git::integrate(&dir, &wt.branch, &s.strategy, &message, &wt.path, target).await;
        if temporary {
            drop_worktree(&repo, &tmp).await;
        }
        self.git.refresh(&project.id);
        // The target is as it was: the next merge into this repository may go.
        drop(merging);
        match result? {
            git::Integrated::Done => Ok(Some((board::merged_outcome(target, &s.strategy), None))),
            git::Integrated::Conflict(files) => {
                self.on_conflict(t, s, target, files).await?;
                Ok(None)
            }
        }
    }

    /// After a merge, with "Supprimer le worktree": the ticket's worktree and branch go. Its
    /// agent's process is killed first with all it started: the archive closed its input, but it
    /// may take its time to end (or something still holds it), and on Windows a live process
    /// keeps the folder. The removal is tried again for a few seconds.
    async fn remove_worktree_of(
        &self,
        t: &Ticket,
        project: &Project,
        wt: &Worktree,
        proc: Option<Arc<ClaudeProcess>>,
    ) {
        if let Some(p) = proc {
            p.kill();
            for _ in 0..30 {
                if !p.is_alive() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        for attempt in 1..=10 {
            match git::worktree_remove(&project.path, &wt.path, &wt.branch).await {
                Ok(()) => break,
                Err(e) if attempt == 10 => {
                    log::warn!("ticket {}: worktree not removed: {e:#}", t.key)
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(300)).await,
            }
        }
        self.git.refresh(&project.id);
    }

    // ---------- pull requests, conflicts, rejection ----------

    /// The GitHub CLI: the one known, else looked for again on the PATH (installed since the app
    /// started), and kept once found.
    fn gh_cli(&self) -> Option<PathBuf> {
        let known = self.gh.read().clone();
        if known.is_some() {
            return known;
        }
        #[cfg(test)]
        let found = self.gh_on_path.read().clone();
        #[cfg(not(test))]
        let found = crate::core::locate_gh();
        if let Some(gh) = &found {
            *self.gh.write() = Some(gh.clone());
        }
        found
    }

    /// Pushes the ticket's branch, then opens its PR with `gh` on GitHub; without `gh` (or when it
    /// fails), GitHub's page to finish it opens in the browser; another host: pushed only.
    async fn open_pr(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        wt: &Worktree,
        target: &str,
        message: Option<String>,
    ) -> Result<(String, Option<String>)> {
        let remote = git::push_branch(&wt.path, &wt.branch).await?;
        let title = match message {
            Some(m) => m,
            None => self.commit_message(t, s, &wt.path, target).await,
        };
        let body = board::pr_body(t);
        let github = git::remote_url(&wt.path, &remote)
            .await
            .as_deref()
            .and_then(board::github_repo);
        let Some((owner, repo)) = github else {
            return Ok((board::pushed_elsewhere_outcome(&wt.branch), None));
        };
        if let Some(gh) = self.gh_cli() {
            let pr = PullRequest {
                repo: &format!("{owner}/{repo}"),
                base: target,
                head: &wt.branch,
                title: &title,
                body: &body,
                draft: s.draft,
            };
            match gh_pr_create(&gh, &wt.path, &pr).await {
                Ok(out) => {
                    if let Some((n, url)) = board::pr_number(&out) {
                        return Ok((board::pr_outcome(n, target), Some(url)));
                    }
                    log::warn!("ticket {}: no pull request in gh's answer: {out}", t.key);
                }
                Err(e) => log::warn!("ticket {}: gh pr create failed: {e:#}", t.key),
            }
        }
        let url = board::compare_url(&owner, &repo, target, &wt.branch, &title, &body);
        self.hub.emit(UiEvent::OpenUrl { url: url.clone() });
        Ok((board::pushed_for_pr_outcome(&wt.branch), Some(url)))
    }

    /// A merge stopped on conflicts (undone): blocked with "L'agent résout" / "Annuler" ("ask"),
    /// handed to the agent ("agent"), or blocked with the files ("abort").
    async fn on_conflict(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        target: &str,
        files: Vec<String>,
    ) -> Result<()> {
        if s.conflict == "agent" {
            return self.agent_resolves(&t.id).await;
        }
        let ask = s.conflict != "abort";
        let reason = if ask {
            format!("Conflit avec {target}")
        } else {
            board::first_line(&format!("Conflit avec {target} sur : {}", files.join(", ")))
        };
        let agent_id = t.agent_id.clone().unwrap_or_default();
        self.edit_ticket(&t.id, |x| {
            // Its agent archived or deleted meanwhile: it went back to do, and stays there.
            if !validating(x, &agent_id) {
                bail!(CHANGED);
            }
            x.step = None;
            x.conflict = ask;
            x.blocked = Some(reason);
            Ok(())
        })?;
        self.notify_ticket(&t.id, false);
        Ok(())
    }

    /// "L'agent résout": the target is merged into the ticket's branch, its conflicts left in the
    /// worktree for the agent, or the agent is asked to rebase; the ticket goes back "En cours",
    /// loop 1. A merge git refuses to start (changes in the way…) is the error: the agent is told
    /// nothing. Runs while the ticket is held, its step set (its validation, or
    /// `ticket_resolve_conflict`).
    async fn agent_resolves(self: &Arc<Self>, id: &str) -> Result<()> {
        let t = self.ticket(id)?;
        let project = self.project(&t.project_id)?;
        let agent_id = t
            .agent_id
            .clone()
            .ok_or_else(|| anyhow!("Ce ticket n'a pas d'agent."))?;
        let wt = self
            .agent(&agent_id)?
            .lock()
            .meta
            .worktree
            .clone()
            .ok_or_else(|| anyhow!("L'agent de ce ticket n'a pas de worktree."))?;
        let target = self.target_of(&project).await;
        // Nothing goes into the worktree of an agent archived meanwhile, or at work.
        self.still_validating(id, &agent_id)?;
        let rebase = project.board.strategy == "rebase";
        let text = if rebase {
            board::rebase_message(&target)
        } else {
            match git::run(&wt.path, &["merge", "--no-edit", &target]).await {
                Ok(_) => board::conflict_message(&target, &[]),
                Err(e) => {
                    let files = git::unmerged(&wt.path).await;
                    if files.is_empty() {
                        return Err(
                            e.context(format!("Merge de {target} dans {} impossible", wt.branch))
                        );
                    }
                    board::conflict_message(&target, &files)
                }
            }
        };
        let handed = self.edit_ticket(id, |x| {
            if !validating(x, &agent_id) {
                bail!(CHANGED);
            }
            board::back_to_work(x);
            Ok(())
        });
        if let Err(e) = handed {
            // It left meanwhile (its agent archived): its worktree is not left mid-merge.
            if !rebase {
                let _ = git::run(&wt.path, &["merge", "--abort"]).await;
            }
            return Err(e);
        }
        self.send_or_block(id, &agent_id, text).await
    }

    /// "L'agent résout" on the card of a ticket "À tester": refused while a validation of it runs
    /// (its merge may be under way), and held as one meanwhile (a second click, "Réessayer").
    pub async fn ticket_resolve_conflict(self: &Arc<Self>, id: &str) -> Result<()> {
        let agent_id = self.edit_ticket(id, |t| {
            if t.column != Column::Review {
                bail!("Ce ticket n'est pas à tester.");
            }
            if t.step.is_some() {
                bail!("La validation de ce ticket est déjà en cours.");
            }
            t.step = Some("Merge…".into());
            Ok(t.agent_id.clone())
        })?;
        let resolved = self.agent_resolves(id).await;
        if resolved.is_err() {
            // Not handed over: the ticket is as it was, its block still shown.
            let _ = self.edit_ticket(id, |x| {
                if x.column == Column::Review && x.agent_id == agent_id {
                    x.step = None;
                }
                Ok(())
            });
        }
        resolved
    }

    /// "Annuler" on a ticket "À tester" (a conflict, a failed step): the block goes, the ticket
    /// stays to test.
    pub fn ticket_dismiss(&self, id: &str) -> Result<()> {
        self.edit_ticket(id, |t| {
            if t.column != Column::Review {
                bail!("Ce ticket n'est pas à tester.");
            }
            t.blocked = None;
            t.conflict = false;
            Ok(())
        })
    }

    /// "Renvoyer": back "En cours" with the same agent and worktree, loop 1, with what is wrong.
    /// Refused while its agent works: the end of that turn would be read as the end of the
    /// rework.
    pub async fn ticket_reject(self: &Arc<Self>, id: &str, comment: &str) -> Result<()> {
        let comment = comment.trim();
        if comment.is_empty() {
            bail!("Dis ce qui ne va pas.");
        }
        let busy = self
            .ticket(id)?
            .agent_id
            .and_then(|a| self.live_agent(&a))
            .is_some_and(|m| m.status.is_active());
        if busy {
            bail!(AGENT_BUSY_REJECT);
        }
        let (key, agent_id) = self.edit_ticket(id, |t| {
            if t.column != Column::Review {
                bail!("Ce ticket n'est pas à tester.");
            }
            if t.step.is_some() {
                bail!("La validation de ce ticket est déjà en cours.");
            }
            let agent_id = t
                .agent_id
                .clone()
                .ok_or_else(|| anyhow!("Ce ticket n'a pas d'agent."))?;
            board::back_to_work(t);
            Ok((t.key.clone(), agent_id))
        })?;
        self.send_or_block(id, &agent_id, board::reject_message(&key, comment))
            .await
    }

    // ---------- test launches ----------

    /// "Préparer le lancement": reserves the agent's block of ports if it has none, then asks it
    /// for its recipe, as any message (during a turn, Claude Code takes it once the turn is over).
    /// Only for an agent with a worktree, and not archived (it would hold a block for nothing).
    pub async fn agent_prepare_launch(self: &Arc<Self>, id: &str) -> Result<()> {
        let h = self.agent(id)?;
        let (has_worktree, archived, base) = {
            let rt = h.lock();
            (
                rt.meta.worktree.is_some(),
                rt.meta.archived,
                rt.meta.port_base,
            )
        };
        if !has_worktree {
            bail!(NO_WORKTREE);
        }
        if archived {
            bail!(ARCHIVED);
        }
        let base = match base {
            Some(b) => b,
            None => {
                // Reserved, then written on the agent with no wait in between: no other start
                // or preparation gets this block meanwhile.
                let reserved = self
                    .reserve_ports()
                    .ok_or_else(|| anyhow!("Aucun bloc de ports libre à partir de 4100."))?;
                let held = {
                    let mut rt = h.lock();
                    // Archived meanwhile, it holds no block; given one meanwhile (another
                    // preparation at once), it keeps that one.
                    (!rt.meta.archived).then(|| *rt.meta.port_base.get_or_insert(reserved))
                };
                // The agent holds its block from now on (or the one reserved is free again).
                self.unreserve_ports(Some(reserved));
                let base = held.ok_or_else(|| anyhow!(ARCHIVED))?;
                self.emit_agent(&h);
                self.request_save();
                base
            }
        };
        // Archived meanwhile, whichever way it got its block: it is not woken up for nothing.
        if h.lock().meta.archived {
            bail!(ARCHIVED);
        }
        self.send_message(id, board::prepare_message(base), vec![])
            .await
    }

    /// Why the agent may run no test launch now: archived (its launches stopped with it), without
    /// a worktree, or its ticket being validated (or its conflict handed to it), whose tests
    /// would find its ports taken.
    fn launch_refusal(&self, meta: &AgentMeta) -> Option<&'static str> {
        if meta.archived {
            Some(ARCHIVED)
        } else if meta.worktree.is_none() {
            Some(NO_WORKTREE)
        } else if self.tickets.read().iter().any(|t| validating(t, &meta.id)) {
            Some(VALIDATING)
        } else {
            None
        }
    }

    /// What step `index` of `kind` ("prep" or "run") of the agent's recipe runs, when it may run
    /// one (`launch_refusal`).
    pub fn test_run_spec(
        &self,
        agent_id: &str,
        kind: &str,
        index: usize,
    ) -> Result<testlaunch::RunSpec> {
        let meta = self.agent(agent_id)?.lock().meta.clone();
        if let Some(why) = self.launch_refusal(&meta) {
            bail!(why);
        }
        testlaunch::run_spec(&meta, kind, index)
    }

    /// The terminal `term_id` is one of the agent's test launches, kept to be stopped with them.
    /// One started as the agent was archived or deleted, or as its ticket's validation began
    /// (after its launches were stopped), is stopped at once: the error tells why.
    pub fn track_test_run(&self, agent_id: &str, term_id: &str) -> Result<()> {
        self.test_runs
            .lock()
            .entry(agent_id.to_string())
            .or_default()
            .push(term_id.to_string());
        // Read after it is kept: an archive, a deletion or a validation that this does not see
        // stops it when it comes.
        let refused = match self.agent(agent_id) {
            Ok(h) => {
                let meta = h.lock().meta.clone();
                self.launch_refusal(&meta).map(|why| anyhow!(why))
            }
            Err(e) => Some(e),
        };
        if let Some(e) = refused {
            self.stop_test_runs(agent_id);
            return Err(e);
        }
        Ok(())
    }

    /// The agent's test launches stop (its validation, archive or deletion, its project closed).
    pub fn stop_test_runs(&self, agent_id: &str) {
        let ids = self.test_runs.lock().remove(agent_id).unwrap_or_default();
        for id in ids {
            self.pty.kill(&id);
        }
    }
}

/// How long `gh pr create` may take.
const GH_LIMIT: Duration = Duration::from_secs(120);

/// A pull request to open on GitHub.
struct PullRequest<'a> {
    /// `owner/repo`, the repository the branch was pushed to.
    repo: &'a str,
    base: &'a str,
    head: &'a str,
    title: &'a str,
    body: &'a str,
    draft: bool,
}

/// `gh pr create` in `cwd`, on the repository the branch was pushed to (whatever `gh repo
/// set-default` says), its description through the standard input (an argument with line breaks
/// cannot go through a `.cmd`): what it printed.
async fn gh_pr_create(gh: &Path, cwd: &str, pr: &PullRequest<'_>) -> Result<String> {
    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new(gh);
    cmd.args([
        "pr",
        "create",
        "--repo",
        pr.repo,
        "--base",
        pr.base,
        "--head",
        pr.head,
        "--title",
        pr.title,
        "--body-file",
        "-",
    ]);
    if pr.draft {
        cmd.arg("--draft");
    }
    cmd.current_dir(cwd)
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(crate::claude::CREATE_NO_WINDOW);
    crate::job::isolate(&mut cmd);
    let mut child = cmd.spawn()?;
    // Dropped when this returns: whatever it left running (a `.cmd`'s node…) ends with it.
    let _job = crate::job::Job::for_child(&child);
    let created = async move {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(pr.body.as_bytes()).await?;
            // Dropped here: the description ends.
        }
        anyhow::Ok(child.wait_with_output().await?)
    };
    let out = tokio::time::timeout(GH_LIMIT, created)
        .await
        .map_err(|_| {
            anyhow!(
                "gh pr create toujours pas terminé après {} s",
                GH_LIMIT.as_secs()
            )
        })??;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Of the files copied from the project, those the branch checked out in `cwd` has not
/// committed: left out of the validation's commit, staged or not (unstaging one it has would
/// commit its deletion; the branch's check refuses those).
async fn not_committed(cwd: &str, copied: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for f in copied {
        if git::run(cwd, &["cat-file", "-e", &format!("HEAD:{f}")])
            .await
            .is_err()
        {
            out.push(f.clone());
        }
    }
    out
}

/// The key of a repository's merge lock: its folder however it is spelled, as `same_dir` sees it
/// (the real path, else the path written with `/` and in lower case).
pub(crate) fn merge_lock_key(repo: &str) -> String {
    match std::fs::canonicalize(repo) {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(_) => repo.replace('\\', "/").trim_end_matches('/').to_lowercase(),
    }
}

/// The block when the ticket left its validation (its agent archived or deleted meanwhile).
const CHANGED: &str = "Ce ticket a changé pendant sa validation.";

/// The refusal while the ticket's agent works: its files may be half written.
const AGENT_BUSY: &str =
    "L'agent de ce ticket travaille encore : attends la fin de son tour pour valider.";

/// "Renvoyer" refused while the ticket's agent works: the end of that turn would be read as the
/// end of the rework.
const AGENT_BUSY_REJECT: &str =
    "L'agent de ce ticket travaille encore : attends la fin de son tour pour le renvoyer.";

/// An archived agent holds no ports and runs no test launch.
const ARCHIVED: &str = "Cet agent est archivé : il n'a pas de lancement de test.";

/// Only an agent with a worktree has ports and a test launch.
const NO_WORKTREE: &str = "Seul un agent à worktree a un lancement de test.";

/// No test launch while its ticket is validated: its servers would take the ports its tests use.
const VALIDATING: &str = "Validation en cours : le lancement de test attendra sa fin.";

/// `t` is still "À tester" with this agent, its validation under way.
fn validating(t: &Ticket, agent_id: &str) -> bool {
    t.column == Column::Review && t.step.is_some() && t.agent_id.as_deref() == Some(agent_id)
}

/// `path` is a temporary worktree of a validation (`.claude/worktrees/.merge-<clé>`) of `repo`.
fn merge_leftover(repo: &str, path: &str) -> bool {
    let p = Path::new(path);
    let worktrees = Path::new(repo).join(".claude").join("worktrees");
    p.file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with(".merge-"))
        && p.parent()
            .is_some_and(|d| same_dir(&d.to_string_lossy(), &worktrees.to_string_lossy()))
}

/// Removes the worktree at `path`, registered or not, its folder there or not.
async fn drop_worktree(repo: &str, path: &Path) {
    let p = path.to_string_lossy();
    let _ = git::run(repo, &["worktree", "remove", "--force", &p]).await;
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
    let _ = git::run(repo, &["worktree", "prune"]).await;
}

/// The same folder, however git and Windows spell it (short 8.3 names, slashes, case).
fn same_dir(a: &str, b: &str) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => {
            let norm = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_lowercase();
            norm(a) == norm(b)
        }
    }
}
