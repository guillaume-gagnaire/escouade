//! « Projet privé », its files (ESC-140): the paths an agent's harness is told to refuse it.
//!
//! The MCP server keeps a private project out of sight (`mcp::resolve::Sight`); its files are kept
//! out by the rules each harness takes at its start (`Spawn::fence`):
//! - an agent of a project that is not private: the folders of the private projects (their own,
//!   their worktrees', those of their agents), their data in Escouade's folder, and their sessions
//!   in the harnesses' folders;
//! - an agent of a private project: those of every other project, the same;
//! - both: the rest of Escouade's folder (`state.json` holds every project) but the folders of the
//!   files joined to messages and to tickets, and the sessions a harness keeps mixed, whatever
//!   their project (Codex's are by date).
//!
//! With no private project, nothing is refused. The paths are those of this machine: each harness
//! writes them its way (`Engine::fence_settings`, Codex's profiles, OpenCode's rules). What is on
//! the disk is read at the agent's start: a folder made after (an agent made since) is the next
//! start's.

use std::path::{Path, PathBuf};

/// The paths refused, each with everything under it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fence {
    paths: Vec<PathBuf>,
}

impl Fence {
    /// `paths`, each once, those under another of them left out (a worktree under its project's
    /// folder): refused with it.
    pub fn new(paths: Vec<PathBuf>) -> Self {
        let mut keyed: Vec<(String, PathBuf)> = paths
            .into_iter()
            .map(|p| (key_of(&p), p))
            .filter(|(k, _)| !k.is_empty())
            .collect();
        // The shortest first: a path above another comes before it.
        keyed.sort_by_key(|(k, _)| k.len());
        let mut kept: Vec<(String, PathBuf)> = Vec::new();
        for (key, p) in keyed {
            if !kept.iter().any(|(root, _)| under(&key, root)) {
                kept.push((key, p));
            }
        }
        Fence {
            paths: kept.into_iter().map(|(_, p)| p).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Whether `path` is one of the paths refused or under one, whatever its spelling (case under
    /// Windows, separators).
    pub fn covers(&self, path: &str) -> bool {
        let key = crate::paths::folder_key(path);
        self.paths.iter().any(|p| under(&key, &key_of(p)))
    }
}

/// What an agent's fence follows of the projects' privacy (« Projet privé »): whether its project
/// is private, and which projects are. Its process starts again before its next turn when this
/// changed since it started (`Core::refence`); a path made since (an agent, a ticket, a file
/// joined, a session of another project) waits for its next start of its own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Privacy {
    pub own: bool,
    /// The ids of the private projects, sorted.
    pub private: Vec<String>,
}

fn key_of(p: &Path) -> String {
    crate::paths::folder_key(&p.to_string_lossy())
}

/// Whether the key `path` is the key `root` or under it.
fn under(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/') || root.ends_with('/'))
}

/// A project, as its fence reads it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Side {
    pub id: String,
    pub private: bool,
    /// Its folder, its worktrees' folder, and its agents' folders (worktrees made elsewhere).
    pub folders: Vec<PathBuf>,
    /// Its data in Escouade's folder: the folders of its agents' joined files and of its tickets.
    pub data: Vec<PathBuf>,
}

/// The harness a folder of sessions is of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeKind {
    /// A configuration folder of Claude Code (`~/.claude`, an account's `CLAUDE_CONFIG_DIR`).
    Claude,
    /// `CODEX_HOME`.
    Codex,
    /// `COPILOT_HOME`.
    Copilot,
    /// OpenCode's data folder (`~/.local/share/opencode`).
    Opencode,
}

/// A folder where a harness keeps its sessions.
#[derive(Debug, Clone)]
pub(crate) struct Home {
    pub kind: HomeKind,
    pub dir: PathBuf,
}

/// What an agent's fence is made from.
#[derive(Debug, Clone, Default)]
pub(crate) struct Ground {
    /// The agent's project.
    pub own: String,
    pub projects: Vec<Side>,
    /// Escouade's data folder.
    pub data_root: PathBuf,
    /// The agent's own folders, never refused: its folder, and those of its files in Escouade's.
    pub keep: Vec<PathBuf>,
    pub homes: Vec<Home>,
    /// Files of a harness's that hold every project's sessions or folders, beside its homes
    /// (`~/.claude.json`).
    pub files: Vec<PathBuf>,
}

