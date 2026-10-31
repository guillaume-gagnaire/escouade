//! The workflow of a project (ESC-192, choices 133 to 138 and 155 of `UX-NOTES.md`; the study is
//! `docs/LOOPS.md`): what the files of each harness say of how the work goes, read by rules, on the
//! machine, at no cost.
//!
//! A workflow is never written in one place. It is put together from the commands, skills, prompts
//! and sub-agents of the harness' folders (`FOLDERS`): their names are the *doors*, the ones that
//! name each other (`/gsd:plan-phase`, `superpowers:writing-plans`, an `@file` they include) are the
//! *steps*, the sub-agents a step calls (`subagent_type="gsd-planner"`) are its agents, the files
//! it writes and reads (`.planning/STATE.md`) are its state, and « max 3 iterations » is a way
//! back. Two steps or more that are tied make a workflow; a command alone is not one. A prompt run
//! over and over (`PROMPT.md`, `/loop`) is one, by its turns and its checked list.
//!
//! The reading is a function of the files: `read` gives the same `Scan` for the same folders. The
//! commands of the user's own folders (`~/.claude`, `CODEX_HOME`) count for a project that has their
//! trace (a state file they write, a door an agent launched), and are never written. Each
//! repository of a project of several is read on its own (ESC-169). The last reading is kept in the
//! app's data folder (`Core::workflows`), read again when a file it read changed.

use crate::model::EngineId;
use crate::repos;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// A file longer than this is not read: a command is text, not a log.
const MAX_FILE: u64 = 256 * 1024;
/// How deep a folder of commands is walked (`commands/gsd/…`).
const WALK_DEPTH: usize = 4;
/// How many files an `@file` brings in, one from another.
const INCLUDE_DEPTH: usize = 4;
/// A door that names more than this many others is a list of them (`/gsd:help`), not a step.
const DIRECTORY: usize = 6;

/// The only kinds of file a workflow is read from.
const TEXT_EXT: &[&str] = &["md", "mdc", "txt", "yml", "yaml", "json", "toml"];
/// What is never read, even named: a sign-in, a secret, a conversation.
const SECRET_NAMES: &[&str] = &[".credentials.json", "auth.json"];
/// A harness' settings (`settings.json`, `settings.local.json`) or any `*.local.*` file: they may hold keys
/// or the user's own choices, never read (ESC-196).
fn is_config(name: &str) -> bool {
    (name.starts_with("settings") && name.ends_with(".json")) || name.contains(".local.")
}

/// The folders of a harness' own folder that hold its conversations.
const TRANSCRIPTS: &[&str] = &["projects", "sessions"];
/// What `Scan::searched` says of a file named and not read: `ignored:<why>:<as named>`, `<why>` one
/// of `outside`, `extension`, `secret`, `config`, `transcript`. Never its content.
pub const IGNORED: &str = "ignored:";

/// The folders of a harness, from a repository, and what each holds (`Engine::workflow_folders`):
/// what a reading tells it has looked in (« l'onglet dit ce qui a été cherché »).
pub type Folders = &'static [(&'static str, DocKind)];

/// The user's own folder of a harness its workflows are read from too (`Engine::workflow_user`),
/// as the roots of a scan have them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRoot {
    /// Claude Code's (`~/.claude`, or an account's): `CLAUDE_USER`.
    Claude,
    /// Codex's (`CODEX_HOME`, shared between its accounts): `CODEX_USER`.
    Codex,
}

/// The prompts and skills of Codex's folder (`CODEX_HOME`, shared between its accounts).
const CODEX_USER: Folders = &[("prompts", DocKind::Command), ("skills", DocKind::Skill)];
/// The folders of Claude Code's own folder (`~/.claude`, or an account's).
const CLAUDE_USER: Folders = &[
    ("commands", DocKind::Command),
    ("skills", DocKind::Skill),
    ("agents", DocKind::Agent),
];

/// The harnesses whose folders a repository is read for, in the registry's order.
fn read_harnesses() -> Vec<&'static dyn crate::engine::Engine> {
    crate::engine::registry::all()
        .into_iter()
        .filter(|e| !e.workflow_folders().is_empty())
        .collect()
}

/// The instructions read to understand, never drawn (choice 136).
const INSTRUCTIONS: &[&str] = &["CLAUDE.md", "AGENTS.md", ".github/copilot-instructions.md"];

/// The prompt of a loop of one prompt, and the lists it works through.
const PROMPTS: &[&str] = &["PROMPT.md", ".ralph/PROMPT.md"];
const LISTS: &[&str] = &["fix_plan.md", "@fix_plan.md", ".ralph/fix_plan.md"];

/// The state files known by name wherever they are said.
const STATE_NAMES: &[&str] = &[
    "STATE.md",
    "ROADMAP.md",
    "REQUIREMENTS.md",
    "PROJECT.md",
    "CONTEXT.md",
    "PLAN.md",
    "SUMMARY.md",
    "VERIFICATION.md",
    "fix_plan.md",
    "PROGRESS.md",
];

