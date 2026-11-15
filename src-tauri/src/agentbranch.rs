//! The branch of the agent open, as the status bar shows it: the one git says its folder is on
//! (`GitInfo::agent_git`), followed when the agent switches by itself, and switched by the user
//! from the status bar's picker. A worktree switched by the user tells the agent with the next
//! message it gets (`branch_moved`), never by a turn of its own.

use crate::core::{Core, SyncOp};
use crate::forge::{PrState, PullRequest};
use crate::git;
use crate::i18n::{self, Lang};
use crate::model::{AgentGit, BranchMove, RepoGit};
use anyhow::{bail, Result};
use std::sync::Arc;

/// What the status bar shows of a checkout of an agent's own, from its `git status`.
pub fn agent_git_of(st: &git::Status, has_remote: bool) -> AgentGit {
    AgentGit {
        branch: st.branch.clone(),
        upstream: st.upstream.clone(),
        upstream_gone: st.upstream_gone,
        ahead: st.ahead,
        behind: st.behind,
        has_remote,
        repos: Vec::new(),
    }
}

/// An agent's checkouts of several repositories (ESC-169), as one: their sums, and their branch
/// when they share it.
pub fn agent_git_of_repos(repos: Vec<RepoGit>) -> AgentGit {
    let mut g = AgentGit::default();
    for r in &repos {
        g.ahead += r.ahead;
        g.behind += r.behind;
        g.has_remote |= r.has_remote;
    }
    let mut branches = repos.iter().filter(|r| !r.gone).map(|r| &r.branch);
    if let Some(first) = branches.next() {
        if branches.all(|b| b == first) {
            g.branch = first.clone();
        }
    }
    g.repos = repos;
    g
}

/// The line of the conversation that says the agent's folder went to another branch; `deleted`:
/// the branch made for it, which had nothing of its own, and went.
pub fn moved_line(lang: Lang, from: &str, to: &str, deleted: Option<&str>) -> String {
    let line = tr_in!(lang, "Branche : {from} → {to}.", "Branch: {from} → {to}.");
    match deleted {
        Some(gone) => {
            line + &tr_in!(
                lang,
                " « {gone} » n’avait aucun commit à elle : supprimée.",
                " “{gone}” had no commit of its own: deleted."
            )
        }
        None => line,
    }
}

/// What the agent is told, before the next message it gets, of the branch the user switched its
/// folder to, with its pull request when one is known.
pub fn moved_note(lang: Lang, mv: &BranchMove, word: &str, pr: Option<&PullRequest>) -> String {
    let (from, to) = (&mv.from, &mv.to);
    let mut note = tr_in!(
        lang,
        "Note d'Escouade : l'utilisateur a fait passer ton dossier de travail de la branche « {from} » à la branche « {to} ». Tiens compte de son état (git status, git log) avant de continuer.",
        "Escouade's note: the user switched your working folder from the branch “{from}” to the branch “{to}”. Take its state into account (git status, git log) before going on."
    );
    if let Some(p) = pr {
        let (n, title, base, url) = (p.number, &p.title, &p.base, &p.url);
        note.push(' ');
        note.push_str(&match p.state {
            PrState::Open => tr_in!(
                lang,
                "Elle a la {word} #{n} « {title} », ouverte vers {base} : {url}.",
                "It has {word} #{n} “{title}”, open into {base}: {url}."
            ),
            PrState::Merged => tr_in!(
                lang,
                "Sa {word} #{n} « {title} » est déjà mergée dans {base} : {url}.",
                "Its {word} #{n} “{title}” is already merged into {base}: {url}."
            ),
            PrState::Closed => tr_in!(
                lang,
                "Sa {word} #{n} « {title} » est fermée : {url}.",
                "Its {word} #{n} “{title}” is closed: {url}."
            ),
        });
    }
    note
}

/// Two moves before the agent was told: from where it was told last to where it is now; none
/// when it is back there.
pub fn chained(before: Option<&BranchMove>, from: &str, to: &str) -> Option<BranchMove> {
    let from = before.map_or(from, |b| b.from.as_str());
    (from != to).then(|| BranchMove {
        from: from.to_string(),
        to: to.to_string(),
    })
}

fn busy(agent: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "L’agent {agent} travaille : attends la fin de son tour.",
        "Agent {agent} is working: wait for the end of its turn."
    ))
}

fn not_switchable_ticket() -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "La branche d’un agent de ticket suit son ticket : elle ne change pas.",
        "A ticket agent’s branch follows its ticket: it doesn’t change."
    ))
}

