//! « Projet privé », its files (ESC-140): the paths an agent's harness is told to refuse it, as the app's `fence`
//! works them out from the projects and the accounts.

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
pub fn under(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/') || root.ends_with('/'))
}

/// A project, as its fence reads it.
#[derive(Debug, Clone, Default)]
pub struct Side {
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
pub struct Home {
    pub kind: HomeKind,
    pub dir: PathBuf,
}

/// What an agent's fence is made from.
#[derive(Debug, Clone, Default)]
pub struct Ground {
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
pub fn claude_project_name(folder: &Path) -> String {
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
pub fn compute(g: &Ground) -> Fence {
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

#[cfg(test)]
mod tests {
    use super::*;
    use escouade_base::testing::test_dir;

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
}