const WRITE_WORDS: &[&str] = &[
    "write",
    "update",
    "create",
    "append",
    "mark",
    "save",
    "record",
    "écri",
    "met à jour",
    "mets à jour",
    "crée",
    "ajoute",
    "coche",
];
const READ_WORDS: &[&str] = &[
    "read", "load", "parse", "consult", "lis ", "lit ", "lire", "charge", "check", "@",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DocKind {
    Command,
    Skill,
    Agent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// In the repository: versioned with it, the team's.
    Project,
    /// In the harness' own folder, on this machine (`~/.claude`).
    User,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Steps that name each other.
    Cycle,
    /// A prompt run over and over.
    Prompt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Where one goes in: the first step, or a door that runs several.
    Entry,
    Step,
    /// A door of the same family that is no step of this workflow (the « +38 »).
    Other,
}

/// What a reading looked at, to know whether it is still true.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub path: String,
    /// `command`, `skill`, `agent` for what the folders hold, `include`, `state`, `prompt`,
    /// `instruction` for the rest.
    pub kind: String,
    /// Modified, in milliseconds; 0 for a file gone.
    pub modified: i64,
    pub len: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Door {
    pub name: String,
    /// As typed: `/gsd:plan-phase`.
    pub command: String,
    pub kind: DocKind,
    pub scope: Scope,
    /// From the project's folder (`~/…` for the user's own folder).
    pub path: String,
    pub description: String,
    pub role: Role,
    /// For an entry that runs several steps: which.
    pub enters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StepAgent {
    pub name: String,
    /// None for one no file of the project or the user's folder defines (`general-purpose`).
    pub path: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub name: String,
    pub command: String,
    /// How far from the entry, going forward.
    pub order: usize,
    pub description: String,
    pub path: String,
    pub agents: Vec<StepAgent>,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
    /// A step the reading is not sure of (« deviné ») : dotted until « Garder » or « Retirer »
    /// (ESC-195). The rules make none; the agent that refines may.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub guessed: bool,
    /// The other commands and names that signal it (ESC-195): those of the steps merged into it,
    /// its name as read once renamed, the commands added by hand after the first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub from: String,
    pub to: String,
    /// A way back to a step already passed (or to the same one, again).
    pub back: bool,
    /// The line that says when, as the step wrote it.
    pub condition: Option<String>,
    pub max: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Progress {
    pub done: u32,
    pub total: u32,
}

/// What a state file says of where the work is.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Says {
    /// « phase 3 sur 6 ».
    pub phase: Option<Progress>,
    /// The boxes checked of those in it.
    pub checked: Option<Progress>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StateFile {
    /// From the project's folder.
    pub path: String,
    /// On this machine, now: never in the shared files (`workflowfiles`).
    #[serde(default)]
    pub exists: bool,
    pub written_by: Vec<String>,
    pub read_by: Vec<String>,
    pub says: Option<Says>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoopStep {
    pub n: u32,
    pub text: String,
}

/// A prompt run over and over: its turns are what is followed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromptLoop {
    /// `file` for a `PROMPT.md` run again, `loop` for a command that is a `/loop`.
    pub via: String,
    pub prompt: String,
    pub list: Option<StateFile>,
    /// The steps of a turn, from its numbered list.
    pub steps: Vec<LoopStep>,
    pub returns: Vec<Transition>,
    pub max_turns: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    /// None for a prompt loop that is no harness' (`PROMPT.md`).
    pub harness: Option<EngineId>,
    /// The repository of a project of several, from the project's folder; empty for the root.
    pub repo: String,
    pub doors: Vec<Door>,
    pub steps: Vec<Step>,
    pub transitions: Vec<Transition>,
    pub state_files: Vec<StateFile>,
    /// The line that says when it is over.
    pub exit: Option<String>,
    pub prompt_loop: Option<PromptLoop>,
    /// Set when « Affiner… » (ESC-196) made this definition: the fingerprint of the reading it was
    /// made from (`workflowrefine`). A reading that finds that same definition again leaves it; one
    /// that finds it changed writes its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refined: Option<String>,
}

/// Something of a harness' folders that is no workflow (a command alone).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Alone {
    pub name: String,
    pub command: String,
    pub harness: EngineId,
    pub repo: String,
    pub path: String,
    pub description: String,
}

/// A workflow of the user's own folder that this project has no trace of.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Untraced {
    pub name: String,
    pub harness: EngineId,
    pub steps: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Scan {
    /// When it was read, in milliseconds.
    pub read_at: i64,
    pub workflows: Vec<Workflow>,
    pub alone: Vec<Alone>,
    pub untraced: Vec<Untraced>,
    /// Every file looked at.
    pub sources: Vec<Source>,
    /// Every folder looked in, from the repository or the harness' own folder.
    pub searched: Vec<String>,
    /// Where the sources' paths start (ESC-195): the project's folder, Claude Code's folder and the
    /// home folder, for the window to say them as `.planning/STATE.md`, `~/.claude/…`, `~/…`.
    pub root: String,
    pub claude: String,
    pub home: String,
    /// The files of the workflows (ESC-195, `workflowfiles`): one per workflow, and those that could
    /// not be read, with why.
    pub files: Vec<crate::workflowfiles::FileInfo>,
    pub unreadable: Vec<crate::workflowfiles::Unreadable>,
    /// The workflows the user removed (ESC-309): never written again by a reading, proposed again
    /// when what found them changed.
    pub removed: Vec<crate::workflowfiles::RemovedInfo>,
    /// « Pas de workflow pour ce projet » (ESC-309): nothing read nor written, nothing shown.
    pub off: bool,
}

impl Scan {
    /// The workflows of a project that has none (« Pas de workflow pour ce projet », ESC-309).
    pub fn off() -> Self {
        Scan {
            off: true,
            ..Default::default()
        }
    }
}

/// Where the user's own folders are, given: the tests give a throwaway one, never the real one.
#[derive(Debug, Clone, Default)]
pub struct Roots {
    /// Claude Code's folder (`~/.claude`, `CLAUDE_CONFIG_DIR`); also where `~` goes for `@~/…`.
    pub home: PathBuf,
    pub claude: Option<PathBuf>,
    pub codex: Option<PathBuf>,
}

impl Roots {
    /// The machine user's: `~/.claude` (or the app's `CLAUDE_CONFIG_DIR`) and `~/.codex`. In the
    /// tests, folders that do not exist (`accounts::app_env`).
    pub fn of_user() -> Self {
        let (env, home) = crate::accounts::app_env();
        let claude = env
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude"));
        let codex = if cfg!(test) {
            None
        } else {
            std::env::var_os("CODEX_HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        }
        .unwrap_or_else(|| home.join(".codex"));
        Roots {
            home,
            claude: Some(claude),
            codex: Some(codex),
        }
    }
}

// ---------- the files ----------

fn norm(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn millis(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn source_of(path: &Path, kind: &str) -> Source {
    let meta = std::fs::metadata(path).ok();
    Source {
        path: norm(path),
        kind: kind.to_string(),
        modified: meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(millis)
            .unwrap_or(0),
        len: meta.map(|m| m.len()).unwrap_or(0),
    }
}

fn read_text(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(
        String::from_utf8_lossy(&bytes)
            .trim_start_matches('\u{feff}')
            .replace("\r\n", "\n"),
    )
}

/// The `.md` (or `.mdc`) files under `dir`, in the order of their paths.
fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = read.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        if path.is_dir() {
            if depth < WALK_DEPTH {
                walk(&path, depth + 1, out);
            }
        } else if matches!(
            path.extension().and_then(|x| x.to_str()),
            Some("md" | "mdc")
        ) {
            out.push(path);
        }
    }
}

/// The frontmatter of a file (its `key: value` lines) and what follows it.
pub(crate) fn frontmatter(text: &str) -> (BTreeMap<String, String>, &str) {
    let mut map = BTreeMap::new();
    let Some(rest) = text.strip_prefix("---\n") else {
        return (map, text);
    };
    let (head, body) = match rest.find("\n---") {
        Some(i) => (
            &rest[..i],
            rest[i + 4..].split_once('\n').map(|x| x.1).unwrap_or(""),
        ),
        None => return (map, text),
    };
    let mut lines = head.lines().peekable();
    while let Some(line) = lines.next() {
        if line.starts_with([' ', '\t']) {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let mut v = v.trim().to_string();
        if matches!(v.as_str(), ">" | "|" | ">-" | "|-") {
            v = lines
                .peek()
                .filter(|l| l.starts_with([' ', '\t']))
                .map(|l| l.trim().to_string())
                .unwrap_or_default();
        }
        let v = v.trim_matches(['"', '\'']).to_string();
        map.insert(k.trim().to_string(), v);
    }
    (map, body)
}

// ---------- the docs ----------

/// A command, skill, prompt or sub-agent of a harness' folder.
struct Doc {
    name: String,
    kind: DocKind,
    scope: Scope,
    shown: String,
    description: String,
    /// Its text and, below it, that of the files it includes.
    text: String,
    /// Said as `/loop …`: a prompt run again.
    is_loop: bool,
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_'
}

fn doc_name(rel: &Path, kind: DocKind, front: &BTreeMap<String, String>) -> String {
    let stem = |p: &Path| {
        let s = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        for ext in [".prompt.md", ".agent.md", ".md", ".mdc"] {
            if let Some(x) = s.strip_suffix(ext) {
                return x.to_string();
            }
        }
        s.to_string()
    };
    match kind {
        DocKind::Skill => front
            .get("name")
            .filter(|n| !n.is_empty())
            .cloned()
            .unwrap_or_else(|| {
                let dir = rel
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str());
                dir.map(str::to_string).unwrap_or_else(|| stem(rel))
            }),
        DocKind::Agent => front
            .get("name")
            .filter(|n| !n.is_empty())
            .cloned()
            .unwrap_or_else(|| stem(rel)),
        // `commands/gsd/plan-phase.md` is `gsd:plan-phase`.
        DocKind::Command => {
            let parent: Vec<String> = rel
                .parent()
                .map(|p| {
                    p.components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            let mut parts = parent;
            parts.push(stem(rel));
            parts.join(":")
        }
    }
}

struct Ctx<'a> {
    roots: &'a Roots,
    /// The repository being read.
    repo_dir: PathBuf,
    /// Its name from the project's folder: the paths shown are said from there.
    repo: String,
    sources: Vec<Source>,
    /// What was named and not read, with why (`IGNORED`).
    ignored: BTreeSet<String>,
}

impl Ctx<'_> {
    /// Whether the file at `path` (named `token`) may be read: under the project's repository or a
    /// harness' own folder once its links are followed, of a text kind, and neither a secret nor a
    /// conversation. What is refused is noted, not read.
    fn allow(&mut self, token: &str, path: &Path) -> bool {
        match self.refusal(path) {
            None => true,
            Some(why) => {
                self.ignored.insert(format!("{IGNORED}{why}:{token}"));
                false
            }
        }
    }

    fn refusal(&self, path: &Path) -> Option<&'static str> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if SECRET_NAMES.contains(&name.as_str()) || name.starts_with(".env") {
            return Some("secret");
        }
        if is_config(&name) {
            return Some("config");
        }
        let Ok(real) = path.canonicalize() else {
            return Some("outside");
        };
        let real_name = real
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if SECRET_NAMES.contains(&real_name.as_str()) || real_name.starts_with(".env") {
            return Some("secret");
        }
        if is_config(&real_name) {
            return Some("config");
        }
        let own: Vec<PathBuf> = [
            Some(&self.repo_dir),
            self.roots.claude.as_ref(),
            self.roots.codex.as_ref(),
        ]
        .into_iter()
        .flatten()
        .filter_map(|d| d.canonicalize().ok())
        .collect();
        let Some(base) = own.iter().find(|b| real.starts_with(b)) else {
            return Some("outside");
        };
        if *base != self.repo_dir.canonicalize().unwrap_or_default() {
            let first = real
                .strip_prefix(base)
                .ok()
                .and_then(|r| r.components().next())
                .map(|c| c.as_os_str().to_string_lossy().to_lowercase());
            if first.is_some_and(|f| TRANSCRIPTS.contains(&f.as_str())) {
                return Some("transcript");
            }
        }
        let ext = real
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or_default()
            .to_lowercase();
        (!TEXT_EXT.contains(&ext.as_str())).then_some("extension")
    }

    /// `read_text`, for a file that `allow` lets be read.
    fn read(&mut self, token: &str, path: &Path) -> Option<String> {
        if !path.exists() || !self.allow(token, path) {
            return None;
        }
        read_text(path)
    }

    fn shown(&self, path: &Path, scope: Scope) -> String {
        match scope {
            Scope::Project => path
                .strip_prefix(&self.repo_dir)
                .map(|p| repos::prefixed(&self.repo, &norm(p)))
                .unwrap_or_else(|_| norm(path)),
            // Claude Code's folder is `~/.claude` to the user, wherever `CLAUDE_CONFIG_DIR` puts it.
            Scope::User => self
                .roots
                .claude
                .as_ref()
                .and_then(|c| path.strip_prefix(c).ok())
                .map(|r| format!("~/.claude/{}", norm(r)))
                .or_else(|| {
                    path.strip_prefix(&self.roots.home)
                        .ok()
                        .map(|r| format!("~/{}", norm(r)))
                })
                .unwrap_or_else(|| norm(path)),
        }
    }

    /// The text of the file at `path` and of the files its `@` name, one from another, noted as
    /// read. A cycle of includes stops at the file seen.
    fn with_includes(
        &mut self,
        path: &Path,
        text: &str,
        seen: &mut Vec<PathBuf>,
        depth: usize,
    ) -> String {
        let mut out = text.to_string();
        if depth >= INCLUDE_DEPTH {
            return out;
        }
        for token in includes_of(text) {
            let Some(file) = self.resolve(&token, path) else {
                continue;
            };
            if seen.contains(&file) {
                continue;
            }
            let Some(body) = self.read(&token, &file) else {
                continue;
            };
            seen.push(file.clone());
            self.sources.push(source_of(&file, "include"));
            let more = self.with_includes(&file, &body, seen, depth + 1);
            out.push('\n');
            out.push_str(&more);
        }
        out
    }

    /// The file an `@token` of the file `from` names: `~/` from the home folder, a path from the
    /// repository or from the file's own folder.
    fn resolve(&self, token: &str, from: &Path) -> Option<PathBuf> {
        let token = token.replace('\\', "/");
        // `~/.claude/…` is Claude Code's folder (`CLAUDE_CONFIG_DIR` when set): where GSD installs.
        let claude = token
            .strip_prefix("~/.claude/")
            .zip(self.roots.claude.as_ref());
        let candidates: Vec<PathBuf> = if let Some((rest, dir)) = claude {
            vec![dir.join(rest)]
        } else if let Some(rest) = token.strip_prefix("~/") {
            vec![self.roots.home.join(rest)]
        } else if Path::new(&token).is_absolute() {
            vec![PathBuf::from(&token)]
        } else {
            vec![
                self.repo_dir.join(&token),
                from.parent().map(|d| d.join(&token)).unwrap_or_default(),
            ]
        };
        // The first that exists, if it may be read: a refused one is not replaced by another.
        candidates.into_iter().find(|c| c.is_file())
    }
}

/// The paths said after an `@` in `text`: with a `/` or an extension, so that `@gsd-planner` (a
/// name) is not one.
pub(crate) fn includes_of(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut rest = line;
        while let Some(at) = rest.find('@') {
            let before_ok = rest[..at]
                .chars()
                .last()
                .map(|c| !is_name_char(c) && c != '.')
                .unwrap_or(true);
            let after = &rest[at + 1..];
            let end = after
                .find(|c: char| {
                    c.is_whitespace() || matches!(c, '`' | '"' | '\'' | ')' | ',' | ';' | '<' | '>')
                })
                .unwrap_or(after.len());
            let token = after[..end].trim_end_matches(['.', ':', '!', '?']);
            if before_ok
                && !token.is_empty()
                && (token.contains('/') || Path::new(token).extension().is_some())
                && !token.contains("://")
                && !token.contains('{')
                && !token.contains('$')
                && !token.contains('@')
            {
                out.push(token.to_string());
            }
            rest = &after[end..];
        }
    }
    out
}

/// The `subagent_type`s a text names.
pub(crate) fn agent_calls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("subagent_type") {
        let after = &rest[i + "subagent_type".len()..];
        let after = after.trim_start_matches([' ', '=', ':', '"', '\'', '`', '(']);
        let name: String = after
            .chars()
            .take_while(|c| is_name_char(*c) || matches!(c, ':' | '.'))
            .collect();
        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
        rest = &rest[i + "subagent_type".len()..];
    }
    out
}

