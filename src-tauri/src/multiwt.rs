//! The worktrees of an agent of a project of several repositories (ESC-169): a folder of its own
//! under the project's worktrees, with a worktree of each repository at its path from the
//! project's folder, all on one branch.

use crate::core::Core;
use crate::git;
use crate::model::*;
use crate::repos::{join, label, repo_patterns};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::Runtime;

/// What an agent's worktrees are made of.
#[derive(Debug, Default, Clone)]
pub struct Spec {
    /// The name its folder (and, without `branch`, its branch) is made from: `-2`… when taken.
    pub name: String,
    /// The branch, as named (a ticket's, one the user typed, one that exists); None:
    /// `<prefix><folder>`.
    pub branch: Option<String>,
    /// `branch` exists already, in some of the repositories at least: those that have it check it
    /// out, the others make it.
    pub existing: bool,
    /// `branch` was typed by the user: refused when taken, never moved to `-2`.
    pub strict: bool,
    /// The branch chosen to start from (ESC-160): else the project's « Branche de départ ».
    pub over: Option<String>,
    /// For a copy: the commit each repository's worktree starts from, and its base.
    pub starts: HashMap<String, (String, String)>,
    /// The agent whose card says how far it is (« Worktrees · 2 sur 3 dépôts »).
    pub agent: Option<String>,
}

/// A repository no worktree can start from: none in its folder, no commit, a detached HEAD.
async fn unusable(dir: &str) -> Option<String> {
    if !Path::new(dir).is_dir() {
        return Some(tr!("introuvable", "not found"));
    }
    match git::has_commit(dir).await {
        Ok(true) => {}
        Ok(false) => return Some(tr!("aucun commit", "no commit")),
        // Git failed to read it (ESC-687): said as such, never « no commit ».
        Err(e) => {
            return Some(tr!(
                "lecture impossible : {e}",
                "could not be read: {e}",
                e = format!("{e:#}")
            ))
        }
    }
    if git::head_branch(dir).await.is_empty() {
        return Some(tr!("HEAD détaché", "detached HEAD"));
    }
    None
}

/// The checkouts made so far (repository's folder, worktree, branch made by us), undone.
async fn undo(made: &[(String, String, bool)], branch: &str, folder: &Path) {
    for (repo_dir, dir, branch_made) in made.iter().rev() {
        let _ = git::worktree_remove_dir(repo_dir, dir).await;
        if *branch_made {
            let _ = git::branch_delete(repo_dir, branch, true).await;
        }
    }
    let _ = std::fs::remove_dir_all(folder);
}

/// Copies the files of the project's folder that are in none of its repositories (its own
/// `docker-compose.yml`, `README.md`…) into the agent's folder: its files only, never its folders.
fn copy_root_files(project: &Project, folder: &Path) {
    let Ok(read) = std::fs::read_dir(&project.path) else {
        return;
    };
    for e in read.flatten() {
        if e.file_type().is_ok_and(|t| t.is_file()) {
            let to = folder.join(e.file_name());
            if !to.exists() {
                let _ = std::fs::copy(e.path(), to);
            }
        }
    }
}

