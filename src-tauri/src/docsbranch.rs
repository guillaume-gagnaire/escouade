//! Where the docs of a project live (ESC-256): the branch of the docs, set per project
//! (`Project::docs_branch`).
//!
//! - **The current branch** (empty): the branch checked out where the caller works. An agent
//!   writes in its own folder, so a ticket's spec changes go in its branch and merge with its
//!   code; the window, and a client outside Escouade, work on the project's folder.
//! - **A fixed branch**: its own worktree, made and held by Escouade under the project's folder in
//!   Escouade's data (`DataDir::project_files`), or the folder where git already has that branch
//!   checked out. Escouade writes the files there and does nothing else with the branch: it never
//!   merges it anywhere (the team does, or the branch is `main`), and never commits nor pushes.
//!   A ticket's agent works on a copy of it, poured on the branch when its ticket is validated
//!   (`docsticket`, ESC-263).
//!
//! The text extracted from the documents is kept in the data folder too (`Space::text_dir`).

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use tauri::Runtime;

use crate::core::Core;
use crate::docs::{Ref, Register, Space};
use crate::i18n::Lang;
use crate::model::Project;

/// One worktree is made at a time: two tools asking at once must not both make it.
static MAKING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The folder that holds a project's `.escouade/` for the files shared with the team: its
/// folder, or its first repository's when it has several (as the shared tickets, ESC-169).
pub fn base_of(project: &Project) -> PathBuf {
    let root = Path::new(&project.path);
    match project.repos.first() {
        Some(first) if !first.is_empty() => root.join(first),
        _ => root.to_path_buf(),
    }
}

/// Why `name` cannot be the docs branch of a project, if it cannot (git's own rules, checked once
/// more when the worktree is made).
pub fn branch_problem(name: &str) -> Option<String> {
    let bad = name.starts_with('-')
        || name.starts_with('/')
        || name.ends_with('/')
        || name.ends_with('.')
        || name.ends_with(".lock")
        || name.contains("..")
        || name.contains("//")
        || name.contains("@{")
        || name == "@"
        || name
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "~^:?*[\\".contains(c));
    bad.then(|| {
        tr!(
            "« {name} » n’est pas un nom de branche valide.",
            "“{name}” is not a valid branch name."
        )
    })
}

impl<R: Runtime> Core<R> {
    /// Where the extracted text of a project's documents is kept.
    pub(crate) fn docs_text_dir(&self, project_id: &str) -> PathBuf {
        self.data.project_files(project_id).join("docs-text")
    }

    /// What an agent launched on `project` is told when its spec is in Escouade (ESC-278): None otherwise.
    pub(crate) async fn docs_follow(&self, project: &Project, lang: Lang) -> Option<String> {
        self.docs_space(&project.id, None)
            .await
            .ok()?
            .follow_rule(lang)
    }

    /// Where the fixed branch's worktree is made.
    fn docs_worktrees(&self, project_id: &str) -> PathBuf {
        self.data.project_files(project_id).join("docs")
    }

    /// The docs of `project` as `agent` (the id of an agent of Escouade, or None for the window and
    /// Claude outside Escouade) sees them. See the module's comment.
    pub async fn docs_space(&self, project_id: &str, agent: Option<&str>) -> Result<Space> {
        let mut space = self.docs_space_of(project_id, agent).await?;
        space.facts = Some((self.facts.emitter(), project_id.to_string()));
        space.checks = Some(self.docs_checks_file(project_id));
        Ok(space)
    }

    /// Where what the comparison to the code found of the project's pages is kept (ESC-301): in
    /// Escouade's folder, whatever the branch, so that git never sees it.
    pub(crate) fn docs_checks_file(&self, project_id: &str) -> PathBuf {
        self.data.project_files(project_id).join("docs-checks.json")
    }

    async fn docs_space_of(&self, project_id: &str, agent: Option<&str>) -> Result<Space> {
        let project = self.project(project_id)?;
        let text = Some(self.docs_text_dir(&project.id));
        let branch = project.docs_branch.trim().to_string();
        if branch.is_empty() {
            return Ok(Space::new(self.docs_current(&project, agent), text));
        }
        let fixed = Space::new(self.docs_worktree(&project, &branch).await?, text);
        // A ticket's agent works on a copy, poured on the branch when its ticket is validated.
        match agent.and_then(|a| self.ticket_key_of_agent(&project.id, a)) {
            Some(key) => self.docs_ticket_space(&project.id, &key, fixed),
            None => Ok(fixed),
        }
    }

    /// The folder the branch checked out where the caller works holds: the agent's own (a worktree
    /// of its ticket) when it works in this project, else the project's.
    fn docs_current(&self, project: &Project, agent: Option<&str>) -> PathBuf {
        let own = agent
            .and_then(|id| self.agent(id).ok())
            .map(|h| {
                let m = h.lock().meta.clone();
                (m.project_id, m.cwd)
            })
            .filter(|(pid, cwd)| *pid == project.id && !cwd.is_empty() && Path::new(cwd).is_dir())
            .map(|(_, cwd)| PathBuf::from(cwd));
        let Some(cwd) = own else {
            return base_of(project);
        };
        // A project of several repositories: the first one's, as its folder.
        match project.repos.first() {
            Some(first) if !first.is_empty() && cwd.join(first).is_dir() => cwd.join(first),
            _ => cwd,
        }
    }

