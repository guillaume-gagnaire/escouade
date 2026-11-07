//! The validation of a ticket whose agent has a worktree of several repositories (ESC-169): what
//! is left is committed in each, everything is checked, then the board's action (merge, pull
//! request, push) goes in each repository the branch changed, or in none.

use crate::board::{self, Outcome};
use crate::core::Core;
use crate::git;
use crate::model::*;
use crate::repos::{join, label, prefixed, repo_patterns};
use crate::testlaunch;
use anyhow::{bail, Result};
use std::sync::Arc;
use tauri::Runtime;

/// One repository of the validation.
struct Step {
    repo: String,
    /// Its folder in the project, and its worktree as a worktree of one.
    repo_dir: String,
    one: Worktree,
    target: String,
}

/// The outcomes of the repositories on one line, the glyph of the first in front as a card reads it
/// (« ⤵ api : Mergé dans main · squash · front : Mergé dans develop · squash »).
fn joined(parts: Vec<(String, Outcome)>) -> Outcome {
    let glyph = |text: &str| -> Option<(String, String)> {
        let (g, rest) = text.split_once(' ')?;
        (!g.chars().any(char::is_alphanumeric)).then(|| (g.to_string(), rest.to_string()))
    };
    let line = |f: &dyn Fn(&Outcome) -> &String| {
        let mut lead = None;
        let body = parts
            .iter()
            .map(|(r, o)| match glyph(f(o)) {
                Some((g, rest)) => {
                    lead.get_or_insert(g);
                    format!("{r} : {rest}")
                }
                None => format!("{r} : {}", f(o)),
            })
            .collect::<Vec<_>>()
            .join(" · ");
        match lead {
            Some(g) => format!("{g} {body}"),
            None => body,
        }
    };
    Outcome {
        ui: line(&|o| &o.ui),
        claude: line(&|o| &o.claude),
        url: parts.iter().find_map(|(_, o)| o.url.clone()),
        commit: parts.iter().find_map(|(_, o)| o.commit.clone()),
    }
}

