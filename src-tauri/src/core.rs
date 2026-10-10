//! Application core: projects, agents and their Claude processes, git, usage, persistence.

use crate::agent::{AgentAlert, AgentHandle, AgentRt, Effects, NotifyKind};
use crate::board;
use crate::claude::{self, ClaudeProcess, SpawnOpts};
use crate::conv;
use crate::convsearch;
use crate::fsedit;
use crate::git::{self, GitService};
use crate::hub::Hub;
use crate::integrations;
use crate::isola;
use crate::job::JobUsage;
use crate::model::*;
use crate::notify;
use crate::paths::{self, DataDir};
use crate::pty::{PtyManager, ShellInfo};
use crate::resources;
use crate::stats::{AgentLabel, Labels, Stats, StatsView, TicketLabel};
use crate::testlaunch;
use crate::usage;
use crate::worktrees::{self, RunSuggestion, WorktreeSuggestion};
use anyhow::{anyhow, bail, Context, Result};
use parking_lot::{Mutex, RwLock};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Runtime, Wry};
use tokio::sync::mpsc;

/// The Claude process exited while starting; the exit handler recorded why in the conversation.
#[derive(Debug)]
pub struct StartupFailure;

impl std::fmt::Display for StartupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Claude Code n'a pas pu démarrer : voir le détail dans la conversation.")
    }
}

impl std::error::Error for StartupFailure {}

/// The tag that opens `NotOnBase::wire`: the frontend recognizes the refusal by it.
const NOT_ON_BASE: &str = "NOT_ON_BASE";

/// A merge refused because the project's folder is not on the branch the agent's worktree left
/// (its base): the merge would land on whatever is checked out. The user may let Escouade switch.
#[derive(Debug)]
pub struct NotOnBase {
    /// The branch the folder is on; empty on a detached HEAD.
    pub current: String,
    pub base: String,
}

impl NotOnBase {
    /// What the frontend gets: the tag and the current branch, which cannot hold a `:`.
    pub fn wire(&self) -> String {
        format!("{NOT_ON_BASE}:{}:{}", self.current, self.base)
    }
}

impl std::fmt::Display for NotOnBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.current.is_empty() {
            write!(
                f,
                "Le projet n'est sur aucune branche (HEAD détachée) : bascule sur « {} » avant de merger.",
                self.base
            )
        } else {
            write!(
                f,
                "Le projet est sur la branche « {} » : bascule sur « {} » avant de merger.",
                self.current, self.base
            )
        }
    }
}

impl std::error::Error for NotOnBase {}

/// A sync of a project's checkout with its remote, asked by the user.
#[derive(Debug, Clone, Copy)]
pub enum SyncOp {
    Fetch,
    Pull,
    Push,
}

/// A question to Claude with `claude -p` (`Core::ask_claude`).
struct Ask<'a> {
    /// As a timeout names it.
    who: &'a str,
    model: &'a str,
    /// The tools it may use (`--tools`), none when empty.
    tools: &'a str,
    cwd: &'a Path,
    system: &'a str,
    prompt: &'a str,
    limit: Duration,
}

/// "1/2 · npm ci": step `i` of a setup, as the agent shows it.
fn setup_label(steps: &[WorktreeStep], i: usize) -> String {
    format!(
        "{}/{} · {}",
        i + 1,
        steps.len(),
        worktrees::label(&steps[i])
    )
}

/// How often, at most, the window is sent the lines the step running of a worktree's setup wrote.
const SETUP_SEND_EVERY: Duration = Duration::from_millis(50);

/// The setup of an agent's new worktree, under way.
struct Setup {
    /// True once it is over, whichever way (its sender dropped: stopped).
    done: tokio::sync::watch::Receiver<bool>,
    task: tauri::async_runtime::JoinHandle<()>,
}

/// After the usage limit resets, before sending "continue" (or starting a ticket again): clocks
/// may differ a little.
pub(crate) const RESUME_MARGIN_MS: i64 = 30_000;

/// How often every repository with a remote is fetched in the background.
const FETCH_EVERY: Duration = Duration::from_secs(5 * 60);

/// A file attached to a message: `data` is base64 for images and PDFs, the text itself for
/// text files (`text/plain`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub name: String,
    pub media_type: String,
    pub data: String,
}

impl Attachment {
    fn is_image(&self) -> bool {
        self.media_type.starts_with("image/")
    }
}

/// The only image formats the API reads.
const IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];
const KB: usize = 1024;
const MB: usize = 1024 * KB;
/// Past this, a text file alone would fill Claude's context (~4 bytes a token).
const MAX_TEXT: usize = 256 * KB;
/// All the files of a message: sent in base64 (4/3 bigger), under the API's 32 MB a request.
const MAX_TOTAL: usize = 18 * MB;

fn size_label(bytes: usize) -> String {
    if bytes >= MB {
        format!("{} Mo", bytes / MB)
    } else {
        format!("{} Ko", bytes / KB)
    }
}

/// Content of a user message: its text alone, or the attached files as content blocks
/// followed by the text.
pub fn user_content(text: &str, attachments: &[Attachment]) -> Result<Value> {
    if attachments.is_empty() {
        return Ok(Value::String(text.to_string()));
    }
    let (mut blocks, sizes): (Vec<Value>, Vec<usize>) = attachments
        .iter()
        .map(attachment_block)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .unzip();
    if sizes.iter().sum::<usize>() > MAX_TOTAL {
        bail!(
            "Les fichiers joints à un message sont limités à {} en tout.",
            size_label(MAX_TOTAL)
        );
    }
    if !text.is_empty() {
        blocks.push(json!({ "type": "text", "text": text }));
    }
    Ok(Value::Array(blocks))
}

/// The content block of a file, and its size.
fn attachment_block(a: &Attachment) -> Result<(Value, usize)> {
    // Base64 carries 3 bytes in 4 characters, the padding none.
    let padding = a.data.bytes().rev().take_while(|&b| b == b'=').count();
    let decoded = (a.data.len() / 4 * 3).saturating_sub(padding);
    let (block, size, max) = match a.media_type.as_str() {
        t if IMAGE_TYPES.contains(&t) => (
            json!({ "type": "image", "source": { "type": "base64", "media_type": t, "data": a.data } }),
            decoded,
            5 * MB,
        ),
        "application/pdf" => (
            json!({
                "type": "document",
                "title": a.name,
                "source": { "type": "base64", "media_type": "application/pdf", "data": a.data },
            }),
            decoded,
            18 * MB,
        ),
        "text/plain" => (
            json!({
                "type": "document",
                "title": a.name,
                "source": { "type": "text", "media_type": "text/plain", "data": a.data },
            }),
            a.data.len(),
            MAX_TEXT,
        ),
        t => bail!(
            "« {} » ({t}) ne peut pas être joint : les fichiers acceptés sont les images (PNG, JPEG, GIF, WebP), les PDF et les fichiers texte.",
            a.name
        ),
    };
    if size > max {
        bail!("{} dépasse {}", a.name, size_label(max));
    }
    Ok((block, size))
}

/// How a new agent differs from the default one (`create_agent_with`).
#[derive(Debug, Clone, Default)]
pub struct AgentOptions {
    pub model: Option<String>,
    pub effort: Option<String>,
    pub mode: Option<String>,
    /// Already named: kept (suffixed when taken), never renamed by Haiku.
    pub name: Option<String>,
    /// A worktree on this new branch from this base, whatever the project's setting; the agent
    /// is not created without it.
    pub worktree: Option<(String, String)>,
    /// Appended to Claude Code's system prompt at each start.
    pub append_prompt: Option<String>,
    pub ticket_id: Option<String>,
    pub port_base: Option<u16>,
    /// Selected in the sidebar (the user created it).
    pub select: bool,
    /// A copy of this agent (`duplicate_agent`): named after it, and working where it does.
    pub copy_of: Option<CopyOf>,
}

/// What a copy takes of its original, read while no turn of the original ran.
#[derive(Debug, Clone)]
pub struct CopyOf {
    /// The copy is « <name> (copie) » (`copy_name`), never renamed by Haiku.
    pub name: String,
    /// The original's session, which the copy's first start forks.
    pub session_id: Option<String>,
    /// Its latest entry: where the copy's starts fork it.
    pub entry: Option<String>,
    /// The original's worktree: the copy gets one of its own, on a new branch from the commit it
    /// is on (none: the copy works in the same folder, whatever the project's setting).
    pub worktree: Option<Worktree>,
    /// The original's conversation, which the copy's starts with.
    pub items: Vec<Value>,
    /// What the copy's conversation tells after it (the original's changes left out).
    pub notice: Option<String>,
}

/// Work under way, counted in `Core::works` until dropped.
pub(crate) struct Working<'a>(&'a AtomicUsize);

impl Drop for Working<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub struct Core<R: Runtime = Wry> {
    /// Itself, for what runs in the background from a method that only has `&self`.
    me: std::sync::Weak<Self>,
    pub app: AppHandle<R>,
    pub data: DataDir,
    pub hub: Hub,
    pub settings: RwLock<Settings>,
    pub projects: RwLock<Vec<Project>>,
    /// Every project's tickets. Lock order: projects before tickets; never hold tickets while
    /// taking projects. Guards that end with their statement do not nest; mind the temporaries of
    /// a struct literal or a tail expression, which live until its end.
    pub tickets: RwLock<Vec<Ticket>>,
    pub ui: RwLock<UiState>,
    pub agents: RwLock<HashMap<String, AgentHandle>>,
    pub stats: Stats,
    pub usage: Mutex<UsageSnapshot>,
    /// Claude Code's models as it last reported them (the version each alias runs).
    pub models: RwLock<Vec<ModelInfo>>,
    pub git: GitService,
    pub git_cache: RwLock<HashMap<String, GitInfo>>,
    pub pty: PtyManager,
    spawn_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// Serializes agent creation so that concurrent creations get distinct names.
    create_lock: tokio::sync::Mutex<()>,
    conv_buffer: Mutex<HashMap<String, Vec<ConvOp>>>,
    conv_flush: tokio::sync::Notify,
    last_oauth_call: Mutex<Option<i64>>,
    resources: Mutex<resources::Sampler>,
    git_inflight: Mutex<std::collections::HashSet<String>>,
    /// One fetch, pull or push at a time per repository.
    sync_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    toplevels: Mutex<HashMap<String, Option<String>>>,
    dirty: AtomicBool,
    waiting: AtomicUsize,
    pub quitting: AtomicBool,
    /// Files left unsaved in the editor, as the window last said.
    pub unsaved: AtomicUsize,
    /// Work under way outside of an agent's turn that a restart would cut (`Core::working`): a
    /// ticket being started, a validation and its cleanup, a message being delivered, a merge.
    pub(crate) works: AtomicUsize,
    /// One scheduling pass of the board at a time.
    pub(crate) board_lock: tokio::sync::Mutex<()>,
    /// Agents whose turn the app's previous stop cut (active when saved): their tickets go on.
    pub(crate) cut_turns: Mutex<Vec<String>>,
    /// The GitHub CLI, when installed (pull requests).
    pub gh: RwLock<Option<PathBuf>>,
    /// The terminals of each agent's test launches.
    pub(crate) test_runs: Mutex<HashMap<String, Vec<String>>>,
    /// The setups of new worktrees under way, by agent.
    setups: Mutex<HashMap<String, Setup>>,
    /// The last pass found no Claude Code (logged once until it is found again).
    pub(crate) claude_missing: AtomicBool,
    /// Why no ticket of a project's board starts (its target branch has no commit yet, or is
    /// gone), by project, as the window is told: not saved, found again by the next pass.
    pub(crate) board_issues: Mutex<HashMap<String, String>>,
    /// What holds the autopilot back besides the quota windows read (the pause after a usage
    /// limit with no resume planned), or no longer does ("Reprendre maintenant"). Saved with the
    /// quota windows last read (`SavedPause`).
    pub(crate) hold: Mutex<Hold>,
    /// The autopilot's pause as the window was last told (`refresh_pause`).
    pub(crate) pause_shown: Mutex<Option<AutopilotPause>>,
    /// Blocks of ports reserved for agents being made, or being given one: taken until the agent
    /// holds its own.
    pub(crate) ports_reserved: Mutex<Vec<u16>>,
    /// One validation's merge at a time per repository (`merge_lock`).
    pub(crate) merge_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// The accounts of the external ticket systems (`integrations.json`), with their secrets.
    pub accounts: RwLock<integrations::Accounts>,
    /// Where the accounts' secrets are kept: the system's keychain (memory in tests).
    pub(crate) secrets: Arc<dyn integrations::secrets::SecretStore>,
    /// Where the Trello and GitHub APIs are.
    pub bases: RwLock<integrations::Bases>,
    /// The GitHub CLI's token, once asked for.
    pub(crate) gh_token: Mutex<Option<String>>,
    /// What the sync of imported tickets is asked to do (a change to tell, failed operations to
    /// try again), one at a time and in order, by a single worker (made at the first one).
    pub(crate) sync_queue: Mutex<Option<integrations::sync::SyncSender>>,
    /// The operations of imported tickets not through yet (`sync-queue.json`).
    pub(crate) pending_syncs: Mutex<integrations::sync::SyncQueue>,
    /// One import at a time (by hand or by label): a ticket comes in once.
    pub(crate) import_lock: tokio::sync::Mutex<()>,
    /// The searches through the conversations the window started (Ctrl+K).
    searches: convsearch::Searches,
    /// Notifications sent, as "<title> | <text>" (tests only).
    #[cfg(test)]
    pub alerts: Mutex<Vec<String>>,
    /// Syncs of external tickets queued and not over yet (tests only).
    #[cfg(test)]
    pub(crate) syncs_queued: AtomicUsize,
    /// What a look for `gh` on the PATH finds (tests only: never the machine's own).
    #[cfg(test)]
    pub gh_on_path: RwLock<Option<PathBuf>>,
    /// Scheduling passes asked for (`schedule`) and not over yet (tests only).
    #[cfg(test)]
    pub(crate) passes_queued: AtomicUsize,
    /// How far ahead of the real clock the autopilot's pauses and the retries of the syncs go by,
    /// in milliseconds (tests only: the end of a pause, or a retry's time, without waiting for
    /// it).
    #[cfg(test)]
    pub(crate) clock_ahead: std::sync::atomic::AtomicI64,
}

/// The GitHub CLI on the PATH. Tests never see the machine's own: there it is absent, unless a
/// test puts one in `Core::gh_on_path`.
pub(crate) fn locate_gh() -> Option<PathBuf> {
    if cfg!(test) {
        None
    } else {
        crate::which::find("gh")
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            log::error!("invalid {}: {e}", path.display());
            let _ = std::fs::copy(path, path.with_extension("broken.json"));
            None
        }
    }
}

const CLAUDE_BASE_ARGS: &[&str] = &[
    "--output-format",
    "stream-json",
    "--verbose",
    "--input-format",
    "stream-json",
    "--permission-prompt-tool",
    "stdio",
    "--include-partial-messages",
    // Messages sent from claude.ai (Remote Control) come back on stdout, so the app shows them.
    "--replay-user-messages",
    "--thinking-display",
    "summarized",
    "--allow-dangerously-skip-permissions",
];

pub fn supports_effort(model: &str) -> bool {
    !model.to_lowercase().contains("haiku")
}

pub(crate) fn claude_args(m: &AgentMeta) -> Vec<String> {
    let mut a: Vec<String> = CLAUDE_BASE_ARGS.iter().map(|s| s.to_string()).collect();
    a.extend(["--model".into(), m.model.clone()]);
    if supports_effort(&m.model) {
        a.extend(["--effort".into(), m.effort.clone()]);
    }
    a.extend(["--permission-mode".into(), m.mode.clone()]);
    if let Some(p) = m.append_prompt.as_deref().filter(|p| !p.trim().is_empty()) {
        // On one line: a `.cmd` launcher (npm's claude.cmd) takes no argument with line breaks.
        let one_line = p.split_whitespace().collect::<Vec<_>>().join(" ");
        a.extend(["--append-system-prompt".into(), one_line]);
    }
    if let Some(s) = &m.session_id {
        a.push(format!("--resume={s}"));
    } else if let Some(s) = &m.fork_of {
        // A copy's conversation so far is its original's, under a new session: the original's
        // is left as it is.
        a.extend([format!("--resume={s}"), "--fork-session".into()]);
        // As it was when copied: the turns the original ran since stay out.
        if let Some(at) = &m.fork_at {
            a.push(format!("--resume-session-at={at}"));
        }
    }
    a
}

/// The arguments as the log shows them: the protocol appended to the system prompt (up to several
/// KB of ticket text) is only counted, not copied.
fn args_for_log(args: &[String]) -> String {
    let mut shown: Vec<String> = Vec::with_capacity(args.len());
    let mut elide = false;
    for a in args {
        if std::mem::take(&mut elide) {
            shown.push(format!("<{} chars>", a.chars().count()));
            continue;
        }
        elide = a == "--append-system-prompt";
        shown.push(a.clone());
    }
    shown.join(" ")
}

/// Kebab-case ASCII slug from free text (accents folded), of at most 40 characters.
pub fn slugify(s: &str) -> String {
    slug_within(s, 40)
}

