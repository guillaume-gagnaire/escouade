//! The deletions agents ask of the user (`delete_agent`, ESC-511): kept on the agent asked about
//! (`AgentMeta::delete_ask`) until the user confirms or refuses it in the window, across restarts;
//! gone with that agent or with the one that asked. Nothing is ever deleted here: the window does
//! it, the user confirming. A refusal is told to the asker with the next message it gets
//! (`AgentMeta::delete_refused`). And what a deletion would lose (`loss`), read by git when the
//! window shows its confirmation.

use crate::core::Core;
use crate::git;
use crate::i18n::Lang;
use crate::model::{now_ms, DeleteAsk};
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

/// The most deletions an agent may have asked for at once, still unanswered: past them, it is
/// refused until the user answers some.
pub(crate) const MAX_ASKS: usize = 5;

/// How long git is given to say what a deletion would lose before the window says it is unknown.
pub(crate) const LOSS_TIMEOUT: Duration = Duration::from_secs(3);

/// The first names of its uncommitted files the window shows.
pub(crate) const LOSS_NAMES: usize = 5;

/// What deleting an agent's worktree would lose, as git says it (`Core::delete_loss`).
#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Loss {
    /// False when git did not answer in time (or failed): the window says the worktree's state is
    /// unknown.
    pub known: bool,
    /// Its files changed and not committed, untracked ones included.
    pub dirty: usize,
    /// The first of them (`LOSS_NAMES`), from its folder, with `/`.
    pub names: Vec<String>,
    /// Its commits its base branch does not have.
    pub unmerged: u32,
    /// That base branch (the first one, of several repositories).
    pub base: String,
}

impl Core {
    /// The agent `asker` asks the user to delete `id` (one it made: the caller checks it). Asked
    /// again before the user answers, the ask is the same, its worktree's choice the new one.
    /// Refused past `MAX_ASKS` unanswered.
    pub(crate) fn ask_delete(&self, asker: &str, id: &str, worktree: Option<bool>) -> Result<()> {
        let open = self
            .agents
            .read()
            .values()
            .filter(|h| {
                let m = &h.lock().meta;
                m.id != id && m.delete_ask.as_ref().is_some_and(|a| a.by == asker)
            })
            .count();
        if open >= MAX_ASKS {
            let max = MAX_ASKS;
            return Err(anyhow!(tr!(
                "Tu as déjà {max} demandes de suppression qui attendent l’utilisateur : attends qu’il en accepte ou en refuse une.",
                "You already have {max} deletions waiting for the user: wait for them to accept or refuse one."
            )));
        }
        let h = self.agent(id)?;
        {
            let mut rt = h.lock();
            let at = rt
                .meta
                .delete_ask
                .as_ref()
                .filter(|a| a.by == asker)
                .map_or_else(now_ms, |a| a.at);
            rt.meta.delete_ask = Some(DeleteAsk {
                by: asker.to_string(),
                at,
                worktree,
            });
        }
        self.emit_agent(&h);
        self.request_save();
        Ok(())
    }

    /// « Refuser » on the card of `id`: its ask goes, and the agent that asked is told with its next
    /// message. Nothing to refuse: nothing done.
    pub fn refuse_delete(&self, id: &str) -> Result<()> {
        let h = self.agent(id)?;
        let (ask, name) = {
            let mut rt = h.lock();
            (rt.meta.delete_ask.take(), rt.meta.name.clone())
        };
        let Some(ask) = ask else { return Ok(()) };
        self.emit_agent(&h);
        if let Ok(asker) = self.agent(&ask.by) {
            {
                let mut rt = asker.lock();
                if !rt.meta.delete_refused.contains(&name) {
                    rt.meta.delete_refused.push(name);
                }
            }
            self.emit_agent(&asker);
        }
        self.request_save();
        Ok(())
    }