/// The folders of Escouade's data an agent still reads: those of the files joined to messages
/// (`DataDir::attachments`) and to tickets (`DataDir::tickets`), each owner's apart.
const DATA_KEPT: [&str; 2] = ["attachments", "tickets"];

/// What Escouade's folder holds of every project (`DataDir`), refused whether it is there yet or
/// not (`state.json` is written after an agent starts): the rest of the folder, as it is at the
/// agent's start, too.
const DATA_ALL: [&str; 21] = [
    "state.json",
    "settings.json",
    "integrations.json",
    "sync-queue.json",
    "mcp-token.json",
    "mcp-refused.json",
    "mcp",
    "fence",
    "tui",
    "conversations",
    "closed-projects",
    "logs",
    "app.log",
    "stats.db",
    "stats.db-wal",
    "stats.db-shm",
    "claude",
    "codex",
    "copilot",
    "opencode",
    "voice",
];

/// What a harness keeps of every project's sessions mixed: refused whenever a fence is up.
fn mixed(kind: HomeKind) -> &'static [&'static str] {
    match kind {
        HomeKind::Claude => &[
            "history.jsonl",
            "file-history",
            "todos",
            "session-env",
            "shell-snapshots",
            "paste-cache",
            "plans",
            "tasks",
        ],
        HomeKind::Codex => &[
            "sessions",
            "archived_sessions",
            "history.jsonl",
            "session_index.jsonl",
            "memories",
        ],
        HomeKind::Copilot => &[
            "session-state",
            "history-session-state",
            "logs",
            "command-history-state.json",
        ],
        HomeKind::Opencode => &["storage", "log", "snapshot"],
    }
}

/// Files a harness keeps mixed, by their extension: Codex's and OpenCode's databases.
fn mixed_db(kind: HomeKind, name: &str) -> bool {
    match kind {
        HomeKind::Codex => name.contains(".sqlite"),
        HomeKind::Opencode | HomeKind::Copilot => name.contains(".db"),
        HomeKind::Claude => false,
    }
}