/// Whether `text` names the door or agent `name` (not as a part of a longer name): as a command
/// (`/name`), as a code (`` `name` ``), qualified (`superpowers:name`), or a name that cannot be
/// a word (`plan-phase`, `gsd:plan`).
pub(crate) fn mentions(text: &str, name: &str) -> bool {
    !mention_lines(text, name, 1).is_empty()
}

/// The lines of `text` that name `name`, at most `limit`.
fn mention_lines<'a>(text: &'a str, name: &str, limit: usize) -> Vec<&'a str> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut from = 0;
        let mut hit = false;
        while let Some(i) = line[from..].find(name) {
            let start = from + i;
            let end = start + name.len();
            from = end;
            let before = line[..start].chars().last();
            let after = line[end..].chars().next();
            if before.is_some_and(is_name_char) || after.is_some_and(is_name_char) {
                continue;
            }
            // `gsd:plan` in `gsd:plan-phase` is cut by the `-` above; in `gsd:plan:x`, by this.
            if after == Some(':') && line[end + 1..].chars().next().is_some_and(is_name_char) {
                continue;
            }
            // A file's name is no mention (`docs/plan.md`, `plan.md`): a path's `/` or an
            // extension after the name (ESC-195).
            let in_path = before == Some('/')
                && line[..start - 1]
                    .chars()
                    .last()
                    .is_some_and(|c| is_name_char(c) || c == '/' || c == '.');
            let extension = after == Some('.')
                && line[end + 1..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric());
            if in_path || extension {
                continue;
            }
            let distinctive = name.contains(['-', ':'])
                || matches!(before, Some('/' | ':'))
                || (before == Some('`') && after == Some('`'));
            if distinctive {
                hit = true;
                break;
            }
        }
        if hit {
            out.push(line);
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

fn clip(line: &str) -> String {
    let line = line.trim().trim_start_matches(['-', '*', '>', ' ']).trim();
    let mut s: String = line.chars().take(180).collect();
    if line.chars().count() > 180 {
        s.push('…');
    }
    s
}

// ---------- state files ----------

/// The files with a note-like extension that a line names.
fn file_tokens(line: &str) -> Vec<String> {
    line.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '`' | '"' | '\'' | '(' | ')' | ',' | ';' | '<' | '>' | '[' | ']' | '*'
            )
    })
    .map(|t| {
        t.trim_start_matches('@')
            .trim_end_matches(['.', ':', '!', '?'])
    })
    .filter(|t| {
        [".md", ".json", ".yml", ".yaml", ".txt", ".jsonl"]
            .iter()
            .any(|e| t.ends_with(e))
    })
    .filter(|t| !t.contains("://"))
    .map(|t| t.replace('\\', "/"))
    .collect()
}

/// The files a text writes and reads that may be state: `(path, writes, reads)`, in the order of
/// their first mention. A command, a skill, an agent, an instruction is not. Which of them are
/// state is `is_state` and what the steps of a workflow write.
pub(crate) fn state_refs(text: &str) -> Vec<(String, bool, bool)> {
    let mut out: Vec<(String, bool, bool)> = Vec::new();
    for line in text.lines() {
        let lower = line.to_lowercase();
        let writes = WRITE_WORDS.iter().any(|w| lower.contains(w));
        let reads = READ_WORDS.iter().any(|w| lower.contains(w)) || !writes;
        for tok in file_tokens(line) {
            let base = tok.rsplit('/').next().unwrap_or(&tok).to_string();
            let machinery = tok.contains("/commands/")
                || tok.contains("/agents/")
                || tok.contains("/skills/")
                || tok.contains("/workflows/")
                || tok.contains("/templates/")
                || tok.contains("/references/")
                || tok.starts_with("~/")
                || tok.starts_with("$")
                || base == "SKILL.md"
                || base == "CLAUDE.md"
                || base == "AGENTS.md"
                || base == "PROMPT.md"
                || base.ends_with(".prompt.md")
                || base.ends_with(".agent.md");
            if machinery {
                continue;
            }
            // Said by its folder (`{phase_dir}/…`) or alone: the same file under another name.
            let key = tok.trim_start_matches("./").to_string();
            match out.iter_mut().find(|(p, _, _)| *p == key) {
                Some(e) => {
                    e.1 |= writes;
                    e.2 |= reads;
                }
                None => out.push((key, writes, reads)),
            }
        }
    }
    out
}

/// Whether a file is state by its name alone: under `.planning/` or `.ralph/`, or known (`STATE.md`).
pub(crate) fn is_state(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    path.starts_with(".planning/")
        || path.starts_with(".ralph/")
        || STATE_NAMES
            .iter()
            .any(|n| base == *n || base.ends_with(&format!("-{n}")))
}