impl<R: Runtime> Core<R> {
    /// Makes an agent's folder with a worktree of each repository of the project that has a
    /// commit and is on a branch, all on one branch. Nothing is left when one fails. Those left
    /// out, and the starting branches not found, are said in `notice`.
    pub(crate) async fn multi_worktree(
        self: &std::sync::Arc<Self>,
        project: &Project,
        held: &[PathBuf],
        spec: &Spec,
        notice: &mut Option<String>,
    ) -> Result<Worktree> {
        let root = project.worktrees_root();
        std::fs::create_dir_all(&root).map_err(|e| {
            anyhow::anyhow!(tr!(
                "Dossier des worktrees {p} : {e}",
                "Worktrees folder {p}: {e}",
                p = root.display()
            ))
        })?;
        let mut repos = Vec::new();
        let mut left_out = Vec::new();
        for r in &project.repos {
            let dir = join(&project.path, r);
            match unusable(&dir).await {
                None => repos.push((r.clone(), dir)),
                Some(why) => left_out.push(format!("{} ({why})", label(r))),
            }
        }
        if repos.is_empty() {
            bail!(tr!(
                "aucun dépôt du projet n’a de commit sur une branche : impossible de créer un worktree",
                "no repository of the project has a commit on a branch: a worktree can’t be created"
            ));
        }
        // The root's repository, when it is one, holds the folders of the others: first.
        if repos.iter().any(|(r, _)| r.is_empty()) {
            git::exclude_worktrees(&project.path, &root).await?;
        }
        let last = spec
            .branch
            .as_deref()
            .unwrap_or(&spec.name)
            .rsplit('/')
            .next()
            .unwrap_or(&spec.name)
            .to_string();
        let mut dir_name = last.clone();
        let mut n = 2;
        let branch = loop {
            let branch = match &spec.branch {
                Some(b) if n == 2 || spec.strict || spec.existing => b.clone(),
                Some(b) => format!("{b}-{}", n - 1),
                None => format!("{}{dir_name}", project.agent_branch_prefix),
            };
            let mut taken = false;
            if !spec.existing {
                for (_, dir) in &repos {
                    if git::branch_exists(dir, &branch).await {
                        taken = true;
                    }
                }
            }
            if taken && spec.strict {
                return Err(git::branch_taken(&branch));
            }
            if !taken && git::free_folder(&root.join(&dir_name), held) {
                break branch;
            }
            dir_name = format!("{last}-{n}");
            n += 1;
        };
        let folder = root.join(&dir_name);
        let mut made: Vec<(String, String, bool)> = Vec::new();
        let mut list = Vec::new();
        let mut notes = Vec::new();
        let total = repos.len();
        for (i, (repo, repo_dir)) in repos.iter().enumerate() {
            if let Some(id) = &spec.agent {
                let done = i;
                self.show_step(
                    id,
                    tr_n!(
                        total,
                        "Worktrees · {done} sur {total} dépôt",
                        "Worktrees · {done} sur {total} dépôts",
                        "Worktrees · {done} of {total} repository",
                        "Worktrees · {done} of {total} repositories"
                    ),
                );
            }
            let dir = join(&folder.to_string_lossy(), repo);
            if let Some(parent) = Path::new(&dir).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let local = if spec.existing {
                git::local_of(repo_dir, &branch).await.ok().flatten()
            } else {
                None
            };
            let done = match &local {
                // The branch is there: its worktree, as « Nouvel agent sur une branche… ».
                Some(local) => {
                    let base = self.base_of(project, repo_dir).await;
                    git::run_hooks(repo_dir, &["worktree", "add", &dir, "--", local])
                        .await
                        .map(|_| (base, false))
                }
                None => {
                    let (start, base) = match spec.starts.get(repo) {
                        Some((commit, base)) => (commit.clone(), base.clone()),
                        None => {
                            let lock = self.sync_lock(repo_dir);
                            let _guard = lock.lock().await;
                            let (from, why) =
                                crate::core::start_point(project, repo_dir, spec.over.as_deref())
                                    .await;
                            if let Some(why) = why {
                                notes.push(format!("{} : {why}", label(repo)));
                            }
                            let base = match &from {
                                Some(b) => b.clone(),
                                None => git::head_branch(repo_dir).await,
                            };
                            let start = from
                                .map_or_else(|| "HEAD".to_string(), |b| format!("refs/heads/{b}"));
                            (start, base)
                        }
                    };
                    git::run_hooks(
                        repo_dir,
                        &["worktree", "add", "-b", &branch, &dir, "--", &start],
                    )
                    .await
                    .map(|_| (base, true))
                }
            };
            match done {
                Ok((base, branch_made)) => {
                    made.push((repo_dir.clone(), dir.clone(), branch_made));
                    list.push(RepoWorktree {
                        repo: repo.clone(),
                        base_branch: base,
                    });
                }
                Err(e) => {
                    undo(&made, &branch, &folder).await;
                    bail!("{} : {e:#}", label(repo));
                }
            }
        }
        copy_root_files(project, &folder);
        let names: Vec<String> = list.iter().map(|r| r.repo.clone()).collect();
        let mut failed = Vec::new();
        for r in &names {
            let patterns = repo_patterns(&project.repos, &project.worktree_copy, r);
            if let Err(e) = crate::testlaunch::copy_worktree_files(
                &join(&project.path, r),
                &join(&folder.to_string_lossy(), r),
                &patterns,
            )
            .await
            {
                failed.push(format!("{} : {e:#}", label(r)));
            }
        }
        if !failed.is_empty() {
            notes.push(tr!(
                "Fichiers non copiés dans le worktree : {f}",
                "Files not copied into the worktree: {f}",
                f = failed.join(" ; ")
            ));
        }
        if !left_out.is_empty() {
            notes.insert(
                0,
                tr!(
                    "Pas de worktree de {r} : un worktree part d’un commit, sur une branche.",
                    "No worktree of {r}: a worktree starts from a commit, on a branch.",
                    r = left_out.join(", ")
                ),
            );
        }
        if !notes.is_empty() {
            *notice = Some(notes.join("\n"));
        }
        Ok(Worktree {
            path: folder.to_string_lossy().into_owned(),
            branch,
            base_branch: list[0].base_branch.clone(),
            existing: spec.existing,
            named_branch: spec.strict,
            repos: list,
        })
    }

