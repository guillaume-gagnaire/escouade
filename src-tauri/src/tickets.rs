//! The board on the core: tickets saved with the app, the scheduler that starts them with agents
//! of their own, what each end of turn does to them, their validation (tests, commit, merge, pull
//! request, push) and the test launches of the worktrees.

use crate::board::{self, TurnEnd};
use crate::claude;
use crate::core::{AgentOptions, Core};
use crate::git;
use crate::model::*;
use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::Runtime;

/// Why a ticket "En cours" stops when its agent's automatic resume after the usage limit is gone
/// (turned off, cancelled, never planned): it would otherwise hold its place forever.
const QUOTA_LOST: &str = "limite d'usage atteinte";

/// The reason a ticket is blocked by an error, on one line (a git error may run over many; the
/// whole error goes to the log): the contexts the app gave, then git's `fatal:` or `error:` line,
/// the one that tells why, when there is one; else the first line, as `board::turn_end` does with
/// an agent's.
pub(crate) fn error_reason(e: &anyhow::Error) -> String {
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
            return format!("Erreur : {}", board::first_line(&parts.join(": ")));
        }
    }
    format!("Erreur : {}", board::first_line(&format!("{e:#}")))
}

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
    /// target branch (the project's current one) when not set.
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
        let current = git::current_branch(&project.path).await;
        let key = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(|| anyhow!("projet introuvable"))?;
            if p.board.prefix.is_empty() {
                p.board.prefix = board::key_prefix(&p.name);
            }
            if p.board.target.is_empty() && !current.is_empty() && current != "HEAD" {
                p.board.target = current;
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

    /// The project is closed: its tickets go with it.
    pub(crate) fn drop_project_tickets(&self, project_id: &str) {
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
            if ids.is_empty() {
                continue;
            }
            // Every ticket's branch starts from the target: while it is gone (deleted, renamed),
            // none starts, rather than each make an agent only to block (choosing another target
            // in the settings, or any change of the board, looks again).
            let target = self.target_of(&p).await;
            if !git::branch_exists(&p.path, &target).await {
                if self.targets_missing.lock().insert(p.id.clone()) {
                    log::info!(
                        "board of {}: target branch {target:?} not found, no ticket starts until it is",
                        p.name
                    );
                }
                continue;
            }
            self.targets_missing.lock().remove(&p.id);
            for id in ids {
                self.start_ticket(&p, &id).await;
            }
        }
    }

    /// The board's target branch, else the project's current one.
    pub(crate) async fn target_of(&self, project: &Project) -> String {
        if project.board.target.is_empty() {
            git::current_branch(&project.path).await
        } else {
            project.board.target.clone()
        }
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
    /// (never sent its first message), if any, is archived.
    fn start_over(self: &Arc<Self>, ticket_id: &str, agent_id: Option<&str>) {
        let _ = self.edit_ticket(ticket_id, |t| {
            if t.column == Column::Doing && t.blocked.is_none() && t.agent_id.as_deref() == agent_id
            {
                board::back_to_todo(t);
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
}