fn parse_progress_after(line: &str, word: &str) -> Option<Progress> {
    let lower = line.to_lowercase();
    let i = lower.find(word)?;
    let rest = &lower[i + word.len()..];
    let digits = |s: &str| -> Option<(u32, usize)> {
        let start = s.find(|c: char| c.is_ascii_digit())?;
        if start > 12 {
            return None;
        }
        let n: String = s[start..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        Some((n.parse().ok()?, start + n.len()))
    };
    let (done, used) = digits(rest)?;
    let tail = rest[used..].trim_start_matches([' ', '(', '*']);
    let tail = ["of", "sur", "/"]
        .iter()
        .find_map(|s| tail.strip_prefix(s))?;
    let (total, _) = digits(tail)?;
    (total >= done && total > 0).then_some(Progress { done, total })
}

pub(crate) fn says_of(text: &str) -> Says {
    let mut says = Says::default();
    let (mut done, mut total) = (0u32, 0u32);
    for line in text.lines() {
        let t = line.trim_start();
        for (open, closed) in [
            ("- [ ]", false),
            ("- [x]", true),
            ("- [X]", true),
            ("* [ ]", false),
            ("* [x]", true),
        ] {
            if t.starts_with(open) {
                total += 1;
                done += u32::from(closed);
                break;
            }
        }
        if says.phase.is_none() {
            says.phase = parse_progress_after(line, "phase");
        }
        if says.status.is_none() {
            let lower = line.to_lowercase();
            if let Some(i) = lower.find("status:") {
                let v = line[i + 7..].trim().trim_matches('*').trim();
                if !v.is_empty() {
                    says.status = Some(clip(v));
                }
            }
        }
    }
    if total > 0 {
        says.checked = Some(Progress { done, total });
    }
    says
}

fn state_file(ctx: &mut Ctx, repo: &str, path: &str) -> StateFile {
    let literal = !path.contains(['{', '$', '*', '<']);
    let abs = ctx.repo_dir.join(path);
    let text = if literal { ctx.read(path, &abs) } else { None };
    if text.is_some() {
        ctx.sources.push(source_of(&abs, "state"));
    }
    StateFile {
        path: repos::prefixed(repo, path),
        exists: text.is_some(),
        written_by: Vec::new(),
        read_by: Vec::new(),
        says: text.map(|t| says_of(&t)),
    }
}

// ---------- loops and ends ----------

/// The number a line puts as a maximum (« max 3 iterations », « jusqu'à 3 fois », « iteration_count
/// < 3 »), when it speaks of doing again.
pub(crate) fn loop_max(line: &str) -> Option<u32> {
    let lower = line.to_lowercase().replace('’', "'");
    let again = [
        "iterat", "retry", "retries", "attempt", "again", "loop", "revis", "repeat", "fois",
        "boucle", "tours", "round", "times", "cycle",
    ];
    if !again.iter().any(|w| lower.contains(w)) {
        return None;
    }
    let number_after = |s: &str| -> Option<u32> {
        let s = s.trim_start_matches([' ', ':', '=', '<', '*', '`']);
        let n: String = s.chars().take_while(char::is_ascii_digit).collect();
        n.parse().ok().filter(|n| (1..100).contains(n))
    };
    for trigger in [
        "max ",
        "max. ",
        "maximum ",
        "max:",
        "up to ",
        "at most ",
        "jusqu'à ",
        "jusqu'a ",
        "iteration_count <",
        "iterations <",
        "attempts <",
        "retries <",
        "< ",
    ] {
        if let Some(i) = lower.find(trigger) {
            if let Some(n) = number_after(&lower[i + trigger.len()..]) {
                return Some(n);
            }
        }
    }
    for unit in [
        "iteration",
        "time",
        "attempt",
        "retr",
        "fois",
        "tour",
        "round",
        "pass",
    ] {
        if let Some(i) = lower.find(unit) {
            let before = lower[..i].trim_end();
            let digits: String = before
                .chars()
                .rev()
                .take_while(char::is_ascii_digit)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if let Some(n) = digits.parse::<u32>().ok().filter(|n| (1..100).contains(n)) {
                return Some(n);
            }
        }
    }
    None
}

/// The line that says when a work is over.
pub(crate) fn exit_line(text: &str) -> Option<String> {
    let markers = [
        "until ",
        "when all ",
        "when there are no",
        "when every",
        "stop when",
        "stop if",
        "is complete when",
        "jusqu'à ",
        "jusqu’à ",
        "quand toutes",
        "quand il n",
        "<promise>",
        "exit when",
        "exit criteria",
        "workflow complete",
        "milestone complete",
        "all phases complete",
    ];
    let found = |loops: bool| {
        text.lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .find(|l| {
                let lower = l.to_lowercase();
                markers.iter().any(|m| lower.contains(m)) && (loops || loop_max(l).is_none())
            })
            .map(clip)
    };
    found(false).or_else(|| found(true))
}

// ---------- the reading ----------

struct Node {
    doc: Doc,
    agents: BTreeSet<String>,
    names: BTreeSet<usize>,
    refs: Vec<(String, bool, bool)>,
}

fn load_docs(
    ctx: &mut Ctx,
    base: &Path,
    rel: &str,
    kind: DocKind,
    scope: Scope,
    out: &mut Vec<Doc>,
) {
    let dir = base.join(rel);
    let mut files = Vec::new();
    walk(&dir, 0, &mut files);
    for file in files {
        let rel_in = file.strip_prefix(&dir).unwrap_or(&file).to_path_buf();
        // A skill is its `SKILL.md`; the other files of its folder are its own.
        if kind == DocKind::Skill && file.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
            continue;
        }
        let shown = ctx.shown(&file, scope);
        let Some(text) = ctx.read(&shown, &file) else {
            continue;
        };
        let (front, body) = frontmatter(&text);
        let name = doc_name(&rel_in, kind, &front);
        let description = front
            .get("description")
            .cloned()
            .filter(|d| !d.is_empty())
            .or_else(|| {
                body.lines()
                    .map(|l| l.trim().trim_start_matches('#').trim())
                    .find(|l| !l.is_empty() && !l.starts_with('<'))
                    .map(clip)
            })
            .unwrap_or_default();
        ctx.sources.push(source_of(
            &file,
            match kind {
                DocKind::Command => "command",
                DocKind::Skill => "skill",
                DocKind::Agent => "agent",
            },
        ));
        let is_loop = kind == DocKind::Command && {
            let first = body
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or("");
            first == "/loop" || first.starts_with("/loop ")
        };
        let mut seen = vec![file.clone()];
        let text = ctx.with_includes(&file, &text, &mut seen, 0);
        out.push(Doc {
            name,
            kind,
            scope,
            shown: ctx.shown(&file, scope),
            description,
            text,
            is_loop,
        });
    }
}

/// The docs of a harness for the repository: its folders in the repository, then (Claude Code,
/// Codex) the user's own, a project's one of the same name hiding the user's.
fn docs_of(ctx: &mut Ctx, harness: EngineId) -> Vec<Doc> {
    let mut docs = Vec::new();
    let repo_dir = ctx.repo_dir.clone();
    let harness = crate::engine::by_id(harness);
    for (rel, kind) in harness.workflow_folders() {
        load_docs(ctx, &repo_dir, rel, *kind, Scope::Project, &mut docs);
    }
    let user: Option<(PathBuf, Folders)> = match harness.workflow_user() {
        Some(UserRoot::Claude) => ctx.roots.claude.clone().map(|d| (d, CLAUDE_USER)),
        Some(UserRoot::Codex) => ctx.roots.codex.clone().map(|d| (d, CODEX_USER)),
        None => None,
    };
    if let Some((dir, folders)) = user {
        let mut theirs = Vec::new();
        for (rel, kind) in folders {
            load_docs(ctx, &dir, rel, *kind, Scope::User, &mut theirs);
        }
        for d in theirs {
            if !docs.iter().any(|m| m.name == d.name && m.kind == d.kind) {
                docs.push(d);
            }
        }
    }
    docs
}

fn command_of(name: &str) -> String {
    format!("/{name}")
}

fn display_name(ns: &str) -> String {
    match ns.to_lowercase().as_str() {
        "gsd" => "GSD".into(),
        "superpowers" => "Superpowers".into(),
        "ralph" => "Ralph".into(),
        other => {
            let mut c = other.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        }
    }
}

/// What a family of steps is called: the namespace most of its names share (`gsd`), else the one
/// the steps call each other by (`superpowers:writing-plans`), else its first step's name.
fn workflow_name(steps: &[&Node], first: &str) -> String {
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    for n in steps {
        if let Some((ns, _)) = n.doc.name.split_once(':') {
            *count.entry(ns.to_string()).or_default() += 1;
        }
    }
    if let Some((ns, _)) = count
        .iter()
        .max_by_key(|(k, v)| (**v, std::cmp::Reverse((*k).clone())))
    {
        return display_name(ns);
    }
    // `superpowers:` before a step's name, in the text of the steps.
    let mut qualified: BTreeMap<String, usize> = BTreeMap::new();
    for n in steps {
        for other in steps {
            let needle = format!(":{}", other.doc.name);
            let mut rest = n.doc.text.as_str();
            while let Some(i) = rest.find(&needle) {
                let prefix: String = rest[..i]
                    .chars()
                    .rev()
                    .take_while(|c| is_name_char(*c))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                if !prefix.is_empty() {
                    *qualified.entry(prefix).or_default() += 1;
                }
                rest = &rest[i + needle.len()..];
            }
        }
    }
    if let Some((ns, _)) = qualified
        .iter()
        .max_by_key(|(k, v)| (**v, std::cmp::Reverse((*k).clone())))
    {
        return display_name(ns);
    }
    first.to_string()
}

/// Whether `from` reaches `to` by the edges `edges` (not through `skip`).
fn reaches(edges: &BTreeMap<usize, BTreeSet<usize>>, from: usize, to: usize, skip: usize) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack = vec![from];
    while let Some(n) = stack.pop() {
        if n == to {
            return true;
        }
        if n == skip || !seen.insert(n) {
            continue;
        }
        if let Some(next) = edges.get(&n) {
            stack.extend(next.iter().copied());
        }
    }
    false
}