/// The name Claude Code gives the folder of a folder's sessions (`projects/<name>`): every
/// character but a letter or a digit a `-`.
pub(crate) fn claude_project_name(folder: &Path) -> String {
    folder
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The names of what is in `dir` (nothing when it cannot be read).
fn entries(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The fence of an agent of `g.own`.
pub(crate) fn compute(g: &Ground) -> Fence {
    let own_private = g.projects.iter().any(|p| p.id == g.own && p.private);
    let walled: Vec<&Side> = g
        .projects
        .iter()
        .filter(|p| p.id != g.own && (own_private || p.private))
        .collect();
    if !own_private && walled.is_empty() {
        return Fence::default();
    }
    let own = g.projects.iter().find(|p| p.id == g.own);
    let mut paths: Vec<PathBuf> = Vec::new();
    for side in &walled {
        paths.extend(side.folders.iter().cloned());
        paths.extend(side.data.iter().cloned());
    }
    // Escouade's folder: every project's state, conversations, logs and tokens, but the folders of
    // the joined files, each owner's apart.
    for name in DATA_ALL
        .iter()
        .map(|n| n.to_string())
        .chain(entries(&g.data_root))
    {
        if !DATA_KEPT.contains(&name.as_str()) {
            paths.push(g.data_root.join(name));
        }
    }
    if own_private {
        // Of the joined files, its project's alone: those of owners made since are the next start's.
        let mine: Vec<String> = own
            .map(|o| o.data.iter().chain(&g.keep).map(|p| key_of(p)).collect())
            .unwrap_or_default();
        for kept in DATA_KEPT {
            let dir = g.data_root.join(kept);
            for name in entries(&dir) {
                let entry = dir.join(name);
                let key = key_of(&entry);
                if !mine.iter().any(|m| under(m, &key)) {
                    paths.push(entry);
                }
            }
        }
    }
    // The harnesses' sessions.
    let own_names: Vec<String> = own
        .map(|o| o.folders.iter().map(|f| claude_project_name(f)).collect())
        .unwrap_or_default();
    let of_own = |name: &str| {
        own_names
            .iter()
            .any(|o| name == o || name.starts_with(&format!("{o}-")))
    };
    for home in &g.homes {
        for name in mixed(home.kind) {
            paths.push(home.dir.join(name));
        }
        for name in entries(&home.dir) {
            if mixed_db(home.kind, &name) {
                paths.push(home.dir.join(&name));
            }
        }
        if home.kind != HomeKind::Claude {
            continue;
        }
        let sessions = home.dir.join("projects");
        let on_disk = entries(&sessions);
        for side in &walled {
            for folder in &side.folders {
                let name = claude_project_name(folder);
                paths.push(sessions.join(&name));
                // Its worktrees under it: `<name>--claude-worktrees-<agent>`.
                let prefix = format!("{name}-");
                for n in on_disk.iter().filter(|n| n.starts_with(&prefix)) {
                    if !of_own(n) {
                        paths.push(sessions.join(n));
                    }
                }
            }
        }
        if own_private {
            for n in on_disk.iter().filter(|n| !of_own(n)) {
                paths.push(sessions.join(n));
            }
        }
    }
    paths.extend(g.files.iter().cloned());
    // Never what the agent works in, nor a folder above it (a project inside another's folder):
    // refused, it could not work.
    let keep: Vec<String> = g
        .keep
        .iter()
        .chain(own.map(|o| o.folders.iter()).into_iter().flatten())
        .map(|p| key_of(p))
        .collect();
    paths.retain(|p| {
        let key = key_of(p);
        !keep.iter().any(|k| under(k, &key))
    });
    Fence::new(paths)
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// What the files of project `id` keep out of its index: the fence of its own agents, as the
    /// projects are now (`compute`, with no agent). Nothing without a private project.
    pub(crate) fn project_fence(&self, id: &str) -> Fence {
        let projects = self.projects.read().clone();
        if !projects.iter().any(|p| p.private) {
            return Fence::default();
        }
        let sides = projects
            .iter()
            .map(|p| Side {
                id: p.id.clone(),
                private: p.private,
                folders: vec![PathBuf::from(&p.path), p.worktrees_root()],
                data: Vec::new(),
            })
            .collect();
        let settings = self.settings.read().clone();
        let (homes, files) = homes(&settings);
        compute(&Ground {
            own: id.to_string(),
            projects: sides,
            data_root: self.data.root().to_path_buf(),
            keep: Vec::new(),
            homes,
            files,
        })
    }

    /// The fence of the agent `id` as the projects are now (`compute`); none for an agent the
    /// core does not have.
    pub(crate) fn agent_fence(&self, id: &str) -> Fence {
        let Ok(h) = self.agent(id) else {
            return Fence::default();
        };
        let (own, cwd, ticket) = {
            let m = &h.lock().meta;
            (m.project_id.clone(), m.cwd.clone(), m.ticket_id.clone())
        };
        let projects = self.projects.read().clone();
        if !projects.iter().any(|p| p.private) {
            return Fence::default();
        }
        // Read apart: an agent's lock is never held with the list's.
        let handles: Vec<crate::agent::AgentHandle> =
            self.agents.read().values().cloned().collect();
        let agents: Vec<(String, String, String)> = handles
            .iter()
            .map(|h| {
                let m = &h.lock().meta;
                (m.id.clone(), m.project_id.clone(), m.cwd.clone())
            })
            .collect();
        let tickets: Vec<(String, String)> = self
            .tickets
            .read()
            .iter()
            .map(|t| (t.id.clone(), t.project_id.clone()))
            .collect();
        let sides = projects
            .iter()
            .map(|p| {
                let mut folders = vec![PathBuf::from(&p.path), p.worktrees_root()];
                let mut data = Vec::new();
                for (agent, project, cwd) in &agents {
                    if *project == p.id {
                        folders.push(PathBuf::from(cwd));
                        if crate::paths::is_agent_folder_id(agent) {
                            data.push(self.data.agent_files(agent));
                        }
                    }
                }
                for (ticket, project) in &tickets {
                    if *project == p.id {
                        data.push(self.data.ticket_dir(ticket));
                    }
                }
                Side {
                    id: p.id.clone(),
                    private: p.private,
                    folders,
                    data,
                }
            })
            .collect();
        let mut keep = vec![PathBuf::from(&cwd), self.data.agent_files(id)];
        if let Some(t) = &ticket {
            keep.push(self.data.ticket_dir(t));
        }
        let settings = self.settings.read().clone();
        let (homes, files) = homes(&settings);
        compute(&Ground {
            own,
            projects: sides,
            data_root: self.data.root().to_path_buf(),
            keep,
            homes,
            files,
        })
    }
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// What the agent `id` is fenced by of the projects' privacy, now (`Privacy`).
    pub(crate) fn agent_privacy(&self, id: &str) -> Privacy {
        let own = self
            .agent(id)
            .ok()
            .map(|h| h.lock().meta.project_id.clone())
            .unwrap_or_default();
        let projects = self.projects.read();
        let mut private: Vec<String> = projects
            .iter()
            .filter(|p| p.private)
            .map(|p| p.id.clone())
            .collect();
        private.sort();
        Privacy {
            own: private.contains(&own),
            private,
        }
    }

    /// Whether the live process of the agent `id` (`h`) was stopped to start again with its fence:
    /// a project made private, or no longer, since it started (`Privacy`), not a path made since. Only between its turns, as the idle
    /// stop does it (never one that must stay reachable, or whose process holds work): its session
    /// is kept, the next start resumes it. An agent at work keeps its fence until its turn is over.
    pub(crate) async fn refence(
        &self,
        h: &crate::agent::AgentHandle,
        id: &str,
    ) -> anyhow::Result<bool> {
        let now = self.agent_privacy(id);
        let stopped = {
            let mut rt = h.lock();
            let idle = rt.proc.is_some()
                && !rt.meta.remote_control
                && !rt.meta.status.is_active()
                && rt.setup.is_none()
                && !rt
                    .meta
                    .plan
                    .as_ref()
                    .is_some_and(|p| p.holds_process(crate::model::now_ms()));
            if !idle || rt.privacy == now {
                return Ok(false);
            }
            rt.detach()
        };
        let Some(p) = stopped else {
            return Ok(false);
        };
        log::info!("agent {id}: its fence changed, its process starts again");
        self.emit_agent(h);
        if !crate::accounts::ended(&p).await {
            anyhow::bail!(tr!(
                "Le process de l’agent ne s’arrête pas.",
                "The agent’s process won’t stop."
            ));
        }
        Ok(true)
    }

    /// Writes `settings` (`Engine::fence_settings`) as the agent `id`'s, and gives its path.
    pub(crate) fn write_fence_settings(
        &self,
        id: &str,
        settings: &serde_json::Value,
    ) -> anyhow::Result<PathBuf> {
        use anyhow::Context;
        let file = self.data.fence_settings(id);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        crate::paths::write_atomic(&file, &serde_json::to_vec_pretty(settings)?)
            .with_context(|| format!("{} not written", file.display()))?;
        Ok(file)
    }
}

/// The folders where the harnesses keep their sessions, of every account and of the user's own
/// (`~/.claude`, `~/.codex`…), and Claude Code's `.claude.json` of each.
fn homes(settings: &crate::model::Settings) -> (Vec<Home>, Vec<PathBuf>) {
    let principals: Vec<_> = crate::engine::registry::all()
        .into_iter()
        .filter(|e| e.principal_sessions())
        .map(|e| crate::accounts::principal_of(e.id()))
        .collect();
    let mut homes = Vec::new();
    let mut files = Vec::new();
    for a in settings.accounts.iter().chain(principals.iter()) {
        // OpenCode's are in its data folder, not its configuration's (below).
        let Some(kind) = crate::engine::by_id(a.engine).session_home() else {
            continue;
        };
        homes.push(Home {
            kind,
            dir: crate::accounts::config_dir(a, settings),
        });
        if kind == HomeKind::Claude {
            files.push(crate::accounts::claude_json(a, settings));
        }
    }
    homes.push(Home {
        kind: HomeKind::Opencode,
        dir: opencode_data(),
    });
    (homes, files)
}

/// OpenCode's data folder: under `XDG_DATA_HOME` when the app has one (never in the tests), else
/// `~/.local/share/opencode`. An account of its own has its folders in Escouade's, refused whole.
fn opencode_data() -> PathBuf {
    let xdg = (!cfg!(test))
        .then(|| std::env::var_os("XDG_DATA_HOME"))
        .flatten()
        .filter(|v| !v.is_empty());
    match xdg {
        Some(v) => PathBuf::from(v).join("opencode"),
        None => crate::accounts::app_env().1.join(".local/share/opencode"),
    }
}

/// Whether `text` (a path, a command) names `path` or a path under it, whatever its separators
/// and, under Windows, its case. A guard: a longer name that starts alike counts too.
pub(crate) fn names(text: &str, path: &str) -> bool {
    let norm = |s: &str| {
        let s = s.replace('\\', "/");
        let s = s.trim_end_matches('/').to_string();
        if cfg!(windows) {
            s.to_lowercase()
        } else {
            s
        }
    };
    let p = norm(path);
    !p.is_empty() && norm(text).contains(&p)
}

/// The paths of `fence` as `program` names them (its Linux ones for a harness of WSL).
pub(crate) fn spelled(fence: &Fence, program: &crate::engine::Program) -> Vec<String> {
    fence.paths().iter().map(|p| program.file_path(p)).collect()
}

/// `path`, as a harness spells it, in a rule of Claude Code's `Read(…)` and `Edit(…)`: `//`
/// then the path from the root, a Windows one in the form Claude Code matches (`C:\x` is
/// `/c/x`), its characters of a pattern taken literally.
pub(crate) fn claude_rule_path(path: &str) -> String {
    // A path of Windows (a POSIX one starts with `/`, and its `\` is a letter of a name).
    let slashed = if path.starts_with('/') {
        path.to_string()
    } else {
        path.replace('\\', "/")
    };
    let bytes = slashed.as_bytes();
    let posix = if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        format!(
            "/{}{}",
            (bytes[0] as char).to_ascii_lowercase(),
            &slashed[2..]
        )
    } else {
        slashed
    };
    let mut out = String::from("/");
    for c in posix.trim_end_matches('/').chars() {
        if matches!(c, '*' | '?' | '[' | ']' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// What Claude Code is told of `fence` in its settings: `permissions.deny` for its tools that read
/// (`Read`: Read, Grep, Glob, and the commands of its Bash that it reads the paths of) and write
/// (`Edit`: Edit, Write), and, where its sandbox runs (macOS, Linux, WSL2; not native Windows),
/// `sandbox.filesystem`: what holds for every command once the user turned it on.
pub(crate) fn claude_settings(
    fence: &Fence,
    program: &crate::engine::Program,
) -> serde_json::Value {
    let paths = spelled(fence, program);
    let deny: Vec<String> = paths
        .iter()
        .flat_map(|p| {
            let r = claude_rule_path(p);
            [
                format!("Read({r})"),
                format!("Read({r}/**)"),
                format!("Edit({r})"),
                format!("Edit({r}/**)"),
            ]
        })
        .collect();
    let mut v = serde_json::json!({ "permissions": { "deny": deny } });
    let sandboxed = !cfg!(windows) || program.wsl().is_some();
    if sandboxed {
        v["sandbox"] =
            serde_json::json!({ "filesystem": { "denyRead": paths, "denyWrite": paths } });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;

    fn side(id: &str, private: bool, root: &Path) -> Side {
        Side {
            id: id.into(),
            private,
            folders: vec![root.join(id), root.join(format!("{id}-worktrees"))],
            data: vec![
                root.join("data/attachments").join(format!("ag-{id}")),
                root.join("data/tickets").join(format!("tk-{id}")),
            ],
        }
    }

    fn ground(root: &Path, own: &str, private: &[&str]) -> Ground {
        let data = root.join("data");
        for d in [
            "attachments/ag-a",
            "attachments/ag-b",
            "attachments/_drafts",
            "tickets/tk-b",
            "mcp",
            "conversations",
        ] {
            std::fs::create_dir_all(data.join(d)).unwrap();
        }
        std::fs::write(data.join("state.json"), "{}").unwrap();
        let claude = root.join("home/.claude");
        for d in [
            "projects/x-a",
            "projects/x-a--claude-worktrees-1",
            "projects/x-b",
            "projects/x-b--claude-worktrees-2",
            "projects/x-elsewhere",
        ] {
            std::fs::create_dir_all(claude.join(d)).unwrap();
        }
        let codex = root.join("home/.codex");
        std::fs::create_dir_all(codex.join("sessions")).unwrap();
        std::fs::write(codex.join("state_5.sqlite"), "").unwrap();
        std::fs::write(codex.join("config.toml"), "").unwrap();
        let mut projects = vec![side("a", false, root), side("b", false, root)];
        for p in &mut projects {
            p.private = private.contains(&p.id.as_str());
            // Folders as Claude Code names them in this test's `projects`.
            p.folders = vec![PathBuf::from(format!("x/{}", p.id))];
            p.folders.push(root.join(&p.id));
        }
        Ground {
            own: own.into(),
            projects,
            data_root: data.clone(),
            keep: vec![data.join("attachments").join(format!("ag-{own}"))],
            homes: vec![
                Home {
                    kind: HomeKind::Claude,
                    dir: claude,
                },
                Home {
                    kind: HomeKind::Codex,
                    dir: codex,
                },
            ],
            files: vec![root.join("home/.claude.json")],
        }
    }

    fn has(f: &Fence, p: &Path) -> bool {
        f.covers(&p.to_string_lossy())
    }

    #[test]
    fn without_a_private_project_nothing_is_refused() {
        let root = test_dir("fence-none");
        assert!(compute(&ground(&root, "a", &[])).is_empty());
    }

    #[test]
    fn the_agents_of_the_other_projects_are_refused_a_private_one() {
        let root = test_dir("fence-shared");
        let f = compute(&ground(&root, "a", &["b"]));
        let data = root.join("data");
        let claude = root.join("home/.claude");
        // Its folders, its data, its sessions.
        assert!(has(&f, &root.join("b").join("src/main.rs")));
        assert!(has(&f, &data.join("attachments/ag-b/x.png")));
        assert!(has(&f, &data.join("tickets/tk-b/files/a.pdf")));
        assert!(has(&f, &claude.join("projects/x-b/s.jsonl")));
        assert!(has(&f, &claude.join("projects/x-b--claude-worktrees-2")));
        // Escouade's state, which holds every project, and the sessions kept mixed.
        assert!(has(&f, &data.join("state.json")));
        assert!(has(&f, &data.join("conversations/c.jsonl")));
        assert!(has(&f, &data.join("mcp")));
        assert!(has(&f, &claude.join("history.jsonl")));
        assert!(has(&f, &root.join("home/.codex/sessions/2026/10/r.jsonl")));
        assert!(has(&f, &root.join("home/.codex/state_5.sqlite")));
        assert!(has(&f, &root.join("home/.claude.json")));
        // Its own, and what is no project's.
        assert!(!has(&f, &root.join("a/src/main.rs")));
        assert!(!has(&f, &data.join("attachments/ag-a/x.png")));
        assert!(!has(&f, &claude.join("projects/x-a")));
        assert!(!has(&f, &claude.join("projects/x-a--claude-worktrees-1")));
        assert!(!has(&f, &claude.join("projects/x-elsewhere")));
        assert!(!has(&f, &root.join("home/.codex/config.toml")));
        assert!(!has(&f, &claude.join("settings.json")));
    }

    #[test]
    fn the_agents_of_a_private_project_are_refused_the_others() {
        let root = test_dir("fence-own");
        let f = compute(&ground(&root, "b", &["b"]));
        let data = root.join("data");
        let claude = root.join("home/.claude");
        assert!(has(&f, &root.join("a/src/main.rs")));
        assert!(has(&f, &data.join("attachments/ag-a")));
        // Another owner's joined files, the drafts of the tickets' forms included.
        assert!(has(&f, &data.join("attachments/_drafts/x")));
        assert!(has(&f, &data.join("state.json")));
        assert!(has(&f, &claude.join("projects/x-a")));
        assert!(has(&f, &claude.join("projects/x-a--claude-worktrees-1")));
        // A session of a folder no project's: someone else's.
        assert!(has(&f, &claude.join("projects/x-elsewhere")));
        // Its own.
        assert!(!has(&f, &root.join("b/src")));
        assert!(!has(&f, &data.join("tickets/tk-b/files/a.pdf")));
        assert!(!has(&f, &claude.join("projects/x-b")));
        assert!(!has(&f, &claude.join("projects/x-b--claude-worktrees-2")));
        assert!(!has(&f, &data.join("attachments/ag-b")));
    }

    #[test]
    fn a_folder_above_the_agents_own_is_never_refused() {
        let root = test_dir("fence-nested");
        let mut g = ground(&root, "a", &["b"]);
        // The private project's folder holds the other one's.
        g.projects[1].folders.push(root.clone());
        let f = compute(&g);
        assert!(!has(&f, &root.join("a")));
        assert!(!f.paths().iter().any(|p| p == &root));
        assert!(has(&f, &root.join("b")));
    }

    #[test]
    fn a_path_is_covered_whatever_its_spelling_and_not_by_a_name_that_starts_alike() {
        let f = Fence::new(vec![PathBuf::from(if cfg!(windows) {
            r"C:\Code\Secret"
        } else {
            "/code/secret"
        })]);
        if cfg!(windows) {
            assert!(f.covers("c:/code/secret/a.txt"));
            assert!(f.covers(r"C:\CODE\SECRET"));
            assert!(!f.covers(r"C:\Code\Secret2\a.txt"));
        } else {
            assert!(f.covers("/code/secret/a.txt"));
            assert!(!f.covers("/code/secret2/a.txt"));
        }
    }

    #[test]
    fn claude_names_a_folder_of_sessions_as_it_does() {
        assert_eq!(
            claude_project_name(Path::new(r"C:\Users\guill\AppData\Local\Temp")),
            "C--Users-guill-AppData-Local-Temp"
        );
        assert_eq!(
            claude_project_name(Path::new("/home/me/my app")),
            "-home-me-my-app"
        );
    }

    #[test]
    fn claude_code_reads_a_path_from_the_root_in_its_form() {
        assert_eq!(
            claude_rule_path(r"C:\Users\me\Secret"),
            "//c/Users/me/Secret"
        );
        assert_eq!(claude_rule_path("/home/me/secret/"), "//home/me/secret");
        // A character of a pattern, literal.
        assert_eq!(claude_rule_path("/home/me/a[1]*"), r"//home/me/a\[1\]\*");
    }

    #[test]
    fn claude_code_is_refused_reading_and_writing_each_path() {
        let path = if cfg!(windows) {
            r"C:\code\secret"
        } else {
            "/code/secret"
        };
        let rule = claude_rule_path(path);
        let program = crate::engine::Program::Native(PathBuf::from("claude"));
        let v = claude_settings(&Fence::new(vec![PathBuf::from(path)]), &program);
        let deny: Vec<&str> = v["permissions"]["deny"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap())
            .collect();
        assert_eq!(
            deny,
            [
                format!("Read({rule})"),
                format!("Read({rule}/**)"),
                format!("Edit({rule})"),
                format!("Edit({rule}/**)"),
            ]
        );
        // Its sandbox where it runs: not on native Windows, where it would be a setting it does
        // not know the paths of.
        if cfg!(windows) {
            assert!(v.get("sandbox").is_none());
        } else {
            assert_eq!(v["sandbox"]["filesystem"]["denyRead"][0], path);
            assert_eq!(v["sandbox"]["filesystem"]["denyWrite"][0], path);
        }
    }
}
