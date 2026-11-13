//! A project of several git repositories (ESC-169): a folder that is not a repository and holds at
//! least one, or a repository that holds others than its submodules. `detect` finds them; the rest
//! of the app reads `Project::repos` (empty: a project of one repository, as before).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

// A repository's folder, which the projects name: in `escouade-model` (docs/CRATES.md).
pub use escouade_model::repos::join;

/// Never looked into: what a build, a package manager or the app itself puts in a folder.
const SKIPPED: [&str; 6] = [
    "node_modules",
    "target",
    "vendor",
    "dist",
    "build",
    "__pycache__",
];

/// Under the root, the folders looked at: its children, and theirs (`services/billing`).
const DEPTH: usize = 2;

/// The name a repository goes by: its folder from the project's (`services/billing`), « racine »
/// for the root's own.
pub fn label(repo: &str) -> String {
    if repo.is_empty() {
        tr!("racine", "root")
    } else {
        repo.to_string()
    }
}

/// Of the repositories `repos`, the one a path from the project's folder (with `/`) is in, and the
/// path from that repository: the deepest one, the root's (`""`) last.
pub fn split<'a>(repos: &'a [String], path: &'a str) -> Option<(&'a str, &'a str)> {
    repos
        .iter()
        .filter(|r| !r.is_empty())
        .filter_map(|r| {
            path.strip_prefix(r.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                .map(|rest| (r.as_str(), rest))
        })
        .max_by_key(|(r, _)| r.len())
        .or_else(|| repos.iter().any(String::is_empty).then_some(("", path)))
}

/// A path of the repository `repo` as said from the project's folder.
pub fn prefixed(repo: &str, path: &str) -> String {
    if repo.is_empty() {
        path.to_string()
    } else {
        format!("{repo}/{path}")
    }
}

fn is_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// The paths of the submodules of the repository at `root`, as its `.gitmodules` names them.
fn submodules(root: &Path) -> HashSet<String> {
    std::fs::read_to_string(root.join(".gitmodules"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "path").then(|| v.trim().trim_end_matches('/').replace('\\', "/"))
        })
        .collect()
}