/// A depth-first walk from the sources: the forward edges' depth of each node, and the edges that
/// go back to a node still being walked.
fn order(
    members: &[usize],
    edges: &BTreeMap<usize, BTreeSet<usize>>,
) -> (BTreeMap<usize, usize>, BTreeSet<(usize, usize)>) {
    let inner: BTreeSet<usize> = members.iter().copied().collect();
    let mut has_in: BTreeSet<usize> = BTreeSet::new();
    for (a, to) in edges {
        if inner.contains(a) {
            has_in.extend(to.iter().copied().filter(|b| inner.contains(b) && b != a));
        }
    }
    let mut starts: Vec<usize> = members
        .iter()
        .copied()
        .filter(|m| !has_in.contains(m))
        .collect();
    if starts.is_empty() {
        starts.push(members[0]);
    }
    let mut back = BTreeSet::new();
    let mut state: BTreeMap<usize, u8> = BTreeMap::new();
    let mut post: Vec<usize> = Vec::new();
    // Iterative, to stay off the stack for a long chain.
    for s in starts.iter().chain(members.iter()) {
        if state.contains_key(s) {
            continue;
        }
        let mut stack: Vec<(usize, Vec<usize>)> = Vec::new();
        state.insert(*s, 1);
        stack.push((
            *s,
            edges
                .get(s)
                .map(|e| e.iter().rev().copied().collect())
                .unwrap_or_default(),
        ));
        while let Some((n, todo)) = stack.last_mut() {
            let n = *n;
            match todo.pop() {
                Some(m) if inner.contains(&m) => match state.get(&m) {
                    None => {
                        state.insert(m, 1);
                        stack.push((
                            m,
                            edges
                                .get(&m)
                                .map(|e| e.iter().rev().copied().collect())
                                .unwrap_or_default(),
                        ));
                    }
                    Some(1) => {
                        back.insert((n, m));
                    }
                    _ => {}
                },
                Some(_) => {}
                None => {
                    state.insert(n, 2);
                    post.push(n);
                    stack.pop();
                }
            }
        }
    }
    // Longest forward path to each node, in the reverse order of finishing.
    let mut depth: BTreeMap<usize, usize> = members.iter().map(|m| (*m, 0)).collect();
    for n in post.iter().rev() {
        let d = depth[n];
        if let Some(next) = edges.get(n) {
            for m in next {
                if inner.contains(m) && !back.contains(&(*n, *m)) && m != n {
                    let e = depth.entry(*m).or_insert(0);
                    *e = (*e).max(d + 1);
                }
            }
        }
    }
    (depth, back)
}