/// Kebab-case ASCII slug from free text (accents folded), of at most `max` characters: whole
/// words while they fit, the first one cut when it alone is longer.
fn slug_within(s: &str, max: usize) -> String {
    let folded: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ÿ' => 'y',
            'ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    let mut out = String::new();
    for part in folded.split('-').filter(|p| !p.is_empty()) {
        if out.len() + part.len() + 1 > max {
            if out.is_empty() {
                // ASCII only by now: any byte ends a character.
                out.push_str(&part[..max.min(part.len())]);
            }
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(part);
    }
    out
}

/// The name of a copy of the agent `original`: « <original> (copie) », then « (copie 2) »,
/// « (copie 3) »… while the project has an agent of that name.
pub(crate) fn copy_name(original: &str, taken: &[String]) -> String {
    let mut name = format!("{original} (copie)");
    let mut n = 2;
    while taken.contains(&name) {
        name = format!("{original} (copie {n})");
        n += 1;
    }
    name
}

/// The branch of `copy`, the copy of the agent `original` that `copy_name` named:
/// `escouade/<original as a slug>-copie` (`-copie-2`…), the original's part shortened so that the
/// marker always fits in a slug's 40 characters.
fn copy_branch(original: &str, copy: &str) -> String {
    let marker = slugify(copy.strip_prefix(original).unwrap_or(copy));
    let base = slug_within(original, 40usize.saturating_sub(marker.len() + 1));
    let slug = if base.is_empty() {
        marker
    } else {
        format!("{base}-{marker}")
    };
    format!("{}{slug}", paths::BRANCH_PREFIX)
}

/// The worktree of a copy of the agent whose worktree is `wt`, in the repository `repo`: on the
/// new branch `branch` (`copy_branch`) from the commit `wt` is on (its branch's, unless the agent
/// switched it), toward the same base. Returns (path, branch, base branch).
async fn copy_worktree(
    repo: &str,
    wt: &Worktree,
    branch: &str,
) -> Result<(String, String, String)> {
    let start = git::text(&wt.path, &["rev-parse", "HEAD"]).await?;
    let (path, branch) = git::worktree_add_on(repo, branch, &start).await?;
    Ok((path, branch, wt.base_branch.clone()))
}

/// Instructions + the task framed as text to name (so that the model does not try to do it).
pub(crate) fn naming_prompt(task: &str) -> String {
    format!(
        "Donne un nom court à la tâche de développement ci-dessous. Ne la réalise pas.\n\
         Réponds uniquement par un slug kebab-case de 2 ou 3 mots (minuscules ASCII, sans accents), \
         par exemple refacto-auth ou tests-e2e.\n\n<tache>\n{}\n</tache>",
        claude::truncate(task.trim(), 2000)
    )
}

/// The model's answer as an agent name, if it looks like one: a sentence (an attempt at the
/// task, a refusal, "Voici le slug : …") is never a name.
pub(crate) fn name_from_answer(raw: &str) -> Option<String> {
    let line = raw
        .trim()
        .lines()
        .next()?
        .trim()
        .trim_matches(|c| matches!(c, '`' | '"' | '\'' | '*'));
    if line.is_empty() || line.contains(':') || line.ends_with(['.', '!', '?']) {
        return None;
    }
    let words = line
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|w| !w.is_empty())
        .count();
    if words > 4 {
        return None;
    }
    let slug = slugify(line);
    (!slug.is_empty()).then_some(slug)
}

/// Haiku's role when it proposes the message of a direct commit.
pub(crate) const COMMIT_PROPOSAL_SYSTEM: &str = "Tu écris des messages de commit, sans jamais réaliser de tâche. Tu réponds uniquement par le message de commit.";

/// How much of a direct commit's diff Haiku reads (bytes), and of its files (the diff covers
/// those only): enough to tell what changed, within a quick answer.
pub(crate) const PROPOSAL_DIFF: usize = 60_000;
pub(crate) const PROPOSAL_FILES: usize = 200;

/// The latest commit subjects of the repository whose style a proposal imitates.
const RECENT_SUBJECTS: usize = 10;

/// What Haiku is asked for a direct commit's message: the changes, framed as text to describe, in
/// the style of the repository's latest `subjects` (newest first).
pub(crate) fn commit_proposal_prompt(
    subjects: &[String],
    files: &[FileChange],
    diff: &str,
) -> String {
    let style = if subjects.is_empty() {
        "Le dépôt n'a pas encore de commit : suis le format Conventional Commits (« type(portée): description »).".to_string()
    } else {
        format!(
            "Imite le style des derniers commits du dépôt ci-dessous : leur langue, leur format, leur longueur.\n\n\
             <sujets-recents>\n{}\n</sujets-recents>",
            subjects.join("\n")
        )
    };
    let mut listed: String = files
        .iter()
        .take(PROPOSAL_FILES)
        .map(|f| format!("{} {}\n", f.status, f.path))
        .collect();
    if files.len() > PROPOSAL_FILES {
        listed.push_str(&format!(
            "… et {} autres fichiers\n",
            files.len() - PROPOSAL_FILES
        ));
    }
    format!(
        "Écris le message de commit des modifications ci-dessous. Ne les réalise pas, ne les commente pas. \
         Réponds uniquement par le message : un sujet d'une ligne, puis, seulement si c'est utile, une ligne vide et un corps court.\n\n\
         {style}\n\n<fichiers>\n{listed}</fichiers>\n\n<diff>\n{}\n</diff>",
        claude::truncate(diff.trim_end(), PROPOSAL_DIFF)
    )
}

/// A direct commit asked for files that no longer have changes to commit.
const NOTHING_TO_COMMIT: &str = "Plus rien à commiter : ces fichiers n'ont plus de modification.";

/// What a direct commit works on: its checkout, its files, and those of them Haiku never reads.
struct DirectScope {
    root: String,
    scope: CommitScope,
    /// The changed files matching the copy's patterns (`.env*`…), committed or not: their content
    /// never goes into Haiku's prompt.
    unread: HashSet<String>,
}

/// Of the files a direct commit takes, those of `paths`: a path from the window counts only when
/// its checkout still lists it.
fn committed(scope: CommitScope, paths: &[String]) -> Vec<FileChange> {
    let asked: HashSet<&str> = paths.iter().map(String::as_str).collect();
    scope
        .files
        .into_iter()
        .filter(|f| asked.contains(f.path.as_str()))
        .collect()
}

/// Haiku's answer as a commit message: a fence around it (```, ```text) and quotes around a
/// one-line answer taken off. None when nothing is left.
pub(crate) fn proposal_from_answer(raw: &str) -> Option<String> {
    let mut lines: Vec<&str> = raw.trim().lines().map(str::trim_end).collect();
    if lines
        .first()
        .is_some_and(|l| l.trim_start().starts_with("```"))
    {
        lines.remove(0);
        if lines.last().is_some_and(|l| l.trim() == "```") {
            lines.pop();
        }
    }
    let text = lines.join("\n");
    let text = text.trim();
    let text = if text.contains('\n') {
        text
    } else {
        board::unquoted(text)
    };
    (!text.is_empty()).then(|| text.to_string())
}

fn sub_prefix(root: &str, sub: &str) -> String {
    let norm = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_lowercase();
    if norm(root) == norm(sub) {
        String::new()
    } else {
        let rel = paths::relative_slash(root, sub);
        if rel.contains(':') {
            String::new()
        } else {
            format!("{rel}/")
        }
    }
}

/// The folders of an editor source (`root`, an agent's `worktree` or else the checkout holding
/// the folder `project`) that hold the agents' worktrees, relative to it: the editor neither
/// renames nor deletes them, nor a folder holding one. The project's own are in its folder, below
/// the root when it is a subfolder of its repository; a validation's are at the root.
fn worktrees_kept(root: &str, project: &str, worktree: bool) -> Vec<String> {
    let mut kept = vec![fsedit::WORKTREES.to_string()];
    let below = if worktree {
        String::new()
    } else {
        sub_prefix(root, project)
    };
    if !below.is_empty() {
        kept.push(format!("{below}{}", fsedit::WORKTREES));
    }
    kept
}

/// `path` from `root` when it is strictly inside it (case, separators and links seen through), as
/// the editor's paths are; None for the root itself or a folder elsewhere.
fn below_root(root: &str, path: &str) -> Option<String> {
    let rel = paths::relative_slash(root, path);
    let elsewhere = rel.is_empty() || rel.contains(':') || Path::new(&rel).is_absolute();
    (!elsewhere && !rel.split('/').any(|p| p == "..")).then_some(rel)
}

impl<R: Runtime> Core<R> {
    pub fn load(app: AppHandle<R>, data: DataDir) -> (Arc<Self>, mpsc::UnboundedReceiver<String>) {
        if let Err(e) = data.ensure() {
            log::error!("cannot create data dir: {e}");
        }
        let settings: Settings = read_json(&data.settings_file()).unwrap_or_default();
        let state: PersistedState = read_json(&data.state_file()).unwrap_or_default();
        // Read before the agents are made: a turn does not survive a restart (they come back done).
        let cut_turns: Vec<String> = state
            .agents
            .iter()
            .filter(|m| m.status.is_active() && !m.archived)
            .map(|m| m.id.clone())
            .collect();
        let mut tickets = state.tickets;
        for t in &mut tickets {
            // A validation the app's stop cut: to run again by hand.
            if t.step.take().is_some() {
                t.blocked = Some("Validation interrompue".into());
                t.conflict = false;
            }
        }
        let conv_dir = data.conversations();
        let agents = state
            .agents
            .into_iter()
            .map(|m| {
                (
                    m.id.clone(),
                    Arc::new(Mutex::new(AgentRt::new(m, &conv_dir))),
                )
            })
            .collect();
        let secrets = integrations::secrets::store_for(&data);
        let accounts = integrations::secrets::load_accounts(
            &data,
            &*secrets,
            integrations::secrets::file_keeps_copy(),
        );
        let pending_syncs = read_json(&data.sync_queue_file()).unwrap_or_default();
        // The autopilot's pause as the app stopped, for the first pass (before any new reading of
        // the quotas, which with an API key never comes): a window read holds until its end, an
        // ended one is dropped (the status bar would show it).
        let saved = state.pause;
        let now = now_ms();
        let ongoing = |w: Option<RateWindow>| {
            w.filter(|w| w.resets_at.is_some_and(|end| end + RESUME_MARGIN_MS > now))
        };
        let usage = UsageSnapshot {
            five_hour: ongoing(saved.five_hour),
            seven_day: ongoing(saved.seven_day),
            ..Default::default()
        };
        let (git, rx) = GitService::new();
        let core = Arc::new_cyclic(|me| Self {
            me: me.clone(),
            stats: Stats::open(&data.stats_db()),
            app,
            data,
            hub: Hub::default(),
            settings: RwLock::new(settings),
            projects: RwLock::new(state.projects),
            tickets: RwLock::new(tickets),
            ui: RwLock::new(state.ui),
            agents: RwLock::new(agents),
            usage: Mutex::new(usage),
            models: RwLock::new(state.models),
            git,
            git_cache: RwLock::default(),
            pty: PtyManager::default(),
            spawn_locks: Mutex::default(),
            create_lock: tokio::sync::Mutex::new(()),
            conv_buffer: Mutex::default(),
            conv_flush: tokio::sync::Notify::new(),
            last_oauth_call: Mutex::new(None),
            resources: Mutex::default(),
            git_inflight: Mutex::default(),
            sync_locks: Mutex::default(),
            toplevels: Mutex::default(),
            dirty: AtomicBool::new(false),
            waiting: AtomicUsize::new(usize::MAX),
            quitting: AtomicBool::new(false),
            unsaved: AtomicUsize::new(0),
            works: AtomicUsize::new(0),
            board_lock: tokio::sync::Mutex::new(()),
            cut_turns: Mutex::new(cut_turns),
            gh: RwLock::new(locate_gh()),
            test_runs: Mutex::default(),
            setups: Mutex::default(),
            claude_missing: AtomicBool::new(false),
            board_issues: Mutex::default(),
            hold: Mutex::new(saved.hold),
            pause_shown: Mutex::default(),
            ports_reserved: Mutex::default(),
            merge_locks: Mutex::default(),
            accounts: RwLock::new(accounts),
            secrets,
            bases: RwLock::new(integrations::Bases::from_env()),
            gh_token: Mutex::default(),
            sync_queue: Mutex::default(),
            pending_syncs: Mutex::new(integrations::sync::SyncQueue::new(pending_syncs)),
            import_lock: tokio::sync::Mutex::new(()),
            searches: convsearch::Searches::default(),
            #[cfg(test)]
            alerts: Mutex::default(),
            #[cfg(test)]
            syncs_queued: AtomicUsize::new(0),
            #[cfg(test)]
            gh_on_path: RwLock::default(),
            #[cfg(test)]
            passes_queued: AtomicUsize::new(0),
            #[cfg(test)]
            clock_ahead: std::sync::atomic::AtomicI64::new(0),
        });
        core.usage.lock().today_cost = core.stats.today_cost();
        (core, rx)
    }