/// The folders of `dir` worth looking into, by name: never a hidden one, nor one of `SKIPPED`,
/// nor `skip` (the project's worktrees folder), nor a link.
fn children(dir: &Path, skip: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = read
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = e.path();
            let skipped = name.starts_with('.')
                || SKIPPED.contains(&name.as_str())
                || skip.iter().any(|s| same(s, &path));
            (!skipped).then_some((name, path))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn same(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The repositories of the project's folder `root` (`worktrees`: its worktrees folder, never looked
/// into), from it with `/`, in the folder's order: empty for a folder that is one repository and
/// holds no other (a project as before), or that holds none. The root, when it is one and holds
/// others, comes first as `""`. A repository found is not looked into (its submodules are its
/// own); nor is a folder the root's repository ignores, unless it is a repository itself.
pub async fn detect(root: &Path, worktrees: &Path) -> Vec<String> {
    let root_repo = is_repo(root);
    let modules = if root_repo {
        submodules(root)
    } else {
        HashSet::new()
    };
    let skip = [worktrees.to_path_buf()];
    let mut found = Vec::new();
    // Breadth first, level by level: (path from the root, folder).
    let mut level = vec![(String::new(), root.to_path_buf())];
    for depth in 1..=DEPTH {
        let mut next = Vec::new();
        for (rel, dir) in &level {
            let mut plain = Vec::new();
            for (name, path) in children(dir, &skip) {
                let rel = if rel.is_empty() {
                    name
                } else {
                    format!("{rel}/{name}")
                };
                if is_repo(&path) {
                    if !modules.contains(&rel) {
                        found.push(rel);
                    }
                } else if depth < DEPTH {
                    plain.push((rel, path));
                }
            }
            if root_repo && !plain.is_empty() {
                let names: Vec<String> = plain.iter().map(|(r, _)| format!("{r}/")).collect();
                let ignored = crate::git::ignored(&root.to_string_lossy(), &names)
                    .await
                    .unwrap_or_default();
                plain.retain(|(r, _)| !ignored.contains(&format!("{r}/")));
            }
            next.extend(plain);
        }
        level = next;
    }
    found.sort();
    if root_repo && !found.is_empty() {
        found.insert(0, String::new());
    }
    found
}

/// How often the window coming back to the front looks for the repositories of every project.
const ALL_EVERY: std::time::Duration = std::time::Duration::from_secs(30);
/// When it last did.
static LAST_ALL: parking_lot::Mutex<Option<std::time::Instant>> = parking_lot::Mutex::new(None);
/// Held while it does.
static ALL_RUNNING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// What a search of the repositories changed (`Core::refresh_repos`).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub added: Vec<String>,
    pub gone: Vec<String>,
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// Looks for the repositories of the project's folder again (ESC-169): at its opening, when
    /// the window comes back to the front, by « Rechercher à nouveau ». A repository found is told
    /// (unless `quiet`: the first search of a project); one gone is kept as « retiré » until it is
    /// forgotten. A project of one repository stays one; one that was a project of several and
    /// holds none any more keeps them, gone.
    pub async fn refresh_repos(&self, project_id: &str, quiet: bool) -> anyhow::Result<Found> {
        let project = self.project(project_id)?;
        let path = project.path.clone();
        let wt_root = project.worktrees_root();
        let now = detect(Path::new(&path), &wt_root).await;
        let (found, updated) = {
            let mut projects = self.projects.write();
            let Some(cur) = projects.iter_mut().find(|p| p.id == project_id) else {
                return Err(crate::core::project_not_found());
            };
            let before: Vec<String> = cur.repos.clone();
            let mut found = Found::default();
            if now.is_empty() && before.is_empty() {
                return Ok(found);
            }
            found.added = now
                .iter()
                .filter(|r| !before.contains(r))
                .cloned()
                .collect();
            found.gone = before
                .iter()
                .filter(|r| !now.contains(r))
                .cloned()
                .collect();
            if found.added.is_empty() && found.gone.is_empty() {
                return Ok(found);
            }
            for g in &found.gone {
                if !cur.repos_gone.contains(g) {
                    cur.repos_gone.push(g.clone());
                }
            }
            cur.repos_gone.retain(|g| !now.contains(g));
            // Gone all: still a project of several, its repositories gone.
            cur.repos = if now.is_empty() { before } else { now };
            if cur.repos.is_empty() {
                cur.repos_gone.clear();
            }
            (found, cur.clone())
        };
        self.request_save();
        self.hub.emit(crate::model::UiEvent::Project {
            project: updated.clone(),
        });
        if !quiet && !found.added.is_empty() {
            let names = found
                .added
                .iter()
                .map(|r| label(r))
                .collect::<Vec<_>>()
                .join(", ");
            self.hub.emit(crate::model::UiEvent::Toast {
                text: tr_n!(
                    found.added.len(),
                    "Nouveau dépôt git dans {project} : {names}",
                    "Nouveaux dépôts git dans {project} : {names}",
                    "New git repository in {project}: {names}",
                    "New git repositories in {project}: {names}",
                    project = updated.name
                ),
            });
        }
        self.git.refresh(project_id);
        Ok(found)
    }

    /// « Oublier » a repository gone.
    pub fn forget_repo(
        &self,
        project_id: &str,
        repo: &str,
    ) -> anyhow::Result<crate::model::Project> {
        let updated = {
            let mut projects = self.projects.write();
            let cur = projects
                .iter_mut()
                .find(|p| p.id == project_id)
                .ok_or_else(crate::core::project_not_found)?;
            cur.repos_gone.retain(|g| g != repo);
            if !Path::new(&join(&cur.path, repo)).is_dir() {
                cur.repos.retain(|r| r != repo);
            }
            cur.clone()
        };
        self.request_save();
        self.hub.emit(crate::model::UiEvent::Project {
            project: updated.clone(),
        });
        Ok(updated)
    }

    /// Every open project's repositories looked for again (the window back to the front).
    /// At most once every `ALL_EVERY`, never two at a time: the window comes back to the front
    /// often, and each search reads every project's folders.
    pub fn refresh_all_repos(self: &std::sync::Arc<Self>) {
        {
            let mut last = LAST_ALL.lock();
            if last.is_some_and(|at| at.elapsed() < ALL_EVERY) {
                return;
            }
            *last = Some(std::time::Instant::now());
        }
        let ids: Vec<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let core = self.clone();
        tauri::async_runtime::spawn(async move {
            let _one = ALL_RUNNING.lock().await;
            for id in ids {
                let _ = core.refresh_repos(&id, false).await;
            }
        });
    }
}

/// The changes of the checkouts `checkouts`, their paths said from the project's folder (or the
/// agent's): each with its repository. A checkout git cannot read is git's error when `strict`,
/// else left out (its folder gone, say).
pub async fn changes_of(
    checkouts: &[crate::model::Checkout],
    strict: bool,
) -> anyhow::Result<Vec<crate::model::FileChange>> {
    let mut out = Vec::new();
    for c in checkouts {
        if !Path::new(&c.dir).is_dir() && !strict {
            continue;
        }
        match crate::git::file_changes(&c.dir).await {
            Ok(changes) => out.extend(changes.into_iter().map(|mut f| {
                f.path = prefixed(&c.repo, &f.path);
                f.repo = c.repo.clone();
                f
            })),
            Err(e) if strict => return Err(e),
            Err(_) => {}
        }
    }
    Ok(out)
}

/// The paths `paths` (from the project's folder) by repository of `repos`, in their order, each
/// from its repository; those of no repository left out.
pub fn by_repo(repos: &[String], paths: &[String]) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for p in paths {
        let Some((repo, rest)) = split(repos, p) else {
            continue;
        };
        match out.iter_mut().find(|(r, _)| r == repo) {
            Some((_, list)) => list.push(rest.to_string()),
            None => out.push((repo.to_string(), vec![rest.to_string()])),
        }
    }
    out.sort_by_key(|(r, _)| repos.iter().position(|x| x == r));
    out
}