impl<R: Runtime> Core<R> {
    /// The validation's steps between its tests and the ticket done: the outcome, and whether
    /// the branch brought nothing anywhere. None when it stopped on a conflict (handled as the
    /// board says).
    pub(crate) async fn validate_repos(
        self: &Arc<Self>,
        t: &Ticket,
        s: &BoardSettings,
        project: &Project,
        wt: &Worktree,
        agent_id: &str,
    ) -> Result<Option<(Outcome, bool)>> {
        let lang = self.lang().claude;
        let mut steps = Vec::new();
        for c in wt.checkouts() {
            let repo_dir = join(&project.path, &c.repo);
            let target = if c.base.is_empty() {
                self.base_of(project, &repo_dir).await
            } else {
                c.base.clone()
            };
            steps.push(Step {
                repo: c.repo.clone(),
                repo_dir,
                one: Worktree {
                    path: c.dir.clone(),
                    branch: wt.branch.clone(),
                    base_branch: c.base.clone(),
                    existing: wt.existing,
                    named_branch: wt.named_branch,
                    repos: Vec::new(),
                },
                target,
            });
        }
        if s.action == "keep" {
            return Ok(Some((Outcome::of(lang, board::kept_outcome), false)));
        }
        self.still_validating(&t.id, agent_id)?;
        self.set_step(&t.id, "Commit…");
        let mut message: Option<String> = None;
        let mut changed: Vec<&Step> = Vec::new();
        for st in &steps {
            let dir = &st.one.path;
            let name = label(&st.repo);
            if !git::branch_exists(dir, &st.target).await {
                bail!(tr!(
                    "{name} : la branche cible {target} n'existe pas.",
                    "{name}: the target branch {target} doesn’t exist.",
                    target = st.target
                ));
            }
            let unmerged = git::unmerged(dir).await;
            if !unmerged.is_empty() {
                bail!(tr!(
                    "{name} : conflits non résolus dans le worktree sur : {files}",
                    "{name}: unresolved conflicts in the worktree on: {files}",
                    files = unmerged.join(", ")
                ));
            }
            if git::current_branch(dir).await != st.one.branch {
                bail!(tr!(
                    "{name} : le worktree n'est plus sur la branche {b}",
                    "{name}: the worktree is no longer on the branch {b}",
                    b = st.one.branch
                ));
            }
            let patterns = repo_patterns(&project.repos, &project.worktree_copy, &st.repo);
            let copied = testlaunch::matching_ignored(&st.repo_dir, &patterns).await;
            let keep_out = crate::tickets::not_committed(dir, &copied).await;
            if git::stage_all(dir, &keep_out).await? {
                let m = match &message {
                    Some(m) => m.clone(),
                    None => self.commit_message(t, s, dir, &st.target).await,
                };
                git::commit_staged(dir, &m).await?;
                message = Some(m);
            }
            let in_tree = git::changed_in(dir, &st.target, &st.one.branch, &copied).await?;
            let in_history = git::touched_by(dir, &st.target, &st.one.branch, &copied).await?;
            if let Some(refusal) = board::copied_refusal(crate::i18n::ui(), &in_tree, &in_history) {
                bail!("{name} : {refusal}");
            }
            if git::ahead_of(dir, &st.target, &st.one.branch).await? > 0 {
                changed.push(st);
            }
        }
        self.still_validating(&t.id, agent_id)?;
        if changed.is_empty() {
            return Ok(Some((Outcome::of(lang, board::nothing_outcome), true)));
        }
        let message = match message {
            Some(m) => m,
            None => {
                self.commit_message(t, s, &changed[0].one.path, &changed[0].target)
                    .await
            }
        };
        if s.action == "merge" {
            // Everything checked first: nothing is merged while one repository can't be.
            self.set_step(&t.id, "Merge…");
            let (mut conflicts, mut where_) = (Vec::new(), Vec::new());
            for st in &changed {
                let files = git::merge_conflicts(&st.repo_dir, &st.target, &st.one.branch).await?;
                if !files.is_empty() && where_.is_empty() {
                    where_.push((label(&st.repo), st.target.clone()));
                } else if !files.is_empty() {
                    where_.push((label(&st.repo), String::new()));
                }
                conflicts.extend(files.iter().map(|f| prefixed(&st.repo, f)));
            }
            if !conflicts.is_empty() {
                // « Conflit avec develop dans front : rien n’est mergé ».
                let repos = where_
                    .iter()
                    .map(|(r, _)| r.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                let target = tr!(
                    "{t} dans {repos} : rien n’est mergé",
                    "{t} in {repos}: nothing is merged",
                    t = where_[0].1
                );
                self.on_conflict(t, s, &target, conflicts).await?;
                return Ok(None);
            }
            let mut parts = Vec::new();
            for st in &changed {
                let repo = self
                    .toplevel(&st.repo_dir)
                    .await
                    .ok_or_else(crate::core::not_a_repo)?;
                match self
                    .merge_ticket_in(
                        t,
                        s,
                        project,
                        &st.one,
                        &st.target,
                        Some(message.clone()),
                        &repo,
                    )
                    .await
                {
                    Ok(Some(done)) => parts.push((label(&st.repo), done)),
                    // A conflict found by the merge itself (the target moved meanwhile).
                    Ok(None) => return Ok(None),
                    Err(e) if parts.is_empty() => return Err(e.context(label(&st.repo))),
                    Err(e) => bail!(tr!(
                        "Mergés : {d}. {r} : {e:#}",
                        "Merged: {d}. {r}: {e:#}",
                        d = parts
                            .iter()
                            .map(|(r, _)| r.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                        r = label(&st.repo)
                    )),
                }
            }
            return Ok(Some((joined(parts), false)));
        }
        self.set_step(&t.id, "Push…");
        let mut parts = Vec::new();
        for st in &changed {
            let done = if s.action == "pr" {
                self.open_pr(t, s, &st.one, &st.target, Some(message.clone()))
                    .await?
                    .0
            } else {
                git::push_branch(&st.one.path, &st.one.branch).await?;
                Outcome::of(lang, |l| board::pushed_outcome(l, &st.one.branch))
            };
            parts.push((label(&st.repo), done));
        }
        Ok(Some((joined(parts), false)))
    }
}