    /// The spec of a copy of the agent whose worktree is `wt`: each repository from the commit
    /// its worktree is on, toward the same base.
    pub(crate) async fn copy_spec(wt: &Worktree, name: &str, branch: &str) -> Result<Spec> {
        let mut starts = HashMap::new();
        for c in wt.checkouts() {
            let head = git::text(&c.dir, &["rev-parse", "HEAD"]).await?;
            starts.insert(c.repo.clone(), (head, c.base.clone()));
        }
        Ok(Spec {
            name: name.to_string(),
            branch: Some(branch.to_string()),
            starts,
            ..Default::default()
        })
    }
}

/// Removes the worktrees of an agent's folder, the deepest first, their branch too unless `keep`
/// (one the user's: `existing`), then the folder.
pub async fn remove(
    project: &Project,
    wt: &Worktree,
    keep: bool,
    retry: std::time::Duration,
) -> Result<()> {
    let mut checkouts = wt.checkouts();
    checkouts.sort_by_key(|c| {
        std::cmp::Reverse(c.repo.matches('/').count() + !c.repo.is_empty() as usize)
    });
    let mut failed = Vec::new();
    for c in &checkouts {
        let repo_dir = join(&project.path, &c.repo);
        let done = if keep {
            git::worktree_remove_dir_within(&repo_dir, &c.dir, retry).await
        } else {
            git::worktree_remove_within(&repo_dir, &c.dir, &wt.branch, retry).await
        };
        if let Err(e) = done {
            failed.push(format!("{} : {e:#}", label(&c.repo)));
        }
    }
    if Path::new(&wt.path).exists() && failed.is_empty() {
        if let Err(e) = std::fs::remove_dir_all(&wt.path) {
            failed.push(format!("{} : {e}", wt.path));
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        bail!(failed.join("\n"))
    }
}

/// One repository's worktree of an agent's folder, removed (its branch too unless `keep`): the
/// cleaning goes repository by repository.
pub async fn remove_one(project: &Project, wt: &Worktree, repo: &str, keep: bool) -> Result<()> {
    let repo_dir = join(&project.path, repo);
    let dir = join(&wt.path, repo);
    if keep {
        git::worktree_remove_dir(&repo_dir, &dir).await
    } else {
        git::worktree_remove(&repo_dir, &dir, &wt.branch).await
    }
}

/// Checks out again the worktrees of an agent's folder that went (its agent restored), the
/// root's first.
pub async fn restore(project: &Project, wt: &Worktree) -> Result<()> {
    for c in wt.checkouts() {
        let repo_dir = join(&project.path, &c.repo);
        if let Some(parent) = Path::new(&c.dir).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        git::worktree_restore(&repo_dir, &c.dir, &wt.branch)
            .await
            .map_err(|e| anyhow::anyhow!("{} : {e:#}", label(&c.repo)))?;
    }
    Ok(())
}

/// What merging an agent of several repositories would do in one of them (ESC-169): its window
/// lists them all, and nothing merges while one of those it changed can't be.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepoMerge {
    pub repo: String,
    pub base: String,
    /// Commits of the agent's branch not in the base: none, nothing to merge here.
    pub commits: u32,
    pub add: u32,
    pub del: u32,
    /// Why it can't be merged (a conflict, changes not committed, its base gone…).
    pub problem: Option<String>,
    /// The files it would conflict on.
    pub conflicts: Vec<String>,
}