/// The workflows (and what is alone) of one harness in one repository.
fn read_harness(ctx: &mut Ctx, repo: &str, harness: EngineId, launched: &[String], out: &mut Scan) {
    let docs = docs_of(ctx, harness);
    let agents: Vec<usize> = (0..docs.len())
        .filter(|i| docs[*i].kind == DocKind::Agent)
        .collect();
    let gates: Vec<usize> = (0..docs.len())
        .filter(|i| docs[*i].kind != DocKind::Agent)
        .collect();
    if gates.is_empty() {
        return;
    }
    let agent_names: Vec<String> = agents.iter().map(|i| docs[*i].name.clone()).collect();
    let mut nodes: Vec<Node> = Vec::new();
    let gate_names: Vec<String> = gates.iter().map(|i| docs[*i].name.clone()).collect();
    let mut docs_iter: Vec<Option<Doc>> = docs.into_iter().map(Some).collect();
    // The agents' docs, kept for the steps' agents.
    let agent_docs: Vec<Doc> = agents.iter().filter_map(|i| docs_iter[*i].take()).collect();
    for i in &gates {
        let Some(doc) = docs_iter[*i].take() else {
            continue;
        };
        let mut called: BTreeSet<String> = agent_calls(&doc.text).into_iter().collect();
        for a in &agent_names {
            if mentions(&doc.text, a) {
                called.insert(a.clone());
            }
        }
        let refs = state_refs(&doc.text);
        nodes.push(Node {
            doc,
            agents: called,
            names: BTreeSet::new(),
            refs,
        });
    }
    let named: Vec<BTreeSet<usize>> = nodes
        .iter()
        .enumerate()
        .map(|(a, n)| {
            gate_names
                .iter()
                .enumerate()
                .filter(|(b, g)| *b != a && mentions(&n.doc.text, g))
                .map(|(b, _)| b)
                .collect()
        })
        .collect();
    for (n, names) in nodes.iter_mut().zip(named) {
        n.names = names;
    }
    let mut edges: BTreeMap<usize, BTreeSet<usize>> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (i, n.names.clone()))
        .collect();
    // A door that lists many, or that runs a step then one that follows it, is an entry, not a
    // step: its edges are taken out of the graph.
    let mut entries: BTreeSet<usize> = BTreeSet::new();
    for (i, n) in nodes.iter().enumerate() {
        let many = n.names.len() > DIRECTORY;
        let runs = n
            .names
            .iter()
            .any(|a| n.names.iter().any(|b| a != b && reaches(&edges, *a, *b, i)));
        if many || runs {
            entries.insert(i);
        }
    }
    for e in &entries {
        edges.insert(*e, BTreeSet::new());
    }
    // A file one writes and another only reads ties two doors none names.
    let mut linked: BTreeSet<(usize, usize)> = BTreeSet::new();
    for a in 0..nodes.len() {
        for b in 0..nodes.len() {
            if a == b || entries.contains(&a) || entries.contains(&b) {
                continue;
            }
            let ties = nodes[a]
                .refs
                .iter()
                .any(|(p, w, _)| *w && nodes[b].refs.iter().any(|(q, w2, r)| p == q && *r && !*w2));
            if ties {
                linked.insert((a, b));
            }
        }
    }
    let strong: BTreeSet<usize> = edges
        .iter()
        .flat_map(|(a, to)| {
            to.iter()
                .copied()
                .chain(std::iter::once(*a))
                .filter(|_| !to.is_empty())
        })
        .collect();
    for (a, b) in linked {
        // Only for doors no name already ties: a file that all read is not a workflow.
        if !strong.contains(&a) && !strong.contains(&b) {
            edges.entry(a).or_default().insert(b);
        }
    }
    // Components of the graph (without its direction).
    let mut parent: Vec<usize> = (0..nodes.len()).collect();
    fn find(p: &mut Vec<usize>, x: usize) -> usize {
        if p[x] != x {
            let r = find(p, p[x]);
            p[x] = r;
        }
        p[x]
    }
    for (a, to) in &edges {
        for b in to {
            let (ra, rb) = (find(&mut parent, *a), find(&mut parent, *b));
            if ra != rb {
                parent[rb.max(ra)] = ra.min(rb);
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..nodes.len() {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let mut in_workflow: BTreeSet<usize> = BTreeSet::new();
    let mut found: Vec<Workflow> = Vec::new();
    for members in groups.values() {
        let steps_idx: Vec<usize> = members
            .iter()
            .copied()
            .filter(|m| !entries.contains(m))
            .collect();
        if steps_idx.len() < 2 {
            continue;
        }
        let (depth, back) = order(&steps_idx, &edges);
        let mut sorted = steps_idx.clone();
        sorted
            .sort_by(|a, b| (depth[a], &nodes[*a].doc.name).cmp(&(depth[b], &nodes[*b].doc.name)));
        let step_nodes: Vec<&Node> = sorted.iter().map(|i| &nodes[*i]).collect();
        let name = workflow_name(&step_nodes, &nodes[sorted[0]].doc.name);
        // Entries that run this group's steps.
        let doors_in: Vec<usize> = entries
            .iter()
            .copied()
            .filter(|e| nodes[*e].names.iter().any(|n| steps_idx.contains(n)))
            .collect();
        let local = |i: usize| nodes[i].doc.name.clone();
        let mut transitions: Vec<Transition> = Vec::new();
        for a in &sorted {
            for b in edges.get(a).into_iter().flatten() {
                if !steps_idx.contains(b) || a == b {
                    continue;
                }
                let is_back = back.contains(&(*a, *b));
                let condition = is_back
                    .then(|| {
                        mention_lines(&nodes[*a].doc.text, &local(*b), 1)
                            .first()
                            .map(|l| clip(l))
                    })
                    .flatten();
                transitions.push(Transition {
                    from: local(*a),
                    to: local(*b),
                    back: is_back,
                    max: condition.as_deref().and_then(loop_max),
                    condition,
                });
            }
        }
        // « jusqu'à 3 fois » said of the step's own work (the planner and its checker): the step
        // goes back to itself, up to that.
        for a in &sorted {
            let lines: Vec<&str> = nodes[*a].doc.text.lines().collect();
            for line in lines.iter() {
                let Some(max) = loop_max(line) else { continue };
                if transitions
                    .iter()
                    .any(|t| t.from == local(*a) && t.back && t.max == Some(max))
                {
                    continue;
                }
                // Attached to a way back already found from this step that names no limit.
                if let Some(t) = transitions
                    .iter_mut()
                    .find(|t| t.from == local(*a) && t.back && t.max.is_none())
                {
                    t.max = Some(max);
                    if t.condition.is_none() {
                        t.condition = Some(clip(line));
                    }
                    continue;
                }
                transitions.push(Transition {
                    from: local(*a),
                    to: local(*a),
                    back: true,
                    condition: Some(clip(line)),
                    max: Some(max),
                });
            }
        }
        // State files.
        let written: BTreeSet<&str> = sorted
            .iter()
            .flat_map(|i| nodes[*i].refs.iter().filter(|r| r.1).map(|r| r.0.as_str()))
            .collect();
        let is_kept = |p: &str| is_state(p) || written.contains(p);
        let mut state: Vec<StateFile> = Vec::new();
        for i in &sorted {
            for (path, w, r) in nodes[*i].refs.iter().filter(|r| is_kept(&r.0)) {
                let sf = match state
                    .iter_mut()
                    .position(|s| s.path == repos::prefixed(repo, path))
                {
                    Some(p) => &mut state[p],
                    None => {
                        state.push(state_file(ctx, repo, path));
                        state.last_mut().unwrap()
                    }
                };
                if *w && !sf.written_by.contains(&local(*i)) {
                    sf.written_by.push(local(*i));
                }
                if *r && !sf.read_by.contains(&local(*i)) {
                    sf.read_by.push(local(*i));
                }
            }
        }
        let exit = sorted
            .iter()
            .rev()
            .find_map(|i| exit_line(&nodes[*i].doc.text));
        let steps: Vec<Step> = sorted
            .iter()
            .map(|i| {
                let n = &nodes[*i];
                Step {
                    name: n.doc.name.clone(),
                    command: command_of(&n.doc.name),
                    order: depth[i],
                    description: n.doc.description.clone(),
                    path: n.doc.shown.clone(),
                    agents: n
                        .agents
                        .iter()
                        .map(|a| {
                            let d = agent_docs.iter().find(|d| d.name == *a);
                            StepAgent {
                                name: a.clone(),
                                path: d.map(|d| d.shown.clone()),
                                description: d.map(|d| d.description.clone()).unwrap_or_default(),
                            }
                        })
                        .collect(),
                    reads: n
                        .refs
                        .iter()
                        .filter(|r| r.2 && is_kept(&r.0))
                        .map(|r| repos::prefixed(repo, &r.0))
                        .collect(),
                    writes: n
                        .refs
                        .iter()
                        .filter(|r| r.1 && is_kept(&r.0))
                        .map(|r| repos::prefixed(repo, &r.0))
                        .collect(),
                    guessed: false,
                    also: Vec::new(),
                }
            })
            .collect();
        let first = sorted[0];
        let ns = nodes[first]
            .doc
            .name
            .split_once(':')
            .map(|x| x.0.to_string());
        let mut doors: Vec<Door> = Vec::new();
        for i in &sorted {
            let n = &nodes[*i];
            doors.push(Door {
                name: n.doc.name.clone(),
                command: command_of(&n.doc.name),
                kind: n.doc.kind,
                scope: n.doc.scope,
                path: n.doc.shown.clone(),
                description: n.doc.description.clone(),
                role: if depth[i] == 0 {
                    Role::Entry
                } else {
                    Role::Step
                },
                enters: Vec::new(),
            });
        }
        for e in &doors_in {
            let n = &nodes[*e];
            doors.push(Door {
                name: n.doc.name.clone(),
                command: command_of(&n.doc.name),
                kind: n.doc.kind,
                scope: n.doc.scope,
                path: n.doc.shown.clone(),
                description: n.doc.description.clone(),
                role: if n.names.len() > DIRECTORY {
                    Role::Other
                } else {
                    Role::Entry
                },
                enters: n
                    .names
                    .iter()
                    .filter(|x| steps_idx.contains(x))
                    .map(|x| nodes[*x].doc.name.clone())
                    .collect(),
            });
        }
        // The rest of the family.
        if let Some(ns) = &ns {
            for (i, n) in nodes.iter().enumerate() {
                let listed = steps_idx.contains(&i) || doors_in.contains(&i);
                if !listed && n.doc.name.split_once(':').is_some_and(|x| x.0 == ns) {
                    doors.push(Door {
                        name: n.doc.name.clone(),
                        command: command_of(&n.doc.name),
                        kind: n.doc.kind,
                        scope: n.doc.scope,
                        path: n.doc.shown.clone(),
                        description: n.doc.description.clone(),
                        role: Role::Other,
                        enters: Vec::new(),
                    });
                    in_workflow.insert(i);
                }
            }
        }
        in_workflow.extend(members.iter().copied());
        let user_made = sorted.iter().any(|i| nodes[*i].doc.scope == Scope::User);
        let traced = !user_made
            || state.iter().any(|s| s.exists && !s.written_by.is_empty())
            || launched.iter().any(|l| {
                let l = l.trim_start_matches('/');
                steps.iter().any(|s| s.name == l) || doors.iter().any(|d| d.name == l)
            });
        if !traced {
            out.untraced.push(Untraced {
                name,
                harness,
                steps: steps.len(),
            });
            continue;
        }
        found.push(Workflow {
            id: format!("{repo}|{}|{}", engine_slug(harness), slug(&name)),
            name,
            kind: Kind::Cycle,
            harness: Some(harness),
            repo: repo.to_string(),
            doors,
            steps,
            transitions,
            state_files: state,
            exit,
            prompt_loop: None,
            refined: None,
        });
    }
    // Loops of one prompt through `/loop`.
    for (i, n) in nodes.iter().enumerate() {
        if n.doc.is_loop && n.doc.scope == Scope::Project {
            in_workflow.insert(i);
            let wf = prompt_loop_of(
                ctx,
                repo,
                Some(harness),
                &n.doc.text,
                &n.doc.shown,
                "loop",
                &n.doc.name,
            );
            if let Some(wf) = wf {
                found.push(wf);
            }
        }
    }
    out.workflows.extend(found);
    for (i, n) in nodes.iter().enumerate() {
        if !in_workflow.contains(&i) && n.doc.scope == Scope::Project && !entries.contains(&i) {
            out.alone.push(Alone {
                name: n.doc.name.clone(),
                command: command_of(&n.doc.name),
                harness,
                repo: repo.to_string(),
                path: n.doc.shown.clone(),
                description: n.doc.description.clone(),
            });
        }
    }
}

fn engine_slug(e: EngineId) -> &'static str {
    e.as_str()
}

fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

/// The numbered steps of a text: `1. Pick the first task`.
pub(crate) fn numbered(text: &str) -> Vec<LoopStep> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() || digits.len() > 2 {
            continue;
        }
        let rest = &t[digits.len()..];
        if let Some(body) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            if let Ok(n) = digits.parse() {
                out.push(LoopStep {
                    n,
                    text: clip(body.trim_start_matches('*')),
                });
            }
        }
    }
    out
}

/// A loop of one prompt: its prompt's text, how it is run, the list it works through.
fn prompt_loop_of(
    ctx: &mut Ctx,
    repo: &str,
    harness: Option<EngineId>,
    text: &str,
    shown: &str,
    via: &str,
    name: &str,
) -> Option<Workflow> {
    let steps = numbered(text);
    let refs = state_refs(text);
    let mut list_path = refs.iter().map(|r| r.0.clone()).find(|p| {
        LISTS.iter().any(|l| {
            p.trim_start_matches('@')
                .ends_with(l.trim_start_matches('@'))
        })
    });
    if list_path.is_none() {
        list_path = LISTS
            .iter()
            .map(|l| l.trim_start_matches('@').to_string())
            .find(|l| ctx.repo_dir.join(l).is_file());
    }
    let list = list_path.map(|p| {
        let mut sf = state_file(ctx, repo, &p);
        sf.read_by.push(name.to_string());
        sf.written_by.push(name.to_string());
        sf
    });
    // « If red, go back to 2 »: a numbered step that goes back to an earlier one.
    let mut returns = Vec::new();
    for s in &steps {
        let lower = s.text.to_lowercase();
        if ![
            "go back",
            "return to",
            "retour",
            "back to",
            "loop back",
            "retry",
            "again",
        ]
        .iter()
        .any(|w| lower.contains(w))
        {
            continue;
        }
        let target = steps.iter().find(|t| {
            t.n < s.n
                && (lower.contains(&format!("step {}", t.n))
                    || lower.contains(&format!("étape {}", t.n))
                    || lower.contains(&format!("back to {}", t.n))
                    || lower.contains(&format!("retour à {}", t.n))
                    || t.text
                        .split_whitespace()
                        .next()
                        .is_some_and(|w| w.len() > 3 && lower.contains(&w.to_lowercase())))
        });
        if let Some(t) = target {
            returns.push(Transition {
                from: s.n.to_string(),
                to: t.n.to_string(),
                back: true,
                condition: Some(s.text.clone()),
                max: loop_max(&s.text),
            });
        }
    }
    let max_turns = text.lines().find_map(|l| {
        let lower = l.to_lowercase();
        (lower.contains("iteration")
            || lower.contains("turns")
            || lower.contains("tour")
            || lower.contains("loop"))
        .then(|| loop_max(l))
        .flatten()
    });
    let exit = exit_line(text);
    let display = {
        let lower = format!("{name} {shown} {text}").to_lowercase();
        if lower.contains("ralph")
            || list
                .as_ref()
                .is_some_and(|l| l.path.ends_with("fix_plan.md"))
        {
            "Ralph".to_string()
        } else {
            name.to_string()
        }
    };
    let loop_steps: Vec<Step> = steps
        .iter()
        .map(|s| Step {
            name: s.n.to_string(),
            command: String::new(),
            order: s.n as usize,
            description: s.text.clone(),
            path: shown.to_string(),
            agents: Vec::new(),
            reads: Vec::new(),
            writes: Vec::new(),
            guessed: false,
            also: Vec::new(),
        })
        .collect();
    Some(Workflow {
        id: format!(
            "{repo}|{}|{}",
            harness.map(engine_slug).unwrap_or("any"),
            slug(&display)
        ),
        name: display,
        kind: Kind::Prompt,
        harness,
        repo: repo.to_string(),
        doors: vec![Door {
            name: name.to_string(),
            command: if via == "loop" {
                command_of(name)
            } else {
                shown.to_string()
            },
            kind: DocKind::Command,
            scope: Scope::Project,
            path: shown.to_string(),
            description: String::new(),
            role: Role::Entry,
            enters: Vec::new(),
        }],
        steps: loop_steps,
        transitions: returns.clone(),
        state_files: list.iter().cloned().collect(),
        exit: exit.clone(),
        prompt_loop: Some(PromptLoop {
            via: via.to_string(),
            prompt: shown.to_string(),
            list,
            steps,
            returns,
            max_turns,
        }),
        refined: None,
    })
}

