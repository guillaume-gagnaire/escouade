//! The board on the core: tickets saved with the app, the scheduler that starts them with agents
//! of their own, what each end of turn does to them, their validation (tests, commit, merge, pull
//! request, push) and the test launches of the worktrees.

use crate::board;
use crate::core::Core;
use crate::git;
use crate::model::*;
use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::sync::Arc;
use tauri::Runtime;

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

    /// Changes a ticket under the lock, then tells the window and saves.
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
        let rank = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == project_id)
            .map(|t| t.rank)
            .max()
            .unwrap_or(0)
            + 1;
        let ticket = Ticket {
            id: new_id(),
            project_id: project_id.to_string(),
            key,
            title,
            description: d.description.trim().to_string(),
            criteria: board::criteria_from(&d.criteria),
            max_loops: board::max_loops(d.max_loops),
            rank,
            created_at: now_ms(),
            ..Default::default()
        };
        self.tickets.write().push(ticket.clone());
        self.emit_project(project_id);
        self.hub.emit(UiEvent::Ticket {
            ticket: ticket.clone(),
        });
        self.request_save();
        Ok(ticket)
    }

    /// Only a ticket "À faire" changes.
    pub fn ticket_update(&self, id: &str, d: TicketDraft) -> Result<Ticket> {
        let title = d.title.trim().to_string();
        if title.is_empty() {
            bail!("Un ticket a besoin d'un titre.");
        }
        self.edit_ticket(id, |t| {
            if t.column != Column::Todo {
                bail!("Seul un ticket « À faire » se modifie.");
            }
            t.title = title;
            t.description = d.description.trim().to_string();
            t.criteria = board::criteria_from(&d.criteria);
            t.max_loops = board::max_loops(d.max_loops);
            Ok(t.clone())
        })
    }

    /// "Passer en tête".
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
            t.rank = first - 1;
            Ok(())
        })
    }

    /// "Lancer": a ticket that starts even with the autopilot off.
    pub fn ticket_start(self: &Arc<Self>, id: &str) -> Result<()> {
        self.edit_ticket(id, |t| {
            if t.column != Column::Todo {
                bail!("Ce ticket est déjà parti.");
            }
            t.forced = true;
            Ok(())
        })
    }

    /// The ticket goes; one "En cours" or "À tester" has its agent archived (worktree kept).
    pub async fn ticket_delete(self: &Arc<Self>, id: &str) -> Result<()> {
        let t = self.ticket(id)?;
        self.tickets.write().retain(|x| x.id != id);
        self.hub.emit(UiEvent::TicketRemoved {
            id: id.to_string(),
            project_id: t.project_id.clone(),
        });
        self.request_save();
        if matches!(t.column, Column::Doing | Column::Review) {
            if let Some(a) = t.agent_id.as_deref().filter(|a| self.agent(a).is_ok()) {
                self.archive_agent(a, true).await?;
            }
        }
        Ok(())
    }

    /// The board's settings as the window sends them: the key's prefix and number stay the
    /// backend's, the agents in parallel stay between 1 and 6.
    pub fn board_set(self: &Arc<Self>, project_id: &str, s: BoardSettings) -> Result<Project> {
        let project = {
            let mut projects = self.projects.write();
            let p = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(|| anyhow!("projet introuvable"))?;
            p.board = BoardSettings {
                prefix: p.board.prefix.clone(),
                next_number: p.board.next_number,
                max_parallel: s.max_parallel.clamp(1, 6),
                ..s
            };
            p.clone()
        };
        self.hub.emit(UiEvent::Project {
            project: project.clone(),
        });
        self.request_save();
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
}