impl<R: Runtime> Core<R> {
    /// Each repository of the agent's folder, as its merge would find it.
    pub async fn merge_check(&self, project: &Project, wt: &Worktree) -> Vec<RepoMerge> {
        let mut out = Vec::new();
        for c in wt.checkouts() {
            let repo_dir = join(&project.path, &c.repo);
            let mut m = RepoMerge {
                repo: c.repo.clone(),
                base: c.base.clone(),
                commits: 0,
                add: 0,
                del: 0,
                problem: None,
                conflicts: Vec::new(),
            };
            if !git::branch_exists(&repo_dir, &c.base).await {
                m.problem = Some(tr!(
                    "la branche de base « {b} » n’existe plus",
                    "the base branch “{b}” no longer exists",
                    b = c.base
                ));
                out.push(m);
                continue;
            }
            m.commits = git::ahead_of(&repo_dir, &c.base, &wt.branch)
                .await
                .unwrap_or(0);
            let dirty = git::status(&c.dir)
                .await
                .map(|s| s.entries.len())
                .unwrap_or(0);
            if dirty > 0 {
                m.problem = Some(tr_n!(
                    dirty,
                    "{dirty} fichier non commité dans son worktree",
                    "{dirty} fichiers non commités dans son worktree",
                    "{dirty} uncommitted file in its worktree",
                    "{dirty} uncommitted files in its worktree"
                ));
            }
            if m.commits == 0 {
                out.push(m);
                continue;
            }
            let (add, del) = git::diff_counts(&repo_dir, &c.base, &wt.branch).await;
            (m.add, m.del) = (add, del);
            if m.problem.is_none()
                && git::has_tracked_changes_but_tickets(&repo_dir)
                    .await
                    .unwrap_or(true)
            {
                m.problem = Some(tr!(
                    "le dépôt a des modifications non commitées",
                    "the repository has uncommitted changes"
                ));
            }
            match git::merge_conflicts(&repo_dir, &c.base, &wt.branch).await {
                Ok(files) if !files.is_empty() => {
                    m.problem.get_or_insert_with(|| {
                        tr!("conflit avec {b}", "conflict with {b}", b = c.base)
                    });
                    m.conflicts = files
                        .iter()
                        .map(|f| crate::repos::prefixed(&c.repo, f))
                        .collect();
                }
                Ok(_) => {}
                Err(e) => {
                    m.problem.get_or_insert_with(|| format!("{e:#}"));
                }
            }
            out.push(m);
        }
        out
    }