/// A diff of the repository `repo` with its paths said from the project's folder.
pub fn prefix_diff(diff: &str, repo: &str) -> String {
    if repo.is_empty() {
        return diff.to_string();
    }
    let mut out = String::with_capacity(diff.len() + 64);
    for line in diff.split_inclusive('\n') {
        let fixed = if let Some(rest) = line.strip_prefix("diff --git ") {
            // `a/x b/x`: each side gets the repository.
            let rest = rest
                .replacen("a/", &format!("a/{repo}/"), 1)
                .replace(" b/", &format!(" b/{repo}/"));
            format!("diff --git {rest}")
        } else if let Some(rest) = line.strip_prefix("--- a/") {
            format!("--- a/{repo}/{rest}")
        } else if let Some(rest) = line.strip_prefix("+++ b/") {
            format!("+++ b/{repo}/{rest}")
        } else if let Some(rest) = line.strip_prefix("rename from ") {
            format!("rename from {repo}/{rest}")
        } else if let Some(rest) = line.strip_prefix("rename to ") {
            format!("rename to {repo}/{rest}")
        } else {
            line.to_string()
        };
        out.push_str(&fixed);
    }
    out
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// The git state of a project of several repositories (ESC-169): each one's, and their sums.
    pub(crate) async fn multi_git(&self, project: &crate::model::Project) -> crate::model::GitInfo {
        use crate::model::{GitInfo, RepoGit};
        let mut info = GitInfo {
            is_repo: true,
            ..Default::default()
        };
        let tally = |c: char, m: &mut u32, a: &mut u32, d: &mut u32| match c {
            'A' => *a += 1,
            'D' => *d += 1,
            _ => *m += 1,
        };
        let mut entries: Vec<String> = Vec::new();
        for c in project.checkouts() {
            let mut rg = RepoGit {
                repo: c.repo.clone(),
                ..Default::default()
            };
            if !Path::new(&c.dir).is_dir() {
                rg.gone = true;
                info.repos.push(rg);
                continue;
            }
            let (st, remotes, commit) = tokio::join!(
                crate::git::status(&c.dir),
                crate::git::remotes(&c.dir),
                crate::git::has_commit(&c.dir)
            );
            let st = st.unwrap_or_default();
            rg.branch = st.branch.clone();
            rg.upstream = st.upstream.clone();
            rg.ahead = st.ahead;
            rg.behind = st.behind;
            rg.has_remote = !remotes.is_empty();
            rg.has_commit = commit;
            for e in &st.entries {
                tally(e.status, &mut rg.modified, &mut rg.added, &mut rg.deleted);
                entries.push(prefixed(&c.repo, &e.path));
            }
            info.ahead += rg.ahead;
            info.behind += rg.behind;
            info.has_remote |= rg.has_remote;
            info.modified += rg.modified;
            info.added += rg.added;
            info.deleted += rg.deleted;
            info.last_fetch = info.last_fetch.max(crate::git::last_fetch(&c.dir));
            info.repos.push(rg);
        }
        let mut branches = info.repos.iter().filter(|r| !r.gone).map(|r| &r.branch);
        if let Some(first) = branches.next() {
            if branches.all(|b| b == first) {
                info.branch = first.clone();
            }
        }
        for h in self.project_agents(&project.id) {
            let (id, wt, touched) = {
                let rt = h.lock();
                (
                    rt.meta.id.clone(),
                    rt.meta.worktree.clone(),
                    rt.meta.touched_files.clone(),
                )
            };
            let count = match wt {
                Some(wt) => {
                    // Its branch in each repository, for the status bar.
                    let mut repos = Vec::new();
                    for c in wt.checkouts() {
                        if !Path::new(&c.dir).is_dir() {
                            repos.push(RepoGit {
                                repo: c.repo.clone(),
                                gone: true,
                                ..Default::default()
                            });
                            continue;
                        }
                        let st = crate::git::status(&c.dir).await.unwrap_or_default();
                        repos.push(RepoGit {
                            repo: c.repo.clone(),
                            branch: st.branch,
                            upstream: st.upstream,
                            ahead: st.ahead,
                            behind: st.behind,
                            has_remote: info.repos.iter().any(|r| r.repo == c.repo && r.has_remote),
                            has_commit: true,
                            ..Default::default()
                        });
                    }
                    info.agent_git
                        .insert(id.clone(), crate::agentbranch::agent_git_of_repos(repos));
                    let changes = changes_of(&wt.checkouts(), false).await.unwrap_or_default();
                    for f in &changes {
                        let c = f.status.chars().next().unwrap_or('M');
                        tally(c, &mut info.modified, &mut info.added, &mut info.deleted);
                    }
                    changes.len() as u32
                }
                None => touched
                    .iter()
                    .filter(|t| entries.iter().any(|e| e.eq_ignore_ascii_case(t)))
                    .count() as u32,
            };
            info.agents.insert(id, count);
        }
        info.total = info.modified + info.added + info.deleted;
        info
    }
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// `dirty_files` of a project of several repositories: the project's own changes, each
    /// credited to the agent without a worktree that touched it, then the agents' worktrees'.
    pub(crate) async fn multi_dirty_files(
        &self,
        project: &crate::model::Project,
        agent_id: Option<String>,
        strict: bool,
    ) -> anyhow::Result<Vec<crate::model::FileChange>> {
        let agents: Vec<(String, Option<crate::model::Worktree>, Vec<String>)> = self
            .project_agents(&project.id)
            .iter()
            .map(|h| {
                let rt = h.lock();
                (
                    rt.meta.id.clone(),
                    rt.meta.worktree.clone(),
                    rt.meta.touched_files.clone(),
                )
            })
            .filter(|(id, _, _)| agent_id.as_ref().is_none_or(|a| a == id))
            .collect();
        let mut out = Vec::new();
        let main = if agents.iter().all(|(_, wt, _)| wt.is_some()) && agent_id.is_some() {
            Vec::new()
        } else {
            changes_of(&project.checkouts(), false).await?
        };
        for mut f in main {
            f.agent_id = agents
                .iter()
                .find(|(_, wt, touched)| {
                    wt.is_none() && touched.iter().any(|t| t.eq_ignore_ascii_case(&f.path))
                })
                .map(|(id, _, _)| id.clone());
            if agent_id.is_none() || f.agent_id.is_some() {
                out.push(f);
            }
        }
        for (id, wt, _) in &agents {
            if let Some(wt) = wt {
                for mut f in changes_of(&wt.checkouts(), strict).await? {
                    f.agent_id = Some(id.clone());
                    f.in_worktree = true;
                    out.push(f);
                }
            }
        }
        Ok(out)
    }
}

