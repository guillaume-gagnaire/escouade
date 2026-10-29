//! The worktrees of tickets that are over (ESC-143): done, whatever the validation did, or
//! deleted. With the board's « Supprimer le worktree » (`BoardSettings::cleanup`), each goes after
//! the project's teardown, unless it holds work nothing else keeps: changes not committed, or
//! commits that are neither merged nor pushed. That one stays, and its agent's card says why
//! (`AgentMeta::worktree_kept`). The same goes for the archived agents' worktrees left behind, by
//! « Nettoyer » in the project's settings.

use crate::core::Core;
use crate::engine::Process;
use crate::git;
use crate::i18n::{self, Lang};
use crate::model::*;
use crate::tickets::same_dir;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::Runtime;

/// What a worktree holds that its removal would lose: changes not committed, commits of its
/// branch that are neither in the target branch (merged, squashed too) nor in any other branch,
/// local or remote (pushed, proposed in a pull request).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unsaved {
    Changes(usize),
    Commits(u32),
    /// git could not tell: kept, to be safe.
    Unknown,
}

impl Unsaved {
    pub(crate) fn text(self, lang: Lang) -> String {
        match self {
            Unsaved::Changes(n) => tr_in!(
                lang,
                "worktree gardé : {n} fichier(s) non commité(s)",
                "worktree kept: {n} uncommitted file(s)"
            ),
            Unsaved::Commits(n) => tr_in!(
                lang,
                "worktree gardé : {n} commit(s) ni mergé(s) ni poussé(s)",
                "worktree kept: {n} commit(s) neither merged nor pushed"
            ),
            Unsaved::Unknown => tr_in!(
                lang,
                "worktree gardé : git n’a pas pu dire s’il reste du travail",
                "worktree kept: git couldn’t tell whether work is left"
            ),
        }
    }
}

/// What `unsaved_work` finds, and whether the branch's work is in `target` (then it may go with
/// its worktree).
pub(crate) struct Inspection {
    pub unsaved: Option<Unsaved>,
    pub merged: bool,
}

/// What `wt` (of the repository `repo`) holds that would be lost with it, `target` being where the
/// project's tickets merge. A folder gone holds nothing.
pub(crate) async fn unsaved_work(repo: &str, wt: &Worktree, target: &str) -> Inspection {
    let mut found = Inspection {
        unsaved: None,
        merged: false,
    };
    if Path::new(&wt.path).is_dir() {
        match git::status(&wt.path).await {
            Ok(s) if !s.entries.is_empty() => {
                found.unsaved = Some(Unsaved::Changes(s.entries.len()));
                return found;
            }
            Ok(_) => {}
            Err(_) => {
                found.unsaved = Some(Unsaved::Unknown);
                return found;
            }
        }
    }
    if !git::branch_exists(repo, &wt.branch).await {
        return found;
    }
    let head = format!("refs/heads/{}", wt.branch);
    if !target.is_empty() && git::is_merged(repo, &head, target).await {
        found.merged = true;
        return found;
    }
    match git::unique_commits(repo, &[head]).await {
        Ok(0) => {}
        Ok(n) => found.unsaved = Some(Unsaved::Commits(n)),
        Err(_) => found.unsaved = Some(Unsaved::Unknown),
    }
    found
}

/// What became of an agent's worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cleaned {
    /// No worktree, already gone, or another agent's too.
    Nothing,
    Removed,
    /// Kept, and why (also on the agent's card).
    Kept(String),
}

/// What « Nettoyer » found in a project.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeftoverReport {
    pub removed: u32,
    /// The agents whose worktree stays, with why.
    pub kept: Vec<KeptWorktree>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeptWorktree {
    pub agent_id: String,
    pub name: String,
    pub reason: String,
}

/// How many times, 300 ms apart, a worktree's removal is tried: a process just killed (on
/// Windows) lets go of its folder a moment later.
const REMOVE_ATTEMPTS: u32 = 10;