    /// The agent `gone` was deleted: the deletions it asked for go with it.
    pub(crate) fn drop_asks_by(&self, gone: &str) {
        let asked: Vec<_> = self
            .agents
            .read()
            .values()
            .filter(|h| {
                h.lock()
                    .meta
                    .delete_ask
                    .as_ref()
                    .is_some_and(|a| a.by == gone)
            })
            .cloned()
            .collect();
        for h in &asked {
            h.lock().meta.delete_ask = None;
            self.emit_agent(h);
        }
        if !asked.is_empty() {
            self.request_save();
        }
    }

    /// The note the agent's next message starts with, when the user refused deletions it asked for
    /// since it was last told: those names, and the line.
    pub(crate) fn refusal_note(&self, id: &str) -> Option<(Vec<String>, String)> {
        let h = self.agent(id).ok()?;
        let names = h.lock().meta.delete_refused.clone();
        if names.is_empty() {
            return None;
        }
        let note = refusal_line(self.lang().claude, &names);
        Some((names, note))
    }

    /// The agent was told of the refusals `names` (its message delivered): told once.
    pub(crate) fn refusals_told(&self, id: &str, names: &[String]) {
        let Ok(h) = self.agent(id) else { return };
        let told = {
            let mut rt = h.lock();
            let before = rt.meta.delete_refused.len();
            rt.meta.delete_refused.retain(|n| !names.contains(n));
            rt.meta.delete_refused.len() != before
        };
        if told {
            self.request_save();
        }
    }

    /// What deleting the worktree of `id` would lose: its uncommitted files and its commits not in
    /// its base branch, read by git within `within`; past it (or git failing), unknown. An agent
    /// without a worktree of its own loses none of its files.
    pub async fn delete_loss(self: &Arc<Self>, id: &str, within: Duration) -> Result<Loss> {
        let worktree = self.agent(id)?.lock().meta.worktree.clone();
        let Some(wt) = worktree else {
            return Ok(Loss {
                known: true,
                ..Default::default()
            });
        };
        let read = async {
            let mut loss = Loss {
                known: true,
                ..Default::default()
            };
            for c in wt.checkouts() {
                let st = git::status(&c.dir).await?;
                loss.dirty += st.entries.len();
                for e in st.entries {
                    if loss.names.len() < LOSS_NAMES {
                        loss.names.push(if c.repo.is_empty() {
                            e.path
                        } else {
                            format!("{}/{}", c.repo, e.path)
                        });
                    }
                }
                if !c.base.is_empty() {
                    loss.unmerged += git::ahead_of(&c.dir, &c.base, "HEAD").await?;
                    if loss.base.is_empty() {
                        loss.base = c.base.clone();
                    }
                }
            }
            anyhow::Ok(loss)
        };
        Ok(match tokio::time::timeout(within, read).await {
            Ok(Ok(loss)) => loss,
            Ok(Err(e)) => {
                log::warn!("agent {id}: what its deletion would lose could not be read: {e:#}");
                Loss::default()
            }
            Err(_) => {
                log::warn!("agent {id}: git did not say in time what its deletion would lose");
                Loss::default()
            }
        })
    }
}

/// The line an agent reads before its next message once the user refused deletions it asked for.
pub(crate) fn refusal_line(lang: Lang, names: &[String]) -> String {
    let list = names
        .iter()
        .map(|n| match lang {
            Lang::Fr => format!("« {n} »"),
            Lang::En => format!("“{n}”"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    tr_in!(
        lang,
        "Note d'Escouade : l'utilisateur a refusé de supprimer {list}, que tu lui avais demandé de supprimer (delete_agent) : ils restent.",
        "Escouade's note: the user refused to delete {list}, which you had asked them to delete (delete_agent): they stay."
    )
}

/// The deletions asked by an agent that is not among `agents` go (read at the start).
pub(crate) fn prune(agents: &mut [crate::model::AgentMeta]) {
    let ids: std::collections::HashSet<String> = agents.iter().map(|m| m.id.clone()).collect();
    for m in agents.iter_mut() {
        if m.delete_ask.as_ref().is_some_and(|a| !ids.contains(&a.by)) {
            m.delete_ask = None;
        }
    }
}