impl Core {
    /// The agent `id` is on `head` in its worktree, as the git pass just read it: when that is
    /// another branch than its worktree's, it switched by itself (`git switch`, `gh pr checkout`),
    /// and its worktree follows: the branch taken is kept as the user's (it may be anyone's), its
    /// pull requests are read again, and its conversation says it. Never on a detached HEAD, for
    /// a ticket's agent (its board holds its branch), for several repositories, nor while
    /// Escouade renames or switches the branch itself (`naming-` lock): git moves first.
    pub(crate) async fn follow_agent_branch(self: &Arc<Self>, id: &str, head: &str) {
        if head.is_empty() || head == "(detached)" {
            return;
        }
        let Ok(h) = self.agent(id) else { return };
        let (project_id, from, path) = {
            let rt = h.lock();
            let Some(wt) = rt.meta.worktree.as_ref() else {
                return;
            };
            if wt.is_multi() || rt.meta.ticket_id.is_some() || rt.meta.archived || wt.branch == head
            {
                return;
            }
            (
                rt.meta.project_id.clone(),
                wt.branch.clone(),
                wt.path.clone(),
            )
        };
        let naming = self.spawn_lock(&format!("naming-{project_id}"));
        let Ok(_naming) = naming.try_lock() else {
            return;
        };
        // Read again under the lock: a rename that just ended has its worktree say so.
        if git::head_branch(&path).await != head {
            return;
        }
        let lang = i18n::ui();
        let moved = self.with_agent(id, |rt, fx| {
            let Some(wt) = rt.meta.worktree.as_mut().filter(|w| w.branch == from) else {
                return Ok(false);
            };
            wt.branch = head.to_string();
            wt.existing = true;
            wt.named_branch = false;
            rt.meta.prs.clear();
            // It knows where it is now: a move the user made before is no news any more.
            rt.meta.branch_moved = None;
            rt.notice("info", moved_line(lang, &from, head, None), fx);
            fx.save = true;
            Ok(true)
        });
        if moved.unwrap_or(false) {
            self.pr_reads.lock().remove(id);
        }
    }

    /// Switches the agent's worktree to `name`: a local branch, or a remote one (`origin/feat`)
    /// through the local branch that tracks it (made when there is none). Refused for an archived
    /// agent, a ticket's, one of several repositories, one without a worktree of Escouade's, one
    /// in the middle of a turn, with uncommitted changes, in the middle of a merge or a rebase, on
    /// a detached HEAD whose commits no branch has, for the board's target and for a branch
    /// another folder has. The branch taken is the user's (`existing`); the one left, made for the
    /// agent, goes when it has no commit beyond its base. The agent is told with its next message
    /// (`branch_moved`), but in the Claude Code view, where nothing is added to what is typed.
    /// Returns the branch it is on.
    pub async fn agent_branch_switch(self: &Arc<Self>, id: &str, name: &str) -> Result<String> {
        let _working = self.working();
        let meta = self.agent(id)?.lock().meta.clone();
        if meta.archived {
            bail!(tr!("Cet agent est archivé.", "This agent is archived."));
        }
        if meta.ticket_id.is_some() {
            return Err(not_switchable_ticket());
        }
        let Some(wt) = meta.worktree.clone() else {
            bail!(tr!(
                "Cet agent n’a pas de worktree d’Escouade : sa branche est celle du dossier où il travaille.",
                "This agent has no worktree of Escouade’s: its branch is the one of the folder it works in."
            ));
        };
        if wt.is_multi() {
            bail!(tr!(
                "Un agent de plusieurs dépôts ne change pas encore de branche d’ici.",
                "An agent of several repositories can’t switch branches from here yet."
            ));
        }
        if meta.status.is_active() {
            return Err(busy(&meta.name));
        }
        let project = self.project(&meta.project_id)?;
        let root = wt.path.clone();
        // Never across a rename of its branch, nor taken for the agent's own switch by the git pass;
        // nor under a fetch, a pull or a switch of the repository it shares its refs with.
        let naming = self.spawn_lock(&format!("naming-{}", project.id));
        let _naming = naming.lock().await;
        let lock = self.sync_lock(&self.repo_root(&project, None).await?);
        let _guard = lock.lock().await;
        // A turn may have started while it waited.
        if self.agent(id)?.lock().meta.status.is_active() {
            return Err(busy(&meta.name));
        }
        let from = match git::head_branch(&root).await {
            b if b.is_empty() => wt.branch.clone(),
            b => b,
        };
        // The board merges into its target: no worktree may hold it.
        let target = project.board.target.trim();
        if !target.is_empty() && git::local_name(&root, name).await? == target {
            bail!(crate::core::target_branch_refusal(i18n::ui()));
        }
        if let Some(local) = git::local_of(&root, name).await? {
            if let Some(path) = git::checkout_of(&root, &local).await? {
                if crate::tickets::same_dir(&path, &project.path) {
                    bail!(tr!(
                        "La branche « {local} » est celle du dossier du projet : change d’abord la sienne.",
                        "The branch “{local}” is the project folder’s: switch that one first."
                    ));
                }
            }
            if self.checked_out_here(&root, &local).await? {
                return Ok(local);
            }
        }
        if let Some(operation) = git::operation_in_progress(&root).await {
            bail!(tr!(
                "Un {operation} est en cours dans le worktree de l’agent : termine-le ou abandonne-le avant de changer de branche.",
                "A {operation} is in progress in the agent’s worktree: finish or abort it before switching branches."
            ));
        }
        let lost = git::detached_commits(&root).await?;
        if lost > 0 {
            bail!(tr_n!(
                lost,
                "Le worktree de l’agent est sur un HEAD détaché qui porte {n} commit qu’aucune branche n’a : crée une branche avant de partir.",
                "Le worktree de l’agent est sur un HEAD détaché qui porte {n} commits qu’aucune branche n’a : crée une branche avant de partir.",
                "The agent’s worktree is on a detached HEAD that holds {n} commit no branch has: create a branch before leaving.",
                "The agent’s worktree is on a detached HEAD that holds {n} commits no branch has: create a branch before leaving.",
                n = lost
            ));
        }
        if git::has_tracked_changes(&root).await? {
            bail!(tr!(
                "Il reste des changements non commités dans le worktree de l’agent : commite-les ou annule-les avant de changer de branche.",
                "There are uncommitted changes in the agent’s worktree: commit or discard them before switching branches."
            ));
        }
        let to = git::switch_to(&root, name).await?;
        // The branch it left, when made for it with nothing of its own: it would only clutter.
        let left = wt.branch.clone();
        let deleted = !wt.existing
            && left != to
            && !wt.base_branch.is_empty()
            && git::ahead_of(&root, &wt.base_branch, &left).await.ok() == Some(0)
            && git::branch_delete(&root, &left, true).await.is_ok();
        let lang = i18n::ui();
        self.with_agent(id, |rt, fx| {
            if let Some(w) = rt.meta.worktree.as_mut() {
                w.branch = to.clone();
                w.existing = true;
                w.named_branch = false;
            }
            rt.meta.prs.clear();
            rt.meta.branch_moved = if rt.meta.terminal {
                None
            } else {
                chained(rt.meta.branch_moved.as_ref(), &from, &to)
            };
            let gone = deleted.then_some(left.as_str());
            rt.notice("info", moved_line(lang, &from, &to, gone), fx);
            fx.save = true;
            Ok(())
        })?;
        self.pr_reads.lock().remove(id);
        // Its conversation, shown, reads the pull requests of the branch taken once the git pass
        // says it is there.
        self.git.refresh(&project.id);
        Ok(to)
    }