fn read_repo(roots: &Roots, repo_dir: &Path, repo: &str, launched: &[String], out: &mut Scan) {
    let mut ctx = Ctx {
        roots,
        repo_dir: repo_dir.to_path_buf(),
        repo: repo.to_string(),
        sources: Vec::new(),
        ignored: BTreeSet::new(),
    };
    for harness in read_harnesses() {
        read_harness(&mut ctx, repo, harness.id(), launched, out);
    }
    // A prompt run over and over: `PROMPT.md`.
    for p in PROMPTS {
        let file = repo_dir.join(p);
        if let Some(text) = ctx.read(p, &file) {
            ctx.sources.push(source_of(&file, "prompt"));
            let mut seen = vec![file.clone()];
            let full = ctx.with_includes(&file, &text, &mut seen, 0);
            if let Some(wf) = prompt_loop_of(
                &mut ctx,
                repo,
                None,
                &full,
                &repos::prefixed(repo, p),
                "file",
                "PROMPT.md",
            ) {
                out.workflows.push(wf);
            }
            break;
        }
    }
    for rel in INSTRUCTIONS {
        let file = repo_dir.join(rel);
        if file.is_file() {
            ctx.sources.push(source_of(&file, "instruction"));
        }
    }
    out.sources.extend(ctx.sources);
    out.searched.extend(ctx.ignored);
}

/// Where each harness looks, said from a repository (what an empty tab says it searched).
pub fn searched() -> Vec<String> {
    let mut v: Vec<String> = read_harnesses()
        .iter()
        .flat_map(|e| e.workflow_folders().iter().map(|(rel, _)| rel.to_string()))
        .collect();
    v.extend(PROMPTS.iter().map(|p| p.to_string()));
    v.sort();
    v.dedup();
    v
}

/// The workflows of the project at `root`: of its repositories `repos` (from `repos::detect`;
/// empty for a project of one), each read on its own, those of the user's own folders counted for
/// the project that has their trace (`launched`: the doors its agents launched, `/gsd:plan-phase`).
pub fn read(root: &Path, repos: &[String], roots: &Roots, launched: &[String]) -> Scan {
    let mut out = Scan {
        read_at: millis(std::time::SystemTime::now()),
        searched: searched(),
        root: norm(root),
        claude: roots.claude.as_deref().map(norm).unwrap_or_default(),
        home: norm(&roots.home),
        ..Default::default()
    };
    let list: Vec<String> = if repos.is_empty() {
        vec![String::new()]
    } else {
        repos.to_vec()
    };
    for repo in &list {
        let dir = PathBuf::from(repos::join(&root.to_string_lossy(), repo));
        if dir.is_dir() {
            read_repo(roots, &dir, repo, launched, &mut out);
        }
    }
    // The family of a workflow of another harness or repository is no workflow's « other ».
    out.sources.sort();
    out.sources.dedup();
    out.workflows.sort_by(|a, b| a.id.cmp(&b.id));
    // A repository's workflow is named by it (« api · GSD »), in the interface; here, only the id
    // and `repo` tell.
    out
}

/// Whether what `scan` read has changed: a file gone, written, or one more in the folders looked
/// in (the stats alone are read, not the files).
pub fn is_stale(scan: &Scan, root: &Path, repos: &[String], roots: &Roots) -> bool {
    let changed = scan
        .sources
        .iter()
        .any(|s| source_of(Path::new(&s.path), &s.kind) != *s);
    if changed {
        return true;
    }
    // A new command or agent: the same files in the folders, no more, no fewer.
    let again = read(root, repos, roots, &[]);
    let listed = |s: &Scan| -> Vec<String> {
        s.sources
            .iter()
            .filter(|x| {
                matches!(
                    x.kind.as_str(),
                    "command" | "skill" | "agent" | "prompt" | "instruction"
                )
            })
            .map(|x| x.path.clone())
            .collect()
    };
    listed(&again) != listed(scan)
}

// ---------- kept, read again ----------

/// How often the window coming back to the front looks at every project's workflows.
const ALL_EVERY: std::time::Duration = std::time::Duration::from_secs(30);
static LAST_ALL: parking_lot::Mutex<Option<std::time::Instant>> = parking_lot::Mutex::new(None);
static ALL_RUNNING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
/// By project: the fingerprint of its workflows' files as last read or written, and what the
/// readings of this run changed of each workflow (« Mis à jour par la relecture »).
#[allow(clippy::type_complexity)]
static FILES: std::sync::LazyLock<
    parking_lot::Mutex<std::collections::HashMap<String, (String, BTreeMap<String, String>)>>,
> = std::sync::LazyLock::new(Default::default);

/// What a change of a workflow's file is said to have been (« Mis à jour par … »).
pub(crate) fn mark_updated(project_id: &str, id: &str, what: &str) {
    FILES
        .lock()
        .entry(project_id.to_string())
        .or_default()
        .1
        .insert(id.to_string(), what.to_string());
}

impl<R: tauri::Runtime> crate::core::Core<R> {
    /// The workflows of the project: the last reading kept in the data folder, read again when a
    /// file it read changed (or `force`: « Relire »), and kept when it did. A project that was
    /// never read is read now.
    pub async fn refresh_workflows(&self, project_id: &str, force: bool) -> anyhow::Result<Scan> {
        let project = self.project(project_id)?;
        // « Pas de workflow pour ce projet » (ESC-309): neither read nor written.
        if !project.workflows_enabled {
            return Ok(Scan::off());
        }
        let file = self.data.workflows_file(project_id);
        let stored: Option<Scan> = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        let (root, repos) = (PathBuf::from(&project.path), project.repos.clone());
        // « Relire quand les fichiers changent » off (ESC-195): only « Relire » reads again.
        let auto = project.workflows_auto_read;
        let (scan, changed) = tauri::async_runtime::spawn_blocking(move || {
            let roots = Roots::of_user();
            match stored {
                Some(s) if !force && (!auto || !is_stale(&s, &root, &repos, &roots)) => (s, false),
                _ => (read(&root, &repos, &roots, &[]), true),
            }
        })
        .await?;
        if changed {
            if let Some(dir) = file.parent() {
                std::fs::create_dir_all(dir)?;
            }
            crate::paths::write_atomic(&file, &serde_json::to_vec_pretty(&scan)?)?;
        }
        // The files of the workflows (ESC-195): what this reading found added or updated there,
        // never removed; then the workflows of the files are those the project shows and follows.
        let places = crate::workflowfiles::places(&project, &self.data);
        let mut folder = crate::workflowfiles::load_all(&places);
        // Those the user removed are not written again (ESC-309).
        let removals_file = crate::workflowfiles::removals_file(&self.data, project_id);
        let before = crate::workflowfiles::Removals::load(&removals_file);
        let mut removals = before.clone();
        let written = crate::workflowfiles::merge_reading_all(
            &places,
            &folder,
            &scan.workflows,
            Path::new(&project.path).is_dir(),
            &mut removals,
        );
        removals.save(&removals_file, &before);
        if !written.is_empty() {
            let mut files = FILES.lock();
            let seen = files.entry(project_id.to_string()).or_default();
            for (id, what) in &written {
                seen.1.insert(id.clone(), what.clone());
            }
            drop(files);
            folder = crate::workflowfiles::load_all(&places);
        }
        let shared = self.scan_from_files(&project, &folder, &scan);
        self.workflow_read(project_id, &shared, changed || !written.is_empty());
        Ok(shared)
    }