    /// Merges the agent's branch into each repository's base where it has commits (those of
    /// `only`, when given: « Merger api seul »), after `merge_check`: nothing is merged when one
    /// of them can't be. A repository whose folder is on another branch is switched to its base
    /// with `switch_to_base`, else refused (`NotOnBase`).
    pub(crate) async fn merge_multi(
        &self,
        project: &Project,
        name: &str,
        wt: &Worktree,
        squash: bool,
        switch_to_base: bool,
        only: Option<&[String]>,
    ) -> Result<String> {
        let checks: Vec<RepoMerge> = self
            .merge_check(project, wt)
            .await
            .into_iter()
            .filter(|m| m.commits > 0)
            .filter(|m| only.is_none_or(|o| o.contains(&m.repo)))
            .collect();
        if checks.is_empty() {
            bail!(tr!(
                "Rien à merger : la branche {b} n'a de nouveau commit dans aucun dépôt.",
                "Nothing to merge: the branch {b} has no new commit in any repository.",
                b = wt.branch
            ));
        }
        let refused: Vec<String> = checks
            .iter()
            .filter_map(|m| {
                m.problem
                    .as_ref()
                    .map(|p| format!("{} : {p}", label(&m.repo)))
            })
            .collect();
        if !refused.is_empty() {
            bail!(tr!(
                "Rien n’est mergé : {r}",
                "Nothing is merged: {r}",
                r = refused.join(" ; ")
            ));
        }
        for m in &checks {
            let repo_dir = join(&project.path, &m.repo);
            let current = git::head_branch(&repo_dir).await;
            if current != m.base && !switch_to_base {
                return Err(crate::core::NotOnBase {
                    current,
                    base: m.base.clone(),
                }
                .into());
            }
        }
        let mut done = Vec::new();
        for m in &checks {
            let repo_dir = join(&project.path, &m.repo);
            let step = async {
                if git::head_branch(&repo_dir).await != m.base {
                    git::switch(&repo_dir, &m.base).await?;
                }
                let message = if squash {
                    let subjects = git::text(
                        &repo_dir,
                        &[
                            "log",
                            "--format=- %s",
                            &format!("{}..{}", m.base, wt.branch),
                        ],
                    )
                    .await
                    .unwrap_or_default();
                    format!("{name}\n\n{subjects}")
                } else {
                    format!("Merge branch '{}'", wt.branch)
                };
                git::merge(&repo_dir, &wt.branch, squash, &message).await
            };
            match step.await {
                Ok(_) => done.push(label(&m.repo)),
                Err(e) if done.is_empty() => bail!("{} : {e:#}", label(&m.repo)),
                Err(e) => bail!(tr!(
                    "Mergés : {d}. {r} : {e:#}",
                    "Merged: {d}. {r}: {e:#}",
                    d = done.join(", "),
                    r = label(&m.repo)
                )),
            }
        }
        Ok(tr!(
            "Mergé dans {d}",
            "Merged into {d}",
            d = done.join(", ")
        ))
    }
}

impl<R: Runtime> Core<R> {
    /// The agent's worktree and its project.
    fn agent_wt(&self, id: &str) -> Result<(Project, String, Worktree)> {
        let h = self.agent(id)?;
        let (pid, name, wt) = {
            let rt = h.lock();
            (
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.meta.worktree.clone(),
            )
        };
        let wt = wt.ok_or_else(|| {
            anyhow::anyhow!(tr!(
                "l’agent n’a pas de worktree",
                "the agent has no worktree"
            ))
        })?;
        Ok((self.project(&pid)?, name, wt))
    }

    /// What its merge would do in each repository (ESC-169), for its window.
    pub async fn agent_merge_check(&self, id: &str) -> Result<Vec<RepoMerge>> {
        let (project, _, wt) = self.agent_wt(id)?;
        Ok(self.merge_check(&project, &wt).await)
    }

    /// « Merger api seul »: the merge of the repositories `repos` alone.
    pub async fn merge_agent_repos(
        &self,
        id: &str,
        squash: bool,
        switch_to_base: bool,
        repos: Vec<String>,
    ) -> Result<String> {
        let _working = self.working();
        let (project, name, wt) = self.agent_wt(id)?;
        let out = self
            .merge_multi(&project, &name, &wt, squash, switch_to_base, Some(&repos))
            .await;
        self.git.refresh(&project.id);
        out
    }
}