    /// Fetch, pull or push of the agent's own folder (its worktree, or the one it went into by
    /// itself), asked by the user from the status bar's picker. Returns a summary for the user.
    pub async fn agent_git_sync(self: &Arc<Self>, id: &str, op: SyncOp) -> Result<String> {
        let meta = self.agent(id)?.lock().meta.clone();
        let root = match (&meta.worktree, &meta.found_worktree) {
            (Some(wt), _) if !wt.is_multi() => wt.path.clone(),
            (None, Some(f)) => f.path.clone(),
            _ => bail!(tr!(
                "Cet agent n’a pas de dossier à lui à synchroniser.",
                "This agent has no folder of its own to sync."
            )),
        };
        // Its refs are the repository's: one fetch, pull or switch of it at a time.
        let project = self.project(&meta.project_id)?;
        let lock = self.sync_lock(&self.repo_root(&project, None).await?);
        let out = {
            let _guard = lock.lock().await;
            match op {
                SyncOp::Fetch => match git::fetch(&root, false).await {
                    Ok(()) => git::status(&root).await.map(|st| git::fetch_summary(&st)),
                    Err(e) => Err(e),
                },
                SyncOp::Pull => git::pull(&root).await,
                SyncOp::Push => git::push(&root).await,
            }
        };
        // Even after a failure: a pull that could not fast-forward has fetched.
        self.git.refresh(&meta.project_id);
        out
    }

    /// The note the agent's next message starts with, when the user switched its worktree since
    /// it was last told (`branch_moved`): in the language of what agents are told, with the pull
    /// request of the branch taken when it was read meanwhile.
    pub(crate) fn branch_note(&self, id: &str) -> Option<(BranchMove, String)> {
        let h = self.agent(id).ok()?;
        let rt = h.lock();
        let mv = rt.meta.branch_moved.clone()?;
        let word = rt.meta.pr_word.clone().unwrap_or_else(|| "PR".into());
        let pr = rt.meta.prs.iter().find(|p| p.head == mv.to);
        let note = moved_note(self.lang().claude, &mv, &word, pr);
        Some((mv, note))
    }

    /// The agent was told of `mv` (its message delivered): told once.
    pub(crate) fn branch_told(&self, id: &str, mv: &BranchMove) {
        let Ok(h) = self.agent(id) else { return };
        let told = {
            let mut rt = h.lock();
            let same = rt.meta.branch_moved.as_ref() == Some(mv);
            if same {
                rt.meta.branch_moved = None;
            }
            same
        };
        if told {
            self.request_save();
        }
    }
}