impl<R: Runtime> Core<R> {
    /// Removes `wt` after the project's teardown (`ports`: those its agent held), its branch too
    /// unless `keep_branch`. `proc`, its agent's process, is killed first with all it started: on
    /// Windows a live process keeps the folder. Tried again for a few seconds.
    pub(crate) async fn remove_worktree(
        &self,
        project: &Project,
        wt: &Worktree,
        proc: Option<Process>,
        ports: Option<u16>,
        keep_branch: bool,
    ) -> Result<()> {
        if let Some(p) = proc {
            p.kill();
            // Gone with all it started (see `wait_gone`); the removal below says if the folder stayed.
            let _ = p.wait_gone(self.agent_gone()).await;
        }
        if let Some(problem) = self.teardown_worktree(project, wt, ports).await {
            log::warn!("{}: {problem}", wt.path);
        }
        let mut attempt = 1;
        let done = loop {
            let removed = if wt.is_multi() {
                crate::multiwt::remove(project, wt, keep_branch, git::REMOVE_RETRY_HELD).await
            } else if keep_branch {
                git::worktree_remove_dir(&project.path, &wt.path).await
            } else {
                git::worktree_remove(&project.path, &wt.path, &wt.branch).await
            };
            match removed {
                Err(_) if attempt < REMOVE_ATTEMPTS => {
                    attempt += 1;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
                done => break done,
            }
        };
        self.git.refresh(&project.id);
        if done.is_ok() {
            self.facts.emit(
                crate::facts::Body::WorktreeRemoved {
                    branch: wt.branch.clone(),
                    path: wt.path.clone(),
                },
                crate::facts::About::project(&project.id),
            );
        }
        done
    }

    /// The worktree of the archived agent `id`, its ticket over, goes when it holds nothing that
    /// would be lost; its branch with it when its work is in the target branch (and it is not the
    /// user's, `Worktree::existing`). Kept, the agent's card says why. One another agent still
    /// works in, or whose branch another has, is left to it.
    pub(crate) async fn clean_agent_worktree(
        self: &Arc<Self>,
        id: &str,
        proc: Option<Process>,
        ports: Option<u16>,
    ) -> Cleaned {
        let Ok(h) = self.agent(id) else {
            return Cleaned::Nothing;
        };
        let (pid, wt, archived) = {
            let rt = h.lock();
            (
                rt.meta.project_id.clone(),
                rt.meta.worktree.clone(),
                rt.meta.archived,
            )
        };
        let (Some(wt), true, Ok(project)) = (wt, archived, self.project(&pid)) else {
            return Cleaned::Nothing;
        };
        if !Path::new(&wt.path).is_dir() {
            let _ = git::run(&project.path, &["worktree", "prune"]).await;
            self.set_worktree_kept(id, None);
            return Cleaned::Nothing;
        }
        // Another agent's folder or branch (a ticket started again on its branch, a worktree
        // restored beside it): never taken from it.
        let others: Vec<Worktree> = self
            .project_agents(&pid)
            .iter()
            .filter_map(|h| {
                let rt = h.lock();
                (rt.meta.id != id && !rt.meta.archived)
                    .then(|| rt.meta.worktree.clone())
                    .flatten()
            })
            .collect();
        if others.iter().any(|o| same_dir(&o.path, &wt.path)) {
            return Cleaned::Nothing;
        }
        let shared_branch = others.iter().any(|o| o.branch == wt.branch);
        // Of several repositories (ESC-169): each one's worktree judged apart.
        if wt.is_multi() {
            let keep = wt.existing || shared_branch;
            let kept = self.clean_multi(&project, &wt, keep, ports, true).await;
            return match kept {
                None => {
                    self.set_worktree_kept(id, None);
                    Cleaned::Removed
                }
                Some((left, reason)) => {
                    let _ = self.with_agent(id, |rt, fx| {
                        if let Some(w) = rt.meta.worktree.as_mut() {
                            w.repos.retain(|r| left.contains(&r.repo));
                        }
                        fx.save = true;
                        Ok(())
                    });
                    self.set_worktree_kept(id, Some(reason.clone()));
                    Cleaned::Kept(reason)
                }
            };
        }
        let target = self.target_of(&project).await;
        let found = unsaved_work(&project.path, &wt, &target).await;
        if let Some(unsaved) = found.unsaved {
            let reason = unsaved.text(i18n::ui());
            log::info!("agent {id}: {reason}");
            self.set_worktree_kept(id, Some(reason.clone()));
            return Cleaned::Kept(reason);
        }
        let keep_branch = wt.existing || shared_branch || !found.merged;
        match self
            .remove_worktree(&project, &wt, proc, ports, keep_branch)
            .await
        {
            Ok(()) => {
                self.set_worktree_kept(id, None);
                Cleaned::Removed
            }
            Err(e) => {
                log::warn!("agent {id}: worktree not removed: {e:#}");
                let reason = not_removed(i18n::ui(), &e);
                self.set_worktree_kept(id, Some(reason.clone()));
                Cleaned::Kept(reason)
            }
        }
    }

    /// The worktrees of an agent's folder of several repositories (ESC-169), each judged apart:
    /// one that holds work stays, the others go, their branch too when its work is in its base
    /// (unless `keep`). None when all went (the folder too, after the teardown); else the
    /// repositories left, and why, in one sentence.
    pub(crate) async fn clean_multi(
        &self,
        project: &Project,
        wt: &Worktree,
        keep: bool,
        ports: Option<u16>,
        teardown: bool,
    ) -> Option<(Vec<String>, String)> {
        let mut kept: Vec<(String, String)> = Vec::new();
        let mut going = Vec::new();
        for c in wt.checkouts() {
            let repo_dir = crate::repos::join(&project.path, &c.repo);
            let one = Worktree {
                path: c.dir.clone(),
                branch: wt.branch.clone(),
                base_branch: c.base.clone(),
                existing: wt.existing,
                named_branch: wt.named_branch,
                repos: Vec::new(),
            };
            let found = unsaved_work(&repo_dir, &one, &c.base).await;
            match found.unsaved {
                Some(u) => kept.push((c.repo.clone(), u.text(i18n::ui()))),
                None => going.push((c.repo.clone(), keep || !found.merged)),
            }
        }
        if kept.is_empty() && teardown {
            if let Some(problem) = self.teardown_worktree(project, wt, ports).await {
                log::warn!("{}: {problem}", wt.path);
            }
        }
        for (repo, keep_branch) in &going {
            if let Err(e) = crate::multiwt::remove_one(project, wt, repo, *keep_branch).await {
                kept.push((repo.clone(), not_removed(i18n::ui(), &e)));
            }
        }
        if kept.is_empty() {
            let _ = std::fs::remove_dir_all(&wt.path);
            return None;
        }
        let reason = kept
            .iter()
            .map(|(r, why)| format!("{} : {why}", crate::repos::label(r)))
            .collect::<Vec<_>>()
            .join(" ; ");
        Some((kept.into_iter().map(|(r, _)| r).collect(), reason))
    }

    /// Says on the agent's card why its worktree was kept, or nothing any more.
    pub(crate) fn set_worktree_kept(self: &Arc<Self>, id: &str, why: Option<String>) {
        let _ = self.with_agent(id, |rt, fx| {
            if rt.meta.worktree_kept != why {
                rt.meta.worktree_kept = why;
                fx.save = true;
            }
            Ok(())
        });
    }

    /// The worktrees of the ticket `ticket_id`'s archived agents, `except` aside (the one its
    /// validation already handled), cleaned as `clean_agent_worktree` does: a ticket sent back and
    /// taken over has several.
    pub(crate) async fn clean_ticket_worktrees(self: &Arc<Self>, ticket_id: &str, except: &str) {
        let ids: Vec<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                let m = &rt.meta;
                (m.archived
                    && m.id != except
                    && m.worktree.is_some()
                    && m.ticket_id.as_deref() == Some(ticket_id))
                .then(|| m.id.clone())
            })
            .collect();
        for id in ids {
            self.clean_agent_worktree(&id, None, None).await;
        }
    }

    /// « Nettoyer » in the project's settings: the worktrees of its archived agents that are still
    /// on the disk, cleaned as `clean_agent_worktree` does, then those git still lists whose
    /// folder went (`git worktree prune`).
    pub async fn clean_leftover_worktrees(
        self: &Arc<Self>,
        project_id: &str,
    ) -> Result<LeftoverReport> {
        self.project(project_id)?;
        let ids: Vec<(String, String)> = self
            .project_agents(project_id)
            .iter()
            .filter_map(|h| {
                let rt = h.lock();
                let m = &rt.meta;
                (m.archived && m.worktree.is_some()).then(|| (m.id.clone(), m.name.clone()))
            })
            .collect();
        let mut report = LeftoverReport::default();
        for (id, name) in ids {
            match self.clean_agent_worktree(&id, None, None).await {
                Cleaned::Removed => report.removed += 1,
                Cleaned::Kept(reason) => report.kept.push(KeptWorktree {
                    agent_id: id,
                    name,
                    reason,
                }),
                Cleaned::Nothing => {}
            }
        }
        for repo in self.project_roots_of(project_id).await {
            let _ = git::run(&repo, &["worktree", "prune"]).await;
        }
        self.git.refresh(project_id);
        Ok(report)
    }
}

/// Why a worktree that could go is still there.
fn not_removed(lang: Lang, e: &anyhow::Error) -> String {
    tr_in!(
        lang,
        "worktree gardé : non supprimé ({e:#})",
        "worktree kept: not removed ({e:#})"
    )
}