    /// The worktree of the fixed branch `branch`: where git already has it checked out, else one of
    /// Escouade's own, the branch made first (from the remote's when it has one, else from HEAD)
    /// when there is none.
    async fn docs_worktree(&self, project: &Project, branch: &str) -> Result<PathBuf> {
        let _one = MAKING.lock().await;
        let repo = base_of(project).to_string_lossy().to_string();
        crate::git::check_ref_format(branch).await?;
        if let Some(at) = crate::git::checkout_of(&repo, branch).await? {
            if Path::new(&at).is_dir() {
                return Ok(PathBuf::from(at));
            }
            // Its folder is gone: git forgets it, then it is made again.
            let _ = crate::git::run(&repo, &["worktree", "prune"]).await;
        }
        if !crate::git::has_commit(&repo).await {
            bail!(tr!(
                "Le dépôt n’a pas encore de commit : la branche des docs « {branch} » ne peut pas être créée.",
                "The repository has no commit yet: the docs branch “{branch}” can’t be made."
            ));
        }
        if !crate::git::branch_exists(&repo, branch).await {
            let remote = format!("origin/{branch}");
            if crate::git::ref_exists(&repo, &format!("refs/remotes/{remote}")).await {
                crate::git::branch_track(&repo, &remote).await?;
            } else {
                crate::git::branch_create(&repo, branch, "HEAD").await?;
            }
        }
        let root = self.docs_worktrees(&project.id);
        std::fs::create_dir_all(&root)?;
        let (path, _) = crate::git::worktree_add_existing(&repo, &root, &[], branch).await?;
        Ok(PathBuf::from(path))
    }

    /// « Accepter » a proposed decision: the window's alone (no tool does it). The name carried
    /// is this machine's user's in the project.
    pub async fn docs_accept_decision(
        &self,
        project_id: &str,
        reference: &Ref,
    ) -> Result<crate::docs::Decision> {
        let project = self.project(project_id)?;
        let by = self.me_in(&project).name;
        let space = self.docs_space(project_id, None).await?;
        let was = space.decision(reference)?;
        let d = space.decision_accept(reference, &by, &Register::default(), None)?;
        self.docs_numbered(project_id, &was, &d);
        Ok(d)
    }

    /// The ticket `key` validated: the proposals its agent left (origin `ticket`, its key) are
    /// accepted, in the name of the one who validates, where that agent wrote them (its worktree,
    /// committed and merged with the ticket, or the fixed branch's). Each takes its number then
    /// (ESC-303), after the register of where it goes: the branch `target` it is merged into, or
    /// the fixed branch; so that two tickets that proposed one each at the same time get two.
    /// The numbers accepted.
    pub async fn docs_accept_ticket_decisions(
        &self,
        project_id: &str,
        agent_id: &str,
        key: &str,
        target: &str,
    ) -> Result<Vec<u32>> {
        let project = self.project(project_id)?;
        let space = self.docs_space(project_id, Some(agent_id)).await?;
        let mine: Vec<crate::docs::Decision> = space
            .decisions()
            .into_iter()
            .filter(|d| {
                d.state == crate::docs::State::Proposed
                    && d.origin == "ticket"
                    && d.origin_ref.eq_ignore_ascii_case(key)
            })
            .collect();
        if mine.is_empty() {
            return Ok(Vec::new());
        }
        // Where it goes: the docs of the project as they are (the fixed branch, or the project's
        // folder), and the branch it is merged into as git has it.
        let mut on = self.docs_space(project_id, None).await?.register();
        if project.docs_branch.trim().is_empty() && !target.is_empty() {
            on = on.and(register_on_branch(&space.root, target).await);
        }
        let by = self.me_in(&project).name;
        let mut done = Vec::new();
        for was in mine {
            // By its own id, never by a number another decision may have too.
            let own = Ref::Id(was.id.clone());
            let d = space.decision_accept(&own, &by, &on, Some(key))?;
            if let Some(n) = d.number {
                on.next = on.next.max(n + 1);
            }
            self.docs_numbered(project_id, &was, &d);
            if let Some(n) = d.number {
                done.push(n);
            }
        }
        Ok(done)
    }

    /// The proposal `was` accepted as `now`: the tickets of the board and what is to plan that
    /// cite it by its id cite its number then (by its id alone: a number may be another's).
    pub(crate) fn docs_numbered(
        &self,
        project_id: &str,
        was: &crate::docs::Decision,
        now: &crate::docs::Decision,
    ) {
        let (old, Some(n)) = (Ref::Id(was.id.clone()), now.number) else {
            return;
        };
        let new = Ref::Number(n);
        if old == new {
            return;
        }
        let swap = |list: &mut Vec<Ref>| {
            if !list.contains(&old) {
                return false;
            }
            let mut out: Vec<Ref> = Vec::new();
            for r in list.iter() {
                let r = if *r == old { new.clone() } else { r.clone() };
                if !out.contains(&r) {
                    out.push(r);
                }
            }
            *list = out;
            true
        };
        let ids: Vec<String> = self
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == project_id && t.decisions.contains(&old))
            .map(|t| t.id.clone())
            .collect();
        for id in ids {
            if let Err(e) = self.edit_ticket(&id, |t| {
                swap(&mut t.decisions);
                Ok(())
            }) {
                log::warn!("ticket {id}: decision {old} not renumbered: {e:#}");
            }
        }
        self.planning_edit(project_id, |p| {
            swap(&mut p.decisions);
        });
    }
}

/// The register on `branch`, as git has it in the repository of `dir`, from its files' names:
/// empty when it has none (or there is no such branch).
async fn register_on_branch(dir: &Path, branch: &str) -> Register {
    let dir = dir.to_string_lossy().to_string();
    let path = format!("{}/{}/", crate::docs::ROOT, crate::docs::DECISIONS);
    let Ok(out) = crate::git::run(&dir, &["ls-tree", "--name-only", branch, "--", &path]).await
    else {
        return Register::default();
    };
    let out = String::from_utf8_lossy(&out);
    Register::of_names(out.lines().filter_map(|l| l.rsplit('/').next()))
}