/// The copy's patterns (`worktree_copy`, from the project's folder) as the repository `repo` reads
/// them: those that name it (`api/.env.local` → `.env.local`), and those that name none of `repos`
/// (`.env*`, `**/.env*`), which go for each.
pub fn repo_patterns(repos: &[String], patterns: &[String], repo: &str) -> Vec<String> {
    patterns
        .iter()
        .filter_map(|p| {
            let named = repos
                .iter()
                .filter(|r| !r.is_empty())
                .find(|r| p.starts_with(&format!("{r}/")));
            match named {
                Some(r) if r == repo => Some(p[r.len() + 1..].to_string()),
                Some(_) => None,
                None => Some(p.clone()),
            }
        })
        .collect()
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// The checkouts the files panel reads for `agent_id` in a project of several repositories:
    /// its worktrees, else the project's repositories. None for a project of one.
    pub(crate) fn multi_checkouts(
        &self,
        project: &crate::model::Project,
        agent_id: Option<&str>,
    ) -> anyhow::Result<Option<Vec<crate::model::Checkout>>> {
        if !project.is_multi() {
            return Ok(None);
        }
        let wt = match agent_id {
            Some(a) => self.agent(a)?.lock().meta.worktree.clone(),
            None => None,
        };
        Ok(Some(match wt {
            Some(wt) => wt.checkouts(),
            None => project.checkouts(),
        }))
    }

    /// The diff of `paths` (from the project's folder; all of them when empty) in the checkouts,
    /// each repository's paths said from the project's folder.
    pub(crate) async fn multi_diff(
        checkouts: &[crate::model::Checkout],
        paths: &[String],
    ) -> anyhow::Result<String> {
        let mut out = String::new();
        if paths.is_empty() {
            for c in checkouts {
                if Path::new(&c.dir).is_dir() {
                    out.push_str(&prefix_diff(&crate::git::diff(&c.dir, &[]).await?, &c.repo));
                }
            }
            return Ok(out);
        }
        let repos: Vec<String> = checkouts.iter().map(|c| c.repo.clone()).collect();
        for (repo, rel) in by_repo(&repos, paths) {
            let Some(c) = checkouts.iter().find(|c| c.repo == repo) else {
                continue;
            };
            out.push_str(&prefix_diff(&crate::git::diff(&c.dir, &rel).await?, &repo));
        }
        Ok(out)
    }

    /// The checkout of `path` (from the project's folder) and the path from it.
    pub(crate) fn checkout_of<'a>(
        checkouts: &'a [crate::model::Checkout],
        path: &str,
    ) -> anyhow::Result<(&'a crate::model::Checkout, String)> {
        let repos: Vec<String> = checkouts.iter().map(|c| c.repo.clone()).collect();
        let (repo, rest) = split(&repos, path).ok_or_else(|| {
            anyhow::anyhow!(tr!(
                "« {path} » n’est dans aucun dépôt du projet",
                "“{path}” is in none of the project’s repositories"
            ))
        })?;
        let c = checkouts
            .iter()
            .find(|c| c.repo == repo)
            .expect("split gives one of them");
        Ok((c, rest.to_string()))
    }

    /// A direct commit's scope in a project of several repositories: each checkout's changes, its
    /// paths from the project's folder, the files copied into the worktrees left out.
    pub(crate) async fn multi_commit_scope(
        &self,
        project: &crate::model::Project,
        checkouts: &[crate::model::Checkout],
        in_worktree: bool,
    ) -> anyhow::Result<(crate::model::CommitScope, HashSet<String>)> {
        let repos: Vec<String> = checkouts.iter().map(|c| c.repo.clone()).collect();
        let mut scope = crate::model::CommitScope {
            files: Vec::new(),
            left_out: Vec::new(),
        };
        let mut unread = HashSet::new();
        for c in checkouts {
            if !Path::new(&c.dir).is_dir() {
                continue;
            }
            let patterns = repo_patterns(&repos, &project.worktree_copy, &c.repo);
            let listed = crate::git::file_changes(&c.dir).await?;
            let matching: Vec<String> = listed
                .iter()
                .filter(|f| {
                    patterns
                        .iter()
                        .any(|p| crate::testlaunch::glob_match(p, &f.path))
                })
                .map(|f| f.path.clone())
                .collect();
            let mut copied: HashSet<String> = crate::git::ignored(&c.dir, &matching)
                .await?
                .into_iter()
                .collect();
            if in_worktree {
                let took =
                    crate::testlaunch::matching_ignored(&join(&project.path, &c.repo), &patterns)
                        .await;
                copied.extend(took.into_iter().filter(|f| matching.contains(f)));
            }
            unread.extend(matching.iter().map(|p| prefixed(&c.repo, p)));
            for mut f in listed {
                let full = prefixed(&c.repo, &f.path);
                if copied.contains(&f.path) {
                    scope.left_out.push(full);
                } else {
                    f.path = full;
                    f.repo = c.repo.clone();
                    scope.files.push(f);
                }
            }
        }
        Ok((scope, unread))
    }
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// The folder of the project's repository `repo` (ESC-169): of a project of several, that one
    /// (the first when None); else the project's repository, whatever `repo` says.
    pub(crate) async fn repo_root(
        &self,
        project: &crate::model::Project,
        repo: Option<&str>,
    ) -> anyhow::Result<String> {
        if project.is_multi() {
            let repo = match repo {
                Some(r) if project.repos.iter().any(|x| x == r) => r.to_string(),
                Some(r) => anyhow::bail!(tr!(
                    "« {r} » n’est pas un dépôt du projet",
                    "“{r}” isn’t a repository of the project"
                )),
                None => project.repos[0].clone(),
            };
            let dir = join(&project.path, &repo);
            if !Path::new(&dir).is_dir() {
                return Err(crate::core::not_a_repo());
            }
            // As git names it: the paths compared with it are git's.
            return self
                .toplevel(&dir)
                .await
                .ok_or_else(crate::core::not_a_repo);
        }
        self.toplevel(&project.path)
            .await
            .ok_or_else(crate::core::not_a_repo)
    }
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// The repositories of the project, as git names their roots: each of a project of several
    /// (ESC-169), else the one its folder is in.
    pub(crate) async fn project_roots_of(&self, project_id: &str) -> Vec<String> {
        let Ok(project) = self.project(project_id) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for c in project.checkouts() {
            if let Some(root) = self.toplevel(&c.dir).await {
                if !out.contains(&root) {
                    out.push(root);
                }
            }
        }
        out
    }
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// The editor's file `path` (from `root`, the project's folder or an agent's) in a project of
    /// several repositories (ESC-169): its repository's folder, the base its agent's worktree of
    /// it left, and the path from it. As it is for a project of one, or a file of no repository.
    pub(crate) fn edit_route(
        &self,
        project_id: &str,
        agent_id: Option<&str>,
        root: String,
        base: Option<String>,
        path: String,
    ) -> (String, Option<String>, String) {
        let Ok(project) = self.project(project_id) else {
            return (root, base, path);
        };
        let Some((repo, rest)) = split(&project.repos, &path) else {
            return (root, base, path);
        };
        let wt = agent_id
            .and_then(|a| self.agent(a).ok())
            .and_then(|h| h.lock().meta.worktree.clone());
        let base = wt
            .and_then(|w| w.repos.into_iter().find(|r| r.repo == repo))
            .map(|r| r.base_branch);
        (join(&root, repo), base, rest.to_string())
    }
}