    /// The reading of this machine kept in the data folder (`workflows.json`), as read.
    pub(crate) fn cached_reading(&self, project_id: &str) -> Scan {
        std::fs::read(self.data.workflows_file(project_id))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// The workflows of the project's files (ESC-195) beside what `local` read of the rest; the
    /// corrections they hold taken by the project when they changed.
    fn scan_from_files(
        &self,
        project: &crate::model::Project,
        folder: &crate::workflowfiles::Folder,
        local: &Scan,
    ) -> Scan {
        let fingerprint = folder.fingerprint();
        let updated = {
            let mut files = FILES.lock();
            let seen = files.entry(project.id.clone()).or_default();
            seen.0 = fingerprint;
            seen.1.clone()
        };
        self.take_workflow_fixes(&project.id, folder.fixes());
        let root = PathBuf::from(&project.path);
        let shown = |p: &Path| match p.strip_prefix(&root) {
            Ok(rel) => norm(rel),
            Err(_) => norm(p),
        };
        let mut scan = crate::workflowfiles::scan_of(project, folder, local, shown, &updated);
        scan.removed = crate::workflowfiles::Removals::load(&crate::workflowfiles::removals_file(
            &self.data,
            &project.id,
        ))
        .listed(&local.workflows);
        scan
    }

    /// The workflows of the project as its files and this machine's last reading say them, read
    /// from neither the harnesses' folders nor anything else: what the tracking starts from.
    pub(crate) fn workflows_of(&self, project_id: &str) -> Scan {
        let local = self.cached_reading(project_id);
        let Ok(project) = self.project(project_id) else {
            return local;
        };
        if !project.workflows_enabled {
            return Scan::off();
        }
        let folder =
            crate::workflowfiles::load_all(&crate::workflowfiles::places(&project, &self.data));
        self.scan_from_files(&project, &folder, &local)
    }

    /// The corrections the files hold, taken by the project when they are not its own: the window
    /// told.
    fn take_workflow_fixes(&self, project_id: &str, fixes: crate::workflowfix::Fixes) {
        let updated = {
            let mut projects = self.projects.write();
            let Some(p) = projects.iter_mut().find(|p| p.id == project_id) else {
                return;
            };
            if p.workflow_fixes == fixes {
                return;
            }
            p.workflow_fixes = fixes;
            p.clone()
        };
        self.request_save();
        self.hub
            .emit(crate::model::UiEvent::Project { project: updated });
    }

    /// The project's corrections written into the files of its workflows (Réglages › Projets ›
    /// Workflow saved).
    pub(crate) fn write_workflow_fixes(&self, project: &crate::model::Project) {
        if !project.workflows_enabled {
            return;
        }
        let places = crate::workflowfiles::places(project, &self.data);
        let folder = crate::workflowfiles::load_all(&places);
        if let Err(e) = crate::workflowfiles::set_fixes(&folder, &project.workflow_fixes) {
            log::warn!("workflow fixes of {}: {e:#}", project.id);
        }
        let folder = crate::workflowfiles::load_all(&places);
        FILES.lock().entry(project.id.clone()).or_default().0 = folder.fingerprint();
        self.take_workflow_fixes(&project.id, folder.fixes());
    }

    /// The project's folder changed (a pull, another branch): its workflows' files read again when
    /// they changed, the corrections they hold taken, its agents followed in them at once.
    pub(crate) fn sync_workflow_files(&self, project_id: &str) {
        let Ok(project) = self.project(project_id) else {
            return;
        };
        if !project.workflows_enabled {
            return;
        }
        let folder =
            crate::workflowfiles::load_all(&crate::workflowfiles::places(&project, &self.data));
        let fingerprint = folder.fingerprint();
        let known = FILES.lock().get(project_id).map(|s| s.0.clone());
        if known.as_deref() == Some(fingerprint.as_str())
            || (known.is_none() && folder.texts.is_empty())
        {
            return;
        }
        let local = self.cached_reading(project_id);
        self.scan_from_files(&project, &folder, &local);
        self.workflow_fixes_changed(project_id);
    }

    /// « Retirer du dépôt »: the file of a workflow goes, and is not written again (ESC-309).
    pub async fn remove_workflow_file(
        &self,
        project_id: &str,
        workflow: &str,
    ) -> anyhow::Result<Scan> {
        self.set_workflow_removed(project_id, workflow, true).await
    }

    /// « Supprimer » (`removed`): the workflow's file goes, and no reading writes it again; one
    /// removed already is kept removed as it is found now (« Laisser supprimé »). « Rétablir »
    /// (not `removed`): read and written again (ESC-309).
    pub async fn set_workflow_removed(
        &self,
        project_id: &str,
        workflow: &str,
        removed: bool,
    ) -> anyhow::Result<Scan> {
        let project = self.project(project_id)?;
        let file = crate::workflowfiles::removals_file(&self.data, project_id);
        let before = crate::workflowfiles::Removals::load(&file);
        let mut removals = before.clone();
        if removed {
            let folder =
                crate::workflowfiles::load_all(&crate::workflowfiles::places(&project, &self.data));
            let doc = folder.doc(workflow);
            if let Some((path, _)) = doc {
                std::fs::remove_file(path)?;
            }
            let reading = self.cached_reading(project_id);
            let now = reading.workflows.iter().find(|w| w.id == workflow);
            removals.remove(workflow, doc.map(|(_, d)| d), now);
            removals.save(&file, &before);
            if let Some(s) = FILES.lock().get_mut(project_id) {
                s.1.remove(workflow);
            }
            let scan = self.workflows_of(project_id);
            self.workflow_fixes_changed(project_id);
            return Ok(scan);
        }
        removals.removed.remove(workflow);
        removals.save(&file, &before);
        let scan = self.refresh_workflows(project_id, false).await?;
        self.workflow_fixes_changed(project_id);
        Ok(scan)
    }

    /// Every project's workflows looked at again (the app starting, the window back to the front):
    /// at most once every `ALL_EVERY`, never two at a time.
    pub fn refresh_all_workflows(self: &std::sync::Arc<Self>) {
        {
            let mut last = LAST_ALL.lock();
            if last.is_some_and(|at| at.elapsed() < ALL_EVERY) {
                return;
            }
            *last = Some(std::time::Instant::now());
        }
        let ids: Vec<String> = self
            .projects
            .read()
            .iter()
            .filter(|p| p.workflows_auto_read && p.workflows_enabled)
            .map(|p| p.id.clone())
            .collect();
        let core = self.clone();
        tauri::async_runtime::spawn(async move {
            let _one = ALL_RUNNING.lock().await;
            for id in ids {
                if let Err(e) = core.refresh_workflows(&id, false).await {
                    log::warn!("workflows of {id}: {e:#}");
                }
            }
        });
    }
}

/// What the checked list of a prompt loop (`path` from the project's folder) says now.
pub fn list_progress(project_root: &Path, path: &str, roots: &Roots) -> Option<Progress> {
    let mut ctx = Ctx {
        roots,
        repo_dir: project_root.to_path_buf(),
        repo: String::new(),
        sources: Vec::new(),
        ignored: BTreeSet::new(),
    };
    let text = ctx.read(path, &project_root.join(path))?;
    says_of(&text).checked
}

/// The most of a file an agent that refines is given, and of all of them (ESC-196).
const CAUSE_EACH: usize = 12_000;
const CAUSE_TOTAL: usize = 150_000;

/// The files the workflows stand on (their doors, steps and sub-agents, `path` from the project's
/// folder or `~/…`) with what they say, as the reader may read them (no secret, no conversation,
/// nothing outside the project and the harnesses' own folders): what an agent that refines is
/// given. Each is clipped, and the whole is bounded.
pub fn files_in_cause(
    project_root: &Path,
    workflows: &[Workflow],
    sources: &[Source],
    roots: &Roots,
) -> Vec<(String, String)> {
    // Only what this machine's own reading found (its commands, skills, sub-agents, prompts, what they
    // include, the instructions): a path that only a shared workflow file names is never read, for such a
    // file comes with a cloned repository.
    let found: BTreeSet<PathBuf> = sources
        .iter()
        .filter(|s| s.kind != "state")
        .filter_map(|s| Path::new(&s.path).canonicalize().ok())
        .collect();
    let mut paths: BTreeSet<String> = BTreeSet::new();
    for w in workflows {
        paths.extend(w.doors.iter().map(|d| d.path.clone()));
        for s in &w.steps {
            paths.insert(s.path.clone());
            paths.extend(s.agents.iter().filter_map(|a| a.path.clone()));
        }
    }
    let mut ctx = Ctx {
        roots,
        repo_dir: project_root.to_path_buf(),
        repo: String::new(),
        sources: Vec::new(),
        ignored: BTreeSet::new(),
    };
    let (mut out, mut total) = (Vec::new(), 0usize);
    for path in paths.into_iter().filter(|p| !p.is_empty()) {
        let Some(file) = ctx.resolve(&path, &project_root.join("x")) else {
            continue;
        };
        if !file.canonicalize().is_ok_and(|f| found.contains(&f)) {
            continue;
        }
        let Some(text) = ctx.read(&path, &file) else {
            continue;
        };
        let text = clip_chars(&text, CAUSE_EACH);
        if total + text.len() > CAUSE_TOTAL {
            break;
        }
        total += text.len();
        out.push((path, text));
    }
    out
}

/// `text` cut to at most `max` bytes, at a character.
fn clip_chars(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}