    pub fn start(self: &Arc<Self>, git_rx: mpsc::UnboundedReceiver<String>) {
        for p in self.projects.read().iter() {
            self.git.watch(&p.id, &p.path);
        }
        let c = self.clone();
        tauri::async_runtime::spawn(async move { c.git_loop(git_rx).await });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                c.conv_flush.notified().await;
                tokio::time::sleep(Duration::from_millis(16)).await;
                c.flush_conv();
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(400)).await;
                if c.dirty.swap(false, Ordering::AcqRel) {
                    c.save_now();
                }
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            loop {
                c.refresh_usage().await;
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                c.stop_idle_processes();
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(20)).await;
                c.resume_due().await;
                // The autopilot's pause ends by itself: a window's end, its 30 minutes after a limit.
                c.pause_tick();
                // A failed sync of an imported ticket, once its wait is over.
                c.retry_due_syncs();
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut shown = false;
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let resources = c.sample_resources();
                // Once more when the last process stops, then quiet until one runs.
                if resources.instances > 0 || shown {
                    shown = resources.instances > 0;
                    c.hub.emit(UiEvent::Resources { resources });
                }
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            // Shortly after startup, then regularly: the commits to pull show up by themselves.
            tokio::time::sleep(Duration::from_secs(15)).await;
            loop {
                c.fetch_all().await;
                tokio::time::sleep(FETCH_EVERY).await;
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            // The automatic import, once on: shortly after, then every so many minutes.
            let mut last: Option<std::time::Instant> = None;
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let s = c.settings.read().integrations.clone();
                let every = Duration::from_secs(u64::from(s.import_every.clamp(1, 24 * 60)) * 60);
                if s.auto_import && last.is_none_or(|l| l.elapsed() >= every) {
                    last = Some(std::time::Instant::now());
                    c.auto_import().await;
                }
            }
        });
        self.start_remote_agents();
        // The syncs the app's last run left waiting go again now, before those of the tickets
        // that go on.
        self.retry_all_syncs();
        self.recover_tickets();
        self.update_tray();
    }

    // ---------- persistence ----------

    fn snapshot(&self) -> PersistedState {
        let mut agents: Vec<AgentMeta> = self
            .agents
            .read()
            .values()
            .map(|h| h.lock().meta.clone())
            .collect();
        agents.sort_by_key(|m| m.created_at);
        // One lock at a time (each guard ends with its statement).
        let projects = self.projects.read().clone();
        let ui = self.ui.read().clone();
        let models = self.models.read().clone();
        let tickets = self.tickets.read().clone();
        let (five_hour, seven_day) = {
            let u = self.usage.lock();
            (u.five_hour, u.seven_day)
        };
        let hold = *self.hold.lock();
        PersistedState {
            projects,
            agents,
            ui,
            models,
            tickets,
            pause: SavedPause {
                five_hour,
                seven_day,
                hold,
            },
        }
    }

    pub fn save_now(&self) {
        let state = self.snapshot();
        match serde_json::to_vec_pretty(&state) {
            Ok(bytes) => {
                if let Err(e) = paths::write_atomic(&self.data.state_file(), &bytes) {
                    log::error!("cannot save state: {e}");
                }
            }
            Err(e) => log::error!("cannot serialize state: {e}"),
        }
    }

    pub fn request_save(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    pub fn save_settings(self: &Arc<Self>, s: Settings) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&s)?;
        paths::write_atomic(&self.data.settings_file(), &bytes)?;
        let auto_resume = s.auto_resume;
        *self.settings.write() = s;
        if !auto_resume {
            // Turned off: the resumes already planned go too.
            let dropped: Vec<AgentView> = self
                .agents
                .read()
                .values()
                .filter_map(|h| {
                    let mut rt = h.lock();
                    rt.meta.resume_at.take()?;
                    Some(rt.view())
                })
                .collect();
            if !dropped.is_empty() {
                for agent in dropped {
                    let id = agent.meta.id.clone();
                    self.hub.emit(UiEvent::Agent { agent });
                    // Its ticket would wait for a resume that no longer comes.
                    self.resume_lost(&id);
                }
                self.request_save();
            }
        }
        // Claude Code may be found now, or no agent waits for its quota any more: the board goes on.
        self.schedule();
        Ok(())
    }

    // ---------- lookups ----------

    /// Itself, without keeping it alive (for a worker that lives as long as it does).
    pub(crate) fn weak(&self) -> std::sync::Weak<Self> {
        self.me.clone()
    }

    pub fn agent(&self, id: &str) -> Result<AgentHandle> {
        self.agents
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("agent introuvable"))
    }

    pub fn project(&self, id: &str) -> Result<Project> {
        self.projects
            .read()
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| anyhow!("projet introuvable"))
    }

    pub fn agent_views(&self) -> Vec<AgentView> {
        let mut v: Vec<AgentView> = self
            .agents
            .read()
            .values()
            .map(|h| h.lock().view())
            .collect();
        v.sort_by_key(|a| a.meta.created_at);
        v
    }

    /// Searches the agents' conversations, those of one project or of all, archived agents
    /// included or not. Blocking: up to `convsearch::TIME_LIMIT` of reading, less when the
    /// window starts another search meanwhile.
    pub fn search_conversations(
        &self,
        text: &str,
        project_id: Option<&str>,
        archived: bool,
    ) -> convsearch::Found {
        let stale = self.searches.start();
        let agents: Vec<convsearch::AgentRef> = self
            .agents
            .read()
            .values()
            .map(|h| {
                let rt = h.lock();
                let m = &rt.meta;
                convsearch::AgentRef {
                    id: m.id.clone(),
                    project_id: m.project_id.clone(),
                    name: m.name.clone(),
                    archived: m.archived,
                    cwd: m.cwd.clone(),
                    last_activity: m.last_activity,
                    created_at: m.created_at,
                }
            })
            .collect();
        let q = convsearch::Query {
            text,
            project_id,
            archived,
        };
        let deadline = Instant::now() + convsearch::TIME_LIMIT;
        convsearch::search(&self.data.conversations(), &agents, &q, deadline, &stale)
    }

    /// The statistics of a period, with the agents and tickets named. Every agent counts, archived
    /// ones too: they keep their ticket, so a ticket sent back and taken up by another agent adds
    /// up both.
    pub fn stats_view(&self, range: &str) -> StatsView {
        let agents = self
            .agents
            .read()
            .values()
            .map(|h| {
                let rt = h.lock();
                let m = &rt.meta;
                let label = AgentLabel {
                    name: m.name.clone(),
                    ticket_id: m.ticket_id.clone(),
                };
                (m.id.clone(), label)
            })
            .collect();
        // Taken after the agents are released: neither lock is held with the other.
        let tickets = self
            .tickets
            .read()
            .iter()
            .map(|t| {
                let label = TicketLabel {
                    key: t.key.clone(),
                    title: t.title.clone(),
                    // As the card of a finished ticket counts them.
                    loops: if t.loops > 0 { t.loops } else { t.iteration },
                };
                (t.id.clone(), label)
            })
            .collect();
        self.stats.query(range, &Labels { agents, tickets })
    }

    pub(crate) fn project_agents(&self, project_id: &str) -> Vec<AgentHandle> {
        self.agents
            .read()
            .values()
            .filter(|h| h.lock().meta.project_id == project_id)
            .cloned()
            .collect()
    }

    /// Late work on a deleted agent (naming, remote control…) must not bring it back in the UI.
    fn is_registered(&self, id: &str) -> bool {
        self.agents.read().contains_key(id)
    }

    pub(crate) fn emit_agent(&self, h: &AgentHandle) {
        let view = h.lock().view();
        if !self.is_registered(&view.meta.id) {
            return;
        }
        self.hub.emit(UiEvent::Agent { agent: view });
        self.update_tray();
    }

    fn spawn_lock(&self, id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.spawn_locks
            .lock()
            .entry(id.to_string())
            .or_default()
            .clone()
    }

    pub(crate) async fn toplevel(&self, path: &str) -> Option<String> {
        if let Some(t) = self.toplevels.lock().get(path) {
            return t.clone();
        }
        let t = git::toplevel(path).await;
        self.toplevels.lock().insert(path.to_string(), t.clone());
        t
    }

    // ---------- effects of agent activity ----------

    /// Streaming deltas are buffered and sent at most every 16 ms (merged); any other op first
    /// flushes the agent's buffered deltas so the UI sees every op in order.
    fn emit_conv(&self, agent_id: &str, ops: Vec<ConvOp>) {
        let only_deltas = ops.iter().all(|o| matches!(o, ConvOp::Delta { .. }));
        let mut buf = self.conv_buffer.lock();
        let was_empty = buf.is_empty();
        let pending = buf.entry(agent_id.to_string()).or_default();
        pending.extend(ops);
        if only_deltas {
            if was_empty {
                self.conv_flush.notify_one();
            }
            return;
        }
        let ops = coalesce_ops(std::mem::take(pending));
        buf.remove(agent_id);
        drop(buf);
        self.hub.emit(UiEvent::Conv {
            agent_id: agent_id.to_string(),
            ops,
        });
    }

    /// Sends every buffered delta now.
    pub fn flush_conv(&self) {
        let drained: Vec<(String, Vec<ConvOp>)> = self.conv_buffer.lock().drain().collect();
        for (agent_id, ops) in drained {
            self.hub.emit(UiEvent::Conv {
                agent_id,
                ops: coalesce_ops(ops),
            });
        }
    }

    fn apply(
        self: &Arc<Self>,
        id: &str,
        project_id: &str,
        name: &str,
        fx: Effects,
        view: Option<AgentView>,
    ) {
        if !fx.ops.is_empty() {
            self.emit_conv(id, fx.ops);
        }
        let changed = view.is_some();
        if let Some(v) = view.filter(|_| self.is_registered(id)) {
            self.hub.emit(UiEvent::Agent { agent: v });
        }
        if !fx.turns.is_empty() {
            self.stats.record_turns(id, project_id, &fx.turns);
            let mut u = self.usage.lock();
            u.today_cost = self.stats.today_cost();
            self.hub.emit(UiEvent::Usage { usage: u.clone() });
        }
        if let Some((five, week)) = fx.rate {
            {
                let mut u = self.usage.lock();
                if five.is_some() {
                    u.five_hour = five;
                }
                if week.is_some() {
                    u.seven_day = week;
                }
                u.updated_at = now_ms();
                self.hub.emit(UiEvent::Usage { usage: u.clone() });
            }
            // The autopilot pauses (or goes on) as the quotas just read say.
            self.pause_tick();
        }
        if let Some(resets_at) = fx.limited {
            self.plan_resume(id, resets_at);
        }
        if fx.files_changed {
            self.git.refresh(project_id);
        }
        if fx.save {
            self.request_save();
        }
        if changed {
            self.update_tray();
        }
        if let Some(alert) = fx.notify {
            // A ticket's agent at work: its board tells when the ticket is ready or blocked; its
            // questions still call for the user.
            if alert.kind == NotifyKind::Question || !self.ticket_doing(id) {
                self.notify_agent(alert, project_id, id, name);
            }
        }
        if let Some(end) = fx.turn_end {
            self.on_turn_end(id, end);
        }
    }

    fn on_frame(self: &Arc<Self>, h: &AgentHandle, gen: u64, frame: Value) {
        let mut fx = Effects::default();
        let (id, pid, name, view) = {
            let mut rt = h.lock();
            if rt.gen != gen {
                return;
            }
            rt.handle_frame(&frame, &mut fx);
            let view = fx.agent_changed.then(|| rt.view());
            (
                rt.meta.id.clone(),
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                view,
            )
        };
        self.apply(&id, &pid, &name, fx, view);
    }

    fn on_exit(self: &Arc<Self>, h: &AgentHandle, gen: u64, code: Option<i32>, stderr: String) {
        let mut fx = Effects::default();
        let (id, pid, name, view) = {
            let mut rt = h.lock();
            let current = rt.gen == gen;
            log::info!(
                "agent {}: claude exited (code {code:?}, current process: {current})",
                rt.meta.id
            );
            if !current {
                // A replaced, stopped or deleted agent's process: nothing to report.
                return;
            }
            // Killed by the app's stop: the turn it cut is no failure of the agent, and is to be
            // saved as running, so that the next start has its ticket go on (`cut_turns`).
            if self.quitting.load(Ordering::Acquire) {
                return;
            }
            rt.on_exit(gen, code, &stderr, &mut fx);
            (
                rt.meta.id.clone(),
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.view(),
            )
        };
        if self.quitting.load(Ordering::Acquire) {
            return;
        }
        self.apply(&id, &pid, &name, fx, Some(view));
    }

    /// Chime, then, out of sight, the taskbar flashes and a system notification shows; a click on
    /// it brings the window back and sends it `focus`.
    ///
    /// A kind the user switched off ("Me prévenir pour") neither chimes nor shows a system
    /// notification; the taskbar flashes all the same, like the agent's tab and card.
    pub(crate) fn alert(
        self: &Arc<Self>,
        kind: NotifyKind,
        title: String,
        body: String,
        focus: UiEvent,
    ) {
        let settings = self.settings.read().clone();
        let wanted = settings.notify_for.allows(kind);
        #[cfg(test)]
        if wanted {
            self.alerts.lock().push(format!("{title} | {body}"));
        }
        if wanted && settings.sound {
            notify::play_chime();
        }
        if notify::window_attended(&self.app) {
            return;
        }
        notify::flash(&self.app);
        if wanted && settings.os_notifications {
            let (app, weak) = (self.app.clone(), Arc::downgrade(self));
            notify::toast(&self.app, &title, &body, move || {
                notify::show_main(&app);
                if let Some(c) = weak.upgrade() {
                    c.hub.emit(focus.clone());
                }
            });
        }
    }

    fn notify_agent(
        self: &Arc<Self>,
        alert: AgentAlert,
        project_id: &str,
        agent_id: &str,
        agent_name: &str,
    ) {
        let project = self.project(project_id).map(|p| p.name).unwrap_or_default();
        self.alert(
            alert.kind,
            format!("{project} · {agent_name}"),
            alert.body,
            UiEvent::Focus {
                project_id: project_id.to_string(),
                agent_id: Some(agent_id.to_string()),
            },
        );
    }

    pub fn update_tray(&self) {
        let n = self
            .agents
            .read()
            .values()
            .filter(|h| h.lock().meta.status == AgentStatus::Waiting)
            .count();
        if self.waiting.swap(n, Ordering::AcqRel) == n {
            return;
        }
        if let Some(tray) = self.app.tray_by_id("main") {
            let _ = tray.set_icon(notify::tray_icon(&self.app, n));
            let tip = match n {
                0 => "Escouade".to_string(),
                1 => "Escouade — 1 agent en attente".to_string(),
                n => format!("Escouade — {n} agents en attente"),
            };
            let _ = tray.set_tooltip(Some(tip));
        }
    }

    // ---------- claude processes ----------

    /// Returns the agent's live process, starting it (with --resume) when needed.
    pub async fn ensure_process(self: &Arc<Self>, id: &str) -> Result<Arc<ClaudeProcess>> {
        // What a start resumes: its session, or a copy's original's, from where it was copied.
        let resumes = |m: &AgentMeta| (m.session_id.clone(), m.fork_of.clone(), m.fork_at.clone());
        let before = resumes(&self.agent(id)?.lock().meta);
        match self.start_process(id).await {
            // What it was to resume could not be (the session, or a copy's point in its
            // original's): its exit handler dropped it, so a second start goes on without it.
            Err(e)
                if e.is::<StartupFailure>() && resumes(&self.agent(id)?.lock().meta) != before =>
            {
                self.start_process(id).await
            }
            other => other,
        }
    }

    async fn start_process(self: &Arc<Self>, id: &str) -> Result<Arc<ClaudeProcess>> {
        let h = self.agent(id)?;
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        if let Some(p) = h.lock().proc.clone() {
            if p.is_alive() {
                return Ok(p);
            }
        }
        let settings = self.settings.read().clone();
        let program = claude::resolve_binary(&settings.claude_path).ok_or_else(|| {
            anyhow!("Claude Code introuvable. Installe-le ou indique son chemin dans les réglages.")
        })?;
        let (opts, gen) = {
            let mut rt = h.lock();
            rt.gen += 1;
            let opts = SpawnOpts {
                program,
                cwd: rt.meta.cwd.clone(),
                args: claude_args(&rt.meta),
                env: settings.claude_env(),
            };
            (opts, rt.gen)
        };
        if !Path::new(&opts.cwd).is_dir() {
            bail!("Le dossier {} n'existe plus", opts.cwd);
        }
        log::info!(
            "agent {id}: starting {} {} in {}",
            opts.program.display(),
            args_for_log(&opts.args),
            opts.cwd
        );
        let started = std::time::Instant::now();
        let (w1, w2) = (Arc::downgrade(self), Arc::downgrade(self));
        let (h1, h2) = (h.clone(), h.clone());
        let proc = ClaudeProcess::spawn(
            opts,
            move |frame| {
                if let Some(c) = w1.upgrade() {
                    c.on_frame(&h1, gen, frame);
                }
            },
            move |code, stderr| {
                if let Some(c) = w2.upgrade() {
                    c.on_exit(&h2, gen, code, stderr);
                }
            },
        )?;
        h.lock().attach(proc.clone());
        self.emit_agent(&h);
        match proc
            .control(json!({ "subtype": "initialize" }), Duration::from_secs(90))
            .await
        {
            Ok(resp) => {
                log::info!("agent {id}: ready in {} ms", started.elapsed().as_millis());
                h.lock().commands = resp["commands"].as_array().cloned().unwrap_or_default();
                self.set_models(&resp["models"]);
                if h.lock().meta.remote_control {
                    if let Err(e) = self.link_remote(&h, &proc).await {
                        log::warn!("agent {id}: remote control failed: {e:#}");
                        let _ = self.with_agent(id, |rt, fx| {
                            rt.notice("warn", format!("Remote control indisponible : {e}"), fx);
                            Ok(())
                        });
                    }
                }
            }
            Err(_) if !proc.is_alive() => {
                log::warn!("agent {id}: claude exited while starting");
                return Err(StartupFailure.into());
            }
            Err(e) => log::warn!(
                "agent {id}: initialize failed after {} ms: {e}",
                started.elapsed().as_millis()
            ),
        }
        Ok(proc)
    }

    /// Keeps the models Claude Code reported at a process start, for every label of the UI. What it
    /// no longer reports goes: better a bare "Sonnet" than a version it no longer runs.
    fn set_models(&self, reported: &Value) {
        let models = ModelInfo::list(reported);
        let mut current = self.models.write();
        if *current == models {
            return;
        }
        *current = models.clone();
        // Under the lock: two processes starting together leave the UI with what is kept.
        self.hub.emit(UiEvent::Models { models });
        self.request_save();
    }

    /// Starts the agent's process in the background so the first message answers fast.
    pub fn warm(self: &Arc<Self>, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        {
            let rt = h.lock();
            if rt.proc.is_some() || rt.meta.archived || rt.meta.status == AgentStatus::Error {
                return;
            }
        }
        let (c, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move {
            if let Err(e) = c.ensure_process(&id).await {
                log::warn!("warm-up failed: {e:#}");
            }
        });
    }

    // ---------- remote control ----------

    /// Name of the agent's session in claude.ai / the Claude app.
    fn remote_name(&self, meta: &AgentMeta) -> String {
        let project = self
            .project(&meta.project_id)
            .map(|p| p.name)
            .unwrap_or_default();
        format!("{project} · {}", meta.name)
    }

    /// Links the agent's live process to claude.ai (Remote Control). Its previous remote session
    /// is reattached, so the link opened on a phone stays valid across restarts.
    async fn link_remote(
        self: &Arc<Self>,
        h: &AgentHandle,
        proc: &Arc<ClaudeProcess>,
    ) -> Result<()> {
        let (name, reattach) = {
            let rt = h.lock();
            (self.remote_name(&rt.meta), rt.meta.remote_session.clone())
        };
        let request = |reattach: Option<&String>| {
            let mut r = json!({ "subtype": "remote_control", "enabled": true, "name": name, "keep_session_on_exit": true });
            if let Some(s) = reattach {
                r["reattach_session_id"] = json!(s);
            }
            r
        };
        let timeout = Duration::from_secs(30);
        let resp = match proc.control(request(reattach.as_ref()), timeout).await {
            Ok(r) => r,
            // The previous remote session is gone: open a new one.
            Err(_) if reattach.is_some() && proc.is_alive() => {
                proc.control(request(None), timeout).await?
            }
            Err(e) => return Err(e),
        };
        {
            let mut rt = h.lock();
            rt.remote_linked = true;
            rt.meta.remote_session = resp["bridge_session_id"].as_str().map(str::to_string);
            rt.meta.remote_url = resp["session_url"].as_str().map(str::to_string);
        }
        self.request_save();
        self.emit_agent(h);
        Ok(())
    }

    /// Turns Remote Control on (the agent's process starts if needed and stays up) or off.
    pub async fn set_remote_control(self: &Arc<Self>, id: &str, enabled: bool) -> Result<()> {
        let h = self.agent(id)?;
        h.lock().meta.remote_control = enabled;
        self.request_save();
        if enabled {
            let linked = match self.ensure_process(id).await {
                Ok(_) if h.lock().remote_linked => Ok(()),
                Ok(proc) => self.link_remote(&h, &proc).await,
                Err(e) => Err(e),
            };
            if let Err(e) = linked {
                h.lock().meta.remote_control = false;
                self.emit_agent(&h);
                return Err(e.context("Remote control indisponible"));
            }
        } else {
            let proc = {
                let mut rt = h.lock();
                let linked = std::mem::take(&mut rt.remote_linked);
                rt.meta.remote_session = None;
                rt.meta.remote_url = None;
                rt.remote_state = None;
                rt.proc.clone().filter(|_| linked)
            };
            if let Some(p) = proc {
                p.control(
                    json!({ "subtype": "remote_control", "enabled": false }),
                    Duration::from_secs(15),
                )
                .await?;
            }
        }
        self.emit_agent(&h);
        Ok(())
    }

    /// Starts the Remote Control agents with the app, so they are reachable from claude.ai.
    pub fn start_remote_agents(self: &Arc<Self>) {
        let ids: Vec<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                (rt.meta.remote_control && !rt.meta.archived).then(|| rt.meta.id.clone())
            })
            .collect();
        for id in ids {
            let c = self.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = c.ensure_process(&id).await {
                    log::warn!("agent {id}: remote control start failed: {e:#}");
                }
            });
        }
    }

    /// Stopped by the usage limit: plans to send the agent "continue" once the limit resets (as
    /// Claude Code told, else the saturated window's), unless turned off in the settings.
    pub fn plan_resume(self: &Arc<Self>, id: &str, resets_at: Option<i64>) {
        if !self.settings.read().auto_resume {
            return;
        }
        // Only a reset still to come: past one, retrying would only meet the limit again.
        let now = now_ms();
        let when = resets_at.filter(|t| *t > now).or_else(|| {
            let u = self.usage.lock();
            [u.five_hour.as_ref(), u.seven_day.as_ref()]
                .into_iter()
                .flatten()
                .filter(|w| w.pct >= 100.0)
                .filter_map(|w| w.resets_at)
                .filter(|t| *t > now)
                .max()
        });
        if let Some(when) = when {
            self.set_resume(id, Some(when + RESUME_MARGIN_MS));
        }
    }

    pub fn cancel_resume(self: &Arc<Self>, id: &str) -> Result<()> {
        self.agent(id)?;
        if self.set_resume(id, None).is_some() {
            // Its ticket would wait for a resume that no longer comes.
            self.resume_lost(id);
        }
        self.schedule();
        Ok(())
    }

    /// Plans (or drops) the agent's resume; returns the one planned before.
    fn set_resume(self: &Arc<Self>, id: &str, at: Option<i64>) -> Option<i64> {
        let h = self.agent(id).ok()?;
        let (before, view) = {
            let mut rt = h.lock();
            let before = std::mem::replace(&mut rt.meta.resume_at, at);
            (before, rt.view())
        };
        self.hub.emit(UiEvent::Agent { agent: view });
        self.request_save();
        before
    }

    /// Sends "continue" to the agents whose planned resume is due.
    pub async fn resume_due(self: &Arc<Self>) {
        if !self.settings.read().auto_resume {
            return;
        }
        let now = now_ms();
        // Taken under the agent's lock: a message or a cancel just before wins.
        let due: Vec<(String, AgentView)> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let mut rt = h.lock();
                let due = rt.meta.resume_at.is_some_and(|t| t <= now)
                    && !rt.meta.status.is_active()
                    && !rt.meta.archived;
                if !due {
                    return None;
                }
                rt.meta.resume_at = None;
                Some((rt.meta.id.clone(), rt.view()))
            })
            .collect();
        let resumed = !due.is_empty();
        for (id, view) in due {
            self.hub.emit(UiEvent::Agent { agent: view });
            self.request_save();
            let text = "continue".to_string();
            // A ticket's agent that cannot go on blocks its ticket, which would wait forever.
            let sent = match self.doing_ticket_of(&id) {
                Some(ticket_id) => self.send_or_block(&ticket_id, &id, text).await,
                None => self.send_message(&id, text, vec![]).await,
            };
            if let Err(e) = sent {
                log::warn!("agent {id}: resume after the usage limit failed: {e:#}");
                let _ = self.with_agent(&id, |rt, fx| {
                    rt.notice(
                        "warn",
                        format!("Reprise automatique impossible : {e:#}"),
                        fx,
                    );
                    Ok(())
                });
            }
        }
        // The quota pause is over.
        if resumed {
            self.schedule();
        }
    }

    /// The running Claude processes, with what they and everything they started use.
    pub fn sample_resources(&self) -> resources::Resources {
        let procs: Vec<(String, Arc<ClaudeProcess>)> = self
            .agents
            .read()
            .iter()
            .filter_map(|(id, h)| Some((id.clone(), h.lock().proc.clone()?)))
            .filter(|(_, p)| p.is_alive())
            .collect();
        let mut usages: Vec<(String, JobUsage)> = procs
            .into_iter()
            .filter_map(|(id, p)| Some((id, p.usage()?)))
            .collect();
        usages.sort_by(|a, b| a.0.cmp(&b.0));
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        self.resources.lock().sample(Instant::now(), cores, usages)
    }

    /// Stops the processes of agents idle for longer than the configured delay. The session
    /// and the conversation stay: the next action on the agent resumes it (--resume).
    pub(crate) fn stop_idle_processes(&self) {
        let minutes = self.settings.read().idle_stop_minutes;
        if minutes == 0 {
            return;
        }
        let limit = now_ms() - minutes as i64 * 60_000;
        let agents: Vec<AgentHandle> = self.agents.read().values().cloned().collect();
        for h in agents {
            let stopped = {
                let mut rt = h.lock();
                // A Remote Control agent must stay reachable from claude.ai.
                let idle = rt.proc.is_some()
                    && !rt.meta.remote_control
                    && !rt.meta.status.is_active()
                    && rt.meta.last_activity < limit;
                if idle {
                    rt.detach()
                } else {
                    None
                }
            };
            if let Some(p) = stopped {
                p.close_input();
                self.emit_agent(&h);
            }
        }
    }

    /// Work under way that an automatic restart for an update waits for, while the guard lives.
    pub(crate) fn working(&self) -> Working<'_> {
        self.works.fetch_add(1, Ordering::AcqRel);
        Working(&self.works)
    }

    /// A window that subscribes (started, or reloaded and so without the files it held) has no unsaved file yet.
    pub fn reset_unsaved(&self) {
        self.unsaved.store(0, Ordering::Release);
    }

    /// "Quitter" goes to the window first when the editor has unsaved files: true when it did.
    pub fn ask_before_quit(&self) -> bool {
        let unsaved = self.unsaved.load(Ordering::Acquire);
        if unsaved == 0 {
            return false;
        }
        self.hub.emit(UiEvent::QuitRequested { unsaved });
        true
    }

    pub fn shutdown(&self) {
        self.quitting.store(true, Ordering::Release);
        // Setups under way stop, with what they started.
        for (_, s) in self.setups.lock().drain() {
            s.task.abort();
        }
        for h in self.agents.read().values() {
            let mut rt = h.lock();
            if let Some(p) = rt.proc.take() {
                p.close_input();
                p.kill();
            }
        }
        self.pty.kill_all();
        self.save_now();
    }

    pub async fn send_message(
        self: &Arc<Self>,
        id: &str,
        text: String,
        attachments: Vec<Attachment>,
    ) -> Result<()> {
        // Until its turn runs (its process started, the message delivered), it is under way.
        let _working = self.working();
        // Refused before starting Claude: the composer keeps the message.
        let content = user_content(&text, &attachments)?;
        // Its new worktree is set up first: Claude would work in it meanwhile (an install running
        // twice, files half written).
        self.wait_setup(id).await;
        // Two attempts: the process may die between being started and receiving the message.
        for attempt in 0..2 {
            let proc = self.ensure_process(id).await?;
            if self.deliver(id, &proc, &text, &content, &attachments)? {
                return Ok(());
            }
            log::warn!("message not delivered (attempt {attempt}): the process exited");
        }
        bail!("Claude Code s'est arrêté pendant l'envoi du message : voir le détail dans la conversation.")
    }

    /// Sends the message to `proc` if it is still the agent's live process, then records it.
    fn deliver(
        self: &Arc<Self>,
        id: &str,
        proc: &Arc<ClaudeProcess>,
        text: &str,
        content: &Value,
        attachments: &[Attachment],
    ) -> Result<bool> {
        let h = self.agent(id)?;
        let mut fx = Effects::default();
        let (pid, name, view, first) = {
            let mut rt = h.lock();
            if !rt.proc.as_ref().is_some_and(|p| Arc::ptr_eq(p, proc)) || !proc.is_alive() {
                return Ok(false);
            }
            let uid = uuid::Uuid::new_v4().to_string();
            let frame = json!({ "type": "user", "message": { "role": "user", "content": content }, "parent_tool_use_id": null, "uuid": uid });
            if proc.send(&frame).is_err() {
                return Ok(false);
            }
            let (images, files): (Vec<_>, Vec<_>) = attachments.iter().partition(|a| a.is_image());
            let files: Vec<String> = files.into_iter().map(|a| a.name.clone()).collect();
            rt.push_user(&uid, text, images.len() as u32, &files, &mut fx);
            let first = !rt.meta.named && rt.meta.prompts == 1;
            (
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.view(),
                first,
            )
        };
        self.stats.record_prompt(id, &pid);
        self.apply(id, &pid, &name, fx, Some(view));
        if first && !text.trim().is_empty() && !text.trim_start().starts_with('/') {
            let (c, id, text) = (self.clone(), id.to_string(), text.to_string());
            tauri::async_runtime::spawn(async move { c.auto_name(&id, &text).await });
        }
        Ok(true)
    }

    pub async fn interrupt(self: &Arc<Self>, id: &str) -> Result<()> {
        let h = self.agent(id)?;
        let proc = {
            let mut rt = h.lock();
            // Only a running turn can be interrupted; a stale flag would hide the next turn's end.
            let active = rt.meta.status.is_active();
            rt.interrupted = active;
            rt.proc.clone().filter(|_| active)
        };
        if let Some(p) = proc {
            if let Err(e) = p
                .control(json!({ "subtype": "interrupt" }), Duration::from_secs(15))
                .await
            {
                h.lock().interrupted = false;
                return Err(e);
            }
        }
        Ok(())
    }

    fn with_agent<T>(
        self: &Arc<Self>,
        id: &str,
        f: impl FnOnce(&mut AgentRt, &mut Effects) -> Result<T>,
    ) -> Result<T> {
        let h = self.agent(id)?;
        let mut fx = Effects::default();
        let (out, pid, name, view) = {
            let mut rt = h.lock();
            let out = f(&mut rt, &mut fx)?;
            (
                out,
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.view(),
            )
        };
        self.apply(id, &pid, &name, fx, Some(view));
        Ok(out)
    }

    pub fn answer_question(
        self: &Arc<Self>,
        id: &str,
        request_id: &str,
        answers: Value,
    ) -> Result<()> {
        self.with_agent(id, |rt, fx| rt.answer_question(request_id, answers, fx))
    }

    pub fn answer_permission(
        self: &Arc<Self>,
        id: &str,
        request_id: &str,
        decision: &str,
        message: Option<String>,
    ) -> Result<()> {
        self.with_agent(id, |rt, fx| {
            rt.answer_permission(request_id, decision, message, fx)
        })
    }

    pub async fn set_agent_options(
        self: &Arc<Self>,
        id: &str,
        model: Option<String>,
        effort: Option<String>,
        mode: Option<String>,
    ) -> Result<()> {
        let h = self.agent(id)?;
        let proc = {
            let mut rt = h.lock();
            if let Some(m) = &model {
                rt.meta.model = m.clone();
            }
            if let Some(e) = &effort {
                rt.meta.effort = e.clone();
            }
            if let Some(m) = &mode {
                rt.meta.mode = m.clone();
            }
            rt.proc.clone()
        };
        self.emit_agent(&h);
        self.request_save();
        if let Some(p) = proc {
            let t = Duration::from_secs(15);
            if let Some(m) = model {
                p.control(json!({ "subtype": "set_model", "model": m }), t)
                    .await?;
            }
            let current_model = h.lock().meta.model.clone();
            if let Some(e) = effort.filter(|_| supports_effort(&current_model)) {
                p.control(
                    json!({ "subtype": "apply_flag_settings", "settings": { "effortLevel": e } }),
                    t,
                )
                .await?;
            }
            if let Some(m) = mode {
                p.control(json!({ "subtype": "set_permission_mode", "mode": m }), t)
                    .await?;
            }
        }
        Ok(())
    }

    // ---------- ports ----------

    /// Reserves a block of 10 ports (`board::allocate_ports`): the first that no agent holds, nor
    /// another reservation in flight, and whose ports all bind. Chosen and recorded under one lock,
    /// so two reservations never get the same block. `unreserve_ports` once the agent made for it
    /// holds it (or was not made).
    pub(crate) fn reserve_ports(&self) -> Option<u16> {
        let mut reserved = self.ports_reserved.lock();
        let mut taken: Vec<u16> = self
            .agents
            .read()
            .values()
            .filter_map(|h| h.lock().meta.port_base)
            .collect();
        taken.extend(reserved.iter().copied());
        let base = board::allocate_ports(&taken, testlaunch::port_free)?;
        reserved.push(base);
        Some(base)
    }

    pub(crate) fn unreserve_ports(&self, base: Option<u16>) {
        if let Some(base) = base {
            self.ports_reserved.lock().retain(|b| *b != base);
        }
    }

    // ---------- agents lifecycle ----------

    pub async fn create_agent(
        self: &Arc<Self>,
        project_id: &str,
        model: Option<String>,
    ) -> Result<AgentView> {
        self.create_agent_with(
            project_id,
            AgentOptions {
                model,
                select: true,
                ..Default::default()
            },
        )
        .await
    }

    /// A new agent of the project, as `o` says (see `AgentOptions`).
    pub async fn create_agent_with(
        self: &Arc<Self>,
        project_id: &str,
        o: AgentOptions,
    ) -> Result<AgentView> {
        // Its worktree is made and its files copied before the agent is one of the app's.
        let _working = self.working();
        let _creating = self.create_lock.lock().await;
        let project = self.project(project_id)?;
        let settings = self.settings.read().clone();
        let existing: Vec<String> = self
            .project_agents(project_id)
            .iter()
            .map(|h| h.lock().meta.name.clone())
            .collect();
        let name = match (&o.copy_of, &o.name) {
            (Some(c), _) => copy_name(&c.name, &existing),
            (None, Some(base)) => {
                let mut name = base.clone();
                let mut n = 2;
                while existing.contains(&name) {
                    name = format!("{base}-{n}");
                    n += 1;
                }
                name
            }
            (None, None) => {
                let mut n = existing.len() + 1;
                while existing.iter().any(|e| *e == format!("agent-{n}")) {
                    n += 1;
                }
                format!("agent-{n}")
            }
        };
        let mut meta = AgentMeta {
            id: new_id(),
            project_id: project_id.to_string(),
            name: name.clone(),
            named: o.name.is_some() || o.copy_of.is_some(),
            fork_of: o.copy_of.as_ref().and_then(|c| c.session_id.clone()),
            fork_at: o
                .copy_of
                .as_ref()
                .filter(|c| c.session_id.is_some())
                .and_then(|c| c.entry.clone()),
            model: o.model.unwrap_or(settings.default_model.clone()),
            effort: o.effort.unwrap_or(settings.default_effort.clone()),
            mode: o.mode.unwrap_or(settings.default_mode.clone()),
            cwd: project.path.clone(),
            created_at: now_ms(),
            last_activity: now_ms(),
            ticket_id: o.ticket_id,
            append_prompt: o.append_prompt,
            port_base: o.port_base,
            ..Default::default()
        };
        let conversations = self.data.conversations();
        // A copy's conversation is written first: when it cannot be, nothing was made for it.
        if let Some(c) = &o.copy_of {
            conv::write_log(&conversations, &meta.id, &c.items)
                .context("conversation non copiée")?;
        }
        // The original's worktree, for a copy of an agent that has one.
        let copied_from = o.copy_of.as_ref().and_then(|c| c.worktree.as_ref());
        let made = match (&o.worktree, &o.copy_of) {
            (Some((branch, base)), _) => Some(
                git::worktree_add_on(&project.path, branch, base)
                    .await
                    .map(|(path, branch)| (path, branch, base.clone())),
            ),
            // A copy works on its original's code: in a worktree of its own, or in the same folder.
            (None, Some(c)) => match copied_from {
                Some(wt) => {
                    let branch = copy_branch(&c.name, &name);
                    Some(copy_worktree(&project.path, wt, &branch).await)
                }
                None => None,
            },
            (None, None) if project.worktree_per_agent => {
                Some(git::worktree_add(&project.path, &name).await)
            }
            (None, None) => None,
        };
        let mut warning = None;
        match made {
            Some(Ok((path, branch, base))) => {
                if let Err(e) =
                    testlaunch::copy_worktree_files(&project.path, &path, &project.worktree_copy)
                        .await
                {
                    warning = Some(format!("Fichiers non copiés dans le worktree : {e:#}"));
                }
                if let Some(from) = copied_from {
                    // Its conversation names the original's folder everywhere (its files' absolute
                    // paths): its Claude is told at every start where it works now.
                    meta.append_prompt = Some(format!(
                        "Cette conversation a été copiée depuis un agent qui travaillait dans {}. Tu travailles maintenant dans {path} : ne lis et n'écris que dedans.",
                        from.path
                    ));
                }
                meta.cwd = path.clone();
                meta.worktree = Some(Worktree {
                    path,
                    branch,
                    base_branch: base,
                });
            }
            // A ticket's agent works in its own worktree or not at all.
            Some(Err(e)) if o.worktree.is_some() => {
                return Err(e.context("worktree du ticket non créé"))
            }
            // So does a copy of an agent that has one: the project's folder is not its code.
            Some(Err(e)) if o.copy_of.is_some() => {
                let _ = std::fs::remove_file(conv::log_path(&conversations, &meta.id));
                return Err(e.context("worktree de la copie non créé"));
            }
            Some(Err(e)) => {
                warning = Some(format!(
                    "Worktree non créé, l'agent travaille dans le dossier du projet : {e}"
                ))
            }
            None => {}
        }
        let id = meta.id.clone();
        let fresh = meta.worktree.clone();
        let h = Arc::new(Mutex::new(AgentRt::new(meta, &conversations)));
        // After the conversation a copy starts with.
        for text in warning.into_iter().chain(o.copy_of.and_then(|c| c.notice)) {
            let mut fx = Effects::default();
            h.lock().notice("warn", text, &mut fx);
        }
        self.agents.write().insert(id.clone(), h.clone());
        // Its new worktree is set up (dependencies…) before it takes a message.
        let setting_up = fresh
            .as_ref()
            .is_some_and(|wt| self.start_setup(&h, &project, wt));
        if o.select {
            self.ui
                .write()
                .selected_agent
                .insert(project_id.to_string(), id.clone());
        }
        self.request_save();
        self.emit_agent(&h);
        self.git.refresh(project_id);
        // Once set up, if it is (`run_setup`): its hooks and MCP servers would meet a worktree
        // half set up.
        if !setting_up {
            self.warm(&id);
        }
        let view = h.lock().view();
        Ok(view)
    }

    /// A copy of the agent (« Dupliquer la conversation »), refused during its turn: « <name>
    /// (copie) » in the same project, with its model, effort and permission mode, its conversation
    /// shown again, and its Claude Code session forked where the original is now (the turns it
    /// runs before the copy's first are not the copy's), the original's left as it is. It works in
    /// a worktree of its own from the commit the original's is on (what the original did not
    /// commit stays out, as its conversation says; its Claude is told of the new folder at every
    /// start), or in the same folder. A ticket's agent is copied as an ordinary one: no ticket,
    /// protocol nor ports.
    pub async fn duplicate_agent(self: &Arc<Self>, id: &str) -> Result<AgentView> {
        // The original's worktree is read, the copy's made, before the copy is one of the app's.
        let _working = self.working();
        // Read at once, the session with the conversation it holds.
        let (original, items) = {
            let h = self.agent(id)?;
            let mut rt = h.lock();
            if rt.meta.status.is_active() {
                bail!(
                    "Attends la fin du tour de {} pour le dupliquer.",
                    rt.meta.name
                );
            }
            (rt.meta.clone(), rt.conv.items())
        };
        // What its next start would resume: its own session where it is now, or, for a copy that
        // ran no turn yet, its original's where it was copied.
        let (session_id, entry) = match original.session_id {
            Some(own) => (Some(own), original.last_entry),
            None => (original.fork_of, original.fork_at),
        };
        let mut notice = None;
        if let Some(wt) = &original.worktree {
            match git::status(&wt.path).await {
                Ok(s) if !s.entries.is_empty() => {
                    notice = Some(format!(
                        "Les modifications non commitées de {} ne sont pas dans cette copie.",
                        original.name
                    ))
                }
                Ok(_) => {}
                // Its folder gone, say: the copy's worktree, made from it, will say why.
                Err(e) => log::warn!("agent {id}: its worktree's changes not read: {e:#}"),
            }
        }
        self.create_agent_with(
            &original.project_id,
            AgentOptions {
                model: Some(original.model),
                effort: Some(original.effort),
                mode: Some(original.mode),
                select: true,
                copy_of: Some(CopyOf {
                    name: original.name,
                    session_id,
                    entry,
                    worktree: original.worktree,
                    items,
                    notice,
                }),
                ..Default::default()
            },
        )
        .await
    }

    // ---------- worktree setup and teardown ----------

    /// Starts the project's setup in the agent's new worktree `wt`, in the background (an earlier
    /// one of the agent stops); its messages wait for it (`wait_setup`), and its process starts
    /// once it is over. False when the project has none.
    fn start_setup(self: &Arc<Self>, h: &AgentHandle, project: &Project, wt: &Worktree) -> bool {
        let steps = worktrees::runnable(&project.worktree_setup);
        if steps.is_empty() {
            return false;
        }
        let (id, ports) = {
            let mut rt = h.lock();
            rt.setup = Some(setup_label(&steps, 0));
            rt.setup_output = Some(SetupOutput::new(0));
            rt.setup_failure = None;
            (rt.meta.id.clone(), rt.meta.port_base)
        };
        let (tx, done) = tokio::sync::watch::channel(false);
        let mine = done.clone();
        let (c, task_id, project_dir, wt) =
            (self.clone(), id.clone(), project.path.clone(), wt.clone());
        // Held while it starts: its end, however soon, finds it in the map.
        let mut setups = self.setups.lock();
        let task = tauri::async_runtime::spawn(async move {
            c.run_setup(&task_id, &project_dir, &wt, ports, &steps)
                .await;
            let _ = tx.send(true);
            let mut setups = c.setups.lock();
            if setups
                .get(&task_id)
                .is_some_and(|s| s.done.same_channel(&mine))
            {
                setups.remove(&task_id);
            }
        });
        if let Some(earlier) = setups.insert(id, Setup { done, task }) {
            earlier.task.abort();
        }
        true
    }

    /// The setup's steps, one after the other, the agent showing the one running and the window
    /// what it writes as it comes, the whole of it going to the agent's setup log; the first that
    /// fails ends it, and the conversation says why (as does its ticket's first message).
    async fn run_setup(
        self: &Arc<Self>,
        id: &str,
        project_dir: &str,
        wt: &Worktree,
        ports: Option<u16>,
        steps: &[WorktreeStep],
    ) {
        let settings = self.settings.read().clone();
        let shells = crate::pty::detect_shells(&settings);
        let mut env = if settings.proxy_terminals {
            settings.proxy_env()
        } else {
            Vec::new()
        };
        env.extend(worktrees::step_env(project_dir, wt, ports));
        let log_path = self.data.setup_log(id);
        let setup_log = worktrees::SetupLog::create(&log_path);
        let started = Instant::now();
        for (i, step) in steps.iter().enumerate() {
            let label = setup_label(steps, i);
            // Gone meanwhile: nothing more to set up.
            if self
                .with_agent(id, |rt, _| {
                    rt.setup = Some(label.clone());
                    rt.setup_output = Some(SetupOutput::new(i));
                    Ok(())
                })
                .is_err()
            {
                return;
            }
            // Nothing written yet: the window forgets the lines of the step before.
            self.hub.emit(UiEvent::SetupOutput {
                agent_id: id.to_string(),
                step: i,
                total: 0,
                lines: Vec::new(),
            });
            setup_log.step(&label);
            let step_started = Instant::now();
            // The lines the window has not been sent yet: a tool writing line by line would send
            // it one event per line, each redrawing the output.
            let unsent = Mutex::new(Vec::new());
            let on_lines = |lines: &[String]| {
                setup_log.lines(lines);
                if self.keep_setup_lines(id, i, lines) {
                    let mut unsent = unsent.lock();
                    unsent.extend_from_slice(lines);
                    let excess = unsent.len().saturating_sub(SETUP_LINES);
                    unsent.drain(..excess);
                }
            };
            let running = worktrees::run_step(
                step,
                &shells,
                &wt.path,
                &env,
                worktrees::SETUP_LIMIT,
                &on_lines,
            );
            tokio::pin!(running);
            let mut every = tokio::time::interval(SETUP_SEND_EVERY);
            every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let ran = loop {
                tokio::select! {
                    ran = &mut running => break ran,
                    _ = every.tick() => self.send_setup_lines(id, i, &unsent),
                }
            };
            // Its last lines, before its end is told.
            self.send_setup_lines(id, i, &unsent);
            if let Err(f) = ran {
                setup_log.end(&format!("échec : {}", f.reason));
                let text = f.describe("La préparation du worktree");
                log::warn!("agent {id}: {text}");
                // The conversation shows its last lines, and says where the others are.
                let shown = if setup_log.written() {
                    format!("{text}\n\nSortie complète : {}", log_path.display())
                } else {
                    text.clone()
                };
                let _ = self.with_agent(id, |rt, fx| {
                    rt.setup = None;
                    rt.setup_output = None;
                    rt.setup_failure = Some(text);
                    rt.notice("warn", shown, fx);
                    Ok(())
                });
                self.warm(id);
                return;
            }
            setup_log.end(&format!(
                "terminée en {} s",
                step_started.elapsed().as_secs()
            ));
        }
        let n = steps.len();
        let text = format!(
            "Worktree préparé ({n} commande{}, {} s).",
            if n > 1 { "s" } else { "" },
            started.elapsed().as_secs()
        );
        let _ = self.with_agent(id, |rt, fx| {
            rt.setup = None;
            rt.setup_output = None;
            rt.notice("info", text, fx);
            Ok(())
        });
        self.warm(id);
    }

    /// Lines the step `step` of the agent's setup just wrote, kept for a window opened while it
    /// runs (`setup_outputs`). False when that setup is no longer under way (stopped, the agent
    /// gone): nothing to send.
    fn keep_setup_lines(&self, id: &str, step: usize, lines: &[String]) -> bool {
        let Ok(h) = self.agent(id) else { return false };
        let mut rt = h.lock();
        match rt.setup_output.as_mut() {
            Some(out) if out.step == step => {
                out.push(lines);
                true
            }
            _ => false,
        }
    }

    /// Sends the window the lines of the step `step` of the agent's setup it was not sent yet
    /// (taken from `unsent`, `SETUP_LINES` at most), in one event with the step's count so far.
    fn send_setup_lines(&self, id: &str, step: usize, unsent: &Mutex<Vec<String>>) {
        let lines = std::mem::take(&mut *unsent.lock());
        if lines.is_empty() {
            return;
        }
        let Ok(h) = self.agent(id) else { return };
        let total = match h.lock().setup_output.as_ref() {
            Some(out) if out.step == step => out.total,
            // Stopped meanwhile.
            _ => return,
        };
        self.hub.emit(UiEvent::SetupOutput {
            agent_id: id.to_string(),
            step,
            total,
            lines,
        });
    }

    /// What the step running of each agent's worktree setup wrote, its last lines, by agent: for a
    /// window opened while it runs.
    pub fn setup_outputs(&self) -> HashMap<String, SetupOutput> {
        self.agents
            .read()
            .iter()
            .filter_map(|(id, h)| Some((id.clone(), h.lock().setup_output.clone()?)))
            .collect()
    }

    /// Waits until the setup of the agent's new worktree is over, if one is under way.
    pub(crate) async fn wait_setup(&self, id: &str) {
        let done = self.setups.lock().get(id).map(|s| s.done.clone());
        if let Some(mut done) = done {
            // An error: stopped, which is over too.
            let _ = done.wait_for(|over| *over).await;
        }
    }

    /// Stops the setup of the agent's worktree, if one is under way, with all it started (its
    /// processes die with their job once its task is dropped); the agent no longer shows it.
    async fn stop_setup(self: &Arc<Self>, id: &str) {
        let setup = self.setups.lock().remove(id);
        if let Some(s) = setup {
            s.task.abort();
            let _ = s.task.await;
            let _ = self.with_agent(id, |rt, _| {
                rt.setup = None;
                rt.setup_output = None;
                Ok(())
            });
        }
    }

    /// Before the project's worktree `wt` is removed: the project's teardown runs in it, each step
    /// even after one failed, then isola's when it runs the worktree's services. What went
    /// wrong, as the user is told after "mais".
    pub(crate) async fn teardown_worktree(
        &self,
        project: &Project,
        wt: &Worktree,
        ports: Option<u16>,
    ) -> Option<String> {
        if !Path::new(&wt.path).is_dir() {
            return None;
        }
        let mut problems = Vec::new();
        let steps = worktrees::runnable(&project.worktree_teardown);
        if !steps.is_empty() {
            let settings = self.settings.read().clone();
            let shells = crate::pty::detect_shells(&settings);
            let mut env = if settings.proxy_terminals {
                settings.proxy_env()
            } else {
                Vec::new()
            };
            env.extend(worktrees::step_env(&project.path, wt, ports));
            for step in &steps {
                // Its output is not shown: the agent is being removed, and its failure says why.
                let ran = worktrees::run_step(
                    step,
                    &shells,
                    &wt.path,
                    &env,
                    worktrees::TEARDOWN_LIMIT,
                    &|_| {},
                )
                .await;
                if let Err(f) = ran {
                    log::warn!("{}", f.describe("Le démontage du worktree"));
                    problems.push(f.summary("le démontage du worktree"));
                }
            }
        }
        if isola::manages(&wt.path) {
            if let Err(e) = isola::run(&wt.path, &["destroy"], isola::LIMIT).await {
                log::warn!("{}: {e:#}", wt.path);
                problems.push(format!("{e:#}"));
            }
        }
        (!problems.is_empty()).then(|| problems.join(" ; "))
    }

    /// A restored agent whose worktree folder went (removed after a pull request or a push) gets
    /// it back from its branch, with the project's files to copy and its setup. Without its
    /// branch (merged, then deleted), it stays as it is.
    async fn restore_worktree(self: &Arc<Self>, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let (pid, wt) = {
            let rt = h.lock();
            (rt.meta.project_id.clone(), rt.meta.worktree.clone())
        };
        let (Some(wt), Ok(project)) = (wt, self.project(&pid)) else {
            return;
        };
        if Path::new(&wt.path).exists() || !git::branch_exists(&project.path, &wt.branch).await {
            return;
        }
        let problem = match git::worktree_restore(&project.path, &wt.path, &wt.branch).await {
            Ok(()) => {
                let copied = testlaunch::copy_worktree_files(
                    &project.path,
                    &wt.path,
                    &project.worktree_copy,
                )
                .await;
                self.start_setup(&h, &project, &wt);
                copied
                    .err()
                    .map(|e| format!("Fichiers non copiés dans le worktree : {e:#}"))
            }
            Err(e) => Some(format!("Worktree non recréé depuis {} : {e:#}", wt.branch)),
        };
        let _ = self.with_agent(id, |rt, fx| {
            if let Some(p) = problem {
                rt.notice("warn", p, fx);
            }
            Ok(())
        });
        self.git.refresh(&pid);
    }

    /// The setup and teardown Claude suggests for the project's worktrees, from what it reads of
    /// the project (in its folder, with tools that only read).
    pub async fn suggest_worktree_steps(&self, project_id: &str) -> Result<WorktreeSuggestion> {
        let project = self.project(project_id)?;
        let shell = self.default_shell()?;
        let prompt = worktrees::suggest_prompt(
            &shell.label,
            &project.worktree_copy,
            isola::configured(&project.path),
        );
        let answer = self
            .read_project(&project, worktrees::SUGGEST_SYSTEM, &prompt)
            .await?;
        let suggestion = worktrees::parse_suggestion(&answer, Path::new(&project.path), &shell.id)
            .ok_or_else(|| anyhow!("Claude n'a pas proposé de commandes lisibles."))?;
        // Not the same as having found nothing to run: it gave some, none of which could be shown
        // as they would run.
        if suggestion.setup.is_empty() && suggestion.teardown.is_empty() && suggestion.refused > 0 {
            bail!(worktrees::ALL_REFUSED);
        }
        Ok(suggestion)
    }

    /// The launch commands Claude suggests for the project (the servers, watchers and services
    /// its developer keeps running), from what it reads of it, each for the default shell. Only
    /// a suggestion: the window shows it in the draft of the settings, nothing is saved here.
    pub async fn suggest_run_commands(&self, project_id: &str) -> Result<RunSuggestion> {
        let project = self.project(project_id)?;
        let shell = self.default_shell()?;
        let prompt = worktrees::run_suggest_prompt(&shell);
        let answer = self
            .read_project(&project, worktrees::RUN_SUGGEST_SYSTEM, &prompt)
            .await?;
        let suggestion =
            worktrees::parse_run_suggestion(&answer, Path::new(&project.path), &shell.id)
                .ok_or_else(|| anyhow!("Claude n'a pas proposé de commandes lisibles."))?;
        // Not the same as having found nothing to launch (see `suggest_worktree_steps`).
        if suggestion.commands.is_empty() && suggestion.refused > 0 {
            bail!(worktrees::ALL_REFUSED);
        }
        Ok(suggestion)
    }

    /// The shell that runs what is not given another: the system's first.
    fn default_shell(&self) -> Result<ShellInfo> {
        let settings = self.settings.read().clone();
        crate::pty::detect_shells(&settings)
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Aucun shell détecté."))
    }

    /// Claude's answer to `prompt` after it read the project, in its folder, with tools that only
    /// read (never Bash nor Edit): it can neither change nor run anything.
    async fn read_project(&self, project: &Project, system: &str, prompt: &str) -> Result<String> {
        self.ask_claude(Ask {
            who: "Claude",
            model: "sonnet",
            tools: "Read,Glob,Grep",
            cwd: Path::new(&project.path),
            system,
            prompt,
            limit: worktrees::SUGGEST_LIMIT,
        })
        .await
    }

    async fn auto_name(self: &Arc<Self>, id: &str, prompt: &str) {
        match self.generate_name(prompt).await {
            Ok(Some(slug)) => {
                if let Err(e) = self.apply_generated_name(id, &slug).await {
                    log::warn!("auto-naming failed: {e:#}");
                }
            }
            Ok(None) => {
                log::info!("agent {id}: no usable name from the model, keeping the default one")
            }
            Err(e) => log::warn!("auto-naming failed: {e:#}"),
        }
    }

    /// One question to Haiku (`claude -p`, no tools, no session, no MCP): its answer, within 90 s.
    pub(crate) async fn one_shot(&self, system: &str, prompt: &str) -> Result<String> {
        self.one_shot_within(system, prompt, Duration::from_secs(90))
            .await
    }

    /// `one_shot` within `limit`, the question's writing included (a `claude` that never reads
    /// it would hold it forever); past it, the process is killed with all it started.
    pub(crate) async fn one_shot_within(
        &self,
        system: &str,
        prompt: &str,
        limit: Duration,
    ) -> Result<String> {
        self.ask_claude(Ask {
            who: "Haiku",
            model: "haiku",
            tools: "",
            cwd: &std::env::temp_dir(),
            system,
            prompt,
            limit,
        })
        .await
    }

    /// One question to Claude (`claude -p`, no session, no settings, no MCP), as `ask` says: its
    /// answer, within its limit.
    async fn ask_claude(&self, ask: Ask<'_>) -> Result<String> {
        use tokio::io::AsyncWriteExt;
        let Ask {
            who,
            model,
            tools,
            cwd,
            system,
            prompt,
            limit,
        } = ask;
        let settings = self.settings.read().clone();
        let program =
            claude::resolve_binary(&settings.claude_path).context("claude introuvable")?;
        let mut cmd = tokio::process::Command::new(program);
        cmd.args([
            "-p",
            "--model",
            model,
            "--output-format",
            "json",
            "--no-session-persistence",
            "--tools",
            tools,
            "--setting-sources",
            "",
            // No MCP servers (account connectors included): nothing that invites the model to act.
            "--strict-mcp-config",
            "--system-prompt",
            system,
        ]);
        // Reading needs no permission in its folder, and is refused outside of it: nothing more is
        // allowed.
        cmd.current_dir(cwd)
            .envs(settings.claude_env())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(claude::CREATE_NO_WINDOW);
        crate::job::isolate(&mut cmd);
        let mut child = cmd.spawn()?;
        // Dropped when this returns: whatever the process left running ends with it.
        let _job = crate::job::Job::for_child(&child);
        let asked = async move {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(prompt.as_bytes()).await?;
                // Dropped here: the input ends, and the CLI answers.
            }
            anyhow::Ok(child.wait_with_output().await?)
        };
        let out = tokio::time::timeout(limit, asked)
            .await
            .map_err(|_| anyhow!("pas de réponse de {who} en {} s", limit.as_secs()))??;
        let v: Value = serde_json::from_slice(&out.stdout)?;
        Ok(v["result"].as_str().unwrap_or("").to_string())
    }

    /// A short name for the task, or None when the model did not answer with one.
    async fn generate_name(&self, prompt: &str) -> Result<Option<String>> {
        let answer = self
            .one_shot(
                "Tu nommes des tâches de développement sans jamais les réaliser. Tu réponds uniquement par un slug.",
                &naming_prompt(prompt),
            )
            .await?;
        Ok(name_from_answer(&answer))
    }

    async fn apply_generated_name(self: &Arc<Self>, id: &str, slug: &str) -> Result<()> {
        let h = self.agent(id)?;
        let project_id = h.lock().meta.project_id.clone();
        let taken: Vec<String> = self
            .project_agents(&project_id)
            .iter()
            .filter(|a| a.lock().meta.id != id)
            .map(|a| a.lock().meta.name.clone())
            .collect();
        let mut name = slug.to_string();
        let mut n = 2;
        while taken.contains(&name) {
            name = format!("{slug}-{n}");
            n += 1;
        }
        let (worktree, proc) = {
            let mut rt = h.lock();
            if rt.meta.named {
                return Ok(());
            }
            rt.meta.name = name.clone();
            rt.meta.named = true;
            (rt.meta.worktree.clone(), rt.proc.clone())
        };
        self.emit_agent(&h);
        self.request_save();
        if let Some(wt) = worktree {
            let project = self.project(&project_id)?;
            let branch = format!("{}{name}", paths::BRANCH_PREFIX);
            if !git::branch_exists(&project.path, &branch).await
                && git::rename_current_branch(&wt.path, &branch).await.is_ok()
            {
                if let Some(w) = h.lock().meta.worktree.as_mut() {
                    w.branch = branch;
                }
                self.request_save();
            }
        }
        if let Some(p) = proc {
            let _ = p
                .control(
                    json!({ "subtype": "rename_session", "title": name, "source": "host" }),
                    Duration::from_secs(10),
                )
                .await;
        }
        Ok(())
    }

    pub async fn rename_agent(self: &Arc<Self>, id: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            bail!("nom vide");
        }
        let h = self.agent(id)?;
        let proc = {
            let mut rt = h.lock();
            rt.meta.name = name.to_string();
            rt.meta.named = true;
            rt.proc.clone()
        };
        self.emit_agent(&h);
        self.request_save();
        if let Some(p) = proc {
            let _ = p
                .control(
                    json!({ "subtype": "rename_session", "title": name, "source": "host" }),
                    Duration::from_secs(10),
                )
                .await;
        }
        Ok(())
    }

    pub async fn archive_agent(self: &Arc<Self>, id: &str, archived: bool) -> Result<()> {
        if archived {
            // An archived agent is no longer reachable from claude.ai.
            if self.agent(id)?.lock().meta.remote_control {
                let _ = self.set_remote_control(id, false).await;
            }
            // Its ticket lets go of it before its turn is stopped: that turn's end, whenever it is
            // read, no longer moves the ticket (no « bloqué » for it).
            self.unlink_ticket(id);
            // Stop the current turn first: closing stdin alone lets it run to completion. Its end
            // is then a stop, as with `interrupt`, never an error of the agent.
            let running = {
                let h = self.agent(id)?;
                let mut rt = h.lock();
                let running = rt.proc.clone().filter(|_| rt.meta.status.is_active());
                if running.is_some() {
                    rt.interrupted = true;
                }
                running
            };
            if let Some(p) = running {
                let _ = p
                    .control(
                        json!({ "subtype": "interrupt", "cancel_queued": true }),
                        Duration::from_secs(5),
                    )
                    .await;
            }
        }
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        self.with_agent(id, |rt, fx| {
            rt.meta.archived = archived;
            if archived {
                rt.meta.resume_at = None;
                // Its test ports are free for others.
                rt.meta.port_base = None;
                if let Some(p) = rt.detach() {
                    p.close_input();
                }
                rt.clear_pending(fx);
                if rt.meta.status.is_active() {
                    rt.set_status(AgentStatus::Done, fx);
                }
            }
            fx.save = true;
            Ok(())
        })?;
        let pid = self.agent(id)?.lock().meta.project_id.clone();
        self.git.refresh(&pid);
        // Its test launches stop with it. Its ticket already let go of it (above); what may start
        // starts now that its place, its ports and its quota wait are free.
        if archived {
            // The setup of its worktree stops; what waited for it finds it archived (and its
            // ticket let go of it, above).
            self.stop_setup(id).await;
            self.stop_test_runs(id);
            self.release_ticket(id);
        } else {
            self.restore_worktree(id).await;
        }
        Ok(())
    }

    /// Removes the agent. Worktree cleanup is best effort: a problem there is returned as a
    /// warning, the agent is removed regardless.
    pub async fn delete_agent(
        self: &Arc<Self>,
        id: &str,
        remove_worktree: bool,
    ) -> Result<Option<String>> {
        // End its claude.ai session rather than leave it behind.
        if self.agent(id)?.lock().meta.remote_control {
            let _ = self.set_remote_control(id, false).await;
        }
        // Wait for an in-flight start (warm-up) so that its process is killed too.
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        let h = self
            .agents
            .write()
            .remove(id)
            .ok_or_else(|| anyhow!("agent introuvable"))?;
        let (pid, worktree, ports) = {
            let mut rt = h.lock();
            rt.gen += 1;
            if let Some(p) = rt.proc.take() {
                p.kill();
            }
            rt.conv.delete_file();
            (
                rt.meta.project_id.clone(),
                rt.meta.worktree.clone(),
                rt.meta.port_base,
            )
        };
        self.spawn_locks.lock().remove(id);
        // The setup of its worktree stops, with what it started (which holds the worktree). Once it
        // is gone: what waited for that setup (its ticket's first message) finds no agent to send to.
        // Its log goes with it.
        self.stop_setup(id).await;
        let _ = std::fs::remove_file(self.data.setup_log(id));
        {
            let mut ui = self.ui.write();
            if ui.selected_agent.get(&pid).map(String::as_str) == Some(id) {
                ui.selected_agent.remove(&pid);
            }
        }
        self.hub.emit(UiEvent::AgentRemoved {
            id: id.to_string(),
            project_id: pid.clone(),
        });
        self.request_save();
        self.update_tray();
        self.stop_test_runs(id);
        self.release_ticket(id);
        let mut problems = Vec::new();
        match (remove_worktree, worktree, self.project(&pid)) {
            (true, Some(wt), Ok(project)) => {
                // Give the killed process tree a moment to release its handles on the worktree.
                tokio::time::sleep(Duration::from_millis(300)).await;
                problems.extend(self.teardown_worktree(&project, &wt, ports).await);
                if let Err(e) = git::worktree_remove(&project.path, &wt.path, &wt.branch).await {
                    problems.push(format!("le worktree n'a pas pu être nettoyé : {e:#}"));
                }
            }
            // Kept: isola's services of the worktree stop all the same.
            (false, Some(wt), _) if isola::manages(&wt.path) => isola::down_later(wt.path),
            _ => {}
        }
        let warning = (!problems.is_empty())
            .then(|| format!("Agent supprimé, mais {}", problems.join(" ; ")));
        self.git.refresh(&pid);
        Ok(warning)
    }

    /// Merges the agent's branch into its base branch, in the project's folder. When the folder is
    /// on another branch the merge is refused (`NotOnBase`), unless `switch_to_base`: the folder is
    /// switched to the base first (it has no tracked change, checked here).
    pub async fn merge_agent(
        self: &Arc<Self>,
        id: &str,
        squash: bool,
        switch_to_base: bool,
    ) -> Result<String> {
        // The app does not restart for an update in the middle of it.
        let _working = self.working();
        let h = self.agent(id)?;
        let (pid, name, wt) = {
            let rt = h.lock();
            (
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.meta.worktree.clone(),
            )
        };
        let wt = wt.ok_or_else(|| anyhow!("cet agent n'a pas de worktree"))?;
        let project = self.project(&pid)?;
        if git::has_tracked_changes(&project.path).await? {
            bail!("Le dépôt principal a des modifications non commitées : commite-les ou mets-les de côté avant de merger.");
        }
        let dirty = git::status(&wt.path).await?.entries.len();
        if dirty > 0 {
            bail!("L'agent a {dirty} fichier(s) non commité(s) : demande-lui de commiter avant de merger.");
        }
        if !git::branch_exists(&project.path, &wt.base_branch).await {
            bail!(
                "La branche de base « {} » n'existe plus : « {} » ne peut pas être mergée dedans.",
                wt.base_branch,
                wt.branch
            );
        }
        // Counted from the base, not from HEAD: the folder may be on another branch.
        if git::ahead_of(&project.path, &wt.base_branch, &wt.branch).await? == 0 {
            bail!(
                "Rien à merger : la branche {} n'a pas de nouveau commit.",
                wt.branch
            );
        }
        let current = git::head_branch(&project.path).await;
        if current != wt.base_branch {
            if !switch_to_base {
                return Err(NotOnBase {
                    current,
                    base: wt.base_branch,
                }
                .into());
            }
            git::switch(&project.path, &wt.base_branch).await?;
        }
        let message = if squash {
            let subjects = git::text(
                &project.path,
                &[
                    "log",
                    "--format=- %s",
                    &format!("{}..{}", wt.base_branch, wt.branch),
                ],
            )
            .await
            .unwrap_or_default();
            format!("{name}\n\n{subjects}")
        } else {
            format!("Merge branch '{}'", wt.branch)
        };
        let out = git::merge(&project.path, &wt.branch, squash, &message).await?;
        self.git.refresh(&pid);
        Ok(out)
    }

    // ---------- projects ----------

    pub async fn create_project(
        self: &Arc<Self>,
        path: &str,
        name: &str,
        color: &str,
        worktree_per_agent: bool,
        first_agent: Option<String>,
    ) -> Result<Project> {
        let path = path.trim().trim_end_matches(['\\', '/']).to_string();
        if !Path::new(&path).is_dir() {
            bail!("Le dossier {path} n'existe pas");
        }
        if git::toplevel(&path).await.is_none() {
            git::init_repo(&path).await.context("git init")?;
        }
        let project = Project {
            id: new_id(),
            name: if name.trim().is_empty() {
                Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "projet".into())
            } else {
                name.trim().to_string()
            },
            path,
            color: color.to_string(),
            worktree_per_agent,
            created_at: now_ms(),
            run_commands: Vec::new(),
            board: BoardSettings::default(),
            worktree_copy: default_worktree_copy(),
            worktree_setup: Vec::new(),
            worktree_teardown: Vec::new(),
            integrations: ProjectIntegrations::default(),
            commit_mode: CommitMode::default(),
        };
        self.projects.write().push(project.clone());
        {
            let mut ui = self.ui.write();
            ui.active_project = Some(project.id.clone());
            ui.view = "project".into();
        }
        self.request_save();
        self.git.watch(&project.id, &project.path);
        if let Some(model) = first_agent {
            self.create_agent(&project.id, Some(model)).await?;
        }
        Ok(project)
    }

    pub fn update_project(&self, p: Project) -> Result<()> {
        let mut projects = self.projects.write();
        let cur = projects
            .iter_mut()
            .find(|x| x.id == p.id)
            .ok_or_else(|| anyhow!("projet introuvable"))?;
        cur.name = p.name;
        cur.color = p.color;
        cur.worktree_per_agent = p.worktree_per_agent;
        cur.run_commands = p.run_commands;
        cur.worktree_copy = p.worktree_copy;
        cur.worktree_setup = p.worktree_setup;
        cur.worktree_teardown = p.worktree_teardown;
        cur.commit_mode = p.commit_mode;
        // What was imported is the backend's own: the window's copy may be older.
        let imported = std::mem::take(&mut cur.integrations.imported);
        cur.integrations = crate::integrations::checked_links(p.integrations);
        cur.integrations.imported = imported;
        drop(projects);
        self.request_save();
        Ok(())
    }

    pub fn reorder_projects(&self, ids: &[String]) {
        let mut projects = self.projects.write();
        projects.sort_by_key(|p| ids.iter().position(|i| *i == p.id).unwrap_or(usize::MAX));
        drop(projects);
        self.request_save();
    }

    pub fn remove_project(self: &Arc<Self>, id: &str) -> Result<()> {
        let agents: Vec<String> = self
            .project_agents(id)
            .iter()
            .map(|h| h.lock().meta.id.clone())
            .collect();
        for aid in agents {
            // Bound first: the map guard must not live across the agent lock and the I/O below.
            let removed = self.agents.write().remove(&aid);
            let mut worktree = None;
            if let Some(h) = removed {
                let mut rt = h.lock();
                rt.gen += 1;
                if let Some(p) = rt.proc.take() {
                    p.kill();
                }
                rt.conv.delete_file();
                worktree = rt.meta.worktree.clone();
            }
            // The setup of its worktree stops, with what it started; its log goes.
            if let Some(s) = self.setups.lock().remove(&aid) {
                s.task.abort();
            }
            let _ = std::fs::remove_file(self.data.setup_log(&aid));
            // Its test launches are the project's terminals too (killed below all the same):
            // none is kept for it. isola's services of its worktree stop too.
            self.stop_test_runs(&aid);
            if let Some(wt) = worktree.filter(|w| isola::manages(&w.path)) {
                isola::down_later(wt.path);
            }
            self.hub.emit(UiEvent::AgentRemoved {
                id: aid,
                project_id: id.to_string(),
            });
        }
        self.pty.kill_project(id);
        self.git.unwatch(id);
        self.projects.write().retain(|p| p.id != id);
        // After the project is out: `ticket_create` checks the project under the projects' read
        // lock, which it holds while it adds the ticket, so a ticket it adds before this is dropped
        // here, and none is added after.
        self.drop_project_tickets(id);
        // Read before taking the ui lock: never hold ui while waiting on projects.
        let fallback = self.projects.read().first().map(|p| p.id.clone());
        {
            let mut ui = self.ui.write();
            ui.selected_agent.remove(id);
            if ui.active_project.as_deref() == Some(id) {
                ui.active_project = fallback;
            }
        }
        self.request_save();
        self.update_tray();
        // Its agents are gone: one of them may have held the board waiting for its quota.
        self.schedule();
        Ok(())
    }

    // ---------- git ----------

    async fn git_loop(self: Arc<Self>, mut rx: mpsc::UnboundedReceiver<String>) {
        let mut due: HashMap<String, Instant> = HashMap::new();
        let mut last: HashMap<String, Instant> = HashMap::new();
        loop {
            let next = due.values().min().copied();
            tokio::select! {
                msg = rx.recv() => {
                    let Some(pid) = msg else { break };
                    let now = Instant::now();
                    let earliest = last.get(&pid).map(|t| *t + Duration::from_millis(700)).unwrap_or(now);
                    due.entry(pid).or_insert_with(|| earliest.max(now + Duration::from_millis(150)));
                }
                _ = tokio::time::sleep_until(next.unwrap_or_else(Instant::now).into()), if next.is_some() => {
                    let now = Instant::now();
                    let ready: Vec<String> = due.iter().filter(|(_, t)| **t <= now).map(|(k, _)| k.clone()).collect();
                    for pid in ready {
                        // A refresh still running for this project (large repo): try again later
                        // rather than overlapping and publishing results out of order.
                        if !self.git_inflight.lock().insert(pid.clone()) {
                            due.insert(pid, now + Duration::from_millis(250));
                            continue;
                        }
                        due.remove(&pid);
                        last.insert(pid.clone(), now);
                        self.git.take_flag(&pid);
                        let c = self.clone();
                        tauri::async_runtime::spawn(async move {
                            c.compute_git(&pid).await;
                            c.git_inflight.lock().remove(&pid);
                        });
                    }
                }
            }
        }
    }

    pub(crate) async fn compute_git(self: &Arc<Self>, project_id: &str) {
        let Ok(project) = self.project(project_id) else {
            return;
        };
        let info = match self.toplevel(&project.path).await {
            None => GitInfo::default(),
            Some(root) => {
                let (st, remotes) = tokio::join!(git::status(&root), git::remotes(&root));
                let st = st.unwrap_or_default();
                let prefix = sub_prefix(&root, &project.path);
                let mut info = GitInfo {
                    is_repo: true,
                    branch: st.branch.clone(),
                    upstream: st.upstream.clone(),
                    upstream_gone: st.upstream_gone,
                    ahead: st.ahead,
                    behind: st.behind,
                    has_remote: !remotes.is_empty(),
                    last_fetch: git::last_fetch(&root),
                    ..Default::default()
                };
                let tally = |c: char, info: &mut GitInfo| match c {
                    'A' => info.added += 1,
                    'D' => info.deleted += 1,
                    _ => info.modified += 1,
                };
                for e in &st.entries {
                    tally(e.status, &mut info);
                }
                let agents: Vec<(String, Option<Worktree>, Vec<String>)> = self
                    .project_agents(project_id)
                    .iter()
                    .map(|h| {
                        let rt = h.lock();
                        (
                            rt.meta.id.clone(),
                            rt.meta.worktree.clone(),
                            rt.meta.touched_files.clone(),
                        )
                    })
                    .collect();
                for (aid, wt, touched) in agents {
                    let count = match wt {
                        Some(wt) => match git::status(&wt.path).await {
                            Ok(ws) => {
                                for e in &ws.entries {
                                    tally(e.status, &mut info);
                                }
                                ws.entries.len() as u32
                            }
                            Err(_) => 0,
                        },
                        None => touched
                            .iter()
                            .filter(|t| {
                                let full = format!("{prefix}{t}");
                                st.entries
                                    .iter()
                                    .any(|e| e.path.eq_ignore_ascii_case(&full))
                            })
                            .count() as u32,
                    };
                    info.agents.insert(aid, count);
                }
                info.total = info.modified + info.added + info.deleted;
                info
            }
        };
        self.git.invalidate_files();
        self.git_cache
            .write()
            .insert(project_id.to_string(), info.clone());
        self.hub.emit(UiEvent::Git {
            project_id: project_id.to_string(),
            git: info,
        });
        // Its board starts nothing until its target is there: a first commit, or the branch made
        // again, starts the queue without waiting for another change of the board.
        if self.board_issues.lock().contains_key(project_id) {
            self.schedule();
        }
    }

    /// Dirty files for the files panel. `agent_id` + scope "agent" restricts to one agent.
    pub async fn git_files(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<Vec<FileChange>> {
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(|| anyhow!("pas un dépôt git"))?;
        let prefix = sub_prefix(&root, &project.path);
        let agents: Vec<(String, Option<Worktree>, Vec<String>)> = self
            .project_agents(project_id)
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
            git::file_changes(&root).await?
        };
        let owner = |path: &str| {
            agents
                .iter()
                .find(|(_, wt, touched)| {
                    wt.is_none()
                        && touched
                            .iter()
                            .any(|t| format!("{prefix}{t}").eq_ignore_ascii_case(path))
                })
                .map(|(id, _, _)| id.clone())
        };
        for mut f in main {
            f.agent_id = owner(&f.path);
            if agent_id.is_none() || f.agent_id.is_some() {
                out.push(f);
            }
        }
        for (id, wt, _) in &agents {
            if let Some(wt) = wt {
                for mut f in git::file_changes(&wt.path).await.unwrap_or_default() {
                    f.agent_id = Some(id.clone());
                    f.in_worktree = true;
                    out.push(f);
                }
            }
        }
        Ok(out)
    }

    /// The checkout holding the files listed for `agent_id` (its worktree), else the project's
    /// repository: the files panel's paths are relative to it.
    async fn files_root(&self, project_id: &str, agent_id: Option<String>) -> Result<String> {
        let project = self.project(project_id)?;
        let worktree = match &agent_id {
            Some(a) => self.agent(a)?.lock().meta.worktree.clone(),
            None => None,
        };
        match worktree {
            Some(wt) => Ok(wt.path),
            None => self
                .toplevel(&project.path)
                .await
                .ok_or_else(|| anyhow!("pas un dépôt git")),
        }
    }

    /// The folder the editor shows for `agent_id`: its worktree (with the branch it left), else
    /// the project's repository, else the project's folder when it is not one.
    pub async fn edit_root(
        &self,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<(String, Option<String>)> {
        let project = self.project(project_id)?;
        if let Some(a) = &agent_id {
            let worktree = self.agent(a)?.lock().meta.worktree.clone();
            if let Some(wt) = worktree {
                return Ok((wt.path, Some(wt.base_branch)));
            }
        }
        let root = self.toplevel(&project.path).await.unwrap_or(project.path);
        Ok((root, None))
    }

    /// `edit_root`, with the folders of it that the editor's renames and deletions keep away from:
    /// those holding the agents' worktrees (see `worktrees_kept`), every worktree of the
    /// repository (the agents' of every project sharing it, a validation's), every agent's, and
    /// every project's folder (renamed, its project would point at nothing).
    pub async fn edit_kept(
        &self,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<(String, Vec<fsedit::Kept>)> {
        let (root, base) = self.edit_root(project_id, agent_id).await?;
        let kept_as = |what| move |path| fsedit::Kept { path, what };
        let mut kept: Vec<fsedit::Kept> =
            worktrees_kept(&root, &self.project(project_id)?.path, base.is_some())
                .into_iter()
                .map(kept_as(fsedit::WORKTREES_KEPT))
                .collect();
        let mut worktrees = git::worktree_paths(&root).await.unwrap_or_default();
        worktrees.extend(self.agents.read().values().filter_map(|h| {
            let wt = h.lock().meta.worktree.clone();
            wt.map(|w| w.path)
        }));
        let projects: Vec<String> = self
            .projects
            .read()
            .iter()
            .map(|p| p.path.clone())
            .collect();
        for (paths, what) in [
            (worktrees, fsedit::WORKTREE_KEPT),
            (projects, fsedit::PROJECT_KEPT),
        ] {
            kept.extend(
                paths
                    .iter()
                    .filter_map(|p| below_root(&root, p))
                    .map(kept_as(what)),
            );
        }
        Ok((root, kept))
    }

    /// The files of an editor source (the agent's worktree, else the project's checkout), with the
    /// ignored ones the project's « Fichiers copiés dans les worktrees » patterns name.
    pub async fn fs_tree(
        &self,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<fsedit::Tree> {
        let (root, base) = self.edit_root(project_id, agent_id).await?;
        let project = self.project(project_id)?;
        // The copy takes from the project's folder, which is below the checkout's root when the project
        // is a subfolder of its repository; into a worktree it puts each file at the root.
        let below = if base.is_some() {
            String::new()
        } else {
            sub_prefix(&root, &project.path)
        };
        Ok(fsedit::tree(&root, &project.worktree_copy, &below).await)
    }

    /// Where a terminal opens: the agent's worktree, else the project's folder, or the folder
    /// `sub` of it (a path of the editor's tree, so relative to the same root, and refused when
    /// it leaves it).
    pub async fn term_cwd(
        &self,
        project_id: &str,
        agent_id: Option<String>,
        sub: Option<&str>,
    ) -> Result<String> {
        // A base branch says the root is a worktree's.
        let (root, base) = self.edit_root(project_id, agent_id).await?;
        // Removed after a pull request or a push: a shell started there would open in the app's
        // own folder instead, as if it were the agent's.
        if base.is_some() && !Path::new(&root).is_dir() {
            bail!("Le worktree de l'agent n'existe plus");
        }
        let Some(sub) = sub.map(str::trim).filter(|s| !s.is_empty()) else {
            // The project's folder itself, not the repository's the editor roots at, which holds
            // it when the project is one of its folders.
            return match base {
                Some(_) => Ok(root),
                None => Ok(self.project(project_id)?.path),
            };
        };
        paths::contained(Path::new(&root), sub)?;
        // Joined by components: the tree's paths use `/`, which a Windows path does not mix
        // with its `\` for the shells to follow (WSL's `--cd` takes it as given).
        let mut dir = PathBuf::from(&root);
        dir.extend(Path::new(sub).components().filter_map(|c| match c {
            std::path::Component::Normal(name) => Some(name),
            _ => None,
        }));
        if !dir.is_dir() {
            bail!("Le dossier « {sub} » n'existe pas");
        }
        Ok(dir.to_string_lossy().into_owned())
    }

    pub async fn git_diff(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        paths: Vec<String>,
    ) -> Result<String> {
        let root = self.files_root(project_id, agent_id).await?;
        git::diff(&root, &paths).await
    }

    /// The diff of the files panel's « Tout le projet » list: exactly the files `git_files` lists
    /// (the project's checkout and every agent's worktree), one entry per owner and checkout, in
    /// the order the list first shows them.
    pub async fn git_project_diff(self: &Arc<Self>, project_id: &str) -> Result<Vec<OwnedDiff>> {
        let mut groups: Vec<(Option<String>, bool, Vec<String>)> = Vec::new();
        for f in self.git_files(project_id, None).await? {
            match groups
                .iter_mut()
                .find(|(a, w, _)| *a == f.agent_id && *w == f.in_worktree)
            {
                Some((_, _, paths)) => paths.push(f.path),
                None => groups.push((f.agent_id, f.in_worktree, vec![f.path])),
            }
        }
        let checkout_groups = groups.iter().filter(|(_, w, _)| !w).count();
        let mut out = Vec::new();
        for (agent_id, in_worktree, paths) in groups {
            let diff = if in_worktree {
                // An agent deleted (or its worktree removed) since the list was made has no file
                // left to show: the project's checkout must not be read in its place.
                let worktree = agent_id
                    .as_deref()
                    .and_then(|a| self.agent(a).ok())
                    .and_then(|h| h.lock().meta.worktree.clone());
                let Some(worktree) = worktree else { continue };
                // Its group is all of the worktree's dirty files: no path to list.
                git::diff(&worktree.path, &[]).await.unwrap_or_default()
            } else {
                // Likewise for the checkout when no agent is credited with some of its files;
                // else each group lists its own paths.
                let listed: &[String] = if checkout_groups == 1 { &[] } else { &paths };
                git::diff(&self.files_root(project_id, None).await?, listed).await?
            };
            out.push(OwnedDiff {
                agent_id,
                in_worktree,
                diff,
            });
        }
        Ok(out)
    }

    /// Reverts a file of the files panel to HEAD (a new file is deleted).
    pub async fn git_discard(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        path: &str,
    ) -> Result<()> {
        let root = self.files_root(project_id, agent_id).await?;
        git::discard(&root, path).await?;
        self.git.refresh(project_id);
        Ok(())
    }

    // ---------- direct commit ----------

    /// What a direct commit of `agent_id`'s changes takes (of the project's own checkout, the
    /// agents' worktrees apart, when None): the checkout it commits in, the files the files panel
    /// lists for it, and among them the files copied into the worktrees, never committed.
    async fn commit_scope(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<DirectScope> {
        let project = self.project(project_id)?;
        let in_worktree = match &agent_id {
            Some(a) => self.agent(a)?.lock().meta.worktree.is_some(),
            None => false,
        };
        let root = self.files_root(project_id, agent_id.clone()).await?;
        let listed = match agent_id {
            // The project's checkout, whoever edited its files.
            None => git::file_changes(&root).await?,
            // Its worktree, else the files it edited in the project's checkout.
            Some(a) => self.git_files(project_id, Some(a)).await?,
        };
        // A copied file (`.env`…) matches one of the copy's patterns, from the project's folder
        // (what the copy took) or the checkout's root (where it put it), and git ignores it. Git
        // lists one only once forced into the index (an agent's `git add -f`): it still goes no
        // further, as with a ticket's validation.
        let prefix = if in_worktree {
            String::new()
        } else {
            sub_prefix(&root, &project.path)
        };
        let copy_pattern = |path: &str| {
            project.worktree_copy.iter().any(|pattern| {
                testlaunch::glob_match(pattern, path)
                    || path
                        .strip_prefix(prefix.as_str())
                        .is_some_and(|rel| testlaunch::glob_match(pattern, rel))
            })
        };
        let unread: HashSet<String> = listed
            .iter()
            .filter(|f| copy_pattern(&f.path))
            .map(|f| f.path.clone())
            .collect();
        let candidates: Vec<String> = unread.iter().cloned().collect();
        let mut copied: HashSet<String> = git::ignored(&root, &candidates)
            .await?
            .into_iter()
            .collect();
        if in_worktree {
            // What the copy took, as a ticket's validation goes by: in its worktree the rules are
            // the agent's, which may have taken `.env` out of its `.gitignore`. The copy put each
            // file where it is in the project's folder.
            let took = testlaunch::matching_ignored(&project.path, &project.worktree_copy).await;
            copied.extend(took.into_iter().filter(|f| unread.contains(f)));
        }
        let (left_out, files): (Vec<FileChange>, Vec<FileChange>) =
            listed.into_iter().partition(|f| copied.contains(&f.path));
        let left_out = left_out.into_iter().map(|f| f.path).collect();
        Ok(DirectScope {
            root,
            scope: CommitScope { files, left_out },
            unread,
        })
    }

    /// What a direct commit of `agent_id`'s changes (the project's own checkout when None) takes.
    pub async fn commit_preview(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<CommitScope> {
        Ok(self.commit_scope(project_id, agent_id).await?.scope)
    }

    /// Haiku's message for a direct commit of `paths`, in the style of the repository's latest
    /// commits. Only proposed: the user reads it, and commits it or not.
    pub async fn commit_propose(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        paths: Vec<String>,
    ) -> Result<String> {
        // Haiku may take its 90 s: the app does not restart for an update meanwhile.
        let _working = self.working();
        let DirectScope {
            root,
            scope,
            unread,
        } = self.commit_scope(project_id, agent_id).await?;
        let files = committed(scope, &paths);
        if files.is_empty() {
            bail!(NOTHING_TO_COMMIT);
        }
        let n = format!("-n{RECENT_SUBJECTS}");
        let subjects: Vec<String> = git::text(&root, &["log", &n, "--format=%s"])
            .await
            .map(|s| s.lines().map(str::to_string).collect())
            .unwrap_or_default();
        // The diff of the files the prompt lists, never more, and never of a file matching the
        // copy's patterns (`.env*`…), whatever git's rules say of it now.
        let read: Vec<String> = files
            .iter()
            .take(PROPOSAL_FILES)
            .filter(|f| !unread.contains(&f.path))
            .map(|f| f.path.clone())
            .collect();
        // Without a path, git would read every change of the checkout.
        let diff = if read.is_empty() {
            String::new()
        } else {
            git::diff(&root, &read).await.unwrap_or_default()
        };
        let prompt = commit_proposal_prompt(&subjects, &files, &diff);
        let answer = self.one_shot(COMMIT_PROPOSAL_SYSTEM, &prompt).await?;
        proposal_from_answer(&answer).ok_or_else(|| anyhow!("Haiku n'a rien proposé"))
    }

    /// Commits, with the user's `message`, the files of `paths` that a direct commit of
    /// `agent_id`'s changes takes (the copied files never). Returns the commit's short hash.
    pub async fn commit_direct(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        paths: Vec<String>,
        message: String,
    ) -> Result<String> {
        let message = message.trim();
        if message.is_empty() {
            bail!("Écris le message du commit.");
        }
        // The app does not restart for an update in the middle of it (a hook may take its time).
        let _working = self.working();
        let DirectScope { root, scope, .. } = self.commit_scope(project_id, agent_id).await?;
        let files: Vec<String> = committed(scope, &paths)
            .into_iter()
            .map(|f| f.path)
            .collect();
        if files.is_empty() {
            bail!(NOTHING_TO_COMMIT);
        }
        let done = git::commit_paths(&root, &files, message).await;
        // Even after a refusal: a hook may have changed files (a formatter) before saying no.
        self.git.refresh(project_id);
        done?;
        git::text(&root, &["rev-parse", "--short", "HEAD"]).await
    }

    /// The repository graph (every branch, agents' worktree branches included) and the branch
    /// the agent works on.
    pub async fn git_log(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<GitLog> {
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(|| anyhow!("pas un dépôt git"))?;
        let worktree = match &agent_id {
            Some(a) => self.agent(a)?.lock().meta.worktree.clone(),
            None => None,
        };
        let head = match worktree {
            Some(wt) => Some(wt.branch),
            None => Some(git::current_branch(&root).await).filter(|b| !b.is_empty() && b != "HEAD"),
        };
        Ok(GitLog {
            commits: git::log(&root, 300).await?,
            head,
        })
    }

    pub async fn git_show(self: &Arc<Self>, project_id: &str, hash: &str) -> Result<String> {
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(|| anyhow!("pas un dépôt git"))?;
        git::show(&root, hash).await
    }

    fn sync_lock(&self, root: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.sync_locks
            .lock()
            .entry(root.to_string())
            .or_default()
            .clone()
    }

    /// Refreshes the git state of every project in the repository at `root`.
    async fn refresh_repo(&self, root: &str) {
        let projects: Vec<(String, String)> = self
            .projects
            .read()
            .iter()
            .map(|p| (p.id.clone(), p.path.clone()))
            .collect();
        for (id, path) in projects {
            if self.toplevel(&path).await.as_deref() == Some(root) {
                self.git.refresh(&id);
            }
        }
    }

    /// Fetches every project's repository that has a remote, one at a time and without ever
    /// asking for credentials, so that the commits to pull show up.
    pub(crate) async fn fetch_all(self: &Arc<Self>) {
        let paths: Vec<String> = self
            .projects
            .read()
            .iter()
            .map(|p| p.path.clone())
            .collect();
        let mut roots: Vec<String> = Vec::new();
        for path in paths {
            match self.toplevel(&path).await {
                Some(root) if !roots.contains(&root) => roots.push(root),
                _ => {}
            }
        }
        for root in roots {
            if git::remotes(&root).await.is_empty() {
                continue;
            }
            let lock = self.sync_lock(&root);
            // The user's own fetch, pull or push is running: no need for another one.
            let Ok(_guard) = lock.try_lock() else {
                continue;
            };
            match git::fetch(&root, true).await {
                Ok(()) => self.refresh_repo(&root).await,
                Err(e) => log::info!("background fetch of {root} failed: {e:#}"),
            }
        }
    }

    /// Fetch, pull or push of the project's main checkout, asked by the user (so credentials
    /// may be asked for). Returns a summary for the user.
    pub async fn git_sync(self: &Arc<Self>, project_id: &str, op: SyncOp) -> Result<String> {
        let project = self.project(project_id)?;
        let root = self
            .toplevel(&project.path)
            .await
            .ok_or_else(|| anyhow!("pas un dépôt git"))?;
        let lock = self.sync_lock(&root);
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
        self.refresh_repo(&root).await;
        out
    }

    pub async fn file_suggestions(
        self: &Arc<Self>,
        agent_id: &str,
        query: &str,
    ) -> Result<Vec<String>> {
        let cwd = self.agent(agent_id)?.lock().meta.cwd.clone();
        let files = self.git.file_index(&cwd).await;
        Ok(git::fuzzy_files(&files, query, 40))
    }

    // ---------- usage ----------

    pub async fn refresh_usage(self: &Arc<Self>) {
        let proc = self
            .agents
            .read()
            .values()
            .find_map(|h| h.lock().proc.clone());
        let mut windows = None;
        if let Some(p) = proc {
            if let Ok(v) = p
                .control(
                    json!({ "subtype": "get_usage", "skip_behaviors": true }),
                    Duration::from_secs(20),
                )
                .await
            {
                if v["rate_limits"].is_object() {
                    windows = Some(usage::parse_windows(&v["rate_limits"]));
                }
            }
        }
        if windows.is_none() && usage::oauth_due(*self.last_oauth_call.lock(), now_ms()) {
            *self.last_oauth_call.lock() = Some(now_ms());
            let settings = self.settings.read().clone();
            match usage::fetch_oauth(&settings).await {
                Ok(w) => windows = Some(w),
                Err(e) => log::debug!("usage endpoint: {e:#}"),
            }
        }
        let read = windows.is_some();
        let snapshot = {
            let mut u = self.usage.lock();
            if let Some((five, week)) = windows {
                u.five_hour = five.or(u.five_hour);
                u.seven_day = week.or(u.seven_day);
                u.updated_at = now_ms();
            }
            u.today_cost = self.stats.today_cost();
            u.clone()
        };
        self.hub.emit(UiEvent::Usage { usage: snapshot });
        if read {
            // Back under the threshold, the tickets go on at once.
            self.pause_tick();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_taken_only_from_slug_like_answers() {
        assert_eq!(
            name_from_answer("refacto-auth"),
            Some("refacto-auth".into())
        );
        assert_eq!(
            name_from_answer("  `tests-e2e`\n"),
            Some("tests-e2e".into())
        );
        assert_eq!(
            name_from_answer("Migration JWT"),
            Some("migration-jwt".into())
        );
        // The model sometimes answers the task instead of naming it: never a name.
        assert_eq!(
            name_from_answer("Je n'ai accès qu'aux outils Claude Docs."),
            None
        );
        assert_eq!(name_from_answer("Voici le slug : creation-fichier"), None);
        assert_eq!(name_from_answer(""), None);
    }

    fn change(path: &str, status: &str) -> FileChange {
        FileChange {
            path: path.into(),
            status: status.into(),
            add: 1,
            del: 0,
            agent_id: None,
            in_worktree: false,
        }
    }

    #[test]
    fn a_commit_proposal_imitates_the_latest_subjects_of_the_repository() {
        let subjects: Vec<String> = [
            "feat(board): supprimer un ticket demande confirmation",
            "fix(git): le merge garde la branche",
        ]
        .map(String::from)
        .to_vec();
        let files = [change("src/app.ts", "M"), change("src/new.ts", "A")];
        let p = commit_proposal_prompt(
            &subjects,
            &files,
            "diff --git a/src/app.ts b/src/app.ts\n+x\n",
        );
        assert!(
            p.contains("<sujets-recents>\nfeat(board): supprimer un ticket demande confirmation\nfix(git): le merge garde la branche\n</sujets-recents>"),
            "{p}"
        );
        assert!(p.contains("style"), "{p}");
        assert!(
            p.contains("<fichiers>\nM src/app.ts\nA src/new.ts\n</fichiers>"),
            "{p}"
        );
        assert!(
            p.contains("<diff>\ndiff --git a/src/app.ts b/src/app.ts\n+x\n</diff>"),
            "{p}"
        );
        // A repository without a commit yet has no style to imitate.
        let first = commit_proposal_prompt(&[], &files, "");
        assert!(!first.contains("<sujets-recents>"), "{first}");
        assert!(first.contains("Conventional Commits"), "{first}");
    }

    #[test]
    fn a_commit_proposal_reads_a_capped_diff_and_list_of_files() {
        let diff = format!("{}FIN-DU-DIFF", "+ligne\n".repeat(20_000));
        let files: Vec<FileChange> = (0..250)
            .map(|i| change(&format!("src/f{i}.ts"), "M"))
            .collect();
        let p = commit_proposal_prompt(&[], &files, &diff);
        assert!(!p.contains("FIN-DU-DIFF"));
        assert!(p.len() < PROPOSAL_DIFF + 10_000, "{}", p.len());
        assert!(p.contains("M src/f199.ts\n") && !p.contains("src/f200.ts"));
        assert!(p.contains("… et 50 autres fichiers"), "{p}");
    }

    #[test]
    fn a_proposed_message_is_haikus_answer_without_fence_nor_quotes() {
        assert_eq!(
            proposal_from_answer("feat: ajoute le commit direct\n\nAvec un corps.\n").as_deref(),
            Some("feat: ajoute le commit direct\n\nAvec un corps.")
        );
        assert_eq!(
            proposal_from_answer("```text\nfix: un sujet\n\nUn corps.\n```").as_deref(),
            Some("fix: un sujet\n\nUn corps.")
        );
        assert_eq!(
            proposal_from_answer("  « docs: le README » ").as_deref(),
            Some("docs: le README")
        );
        assert_eq!(
            proposal_from_answer("`chore: rien`").as_deref(),
            Some("chore: rien")
        );
        assert_eq!(proposal_from_answer(" \n```\n```\n"), None);
        assert_eq!(proposal_from_answer(""), None);
    }

    #[test]
    fn the_task_is_framed_as_text_to_name() {
        let p = naming_prompt("Crée un fichier hello.txt");
        assert!(p.contains("<tache>\nCrée un fichier hello.txt\n</tache>"));
        assert!(p.contains("slug"));
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("Migration JWT rotation"), "migration-jwt-rotation");
        assert_eq!(
            slugify("  réparer l'écran d'accueil! "),
            "reparer-l-ecran-d-accueil"
        );
        assert!(slugify(&"mot ".repeat(30)).len() <= 40);
        // A first word longer than a slug is cut, not dropped.
        assert_eq!(slugify(&"b".repeat(50)), "b".repeat(40));
    }

    #[test]
    fn args_for_haiku_skip_effort() {
        let m = AgentMeta {
            model: "haiku".into(),
            effort: "high".into(),
            mode: "auto".into(),
            session_id: Some("s1".into()),
            ..Default::default()
        };
        let a = claude_args(&m);
        assert!(!a.contains(&"--effort".to_string()));
        assert!(a.contains(&"--resume=s1".to_string()));
        let m = AgentMeta {
            model: "opus".into(),
            effort: "max".into(),
            mode: "plan".into(),
            ..Default::default()
        };
        let a = claude_args(&m);
        assert!(a.windows(2).any(|w| w == ["--effort", "max"]));
    }

    #[test]
    fn a_copy_forks_the_session_of_its_original_until_it_has_its_own() {
        let copy = AgentMeta {
            model: "opus".into(),
            effort: "high".into(),
            mode: "auto".into(),
            fork_of: Some("s1".into()),
            ..Default::default()
        };
        let a = claude_args(&copy);
        assert!(
            a.windows(2).any(|w| w == ["--resume=s1", "--fork-session"]),
            "{a:?}"
        );
        // An original that ran no turn since it was recorded: its session as it is.
        assert!(!a.iter().any(|x| x.starts_with("--resume-session-at")));
        // Where it was copied: the turns the original ran since are left out.
        let pinned = AgentMeta {
            fork_at: Some("e1".into()),
            ..copy
        };
        let a = claude_args(&pinned);
        assert!(
            a.windows(3)
                .any(|w| w == ["--resume=s1", "--fork-session", "--resume-session-at=e1"]),
            "{a:?}"
        );
        // Its own session once a turn gave it one: the original's is read no more.
        let own = AgentMeta {
            session_id: Some("s2".into()),
            ..pinned
        };
        let a = claude_args(&own);
        assert!(a.contains(&"--resume=s2".to_string()), "{a:?}");
        assert!(
            !a.iter().any(|x| x == "--fork-session"
                || x == "--resume=s1"
                || x.starts_with("--resume-session-at")),
            "{a:?}"
        );
    }

    #[test]
    fn a_copys_branch_keeps_its_marker_within_a_slug() {
        assert_eq!(
            copy_branch("refacto-auth", "refacto-auth (copie)"),
            "escouade/refacto-auth-copie"
        );
        assert_eq!(
            copy_branch("refacto-auth", "refacto-auth (copie 2)"),
            "escouade/refacto-auth-copie-2"
        );
        // A ticket agent's name already takes the 40 characters of a slug: it gives way.
        let ticket = "dem-12-ajouter-la-pagination-aux-evenements";
        let slug = |b: &str| b.strip_prefix("escouade/").unwrap().to_string();
        for (copy, marker) in [
            (format!("{ticket} (copie)"), "-copie"),
            (format!("{ticket} (copie 12)"), "-copie-12"),
        ] {
            let branch = slug(&copy_branch(ticket, &copy));
            assert!(branch.len() <= 40, "{branch}");
            assert!(branch.ends_with(marker), "{branch}");
            assert!(
                branch.starts_with("dem-12-ajouter-la-pagination"),
                "{branch}"
            );
            assert!(!branch.contains("--"), "{branch}");
        }
        // A single word longer than a slug: cut inside it.
        let word = "a".repeat(45);
        assert_eq!(
            copy_branch(&word, &format!("{word} (copie)")),
            format!("escouade/{}-copie", "a".repeat(34))
        );
        // Nothing to make a slug of: the marker alone.
        assert_eq!(copy_branch("日本", "日本 (copie)"), "escouade/copie");
    }

    #[test]
    fn a_copy_is_named_after_its_original_and_numbered_once_taken() {
        let taken = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            copy_name("refacto-auth", &taken(&["refacto-auth"])),
            "refacto-auth (copie)"
        );
        assert_eq!(
            copy_name(
                "refacto-auth",
                &taken(&["refacto-auth", "refacto-auth (copie)"])
            ),
            "refacto-auth (copie 2)"
        );
        assert_eq!(
            copy_name(
                "refacto-auth",
                &taken(&["refacto-auth (copie)", "refacto-auth (copie 2)"])
            ),
            "refacto-auth (copie 3)"
        );
    }

    #[test]
    fn the_protocol_is_appended_to_the_system_prompt_on_one_line() {
        let m = AgentMeta {
            model: "sonnet".into(),
            effort: "high".into(),
            mode: "auto".into(),
            append_prompt: Some("Ligne 1\nLigne 2 « % & \" »".into()),
            ..Default::default()
        };
        let a = claude_args(&m);
        let i = a
            .iter()
            .position(|x| x == "--append-system-prompt")
            .unwrap();
        assert_eq!(a[i + 1], "Ligne 1 Ligne 2 « % & \" »");
        assert!(!claude_args(&AgentMeta::default()).contains(&"--append-system-prompt".to_string()));
        // Blank: nothing to append.
        let blank = AgentMeta {
            append_prompt: Some(" \n ".into()),
            ..Default::default()
        };
        assert!(!claude_args(&blank).contains(&"--append-system-prompt".to_string()));
    }

    #[test]
    fn the_log_line_of_a_start_does_not_copy_the_protocol() {
        let m = AgentMeta {
            model: "opus".into(),
            effort: "max".into(),
            mode: "plan".into(),
            session_id: Some("s1".into()),
            append_prompt: Some("Protocole « secret » du ticket ATL-42".into()),
            ..Default::default()
        };
        let line = args_for_log(&claude_args(&m));
        assert!(
            !line.contains("secret") && !line.contains("ATL-42"),
            "{line}"
        );
        // The flag stays, its value is only counted; the arguments around it are untouched.
        assert!(
            line.contains("--append-system-prompt <37 chars> --resume=s1"),
            "{line}"
        );
        assert!(line.contains("--model opus --effort max --permission-mode plan"));
        // Without a protocol, the line is the arguments as they are.
        let plain = claude_args(&AgentMeta::default());
        assert_eq!(args_for_log(&plain), plain.join(" "));
        // A flag ending the list has no value to elide.
        assert_eq!(
            args_for_log(&["--append-system-prompt".to_string()]),
            "--append-system-prompt"
        );
    }

    /// The command line of a `.cmd` launcher (npm's claude.cmd) is cmd.exe's, which takes 8191
    /// characters in all: every argument counts as Rust escapes it (`board::escaped_len`) between
    /// its quotes, with a space before it, behind the launcher's path and cmd.exe's own prefix.
    fn command_line_weight(program: &str, args: &[String]) -> usize {
        const CMD_PREFIX: &str = "C:\\Windows\\System32\\cmd.exe /e:ON /v:OFF /d /c \"";
        CMD_PREFIX.len()
            + program.len()
            + 2
            + args
                .iter()
                .map(|a| 1 + 2 + crate::board::escaped_len(a))
                .sum::<usize>()
    }

    #[test]
    fn a_ticket_agents_whole_command_line_fits_cmds_limit_in_the_worst_case() {
        // Criteria and a title made of what a `.cmd` argument escapes the most (`%`, `"`, `\`),
        // accents, and every kind of line break; far more of them than fit.
        let nasty = "%\"\\é\r\n%\"\\\\\"à\n".repeat(40);
        let t = Ticket {
            id: "t1".into(),
            project_id: "p1".into(),
            key: "ATL-42".into(),
            title: nasty.clone(),
            criteria: (0..60)
                .map(|_| Criterion {
                    text: nasty.clone(),
                    ..Default::default()
                })
                .collect(),
            max_loops: 5,
            ..Default::default()
        };
        // A long launcher path, as npm puts it under a user profile with a long name.
        let program = "C:\\Users\\guillaume.gagnaire-lefebvre\\AppData\\Roaming\\npm\\claude.cmd";
        for ports in [None, Some(4100)] {
            let m = AgentMeta {
                model: "claude-opus-4-5-20251101[1m]".into(),
                effort: "max".into(),
                mode: "bypassPermissions".into(),
                session_id: Some("0f8fad5b-d9cb-469f-a165-70867728950e".into()),
                append_prompt: Some(crate::board::protocol_prompt(&t, ports)),
                ..Default::default()
            };
            let args = claude_args(&m);
            assert!(
                args.contains(&"--append-system-prompt".to_string()),
                "{ports:?}"
            );
            assert!(
                args.iter().all(|a| !a.contains('\n') && !a.contains('\r')),
                "{ports:?}: a line break in an argument"
            );
            let weight = command_line_weight(program, &args);
            assert!(weight < 8191, "{ports:?}: {weight} of 8191");
            // The criteria are cut where they stop fitting, which leaves some of the protocol's
            // budget unused: with all of it used, the line must still fit.
            let protocol = m.append_prompt.as_deref().unwrap();
            let unused = crate::board::PROTOCOL_BUDGET - crate::board::escaped_len(protocol);
            println!(
                "worst-case command line, ports {ports:?}: {weight} of 8191, {} with the whole protocol budget used",
                weight + unused
            );
            assert!(
                weight + unused < 8191,
                "{ports:?}: {} of 8191 with the whole protocol budget used",
                weight + unused
            );
        }
    }

    #[test]
    fn sub_prefixes() {
        assert_eq!(sub_prefix("C:/code/app", "C:\\code\\app"), "");
        assert_eq!(
            sub_prefix("C:/code/mono", "C:/code/mono/packages/web"),
            "packages/web/"
        );
    }

    #[test]
    fn a_folder_below_the_root_is_given_from_it_and_no_other() {
        let root = "C:/code/mono";
        assert_eq!(
            below_root(root, "C:\\code\\mono\\packages\\web").as_deref(),
            Some("packages/web")
        );
        assert_eq!(
            below_root(root, "c:/CODE/mono/.claude/worktrees/dem-1").as_deref(),
            Some(".claude/worktrees/dem-1")
        );
        for elsewhere in [
            "C:/code/mono",
            "C:/code/mono/",
            "C:/code/other",
            "D:/mono/x",
        ] {
            assert_eq!(below_root(root, elsewhere), None, "{elsewhere}");
        }
    }

    #[test]
    fn the_editor_keeps_the_worktrees_of_the_projects_folder_and_of_its_root() {
        assert_eq!(
            worktrees_kept("C:/code/app", "C:\\code\\app", false),
            vec![".claude/worktrees"]
        );
        assert_eq!(
            worktrees_kept("C:/code/mono", "C:/code/mono/packages/web", false),
            vec![".claude/worktrees", "packages/web/.claude/worktrees"]
        );
        // An agent's worktree holds none of the project's.
        assert_eq!(
            worktrees_kept("C:/code/mono/.claude/worktrees/dem-1", "C:/code/mono", true),
            vec![".claude/worktrees"]
        );
    }

    fn att(name: &str, media_type: &str, data: &str) -> Attachment {
        Attachment {
            name: name.into(),
            media_type: media_type.into(),
            data: data.into(),
        }
    }

    #[test]
    fn a_message_without_attachments_is_plain_text() {
        assert_eq!(user_content("Bonjour", &[]).unwrap(), json!("Bonjour"));
    }

    #[test]
    fn attachments_come_before_the_text_as_content_blocks() {
        let c = user_content(
            "Résume",
            &[
                att("capture.png", "image/png", "iVBO"),
                att("rapport.pdf", "application/pdf", "JVBE"),
                att("notes.md", "text/plain", "# Notes\nà faire"),
            ],
        )
        .unwrap();
        assert_eq!(
            c,
            json!([
                { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "iVBO" } },
                { "type": "document", "title": "rapport.pdf",
                  "source": { "type": "base64", "media_type": "application/pdf", "data": "JVBE" } },
                { "type": "document", "title": "notes.md",
                  "source": { "type": "text", "media_type": "text/plain", "data": "# Notes\nà faire" } },
                { "type": "text", "text": "Résume" },
            ])
        );
    }

    #[test]
    fn a_file_alone_is_sent_without_an_empty_text_block() {
        let c = user_content("", &[att("a.pdf", "application/pdf", "JVBE")]).unwrap();
        assert_eq!(c.as_array().unwrap().len(), 1);
    }

    #[test]
    fn unsupported_or_oversized_attachments_are_refused() {
        let e = user_content("x", &[att("plan.docx", "application/msword", "UEsD")]).unwrap_err();
        assert!(e.to_string().contains("plan.docx"), "{e}");
        // The API reads only these image formats.
        assert!(user_content("x", &[att("a.bmp", "image/bmp", "Qk0=")]).is_err());
        let big = "A".repeat(7 * 1024 * 1024);
        let e = user_content("x", &[att("photo.png", "image/png", &big)]).unwrap_err();
        assert!(e.to_string().contains("5 Mo"), "{e}");
    }

    #[test]
    fn text_pdf_and_whole_message_sizes_fit_what_claude_takes() {
        // Past ~256 KB, a text file alone would fill Claude's context.
        let ok = "a".repeat(256 * 1024);
        assert!(user_content("x", &[att("log.txt", "text/plain", &ok)]).is_ok());
        let e = user_content("x", &[att("log.txt", "text/plain", &(ok + "a"))]).unwrap_err();
        assert!(e.to_string().contains("256 Ko"), "{e}");
        // A PDF of 18 MB fits in the whole message's limit.
        let pdf = "A".repeat(19 * MB / 3 * 4);
        let e = user_content("x", &[att("a.pdf", "application/pdf", &pdf)]).unwrap_err();
        assert!(e.to_string().contains("18 Mo"), "{e}");
        let pdf = "A".repeat(18 * MB / 3 * 4);
        assert!(user_content("x", &[att("a.pdf", "application/pdf", &pdf)]).is_ok());
        // Together, the files stay under the API's request size.
        let part = "A".repeat(10 * MB);
        let three = ["a.pdf", "b.pdf", "c.pdf"].map(|n| att(n, "application/pdf", &part));
        let e = user_content("x", &three).unwrap_err();
        assert!(e.to_string().contains("18 Mo en tout"), "{e}");
        assert!(user_content("x", &three[..2]).is_ok());
    }

    #[test]
    fn base64_padding_is_not_counted_as_data() {
        // Exactly 5 MB: 5 MB ≡ 2 (mod 3), so its base64 ends with one '='.
        let n = 5 * MB;
        let b64 = format!("{}=", "A".repeat((n / 3 + 1) * 4 - 1));
        assert!(user_content("x", &[att("p.png", "image/png", &b64)]).is_ok());
    }
}
