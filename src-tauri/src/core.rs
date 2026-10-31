//! Application core: projects, agents and their Claude processes, git, usage, persistence.

use crate::accounts;
use crate::agent::{AgentAlert, AgentHandle, AgentRt, Effects, NotifyKind, RemoteLink};
use crate::attachfiles::{self, AttachedFile};
use crate::board;
use crate::closed;
use crate::conv;
use crate::convsearch;
use crate::engine::{self, EngineProcess, Escouade, OneShot, Process, Remote, Spawn, Weight};
use crate::fsedit;
use crate::git::{self, GitService};
use crate::hub::Hub;
use crate::i18n::{self, LangInfo};
use crate::integrations;
use crate::isola;
use crate::job::JobUsage;
use crate::menus;
use crate::model::*;
use crate::notify;
use crate::paths::{self, DataDir};
use crate::planfiles;
use crate::pty::{PtyManager, ShellInfo};
use crate::resources;
use crate::stats::{AgentLabel, Labels, Stats, StatsView, TicketLabel};
use crate::taskout::TaskOutput;
use crate::telemetry::{Event, Telemetry};
use crate::testlaunch;
use crate::usage;
use crate::worktrees::{self, RunSuggestion, WorktreeSuggestion};
use anyhow::{anyhow, bail, Context, Result};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Runtime, Wry};
use tokio::sync::mpsc;

/// The process of an agent (of the harness named) exited while starting; the exit handler recorded
/// why in the conversation.
#[derive(Debug)]
pub struct StartupFailure(pub &'static str);

impl std::fmt::Display for StartupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.0;
        f.write_str(&tr!(
            "{name} n'a pas pu démarrer : voir le détail dans la conversation.",
            "{name} couldn’t start: see the details in the conversation."
        ))
    }
}

impl std::error::Error for StartupFailure {}

/// Refused: the agent asked for is gone (deleted meanwhile).
pub(crate) fn agent_not_found() -> anyhow::Error {
    anyhow!(tr!("agent introuvable", "agent not found"))
}

/// What a deleted agent's warning says when `running` of its processes still ran `secs` seconds
/// after it was killed: they may hold its worktree.
pub(crate) fn agent_still_running(lang: i18n::Lang, running: u32, secs: u64) -> String {
    tr_n_in!(
        lang,
        running,
        "{running} processus de l'agent tournait encore après {secs} s et peut retenir son worktree",
        "{running} processus de l'agent tournaient encore après {secs} s et peuvent retenir son worktree",
        "{running} process of the agent was still running after {secs} s and may hold its worktree",
        "{running} processes of the agent were still running after {secs} s and may hold its worktree"
    )
}

/// Refused: the project asked for is gone (removed meanwhile).
pub(crate) fn project_not_found() -> anyhow::Error {
    anyhow!(tr!("projet introuvable", "project not found"))
}

/// Refused: the folder is not (or no longer) in a git repository.
pub(crate) fn not_a_repo() -> anyhow::Error {
    anyhow!(tr!("pas un dépôt git", "not a git repository"))
}

/// Where the Remote Control an agent should have comes from, if it should have any: the user's
/// own choice for it first, then the global setting, then its project's. Kept apart by hand (or
/// archived), it has none whatever the settings say.
pub(crate) fn remote_wanted(meta: &AgentMeta, global: bool, project: bool) -> Option<RemoteSource> {
    if meta.archived {
        return None;
    }
    if meta.remote_control && meta.remote_source == Some(RemoteSource::Hand) {
        return Some(RemoteSource::Hand);
    }
    if meta.remote_off {
        return None;
    }
    if global {
        Some(RemoteSource::Global)
    } else if project {
        Some(RemoteSource::Project)
    } else {
        None
    }
}

/// Closes the remote session `bridge` of the agent `id` for good, through its process linked to
/// claude.ai. Turning Remote Control off alone leaves a session opened to be kept
/// (`keep_session_on_exit`, which a restart needs to reattach it) on claude.ai, orphan: it is
/// turned off, reattached under its name without that option, then turned off again, which
/// archives it (seen on Claude Code 2.1.300). With no id of the session, it is turned off, no
/// more. A request that fails is logged and ends it: the session stays on claude.ai, and nothing
/// waits for it. True when every request worked: the session is closed (`Core::close_link` then
/// forgets it; until then the agent keeps its id).
pub(crate) async fn close_remote_session(
    id: &str,
    proc: &Process,
    bridge: Option<&str>,
    name: &str,
    timeout: Duration,
) -> bool {
    let mut requests = vec![Remote::Off];
    if let Some(bridge) = bridge {
        requests.push(Remote::On {
            name: name.to_string(),
            keep: false,
            reattach: Some(bridge.to_string()),
        });
        requests.push(Remote::Off);
    }
    for request in requests {
        if let Err(e) = proc.remote_control(request, timeout).await {
            log::warn!("agent {id}: remote session not closed: {e:#}");
            return false;
        }
    }
    true
}

/// How long the models an account listed serve before its menu reads them again (ESC-186).
const MODELS_KEPT_MS: i64 = 24 * 3600 * 1000;

/// Whether the models of an account of `engine` are read by account, without an agent: every
/// harness of the registry's (Claude Code's by its `initialize`, ESC-124; since ESC-186 Codex's
/// `model/list`, Copilot CLI's `models.list`, Cursor CLI's `cursor/list_available_models`,
/// OpenCode's `opencode models`; since ESC-266 Mistral Vibe's `configOptions`), none of a harness
/// absent.
fn reads_account_models(engine: EngineId) -> bool {
    crate::engine::written(engine)
}

/// Refused: the app is quitting, no agent process starts any more.
pub(crate) fn quitting_refusal() -> anyhow::Error {
    anyhow!(tr!(
        "Escouade se ferme : aucun agent ne démarre.",
        "Escouade is quitting: no agent starts."
    ))
}

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

impl NotOnBase {
    /// The refusal in words, where the window does not ask its question.
    pub fn text(&self, lang: i18n::Lang) -> String {
        let (current, base) = (&self.current, &self.base);
        if current.is_empty() {
            tr_in!(
                lang,
                "Le projet n'est sur aucune branche (HEAD détachée) : bascule sur « {base} » avant de merger.",
                "The project isn’t on any branch (detached HEAD): switch to “{base}” before merging."
            )
        } else {
            tr_in!(
                lang,
                "Le projet est sur la branche « {current} » : bascule sur « {base} » avant de merger.",
                "The project is on the branch “{current}”: switch to “{base}” before merging."
            )
        }
    }
}

impl std::fmt::Display for NotOnBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text(i18n::ui()))
    }
}

impl std::error::Error for NotOnBase {}

/// A branch operation on a project's folder refused for a reason the window tells apart by its
/// code (`wire`), as it does `NotOnBase`; `text` says it where it does not.
#[derive(Debug)]
pub enum BranchRefusal {
    /// Uncommitted changes in the project's folder: a switch would carry them over or fail on them.
    Dirty,
    /// The branch is checked out in an agent's worktree.
    InWorktree {
        branch: String,
        agent_id: String,
        agent: String,
    },
    /// An agent without worktree is in the middle of a turn in the project's folder.
    AgentWorking { agent_id: String, agent: String },
    /// Not merged into the project's base: `commits` of it are in no other branch.
    Unmerged {
        branch: String,
        base: String,
        commits: u32,
    },
}

impl BranchRefusal {
    /// What the window gets: `DIRTY`, `IN_WORKTREE:<agent id>:<agent name>`,
    /// `AGENT_WORKING:<agent id>:<agent name>`, `UNMERGED:<commits>`. An agent's id holds no `:`;
    /// its name, last, may.
    pub fn wire(&self) -> String {
        match self {
            BranchRefusal::Dirty => "DIRTY".into(),
            BranchRefusal::InWorktree {
                agent_id, agent, ..
            } => format!("IN_WORKTREE:{agent_id}:{agent}"),
            BranchRefusal::AgentWorking { agent_id, agent } => {
                format!("AGENT_WORKING:{agent_id}:{agent}")
            }
            BranchRefusal::Unmerged { commits, .. } => format!("UNMERGED:{commits}"),
        }
    }

    /// The refusal in words, where the window does not tell it by its code.
    pub fn text(&self, lang: i18n::Lang) -> String {
        match self {
            BranchRefusal::Dirty => tr_in!(
                lang,
                "Il reste des changements non commités dans le dossier du projet : commite-les ou mets-les de côté (stash) avant de changer de branche.",
                "There are uncommitted changes in the project’s folder: commit or stash them before switching branches."
            ),
            BranchRefusal::InWorktree { branch, agent, .. } => tr_in!(
                lang,
                "La branche « {branch} » est utilisée par l’agent {agent}, dans son worktree.",
                "The branch “{branch}” is used by agent {agent}, in its worktree."
            ),
            BranchRefusal::AgentWorking { agent, .. } => tr_in!(
                lang,
                "L’agent {agent} travaille dans le dossier du projet : attends la fin de son tour.",
                "Agent {agent} is working in the project’s folder: wait for the end of its turn."
            ),
            BranchRefusal::Unmerged {
                branch,
                base,
                commits,
            } => tr_n_in!(
                lang,
                *commits,
                "« {branch} » n’est pas mergée dans « {base} » : {n} commit n’est dans aucune autre branche.",
                "« {branch} » n’est pas mergée dans « {base} » : {n} commits ne sont dans aucune autre branche.",
                "“{branch}” isn’t merged into “{base}”: {n} commit is in no other branch.",
                "“{branch}” isn’t merged into “{base}”: {n} commits are in no other branch.",
                n = commits
            ),
        }
    }
}

impl std::fmt::Display for BranchRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text(i18n::ui()))
    }
}

impl std::error::Error for BranchRefusal {}

/// True when the work of `refname` is in one of `bases` (`Core::merge_bases`), none of which it is:
/// the base is never merged "into itself", its own commits may be in it alone.
async fn merged_into_any(root: &str, refname: &str, bases: &[String]) -> bool {
    if bases.iter().any(|b| b == refname) {
        return false;
    }
    for base in bases {
        if git::is_merged(root, refname, base).await {
            return true;
        }
    }
    false
}

/// The changes of the project's folder put aside before a switch: the stash's name, and its commit
/// (the stash is found again by it, whatever was stashed since).
#[derive(Debug, Clone)]
pub(crate) struct Stash {
    pub message: String,
    pub hash: String,
}

/// The error of a switch that failed, and when the changes put aside could not come back either,
/// where they are (`take_back`'s error) after it.
fn failed_with(failed: anyhow::Error, restore: Result<()>) -> anyhow::Error {
    match restore {
        Ok(()) => failed,
        Err(kept) => anyhow::anyhow!("{failed:#}\n{kept:#}"),
    }
}

/// The name of the stash a switch to `branch` puts the folder's changes in: what `git stash list`
/// shows, to find them again.
pub fn stash_message(lang: i18n::Lang, branch: &str) -> String {
    tr_in!(
        lang,
        "escouade: avant de passer sur {branch}",
        "escouade: before switching to {branch}"
    )
}

/// Refused: the branch an agent was asked to work on is the one the project's folder is on.
pub(crate) fn folder_branch_refusal(lang: i18n::Lang) -> String {
    tr_in!(
        lang,
        "C’est la branche du dossier du projet : un agent sans worktree y travaille déjà, ou change de branche d’abord.",
        "That’s the branch of the project’s folder: an agent without a worktree already works on it, or switch to another branch first."
    )
}

/// Why an agent's branch keeps its name as the agent is named (`rename_agent_branch`).
#[derive(Debug, Clone, PartialEq)]
pub enum BranchKept {
    /// It was there before the agent: the user's.
    Existing,
    /// Made for the agent under the name the user typed (ESC-155).
    Typed,
    /// Its pull request goes by its name (ESC-64).
    PullRequest,
    /// It has an upstream, or a remote has it: renamed, it would part from it.
    Pushed,
    /// The name it would take: another branch has it.
    Taken(String),
    /// Git refused the rename.
    Failed(String),
}

/// What the user is told when the branch `branch` of an agent they renamed keeps its name.
pub fn branch_kept(lang: i18n::Lang, branch: &str, why: &BranchKept) -> String {
    match why {
        BranchKept::Existing => tr_in!(
            lang,
            "La branche « {branch} » garde son nom : elle existait avant l’agent.",
            "The branch “{branch}” keeps its name: it was there before the agent."
        ),
        BranchKept::Typed => tr_in!(
            lang,
            "La branche « {branch} » garde le nom que tu lui as donné.",
            "The branch “{branch}” keeps the name you gave it."
        ),
        BranchKept::PullRequest => tr_in!(
            lang,
            "La branche « {branch} » garde son nom : elle a une pull request.",
            "The branch “{branch}” keeps its name: it has a pull request."
        ),
        BranchKept::Pushed => tr_in!(
            lang,
            "La branche « {branch} » garde son nom : elle est déjà poussée.",
            "The branch “{branch}” keeps its name: it’s pushed already."
        ),
        BranchKept::Taken(other) => tr_in!(
            lang,
            "La branche « {branch} » garde son nom : « {other} » existe déjà.",
            "The branch “{branch}” keeps its name: “{other}” already exists."
        ),
        BranchKept::Failed(e) => tr_in!(
            lang,
            "La branche « {branch} » garde son nom : {e}",
            "The branch “{branch}” keeps its name: {e}"
        ),
    }
}

/// Where a new agent's worktree starts (ESC-160): `over`, the branch chosen for it at its making,
/// else the project's « Branche de départ des nouveaux worktrees ». None: HEAD of the project's
/// folder, with the reason when it was to start elsewhere (a branch gone: `start_from`). A branch
/// named that is a remote one alone (`origin/x`) starts through the local branch made to track it
/// (`x`), as « Nouvel agent sur une branche… » takes one up: its base is a local branch, which
/// « Intégrer », its merge and its pull request go to. No fetch: the agent is made at once.
pub(crate) async fn start_point(
    project: &Project,
    root: &str,
    over: Option<&str>,
) -> (Option<String>, Option<String>) {
    let base = match over.map(str::trim).filter(|b| !b.is_empty()) {
        Some(b) => WorktreeBase::Branch(b.to_string()),
        None => project.worktree_base.clone(),
    };
    match base {
        WorktreeBase::Head => (None, None),
        WorktreeBase::Target => match project.board.target.trim() {
            "" => (None, None),
            target => {
                let gone = tr!(
                    "La branche cible « {target} » n’existe pas : l’agent part de la branche du dossier du projet.",
                    "The target branch “{target}” doesn’t exist: the agent starts from the branch of the project’s folder."
                );
                start_from(root, target, gone).await
            }
        },
        WorktreeBase::Branch(name) => {
            let gone = tr!(
                "La branche de départ « {name} » n’existe plus : l’agent part de la branche du dossier du projet.",
                "The starting branch “{name}” no longer exists: the agent starts from the branch of the project’s folder."
            );
            let local = match git::local_of(root, &name).await {
                Ok(Some(local)) => local,
                // A remote branch no local one tracks: the local one made to track it.
                Ok(None) => match git::branch_track(root, &name).await {
                    Ok(local) => local,
                    Err(e) => {
                        return (
                            None,
                            Some(tr!(
                                "La branche locale qui suivrait « {name} » n’a pas pu être créée ({e:#}) : l’agent part de la branche du dossier du projet.",
                                "The local branch that would track “{name}” couldn’t be made ({e:#}): the agent starts from the branch of the project’s folder."
                            )),
                        )
                    }
                },
                // Neither a local branch nor a remote one any more.
                Err(_) if !git::ref_exists(root, &format!("refs/remotes/{name}")).await => {
                    return (None, Some(gone))
                }
                Err(e) => {
                    return (
                        None,
                        Some(tr!(
                            "{e:#} L’agent part de la branche du dossier du projet.",
                            "{e:#} The agent starts from the branch of the project’s folder."
                        )),
                    )
                }
            };
            start_from(root, &local, gone).await
        }
    }
}

/// A new agent starts from the local branch `branch`, moved first to its upstream as fetched
/// last (no fetch: the agent is made at once) when it is only behind it, so that what it starts
/// from is its base too. When it can't be moved (checked out, commits not pushed) the agent starts
/// from it as it is, and the sentence says so; with no such local branch, from HEAD (None), `gone`
/// said.
async fn start_from(root: &str, target: &str, gone: String) -> (Option<String>, Option<String>) {
    if !git::branch_exists(root, target).await {
        return (None, Some(gone));
    }
    let from = Some(target.to_string());
    let Some(up) = git::upstream_of(root, target).await else {
        return (from, None);
    };
    let remote = format!("{}/{}", up.remote, up.branch);
    let local = format!("refs/heads/{target}");
    let behind = git::ahead_of(root, &local, &up.tracking).await.unwrap_or(0);
    if behind == 0 {
        return (from, None);
    }
    let commits = |n: u32| {
        if n == 1 {
            "1 commit".to_string()
        } else {
            format!("{n} commits")
        }
    };
    let ahead = git::ahead_of(root, &up.tracking, &local).await.unwrap_or(0);
    let why = if ahead > 0 {
        tr!(
            "« {target} » a des commits non poussés et {remote} {theirs} qu’elle n’a pas : l’agent part de la « {target} » locale, sans eux.",
            "“{target}” has commits not pushed and {remote} {theirs} it doesn’t have: the agent starts from the local “{target}”, without them.",
            theirs = commits(behind)
        )
    } else {
        match git::fast_forward_branch(root, target, &up.tracking).await {
            Ok(()) => return (from, None),
            Err(e) => tr!(
                "{remote} a {theirs} que « {target} » n’a pas, mais elle n’a pas pu avancer ({e}) : l’agent part de la « {target} » locale, sans eux.",
                "{remote} has {theirs} “{target}” doesn’t have, but it couldn’t move forward ({e}): the agent starts from the local “{target}”, without them.",
                theirs = commits(behind)
            ),
        }
    };
    (from, Some(why))
}

/// Refused: the branch an agent (or a ticket's) was asked to work on is the board's target branch,
/// which the tickets are merged into (the Kanban's, not an agent's to work on directly).
pub(crate) fn target_branch_refusal(lang: i18n::Lang) -> String {
    tr_in!(
        lang,
        "C’est la branche cible du Kanban : un agent n’y travaille pas directement.",
        "It’s the board’s target branch: an agent doesn’t work on it directly."
    )
}

/// « Intégrer <base> » refused: the agent's worktree has changes of its own.
pub(crate) fn integrate_dirty(lang: i18n::Lang) -> String {
    tr_in!(
        lang,
        "Commite ou mets de côté les changements de l’agent d’abord.",
        "Commit or stash the agent’s changes first."
    )
}

/// « Intégrer <base> » refused during the turn of the agent `name`.
pub(crate) fn integrate_busy(lang: i18n::Lang, name: &str, base: &str) -> String {
    tr_in!(
        lang,
        "Attends la fin du tour de {name} pour intégrer {base}.",
        "Wait for the end of {name}’s turn to integrate {base}."
    )
}

/// « Intégrer <base> » refused for an archived agent.
pub(crate) fn integrate_archived(lang: i18n::Lang, name: &str, base: &str) -> String {
    tr_in!(
        lang,
        "{name} est archivé : restaure-le pour intégrer {base}.",
        "{name} is archived: restore it to integrate {base}."
    )
}

/// « Intégrer <base> » refused: the base branch is gone.
pub(crate) fn integrate_base_gone(lang: i18n::Lang, base: &str) -> String {
    tr_in!(
        lang,
        "La branche de base « {base} » n’existe plus.",
        "The base branch “{base}” no longer exists."
    )
}

/// « Intégrer <base> » refused: `operation` (a merge, a rebase…) is under way in the agent's
/// worktree already, which it has to finish first.
pub(crate) fn integrate_under_way(lang: i18n::Lang, name: &str, operation: &str) -> String {
    tr_in!(
        lang,
        "Un {operation} est en cours dans le worktree de {name} : il doit le terminer d’abord.",
        "A {operation} is under way in {name}’s worktree: it has to finish it first."
    )
}

/// What an agent without a ticket is told when the merge of its base into its branch stopped on
/// conflicts, left in its worktree (a ticket's agent is told `board::conflict_message`, which asks
/// for the report its ticket's loop reads).
pub(crate) fn integrate_conflict_message(lang: i18n::Lang, base: &str, files: &[String]) -> String {
    tr_in!(
        lang,
        "Le merge de {base} dans ta branche a des conflits sur : {files}. Résous-les, puis commite le merge.",
        "The merge of {base} into your branch has conflicts in: {files}. Resolve them, then commit the merge.",
        files = files.join(", ")
    )
}

/// What an agent without a ticket is told when the rebase of its branch onto its base stopped on
/// conflicts (undone: it does the rebase itself).
pub(crate) fn integrate_rebase_message(lang: i18n::Lang, base: &str) -> String {
    tr_in!(
        lang,
        "Rebase ta branche sur {base} et résous ses conflits.",
        "Rebase your branch onto {base} and resolve its conflicts."
    )
}

/// Refused: the agent works in the project's folder, not in a worktree of its own.
fn agent_without_worktree() -> anyhow::Error {
    anyhow!(tr!(
        "cet agent n'a pas de worktree",
        "this agent has no worktree"
    ))
}

/// A sync of a project's checkout with its remote, asked by the user.
#[derive(Debug, Clone, Copy)]
pub enum SyncOp {
    Fetch,
    Pull,
    Push,
}

/// Who answers a one-shot question (`Core::one_shot_who`).
pub(crate) struct OneShotWho {
    pub engine: EngineId,
    /// As the interface names it (`Claude Haiku`).
    pub model: String,
    /// What the price grid knows it by (`haiku`).
    pub priced: String,
    pub account: Account,
}

/// A one-shot question to a harness (`Core::ask_harness`).
struct Ask<'a> {
    weight: Weight,
    cwd: &'a Path,
    system: &'a str,
    prompt: &'a str,
    limit: Duration,
    /// The agent it is about: asked of its account.
    agent: Option<&'a str>,
    /// The project it is about, when no agent is: asked of the account a new agent of it would
    /// go to (as one about neither is: of the first usable account).
    project: Option<&'a str>,
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

/// How long a killed agent's process tree is waited for before its worktree is removed: under
/// Windows, what still runs keeps the folder, and a tree can take long to go on a loaded machine.
const AGENT_GONE: Duration = Duration::from_secs(15);

/// How long the name of a new agent whose worktree waits for its first message (ESC-156) is
/// waited for before its worktree is made under the name it has (`agent-N`): the writer’s model
/// answers in a few seconds, a cold start of its harness included. Past it, the name, come later,
/// renames the branch as for any agent.
const NAMING_LIMIT: Duration = Duration::from_secs(15);

/// How long a deleted agent's worktree waits for the project's running git refresh, which has
/// `git` commands in the folder.
const GIT_IDLE: Duration = Duration::from_secs(10);

/// How often every repository with a remote is fetched in the background.
const FETCH_EVERY: Duration = Duration::from_secs(5 * 60);

/// How many processes and interfaces a Remote Control setting (or the app's start) starts at once:
/// a setting turned on for every agent would otherwise start them all together.
const REMOTE_STARTS: usize = 3;
/// How long a start of an interface that a setting makes keeps its place among them, at most:
/// until the interface is up (`Core::wait_interface_ready`), so that they do not all begin together.
const REMOTE_START_READY: Duration = Duration::from_secs(15);

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

const KB: usize = 1024;
pub(crate) const MB: usize = 1024 * KB;
/// All the files of a message: sent in base64 (4/3 bigger), under the API's 32 MB a request.
pub(crate) const MAX_TOTAL: usize = 18 * MB;
/// An image, as the API takes it.
pub(crate) const MAX_IMAGE: usize = 5 * MB;

pub(crate) fn size_label(lang: i18n::Lang, bytes: usize) -> String {
    if bytes >= MB {
        tr_in!(lang, "{n} Mo", "{n} MB", n = bytes / MB)
    } else {
        tr_in!(lang, "{n} Ko", "{n} KB", n = bytes / KB)
    }
}

/// `text`, written by `origin` (a caller of the MCP server) and not by the user: « Message de
/// <origin> : <text> ». The author is one line (an agent's name, « Claude (hors Escouade) »).
pub(crate) fn from_origin(lang: i18n::Lang, origin: &str, text: &str) -> String {
    let origin = origin.split_whitespace().collect::<Vec<_>>().join(" ");
    tr_in!(
        lang,
        "Message de {origin} : {text}",
        "Message from {origin}: {text}"
    )
}

/// What « Intégrer <base> » did (`Core::integrate_base`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Integration {
    /// The branch had all of the base already.
    UpToDate,
    Done,
    /// Stopped on conflicts in `files`: the agent was asked to resolve them.
    Conflict {
        files: Vec<String>,
        rebase: bool,
    },
}

/// How a new agent differs from the default one (`create_agent_with`).
#[derive(Debug, Clone, Default)]
pub struct AgentOptions {
    /// The harness it runs on; None: its project's (`Project::engine`).
    pub engine: Option<EngineId>,
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
    /// The Claude account it runs on (an `Account`'s id); None: Principal.
    pub account: Option<String>,
    /// A worktree of its own (true) or the project's folder (false), whatever the project does by
    /// default; None: as the project does. `worktree` and `copy_of` name theirs and win.
    pub isolated: Option<bool>,
    /// A worktree on this branch, which exists already (a local one, or a remote one through the
    /// local branch that tracks it), whatever the project's setting; the agent is not created
    /// without it. Refused for the branch of the project's folder and for one another worktree
    /// has. Wins over `worktree`, `isolated` and `copy_of`.
    pub branch: Option<String>,
    /// With `branch`: the branch it is merged into. None: the project's (the board's target while
    /// it exists, else the branch the folder is on). With `new_branch`, or for an agent of its own
    /// worktree (which it then asks for, made at once or at its first message): the branch it
    /// starts from, and so its base, a local or a remote one (ESC-160). None: where the project's
    /// new worktrees start (`Project::worktree_base`).
    pub base: Option<String>,
    /// With `branch`: a new branch of that name, made first (checked as git does, refused when it
    /// exists) from where a new agent starts (`agent_worktree`), and the agent's own.
    pub new_branch: bool,
    /// Set apart from Remote Control from the start (`AgentMeta::remote_off`): a ticket's next
    /// agent, when the user set the ticket's previous one apart.
    pub remote_off: bool,
    /// Works in this folder (where the project's docs are) when it has no worktree of its own.
    pub cwd: Option<String>,
    /// A discussion of the Docs tab (ESC-261).
    pub discussion: Option<crate::discussion::Discussion>,
}

/// What a copy takes of its original, read while no turn of the original ran.
#[derive(Debug, Clone)]
pub struct CopyOf {
    /// The copy is « <name> (copie) », “<name> (copy)” in English (`copy_name`), never renamed by
    /// Haiku.
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
    /// The original was set apart from Remote Control by hand: so is the copy (it never takes the
    /// Remote Control the original was given by hand, but it keeps the original's refusal).
    pub remote_off: bool,
}

/// Work under way, counted in `Core::works` until dropped.
pub(crate) struct Working<'a>(&'a AtomicUsize);

impl Drop for Working<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// `Working`, for work a task of its own goes on with: it owns the core, so it can be taken before
/// the task is spawned.
pub(crate) struct WorkingOwned<R: Runtime>(Arc<Core<R>>);

impl<R: Runtime> Drop for WorkingOwned<R> {
    fn drop(&mut self) {
        self.0.works.fetch_sub(1, Ordering::AcqRel);
    }
}

pub struct Core<R: Runtime = Wry> {
    /// Itself, for what runs in the background from a method that only has `&self`.
    me: std::sync::Weak<Self>,
    pub app: AppHandle<R>,
    pub data: DataDir,
    pub hub: Hub,
    pub settings: RwLock<Settings>,
    /// The usage statistics, once the user agreed to them (`telemetry`).
    pub telemetry: Arc<Telemetry>,
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
    /// The models the harnesses that list them told (`PersistedState::engine_models`).
    pub engine_models: RwLock<BTreeMap<EngineId, Vec<crate::engine::registry::ToldModel>>>,
    /// What the `claude` of each account told of its models (`PersistedState::account_models`).
    pub account_models: RwLock<BTreeMap<String, crate::model::AccountModels>>,
    /// The accounts whose models are being read (`read_account_models`): one reading at a time.
    models_reading: Mutex<HashSet<String>>,
    pub git: GitService,
    /// The projects' code indexes (`codeindex`).
    pub(crate) codeindex: crate::codeindex::Service,
    /// The installed plugins (`plugins`).
    pub plugins: Arc<crate::plugins::Plugins>,
    /// The store in the app (STORE-9): its catalogue and its revocation list, cached.
    pub store: Arc<crate::plugins::store::Store>,
    /// Their processes (`plugins::runtime`): started at their first activation, closed with the app.
    pub(crate) plugin_runtime: Arc<crate::plugins::runtime::Runtime>,
    /// The facts' bus (PLG-5): what happened, told where the state changes.
    pub facts: crate::facts::Bus,
    /// What hands the facts to the plugins (`plugins::events`).
    pub(crate) plugin_events: Arc<crate::plugins::events::Dispatch>,
    /// The download of the vectors' model under way (ESC-254), by its flag to call it off.
    pub(crate) vectors_download: Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>,
    pub git_cache: RwLock<HashMap<String, GitInfo>>,
    pub pty: PtyManager,
    spawn_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// Held while an interface of the agent quits, which what starts a process or an interface of
    /// the agent waits for (`Core::quit_soon`); apart from the spawn lock, which is not held for
    /// the seconds a quit takes.
    pub(crate) quit_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// Serializes, by agent, what links the agent's process to claude.ai and what closes its remote
    /// session (`link_remote`, `close_link`): two such sequences never interleave their requests.
    remote_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// The projects being closed (`remove_project`): a second closing of one has nothing to do,
    /// and no validation of their tickets starts.
    pub(crate) closing: Mutex<HashSet<String>>,
    /// The histories of the closed projects as the folder holds them (`closed::ClosedEntry`):
    /// read when the app starts, kept up to date by what the app writes and erases.
    pub(crate) closed_index: closed::ClosedIndex,
    /// The ids of the histories whose file could not be removed once the state held the truth
    /// (`PersistedState::leftover_histories`).
    pub(crate) leftover_histories: Mutex<Vec<String>>,
    /// Serializes agent creation so that concurrent creations get distinct names.
    create_lock: tokio::sync::Mutex<()>,
    conv_buffer: Mutex<HashMap<String, Vec<ConvOp>>>,
    conv_flush: tokio::sync::Notify,
    /// Where the agents are in their projects' workflows (ESC-193).
    pub(crate) wf_tracks: Mutex<crate::workflow_track::Store>,
    /// `git config user.name` read in each project's folder (ESC-243), by path: the name of the shared
    /// tickets when none was given (`me_in`).
    pub(crate) git_names: Mutex<HashMap<String, String>>,
    /// When the usage endpoint was last asked, by account.
    pub(crate) last_oauth_call: Mutex<HashMap<String, i64>>,
    /// Where the usage endpoint is (`usage::api_from_env`; a fake one in tests).
    pub(crate) usage_api: RwLock<String>,
    resources: Mutex<resources::Sampler>,
    /// The projects whose git state is being computed (`compute_git`).
    pub(crate) git_inflight: Mutex<std::collections::HashSet<String>>,
    /// One fetch, pull or push at a time per repository.
    sync_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    toplevels: Mutex<HashMap<String, Option<String>>>,
    /// How the look at the files of each agent's plan stands, by agent (`ask_plan_scan`).
    plan_scans: Mutex<HashMap<String, PlanScan>>,
    /// Where the transcripts of the subagents were found, in the Claude Code view
    /// (`subagent_transcript`): the meta files are not looked through at each read.
    pub(crate) sidechains: Mutex<crate::agent::sidechain::Found>,
    dirty: AtomicBool,
    waiting: AtomicUsize,
    pub quitting: AtomicBool,
    /// Files left unsaved in the editor, as the window last said.
    pub unsaved: AtomicUsize,
    /// The agents the window holds a message for (`outbox.svelte.ts`).
    pub held: Mutex<HashSet<String>>,
    /// What the page showed when its process ended abnormally, until the page told it (ESC-290).
    pub page_crash: Mutex<Option<crate::pagecrash::PageCrash>>,
    /// Work under way outside of an agent's turn that a restart would cut (`Core::working`): a
    /// ticket being started, a validation and its cleanup, a message being delivered, a merge.
    pub(crate) works: AtomicUsize,
    /// One scheduling pass of the board at a time.
    pub(crate) board_lock: tokio::sync::Mutex<()>,
    /// Why a ticket launched by hand was not started by the pass that took it up (`StartRefusal`),
    /// until it is launched again: the tool that launched it says it.
    pub(crate) start_refusals: Mutex<HashMap<String, crate::tickets::StartRefusal>>,
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
    /// The autopilot's pauses as the window was last told (`refresh_pause`): the one of the
    /// projects that go to any account, and those of the projects that prefer an account.
    pub(crate) pause_shown: Mutex<(Option<AutopilotPause>, BTreeMap<String, AutopilotPause>)>,
    /// The agents being looked at for a move past « Changer de compte à » (`switch_if_over`).
    pub(crate) switching: Mutex<HashSet<String>>,
    /// The account each agent past it was told it stays on, no other taking it: told once.
    pub(crate) switch_told: Mutex<HashMap<String, String>>,
    /// Blocks of ports reserved for agents being made, or being given one: taken until the agent
    /// holds its own.
    pub(crate) ports_reserved: Mutex<Vec<u16>>,
    /// What each project's docs have to share, as last told to the window, and the conflict a
    /// « Partager » stopped on (`docsshare`).
    pub(crate) docs_shares: Mutex<HashMap<String, crate::docsshare::Memo>>,
    /// One validation's merge at a time per repository (`merge_lock`).
    pub(crate) merge_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    /// One change of the Claude accounts' settings at a time (the accounts' tab, the window's
    /// other settings): each reads the settings and saves them changed, without undoing another.
    pub(crate) claude_accounts_lock: Mutex<()>,
    /// The accounts of the external ticket systems (`integrations.json`), with their secrets.
    pub accounts: RwLock<integrations::Accounts>,
    /// Where the accounts' secrets are kept: the system's keychain (memory in tests).
    pub(crate) secrets: Arc<dyn integrations::secrets::SecretStore>,
    /// Where the Trello and GitHub APIs are.
    pub bases: RwLock<integrations::Bases>,
    /// The GitHub CLI's token, once asked for.
    pub(crate) gh_token: Mutex<Option<String>>,
    /// The code hosts Escouade knows (`forge`).
    pub(crate) forges: RwLock<crate::forge::Registry>,
    /// What the last check of each code host's account found, by host.
    pub(crate) forge_health: Mutex<HashMap<String, crate::forge::connect::Status>>,
    /// The tokens of the code hosts' CLIs (`glab`, `gh` for another instance), once asked for.
    pub(crate) forge_cli_tokens: Mutex<HashMap<String, String>>,
    /// When the pull requests of each agent were last asked of its code host (`agentprs`).
    pub(crate) pr_reads: Mutex<HashMap<String, Instant>>,
    /// The repository each repository on a code host was forked from (`host/namespace/name`),
    /// none for one that is no fork, as its host told it once, or when it did not answer
    /// (`agentprs`).
    pub(crate) repo_parents: Mutex<HashMap<String, crate::agentprs::ParentRead>>,
    /// What the sync of imported tickets is asked to do (a change to tell, failed operations to
    /// try again), one at a time and in order, by a single worker (made at the first one).
    pub(crate) sync_queue: Mutex<Option<integrations::sync::SyncSender>>,
    /// The operations of imported tickets not through yet (`sync-queue.json`).
    pub(crate) pending_syncs: Mutex<integrations::sync::SyncQueue>,
    /// One import at a time (by hand or by label): a ticket comes in once.
    pub(crate) import_lock: tokio::sync::Mutex<()>,
    /// The searches through the conversations the window started (Ctrl+K).
    searches: convsearch::Searches,
    /// The MCP server (« Claude et les agents peuvent piloter Escouade »), running while `sync_mcp` wants it.
    pub(crate) mcp: crate::mcp::McpServer<R>,
    /// The server the hooks of the agents' Claude Code interfaces call (`tui`).
    pub(crate) hooks: crate::tui::Hooks,
    /// The copies of the files joined to messages that are under way (`attachfiles`).
    pub(crate) uploads: crate::attachfiles::Uploads,
    /// The paths the window's host read for the files the page handed it (`fileorigin`): the table
    /// the app manages (the listener fills it, `file_origin` answers from it), or a table of its
    /// own where the app has none. `attach_origin` takes the paths handed out from it, only.
    pub(crate) file_origins: crate::fileorigin::FileOrigins,
    /// The files of the user's own joined by their path, by agent: the only ones a message may
    /// give as joined by it (`attachfiles::OriginFiles`).
    pub(crate) origin_files: crate::attachfiles::OriginFiles,
    /// How many processes and interfaces the application of Remote Control settings (and the
    /// app's start) starts at once: `REMOTE_STARTS`. A message or a window opening starts its own
    /// at once, with no permit.
    pub(crate) remote_starts: tokio::sync::Semaphore,
    /// Times the files of an agent's plan were read, not counting the looks that found them as
    /// before (tests only).
    #[cfg(test)]
    pub(crate) plan_reads: AtomicUsize,
    /// Looks at the files of an agent's plan that ended, whatever they found (tests only).
    #[cfg(test)]
    pub(crate) plan_looks: AtomicUsize,
    /// The least time between two looks at the files of a plan, in milliseconds (tests only: the
    /// interval is `PLAN_SCAN_EVERY`).
    #[cfg(test)]
    pub(crate) plan_scan_ms: AtomicU64,
    /// How long a deleted agent's processes are waited for, in milliseconds (tests only: the wait
    /// is `AGENT_GONE`).
    #[cfg(test)]
    pub(crate) agent_gone_ms: AtomicU64,
    /// How long a new agent’s name is waited for before its worktree is made, in milliseconds
    /// (tests only: the wait is `NAMING_LIMIT`).
    #[cfg(test)]
    pub(crate) naming_ms: AtomicU64,
    /// The conversation's ops handed to the window so far, sent or buffered (tests only: the
    /// stream bench, `fluxbench`, tells by it the lines that made something).
    #[cfg(test)]
    pub(crate) conv_ops: AtomicU64,
    /// The names that came for an agent gone meanwhile and were dropped (tests only).
    #[cfg(test)]
    pub(crate) names_dropped: AtomicU64,
    /// Notifications sent, as "<title> | <text>" (tests only).
    #[cfg(test)]
    pub alerts: Mutex<Vec<String>>,
    /// The languages the native menus were written again in (tests only: they have none).
    #[cfg(test)]
    pub(crate) relabels: Mutex<Vec<i18n::Lang>>,
    /// Syncs of external tickets queued and not over yet (tests only).
    #[cfg(test)]
    pub(crate) syncs_queued: AtomicUsize,
    /// What a look for `gh` on the PATH finds (tests only: never the machine's own).
    #[cfg(test)]
    pub gh_on_path: RwLock<Option<PathBuf>>,
    /// Scheduling passes asked for (`schedule`) and not over yet (tests only).
    #[cfg(test)]
    pub(crate) passes_queued: AtomicUsize,
    /// The model of the vectors the tests give (tests only: none is downloaded).
    #[cfg(test)]
    pub(crate) test_embedder:
        parking_lot::Mutex<Option<std::sync::Arc<dyn crate::vectors::embed::Embedder>>>,
    /// How far ahead of the real clock the autopilot's pauses and the retries of the syncs go by,
    /// in milliseconds (tests only: the end of a pause, or a retry's time, without waiting for
    /// it).
    #[cfg(test)]
    pub(crate) clock_ahead: std::sync::atomic::AtomicI64,
}

/// The least time between two looks at the files of an agent's plan (`planfiles`).
pub(crate) const PLAN_SCAN_EVERY: Duration = Duration::from_secs(2);

/// The look at the files of one agent's plan: under way (or waiting for its turn), asked for again
/// meanwhile, when it last began, what it saw.
#[derive(Default)]
struct PlanScan {
    running: bool,
    again: bool,
    last: Option<Instant>,
    stamp: Option<planfiles::Stamp>,
}

impl PlanScan {
    /// A look is asked for: true when none is under way or waiting, and the caller starts one.
    /// Else the one there is will be followed by another.
    fn ask(&mut self) -> bool {
        if self.running {
            self.again = true;
            return false;
        }
        self.running = true;
        true
    }

    /// How long the look must wait for its turn, `every` being the least time between two.
    fn wait(&self, every: Duration) -> Duration {
        self.last
            .map_or(Duration::ZERO, |at| every.saturating_sub(at.elapsed()))
    }

    /// The look is about to read the files: what is asked from here on is for the next look.
    fn begin(&mut self) {
        self.last = Some(Instant::now());
        self.again = false;
    }

    /// The look ended: true when another is owed (the loop goes on), else none is under way.
    fn end(&mut self) -> bool {
        if !self.again {
            self.running = false;
        }
        self.again
    }
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

/// The settings file, the defaults when there is none (a new install follows the system's language)
/// or when it is unreadable (a copy is kept). A file written before the interface had languages
/// (before 1.7: no `language` key) was shown in French, and stays so: it must not turn English on
/// an English system the day the app is updated. Whatever `language` says, when the key is there,
/// is the user's.
fn read_settings(path: &Path) -> Settings {
    let Some(mut settings) = read_json::<Settings>(path) else {
        return Settings::default();
    };
    let has_language = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .is_some_and(|v| v.get("language").is_some());
    if !has_language {
        settings.language = "fr".into();
        settings.claude_language = "ui".into();
    }
    settings
}

/// Said for a background task the agent's plan does not have.
fn unknown_bg_task() -> String {
    tr!(
        "Cette tâche de fond n’est pas connue de l’agent.",
        "The agent has no such background task."
    )
}

/// Said for a subagent that does not run (any more), or that the agent's plan does not have:
/// there is nothing to stop and nothing to write to.
fn subagent_over() -> String {
    tr!(
        "Ce sous-agent ne tourne plus.",
        "This subagent is no longer running."
    )
}

/// The message of the last `SubagentHandback` among the `items` of a subagent's thread that says
/// something, trimmed: what it handed back.
fn handed_back(items: &[Value]) -> Option<String> {
    items
        .iter()
        .rev()
        .filter(|i| i["kind"] == "tool" && i["name"] == "SubagentHandback")
        .filter_map(|i| i["input"]["message"].as_str())
        .map(str::trim)
        .find(|m| !m.is_empty())
        .map(str::to_string)
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
/// « (copie 3) »… while the project has an agent of that name; “(copy)”, “(copy 2)” in English.
pub(crate) fn copy_name(lang: i18n::Lang, original: &str, taken: &[String]) -> String {
    let mut name = tr_in!(lang, "{original} (copie)", "{original} (copy)");
    let mut n = 2;
    while taken.contains(&name) {
        name = tr_in!(lang, "{original} (copie {n})", "{original} (copy {n})");
        n += 1;
    }
    name
}

/// The branch of `copy`, the copy of the agent `original` that `copy_name` named:
/// `<prefix><original as a slug>-copie` (`-copie-2`…; `-copy`, `-copy-2` for a copy named in
/// English), the original's part shortened so that the marker always fits in a slug's 40
/// characters; `prefix` is the project's « Préfixe des branches des agents ».
fn copy_branch(prefix: &str, original: &str, copy: &str) -> String {
    let marker = slugify(copy.strip_prefix(original).unwrap_or(copy));
    let base = slug_within(original, 40usize.saturating_sub(marker.len() + 1));
    let slug = if base.is_empty() {
        marker
    } else {
        format!("{base}-{marker}")
    };
    format!("{prefix}{slug}")
}

/// The worktree of a copy of the agent whose worktree is `wt`, in the project's repository, its
/// folder of worktrees: on the new branch `branch` (`copy_branch`) from the commit `wt` is on (its
/// branch's, unless the agent switched it), toward the same base. Returns (path, branch, base
/// branch).
async fn copy_worktree(
    project: &Project,
    held: &[PathBuf],
    wt: &Worktree,
    branch: &str,
) -> Result<(String, String, String)> {
    let start = git::text(&wt.path, &["rev-parse", "HEAD"]).await?;
    let (path, branch) = git::worktree_add_on(
        &project.path,
        &project.worktrees_root(),
        held,
        branch,
        &start,
    )
    .await?;
    Ok((path, branch, wt.base_branch.clone()))
}

/// Haiku's role when it names an agent from its first message.
pub(crate) fn naming_system(lang: i18n::Lang) -> String {
    tr_in!(
        lang,
        "Tu nommes des tâches de développement sans jamais les réaliser. Tu réponds uniquement par un slug.",
        "You name development tasks without ever carrying them out. You answer with a slug only."
    )
}

/// Instructions + the task framed as text to name (so that the model does not try to do it).
pub(crate) fn naming_prompt(lang: i18n::Lang, task: &str) -> String {
    tr_in!(
        lang,
        "Donne un nom court à la tâche de développement ci-dessous. Ne la réalise pas.\n\
         Réponds uniquement par un slug kebab-case de 2 ou 3 mots (minuscules ASCII, sans accents), \
         par exemple refacto-auth ou tests-e2e.\n\n<tache>\n{task}\n</tache>",
        "Give a short name to the development task below. Don’t carry it out.\n\
         Answer with a kebab-case slug of 2 or 3 words only (lowercase ASCII, no accents), \
         for example auth-refactor or e2e-tests.\n\n<task>\n{task}\n</task>",
        task = crate::text::truncate(task.trim(), 2000)
    )
}

/// The commands built into Claude Code: typed alone or with arguments, they say nothing about the
/// task (`/model sonnet`, `/compact keep the plan`). Any other command is a skill or one of the
/// user's own, and its arguments are the task.
const BUILTIN_COMMANDS: &[&str] = &[
    "add-dir",
    "agents",
    "bashes",
    "bug",
    "clear",
    "compact",
    "config",
    "context",
    "cost",
    "doctor",
    "effort",
    "exit",
    "export",
    "fast",
    "help",
    "hooks",
    "ide",
    "init",
    "install-github-app",
    "login",
    "logout",
    "mcp",
    "memory",
    "model",
    "output-style",
    "permissions",
    "plugin",
    "pr-comments",
    "privacy-settings",
    "release-notes",
    "resume",
    "review",
    "rewind",
    "security-review",
    "status",
    "statusline",
    "terminal-setup",
    "todos",
    "upgrade",
    "usage",
    "vim",
];

/// What an agent that is not named yet is named from, given a message of its user's: the message
/// itself; for a command, the arguments of a skill or a custom command, without the leading slash
/// (`/impeccable propose un design` is « impeccable propose un design »). None when the message
/// says nothing to name it by: empty, a built-in command of Claude Code (`/clear`, `/model
/// sonnet`), or a skill with no arguments (`/impeccable`): the next message is looked at then.
pub(crate) fn naming_text(prompt: &str) -> Option<String> {
    let text = prompt.trim();
    let Some(command) = text.strip_prefix('/') else {
        return (!text.is_empty()).then(|| text.to_string());
    };
    let (word, arguments) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    (!arguments.trim().is_empty() && !BUILTIN_COMMANDS.contains(&word))
        .then(|| command.trim_start().to_string())
}

/// What an agent stopped by the usage limit is sent by itself once the quota resets: the same
/// word in both languages.
pub(crate) fn auto_resume(lang: i18n::Lang) -> String {
    tr_in!(lang, "continue", "continue")
}

/// The instructions of a new agent: those it is given (a ticket's protocol…), then what it is told of the project's
/// shared memory (ESC-255), so that every harness reads and writes it.
pub(crate) fn with_memory(lang: i18n::Lang, given: Option<String>) -> Option<String> {
    let line = crate::memory::instruction(lang);
    Some(match given.filter(|g| !g.trim().is_empty()) {
        Some(g) => format!("{g} {line}"),
        None => line,
    })
}

/// `given` with what the agent is told of a spec that lives in Escouade (ESC-278, `Space::follow_rule`) after it.
pub(crate) fn with_follow(given: Option<String>, follow: Option<&str>) -> Option<String> {
    match (given.filter(|g| !g.trim().is_empty()), follow) {
        (Some(g), Some(f)) => Some(format!("{g} {f}")),
        (None, Some(f)) => Some(f.to_string()),
        (g, None) => g,
    }
}

/// What the Claude of a copy of an agent is told at every start: its conversation names the
/// original's folder (`from`) everywhere, it works in `path`.
pub(crate) fn copied_conversation(lang: i18n::Lang, from: &str, path: &str) -> String {
    tr_in!(
        lang,
        "Cette conversation a été copiée depuis un agent qui travaillait dans {from}. Tu travailles maintenant dans {path} : ne lis et n'écris que dedans.",
        "This conversation was copied from an agent that worked in {from}. You now work in {path}: only read and write in it."
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
pub(crate) fn commit_proposal_system(lang: i18n::Lang) -> String {
    tr_in!(
        lang,
        "Tu écris des messages de commit, sans jamais réaliser de tâche. Tu réponds uniquement par le message de commit.",
        "You write commit messages, without ever carrying out a task. You answer with the commit message only."
    )
}

/// How much of a direct commit's diff Haiku reads (bytes), and of its files (the diff covers
/// those only): enough to tell what changed, within a quick answer.
pub(crate) const PROPOSAL_DIFF: usize = 60_000;
pub(crate) const PROPOSAL_FILES: usize = 200;

/// The latest commit subjects of the repository whose style a proposal imitates.
const RECENT_SUBJECTS: usize = 10;

/// What Haiku is asked for a direct commit's message, in `lang`: the changes, framed as text to
/// describe, in the style of the repository's latest `subjects` (newest first). When the settings
/// name the language of the texts for Claude (`named`), the message is written in it; else in the
/// language of those commits, as before that setting.
pub(crate) fn commit_proposal_prompt(
    lang: i18n::Lang,
    named: bool,
    subjects: &[String],
    files: &[FileChange],
    diff: &str,
) -> String {
    let style = if subjects.is_empty() && named {
        tr_in!(
            lang,
            "Le dépôt n'a pas encore de commit : suis le format Conventional Commits (« type(portée): description »). Écris-le en français.",
            "The repository has no commit yet: follow the Conventional Commits format (“type(scope): description”). Write it in English."
        )
    } else if subjects.is_empty() {
        tr_in!(
            lang,
            "Le dépôt n'a pas encore de commit : suis le format Conventional Commits (« type(portée): description »).",
            "The repository has no commit yet: follow the Conventional Commits format (“type(scope): description”)."
        )
    } else {
        let subjects = subjects.join("\n");
        if named {
            tr_in!(
                lang,
                "Écris-le en français, dans le style des derniers commits du dépôt ci-dessous : leur format, leur longueur.\n\n\
                 <sujets-recents>\n{subjects}\n</sujets-recents>",
                "Write it in English, in the style of the repository’s latest commits below: their format, their length.\n\n\
                 <recent-subjects>\n{subjects}\n</recent-subjects>"
            )
        } else {
            tr_in!(
                lang,
                "Imite le style des derniers commits du dépôt ci-dessous : leur langue, leur format, leur longueur.\n\n\
                 <sujets-recents>\n{subjects}\n</sujets-recents>",
                "Imitate the style of the repository’s latest commits below: their language, their format, their length.\n\n\
                 <recent-subjects>\n{subjects}\n</recent-subjects>"
            )
        }
    };
    let mut listed: String = files
        .iter()
        .take(PROPOSAL_FILES)
        .map(|f| format!("{} {}\n", f.status, f.path))
        .collect();
    if files.len() > PROPOSAL_FILES {
        let n = files.len() - PROPOSAL_FILES;
        listed.push_str(&tr_n_in!(
            lang,
            n,
            "… et {n} autre fichier\n",
            "… et {n} autres fichiers\n",
            "… and {n} more file\n",
            "… and {n} more files\n"
        ));
    }
    tr_in!(
        lang,
        "Écris le message de commit des modifications ci-dessous. Ne les réalise pas, ne les commente pas. \
         Réponds uniquement par le message : un sujet d'une ligne, puis, seulement si c'est utile, une ligne vide et un corps court.\n\n\
         {style}\n\n<fichiers>\n{listed}</fichiers>\n\n<diff>\n{diff}\n</diff>",
        "Write the commit message of the changes below. Don’t carry them out, don’t comment on them. \
         Answer with the message only: a one-line subject, then, only if useful, a blank line and a short body.\n\n\
         {style}\n\n<files>\n{listed}</files>\n\n<diff>\n{diff}\n</diff>",
        diff = crate::text::truncate(diff.trim_end(), PROPOSAL_DIFF)
    )
}

/// The block of a ticket whose validation the app's stop cut, set as the app starts again.
pub(crate) fn approval_interrupted(lang: i18n::Lang) -> String {
    tr_in!(lang, "Validation interrompue", "Approval interrupted")
}

/// A direct commit asked for files that no longer have changes to commit.
fn nothing_to_commit() -> anyhow::Error {
    anyhow!(tr!(
        "Plus rien à commiter : ces fichiers n'ont plus de modification.",
        "Nothing left to commit: these files no longer have changes."
    ))
}

/// What the conversation of an agent whose worktree was set up says: how many commands, how long.
fn setup_done(lang: i18n::Lang, n: usize, secs: u64) -> String {
    tr_n_in!(
        lang,
        n,
        "Worktree préparé ({n} commande, {secs} s).",
        "Worktree préparé ({n} commandes, {secs} s).",
        "Worktree set up ({n} command, {secs} s).",
        "Worktree set up ({n} commands, {secs} s)."
    )
}

/// The refusal of a merge while the agent's worktree has `n` files not committed.
fn uncommitted_in_agent(lang: i18n::Lang, n: usize) -> String {
    tr_n_in!(
        lang,
        n,
        "L'agent a {n} fichier non commité : demande-lui de commiter avant de merger.",
        "L'agent a {n} fichiers non commités : demande-lui de commiter avant de merger.",
        "The agent has {n} uncommitted file: ask it to commit before merging.",
        "The agent has {n} uncommitted files: ask it to commit before merging."
    )
}

/// The files the project copies into its worktrees could not be copied: the agent works without.
fn files_not_copied(e: &anyhow::Error) -> String {
    tr!(
        "Fichiers non copiés dans le worktree : {e:#}",
        "Files not copied into the worktree: {e:#}"
    )
}

/// An agent restored: its worktree went with its branch (merged, then deleted), nothing to make it
/// again from.
fn worktree_gone(lang: i18n::Lang, branch: &str) -> String {
    tr_in!(
        lang,
        "Son worktree a été supprimé avec sa branche {branch} (mergée) : il n'est pas recréé.",
        "Its worktree was removed with its branch {branch} (merged): it isn’t made again."
    )
}

/// An agent restored: its worktree, gone, made again at `to`, as another worktree took its folder
/// `from` meanwhile.
fn worktree_moved(lang: i18n::Lang, from: &str, to: &str) -> String {
    tr_in!(
        lang,
        "Le dossier {from} est pris par un autre worktree : celui de l'agent est recréé dans {to}.",
        "The folder {from} is taken by another worktree: the agent’s is made again in {to}."
    )
}

/// A project of the folder `path` with the settings a new one starts from, and no id: what
/// `add_project` fills in, or what the new project's assistant asks Claude about before it is made.
pub(crate) fn blank_project(path: &str) -> Project {
    Project {
        repos: Vec::new(),
        repos_gone: Vec::new(),
        id: String::new(),
        name: String::new(),
        path: path.to_string(),
        color: String::new(),
        tile_letters: String::new(),
        logo: String::new(),
        tile_label: false,
        tile_label_text: String::new(),
        user_name: String::new(),
        share_new: false,
        worktree_per_agent: false,
        created_at: 0,
        run_commands: Vec::new(),
        board: BoardSettings::default(),
        worktree_copy: default_worktree_copy(),
        worktree_setup: Vec::new(),
        worktree_teardown: Vec::new(),
        integrations: ProjectIntegrations::default(),
        commit_mode: CommitMode::default(),
        account: String::new(),
        worktree_dir: String::new(),
        worktree_at_first_message: true,
        worktree_base: WorktreeBase::Head,
        agent_branch_prefix: crate::model::default_agent_branch_prefix(),
        ticket_branch_prefix: crate::model::default_ticket_branch_prefix(),
        remote_control: false,
        engine: None,
        private: false,
        docs_branch: String::new(),
        shared: Default::default(),
        shared_on_limit: false,
        claude_memory_asked: false,
        workflow_fixes: Vec::new(),
        workflows_auto_read: true,
        workflows_enabled: true,
        workflows_in_repo: true,
        code_index: None,
        code_index_before: false,
    }
}

/// `blank_project` of a folder that must exist: Claude reads it.
fn unsaved_project(path: &str) -> Result<Project> {
    let path = path.trim().trim_end_matches(['\\', '/']);
    if !Path::new(path).is_dir() {
        bail!(tr!(
            "Le dossier {path} n'existe pas",
            "The folder {path} doesn’t exist"
        ));
    }
    Ok(blank_project(path))
}

/// Claude's answer held no list of commands to read.
fn no_readable_commands() -> anyhow::Error {
    anyhow!(tr!(
        "Claude n'a pas proposé de commandes lisibles.",
        "Claude proposed no readable commands."
    ))
}

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
/// renames nor deletes them, nor a folder holding one, makes nothing in them, and its search leaves
/// them out. The project's own are in its folder, below the root when it is a subfolder of its
/// repository; a validation's are at the root; and `own`, the project's « Dossier des worktrees »
/// when it sets one, wherever it is in the source.
fn worktrees_kept(root: &str, project: &str, worktree: bool, own: Option<&Path>) -> Vec<String> {
    let mut kept = vec![paths::DEFAULT_WORKTREES.to_string()];
    let below = if worktree {
        String::new()
    } else {
        sub_prefix(root, project)
    };
    if !below.is_empty() {
        kept.push(format!("{below}{}", paths::DEFAULT_WORKTREES));
    }
    if let Some(rel) = own.and_then(|d| below_root(root, &d.to_string_lossy())) {
        if !kept.contains(&rel) {
            kept.push(rel);
        }
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
    /// Reads the store's catalogue and revocation list again (through Réglages › Réseau), and
    /// hands them to the plugins: a version withdrawn stops, the updates of the store are offered.
    pub async fn refresh_store(&self) {
        let settings = self.settings.read().clone();
        match crate::plugins::store::client(&settings) {
            Ok(http) => self.store.refresh(&http, crate::i18n::ui()).await,
            Err(e) => log::warn!("the store's client: {e}"),
        }
        self.plugins
            .apply_store(self.store.revocations(), self.store.offers());
    }

    pub fn load(app: AppHandle<R>, data: DataDir) -> (Arc<Self>, mpsc::UnboundedReceiver<String>) {
        if let Err(e) = data.ensure() {
            log::error!("cannot create data dir: {e}");
        }
        let mut settings = read_settings(&data.settings_file());
        // First: what the backend writes from now on (the menus, its texts) is in these languages.
        let lang = i18n::configure(&settings.language, &settings.claude_language);
        log::info!("languages: {lang:?}");
        // Principal named in them, the first time.
        accounts::normalize(&mut settings);
        settings.normalize_display();
        settings.normalize_mid_turn_delay();
        let mut state: PersistedState = read_json(&data.state_file()).unwrap_or_default();
        // « Les agents partent de la branche cible » kept as their branch to start from (ESC-160).
        for p in &mut state.projects {
            p.settle_legacy();
        }
        // The histories whose file could not be removed at the last run are not read: removed, if
        // they can be now.
        let mut leftover = std::mem::take(&mut state.leftover_histories);
        closed::retry_leftovers(&data, &mut leftover);
        // The folder of the closed projects is read here, once: the index (`closed_index`) keeps
        // what the app needs of it from now on.
        let closed::Scan { entries, histories } = closed::scan(&data, &leftover);
        // A closing the last stop cut may have been saved with some of its project's agents and
        // tickets only: they come back from the history it wrote first (forgotten once saved).
        let recovered = closed::recover_open(
            histories,
            &state.projects,
            &mut state.agents,
            &mut state.tickets,
        );
        let closed_index = entries.into_iter().map(|e| (e.id.clone(), e)).collect();
        // Read before the agents are made: a turn does not survive a restart (they come back done).
        let cut_turns: Vec<String> = state
            .agents
            .iter()
            .filter(|m| m.status.is_active() && !m.archived)
            .map(|m| m.id.clone())
            .collect();
        // An agent is told archived before its turn is stopped (the window then sends it none of the
        // messages it holds): an archiving the app's stop cut short lets go of its planned resume and
        // its test ports here, as it would have.
        for m in state.agents.iter_mut().filter(|m| m.archived) {
            m.resume_at = None;
            m.port_base = None;
        }
        let mut tickets = state.tickets;
        for t in &mut tickets {
            // A validation the app's stop cut: to run again by hand.
            if t.step.take().is_some() {
                t.blocked = Some(approval_interrupted(lang.ui));
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
        // Read one operation at a time: one a later version added (the app rolled back) is left
        // out, rather than the whole queue with it.
        let pending_syncs = read_json::<Vec<Value>>(&data.sync_queue_file())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|op| match serde_json::from_value(op) {
                Ok(op) => Some(op),
                Err(e) => {
                    log::warn!("sync left out of {}: {e}", data.sync_queue_file().display());
                    None
                }
            })
            .collect();
        // The autopilot's pause as the app stopped, for the first pass (before any new reading of
        // the quotas, which with an API key never comes): a window read holds until its end, an
        // ended one is dropped (the status bar would show it). Each account's own.
        let saved = state.pause;
        let now = now_ms();
        let ongoing = |m: QuotaMeter| {
            Some(m).filter(|m| m.resets_at.is_some_and(|end| end + RESUME_MARGIN_MS > now))
        };
        let usage = UsageSnapshot::restored(&saved, &settings, ongoing, now);
        let (git, rx) = GitService::new();
        let telemetry = Arc::new(Telemetry::new(
            data.telemetry_id_file(),
            app.package_info().version.to_string(),
            settings.telemetry,
            paths::sandbox_dir().is_none(),
        ));
        // The table the window's listener fills, when the app made it (it does, before the core).
        let file_origins = {
            use tauri::Manager;
            app.try_state::<crate::fileorigin::FileOrigins>()
                .map(|s| s.inner().clone())
                .unwrap_or_default()
        };
        let plugins = crate::plugins::Plugins::load(data.clone(), env!("CARGO_PKG_VERSION"));
        let store = Arc::new(crate::plugins::store::Store::load(
            &data,
            crate::plugins::store::store_url(),
            crate::plugins::store::mirror_url(),
            env!("CARGO_PKG_VERSION"),
        ));
        // What was cached applies from the start: a version withdrawn stays stopped offline.
        plugins.apply_store(store.revocations(), store.offers());
        let plugin_runtime = crate::plugins::runtime::Runtime::new(
            plugins.clone(),
            secrets.clone(),
            &data,
            env!("CARGO_PKG_VERSION"),
            crate::plugins::runtime::Limits::default(),
        );
        let plugin_events = crate::plugins::events::Dispatch::new(
            plugins.clone(),
            plugin_runtime.clone(),
            secrets.clone(),
            crate::plugins::events::QUEUE,
        );
        let facts = crate::facts::Bus::new();
        facts.subscribe(plugin_events.clone());
        let core = Arc::new_cyclic(|me| Self {
            me: me.clone(),
            telemetry,
            stats: Stats::open(&data.stats_db()),
            codeindex: crate::codeindex::Service::new(data.clone()),
            plugin_runtime,
            facts,
            plugin_events,
            plugins,
            store,
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
            engine_models: RwLock::new(state.engine_models),
            // The menus as the names they keep make them (a list kept before ESC-124 makes none).
            account_models: RwLock::new(
                state
                    .account_models
                    .into_iter()
                    .map(|(id, m)| {
                        // A Codex account's keeps no models of Claude Code's, its menu alone.
                        if m.models.is_empty() {
                            return (id, m);
                        }
                        let read_at = m.read_at;
                        let made = crate::model::AccountModels::new(m.version, m.models);
                        (id, crate::model::AccountModels { read_at, ..made })
                    })
                    .collect(),
            ),
            models_reading: Mutex::default(),
            vectors_download: Mutex::default(),
            git,
            git_cache: RwLock::default(),
            pty: PtyManager::default(),
            spawn_locks: Mutex::default(),
            quit_locks: Mutex::default(),
            remote_locks: Mutex::default(),
            closing: Mutex::default(),
            closed_index: Mutex::new(closed_index),
            leftover_histories: Mutex::new(leftover),
            create_lock: tokio::sync::Mutex::new(()),
            conv_buffer: Mutex::default(),
            conv_flush: tokio::sync::Notify::new(),
            wf_tracks: Mutex::default(),
            git_names: Mutex::default(),
            last_oauth_call: Mutex::default(),
            usage_api: RwLock::new(usage::api_from_env()),
            resources: Mutex::default(),
            git_inflight: Mutex::default(),
            sync_locks: Mutex::default(),
            toplevels: Mutex::default(),
            plan_scans: Mutex::default(),
            sidechains: Mutex::default(),
            dirty: AtomicBool::new(false),
            waiting: AtomicUsize::new(usize::MAX),
            quitting: AtomicBool::new(false),
            unsaved: AtomicUsize::new(0),
            held: Mutex::default(),
            page_crash: Mutex::default(),
            works: AtomicUsize::new(0),
            board_lock: tokio::sync::Mutex::new(()),
            start_refusals: Mutex::new(HashMap::new()),
            cut_turns: Mutex::new(cut_turns),
            gh: RwLock::new(locate_gh()),
            test_runs: Mutex::default(),
            setups: Mutex::default(),
            claude_missing: AtomicBool::new(false),
            board_issues: Mutex::default(),
            hold: Mutex::new(saved.hold),
            pause_shown: Mutex::default(),
            switching: Mutex::default(),
            switch_told: Mutex::default(),
            ports_reserved: Mutex::default(),
            docs_shares: Mutex::default(),
            merge_locks: Mutex::default(),
            claude_accounts_lock: Mutex::default(),
            accounts: RwLock::new(accounts),
            secrets,
            bases: RwLock::new(integrations::Bases::from_env()),
            gh_token: Mutex::default(),
            forges: RwLock::new(crate::forge::Registry::standard()),
            forge_health: Mutex::default(),
            forge_cli_tokens: Mutex::default(),
            pr_reads: Mutex::default(),
            repo_parents: Mutex::default(),
            sync_queue: Mutex::default(),
            pending_syncs: Mutex::new(integrations::sync::SyncQueue::new(pending_syncs)),
            import_lock: tokio::sync::Mutex::new(()),
            searches: convsearch::Searches::default(),
            mcp: crate::mcp::McpServer::new(me.clone()),
            hooks: Default::default(),
            uploads: Default::default(),
            file_origins,
            origin_files: Default::default(),
            remote_starts: tokio::sync::Semaphore::new(REMOTE_STARTS),
            #[cfg(test)]
            plan_reads: AtomicUsize::new(0),
            #[cfg(test)]
            plan_looks: AtomicUsize::new(0),
            #[cfg(test)]
            plan_scan_ms: AtomicU64::new(PLAN_SCAN_EVERY.as_millis() as u64),
            #[cfg(test)]
            agent_gone_ms: AtomicU64::new(AGENT_GONE.as_millis() as u64),
            #[cfg(test)]
            naming_ms: AtomicU64::new(NAMING_LIMIT.as_millis() as u64),
            #[cfg(test)]
            conv_ops: AtomicU64::new(0),
            #[cfg(test)]
            names_dropped: AtomicU64::new(0),
            #[cfg(test)]
            alerts: Mutex::default(),
            #[cfg(test)]
            relabels: Mutex::default(),
            #[cfg(test)]
            syncs_queued: AtomicUsize::new(0),
            #[cfg(test)]
            gh_on_path: RwLock::default(),
            #[cfg(test)]
            passes_queued: AtomicUsize::new(0),
            #[cfg(test)]
            test_embedder: parking_lot::Mutex::new(None),
            #[cfg(test)]
            clock_ahead: std::sync::atomic::AtomicI64::new(0),
        });
        core.usage.lock().set_today(&core.stats.today_by_account());
        // The names in the facts are the core's; each agent tells its own.
        let resolver: std::sync::Weak<dyn crate::facts::Resolve> =
            Arc::downgrade(&(core.clone() as Arc<dyn crate::facts::Resolve>));
        core.facts.resolve_with(resolver);
        let place: std::sync::Weak<dyn crate::plugins::events::Place> =
            Arc::downgrade(&(core.clone() as Arc<dyn crate::plugins::events::Place>));
        core.plugin_events.place_with(place.clone());
        core.plugin_runtime.place_with(place);
        for h in core.agents.read().values() {
            h.lock().facts = core.facts.emitter();
        }
        // The API keys of the Cursor accounts, which their processes go by (ESC-43).
        core.load_account_keys();
        // The images pasted in the interfaces a week ago or more, also when nobody pastes any more.
        crate::tui::clear_pastes(&core.pastes_dir());
        // The copies of joined files a window left half written, the drafts' of a day ago, and
        // those older than the days of the settings (then every hour: `start`).
        core.maintain_attachments();
        core.settle_recovered(&recovered);
        (core, rx)
    }

    /// `app.started` on the facts' bus (PLG-5).
    pub(crate) fn tell_started(&self) {
        self.facts.emit(
            crate::facts::Body::AppStarted {
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            crate::facts::About::default(),
        );
    }

    pub fn start(self: &Arc<Self>, git_rx: mpsc::UnboundedReceiver<String>) {
        let projects = self.projects.read().clone();
        self.tell_started();
        // The plugins that decide run before the first permission asked (PLG-8).
        crate::plugins::decide::warm(&self.plugins, &self.plugin_runtime);
        self.telemetry.track(self.started_event());
        if !cfg!(test) {
            self.telemetry.spawn();
        }
        for p in &projects {
            self.watch_project(p);
        }
        // Repositories added or removed while the app was closed (ESC-169).
        self.refresh_all_repos();
        // And their workflows, read again if a file changed meanwhile (ESC-192).
        self.refresh_all_workflows();
        // The shared tickets' files read before the window asks for the tickets (ESC-190): what a pull brought while
        // the app was closed is on the board from the start. This machine's user's id is made once, then they are read
        // again with it.
        self.sync_all_shared();
        if self.settings.read().user_id.is_empty() {
            let c = self.clone();
            tauri::async_runtime::spawn(async move {
                c.ensure_identity().await;
                c.sync_all_shared();
            });
        }
        // The newer versions of the installed plugins, looked for once at the start (PLG-4): offered in
        // the view Plugins, never installed by themselves.
        if !cfg!(test) {
            let plugins = self.plugins.clone();
            tauri::async_runtime::spawn(async move { plugins.check_updates().await });
            // The store's catalogue and revocation list (STORE-9): at the start, then every 6 hours.
            let c = self.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    c.refresh_store().await;
                    tokio::time::sleep(crate::plugins::store::EVERY).await;
                }
            });
        }
        let c = self.clone();
        tauri::async_runtime::spawn(async move { c.git_loop(git_rx).await });
        tauri::async_runtime::spawn(self.clone().conv_flusher());
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(400)).await;
                if c.dirty.swap(false, Ordering::AcqRel) {
                    c.save_now();
                    // A ticket shared with the team writes its file at each change (ESC-190).
                    c.sync_all_shared();
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
        // The copies of joined files that are old enough to go (`attachKeepDays`), and the ones
        // lost half written: at the start (`load`), then every hour.
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                let core = c.clone();
                let _ =
                    tauri::async_runtime::spawn_blocking(move || core.maintain_attachments()).await;
            }
        });
        let c = self.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(20)).await;
                c.resume_due().await;
                // An account signed in again: the agents that waited for it go on (ESC-162).
                c.sign_in_due().await;
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
        // The MCP server first: the agents started below reach it.
        self.start_mcp();
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
        let engine_models = self.engine_models.read().clone();
        let account_models = self.account_models.read().clone();
        let tickets = self.tickets.read().clone();
        let hold = self.hold.lock().clone();
        let pause = self.usage.lock().saved(hold);
        let leftover_histories = self.leftover_histories.lock().clone();
        PersistedState {
            projects,
            agents,
            ui,
            models,
            engine_models,
            account_models,
            tickets,
            pause,
            leftover_histories,
        }
    }

    /// The state saved; the error is for the caller that must lose nothing when it fails.
    pub(crate) fn save_state(&self) -> Result<()> {
        let state = self.snapshot();
        let bytes = serde_json::to_vec_pretty(&state).context("cannot serialize state")?;
        paths::write_atomic(&self.data.state_file(), &bytes).context("cannot save state")?;
        Ok(())
    }

    pub fn save_now(&self) {
        if let Err(e) = self.save_state() {
            log::error!("{e:#}");
        }
    }

    pub fn request_save(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    /// The app's setup, in broad strokes, for the usage statistics.
    pub(crate) fn started_event(&self) -> Event {
        // One lock at a time: a read taken twice waits behind a writer queued in between.
        let language = serde_json::to_value(self.lang().ui)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        let (accounts, mcp) = {
            let s = self.settings.read();
            (s.accounts.len(), s.mcp_enabled)
        };
        let (projects, wsl) = {
            let projects = self.projects.read();
            let wsl = projects
                .iter()
                .any(|p| crate::wsl::on_share(Path::new(&p.path)));
            (projects.len(), wsl)
        };
        Event::AppStarted {
            language,
            accounts,
            projects,
            mcp,
            wsl,
        }
    }

    /// The answer to « Statistiques d’usage », kept at once (the question of the first
    /// start; the settings window saves it with the rest).
    pub fn set_telemetry(self: &Arc<Self>, consent: bool) -> Result<()> {
        let mut s = self.settings.read().clone();
        s.telemetry = Some(consent);
        self.save_settings(s)
    }

    /// `model` of `engine` marked as a favorite, or no longer; the favorites as they then are
    /// (kept at once, as the accounts are).
    pub fn set_favorite_model(
        self: &Arc<Self>,
        engine: EngineId,
        model: &str,
        favorite: bool,
    ) -> Result<std::collections::BTreeMap<EngineId, Vec<String>>> {
        let mut s = self.settings.read().clone();
        let list = s.favorite_models.entry(engine).or_default();
        list.retain(|m| m != model);
        if favorite {
            list.push(model.to_string());
        }
        if list.is_empty() {
            s.favorite_models.remove(&engine);
        }
        let favorites = s.favorite_models.clone();
        self.save_settings(s)?;
        Ok(favorites)
    }

    pub fn save_settings(self: &Arc<Self>, mut s: Settings) -> Result<()> {
        // The MCP server's port is the backend's own (`sync_mcp`): the window's copy may be older.
        s.mcp_port = self.settings.read().mcp_port;
        // « Index du code sur tous les projets » is `set_code_index_all`'s (ESC-293): the window's
        // draft may be older.
        s.code_index_all = self.settings.read().code_index_all;
        // The user's id is the backend's too (`ensure_identity`): a window's copy may be older.
        if s.user_id.is_empty() {
            s.user_id = self.settings.read().user_id.clone();
        }
        s.normalize_attachments();
        s.normalize_mid_turn_delay();
        s.normalize_display();
        accounts::normalize(&mut s);
        accounts::check(&s)?;
        let bytes = serde_json::to_vec_pretty(&s)?;
        paths::write_atomic(&self.data.settings_file(), &bytes)?;
        let auto_resume = s.auto_resume;
        let (mcp_enabled, before) = (s.mcp_enabled, self.lang());
        let telemetry = s.telemetry;
        let remote_all = s.remote_control_all;
        let self_update = s.claude_self_update;
        let before_settings = std::mem::replace(&mut *self.settings.write(), s);
        let mcp_was_enabled = before_settings.mcp_enabled;
        if self_update && !before_settings.claude_self_update {
            // « Mettre à jour Claude Code automatiquement » turned on: run when due, at once.
            crate::selfupdate::poke();
        }
        if before_settings.remote_control_all != remote_all {
            // Every agent gets Remote Control or loses it, on tasks of their own: the save does
            // not wait for the processes it starts.
            self.apply_remote_to(None);
        }
        if self.telemetry.set_consent(telemetry) {
            // Said yes during the session: it counts as started.
            self.telemetry.track(self.started_event());
        }
        if mcp_was_enabled && !mcp_enabled {
            // « Claude et les agents peuvent piloter Escouade » turned off: the token changes at once, and its entry
            // leaves every account (`declare_now`, on its own task); the agents lose it at once.
            self.mcp_turned_off();
        }
        let lang = self.lang();
        if lang != before {
            // At once, without a restart: what the backend writes from now on, the native menus
            // (when the interface's language is the one that changed), and the window.
            i18n::set(lang);
            if lang.ui != before.ui {
                self.relabel_menus(lang.ui);
            }
            self.hub.emit(UiEvent::Language { lang });
        }
        // Accounts added, removed, put in another order, a threshold or a language changed: the
        // quotas as the window shows them.
        self.settle_usage();
        // A ticket set on an account removed or switched off goes back to « Auto » (ESC-153).
        self.release_lost_accounts(&before_settings);
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
        // « Claude et les agents peuvent piloter Escouade » may have changed.
        self.sync_mcp();
        Ok(())
    }

    // ---------- lookups ----------

    /// The languages its settings stand for, as the window is told them. The ones the backend
    /// writes in (`i18n::ui()`) are these, except in tests: there they stay French.
    pub fn lang(&self) -> LangInfo {
        let s = self.settings.read();
        i18n::resolve(&s.language, &s.claude_language)
    }

    /// Itself, without keeping it alive (for a worker that lives as long as it does).
    pub(crate) fn weak(&self) -> std::sync::Weak<Self> {
        self.me.clone()
    }

    pub fn agent(&self, id: &str) -> Result<AgentHandle> {
        self.agents
            .read()
            .get(id)
            .cloned()
            .ok_or_else(agent_not_found)
    }

    pub fn project(&self, id: &str) -> Result<Project> {
        self.projects
            .read()
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(project_not_found)
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

    /// `stats_view_by` of every harness.
    #[cfg(test)]
    pub fn stats_view(&self, range: &str, account: Option<&str>) -> StatsView {
        self.stats_view_by(range, account, None)
    }

    /// The statistics of a period, of an account, a harness or both (« Tous les harness / … »),
    /// with the agents and tickets named. Every agent counts, archived ones too: they keep their
    /// ticket, so a ticket sent back and taken up by another agent adds up both.
    pub fn stats_view_by(
        &self,
        range: &str,
        account: Option<&str>,
        engine: Option<&str>,
    ) -> StatsView {
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
        let filter = crate::stats::Filter { account, engine };
        self.stats
            .query_by(range, filter, &Labels { agents, tickets })
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

    /// What a fact of the agent `id` is about: the agent and its project (`facts`).
    pub(crate) fn about_agent(&self, id: &str) -> crate::facts::About {
        match self.agent(id) {
            Ok(h) => {
                let rt = h.lock();
                crate::facts::About::agent(id, &rt.meta.name, &rt.meta.project_id)
            }
            // Gone: its project unknown, the fact goes to no plugin that could not see it.
            Err(_) => crate::facts::About {
                agent: Some(crate::facts::Named {
                    id: id.to_string(),
                    name: String::new(),
                }),
                private: Some(true),
                ..Default::default()
            },
        }
    }

    pub(crate) fn emit_agent(&self, h: &AgentHandle) {
        let view = h.lock().view();
        if !self.is_registered(&view.meta.id) {
            return;
        }
        self.hub.emit(UiEvent::Agent { agent: view });
        self.update_tray();
    }

    pub(crate) fn spawn_lock(&self, id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.spawn_locks
            .lock()
            .entry(id.to_string())
            .or_default()
            .clone()
    }

    /// The lock of the agent's Remote Control requests (`remote_locks`).
    fn remote_lock(&self, id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.remote_locks
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
        #[cfg(test)]
        self.conv_ops.fetch_add(ops.len() as u64, Ordering::AcqRel);
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

    /// Sends the buffered deltas 16 ms after the first of them (`emit_conv`), for good; the stream
    /// bench (INT-0, `fluxbench`) runs it as the app does.
    pub(crate) async fn conv_flusher(self: Arc<Self>) {
        loop {
            self.conv_flush.notified().await;
            tokio::time::sleep(Duration::from_millis(16)).await;
            self.flush_conv();
        }
    }

    /// Whether deltas of the agent `id` wait in the buffer for `conv_flusher`.
    #[cfg(test)]
    pub(crate) fn conv_pending(&self, id: &str) -> bool {
        self.conv_buffer.lock().contains_key(id)
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

    pub(crate) fn apply(
        self: &Arc<Self>,
        id: &str,
        project_id: &str,
        name: &str,
        fx: Effects,
        view: Option<AgentView>,
    ) {
        self.track_workflow(
            id,
            project_id,
            &fx.ops,
            fx.turns.iter().map(|t| t.cost).sum(),
            fx.turn_end.as_ref(),
        );
        // Before the turn's end is shown: whoever sees a limited turn finds its resume planned.
        if let Some(windows) = fx.rate {
            // The quota of the account the agent's process runs on.
            let account = self.account_of(id);
            self.update_usage(|u| {
                u.record(&account, usage::Reading::Windows(windows), now_ms());
                if let Some(overage) = fx.overage {
                    u.account_mut(&account).overage = overage;
                }
            });
            // The autopilot pauses (or goes on) as the quotas just read say.
            self.pause_tick();
            // Read after its turn ended (Claude Code tells it after the result): its account may
            // just have passed « Changer de compte à » (ESC-148).
            self.switch_soon(id);
        }
        let limited = fx.limited.is_some();
        if let Some(resets_at) = fx.limited {
            self.plan_resume(id, resets_at);
        }
        if !fx.ops.is_empty() {
            self.emit_conv(id, fx.ops);
        }
        let changed = view.is_some();
        // The view given was taken before its resume was planned: told as it is now, else the
        // window would be shown the agent without it, after the one `set_resume` sent.
        let view = match view {
            Some(_) if limited => self.agent(id).ok().map(|h| h.lock().view()),
            view => view,
        };
        if let Some(v) = view.filter(|_| self.is_registered(id)) {
            self.hub.emit(UiEvent::Agent { agent: v });
        }
        if !fx.turns.is_empty() {
            // The end of a turn: the code it changed is read by the index (ESC-251).
            self.code_index_event(project_id);
            self.stats
                .record_turns(&self.account_of(id), id, project_id, &fx.turns);
            {
                let mut u = self.usage.lock();
                u.set_today(&self.stats.today_by_account());
                self.hub.emit(UiEvent::Usage { usage: u.clone() });
            }
            // A day's budget may just have been reached (`board::account_pause`).
            if self
                .usage
                .lock()
                .accounts
                .iter()
                .any(|a| a.daily_budget.is_some())
            {
                self.pause_tick();
            }
        }
        if fx.sign_in_lost {
            self.sign_in_lost_soon(id);
        }
        if fx.files_changed {
            self.git.refresh(project_id);
        }
        if fx.save {
            self.request_save();
        }
        if fx.plan_files {
            self.ask_plan_scan(id);
        }
        for (item, input) in fx.sent {
            self.deliver_sent(id, item, input);
        }
        if changed {
            self.update_tray();
        }
        // The plugins that decide answer first (PLG-8): the user is told only if they leave it.
        let notify = if fx.asked.is_empty() {
            fx.notify
        } else {
            self.plugins_decide(id, project_id, name, fx.asked, fx.notify)
        };
        if let Some(alert) = notify {
            self.notify_agent_now(alert, id, project_id, name);
        }
        if let Some(end) = fx.turn_end {
            self.facts.emit(
                crate::facts::turn_ended(&end),
                crate::facts::About::agent(id, name, project_id),
            );
            self.on_turn_end(id, end);
        }
    }

    fn notify_agent_now(
        self: &Arc<Self>,
        alert: AgentAlert,
        id: &str,
        project_id: &str,
        name: &str,
    ) {
        // A ticket's agent at work: its board tells when the ticket is ready or blocked; its
        // questions still call for the user. Nor a turn's end the window follows with a message
        // it holds.
        let next = alert.kind == NotifyKind::Done && self.held.lock().contains(id);
        if (alert.kind == NotifyKind::Question || !self.ticket_doing(id)) && !next {
            self.notify_agent(alert, project_id, id, name);
        }
    }

    /// The permissions `asked` go first to the plugins that decide in the project (PLG-8,
    /// `plugins::decide`): what they allow or refuse is answered by them, named on the card
    /// (« Autorisé par <plugin> via Escouade »); what they leave goes to the user, told then by
    /// `alert`. Gives `alert` back when no plugin decides there.
    fn plugins_decide(
        self: &Arc<Self>,
        id: &str,
        project_id: &str,
        name: &str,
        asked: Vec<crate::plugins::decide::Asked>,
        alert: Option<AgentAlert>,
    ) -> Option<AgentAlert> {
        use crate::facts::Resolve;
        use crate::plugins::decide;
        let mut about = crate::facts::About::agent(id, name, project_id);
        let private = self.resolve(&mut about).unwrap_or(true);
        let deciders = decide::deciders(&self.plugins, project_id, private);
        if deciders.is_empty() {
            return alert;
        }
        let mut subjects = serde_json::json!({
            "project": {
                "id": project_id,
                "name": about.project.map(|p| p.name).unwrap_or_default(),
                "private": private,
            },
            "agent": { "id": id, "name": about.agent.map(|a| a.name).unwrap_or_default() },
        });
        if let Some(t) = about.ticket {
            subjects["ticket"] = serde_json::json!({ "id": t.id, "key": t.key, "title": t.title });
        }
        let (me, id, project_id, name) = (
            self.clone(),
            id.to_string(),
            project_id.to_string(),
            name.to_string(),
        );
        tauri::async_runtime::spawn(async move {
            let mut left = false;
            for a in asked {
                let arg: String = a.arg.chars().take(200).collect();
                let what = format!("{} {arg}", a.tool);
                let params = decide::params(&a, subjects.clone());
                let verdict =
                    decide::decide(&me.plugin_runtime, &deciders, &params, what.trim()).await;
                let answered = match &verdict {
                    decide::Verdict::Allow { by } => {
                        me.answer_permission_by(&id, &a.request_id, "allow", None, Some(by))
                    }
                    decide::Verdict::Deny { by, message } => me.answer_permission_by(
                        &id,
                        &a.request_id,
                        "deny",
                        message.clone(),
                        Some(by),
                    ),
                    decide::Verdict::Ask => {
                        left = true;
                        continue;
                    }
                };
                match answered {
                    Ok(()) => log::info!("agent {id}: {verdict:?} for {}", a.request_id),
                    // Answered meanwhile (the user in the window, a client), or its process gone.
                    Err(e) => log::info!("agent {id}: {verdict:?} not given: {e:#}"),
                }
            }
            if let Some(alert) = alert.filter(|_| left) {
                me.notify_agent_now(alert, &id, &project_id, &name);
            }
        });
        None
    }

    pub(crate) fn on_frame(self: &Arc<Self>, h: &AgentHandle, gen: u64, frame: Value) {
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

    /// How long a new agent’s name is waited for before its worktree is made (`NAMING_LIMIT`).
    fn naming_limit(&self) -> Duration {
        #[cfg(test)]
        return Duration::from_millis(self.naming_ms.load(Ordering::Acquire));
        #[cfg(not(test))]
        NAMING_LIMIT
    }

    /// How long the processes of a deleted agent are waited for before its worktree goes.
    pub(crate) fn agent_gone(&self) -> Duration {
        #[cfg(test)]
        return Duration::from_millis(self.agent_gone_ms.load(Ordering::Acquire));
        #[cfg(not(test))]
        AGENT_GONE
    }

    /// Waits (`agent_gone`) for the processes of the killed agent `id`, `p`, to be gone. None when
    /// they are; else what the warning says: how many still run (the log, which ones).
    pub(crate) async fn await_agent_gone(&self, id: &str, p: &Process) -> Option<String> {
        let (within, waited) = (self.agent_gone(), Instant::now());
        let gone = p.wait_gone(within).await;
        log::info!(
            "agent {id}: its processes {} after {} ms",
            if gone { "are gone" } else { "still run" },
            waited.elapsed().as_millis()
        );
        if gone {
            return None;
        }
        let running = p.still_running();
        log::warn!(
            "agent {id}: {running} process(es) still run after {} s, pids {:?}",
            within.as_secs(),
            p.pids()
        );
        Some(agent_still_running(i18n::ui(), running, within.as_secs()))
    }

    /// Waits, at most `within`, until no refresh of the project's git state is running: its `git`
    /// commands sit in the agents' worktrees, which cannot go while one does.
    async fn wait_git_idle(&self, project_id: &str, within: Duration) {
        let until = Instant::now() + within;
        while self.git_inflight.lock().contains(project_id) && Instant::now() < until {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    /// The least time between two looks at the files of an agent's plan.
    fn plan_scan_every(&self) -> Duration {
        #[cfg(test)]
        return Duration::from_millis(self.plan_scan_ms.load(Ordering::Acquire));
        #[cfg(not(test))]
        PLAN_SCAN_EVERY
    }

    /// The files of the agent's plan may have moved (a command ended, a subagent came back, the
    /// turn ended): they are looked at, off the agent's lock, once per `PLAN_SCAN_EVERY` at the
    /// most. Asked again while a look is under way or waiting for its turn, one more follows it,
    /// so that the last thing a burst of commands did is seen.
    fn ask_plan_scan(self: &Arc<Self>, id: &str) {
        if !self
            .plan_scans
            .lock()
            .entry(id.to_string())
            .or_default()
            .ask()
        {
            return;
        }
        let (core, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move { core.plan_scan_loop(&id).await });
    }

    async fn plan_scan_loop(self: Arc<Self>, id: &str) {
        loop {
            let owed = self
                .plan_scans
                .lock()
                .get(id)
                .map(|s| s.wait(self.plan_scan_every()));
            if let Some(wait) = owed.filter(|w| !w.is_zero()) {
                tokio::time::sleep(wait).await;
            }
            if let Some(scan) = self.plan_scans.lock().get_mut(id) {
                scan.begin();
            }
            let alive = self.scan_plan(id).await;
            #[cfg(test)]
            self.plan_looks.fetch_add(1, Ordering::AcqRel);
            let mut scans = self.plan_scans.lock();
            match scans.get_mut(id) {
                Some(scan) if alive => {
                    if !scan.end() {
                        return;
                    }
                }
                _ => {
                    scans.remove(id);
                    return;
                }
            }
        }
    }

    /// One look at the folder of the agent `id`: its plan and its ledger, read when they are not
    /// as the last look saw them, then handed to its plan. A read that fails changes nothing and
    /// says nothing. False: the agent is gone.
    async fn scan_plan(self: &Arc<Self>, id: &str) -> bool {
        let Ok(agent) = self.agent(id) else {
            return false;
        };
        let (cwd, written, since) = {
            let rt = agent.lock();
            (rt.meta.cwd.clone(), rt.plan_written.clone(), rt.plan_since)
        };
        let root = self.toplevel(&cwd).await.unwrap_or_else(|| cwd.clone());
        let last = self.plan_scans.lock().get(id).and_then(|s| s.stamp.clone());
        let scanned = tokio::task::spawn_blocking(move || {
            // The paths it wrote are spelled as it spells its folder; git says where that is.
            let written = planfiles::reroot(Path::new(&root), Path::new(&cwd), &written);
            planfiles::scan(Path::new(&root), &written, since, last.as_ref())
        })
        .await;
        let stamp = match scanned {
            Ok(planfiles::Scan::Read(stamp, list)) => {
                #[cfg(test)]
                self.plan_reads.fetch_add(1, Ordering::AcqRel);
                // Read for the conversation it was asked in: if that ended meanwhile, what was
                // read is not its plan, and the next look reads it again.
                let mut fx = Effects::default();
                let (pid, name, view, applied) = {
                    let mut rt = agent.lock();
                    let applied = rt.plan_from_files(since, Some(list), &mut fx);
                    let view = fx.agent_changed.then(|| rt.view());
                    (
                        rt.meta.project_id.clone(),
                        rt.meta.name.clone(),
                        view,
                        applied,
                    )
                };
                self.apply(id, &pid, &name, fx, view);
                applied.then_some(stamp)
            }
            Ok(planfiles::Scan::Unchanged) => return true,
            Ok(planfiles::Scan::Nothing) | Err(_) => None,
        };
        if let Some(scan) = self.plan_scans.lock().get_mut(id) {
            scan.stamp = stamp;
        }
        true
    }

    fn on_exit(self: &Arc<Self>, h: &AgentHandle, gen: u64, code: Option<i32>, stderr: String) {
        let mut fx = Effects::default();
        let (id, pid, name, view) = {
            let mut rt = h.lock();
            let current = rt.gen == gen;
            log::info!(
                "agent {}: {} exited (code {code:?}, current process: {current})",
                rt.meta.id,
                rt.meta.engine
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

    pub(crate) fn notify_agent(
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
        if let Some(tray) = self.app.tray_by_id(menus::TRAY_ID) {
            let _ = tray.set_icon(notify::tray_icon(&self.app, n));
            let _ = tray.set_tooltip(Some(menus::tray_tooltip(i18n::ui(), n)));
        }
    }

    /// The native menus and the tray icon's tooltip in `lang`, after a change of language.
    fn relabel_menus(&self, lang: i18n::Lang) {
        // Before the first count (`start`), no agent is counted as waiting.
        let waiting = match self.waiting.load(Ordering::Acquire) {
            usize::MAX => 0,
            n => n,
        };
        #[cfg(test)]
        self.relabels.lock().push(lang);
        menus::relabel(&self.app, lang, waiting);
    }

    // ---------- claude processes ----------

    /// Returns the agent's live process, starting it (with --resume) when needed.
    pub async fn ensure_process(self: &Arc<Self>, id: &str) -> Result<Process> {
        self.ensure_process_for(id, false).await
    }

    /// `ensure_process`; `warming`: only the warm-up of a new agent, which comes as late as the
    /// runtime lets it, and finds the agent archived meanwhile: no process is started for it then
    /// (nothing would close it, and it would hold the folder of its worktree).
    pub(crate) async fn ensure_process_for(
        self: &Arc<Self>,
        id: &str,
        warming: bool,
    ) -> Result<Process> {
        // Never in the project's folder: its worktree, which waited for its first message, is made
        // under the name it has (ESC-156), and set up.
        if !warming && self.agent(id)?.lock().meta.worktree_pending {
            self.make_pending_worktree(id, None).await;
            self.wait_setup(id).await;
        }
        // What a start resumes: its session, or a copy's original's, from where it was copied.
        let resumes = |m: &AgentMeta| (m.session_id.clone(), m.fork_of.clone(), m.fork_at.clone());
        let before = resumes(&self.agent(id)?.lock().meta);
        match self.start_process(id, warming).await {
            // What it was to resume could not be (the session, or a copy's point in its
            // original's): its exit handler dropped it, so a second start goes on without it.
            Err(e)
                if e.is::<StartupFailure>() && resumes(&self.agent(id)?.lock().meta) != before =>
            {
                self.start_process(id, warming).await
            }
            other => other,
        }
    }

    /// Before a `claude` of WSL starts for the agent of `h`: its worktree leads to the repository
    /// by a relative path, which git of Linux can follow (`git::relative_gitdir`).
    pub(crate) fn ready_for_wsl(&self, program: &engine::Program, h: &AgentHandle) {
        if program.wsl().is_none() {
            return;
        }
        let Some(worktree) = h.lock().meta.worktree.as_ref().map(|w| w.path.clone()) else {
            return;
        };
        if let Err(e) = git::relative_gitdir(Path::new(&worktree)) {
            log::warn!("worktree {worktree}: its .git stays as it is for git of WSL: {e}");
        }
    }

    async fn start_process(self: &Arc<Self>, id: &str, warming: bool) -> Result<Process> {
        let h = self.agent(id)?;
        self.start_process_in(&h, id, warming).await
    }

    /// `start_process` of the agent `h` (`id`), as found at the call.
    #[cfg(test)]
    pub(crate) async fn start_process_of(
        self: &Arc<Self>,
        h: &AgentHandle,
        id: &str,
    ) -> Result<Process> {
        self.start_process_in(h, id, false).await
    }

    async fn start_process_in(
        self: &Arc<Self>,
        h: &AgentHandle,
        id: &str,
        warming: bool,
    ) -> Result<Process> {
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        // Archived while its warm-up waited for the lock (the archive has it too, and found no
        // process to close).
        if warming && h.lock().meta.archived {
            bail!(tr!("L’agent est archivé.", "The agent is archived."));
        }
        // Deleted while it waited for the lock (the deletion has its own, now gone): a process
        // started now would run, unseen, in the folder that is being removed.
        if !self
            .agents
            .read()
            .get(id)
            .is_some_and(|a| Arc::ptr_eq(a, h))
        {
            return Err(agent_not_found());
        }
        // Taken apart: the agent is not held while its process may be stopped (`refence`).
        let live = h.lock().proc.clone();
        if let Some(p) = live {
            if p.is_alive() && !self.refence(h, id).await? {
                return Ok(p);
            }
        }
        // None starts once the app quits: the stop of the processes has been through, and one
        // started after (the warm-up of an agent just made comes as late as the runtime lets it)
        // would be left running.
        if self.quitting.load(Ordering::Acquire) {
            bail!(quitting_refusal());
        }
        // Its harness is not in the registry (« harness absent », INT-3): kept, never started.
        let engine = h.lock().meta.engine;
        if !engine::written(engine) {
            bail!(engine::absent_line(engine, crate::i18n::ui()));
        }
        // In the Claude Code view, its interface has the session (`tui`).
        if h.lock().meta.terminal {
            bail!(tr!(
                "L’agent est en vue Claude Code : c’est son terminal qui le fait tourner.",
                "The agent is in the Claude Code view: its terminal runs it."
            ));
        }
        // An interface of the agent that is quitting is gone first: never two processes on one
        // session.
        self.after_quit(id).await;
        // A session its interface named and never wrote is not resumed: it would fail, and tell a
        // loss the interface told already, or that there was not.
        self.drop_unwritten_session(h);
        let settings = self.settings.read().clone();
        // Its account's harness, where its session is kept.
        let harness = engine::of(&h.lock().meta);
        // An account removed meanwhile: another of its harness, else refused (ESC-128).
        let account = self.agent_account(h)?;
        // A Cursor account without its key would run as the machine's sign-in (ESC-43).
        accounts::key_ready(&account)?;
        let program = accounts::program(&account, &settings)
            .ok_or_else(|| anyhow!(engine::missing_line(harness.id(), i18n::ui())))?;
        // A version older than the tested ones is refused, with its reason.
        engine::registry::check(harness, &program).await?;
        // Read already by the check: kept with what its models say.
        let version = engine::registry::version(harness, &program)
            .await
            .unwrap_or_default();
        // A model none of its harness's: the one its account runs by default, as its harness lists
        // them (OpenCode, whose server never starts without a model of its own).
        let env = [settings.agent_env(), accounts::launch_env(&account)].concat();
        self.settle_model(h, harness, &program, env).await;
        let (mut spawn, gen, engine) = {
            let mut rt = h.lock();
            rt.gen += 1;
            let mut spawn = Spawn::of(
                &rt.meta,
                program,
                [settings.agent_env(), accounts::launch_env(&account)].concat(),
            );
            spawn.account = account.id.clone();
            spawn.takes_effort = (engine::of(&rt.meta).models_by_alias()
                && !rt.meta.effort.is_empty())
            .then(|| self.takes_effort(rt.meta.engine, &account.id, &rt.meta.model));
            (spawn, rt.gen, engine::of(&rt.meta))
        };
        // A relay its interface left in its folder (the app stopped in « Vue Cursor »).
        self.clear_stale_relay(h);
        // A configuration folder of the agent's own (Cursor: what its « Toujours » keeps).
        if let Some(var) = engine.agent_home_var() {
            let home = self.data.agent_home(id);
            std::fs::create_dir_all(&home).with_context(|| format!("{}", home.display()))?;
            spawn.env.retain(|(k, _)| k != var);
            spawn
                .env
                .push((var.to_string(), home.to_string_lossy().into_owned()));
        }
        if !Path::new(&spawn.cwd).is_dir() {
            bail!(tr!(
                "Le dossier {dir} n'existe plus",
                "The folder {dir} no longer exists",
                dir = spawn.cwd
            ));
        }
        self.ready_for_wsl(&spawn.program, h);
        // The folder of the files joined to its messages, which it reads without a question
        // (`attachfiles`).
        spawn.read_dirs.extend(self.agent_read_dir(id));
        // A ticket's agent reads the ticket's files too (`ticketfiles`).
        spawn.read_dirs.extend(self.ticket_read_dir(id));
        // Escouade's MCP server, as this agent (a token of its own) or kept out of it.
        let access = self.agent_access(id, engine);
        spawn.escouade = self.with_refusal_file(id, engine, access);
        // « Projet privé »: the paths its tools are refused, in a settings file of its own for a
        // harness that reads them so. Not written, it does not start: it would read them.
        let privacy = self.agent_privacy(id);
        spawn.fence = self.agent_fence(id);
        if !spawn.fence.is_empty() {
            let refused = matches!(spawn.escouade, Escouade::Denied { .. });
            if let Some(v) = engine.fence_settings(refused, &spawn.fence, &spawn.program) {
                spawn.settings = Some(self.write_fence_settings(id, &v)?);
            }
        }
        // Given back when this process ends, unless a newer one of the agent has its own.
        let token = spawn.escouade.token().map(str::to_string);
        let fence = spawn.fence.clone();
        // Its account's token out of date: after the launch that refreshes it (ESC-162).
        let stamp = self.before_claude(&account).await;
        let started = std::time::Instant::now();
        let (w1, w2) = (Arc::downgrade(self), Arc::downgrade(self));
        let (h1, h2) = (h.clone(), h.clone());
        let (agent_id, granted) = (id.to_string(), token.clone());
        let spawned = engine.spawn(
            spawn,
            Box::new(move |frame| {
                if let Some(c) = w1.upgrade() {
                    c.on_frame(&h1, gen, frame);
                }
            }),
            Box::new(move |code, stderr| {
                if let Some(c) = w2.upgrade() {
                    if let Some(t) = &granted {
                        c.mcp.release_agent(&agent_id, t);
                    }
                    c.on_exit(&h2, gen, code, stderr);
                }
            }),
        );
        let proc = match spawned {
            Ok(p) => p,
            Err(e) => {
                if let Some(t) = &token {
                    self.mcp.release_agent(id, t);
                }
                return Err(e);
            }
        };
        {
            let mut rt = h.lock();
            rt.attach(proc.clone());
            rt.launch_stamp = stamp;
            rt.fence = fence;
            rt.privacy = privacy;
            // Read under its lock: a `mark_escouade_next` (or a switch turned off) that comes after
            // sees the process, and one that came before is read here: a token forgotten meanwhile
            // gives nothing (no lock of the settings or the server is held while an agent's is
            // taken).
            let given = token
                .as_deref()
                .is_some_and(|t| self.mcp.caller(t).is_some());
            rt.started_with_escouade(given, self.escouade_granted());
        }
        // The app began to quit while it started: the stop of the processes missed this one.
        if self.quitting.load(Ordering::Acquire) {
            if let Some(p) = h.lock().proc.take() {
                p.close_input();
                p.kill();
            }
            bail!(quitting_refusal());
        }
        self.emit_agent(h);
        match proc.initialize().await {
            Ok(ready) => {
                log::info!(
                    "agent {id}: {} ready in {} ms",
                    engine.id(),
                    started.elapsed().as_millis()
                );
                h.lock().commands = ready.commands;
                // Claude Code tells the version each alias runs; Codex lists its account's.
                if !ready.models.is_null() {
                    self.set_models(&ready.models);
                    if engine.models_by_alias() {
                        self.set_account_models(&account.id, version.clone(), &ready.models);
                    }
                }
                if let Some(models) = ready.catalog {
                    // Kept by account too where its harness lists them so (Codex, ESC-186).
                    if !engine.models_by_alias() && reads_account_models(engine.id()) {
                        self.set_listed_models(&account.id, version, models);
                    } else {
                        self.set_engine_models(engine.id(), models);
                    }
                }
                if h.lock().meta.remote_control {
                    if let Err(e) = self.link_remote(h, &proc).await {
                        self.remote_unavailable(id, &e, true);
                    }
                } else {
                    // A session it held that nothing could close then (no live process linked)
                    // is closed now.
                    self.close_stale_remote_session(h, &proc).await;
                }
            }
            Err(_) if !proc.is_alive() => {
                log::warn!("agent {id}: {} exited while starting", engine.name());
                return Err(StartupFailure(engine.name()).into());
            }
            Err(e) => log::warn!(
                "agent {id}: initialize failed after {} ms: {e}",
                started.elapsed().as_millis()
            ),
        }
        Ok(proc)
    }

    /// The agent of `h` on a model of its harness: when it has none of those its harness offers
    /// (a new agent before any listed them, one handed over from another harness), the default one
    /// its account lists by a process of its own (`Engine::list_models`), kept for the composer
    /// too. Nothing for a harness that lists none so, nor when the listing fails.
    async fn settle_model(
        self: &Arc<Self>,
        h: &AgentHandle,
        harness: &dyn crate::engine::Engine,
        program: &crate::engine::Program,
        env: Vec<(String, String)>,
    ) {
        if harness.models_by_alias() {
            self.fit_claude_model(h);
            return;
        }
        let model = h.lock().meta.model.clone();
        // Codex's stand-in (GPT-6-Luna) serves only while nothing was listed (ESC-186); the others
        // declare aliases their lists lack (Copilot's `auto`, Cursor's `default`).
        let told: Vec<String> = self
            .engine_models
            .read()
            .get(&harness.id())
            .map(|t| t.iter().map(|m| m.id.clone()).collect())
            .unwrap_or_default();
        let listed = told.contains(&model);
        let declared = harness.models().iter().any(|m| m.id == model);
        let known = if harness.listed_models_only() && !told.is_empty() {
            listed
        } else {
            listed || declared
        };
        if known {
            return;
        }
        let dir = std::env::temp_dir();
        let listed = match harness.list_models(program.clone(), env, &dir).await {
            Ok(Some(list)) => list,
            Ok(None) => return,
            Err(e) => {
                log::warn!("{}: models not listed: {e:#}", harness.name());
                return;
            }
        };
        self.set_engine_models(harness.id(), listed.clone());
        if listed.iter().any(|m| m.id == model) {
            return;
        }
        let Some(pick) = listed.iter().find(|m| m.default).or(listed.first()) else {
            return;
        };
        log::info!(
            "{}: model {model:?} is none of its, {} instead",
            harness.name(),
            pick.id
        );
        h.lock().meta.model = pick.id.clone();
        self.emit_agent(h);
        self.request_save();
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

    /// Keeps what the `claude` of `account` (at `version`) told of its models at a process start
    /// or a reading of its own (`read_claude_models`): which take the mode `auto` (`takes_auto`),
    /// and the menu of its agents.
    fn set_account_models(&self, account: &str, version: String, reported: &Value) {
        let models = crate::model::AccountModels::new(version, ModelInfo::list(reported));
        let mut all = self.account_models.write();
        // When it was read aside (`read_at`): unchanged, the window is not told again.
        let same = all
            .get(account)
            .is_some_and(|m| m.version == models.version && m.models == models.models);
        all.insert(account.to_string(), models.clone());
        if same {
            self.request_save();
            return;
        }
        self.hub.emit(UiEvent::AccountModels {
            account: account.to_string(),
            models,
        });
        self.request_save();
    }

    /// Whether `model` takes the mode `auto` on `engine` for an agent of `account`: as the
    /// `claude` of the account last told it, else as its harness's rule has it
    /// (`Engine::supports_auto`). Followed by the tickets and the handover; the Composer reads the
    /// same (`modelTakes`).
    pub(crate) fn takes_auto(&self, engine: EngineId, account: &str, model: &str) -> bool {
        let harness = crate::engine::by_id(engine);
        let told = self
            .account_models
            .read()
            .get(account)
            .and_then(|m| harness.told_auto(&m.models, model));
        told.unwrap_or_else(|| harness.supports_auto(model))
    }

    /// Whether `model` takes an effort on `engine` for an agent of `account`: as the `claude` of
    /// the account last told it (ESC-124: Haiku 5.5 takes one), else as its harness's rule has it.
    pub(crate) fn takes_effort(&self, engine: EngineId, account: &str, model: &str) -> bool {
        let harness = crate::engine::by_id(engine);
        let told = self
            .account_models
            .read()
            .get(account)
            .and_then(|m| harness.told_effort(&m.models, model));
        told.unwrap_or_else(|| harness.supports_effort(model))
    }

    /// The agent of `h` on a model of its Claude account (ESC-124): when the `claude` of the
    /// account told its models and lists none that its model names (a model of another account it
    /// moved to, ESC-88), the default one, as the settings name it when the account has it, else
    /// as its `claude` runs it; its conversation says so. Nothing while the account told none.
    pub(crate) fn fit_claude_model(self: &Arc<Self>, h: &AgentHandle) {
        let (id, account, model) = {
            let rt = h.lock();
            (
                rt.meta.id.clone(),
                rt.meta.account.clone(),
                rt.meta.model.clone(),
            )
        };
        let account = accounts::get(&self.settings.read(), &account);
        let Some(told) = self.account_models.read().get(&account.id).cloned() else {
            return;
        };
        let listed = |m: &str| crate::engine::claude_told_model(&told.models, m).is_some();
        if told.catalog.is_empty() || listed(&model) {
            return;
        }
        let wanted = accounts::defaults(Some(&account), account.engine).model;
        let Some(pick) = told
            .catalog
            .iter()
            .find(|m| m.id == wanted)
            .or_else(|| told.catalog.iter().find(|m| m.default))
            .or(told.catalog.first())
            .cloned()
        else {
            return;
        };
        log::info!(
            "agent {id}: model {model:?} not listed by account {}, {} instead",
            account.id,
            pick.id
        );
        let lang = crate::i18n::ui();
        let name = accounts::name_in(lang, &account);
        let takes = self.takes_effort(account.engine, &account.id, &pick.id);
        let _ = self.with_agent(&id, |rt, fx| {
            rt.meta.model = pick.id.clone();
            if takes && rt.meta.effort.is_empty() {
                rt.meta.effort = pick
                    .efforts
                    .iter()
                    .find(|e| e.as_str() == "medium")
                    .or(pick.efforts.first())
                    .cloned()
                    .unwrap_or_default();
            }
            rt.notice(
                "info",
                tr_in!(
                    lang,
                    "Le compte {account} n’a pas le modèle {model} : l’agent passe sur {pick}, son modèle par défaut.",
                    "The {account} account doesn’t have the {model} model: the agent goes on {pick}, its default model.",
                    account = name,
                    model = model,
                    pick = pick.label
                ),
                fx,
            );
            Ok(())
        });
    }

    /// What the `claude` of the Claude account `id` tells of its models, read without a turn
    /// (`engine::claude_read_models`, ESC-124) and kept: at the start of the app, once an account
    /// is made or signed in. Nothing for an account that is not Claude Code's, switched off, or
    /// whose `claude` is not found.
    pub async fn read_claude_models(self: &Arc<Self>, id: &str) {
        let settings = self.settings.read().clone();
        let Some(account) = settings.accounts.iter().find(|a| a.id == id).cloned() else {
            return;
        };
        let harness = crate::engine::by_id(account.engine);
        if !harness.models_by_alias() || !account.active {
            return;
        }
        let found = {
            let (account, settings) = (account.clone(), settings.clone());
            tokio::task::spawn_blocking(move || accounts::program(&account, &settings))
                .await
                .ok()
                .flatten()
        };
        let Some(program) = found else { return };
        // One older than the tested ones is not started, as an agent's is not.
        if let Err(e) = engine::registry::check(harness, &program).await {
            log::info!("account {id}: models not read: {e:#}");
            return;
        }
        let version = engine::registry::version(harness, &program)
            .await
            .unwrap_or_default();
        let env = [settings.agent_env(), accounts::launch_env(&account)].concat();
        let dir = std::env::temp_dir();
        self.before_claude(&account).await;
        match harness.read_told_models(program, env, &dir).await {
            Ok(models) if !models.is_null() => {
                self.set_account_models(&account.id, version, &models);
            }
            Ok(_) => {}
            Err(e) => log::warn!("account {id}: models not read: {e:#}"),
        }
    }

    /// What the harness of the account `id` lists of its models, read without an agent and kept by
    /// account: Claude Code's by `read_claude_models`, Codex's by an app-server of its own
    /// (`Engine::list_models`, every page of `model/list`, ESC-186). At the start of the app, once
    /// an account is made or signed in, and when the menu of its models opens on a list a day old
    /// (`refresh_account_models`). Nothing for another harness, an account switched off, or whose
    /// harness is not found; one reading of an account at a time.
    pub async fn read_account_models(self: &Arc<Self>, id: &str) {
        let Some(account) = self
            .settings
            .read()
            .accounts
            .iter()
            .find(|a| a.id == id)
            .cloned()
        else {
            return;
        };
        if !account.active || !reads_account_models(account.engine) {
            return;
        }
        if !self.models_reading.lock().insert(id.to_string()) {
            return;
        }
        if crate::engine::by_id(account.engine).models_by_alias() {
            self.read_claude_models(id).await;
        } else {
            self.read_harness_models(&account).await;
        }
        self.models_reading.lock().remove(id);
    }

    /// `read_account_models` of a harness that lists its models by a process of its own (Codex).
    async fn read_harness_models(self: &Arc<Self>, account: &Account) {
        let settings = self.settings.read().clone();
        let found = {
            let (account, settings) = (account.clone(), settings.clone());
            tokio::task::spawn_blocking(move || accounts::program(&account, &settings))
                .await
                .ok()
                .flatten()
        };
        let Some(program) = found else { return };
        let harness = crate::engine::by_id(account.engine);
        // Read even from a version older than the tested ones (`registry::check`): `model/list`
        // reads and starts nothing, and the menu would say its list is being read for good.
        let version = engine::registry::version(harness, &program)
            .await
            .unwrap_or_default();
        let env = [settings.agent_env(), accounts::launch_env(account)].concat();
        let started = std::time::Instant::now();
        match harness
            .list_models(program, env, &std::env::temp_dir())
            .await
        {
            Ok(Some(models)) => {
                log::info!(
                    "account {}: {} models of {} read in {} ms",
                    account.id,
                    models.len(),
                    harness.name(),
                    started.elapsed().as_millis()
                );
                self.set_listed_models(&account.id, version, models);
            }
            Ok(None) => {}
            Err(e) => log::warn!("account {}: models not read: {e:#}", account.id),
        }
    }

    /// Keeps the models the harness of `account` (at `version`) listed, by an agent's start or a
    /// reading of its own: the menu of its agents, and of its harness while none says its account.
    fn set_listed_models(
        &self,
        account: &str,
        version: String,
        models: Vec<crate::engine::registry::ToldModel>,
    ) {
        let engine = accounts::get(&self.settings.read(), account).engine;
        self.set_engine_models(engine, models.clone());
        // As the harness-wide list has them (OpenCode's providers by their names, `set_engine_models`).
        let models = self
            .engine_models
            .read()
            .get(&engine)
            .cloned()
            .unwrap_or(models);
        let catalog = {
            let settings = self.settings.read();
            let account = accounts::get(&settings, account);
            // What its folder adds (Codex: the model its `config.toml` names, ESC-186).
            crate::engine::by_id(engine)
                .account_models(models, &accounts::config_dir(&account, &settings))
        };
        let told = crate::model::AccountModels {
            version,
            models: Vec::new(),
            catalog,
            read_at: chrono::Utc::now().timestamp_millis(),
        };
        let mut all = self.account_models.write();
        let same = all
            .get(account)
            .is_some_and(|m| m.version == told.version && m.catalog == told.catalog);
        all.insert(account.to_string(), told.clone());
        if !same {
            self.hub.emit(UiEvent::AccountModels {
                account: account.to_string(),
                models: told,
            });
        }
        self.request_save();
    }

    /// The menu of the models of the account `id` opened (ESC-186): its models read again when
    /// none were, or a day ago or more.
    pub async fn refresh_account_models(self: &Arc<Self>, id: &str) {
        let now = chrono::Utc::now().timestamp_millis();
        let fresh = self
            .account_models
            .read()
            .get(id)
            .is_some_and(|m| !m.catalog.is_empty() && now - m.read_at < MODELS_KEPT_MS);
        if !fresh {
            self.read_account_models(id).await;
        }
    }

    /// `read_account_models` of every active account whose harness lists its models so, in the
    /// background.
    pub fn read_all_account_models(self: &Arc<Self>) {
        let ids: Vec<String> = self
            .settings
            .read()
            .accounts
            .iter()
            .filter(|a| reads_account_models(a.engine) && a.active)
            .map(|a| a.id.clone())
            .collect();
        for id in ids {
            let c = self.clone();
            tauri::async_runtime::spawn(async move { c.read_account_models(&id).await });
        }
    }

    /// The settings saved before 1.8.1 brought to the accounts added alone (ESC-128,
    /// `accounts::migrate`), once, at the start: a Principal signed in, or that served (an agent,
    /// a project that prefers it, a line of the statistics), stays; another goes, its MCP entry
    /// taken out of its config. Nothing for settings brought already, a new installation's too.
    pub async fn migrate_accounts(self: &Arc<Self>) {
        let principals: Vec<Account> = {
            let s = self.settings.read();
            if s.accounts_version >= accounts::ACCOUNTS_VERSION {
                return;
            }
            // Claude Code's too when the settings do not list it (`accounts::migrate`).
            let missing =
                (!s.accounts.iter().any(|a| a.id == accounts::PRINCIPAL)).then(accounts::principal);
            s.accounts
                .iter()
                .filter(|a| accounts::is_principal(a))
                .cloned()
                .chain(missing)
                .collect()
        };
        let mut keep = HashSet::new();
        for a in &principals {
            if self.account_served(&a.id) || self.signed_in(a).await {
                keep.insert(a.id.clone());
            }
        }
        let dropped: Vec<Account> = principals
            .into_iter()
            .filter(|a| !keep.contains(&a.id))
            .collect();
        let migrated = self.change_claude_accounts(|s| {
            accounts::migrate(s, |a| keep.contains(&a.id));
            Ok(())
        });
        if let Err(e) = migrated {
            log::warn!("accounts not brought to 1.8.1: {e:#}");
            return;
        }
        log::info!(
            "accounts brought to 1.8.1: Principals kept {keep:?}, dropped {:?}",
            dropped.iter().map(|a| &a.id).collect::<Vec<_>>()
        );
        // The MCP server is declared in the accounts added alone (ESC-119): out of the
        // Principals dropped, and of the machine's config of a harness without an account, where
        // it was declared until now.
        let left: Vec<Account> = {
            let s = self.settings.read();
            engine::registry::all()
                .iter()
                .map(|e| e.id())
                .filter(|e| *e != EngineId::CLAUDE && !accounts::has_any(&s, *e))
                .filter(|e| !dropped.iter().any(|a| a.engine == *e))
                .map(accounts::principal_of)
                .collect()
        };
        for gone in dropped.into_iter().chain(left) {
            self.withdraw_removed(gone);
        }
        let (accounts, engine_order) = {
            let s = self.settings.read();
            (s.accounts.clone(), s.engine_order.clone())
        };
        self.hub.emit(UiEvent::Accounts {
            accounts,
            engine_order,
        });
        self.settle_usage();
    }

    /// Whether the account `id` served: an agent on it (archived ones too), a project that prefers
    /// it, a turn or a message of the statistics written on it.
    fn account_served(&self, id: &str) -> bool {
        self.agents
            .read()
            .values()
            .any(|h| h.lock().meta.account == id)
            || self.projects.read().iter().any(|p| p.account == id)
            || self.stats.used_by(id)
    }

    /// Whether the harness of `account` is signed in there (OpenCode, which signs in to none, is
    /// told by what it served alone). Claude Code's too when its `.claude.json` names the
    /// claude.ai account it signed in with: a keychain that does not answer drops nobody.
    async fn signed_in(&self, account: &Account) -> bool {
        let harness = crate::engine::by_id(account.engine);
        if !harness.signs_in() {
            return false;
        }
        let settings = self.settings.read().clone();
        if harness.signed_in_by_files(account, &settings) {
            return true;
        }
        self.status_of(account).await.connected
    }

    /// The account agent `h` is started on: its own, when the settings have it for its harness;
    /// else (removed, ESC-128: never another one in its place silently) the one a new agent of its
    /// project on its harness goes to, its own from then on. Refused when no account of its
    /// harness serves its project (`account_for_new`).
    pub(crate) fn agent_account(&self, h: &AgentHandle) -> Result<Account> {
        let (id, own, project, engine) = {
            let rt = h.lock();
            let m = &rt.meta;
            (
                m.id.clone(),
                m.account.clone(),
                m.project_id.clone(),
                m.engine,
            )
        };
        let settings = self.settings.read().clone();
        if let Some(a) = accounts::find(&settings, &own).filter(|a| a.engine == engine) {
            return Ok(a.clone());
        }
        let next = self.account_for_new(&project, engine)?;
        log::warn!("agent {id}: account {own:?} unknown, {next:?} instead");
        h.lock().meta.account = next.clone();
        self.request_save();
        Ok(accounts::get(&settings, &next))
    }

    /// Keeps the models a harness listed for its account at a process start (Codex: `model/list`):
    /// the composer's menu of its agents.
    fn set_engine_models(
        &self,
        engine: EngineId,
        mut models: Vec<crate::engine::registry::ToldModel>,
    ) {
        let mut all = self.engine_models.write();
        // A list that knows a provider by its id alone (OpenCode's `opencode models`, without its
        // server) keeps the name its server gave it before (`Ollama (local)`, not `ollama`).
        if let Some(known) = all.get(&engine) {
            for m in &mut models {
                let Some((prefix, _)) = m.id.split_once('/') else {
                    continue;
                };
                if m.provider.as_deref().is_some_and(|p| p != prefix) {
                    continue;
                }
                let named = known.iter().find_map(|k| {
                    let p = k.provider.as_deref()?;
                    (k.id.split_once('/').map(|(kp, _)| kp) == Some(prefix) && p != prefix)
                        .then(|| p.to_string())
                });
                if let Some(name) = named {
                    m.provider = Some(name);
                }
            }
        }
        if all.get(&engine) == Some(&models) {
            return;
        }
        all.insert(engine, models.clone());
        self.hub.emit(UiEvent::EngineModels { engine, models });
        self.request_save();
    }

    /// The model of a new agent of `engine` when `wanted` is none of its own: the one its account
    /// runs by default (`model/list`), else the first of its menu. Claude Code takes `wanted`
    /// (its aliases and full ids alike).
    pub(crate) fn model_for(&self, engine: EngineId, wanted: String) -> String {
        if crate::engine::by_id(engine).models_by_alias() {
            return wanted;
        }
        let told = self
            .engine_models
            .read()
            .get(&engine)
            .cloned()
            .unwrap_or_default();
        let declared = crate::engine::by_id(engine).models();
        // Codex's stand-in serves only while nothing was listed (ESC-186, as `settle_model`).
        let listed = told.iter().any(|m| m.id == wanted);
        let declared_one = declared.iter().any(|m| m.id == wanted);
        let ok = if crate::engine::by_id(engine).listed_models_only() && !told.is_empty() {
            listed
        } else {
            listed || declared_one
        };
        if ok {
            return wanted;
        }
        told.iter()
            .find(|m| m.default)
            .or(told.first())
            .map(|m| m.id.clone())
            .or_else(|| declared.first().map(|m| m.id.to_string()))
            .unwrap_or(wanted)
    }

    /// Starts the agent's process in the background so the first message answers fast. In the
    /// Claude Code view there is none to warm up, but the interface of an agent that holds
    /// Remote Control (it stays up).
    pub fn warm(self: &Arc<Self>, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let interface = {
            let rt = h.lock();
            if rt.meta.terminal {
                if !rt.meta.remote_control || rt.meta.archived {
                    return;
                }
                true
            } else {
                // Its worktree, which waits for its first message, is where it is to start.
                if rt.proc.is_some()
                    || rt.meta.archived
                    || rt.meta.status == AgentStatus::Error
                    || rt.meta.worktree_pending
                {
                    return;
                }
                false
            }
        };
        let (c, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move {
            if interface {
                if let Err(e) = c.start_interface(&id).await {
                    c.remote_unavailable(&id, &e, false);
                }
                // Cut while it started: it has no reason to run.
                if let Ok(h) = c.agent(&id) {
                    c.stop_unseen_interface(&h);
                }
            } else if let Err(e) = c.ensure_process_for(&id, true).await {
                log::warn!("warm-up failed: {e:#}");
            }
        });
    }

    /// « Relancer »: the agent's process, at rest, stopped (its input closed, killed if it does
    /// not end) and started again on the same session (`--resume`): nothing of its conversation is
    /// lost, and the new process has what a start gives now, Escouade's server among it once
    /// « Claude et les agents peuvent piloter Escouade » was turned on after it started (ESC-91).
    /// Refused while it works, waits for an answer, has its worktree set up or runs a subagent, a
    /// workflow or a command in the background (what it waits for, or does, would be lost), and
    /// once archived.
    pub async fn restart_process(self: &Arc<Self>, id: &str) -> Result<()> {
        let h = self.agent(id)?;
        {
            // Its process is not started again meanwhile (a message, a warm-up wait for the lock).
            let lock = self.spawn_lock(id);
            let _guard = lock.lock().await;
            let stopped = {
                let mut rt = h.lock();
                if rt.meta.terminal {
                    bail!(tr!(
                        "L’agent est en vue Claude Code : relance son interface depuis son terminal.",
                        "The agent is in the Claude Code view: restart its interface from its terminal."
                    ));
                }
                if rt.meta.archived {
                    bail!(tr!("L’agent est archivé.", "The agent is archived."));
                }
                let background = rt.meta.plan.as_ref().is_some_and(|p| p.runs_something());
                if rt.meta.status.is_active() || rt.setup.is_some() || background {
                    bail!(tr!(
                        "L’agent travaille encore : relance son process quand il aura fini.",
                        "The agent is still working: restart its process once it is done."
                    ));
                }
                rt.detach()
            };
            if let Some(p) = stopped {
                self.emit_agent(&h);
                if !accounts::ended(&p).await {
                    bail!(tr!(
                        "Le process de l’agent ne s’arrête pas.",
                        "The agent’s process won’t stop."
                    ));
                }
            }
        }
        self.ensure_process(id).await?;
        Ok(())
    }

    // ---------- remote control ----------

    /// Name of the agent's session in claude.ai / the Claude app.
    pub(crate) fn remote_name(&self, meta: &AgentMeta) -> String {
        let project = self
            .project(&meta.project_id)
            .map(|p| p.name)
            .unwrap_or_default();
        format!("{project} · {}", meta.name)
    }

    /// The title of the agent's session for its live process: its name, or, while Remote Control
    /// links it to claude.ai, the name it has there (« projet · agent »), which Claude Code may
    /// show in place of the session's own title.
    fn session_title(&self, rt: &AgentRt) -> String {
        if rt.remote_linked {
            self.remote_name(&rt.meta)
        } else {
            rt.meta.name.clone()
        }
    }

    /// The global Remote Control setting and the agent's project's, in that order. Read with no
    /// lock of an agent held. The order of the locks is the agent's first, then the projects'
    /// (read only: `remote_name`, `session_title`, the start of an interface), never the other
    /// way round; the settings' are never held with an agent's.
    fn remote_settings(&self, h: &AgentHandle) -> (bool, bool) {
        let global = self.settings.read().remote_control_all;
        let project_id = h.lock().meta.project_id.clone();
        let project = self.project(&project_id).is_ok_and(|p| p.remote_control);
        (global, project)
    }

    /// Links the agent's live process to claude.ai (Remote Control). Its previous remote session
    /// is reattached, so the link opened on a phone stays valid across restarts.
    async fn link_remote(self: &Arc<Self>, h: &AgentHandle, proc: &Process) -> Result<()> {
        let id = {
            let mut rt = h.lock();
            rt.remote_tried = true;
            rt.meta.id.clone()
        };
        // After a closing of the agent's remote session that is under way: its requests and this
        // one's never interleave on the process (the session it closes is no longer there to
        // reattach when this one begins).
        let lock = self.remote_lock(&id);
        let _held = lock.lock().await;
        let (name, reattach) = {
            let rt = h.lock();
            // Cut meanwhile, or gone to the Claude Code view: nothing to link.
            if !rt.meta.remote_control || rt.meta.terminal {
                return Ok(());
            }
            (self.remote_name(&rt.meta), rt.meta.remote_session.clone())
        };
        let request = |reattach: Option<String>| Remote::On {
            name: name.clone(),
            keep: true,
            reattach,
        };
        let timeout = Duration::from_secs(30);
        let resp = match proc
            .remote_control(request(reattach.clone()), timeout)
            .await
        {
            Ok(r) => r,
            // The previous remote session is gone: open a new one.
            Err(_) if reattach.is_some() && proc.is_alive() => {
                proc.remote_control(request(None), timeout).await?
            }
            Err(e) => return Err(e),
        };
        let in_force = {
            let mut rt = h.lock();
            // Cut meanwhile (the link took its time), or gone to the Claude Code view, whose
            // interface holds the session: nothing of the link is kept.
            let in_force = rt.meta.remote_control && !rt.meta.terminal;
            if in_force {
                rt.remote_linked = true;
                // It works: the next failure is news again.
                rt.remote_told = false;
                rt.meta.remote_session = resp["bridge_session_id"].as_str().map(str::to_string);
                rt.meta.remote_url = resp["session_url"].as_str().map(str::to_string);
            }
            in_force
        };
        if !in_force {
            let bridge = resp["bridge_session_id"].as_str();
            if close_remote_session(&id, proc, bridge, &name, Duration::from_secs(15)).await {
                // The id of the session it reattached, if the agent still kept it, is its no more.
                h.lock().forget_remote_session(bridge);
                self.request_save();
            }
            return Ok(());
        }
        self.request_save();
        self.emit_agent(h);
        Ok(())
    }

    /// Tells the agent that its Remote Control could not be linked (the setting stays in force:
    /// its next start tries again), once: not again at every start, until a link works or the user
    /// turns it on by hand (`AgentRt::remote_told`). `stream`: it is the stream's process that
    /// failed.
    pub(crate) fn remote_unavailable(self: &Arc<Self>, id: &str, e: &anyhow::Error, stream: bool) {
        log::warn!("agent {id}: remote control failed: {e:#}");
        let _ = self.with_agent(id, |rt, fx| {
            // Cut meanwhile, or (for the stream) gone to the Claude Code view, whose interface
            // holds the session: there is nothing to tell.
            if !rt.meta.remote_control || (stream && rt.meta.terminal) {
                return Ok(());
            }
            rt.remote_unavailable_notice(
                tr!(
                    "Remote control indisponible : {e}",
                    "Remote control unavailable: {e}"
                ),
                fx,
            );
            Ok(())
        });
    }

    /// Ends the agent's Remote Control: no longer in force, at once, and its remote session
    /// closed for good (its live process, when linked, is told). Neither sets it apart
    /// (`remote_off`) nor changes what a setting wants for it. A session that cannot be closed,
    /// or that no live process can close now, keeps its id (`AgentRt::end_link`): the agent's
    /// next process closes it (`close_stale_remote_session`), or takes it back if Remote Control
    /// is back by then.
    async fn cut_remote(self: &Arc<Self>, h: &AgentHandle) {
        let (name, link) = {
            let mut rt = h.lock();
            (self.remote_name(&rt.meta), rt.end_remote())
        };
        self.request_save();
        self.emit_agent(h);
        // An interface started for it, and that no window showed, has no reason to run.
        self.stop_unseen_interface(h);
        if let Some(link) = link {
            // Unless Remote Control came back meanwhile: the link that follows takes the session.
            self.close_link(h, link, &name, Duration::from_secs(15), true)
                .await;
        }
    }

    /// Closes the remote session of the agent that `link` held (the process that held it), when
    /// no other request on its Remote Control is under way (`remote_locks`), and forgets its id
    /// once it is closed. With `unless_back`, nothing is closed if the agent holds Remote Control
    /// again by then: its link reattaches the session, which is still there.
    pub(crate) async fn close_link(
        self: &Arc<Self>,
        h: &AgentHandle,
        link: RemoteLink,
        name: &str,
        timeout: Duration,
        unless_back: bool,
    ) {
        let id = h.lock().meta.id.clone();
        let lock = self.remote_lock(&id);
        let _held = lock.lock().await;
        if unless_back && h.lock().meta.remote_control {
            return;
        }
        if close_remote_session(&id, &link.proc, link.bridge.as_deref(), name, timeout).await {
            h.lock().forget_remote_session(link.bridge.as_deref());
            self.request_save();
        }
    }

    /// The agent's process, just started, closes the remote session the agent still names though
    /// it holds no Remote Control: one that no live process could close when it ended
    /// (`AgentRt::end_link`). Its id is forgotten once it is closed.
    async fn close_stale_remote_session(self: &Arc<Self>, h: &AgentHandle, proc: &Process) {
        let id = h.lock().meta.id.clone();
        let lock = self.remote_lock(&id);
        let _held = lock.lock().await;
        let (name, bridge) = {
            let rt = h.lock();
            // Turned on meanwhile: its link reattaches that session.
            if rt.meta.remote_control {
                return;
            }
            let Some(bridge) = rt.meta.remote_session.clone() else {
                return;
            };
            (self.remote_name(&rt.meta), bridge)
        };
        if close_remote_session(&id, proc, Some(&bridge), &name, Duration::from_secs(15)).await {
            h.lock().forget_remote_session(Some(&bridge));
            self.request_save();
        }
    }

    /// The user turns Remote Control on (the agent's process starts if needed and stays up) or off
    /// for this agent, from its menu. On, it is the agent's own whatever the settings do; off, the
    /// agent is set apart, whether a setting covers it or not: the settings give it none until the
    /// user turns it on again (its copies, and a ticket's next agents, are set apart too). True
    /// when, in the Claude Code view, the change waits for the interface's next start: it runs,
    /// and was started with another state of Remote Control than the one asked for.
    pub async fn set_remote_control(self: &Arc<Self>, id: &str, enabled: bool) -> Result<bool> {
        let h = self.agent(id)?;
        if !enabled {
            h.lock().meta.remote_off = true;
            self.remember_remote_off(id, true);
            self.cut_remote(&h).await;
            return Ok(self.waits_for_next_start(&h, false));
        }
        let (before, terminal) = {
            let mut rt = h.lock();
            let before = (
                rt.meta.remote_control,
                rt.meta.remote_source,
                rt.meta.remote_off,
            );
            rt.meta.remote_control = true;
            rt.meta.remote_source = Some(RemoteSource::Hand);
            rt.meta.remote_off = false;
            // By hand: the interface tries the option again, and what fails is news again.
            rt.remote_fallback = false;
            rt.remote_told = false;
            (before, rt.meta.terminal)
        };
        self.remember_remote_off(id, false);
        self.request_save();
        // In the Claude Code view the interface holds the session: it is started with Remote
        // Control (a running one has it from its next start).
        let linked = if terminal {
            self.start_interface(id).await
        } else {
            match self.ensure_process(id).await {
                Ok(_) if h.lock().remote_linked => Ok(()),
                Ok(proc) => self.link_remote(&h, &proc).await,
                Err(e) => Err(e),
            }
        };
        if let Err(e) = linked {
            {
                let mut rt = h.lock();
                (
                    rt.meta.remote_control,
                    rt.meta.remote_source,
                    rt.meta.remote_off,
                ) = before;
            }
            self.remember_remote_off(id, before.2);
            self.request_save();
            self.emit_agent(&h);
            return Err(e.context(tr!(
                "Remote control indisponible",
                "Remote control unavailable"
            )));
        }
        self.emit_agent(&h);
        Ok(self.waits_for_next_start(&h, true))
    }

    /// In the Claude Code view, an interface still runs that was started with another state of
    /// Remote Control than `enabled`, the one just asked for: the change is for its next start.
    fn waits_for_next_start(&self, h: &AgentHandle, enabled: bool) -> bool {
        let rt = h.lock();
        rt.meta.terminal && rt.tui.as_ref().is_some_and(|t| t.remote != enabled)
    }

    /// Gives the agent the Remote Control the settings want for it, or takes back what they no
    /// longer want it to have (`remote_wanted`). What the user turned on by hand stays; so does
    /// what it set apart. A Remote Control that cannot be linked stays in force, and the agent
    /// says why: its next start tries again.
    pub(crate) async fn apply_remote(self: &Arc<Self>, id: &str) {
        enum Step {
            Nothing,
            Changed,
            On,
            Off,
        }
        let Ok(h) = self.agent(id) else { return };
        let (global, project) = self.remote_settings(&h);
        let step = {
            let mut rt = h.lock();
            match (
                remote_wanted(&rt.meta, global, project),
                rt.meta.remote_control,
            ) {
                (Some(source), false) => {
                    rt.meta.remote_control = true;
                    rt.meta.remote_source = Some(source);
                    Step::On
                }
                (Some(source), true) if rt.meta.remote_source != Some(source) => {
                    rt.meta.remote_source = Some(source);
                    Step::Changed
                }
                (None, true) => Step::Off,
                _ => Step::Nothing,
            }
        };
        match step {
            Step::Nothing => {}
            Step::Changed => {
                self.request_save();
                self.emit_agent(&h);
            }
            Step::Off => self.cut_remote(&h).await,
            Step::On => {
                self.request_save();
                self.emit_agent(&h);
                let (terminal, setting_up) = {
                    let rt = h.lock();
                    (
                        rt.meta.terminal,
                        rt.setup.is_some() || rt.meta.worktree_pending,
                    )
                };
                // A new worktree is set up before the process starts (which links it, once it is
                // over), or waits for the first message (ESC-156), which starts it.
                if setting_up {
                    return;
                }
                // Not all at once: a setting turned on for every agent starts three at a time.
                let _start = self.remote_starts.acquire().await;
                // The setting went off while it waited for its turn: nothing to start.
                if !h.lock().meta.remote_control {
                    return;
                }
                // Its interface holds the session in the Claude Code view: started with it.
                if terminal {
                    if let Err(e) = self.start_interface(id).await {
                        self.remote_unavailable(id, &e, false);
                    }
                    // Its place is kept until the interface is up: it has launched, it has not
                    // started.
                    self.wait_interface_ready(&h, REMOTE_START_READY).await;
                    // The setting went off while it started: no reason for it to run.
                    self.stop_unseen_interface(&h);
                    return;
                }
                // Through the start, which waits for one under way: that start links it (and says
                // why when it cannot), unless it began before the setting did.
                let linked = match self.ensure_process(id).await {
                    Ok(p) => {
                        let (linked, tried) = {
                            let rt = h.lock();
                            (rt.remote_linked, rt.remote_tried)
                        };
                        if linked || tried {
                            Ok(())
                        } else {
                            self.link_remote(&h, &p).await
                        }
                    }
                    Err(e) => Err(e),
                };
                if let Err(e) = linked {
                    self.remote_unavailable(id, &e, true);
                }
            }
        }
    }

    /// `apply_remote` on its own task. The task is for whoever waits for it (tests).
    pub(crate) fn apply_remote_soon(
        self: &Arc<Self>,
        id: &str,
    ) -> tauri::async_runtime::JoinHandle<()> {
        let (c, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move { c.apply_remote(&id).await })
    }

    /// `apply_remote` on every agent of the project (every agent, with none), each on its own
    /// task: a setting that changes does not wait for the processes it starts.
    pub(crate) fn apply_remote_to(
        self: &Arc<Self>,
        project: Option<&str>,
    ) -> Vec<tauri::async_runtime::JoinHandle<()>> {
        let ids: Vec<String> = self
            .agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                project
                    .is_none_or(|p| rt.meta.project_id == p)
                    .then(|| rt.meta.id.clone())
            })
            .collect();
        ids.iter().map(|id| self.apply_remote_soon(id)).collect()
    }

    /// With the app: the settings are applied to every agent (`start_remote`).
    pub fn start_remote_agents(self: &Arc<Self>) {
        let ids: Vec<String> = self.agents.read().keys().cloned().collect();
        self.start_remote(ids);
    }

    /// The settings applied to the agents `ids` (all of them when the app starts, those of a
    /// project opened again): the ones left in force without a setting that covers them go,
    /// the ones a setting covers come in, and those that hold Remote Control start their process
    /// (their interface, in the Claude Code view), so they are reachable from claude.ai.
    pub(crate) fn start_remote(self: &Arc<Self>, ids: Vec<String>) {
        let held: Vec<(String, bool)> = ids
            .into_iter()
            .filter_map(|id| {
                let held = self.agent(&id).ok()?.lock().meta.remote_control;
                Some((id, held))
            })
            .collect();
        for (id, held) in held {
            let c = self.clone();
            tauri::async_runtime::spawn(async move {
                // One that comes in starts its process on its own.
                c.apply_remote(&id).await;
                if !held {
                    return;
                }
                let Ok(h) = c.agent(&id) else { return };
                let (holds, terminal) = {
                    let rt = h.lock();
                    (
                        rt.meta.remote_control && !rt.meta.archived,
                        rt.meta.terminal,
                    )
                };
                if !holds {
                    return;
                }
                // Not all at once: the agents of the last run start three at a time.
                let _start = c.remote_starts.acquire().await;
                // The interface, in the Claude Code view, runs with no window until one asks.
                let started = if terminal {
                    c.start_interface(&id).await
                } else {
                    c.ensure_process(&id).await.map(|_| ())
                };
                if let Err(e) = started {
                    log::warn!("agent {id}: remote control start failed: {e:#}");
                }
                if terminal {
                    // Its place is kept until the interface is up: it has launched, it has not
                    // started.
                    c.wait_interface_ready(&h, REMOTE_START_READY).await;
                    // Cut while its interface started: it has no reason to run.
                    c.stop_unseen_interface(&h);
                }
            });
        }
    }

    /// Stopped by the usage limit: plans to send the agent "continue" once the limit resets (as
    /// Claude Code told, else the saturated window's of its account), unless turned off in the
    /// settings.
    pub fn plan_resume(self: &Arc<Self>, id: &str, resets_at: Option<i64>) {
        if !self.settings.read().auto_resume {
            return;
        }
        // Only a reset still to come: past one, retrying would only meet the limit again.
        let now = now_ms();
        let when = resets_at.filter(|t| *t > now).or_else(|| {
            let account = self.account_of(id);
            let u = self.usage.lock();
            let a = u.account(&account)?;
            a.meters
                .iter()
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

    /// "continue", as the agent stopped by the usage limit is sent it when it goes on. A ticket's
    /// agent that cannot go on blocks its ticket, which would wait forever.
    pub(crate) async fn send_continue(self: &Arc<Self>, id: &str) -> Result<()> {
        let text = auto_resume(self.lang().claude);
        match self.doing_ticket_of(id) {
            Some(ticket_id) => self.send_or_block(&ticket_id, id, text).await,
            None => self.send_by_app(id, text).await,
        }
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
            if let Err(e) = self.send_continue(&id).await {
                log::warn!("agent {id}: resume after the usage limit failed: {e:#}");
                let _ = self.with_agent(&id, |rt, fx| {
                    rt.notice(
                        "warn",
                        tr!(
                            "Reprise automatique impossible : {e:#}",
                            "Automatic resume failed: {e:#}"
                        ),
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
        let procs: Vec<(String, Process)> = self
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
                // A Remote Control agent must stay reachable from claude.ai. One with a command, a
                // monitor, a subagent or a workflow running in the background is not idle: they
                // live in its process, which stopping would kill. Nor one waiting for a job of its
                // session or the wake-up of a loop: they fire only in that process.
                let idle = rt.proc.is_some()
                    && !rt.meta.remote_control
                    && !rt.meta.status.is_active()
                    && !rt
                        .meta
                        .plan
                        .as_ref()
                        .is_some_and(|p| p.holds_process(now_ms()))
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

    /// `working`, for a task spawned from here: counted from now, not from when the task begins.
    pub(crate) fn working_owned(self: &Arc<Self>) -> WorkingOwned<R> {
        self.works.fetch_add(1, Ordering::AcqRel);
        WorkingOwned(self.clone())
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
        // The microphone is let go first.
        if let Some(v) = tauri::Manager::try_state::<Arc<crate::voice::Voice>>(&self.app) {
            v.shutdown();
        }
        self.mcp.stop();
        // Setups under way stop, with what they started.
        for (_, s) in self.setups.lock().drain() {
            s.task.abort();
        }
        self.kill_agent_processes();
        // Their tokens go with the app: their MCP configs too.
        self.mcp.clear_agent_configs();
        self.pty.kill_all();
        // The plugins are asked to `shutdown`, then their jobs end (a second at most).
        self.plugin_runtime.shutdown();
        self.save_now();
        // The usage statistics of the last minute, if they can leave in time.
        self.telemetry.flush_at_exit();
    }

    /// Every agent's process stops, with what it started.
    fn kill_agent_processes(&self) {
        for h in self.agents.read().values() {
            let mut rt = h.lock();
            if let Some(p) = rt.proc.take() {
                p.close_input();
                p.kill();
            }
        }
    }

    /// What is left of a core that is done with (a test's): its agents' processes stop (their
    /// interfaces too, which run with no window), and none starts after. The background tasks it
    /// spawned live on, on the application's runtime.
    #[cfg(test)]
    pub(crate) fn stop_for_good(&self) {
        self.quitting.store(true, Ordering::Release);
        self.kill_agent_processes();
        self.pty.kill_all();
    }

    pub async fn send_message(
        self: &Arc<Self>,
        id: &str,
        text: String,
        attachments: Vec<Attachment>,
    ) -> Result<()> {
        self.send_message_from(id, None, text, attachments).await
    }

    /// `send_message` with files joined by their path (`files`, ESC-95), as the window sends them:
    /// Claude is told where they are (`attachfiles::message_with_files`).
    pub async fn send_message_files(
        self: &Arc<Self>,
        id: &str,
        text: String,
        attachments: Vec<Attachment>,
        files: Vec<AttachedFile>,
    ) -> Result<()> {
        self.send(id, None, text, attachments, files, true, false, None)
            .await
    }

    /// A sentence said in a conversation (ESC-97): the agent is asked to end its answer with a passage to be read
    /// aloud (`voice::spoken`).
    pub async fn send_spoken(self: &Arc<Self>, id: &str, text: String) -> Result<()> {
        self.send(id, None, text, Vec::new(), Vec::new(), true, true, None)
            .await
    }

    /// `send_message` of a text someone else wrote than the user at the window: the author's name
    /// is `origin` (a caller of the MCP server). The agent reads it, and the conversation shows
    /// it, under « Message de <origin> : » (`from_origin`), so that it is never taken for the
    /// user's own words. The agent is named after the text alone.
    pub(crate) async fn send_message_from(
        self: &Arc<Self>,
        id: &str,
        origin: Option<&str>,
        text: String,
        attachments: Vec<Attachment>,
    ) -> Result<()> {
        self.send(id, origin, text, attachments, Vec::new(), true, false, None)
            .await
    }

    /// `send_message` of a text the app writes by itself (« continue » after the usage limit, the
    /// conflicts of the base to resolve): no task of the user's to name the agent after, so it
    /// never does.
    pub(crate) async fn send_by_app(self: &Arc<Self>, id: &str, text: String) -> Result<()> {
        self.send(id, None, text, Vec::new(), Vec::new(), false, false, None)
            .await
    }

    /// The handover file (`handover`), the first message of the agent on the harness it goes on
    /// on: its conversation shows it as such (`origin`: "handover"), folded. A ticket's agent
    /// that cannot get it blocks its ticket.
    pub(crate) async fn send_handover(self: &Arc<Self>, id: &str, text: String) -> Result<()> {
        let sent = self
            .send(
                id,
                None,
                text,
                Vec::new(),
                Vec::new(),
                false,
                // Escouade's own text, never said aloud.
                false,
                Some("handover"),
            )
            .await;
        if let (Err(e), Some(ticket)) = (&sent, self.doing_ticket_of(id)) {
            log::warn!("agent {id}: handover file to its ticket's agent not sent: {e:#}");
            let _ = self.edit_ticket(&ticket, |t| {
                t.blocked = Some(crate::tickets::error_reason(e));
                Ok(())
            });
            self.notify_ticket(&ticket, false);
        }
        sent
    }

    /// Sends the message to the agent; `names`: it may name the agent (`naming_text`); `kind`: the
    /// origin its item in the conversation shows (`push_user_from`). `files` are
    /// the files joined by their path: they are checked again (`checked_files`), the message ends
    /// with a line for each, and the agent may read the ones of the user's own from then on. The
    /// agent is named from the text alone, never from those lines. `spoken`: said in a conversation, it ends with the
    /// instruction of `voice::spoken`.
    #[allow(clippy::too_many_arguments)]
    async fn send(
        self: &Arc<Self>,
        id: &str,
        origin: Option<&str>,
        text: String,
        attachments: Vec<Attachment>,
        files: Vec<AttachedFile>,
        names: bool,
        spoken: bool,
        kind: Option<&str>,
    ) -> Result<()> {
        // Until its turn runs (its process started, the message delivered), it is under way.
        let _working = self.working();
        let delivered = match origin {
            Some(origin) => from_origin(i18n::ui(), origin, &text),
            None => text.clone(),
        };
        let delivered = if spoken {
            crate::voice::spoken::with_instruction(self.lang().claude, &delivered)
        } else {
            delivered
        };
        // What the agent's name is made from: nothing, for a message of the app's.
        let naming = if names { text.as_str() } else { "" };
        // In the Claude Code view: typed in its interface.
        let h = self.agent(id)?;
        // Its worktree went to another branch since it was last told (`agentbranch`): the agent
        // reads it first, the conversation shows the message alone.
        let note = self.branch_note(id);
        let with_note = |text: &str| match &note {
            Some((_, n)) => format!("{n}\n\n{text}"),
            None => text.to_string(),
        };
        if h.lock().meta.terminal {
            if !attachments.is_empty() || !files.is_empty() {
                bail!(tr!(
                    "Pas de pièce jointe en vue Claude Code : colle-la dans son terminal.",
                    "No attachments in the Claude Code view: paste them in its terminal."
                ));
            }
            // Typed in its interface as it is: the conversation read from its transcript shows it
            // whole, and the user, there, sees the line of the switch. The note is not kept for later,
            // where it could be stale.
            let message = delivered.clone();
            // Its entry in the transcript, read later, is told from the user's by its text.
            if !names {
                h.lock().unprompted = Some(message.clone());
            }
            // Off the runtime's threads: it waits for the paste to be written, then pauses.
            let (core, agent) = (self.clone(), id.to_string());
            let sent = tokio::task::spawn_blocking(move || core.send_to_tui(&agent, &message))
                .await
                .unwrap_or_else(|e| Err(anyhow!(e)));
            if sent.is_err() && !names {
                h.lock().unprompted = None;
            }
            if let (Ok(()), Some((mv, _))) = (&sent, &note) {
                self.branch_told(id, mv);
            }
            return sent;
        }
        // `/compact` typed by the user, to a harness that compacts by a request of its own: no
        // message to its model (ESC-205).
        if origin.is_none() && kind.is_none() && names && !spoken && attachments.is_empty() {
            if let Some(instructions) = engine::compact_command(&text) {
                let harness = engine::of(&h.lock().meta);
                match harness.compaction() {
                    engine::Compaction::Message => {}
                    engine::Compaction::InItsInterface(command) => {
                        let name = harness.name();
                        bail!(tr!(
                            "{name} ne compacte pas depuis Escouade : tape {command} dans sa vue terminal.",
                            "{name} doesn’t compact from Escouade: type {command} in its terminal view."
                        ));
                    }
                    _ => return self.compact(id, text.trim(), instructions).await,
                }
            }
        }
        // Refused before starting Claude: the composer keeps the message.
        let files = self.checked_files(id, files)?;
        let delivered = attachfiles::message_with_files(self.lang().claude, &delivered, &files);
        let content = engine::of(&h.lock().meta).message(&with_note(&delivered), &attachments)?;
        // Before its turn: its first question about one of them finds the right to ask it.
        self.grant_files(id, &files);
        // Its worktree, when it waited for this message (ESC-156): the agent named first, then its
        // branch and its folder under that name, and its setup started.
        let lazy = self
            .make_pending_worktree(id, names.then_some(text.as_str()))
            .await;
        // Its new worktree is set up first: Claude would work in it meanwhile (an install running
        // twice, files half written).
        self.wait_setup(id).await;
        // At rest on an account past « Changer de compte à »: this turn goes to another (ESC-148).
        self.switch_if_over(id).await;
        // Two attempts: the process may die between being started and receiving the message.
        for attempt in 0..2 {
            let proc = self.ensure_process(id).await?;
            if self.deliver(id, &proc, &delivered, naming, &content, &attachments, kind)? {
                if let Some((mv, _)) = &note {
                    self.branch_told(id, mv);
                }
                // Named before it ran: its session takes the name.
                if lazy && h.lock().meta.named {
                    self.retitle_soon(id, proc);
                }
                return Ok(());
            }
            log::warn!("message not delivered (attempt {attempt}): the process exited");
        }
        let name = engine::of(&h.lock().meta).name();
        bail!(tr!(
            "{name} s'est arrêté pendant l'envoi du message : voir le détail dans la conversation.",
            "{name} stopped while the message was being sent: see the details in the conversation."
        ))
    }

    /// Sends the message to `proc` if it is still the agent's live process, then records it.
    /// `text` is what the agent is sent, `naming` what its name is made from when it is not named
    /// yet (the same, unless the text is headed by its author's name): Haiku is asked once, at the
    /// first message that gives it a text to name the agent from (`naming_text`).
    #[allow(clippy::too_many_arguments)]
    fn deliver(
        self: &Arc<Self>,
        id: &str,
        proc: &Process,
        text: &str,
        naming: &str,
        content: &engine::Message,
        attachments: &[Attachment],
        kind: Option<&str>,
    ) -> Result<bool> {
        let h = self.agent(id)?;
        let mut fx = Effects::default();
        let (pid, name, view, name_from) = {
            let mut rt = h.lock();
            if !rt.proc.as_ref().is_some_and(|p| Arc::ptr_eq(p, proc)) || !proc.is_alive() {
                return Ok(false);
            }
            let uid = uuid::Uuid::new_v4().to_string();
            if proc.send_message(&uid, content).is_err() {
                return Ok(false);
            }
            // What was said in its interface (« Vue Cursor ») went with it.
            if rt.meta.relay.take().is_some() {
                fx.save = true;
            }
            let (images, files): (Vec<_>, Vec<_>) = attachments.iter().partition(|a| a.is_image());
            let files: Vec<String> = files.into_iter().map(|a| a.name.clone()).collect();
            rt.push_user_from(&uid, text, images.len() as u32, &files, kind, &mut fx);
            let name_from = rt.name_from(naming);
            (
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.view(),
                name_from,
            )
        };
        let engine = self
            .agent(id)
            .map_or(EngineId::CLAUDE, |h| h.lock().meta.engine);
        self.stats
            .record_prompt(&self.account_of(id), engine.as_str(), id, &pid);
        self.apply(id, &pid, &name, fx, Some(view));
        if let Some(naming) = name_from {
            let (c, id) = (self.clone(), id.to_string());
            tauri::async_runtime::spawn(async move { c.auto_name(&id, &naming).await });
        }
        Ok(true)
    }

    /// `/compact` (`typed`, its `instructions` after it) to an agent whose harness compacts by a
    /// request (`Engine::compaction`, ESC-205): the conversation shows the command, then what its
    /// harness says of the compaction. Refused, the composer keeping the text, during a turn (the
    /// compaction would cut it), before the conversation began (nothing to compact), and with an
    /// instruction its harness does not take.
    async fn compact(self: &Arc<Self>, id: &str, typed: &str, instructions: &str) -> Result<()> {
        let h = self.agent(id)?;
        let harness = engine::of(&h.lock().meta);
        let name = harness.name();
        {
            let rt = h.lock();
            if rt.meta.status.is_active() {
                bail!(tr!(
                    "Un tour est en cours : envoie /compact une fois qu’il est fini.",
                    "A turn is under way: send /compact once it’s over."
                ));
            }
            if rt.meta.session_id.is_none() {
                bail!(tr!(
                    "Rien à compacter : la conversation n’a pas commencé.",
                    "Nothing to compact: the conversation hasn’t started."
                ));
            }
        }
        if !instructions.is_empty() && harness.compaction() == engine::Compaction::Bare {
            bail!(tr!(
                "{name} ne prend pas d’instruction pour /compact : envoie /compact seul.",
                "{name} takes no instruction for /compact: send /compact alone."
            ));
        }
        let proc = self.ensure_process(id).await?;
        let mut fx = Effects::default();
        let (pid, agent_name, view) = {
            let mut rt = h.lock();
            rt.push_user_from(
                &uuid::Uuid::new_v4().to_string(),
                typed,
                0,
                &[],
                None,
                &mut fx,
            );
            (rt.meta.project_id.clone(), rt.meta.name.clone(), rt.view())
        };
        self.apply(id, &pid, &agent_name, fx, Some(view));
        // Copilot answers once it is over: the composer does not wait for it.
        let (core, id, instructions) = (self.clone(), id.to_string(), instructions.to_string());
        tauri::async_runtime::spawn(async move {
            let over = proc.compact(&instructions).await;
            core.compacted(&id, name, over);
        });
        Ok(())
    }

    /// The end of a compaction asked by `/compact` (`over`, as `EngineProcess::compact` says).
    fn compacted(self: &Arc<Self>, id: &str, name: &str, over: Result<bool>) {
        if matches!(over, Ok(false)) {
            // A turn of its harness's, which ends it.
            return;
        }
        let Ok(h) = self.agent(id) else {
            return;
        };
        let mut fx = Effects::default();
        let (pid, agent_name, view) = {
            let mut rt = h.lock();
            if let Err(e) = &over {
                log::warn!("agent {id}: compaction refused: {e:#}");
                rt.notice(
                    "error",
                    tr!(
                        "{name} n’a pas compacté le contexte : {e}",
                        "{name} didn’t compact the context: {e}"
                    ),
                    &mut fx,
                );
            }
            // Over (or never begun): the agent is at rest again, as after a turn.
            if rt.meta.status.is_active() {
                rt.set_status(AgentStatus::Done, &mut fx);
            }
            (rt.meta.project_id.clone(), rt.meta.name.clone(), rt.view())
        };
        self.apply(id, &pid, &agent_name, fx, Some(view));
    }

    pub async fn interrupt(self: &Arc<Self>, id: &str) -> Result<()> {
        if self.interrupt_tui(id)? {
            // An interface that tells nothing of a turn it stopped (Gemini CLI): over here.
            let _ = self.with_agent(id, |rt, fx| {
                if let Ok((view, _)) = crate::tui::view_of(&rt.meta) {
                    view.interrupted(rt, fx);
                }
                Ok(())
            });
            return Ok(());
        }
        let h = self.agent(id)?;
        let proc = {
            let mut rt = h.lock();
            // Only a running turn can be interrupted; a stale flag would hide the next turn's end.
            let active = rt.meta.status.is_active();
            rt.interrupted = active;
            rt.proc.clone().filter(|_| active)
        };
        if let Some(p) = proc {
            if let Err(e) = p.interrupt(false, Duration::from_secs(15)).await {
                h.lock().interrupted = false;
                return Err(e);
            }
        }
        Ok(())
    }

    /// A message to a subagent of the agent `id` (`call`, the call that launched it). Claude Code
    /// lets no host write to a subagent (`send_task_message` is refused, docs/PROTOCOL.md, « Lui
    /// écrire »): the message goes to the agent, which is asked to pass it on with its
    /// `SendMessage` tool to the subagent's task, as Claude Code tells it it can. Refused for a
    /// subagent that does not run any more, whose task no frame told, or that works in the
    /// foreground (`subagent_target`).
    pub async fn message_subagent(
        self: &Arc<Self>,
        id: &str,
        call: &str,
        text: &str,
    ) -> Result<()> {
        self.message_subagent_from(id, call, text, None).await
    }

    /// `message_subagent`, written by someone else than the user (a client of the MCP server):
    /// with `origin`, the words the agent is to pass on are headed by the author's name (« Message
    /// de <origin> : … »), as any message from the server is, and never given as the user's own. The
    /// refusals are the same, and come before the name is put on an empty text.
    pub async fn message_subagent_from(
        self: &Arc<Self>,
        id: &str,
        call: &str,
        text: &str,
        origin: Option<&str>,
    ) -> Result<()> {
        let text = text.trim();
        if text.is_empty() {
            bail!(tr!("Le message est vide.", "The message is empty."));
        }
        let (title, task) = self.subagent_target(id, call)?;
        let words = match origin {
            Some(origin) => from_origin(i18n::claude(), origin, text),
            None => text.to_string(),
        };
        let harness = self
            .agents
            .read()
            .get(id)
            .map(|h| engine::of(&h.lock().meta))
            .unwrap_or_else(engine::default);
        let message = harness
            .subagent_relay(i18n::claude(), &title, &task, &words)
            .ok_or_else(|| {
                anyhow!(tr!(
                    "Son harness ne permet pas d’écrire à un sous-agent.",
                    "Its harness lets no one write to a subagent."
                ))
            })?;
        // Not the user's task: it never names the agent.
        self.send_by_app(id, message).await
    }

    /// The title and the task of the subagent `call` of the agent `id`, if a message can be written
    /// to it: the guards of `message_subagent`, without sending anything (the one place they are
    /// said, for whoever wants to know before it goes). Refused, in this order, for an archived
    /// agent, the Claude Code view (a subagent is written to from its interface), a subagent that
    /// does not run any more or that the plan does not have, one that works in the foreground (the
    /// agent waits for it: the message would only reach it once it ended) and one whose task no frame
    /// told. What waiting would not change comes first, so that a caller is not told to try again
    /// later for something that will be refused anyway.
    pub(crate) fn subagent_target(&self, id: &str, call: &str) -> Result<(String, String)> {
        let h = self.agent(id)?;
        let rt = h.lock();
        if rt.meta.archived {
            bail!(tr!("L’agent est archivé.", "The agent is archived."));
        }
        if rt.meta.terminal {
            bail!(tr!(
                "En vue Claude Code, on écrit à un sous-agent depuis l’interface de Claude Code.",
                "In the Claude Code view, a subagent is written to from Claude Code’s interface."
            ));
        }
        let row = rt
            .meta
            .plan
            .as_ref()
            .and_then(|p| p.agents.iter().find(|a| a.id == call))
            .filter(|a| a.status == crate::plan::RunStatus::Running);
        let Some(row) = row else {
            bail!(subagent_over());
        };
        // In the foreground, the agent waits for it: the message would only reach it once it ended.
        if !row.background {
            bail!(tr!(
                "Ce sous-agent travaille au premier plan : l’agent ne lui transmettrait le message qu’une fois fini.",
                "This subagent works in the foreground: the agent would only pass the message on once it ended."
            ));
        }
        let Some(task) = row.task_id.clone() else {
            bail!(tr!(
                "Claude Code n’a pas donné la tâche de ce sous-agent : on ne peut pas lui écrire.",
                "Claude Code has not told this subagent’s task: it cannot be written to."
            ));
        };
        Ok((row.title.clone(), task))
    }

    /// « Arrêter » on a subagent of the agent `id`, `call` being the call that launched it: Claude
    /// Code's `stop_task` on its task (docs/PROTOCOL.md, « Arrêter un sous-agent »). Its end comes
    /// with the frames that follow (the task killed, its notification « stopped »), as any end.
    /// Refused for a subagent that does not run, or whose task no frame told (nothing to name), or
    /// with no process to ask (in the Claude Code view, its interface does it).
    pub async fn stop_subagent(self: &Arc<Self>, id: &str, call: &str) -> Result<()> {
        let h = self.agent(id)?;
        let (proc, task) = {
            let rt = h.lock();
            let row = rt
                .meta
                .plan
                .as_ref()
                .and_then(|p| p.agents.iter().find(|a| a.id == call))
                .filter(|a| a.status == crate::plan::RunStatus::Running);
            let Some(row) = row else {
                bail!(subagent_over());
            };
            // In the foreground the agent waits for it, and its own « Arrêter » stops it with its
            // turn; stopping one alone there was never tried on a real stream (docs/PROTOCOL.md).
            if !row.background {
                bail!(tr!(
                    "Ce sous-agent travaille au premier plan : l’agent l’attend, et « Arrêter » de l’agent l’arrête avec son tour.",
                    "This subagent works in the foreground: the agent waits for it, and the agent’s « Stop » stops it with its turn."
                ));
            }
            let Some(task) = row.task_id.clone() else {
                bail!(tr!(
                    "Claude Code n’a pas encore donné la tâche de ce sous-agent : il ne peut pas être arrêté seul.",
                    "Claude Code has not told this subagent’s task yet: it cannot be stopped on its own."
                ));
            };
            if rt.tui.is_some() {
                bail!(tr!(
                    "En vue Claude Code, un sous-agent s’arrête depuis son interface.",
                    "In the Claude Code view, a subagent is stopped from its interface."
                ));
            }
            (rt.proc.clone(), task)
        };
        let Some(p) = proc else {
            bail!(tr!(
                "Le processus de l’agent ne tourne pas.",
                "The agent’s process is not running."
            ));
        };
        p.stop_task(&task).await.context(tr!(
            "Claude Code n’a pas arrêté le sous-agent",
            "Claude Code did not stop the subagent"
        ))?;
        Ok(())
    }

    /// « Arrêter » on a task Claude Code runs in the background for the agent `id`: a command, a
    /// monitor, a remote agent, a teammate (a `BgTask` of its plan) or a workflow, `task` being its
    /// id. Claude Code's `stop_task` on it (docs/PROTOCOL.md): its end comes with the frames that
    /// follow (the task killed, its notification « stopped »), as any end, and the plan is not
    /// touched here. Refused for a task the plan does not have, one that does not run, or, with no
    /// process to ask, or in the Claude Code view (its interface does it).
    pub async fn stop_bg_task(self: &Arc<Self>, id: &str, task: &str) -> Result<()> {
        use crate::plan::RunStatus;
        let h = self.agent(id)?;
        let proc = {
            let rt = h.lock();
            let status = rt.meta.plan.as_ref().and_then(|p| {
                let row = p.bg_tasks.iter().find(|t| t.id == task).map(|t| t.status);
                row.or_else(|| p.workflows.iter().find(|w| w.id == task).map(|w| w.status))
            });
            match status {
                None => bail!(unknown_bg_task()),
                Some(RunStatus::Running) => {}
                Some(_) => bail!(tr!(
                    "Cette tâche de fond ne tourne plus.",
                    "This background task is no longer running."
                )),
            }
            if rt.tui.is_some() {
                bail!(tr!(
                    "En vue Claude Code, une tâche de fond s’arrête depuis son interface.",
                    "In the Claude Code view, a background task is stopped from its interface."
                ));
            }
            rt.proc.clone()
        };
        let Some(p) = proc else {
            bail!(tr!(
                "Le processus de l’agent ne tourne pas.",
                "The agent’s process is not running."
            ));
        };
        p.stop_task(task).await.context(tr!(
            "Claude Code n’a pas arrêté la tâche",
            "Claude Code did not stop the task"
        ))?;
        Ok(())
    }

    /// The end of the output of the command or monitor `task` of the agent `id`, running or over.
    /// Two sources, in the order that serves the window best (docs/PROTOCOL.md):
    ///
    /// - a task that runs: its process first, asked without a turn of the model
    ///   (`get_task_output`: 8 KiB, the end), else, or when Claude Code does not answer, the end of
    ///   the file Claude Code writes the whole output to (`<task>.output`, 8 KiB too);
    /// - a task that is over (done, failed, stopped, interrupted): the file first, as far back as
    ///   256 KiB (`MAX_FINISHED_OUTPUT`: the first error of a failed build is not in the last 8 KiB),
    ///   else, if it is absent, unreadable or refused and the process lives, the process (8 KiB).
    ///
    /// Never any other file than the task's own (`taskout`), and none of another machine: the path
    /// as the frames said it must pass `safe_origin` (a share, a verbatim path or a device are not
    /// read, nothing of the system is touched for them), before it is converted for a `claude`
    /// under WSL. No lock of the agent is held while the process answers or the file is read. A
    /// task that is neither a command nor a monitor has no output to read (`source` `none`); one
    /// the plan does not have is refused.
    pub async fn bg_task_output(self: &Arc<Self>, id: &str, task: &str) -> Result<TaskOutput> {
        use crate::plan::{BgKind, RunStatus};
        use crate::taskout::{safe_origin, MAX_FINISHED_OUTPUT, MAX_OUTPUT};
        let h = self.agent(id)?;
        let (proc, file, account, over) = {
            let rt = h.lock();
            let row = rt
                .meta
                .plan
                .as_ref()
                .and_then(|p| p.bg_tasks.iter().find(|t| t.id == task));
            let Some(row) = row else {
                bail!(unknown_bg_task());
            };
            if !matches!(row.kind, BgKind::Command | BgKind::Monitor) {
                return Ok(TaskOutput::none());
            }
            // The Claude Code view has no process to ask: its files are all there is.
            let proc = rt.proc.clone().filter(|p| rt.tui.is_none() && p.is_alive());
            // The path as the frames said it, asked before anything is done with it.
            let file = row.output_file.clone().filter(|f| safe_origin(f));
            (
                proc,
                file,
                rt.meta.account.clone(),
                row.status != RunStatus::Running,
            )
        };
        if !over {
            if let Some(out) = Self::output_of_process(proc.as_deref(), task).await {
                return Ok(out);
            }
            return self.output_of_file(file, task, &account, MAX_OUTPUT).await;
        }
        let from_file = self
            .output_of_file(file, task, &account, MAX_FINISHED_OUTPUT)
            .await?;
        if from_file.source != "none" {
            return Ok(from_file);
        }
        Ok(Self::output_of_process(proc.as_deref(), task)
            .await
            .unwrap_or(from_file))
    }

    /// What the process of the agent answers to `get_task_output` for `task`: none without a
    /// process, when Claude Code does not know the task (any more, or never) or answers something
    /// else.
    async fn output_of_process(proc: Option<&dyn EngineProcess>, task: &str) -> Option<TaskOutput> {
        let asked = proc?.task_output(task).await;
        match asked {
            Ok(answer) => TaskOutput::from_process(&answer),
            Err(e) => {
                log::debug!("get_task_output {task}: {e:#}");
                None
            }
        }
    }

    /// The end of `file`, `max` bytes at most, off the runtime's threads; `source` `none` when
    /// there is no file or it is not the task's.
    async fn output_of_file(
        &self,
        file: Option<String>,
        task: &str,
        account: &str,
        max: usize,
    ) -> Result<TaskOutput> {
        let Some(file) = file else {
            return Ok(TaskOutput::none());
        };
        let settings = self.settings.read().clone();
        let (task, account) = (task.to_string(), account.to_string());
        tauri::async_runtime::spawn_blocking(move || {
            // A `claude` of WSL tells its files by their Linux path: our own conversion of a path
            // already accepted, to a share of ours (`\\wsl.localhost\…`), which `from_file` is
            // given as it is.
            let wsl = (cfg!(windows) && file.starts_with('/'))
                .then(|| {
                    let account = accounts::get(&settings, &account);
                    accounts::program(&account, &settings).and_then(|p| p.wsl().cloned())
                })
                .flatten();
            let path = match wsl {
                Some(wsl) => wsl.windows_path(&file),
                None => PathBuf::from(&file),
            };
            TaskOutput::from_file(&path, &task, max)
        })
        .await
        .map_err(|e| anyhow!("{e}"))
    }

    /// The conversation of the subagent `call` launched, in the Claude Code view, where the stream
    /// does not tell it: read from the transcript of its own that Claude Code writes beside the
    /// agent's (`agent::sidechain`). `since` is the cursor of the read before: None back when its
    /// files are as that read left them (the window asks every 2 s). No items and no cursor while
    /// there is no file. Refused for an agent that is not in that view (its subagents' threads are
    /// in its conversation then).
    ///
    /// Its files may say how the subagent ended, which the view's frames never do (stopped in the
    /// interface, an API error: `ended`); its row of the plan, if it still runs, ends so, and the
    /// window is sent the agent. The model its messages tell goes on its row too (the view's
    /// frames carry none of a subagent's). That is what opening its view in the window does, and
    /// the window's alone: whoever only reads it (`subagent_report`) uses `read_sidechain`, which
    /// changes nothing.
    pub fn subagent_transcript(
        self: &Arc<Self>,
        id: &str,
        call: &str,
        since: Option<crate::agent::sidechain::Cursor>,
    ) -> Result<Option<crate::agent::sidechain::SidechainRead>> {
        let read = self.read_sidechain(id, call, since)?;
        let told = read
            .as_ref()
            .filter(|r| r.model.is_some() || r.ended.is_some());
        if let (Some(r), Ok(h)) = (told, self.agent(id)) {
            // What its files tell goes on its row at once, for the window to be sent the agent once.
            let mut fx = Effects::default();
            let (pid, name, view) = {
                let mut rt = h.lock();
                if let Some(model) = r.model.as_deref() {
                    rt.subagent_model(call, model, &mut fx);
                }
                if let Some(how) = r.ended {
                    rt.end_subagent(call, how, &mut fx);
                }
                (
                    rt.meta.project_id.clone(),
                    rt.meta.name.clone(),
                    fx.agent_changed.then(|| rt.view()),
                )
            };
            self.apply(id, &pid, &name, fx, view);
        }
        Ok(read)
    }

    /// The transcript of the subagent `call` as `subagent_transcript` reads it, and nothing done
    /// with what its files say: the plan and the window are left as they are (the path found is
    /// kept, which nothing outside the core sees).
    fn read_sidechain(
        &self,
        id: &str,
        call: &str,
        since: Option<crate::agent::sidechain::Cursor>,
    ) -> Result<Option<crate::agent::sidechain::SidechainRead>> {
        use crate::agent::sidechain;
        let h = self.agent(id)?;
        // What is read is known under the agent's lock; the files are read out of it.
        let (read, session, account, task) = {
            let rt = h.lock();
            if !rt.meta.terminal {
                bail!(tr!(
                    "Hors vue Claude Code, la conversation d’un sous-agent est dans celle de l’agent.",
                    "Outside the Claude Code view, a subagent’s conversation is in the agent’s."
                ));
            }
            let task = rt
                .meta
                .plan
                .as_ref()
                .and_then(|p| p.agents.iter().find(|a| a.id == call))
                .and_then(|a| a.task_id.clone());
            (
                rt.meta
                    .transcript_read
                    .as_ref()
                    .map(|(file, _)| PathBuf::from(file)),
                rt.meta.session_id.clone(),
                rt.meta.account.clone(),
                task,
            )
        };
        let file = self.find_sidechain(id, call, task.as_deref(), || {
            self.agent_transcript(read, session, &account)
        });
        let Some(file) = file else {
            return Ok(Some(sidechain::SidechainRead {
                items: Vec::new(),
                cursor: None,
                ended: None,
                model: None,
            }));
        };
        sidechain::read_changed(&file, call, since).context(tr!(
            "Le transcript du sous-agent n’a pas pu être lu",
            "The subagent’s transcript could not be read"
        ))
    }

    /// What the subagent `call` of the agent `id` handed back, as the window's view of it shows
    /// it (`subagentInfo`, `SubagentView`): the report its call keeps (`report`, 16 000 bytes at
    /// most), else the result of a call that ended well, in the foreground (a launch in the
    /// background answers « Async agent launched… », which is no report), else, in the Claude Code
    /// view, where the stream has neither, the message of the last `SubagentHandback` of its
    /// transcript that says something. Trimmed. None while it has handed nothing back, or when
    /// `call` is no subagent's. The transcript is read like the window's view reads it, off the
    /// runtime's threads and with no lock of the agent held, but only read: a subagent its files
    /// show ended is not ended by it (`subagent_transcript` does that, for the window), reading
    /// a report changes nothing.
    pub async fn subagent_report(self: &Arc<Self>, id: &str, call: &str) -> Result<Option<String>> {
        let h = self.agent(id)?;
        let (kept, terminal) = {
            let mut rt = h.lock();
            let terminal = rt.meta.terminal;
            let item = rt.conv.get(call).filter(|i| {
                i["kind"] == "tool" && matches!(i["name"].as_str(), Some("Agent" | "Task"))
            });
            let said = |v: &Value| {
                v.as_str()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            };
            let report = item.and_then(|i| said(&i["report"]));
            let result = item
                .filter(|i| i["status"] == "ok")
                .and_then(|i| said(&i["result"]["text"]))
                .filter(|t| !t.starts_with("Async agent launched"));
            (report.or(result), terminal)
        };
        if kept.is_some() || !terminal {
            return Ok(kept);
        }
        let (core, id, call) = (self.clone(), id.to_string(), call.to_string());
        let read =
            tauri::async_runtime::spawn_blocking(move || core.read_sidechain(&id, &call, None))
                .await
                .map_err(|e| anyhow!("{e}"))??;
        Ok(read.and_then(|r| handed_back(&r.items)))
    }

    /// The transcript of the agent in the Claude Code view, beside which its subagents' are: the
    /// one Escouade reads (`read`), else its session's in the folder of its account.
    fn agent_transcript(
        &self,
        read: Option<PathBuf>,
        session: Option<String>,
        account: &str,
    ) -> Option<PathBuf> {
        read.or_else(|| {
            let settings = self.settings.read().clone();
            let account = accounts::get(&settings, account);
            crate::tui::find_transcript(&accounts::config_dir(&account, &settings), &session?)
        })
    }

    /// The transcript of the subagent `call` of the agent `id`: the path found before, while it is
    /// still what was found, else looked for beside the agent's own transcript (`main`, asked for
    /// only then) and kept. A path that is gone is forgotten.
    fn find_sidechain(
        &self,
        id: &str,
        call: &str,
        task: Option<&str>,
        main: impl FnOnce() -> Option<PathBuf>,
    ) -> Option<PathBuf> {
        use crate::agent::sidechain;
        let kept = self.sidechains.lock().get(id, call);
        if let Some(file) = kept {
            if sidechain::still_found(&file) {
                return Some(file);
            }
            self.sidechains.lock().forget(id, call);
        }
        let file = sidechain::sidechain_file(&main()?, call, task)?;
        self.sidechains.lock().remember(id, call, file.clone());
        Some(file)
    }

    /// The paths of the transcripts of the agent `id`'s subagents are not kept any more (it is
    /// deleted).
    pub(crate) fn forget_sidechains(&self, id: &str) {
        self.sidechains.lock().forget_agent(id);
    }

    /// The subagents of the agent `id` that `ends` names (the call that launched each, and how it
    /// ended) ended as their files show: the row of each that runs still ends so (the plan never
    /// opens an ended one again), with `run` written on its call, and the window is sent the agent
    /// when something changed.
    fn end_subagents(
        self: &Arc<Self>,
        id: &str,
        ends: &[(String, crate::agent::sidechain::Ended)],
    ) {
        let Ok(h) = self.agent(id) else { return };
        let mut fx = Effects::default();
        let (pid, name, view) = {
            let mut rt = h.lock();
            for (call, how) in ends {
                rt.end_subagent(call, *how, &mut fx);
            }
            (
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                fx.agent_changed.then(|| rt.view()),
            )
        };
        self.apply(id, &pid, &name, fx, view);
    }

    /// The end of a turn of the agent's Claude Code interface (its `Stop` hook): the subagents in
    /// the background that run still, in its plan, are looked at once. One the user stopped in the
    /// interface is told only by its meta file (`stoppedByUser`): no frame, and the agent's
    /// transcript does not always replay its notification. Their rows end, as `subagent_transcript`
    /// ends one, for the sidebar not to say « En cours » while its view is not open. A small file
    /// for each row, `MAX_WATCHED` at most, read out of the agent's lock.
    pub(crate) fn end_stopped_subagents(self: &Arc<Self>, id: &str) {
        use crate::agent::sidechain;
        /// The most rows looked at.
        const MAX_WATCHED: usize = 30;
        let Ok(h) = self.agent(id) else { return };
        let (rows, read, session, account) = {
            let rt = h.lock();
            let rows: Vec<(String, Option<String>)> = rt
                .meta
                .plan
                .iter()
                .flat_map(|p| p.agents.iter())
                .filter(|a| a.status == crate::plan::RunStatus::Running && a.background)
                .take(MAX_WATCHED)
                .map(|a| (a.id.clone(), a.task_id.clone()))
                .collect();
            (
                rows,
                rt.meta
                    .transcript_read
                    .as_ref()
                    .map(|(file, _)| PathBuf::from(file)),
                rt.meta.session_id.clone(),
                rt.meta.account.clone(),
            )
        };
        if rows.is_empty() {
            return;
        }
        let main = std::cell::OnceCell::new();
        let ends: Vec<_> = rows
            .into_iter()
            .filter_map(|(call, task)| {
                let file = self.find_sidechain(id, &call, task.as_deref(), || {
                    main.get_or_init(|| {
                        self.agent_transcript(read.clone(), session.clone(), &account)
                    })
                    .clone()
                })?;
                let meta = sidechain::read_meta(&sidechain::meta_file(&file))?;
                Some((call, sidechain::how_it_ended(Some(&meta), "")?))
            })
            .collect();
        if !ends.is_empty() {
            self.end_subagents(id, &ends);
        }
    }

    pub(crate) fn with_agent<T>(
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

    /// The user's answer to an agent's question, from the window.
    pub fn answer_question(
        self: &Arc<Self>,
        id: &str,
        request_id: &str,
        answers: Value,
    ) -> Result<()> {
        self.answer_question_by(id, request_id, answers, None)
    }

    /// An answer to an agent's question given by `by`, a client of the MCP server (named as the
    /// activity log names it): the same as the window's, and its card says who answered.
    pub fn answer_question_by(
        self: &Arc<Self>,
        id: &str,
        request_id: &str,
        answers: Value,
        by: Option<&str>,
    ) -> Result<()> {
        self.with_agent(id, |rt, fx| {
            rt.answer_question_by(request_id, answers, by, fx)
        })
    }

    /// The user's answer to an agent's permission, from the window.
    pub fn answer_permission(
        self: &Arc<Self>,
        id: &str,
        request_id: &str,
        decision: &str,
        message: Option<String>,
    ) -> Result<()> {
        self.answer_permission_by(id, request_id, decision, message, None)
    }

    /// An answer to an agent's permission given by `by`, a client of the MCP server: the same as
    /// the window's, its card says who answered, and a refusal tells Claude who refused.
    pub fn answer_permission_by(
        self: &Arc<Self>,
        id: &str,
        request_id: &str,
        decision: &str,
        message: Option<String>,
        by: Option<&str>,
    ) -> Result<()> {
        self.with_agent(id, |rt, fx| {
            let auto = self.takes_auto(rt.meta.engine, &rt.meta.account, &rt.meta.model);
            rt.answer_permission_in(request_id, decision, message, by, auto, fx)
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
        // A discussion of the Docs tab stays in a mode that asks before a write (ESC-261).
        if let Some(m) = mode.as_deref() {
            if h.lock().meta.discussion.is_some() && !crate::discussion::MODES.contains(&m) {
                bail!(tr!(
                    "Une discussion ne modifie jamais le code : elle reste en mode « Demander » ou « Plan ».",
                    "A discussion never changes the code: it stays in “Ask” or “Plan” mode."
                ));
            }
        }
        let proc = {
            let mut rt = h.lock();
            if let Some(m) = &model {
                rt.meta.model = m.clone();
            }
            if let Some(e) = &effort {
                rt.meta.effort = e.clone();
            }
            if let Some(m) = &mode {
                rt.meta.set_mode(m);
            }
            rt.proc.clone()
        };
        self.emit_agent(&h);
        self.request_save();
        if let Some(p) = proc {
            if let Some(m) = model {
                p.set_model(&m).await?;
            }
            let (current_model, engine) = {
                let rt = h.lock();
                (rt.meta.model.clone(), engine::of(&rt.meta))
            };
            let account = h.lock().meta.account.clone();
            if let Some(e) =
                effort.filter(|_| self.takes_effort(engine.id(), &account, &current_model))
            {
                p.set_effort(&e).await?;
            }
            if let Some(m) = mode {
                p.set_mode(&m).await?;
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
        // The agents of a closed project hold theirs for when it comes back (read after the
        // agents: one being closed is in the one or the other).
        taken.extend(self.closed_ports());
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

    /// A new agent in a worktree on the existing branch `branch` (a local one, or a remote one
    /// such as `origin/feat`), selected in the sidebar. Refused for the branch of the project's
    /// folder and for one another worktree has (`IN_WORKTREE` for an agent's). The command, which
    /// also names a harness, goes through `create_agent_with`.
    #[cfg(test)]
    pub async fn create_agent_on_branch(
        self: &Arc<Self>,
        project_id: &str,
        branch: &str,
        model: Option<String>,
    ) -> Result<AgentView> {
        self.create_agent_with(
            project_id,
            AgentOptions {
                model,
                select: true,
                branch: Some(branch.to_string()),
                ..Default::default()
            },
        )
        .await
    }

    /// A new agent in a worktree on a new branch `branch`, made from where a new agent starts.
    #[cfg(test)]
    pub async fn create_agent_on_new_branch(
        self: &Arc<Self>,
        project_id: &str,
        branch: &str,
        model: Option<String>,
    ) -> Result<AgentView> {
        self.create_agent_with(
            project_id,
            AgentOptions {
                model,
                select: true,
                branch: Some(branch.to_string()),
                new_branch: true,
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
            (Some(c), _) => copy_name(i18n::ui(), &c.name, &existing),
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
        let engine = o.engine.unwrap_or_else(|| project.engine());
        // None is made on a harness absent (INT-3): its agents are kept, no new one.
        if !crate::engine::written(engine) {
            bail!(engine::absent_line(engine, crate::i18n::ui()));
        }
        // Whatever the agent is (ticket, discussion, planner…), a spec in Escouade is the one to follow.
        let follow = self.docs_follow(&project, self.lang().claude).await;
        let mut meta = AgentMeta {
            id: new_id(),
            project_id: project_id.to_string(),
            name: name.clone(),
            named: o.name.is_some() || o.copy_of.is_some(),
            engine,
            fork_of: o.copy_of.as_ref().and_then(|c| c.session_id.clone()),
            fork_at: o
                .copy_of
                .as_ref()
                .filter(|c| c.session_id.is_some())
                .and_then(|c| c.entry.clone()),
            // Set below, once its account is known: unless they say theirs, its account's.
            model: String::new(),
            cwd: project.path.clone(),
            created_at: now_ms(),
            last_activity: now_ms(),
            // Never a discussion (ESC-289): it stays in its conversation, where Escouade refuses its
            // writes; its first message is sent at its making.
            terminal: settings.terminal_view
                && o.ticket_id.is_none()
                && o.discussion.is_none()
                && crate::engine::by_id(engine).terminal().is_some(),
            attached: settings.terminal_view
                && o.ticket_id.is_none()
                && o.discussion.is_none()
                && crate::engine::by_id(engine).attaches(),
            remote_off: o.remote_off || o.copy_of.as_ref().is_some_and(|c| c.remote_off),
            ticket_id: o.ticket_id.clone(),
            // The code index's rule, for an agent that has no ticket's protocol (which carries it),
            // then what every agent is told of the shared memory.
            append_prompt: with_memory(
                self.lang().claude,
                with_follow(
                    o.append_prompt.or_else(|| {
                        (self.code_index_on(&project) && o.ticket_id.is_none())
                            .then(|| crate::codeindex::hint::rule(self.lang().claude))
                    }),
                    follow.as_deref(),
                ),
            ),
            port_base: o.port_base,
            discussion: o.discussion.clone(),
            ..Default::default()
        };
        // The one asked for if the settings know it as an account of the agent's harness
        // (Principal otherwise), else the project's, else the first usable one of the harness.
        meta.account = match o
            .account
            .as_deref()
            .and_then(|id| accounts::find(&settings, id))
        {
            Some(a) if a.engine == meta.engine => a.id.clone(),
            _ => self.account_for_new(project_id, meta.engine)?,
        };
        // Its model, effort and mode: those asked for, else its account's (ESC-128).
        let defaults = accounts::defaults(accounts::find(&settings, &meta.account), engine);
        meta.model = self.model_for(engine, o.model.unwrap_or(defaults.model));
        meta.effort = o.effort.unwrap_or(defaults.effort);
        meta.mode = o.mode.unwrap_or_else(|| defaults.mode.clone());
        // Started in Plan: its plan approved, it goes on in its mode of departure, the ticket's or
        // the board's (`o.mode`), else its account's, else the app's (ESC-277).
        if meta.mode == "plan" {
            meta.plan_from = Some(defaults.mode).filter(|m| m != "plan");
        }
        let conversations = self.data.conversations();
        // A copy's conversation is written first: when it cannot be, nothing was made for it.
        if let Some(c) = &o.copy_of {
            conv::write_log(&conversations, &meta.id, &c.items)
                .with_context(|| tr!("conversation non copiée", "conversation not copied"))?;
        }
        // The original's worktree, for a copy of an agent that has one.
        let copied_from = o.copy_of.as_ref().and_then(|c| c.worktree.as_ref());
        // The folders of the app's agents' worktrees: a new one never takes one of them.
        let held = self.held_worktrees();
        // Why it does not start from the branch it was to (`start_point`).
        let mut start_notice = None;
        // A branch to start from asks for a worktree.
        let isolated = o.isolated.unwrap_or(project.worktree_per_agent) || o.base.is_some();
        // Of several repositories (ESC-169): a folder of its own, with a worktree of each.
        let multi: Option<Result<Worktree>> = if project.is_multi() {
            use crate::multiwt::Spec;
            let spec = match (&o.branch, &o.worktree, &o.copy_of) {
                (Some(branch), _, _) => Some(Ok(Spec {
                    name: branch.clone(),
                    branch: Some(branch.clone()),
                    existing: !o.new_branch,
                    strict: o.new_branch,
                    over: o.base.clone(),
                    ..Default::default()
                })),
                (None, Some((branch, base)), _) => Some(Ok(Spec {
                    name: branch.clone(),
                    branch: Some(branch.clone()),
                    over: Some(base.clone()),
                    ..Default::default()
                })),
                (None, None, Some(c)) => match copied_from {
                    Some(wt) => {
                        let branch = copy_branch(&project.agent_branch_prefix, &c.name, &name);
                        Some(Self::copy_spec(wt, &name, &branch).await)
                    }
                    None => None,
                },
                (None, None, None)
                    if isolated
                        && project.worktree_at_first_message
                        && o.name.is_none()
                        && !meta.terminal =>
                {
                    meta.worktree_pending = true;
                    meta.worktree_from = o.base.clone();
                    None
                }
                (None, None, None) if isolated => Some(Ok(Spec {
                    name: name.clone(),
                    over: o.base.clone(),
                    ..Default::default()
                })),
                (None, None, None) => None,
            };
            match spec {
                Some(Ok(spec)) => Some(
                    self.multi_worktree(&project, &held, &spec, &mut start_notice)
                        .await,
                ),
                Some(Err(e)) => Some(Err(e)),
                None => None,
            }
        } else {
            None
        };
        let made = match (&o.branch, &o.worktree, &o.copy_of) {
            // Made above, of each repository.
            _ if project.is_multi() => match multi {
                Some(Ok(wt)) => {
                    meta.cwd = wt.path.clone();
                    if !self.git_watches(&project, &wt.path) {
                        self.watch_project(&project);
                    }
                    meta.worktree = Some(wt);
                    None
                }
                Some(Err(e)) => Some(Err(e)),
                None => None,
            },
            // A new branch, named by the user, then the agent on it as on one that exists.
            (Some(branch), _, _) if o.new_branch => Some(
                self.worktree_on_new_branch(
                    &project,
                    &held,
                    branch,
                    o.base.as_deref(),
                    &mut start_notice,
                )
                .await,
            ),
            // A branch that exists: the agent works on it, whatever else is asked.
            (Some(branch), _, _) => Some(
                self.worktree_on_branch(&project, &held, branch, o.base.as_deref())
                    .await,
            ),
            (None, Some((branch, base)), _) => Some(
                git::worktree_add_on(
                    &project.path,
                    &project.worktrees_root(),
                    &held,
                    branch,
                    base,
                )
                .await
                .map(|(path, branch)| (path, branch, base.clone())),
            ),
            // A copy works on its original's code: in a worktree of its own, or in the same folder.
            (None, None, Some(c)) => match copied_from {
                Some(wt) => {
                    let branch = copy_branch(&project.agent_branch_prefix, &c.name, &name);
                    Some(copy_worktree(&project, &held, wt, &branch).await)
                }
                None => None,
            },
            // Its worktree waits for its first message, which names it (ESC-156): unless the user
            // named it, or its harness's interface runs from the start (the Claude Code view).
            // Its base kept until then (ESC-160).
            (None, None, None)
                if isolated
                    && project.worktree_at_first_message
                    && o.name.is_none()
                    && !meta.terminal =>
            {
                meta.worktree_pending = true;
                meta.worktree_from = o.base.clone();
                None
            }
            (None, None, None) if isolated => Some(
                self.agent_worktree(&project, &held, &name, o.base.as_deref(), &mut start_notice)
                    .await,
            ),
            (None, None, None) => None,
        };
        let mut warning = None;
        match made {
            Some(Ok((path, branch, base))) => {
                if let Err(e) =
                    testlaunch::copy_worktree_files(&project.path, &path, &project.worktree_copy)
                        .await
                {
                    warning = Some(files_not_copied(&e));
                }
                if let Some(from) = copied_from {
                    // Its conversation names the original's folder everywhere (its files' absolute
                    // paths): its Claude is told at every start where it works now, as it names
                    // folders (`/mnt/c/…` for a `claude` of WSL).
                    let program =
                        accounts::program(&accounts::get(&settings, &meta.account), &settings);
                    let shown = |p: &str| match program.as_ref().and_then(engine::Program::wsl) {
                        Some(wsl) => wsl.linux_path(p).unwrap_or_else(|| p.to_string()),
                        None => p.to_string(),
                    };
                    meta.append_prompt = with_memory(
                        self.lang().claude,
                        with_follow(
                            Some(copied_conversation(
                                self.lang().claude,
                                &shown(&from.path),
                                &shown(&path),
                            )),
                            follow.as_deref(),
                        ),
                    );
                }
                meta.cwd = path.clone();
                // Out of the project's folder (its « Dossier des worktrees » beside it): watched too.
                if !self.git_watches(&project, &path) {
                    self.watch_project(&project);
                }
                meta.worktree = Some(Worktree {
                    repos: Vec::new(),
                    path,
                    branch,
                    base_branch: base,
                    existing: o.branch.is_some() && !o.new_branch,
                    named_branch: o.new_branch,
                });
            }
            // Refused as it is (a code the window reads, or a sentence): nothing was made.
            Some(Err(e)) if o.branch.is_some() => return Err(e),
            // A ticket's agent works in its own worktree or not at all.
            Some(Err(e)) if o.worktree.is_some() => {
                return Err(e.context(tr!(
                    "worktree du ticket non créé",
                    "ticket’s worktree not created"
                )))
            }
            // So does a copy of an agent that has one: the project's folder is not its code.
            Some(Err(e)) if o.copy_of.is_some() => {
                let _ = std::fs::remove_file(conv::log_path(&conversations, &meta.id));
                return Err(e.context(tr!(
                    "worktree de la copie non créé",
                    "copy’s worktree not created"
                )));
            }
            Some(Err(e)) => {
                warning = Some(tr!(
                    "Worktree non créé, l'agent travaille dans le dossier du projet : {e}",
                    "Worktree not created, the agent works in the project’s folder: {e}"
                ))
            }
            None => {}
        }
        if let Some(cwd) = o.cwd.as_ref().filter(|_| meta.worktree.is_none()) {
            meta.cwd = cwd.clone();
        }
        let id = meta.id.clone();
        let fresh = meta.worktree.clone();
        let h = Arc::new(Mutex::new(AgentRt::new(meta, &conversations)));
        // After the conversation a copy starts with.
        for text in warning
            .into_iter()
            .chain(start_notice)
            .chain(o.copy_of.and_then(|c| c.notice))
        {
            let mut fx = Effects::default();
            h.lock().notice("warn", text, &mut fx);
        }
        self.agents.write().insert(id.clone(), h.clone());
        {
            let mut rt = h.lock();
            rt.facts = self.facts.emitter();
            let m = &rt.meta;
            rt.fact(crate::facts::Body::AgentCreated {
                harness: m.engine.to_string(),
                model: m.model.clone(),
            });
            if let Some(wt) = &fresh {
                rt.fact(crate::facts::Body::WorktreeCreated {
                    branch: wt.branch.clone(),
                    path: wt.path.clone(),
                });
            }
        }
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
        // The Remote Control a setting gives it (a copy never takes its original's).
        self.apply_remote_soon(&id);
        let view = h.lock().view();
        Ok(view)
    }

    /// A copy of the agent (« Dupliquer la conversation »), refused during its turn: « <name>
    /// (copie) » (“(copy)” in English) in the same project, with its model, effort and permission mode, its conversation
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
                bail!(tr!(
                    "Attends la fin du tour de {name} pour le dupliquer.",
                    "Wait for the end of {name}’s turn to duplicate it.",
                    name = rt.meta.name
                ));
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
                    notice = Some(tr!(
                        "Les modifications non commitées de {name} ne sont pas dans cette copie.",
                        "The uncommitted changes of {name} aren’t in this copy.",
                        name = original.name
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
                // The harness it has now: the session it forks is that one's.
                engine: Some(original.engine),
                model: Some(original.model),
                effort: Some(original.effort),
                mode: Some(original.mode),
                // Where the session it forks is kept.
                account: Some(original.account),
                select: true,
                copy_of: Some(CopyOf {
                    name: original.name,
                    session_id,
                    entry,
                    worktree: original.worktree,
                    items,
                    notice,
                    remote_off: original.remote_off,
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
                setup_log.end(&tr!("échec : {r}", "failed: {r}", r = f.reason));
                let text = f.describe(i18n::ui(), worktrees::Stage::Setup);
                log::warn!("agent {id}: {text}");
                // The conversation shows its last lines, and says where the others are.
                let shown = if setup_log.written() {
                    tr!(
                        "{text}\n\nSortie complète : {log}",
                        "{text}\n\nFull output: {log}",
                        log = log_path.display()
                    )
                } else {
                    text.clone()
                };
                // Its first message tells its Claude too, in the language of the texts for Claude.
                let told = f.describe(self.lang().claude, worktrees::Stage::Setup);
                let _ = self.with_agent(id, |rt, fx| {
                    rt.setup = None;
                    rt.setup_output = None;
                    rt.setup_failure = Some(told);
                    rt.notice("warn", shown, fx);
                    Ok(())
                });
                self.warm(id);
                return;
            }
            setup_log.end(&tr!(
                "terminée en {s} s",
                "done in {s} s",
                s = step_started.elapsed().as_secs()
            ));
        }
        let text = setup_done(i18n::ui(), steps.len(), started.elapsed().as_secs());
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

    /// The worktree of an agent that waits for its first message (`worktree_pending`, ESC-156),
    /// made now. When `naming` gives something to name it by, the agent is named first, within
    /// `naming_limit` (a name come later renames its branch, as any agent's); then its branch
    /// `<prefix><name>` and its folder `<name>` (`-2`… when taken, as for every agent's) from
    /// where a new agent starts, its files copied, its setup started. The conversation shows each
    /// step. A worktree that cannot be made leaves the agent in the project's folder, as at its
    /// making. True when it waited.
    pub(crate) async fn make_pending_worktree(
        self: &Arc<Self>,
        id: &str,
        naming: Option<&str>,
    ) -> bool {
        // Once: a message and a start at the same time find it made by the first.
        let lock = self.spawn_lock(&format!("worktree-{id}"));
        let _once = lock.lock().await;
        let Ok(h) = self.agent(id) else { return false };
        if !h.lock().meta.worktree_pending {
            return false;
        }
        let ask = naming.and_then(|text| h.lock().name_from(text));
        if let Some(text) = ask {
            self.show_step(id, tr!("Nom de l’agent", "Naming the agent"));
            let (c, agent) = (self.clone(), id.to_string());
            let named =
                tauri::async_runtime::spawn(async move { c.auto_name(&agent, &text).await });
            // Not waited for longer: it goes on, and renames the branch once it comes.
            if tokio::time::timeout(self.naming_limit(), named)
                .await
                .is_err()
            {
                log::info!("agent {id}: no name yet, its worktree is made under the one it has");
            }
        }
        // As at an agent's making: the folders taken are read, and one taken, in turn.
        let creating = self.create_lock.lock().await;
        let project_id = h.lock().meta.project_id.clone();
        // A name come meanwhile waits for the branch, then renames it (`apply_name_to`).
        let naming_lock = self.spawn_lock(&format!("naming-{project_id}"));
        let naming_guard = naming_lock.lock().await;
        let made = match self.project(&project_id) {
            Ok(project) => {
                let name = h.lock().meta.name.clone();
                let slug = match slugify(&name) {
                    s if s.is_empty() => "agent".to_string(),
                    s => s,
                };
                self.show_step(
                    id,
                    tr!(
                        "Création du worktree {slug}",
                        "Creating the worktree {slug}"
                    ),
                );
                let held = self.held_worktrees();
                let mut notice = None;
                // The branch chosen at its making, if one was.
                let from = h.lock().meta.worktree_from.clone();
                let made = if project.is_multi() {
                    // Of several repositories (ESC-169): a worktree of each, in its folder.
                    let spec = crate::multiwt::Spec {
                        name: slug,
                        over: from,
                        agent: Some(id.to_string()),
                        ..Default::default()
                    };
                    self.multi_worktree(&project, &held, &spec, &mut notice)
                        .await
                } else {
                    self.agent_worktree(&project, &held, &slug, from.as_deref(), &mut notice)
                        .await
                        .map(|(path, branch, base)| Worktree {
                            repos: Vec::new(),
                            path,
                            branch,
                            base_branch: base,
                            existing: false,
                            named_branch: false,
                        })
                };
                Some((project, made, notice))
            }
            Err(e) => {
                log::warn!("agent {id}: its worktree not made: {e:#}");
                None
            }
        };
        drop(creating);
        let mut warnings = Vec::new();
        let mut setup = None;
        if let Some((project, made, notice)) = made {
            match made {
                Ok(wt) => {
                    // Those of several repositories were copied as they were made.
                    if !wt.is_multi() {
                        if let Err(e) = testlaunch::copy_worktree_files(
                            &project.path,
                            &wt.path,
                            &project.worktree_copy,
                        )
                        .await
                        {
                            warnings.push(files_not_copied(&e));
                        }
                    }
                    // Out of the project's folder (its « Dossier des worktrees » beside it): watched too.
                    if !self.git_watches(&project, &wt.path) {
                        self.watch_project(&project);
                    }
                    setup = Some((project, wt));
                }
                Err(e) => warnings.push(tr!(
                    "Worktree non créé, l'agent travaille dans le dossier du projet : {e}",
                    "Worktree not created, the agent works in the project’s folder: {e}"
                )),
            }
            warnings.extend(notice);
        }
        let kept = self.with_agent(id, |rt, fx| {
            rt.meta.worktree_pending = false;
            rt.meta.worktree_from = None;
            rt.setup = None;
            if let Some((_, wt)) = &setup {
                rt.meta.cwd = wt.path.clone();
                rt.meta.worktree = Some(wt.clone());
                rt.fact(crate::facts::Body::WorktreeCreated {
                    branch: wt.branch.clone(),
                    path: wt.path.clone(),
                });
            }
            for text in warnings {
                rt.notice("warn", text, fx);
            }
            Ok(())
        });
        drop(naming_guard);
        // Deleted meanwhile: what was made for it goes too.
        if kept.is_err() {
            if let Some((project, wt)) = setup {
                if wt.is_multi() {
                    let _ = crate::multiwt::remove(&project, &wt, false, git::REMOVE_RETRY).await;
                } else {
                    let _ = git::worktree_remove_dir(&project.path, &wt.path).await;
                    let _ = git::branch_delete(&project.path, &wt.branch, true).await;
                }
            }
            return true;
        }
        if let Some((project, wt)) = &setup {
            self.start_setup(&h, project, wt);
            self.emit_agent(&h);
        }
        self.request_save();
        self.git.refresh(&project_id);
        true
    }

    /// The agent shows `step` of the making of its worktree (`setup`, as its setup's steps).
    pub(crate) fn show_step(self: &Arc<Self>, id: &str, step: String) {
        let _ = self.with_agent(id, |rt, _| {
            rt.setup = Some(step);
            Ok(())
        });
    }

    /// The session of `proc`, the agent's, takes the agent's name once its harness has one to
    /// rename (Codex's thread, OpenCode's and Copilot's sessions come with the first message).
    fn retitle_soon(self: &Arc<Self>, id: &str, proc: Process) {
        let (c, id) = (self.clone(), id.to_string());
        tauri::async_runtime::spawn(async move {
            for _ in 0..50 {
                let title = {
                    let Ok(h) = c.agent(&id) else { return };
                    let rt = h.lock();
                    if !rt.proc.as_ref().is_some_and(|p| Arc::ptr_eq(p, &proc)) {
                        return;
                    }
                    c.session_title(&rt)
                };
                match proc.rename(&title).await {
                    Ok(()) => return,
                    Err(e) => log::debug!("agent {id}: session not renamed yet: {e:#}"),
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
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
                    log::warn!("{}", f.describe(i18n::Lang::En, worktrees::Stage::Teardown));
                    problems.push(f.summary(i18n::ui(), worktrees::Stage::Teardown));
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

    /// A restored agent whose worktree folder went (removed once its ticket was over) gets it back
    /// from its branch, with the project's files to copy and its setup. Without its branch
    /// (merged, then deleted), it stays as it is, and its conversation says so.
    async fn restore_worktree(self: &Arc<Self>, id: &str) {
        let Ok(h) = self.agent(id) else { return };
        let (pid, wt) = {
            let rt = h.lock();
            (rt.meta.project_id.clone(), rt.meta.worktree.clone())
        };
        let (Some(mut wt), Ok(project)) = (wt, self.project(&pid)) else {
            return;
        };
        // Of several repositories (ESC-169): its folder made again, a worktree of each.
        if wt.is_multi() {
            if Path::new(&wt.path).exists() {
                return;
            }
            let problem = match crate::multiwt::restore(&project, &wt).await {
                Ok(()) => {
                    self.start_setup(&h, &project, &wt);
                    None
                }
                Err(e) => Some(tr!(
                    "Worktree non recréé depuis {b} : {e:#}",
                    "Worktree not recreated from {b}: {e:#}",
                    b = wt.branch
                )),
            };
            if let Some(p) = problem {
                let _ = self.with_agent(id, |rt, fx| {
                    rt.notice("warn", p, fx);
                    Ok(())
                });
            }
            return;
        }
        if !git::branch_exists(&project.path, &wt.branch).await {
            if !Path::new(&wt.path).exists() {
                let text = worktree_gone(self.lang().ui, &wt.branch);
                let _ = self.with_agent(id, |rt, fx| {
                    rt.notice("warn", text, fx);
                    Ok(())
                });
            }
            return;
        }
        let mut moved = None;
        if Path::new(&wt.path).exists() {
            // Its own, still there: nothing to do.
            let ours = git::checkout_of(&project.path, &wt.branch)
                .await
                .ok()
                .flatten()
                .is_some_and(|p| crate::tickets::same_dir(&p, &wt.path));
            if ours {
                return;
            }
            // Another worktree took its folder meanwhile (a folder of worktrees shared with
            // another repository): its own is made beside it, never worked in another's.
            let held = self.held_worktrees();
            let from = PathBuf::from(&wt.path);
            let name = from
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let parent = from.parent().unwrap_or(Path::new("")).to_path_buf();
            let mut n = 2;
            let mut dir = parent.join(format!("{name}-{n}"));
            while !git::free_folder(&dir, &held) {
                n += 1;
                dir = parent.join(format!("{name}-{n}"));
            }
            moved = Some(wt.path.clone());
            wt.path = dir.to_string_lossy().into_owned();
        }
        let problem = match git::worktree_restore(&project.path, &wt.path, &wt.branch).await {
            Ok(()) => {
                if let Some(from) = &moved {
                    let (path, lang) = (wt.path.clone(), self.lang().ui);
                    let _ = self.with_agent(id, |rt, fx| {
                        if rt.meta.cwd == *from {
                            rt.meta.cwd = path.clone();
                        }
                        if let Some(w) = rt.meta.worktree.as_mut() {
                            w.path = path.clone();
                        }
                        rt.notice("warn", worktree_moved(lang, from, &path), fx);
                        Ok(())
                    });
                    self.request_save();
                    if !self.git_watches(&project, &wt.path) {
                        self.watch_project(&project);
                    }
                }
                let copied = testlaunch::copy_worktree_files(
                    &project.path,
                    &wt.path,
                    &project.worktree_copy,
                )
                .await;
                self.start_setup(&h, &project, &wt);
                copied.err().map(|e| files_not_copied(&e))
            }
            Err(e) => Some(tr!(
                "Worktree non recréé depuis {b} : {e:#}",
                "Worktree not recreated from {b}: {e:#}",
                b = wt.branch
            )),
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
        self.suggest_worktree_steps_for(&self.project(project_id)?)
            .await
    }

    /// The same for a project not made yet (the new project's assistant), from its folder alone.
    pub async fn suggest_worktree_steps_at(&self, path: &str) -> Result<WorktreeSuggestion> {
        self.suggest_worktree_steps_for(&unsaved_project(path)?)
            .await
    }

    async fn suggest_worktree_steps_for(&self, project: &Project) -> Result<WorktreeSuggestion> {
        let shell = self.default_shell()?;
        let lang = self.lang().claude;
        let prompt = worktrees::suggest_prompt(
            lang,
            &shell.label,
            &project.worktree_copy,
            isola::configured(&project.path),
        );
        let answer = self
            .read_project(project, &worktrees::suggest_system(lang), &prompt)
            .await?;
        let suggestion = worktrees::parse_suggestion(&answer, Path::new(&project.path), &shell.id)
            .ok_or_else(no_readable_commands)?;
        // Not the same as having found nothing to run: it gave some, none of which could be shown
        // as they would run.
        if suggestion.setup.is_empty() && suggestion.teardown.is_empty() && suggestion.refused > 0 {
            bail!(worktrees::all_refused(i18n::ui()));
        }
        Ok(suggestion)
    }

    /// The launch commands Claude suggests for the project (the servers, watchers and services
    /// its developer keeps running), from what it reads of it, each for the default shell. Only
    /// a suggestion: the window shows it in the draft of the settings, nothing is saved here.
    pub async fn suggest_run_commands(&self, project_id: &str) -> Result<RunSuggestion> {
        self.suggest_run_commands_for(&self.project(project_id)?)
            .await
    }

    /// The same for a project not made yet (the new project's assistant), from its folder alone.
    pub async fn suggest_run_commands_at(&self, path: &str) -> Result<RunSuggestion> {
        self.suggest_run_commands_for(&unsaved_project(path)?).await
    }

    async fn suggest_run_commands_for(&self, project: &Project) -> Result<RunSuggestion> {
        let shell = self.default_shell()?;
        let lang = self.lang().claude;
        let prompt = worktrees::run_suggest_prompt(lang, &shell);
        let answer = self
            .read_project(project, &worktrees::run_suggest_system(lang), &prompt)
            .await?;
        let suggestion =
            worktrees::parse_run_suggestion(&answer, Path::new(&project.path), &shell.id)
                .ok_or_else(no_readable_commands)?;
        // Not the same as having found nothing to launch (see `suggest_worktree_steps`).
        if suggestion.commands.is_empty() && suggestion.refused > 0 {
            bail!(worktrees::all_refused(i18n::ui()));
        }
        Ok(suggestion)
    }

    /// The shell that runs what is not given another: the system's first.
    fn default_shell(&self) -> Result<ShellInfo> {
        let settings = self.settings.read().clone();
        crate::pty::detect_shells(&settings)
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!(tr!("Aucun shell détecté.", "No shell found.")))
    }

    /// Claude's answer to `prompt` after it read the project, in its folder, with tools that only
    /// read (never Bash nor Edit): it can neither change nor run anything.
    async fn read_project(&self, project: &Project, system: &str, prompt: &str) -> Result<String> {
        self.ask_harness(Ask {
            weight: Weight::Reading,
            cwd: Path::new(&project.path),
            system,
            prompt,
            limit: worktrees::SUGGEST_LIMIT,
            agent: None,
            // A project not made yet has no account of its own: one outside the projects reads it.
            project: (!project.id.is_empty()).then_some(project.id.as_str()),
        })
        .await
    }

    /// `read_project` in `cwd` (a folder of `project_id`'s), within `limit` (ESC-263: the
    /// comparison of the spec to the code).
    pub(crate) async fn read_folder(
        &self,
        project_id: &str,
        cwd: &Path,
        system: &str,
        prompt: &str,
        limit: Duration,
    ) -> Result<String> {
        self.ask_harness(Ask {
            weight: Weight::Reading,
            cwd,
            system,
            prompt,
            limit,
            agent: None,
            project: Some(project_id),
        })
        .await
    }

    /// The agent `id` named after `prompt` by the writer model. The name is the agent's that asked
    /// for it: one closed meanwhile (its project closed, the agent deleted) takes none, nor does
    /// the agent its project's history gives back, though it has the same id (ESC-300).
    pub(crate) async fn auto_name(self: &Arc<Self>, id: &str, prompt: &str) {
        let Ok(h) = self.agent(id) else { return };
        match self.generate_name(id, prompt).await {
            Ok(Some(slug)) => {
                if let Err(e) = self.apply_name_to(&h, id, &slug).await {
                    log::warn!("auto-naming failed: {e:#}");
                }
            }
            Ok(None) => {
                log::info!("agent {id}: no usable name from the model, keeping the default one")
            }
            Err(e) => log::warn!("auto-naming failed: {e:#}"),
        }
    }

    /// One question to the harness of « Modèle des textes rédigés » (Claude: Haiku, no tools, no
    /// session, no MCP) about `agent`, asked of its account (with none, of the account a new agent
    /// of `project` would go to): its answer, within 90 s.
    pub(crate) async fn one_shot(
        &self,
        system: &str,
        prompt: &str,
        agent: Option<&str>,
        project: Option<&str>,
    ) -> Result<String> {
        self.one_shot_within(system, prompt, engine::oneshot::LIMIT, agent, project)
            .await
    }

    /// `one_shot` within `limit`, the question's writing included (a `claude` that never reads
    /// it would hold it forever); past it, the process is killed with all it started.
    pub(crate) async fn one_shot_within(
        &self,
        system: &str,
        prompt: &str,
        limit: Duration,
        agent: Option<&str>,
        project: Option<&str>,
    ) -> Result<String> {
        self.ask_harness(Ask {
            weight: Weight::Light,
            cwd: &std::env::temp_dir(),
            system,
            prompt,
            limit,
            agent,
            project,
        })
        .await
    }

    /// Who answers a one-shot question about `project`, said before it is asked (ESC-196): the
    /// harness of « Modèle des textes rédigés », its model as the interface names it, and the
    /// account a new agent of the project would go to (a private project's reserved one).
    pub(crate) async fn one_shot_who(&self, project: &str) -> Result<OneShotWho> {
        let settings = self.settings.read().clone();
        let id = engine::oneshot::writer(settings.writer_engine, &self.engines_found())
            .unwrap_or(EngineId::CLAUDE);
        let engine = engine::by_id(id);
        let account = self.account_for(id, None, Some(project))?;
        let (model, priced) = if engine.writer_takes_model() {
            let program = accounts::program(&account, &settings);
            let env = [settings.claude_env(), accounts::launch_env(&account)].concat();
            let model = match program {
                Some(p) => self.writer_model(engine, &p, env).await,
                None => String::new(),
            };
            (format!("{} ({model})", engine.writer(Weight::Light)), model)
        } else {
            (
                engine.writer(Weight::Light).to_string(),
                engine.writer_priced(Weight::Light),
            )
        };
        Ok(OneShotWho {
            engine: id,
            model,
            priced,
            account,
        })
    }

    /// The id of the account agent `id` runs on: empty for an agent gone.
    pub(crate) fn account_of(&self, id: &str) -> String {
        let account = self
            .agent(id)
            .map(|h| h.lock().meta.account.clone())
            .unwrap_or_default();
        accounts::get(&self.settings.read(), &account).id
    }

    /// The account of `engine` a question is asked of: its `agent`'s when the agent runs on that
    /// harness; else, as a new agent of its `project` (when it is about one) would go. Refused when
    /// no account of it serves that project.
    fn account_for(
        &self,
        engine: EngineId,
        agent: Option<&str>,
        project: Option<&str>,
    ) -> Result<Account> {
        let settings = self.settings.read().clone();
        // No account of the harness (ESC-128): one is to be added first.
        if !accounts::has_any(&settings, engine) {
            return Err(accounts::no_account_of(engine));
        }
        let of_agent = agent
            .and_then(|id| self.agent(id).ok())
            .map(|h| h.lock().meta.account.clone())
            .filter(|id| accounts::get(&settings, id).engine == engine);
        let id = match (of_agent, project) {
            (Some(id), _) => id,
            (None, Some(project)) => self.account_for_new(project, engine)?,
            (None, None) if !accounts::serves_any(&settings, engine, None) => {
                return Err(accounts::no_account_outside_projects(engine))
            }
            (None, None) => self
                .pick_account(&accounts::for_project(&settings, engine, None), None)
                .ok_or_else(|| accounts::no_account_of(engine))?,
        };
        Ok(accounts::get(&settings, &id))
    }

    /// One question to the harness of « Modèle des textes rédigés » (no session, no settings, no
    /// MCP), as `ask` says, on the account of the agent or the project it is about: its answer,
    /// within its limit (`engine::oneshot`).
    async fn ask_harness(&self, ask: Ask<'_>) -> Result<String> {
        let Ask {
            weight,
            cwd,
            system,
            prompt,
            limit,
            agent,
            project,
        } = ask;
        let settings = self.settings.read().clone();
        // None found: Claude Code's, which says what is missing, as before there was a choice.
        let id = engine::oneshot::writer(settings.writer_engine, &self.engines_found())
            .unwrap_or(EngineId::CLAUDE);
        let engine = engine::by_id(id);
        let account = self.account_for(id, agent, project)?;
        accounts::key_ready(&account)?;
        let settings = self.settings.read().clone();
        let program = accounts::program(&account, &settings).ok_or_else(|| {
            // By its command (`claude`), as before there was a choice.
            let name = engine.command();
            anyhow!(tr!("{name} introuvable", "{name} not found"))
        })?;
        let env = [settings.claude_env(), accounts::launch_env(&account)].concat();
        self.before_claude(&account).await;
        let model = if engine.writer_takes_model() {
            self.writer_model(engine, &program, env.clone()).await
        } else {
            String::new()
        };
        engine::oneshot::ask(
            engine,
            OneShot {
                program,
                env,
                weight,
                cwd,
                system,
                prompt,
                limit,
                model,
            },
        )
        .await
    }

    /// The model the texts of `harness` are written by (`Engine::writer_takes_model`): the one the
    /// settings name while it is one of its account's, else its default one, its models listed
    /// first when none is known yet (`opencode models`, no model asked anything). Empty when it
    /// has none: no text is asked then (OpenCode would pick one itself, OpenCode Zen without an
    /// account).
    async fn writer_model(
        &self,
        harness: &dyn crate::engine::Engine,
        program: &crate::engine::Program,
        env: Vec<(String, String)>,
    ) -> String {
        let chosen = self.settings.read().writer_model.clone();
        let pick = |told: &[crate::engine::registry::ToldModel]| {
            told.iter()
                .find(|m| !chosen.is_empty() && m.id == chosen)
                .or(told.iter().find(|m| m.default))
                .or(told.first())
                .map(|m| m.id.clone())
        };
        let known = self
            .engine_models
            .read()
            .get(&harness.id())
            .cloned()
            .unwrap_or_default();
        let listed_chosen = chosen.is_empty() || known.iter().any(|m| m.id == chosen);
        if !known.is_empty() && listed_chosen {
            return pick(&known).unwrap_or_default();
        }
        match harness
            .list_models(program.clone(), env, &std::env::temp_dir())
            .await
        {
            Ok(Some(listed)) => {
                self.set_engine_models(harness.id(), listed.clone());
                pick(&listed).unwrap_or_default()
            }
            Ok(None) => pick(&known).unwrap_or_default(),
            Err(e) => {
                log::warn!("{}: models not listed: {e:#}", harness.name());
                pick(&known).unwrap_or_default()
            }
        }
    }

    /// A short name for the task, or None when the model did not answer with one.
    async fn generate_name(&self, agent: &str, prompt: &str) -> Result<Option<String>> {
        let lang = self.lang().claude;
        let answer = self
            .one_shot(
                &naming_system(lang),
                &naming_prompt(lang, prompt),
                Some(agent),
                None,
            )
            .await?;
        Ok(name_from_answer(&answer))
    }

    /// The name `slug` given to the agent `id` as it is now, as Haiku's would (tests only).
    #[cfg(test)]
    pub(crate) async fn apply_generated_name(self: &Arc<Self>, id: &str, slug: &str) -> Result<()> {
        let h = self.agent(id)?;
        self.apply_name_to(&h, id, slug).await
    }

    /// Whether `h` is still the agent `id` of the app: neither closed with its project nor
    /// deleted, nor an older handle of an agent its project's history gave back.
    fn is_live(&self, h: &AgentHandle, id: &str) -> bool {
        self.agents
            .read()
            .get(id)
            .is_some_and(|live| Arc::ptr_eq(live, h))
    }

    /// The name `slug` given by Haiku to `h`, the agent `id` as it was when its name was asked:
    /// suffixed when another agent of the project has it, its branch renamed first.
    async fn apply_name_to(self: &Arc<Self>, h: &AgentHandle, id: &str, slug: &str) -> Result<()> {
        let project_id = h.lock().meta.project_id.clone();
        // One agent named at a time in a project, from the names taken to the name set: two named
        // at once with the same name (the branch renames are awaited in between) would both find
        // it free.
        let naming = self.spawn_lock(&format!("naming-{project_id}"));
        let naming_guard = naming.lock().await;
        // Gone meanwhile: nothing is renamed, branch or name. The closing of its project takes
        // this lock to put each agent out (`remove_project`): one still there now stays until its
        // branch and its name are both set, and its history keeps them.
        if !self.is_live(h, id) {
            log::info!("agent {id}: gone before its name came, the name is dropped");
            #[cfg(test)]
            self.names_dropped.fetch_add(1, Ordering::AcqRel);
            return Ok(());
        }
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
        if h.lock().meta.named {
            return Ok(());
        }
        // The branch first, as well as it can be: the agent is said named only once the worktree
        // is on its last branch (renamed or not), so that whoever sees it named (a commit
        // proposed meanwhile) does not find git in the middle of the rename.
        let renamed = self
            .rename_agent_branch(h, &project_id, &name)
            .await
            .ok()
            .flatten();
        let (named_now, proc, title) = {
            let mut rt = h.lock();
            if let (Some(branch), Some(w)) = (renamed, rt.meta.worktree.as_mut()) {
                w.branch = branch;
            }
            // (Named by the user meanwhile: their name stands.)
            let named_now = !rt.meta.named;
            if named_now {
                let was = std::mem::replace(&mut rt.meta.name, name.clone());
                rt.meta.named = true;
                if was != name {
                    rt.fact(crate::facts::Body::AgentRenamed {
                        from: was,
                        to: name.clone(),
                    });
                }
            }
            (named_now, rt.proc.clone(), self.session_title(&rt))
        };
        // The name is set: the next one in the project need not wait for the session rename.
        drop(naming_guard);
        self.emit_agent(h);
        self.request_save();
        if !named_now {
            return Ok(());
        }
        if let Some(p) = proc {
            let _ = p.rename(&title).await;
        }
        Ok(())
    }

    pub async fn rename_agent(self: &Arc<Self>, id: &str, name: &str) -> Result<Option<String>> {
        let name = name.trim();
        if name.is_empty() {
            bail!(tr!("nom vide", "empty name"));
        }
        let h = self.agent(id)?;
        let project_id = h.lock().meta.project_id.clone();
        // Never across Haiku naming it (`apply_name_to`): the branch first, as it does.
        let naming = self.spawn_lock(&format!("naming-{project_id}"));
        let naming_guard = naming.lock().await;
        // Closed while it waited: its history keeps the name it had.
        if !self.is_live(&h, id) {
            return Err(agent_not_found());
        }
        let (renamed, kept) = match self
            .rename_agent_branch(&h, &project_id, &slugify(name))
            .await
        {
            Ok(branch) => (branch, None),
            Err((branch, why)) => (None, Some(branch_kept(i18n::ui(), &branch, &why))),
        };
        let (proc, title) = {
            let mut rt = h.lock();
            if let (Some(branch), Some(w)) = (renamed, rt.meta.worktree.as_mut()) {
                w.branch = branch;
            }
            let was = std::mem::replace(&mut rt.meta.name, name.to_string());
            rt.meta.named = true;
            if was != name {
                rt.fact(crate::facts::Body::AgentRenamed {
                    from: was,
                    to: name.to_string(),
                });
            }
            (rt.proc.clone(), self.session_title(&rt))
        };
        drop(naming_guard);
        self.emit_agent(&h);
        self.request_save();
        if let Some(p) = proc {
            let _ = p.rename(&title).await;
        }
        Ok(kept)
    }

    /// The branch of the agent `h`, named `slug`, renamed `<agent_branch_prefix><slug>` (under the
    /// project's `naming-` lock): Some(the new branch) once renamed, None when there is nothing to
    /// do (no worktree, no slug, the same name). Kept, with the branch and why: one that was there
    /// before the agent or that the user named, one with a pull request (ESC-64), one pushed
    /// (renamed, it would part from its remote), one whose new name a branch has. The worktree's
    /// folder stays where it is.
    async fn rename_agent_branch(
        &self,
        h: &AgentHandle,
        project_id: &str,
        slug: &str,
    ) -> std::result::Result<Option<String>, (String, BranchKept)> {
        let (wt, with_pr) = {
            let rt = h.lock();
            (rt.meta.worktree.clone(), !rt.meta.prs.is_empty())
        };
        let (Some(wt), Ok(project)) = (wt, self.project(project_id)) else {
            return Ok(None);
        };
        let branch = format!("{}{slug}", project.agent_branch_prefix);
        if slug.is_empty() || branch == wt.branch {
            return Ok(None);
        }
        let kept = |why| Err((wt.branch.clone(), why));
        if wt.existing {
            return kept(BranchKept::Existing);
        }
        if wt.named_branch {
            return kept(BranchKept::Typed);
        }
        if with_pr {
            return kept(BranchKept::PullRequest);
        }
        // Of several repositories (ESC-169): the branch of each, or of none.
        let checkouts = wt.checkouts();
        for c in &checkouts {
            // The agent went to another branch by itself, not followed yet (`follow_agent_branch`):
            // `git branch -m` would rename that one, which may be anyone's.
            if git::head_branch(&c.dir).await != wt.branch {
                return Ok(None);
            }
            if git::is_pushed(&c.dir, &wt.branch).await {
                return kept(BranchKept::Pushed);
            }
            if git::branch_exists(&c.dir, &branch).await {
                return kept(BranchKept::Taken(branch));
            }
        }
        let mut renamed: Vec<&str> = Vec::new();
        for c in &checkouts {
            if let Err(e) = git::rename_current_branch(&c.dir, &branch).await {
                for dir in renamed {
                    let _ = git::rename_current_branch(dir, &wt.branch).await;
                }
                return kept(BranchKept::Failed(format!("{e:#}")));
            }
            renamed.push(&c.dir);
        }
        Ok(Some(branch))
    }

    pub async fn archive_agent(self: &Arc<Self>, id: &str, archived: bool) -> Result<()> {
        if archived {
            // An archived agent is no longer reachable from claude.ai (nor set apart: restored, a
            // setting gives it Remote Control again).
            let h = self.agent(id)?;
            if h.lock().meta.remote_control {
                self.cut_remote(&h).await;
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
                    // Told archived with its turn's end: the window sends it none of its held messages.
                    rt.meta.archived = true;
                }
                running
            };
            if let Some(p) = running {
                let _ = p.interrupt(true, Duration::from_secs(5)).await;
            }
        }
        let lock = self.spawn_lock(id);
        let guard = lock.lock().await;
        // The process it had, and the remote session it was linked to on claude.ai (a setting
        // changed meanwhile), with its name there.
        let mut stopped = None;
        let mut linked = None;
        // Its Claude Code interface, which quits as its user would once the agent is archived.
        let mut interface = None;
        // Its interface joined to the process (OpenCode), which stops with it.
        let mut joined = None;
        self.with_agent(id, |rt, fx| {
            rt.meta.archived = archived;
            if !archived {
                rt.meta.worktree_kept = None;
            }
            if archived {
                joined = rt.attach.take().map(|a| a.info.id);
                let name = self.remote_name(&rt.meta);
                linked = rt.end_remote().map(|link| (link, name));
                rt.meta.resume_at = None;
                // Its test ports are free for others.
                rt.meta.port_base = None;
                stopped = rt.detach();
                if let Some(t) = rt.tui.as_ref().map(|t| t.term_id.clone()) {
                    rt.gen += 1;
                    rt.stop_tui(fx);
                    self.hooks.revoke(&rt.meta.id);
                    interface = Some((t, crate::tui::quit_key(&rt.meta)));
                }
                rt.clear_pending(fx);
                if rt.meta.status.is_active() {
                    rt.set_status(AgentStatus::Done, fx);
                }
                rt.fact(crate::facts::Body::AgentStopped { removed: false });
            }
            fx.save = true;
            Ok(())
        })?;
        // Its session on claude.ai is closed for good before its input closes.
        if let Some((link, name)) = linked {
            let h = self.agent(id)?;
            self.close_link(&h, link, &name, Duration::from_secs(5), false)
                .await;
        }
        if let Some(p) = stopped {
            p.close_input();
        }
        if let Some(t) = joined {
            let pty = self.pty.clone();
            let _ = tokio::task::spawn_blocking(move || pty.kill(&t)).await;
        }
        // Marked as quitting under the launch lock, which is let go for the seconds it takes.
        if let Some((t, key)) = interface {
            let quit = self.quit_soon(id, &t, key);
            drop(guard);
            let _ = quit.await;
        }
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
            self.apply_remote_soon(id);
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
        let h = self.agent(id)?;
        if h.lock().meta.remote_control {
            self.cut_remote(&h).await;
        }
        // Wait for an in-flight start (warm-up) so that its process is killed too.
        let lock = self.spawn_lock(id);
        let guard = lock.lock().await;
        let h = self.agents.write().remove(id).ok_or_else(agent_not_found)?;
        let (pid, worktree, ports, killed, interface, meta) = {
            let mut rt = h.lock();
            rt.gen += 1;
            let killed = rt.proc.take();
            if let Some(p) = &killed {
                p.kill();
            }
            let interface = rt
                .tui
                .take()
                .map(|t| (t.term_id, crate::tui::quit_key(&rt.meta)));
            rt.conv.delete_file();
            rt.fact(crate::facts::Body::AgentStopped { removed: true });
            (
                rt.meta.project_id.clone(),
                rt.meta.worktree.clone(),
                rt.meta.port_base,
                killed,
                interface,
                rt.meta.clone(),
            )
        };
        // Refused at once, without waiting for its process to end.
        self.mcp.forget_agent(id);
        self.hooks.revoke(id);
        self.forget_workflow_track(&pid, id);
        // Its Claude Code interface quits as its user would (it archives its remote session), and
        // is gone before its worktree is removed; the launch lock is not held meanwhile.
        let quit = interface.map(|(t, key)| self.quit_soon(id, &t, key));
        self.spawn_locks.lock().remove(id);
        self.quit_locks.lock().remove(id);
        self.remote_locks.lock().remove(id);
        drop(guard);
        if let Some(quit) = quit {
            let _ = quit.await;
        }
        // The setup of its worktree stops, with what it started (which holds the worktree). Once it
        // is gone: what waited for that setup (its ticket's first message) finds no agent to send to.
        // Its log goes with it.
        self.stop_setup(id).await;
        let _ = std::fs::remove_file(self.data.setup_log(id));
        self.forget_pastes(id);
        self.forget_interface(&meta);
        // The files joined to its messages (the copies; never a file of the user's own).
        self.forget_attachments(id);
        self.forget_sidechains(id);
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
                // The killed process tree must be gone to release its handles on the worktree. One
                // that is not by then is not waited for any longer: the warning says how many
                // still run, and the removal below is tried for a short while only.
                let mut held = false;
                if let Some(p) = killed {
                    if let Some(problem) = self.await_agent_gone(id, &p).await {
                        held = true;
                        problems.push(problem);
                    }
                }
                // Its git commands in the folder (the refresh of the project's state) end first.
                let waited = Instant::now();
                self.wait_git_idle(&pid, GIT_IDLE).await;
                log::info!(
                    "agent {id}: the project's git was idle after {} ms",
                    waited.elapsed().as_millis()
                );
                problems.extend(self.teardown_worktree(&project, &wt, ports).await);
                // A branch that was there before the agent is the user's: only its folder goes.
                let retry = if held {
                    git::REMOVE_RETRY_HELD
                } else {
                    git::REMOVE_RETRY
                };
                let waited = Instant::now();
                let removed = if wt.is_multi() {
                    // Of several repositories (ESC-169): each judged apart, its work kept.
                    match self
                        .clean_multi(&project, &wt, wt.existing, ports, false)
                        .await
                    {
                        None => Ok(()),
                        Some((_, reason)) => Err(anyhow!(reason)),
                    }
                } else if wt.existing {
                    git::worktree_remove_dir_within(&project.path, &wt.path, retry).await
                } else {
                    git::worktree_remove_within(&project.path, &wt.path, &wt.branch, retry).await
                };
                log::info!(
                    "agent {id}: its worktree removal took {} ms ({})",
                    waited.elapsed().as_millis(),
                    if removed.is_ok() { "done" } else { "failed" }
                );
                if let Err(e) = removed {
                    problems.push(tr!(
                        "le worktree n'a pas pu être nettoyé : {e:#}",
                        "the worktree couldn’t be cleaned up: {e:#}"
                    ));
                }
            }
            // Kept: isola's services of the worktree stop all the same.
            (false, Some(wt), _) if isola::manages(&wt.path) => isola::down_later(wt.path),
            _ => {}
        }
        let warning = (!problems.is_empty()).then(|| {
            let problems = problems.join(&tr!(" ; ", "; "));
            tr!(
                "Agent supprimé, mais {problems}",
                "Agent deleted, but {problems}"
            )
        });
        self.git.refresh(&pid);
        Ok(warning)
    }

    /// Brings the agent's base branch into its branch, as the project's strategy says (a rebase for
    /// « rebase », a merge for « merge » and « squash »: there is nothing to squash into a branch).
    /// Refused during the agent's turn, with changes of its own in its worktree, with a merge or a
    /// rebase under way there, and for an archived agent or one without a worktree. A merge that
    /// conflicts is left in the worktree and a rebase that conflicts is undone: the agent is asked
    /// to resolve it, in the words of the board for a ticket's agent.
    pub async fn integrate_base(self: &Arc<Self>, id: &str) -> Result<Integration> {
        // The app does not restart for an update in the middle of it.
        let _working = self.working();
        let lang = i18n::ui();
        let h = self.agent(id)?;
        let (pid, name, wt, archived, active, ticketed) = {
            let rt = h.lock();
            (
                rt.meta.project_id.clone(),
                rt.meta.name.clone(),
                rt.meta.worktree.clone(),
                rt.meta.archived,
                rt.meta.status.is_active(),
                rt.meta.ticket_id.is_some(),
            )
        };
        let wt = wt.ok_or_else(agent_without_worktree)?;
        let base = wt.base_branch.clone();
        if archived {
            bail!(integrate_archived(lang, &name, &base));
        }
        if active {
            bail!(integrate_busy(lang, &name, &base));
        }
        // Its ticket's validation commits, merges and rebases in this very worktree.
        if self
            .tickets
            .read()
            .iter()
            .any(|t| crate::tickets::validating(t, id))
        {
            bail!(crate::tickets::Refusal::ApprovingAlready);
        }
        let project = self.project(&pid)?;
        // Of several repositories (ESC-169): each one's base into its worktree, all checked first.
        if wt.is_multi() {
            let checkouts = wt.checkouts();
            for c in &checkouts {
                let repo_dir = crate::repos::join(&project.path, &c.repo);
                if c.base.is_empty() || !git::branch_exists(&repo_dir, &c.base).await {
                    bail!(integrate_base_gone(lang, &c.base));
                }
                if let Some(operation) = git::operation_in_progress(&c.dir).await {
                    bail!(integrate_under_way(lang, &name, operation));
                }
                if git::has_tracked_changes(&c.dir).await? {
                    bail!(integrate_dirty(lang));
                }
            }
            let rebase = project.board.strategy == "rebase";
            let (mut files, mut any_done, mut conflict_base) = (Vec::new(), false, String::new());
            for c in &checkouts {
                match git::update_branch(&c.dir, &c.base, rebase).await? {
                    git::Updated::UpToDate => {}
                    git::Updated::Done => any_done = true,
                    git::Updated::Conflict(f) => {
                        if conflict_base.is_empty() {
                            conflict_base = c.base.clone();
                        }
                        files.extend(f.iter().map(|f| crate::repos::prefixed(&c.repo, f)));
                    }
                }
            }
            self.git.refresh(&pid);
            if files.is_empty() {
                return Ok(if any_done {
                    Integration::Done
                } else {
                    Integration::UpToDate
                });
            }
            let claude = self.lang().claude;
            let text = match (ticketed, rebase) {
                (true, true) => board::rebase_message(claude, &conflict_base),
                (true, false) => board::conflict_message(claude, &conflict_base, &files),
                (false, true) => integrate_rebase_message(claude, &conflict_base),
                (false, false) => integrate_conflict_message(claude, &conflict_base, &files),
            };
            self.send_by_app(id, text).await?;
            return Ok(Integration::Conflict { files, rebase });
        }
        if base.is_empty() || !git::branch_exists(&project.path, &base).await {
            bail!(integrate_base_gone(lang, &base));
        }
        if let Some(operation) = git::operation_in_progress(&wt.path).await {
            bail!(integrate_under_way(lang, &name, operation));
        }
        if git::has_tracked_changes(&wt.path).await? {
            bail!(integrate_dirty(lang));
        }
        let rebase = project.board.strategy == "rebase";
        let updated = git::update_branch(&wt.path, &base, rebase).await;
        self.git.refresh(&pid);
        let files = match updated? {
            git::Updated::UpToDate => return Ok(Integration::UpToDate),
            git::Updated::Done => return Ok(Integration::Done),
            git::Updated::Conflict(files) => files,
        };
        let claude = self.lang().claude;
        let text = match (ticketed, rebase) {
            (true, true) => board::rebase_message(claude, &base),
            (true, false) => board::conflict_message(claude, &base, &files),
            (false, true) => integrate_rebase_message(claude, &base),
            (false, false) => integrate_conflict_message(claude, &base, &files),
        };
        self.send_by_app(id, text).await.map_err(|e| {
            e.context(tr!(
                "Les conflits de {base} sont dans le worktree, mais {name} n’a pas pu en être prévenu",
                "The conflicts with {base} are in the worktree, but {name} could not be told",
                base = base,
                name = name
            ))
        })?;
        Ok(Integration::Conflict { files, rebase })
    }

    /// The branch the project's work goes into: the board's target while that branch exists, else
    /// the one the folder is on ("" on a detached HEAD). Read only, never fixed.
    pub(crate) async fn base_of(&self, project: &Project, root: &str) -> String {
        let target = project.board.target.trim();
        if !target.is_empty() && git::branch_exists(root, target).await {
            target.to_string()
        } else {
            git::head_branch(root).await
        }
    }

    /// A new agent's own worktree, on a new branch: from where its project's new worktrees start
    /// (« Branche de départ des nouveaux worktrees »), or from `over`, the branch chosen for it
    /// (`start_point`, which may say in `notice` why it starts from a branch as it is, or from
    /// HEAD).
    async fn agent_worktree(
        &self,
        project: &Project,
        held: &[PathBuf],
        name: &str,
        over: Option<&str>,
        notice: &mut Option<String>,
    ) -> Result<(String, String, String)> {
        let from_head = over.is_none() && project.worktree_base == WorktreeBase::Head;
        let root = match self.toplevel(&project.path).await {
            Some(root) if !from_head => root,
            _ => {
                return git::worktree_add(
                    &project.path,
                    &project.worktrees_root(),
                    held,
                    &project.agent_branch_prefix,
                    name,
                )
                .await
            }
        };
        // Not while a switch or a pull moves the branches.
        let lock = self.sync_lock(&root);
        let _guard = lock.lock().await;
        let (from, why) = start_point(project, &root, over).await;
        *notice = why;
        git::worktree_add_from(
            &project.path,
            &project.worktrees_root(),
            held,
            &project.agent_branch_prefix,
            name,
            from.as_deref(),
        )
        .await
    }

    /// The worktree of an agent on the branch `name`, which exists (a local one, or a remote one
    /// through the local branch that tracks it, made when there is none): (path, local branch,
    /// base). Refused for the board's target branch (the tickets are merged into it), for the
    /// branch of the project's folder and for one another worktree has (`IN_WORKTREE` for an
    /// agent's); nothing is made then.
    async fn worktree_on_branch(
        &self,
        project: &Project,
        held: &[PathBuf],
        name: &str,
        base: Option<&str>,
    ) -> Result<(String, String, String)> {
        let root = self.toplevel(&project.path).await.ok_or_else(not_a_repo)?;
        // Not while a switch or a pull moves the folder's branch.
        let lock = self.sync_lock(&root);
        let _guard = lock.lock().await;
        // First: when the folder is on the target, that is the reason, not the folder.
        let target = project.board.target.trim();
        if !target.is_empty() && git::local_name(&root, name).await? == target {
            bail!(target_branch_refusal(i18n::ui()));
        }
        if let Some(local) = git::local_of(&root, name).await? {
            if self.checked_out_here(&root, &local).await? {
                bail!(folder_branch_refusal(i18n::ui()));
            }
        }
        let base = match base {
            Some(base) => base.to_string(),
            None => self.base_of(project, &root).await,
        };
        let (path, branch) =
            git::worktree_add_existing(&project.path, &project.worktrees_root(), held, name)
                .await?;
        Ok((path, branch, base))
    }

    /// The worktree of an agent on the new branch `name` (ESC-155): made from where a new agent
    /// starts (`start_point`: `over`, the branch chosen for it, else the project's « Branche de
    /// départ des nouveaux worktrees »), its folder named as for a branch that exists
    /// (`worktree_add_existing`). A name git refuses, or that a branch has, makes nothing; nor does
    /// a worktree that fails (the branch made for it is deleted). (path, branch, base).
    async fn worktree_on_new_branch(
        &self,
        project: &Project,
        held: &[PathBuf],
        name: &str,
        over: Option<&str>,
        notice: &mut Option<String>,
    ) -> Result<(String, String, String)> {
        let root = self.toplevel(&project.path).await.ok_or_else(not_a_repo)?;
        git::check_new_branch(&root, name).await?;
        // Not while a switch or a pull moves the branches.
        let lock = self.sync_lock(&root);
        let _guard = lock.lock().await;
        let (from, why) = start_point(project, &root, over).await;
        *notice = why;
        let base = match &from {
            Some(b) => b.clone(),
            None => git::head_branch(&root).await,
        };
        if base.is_empty() || base == "HEAD" {
            bail!(tr!(
                "le dépôt n'a pas encore de commit : impossible de créer un worktree",
                "the repository has no commit yet: a worktree can’t be created"
            ));
        }
        let start = from.map_or_else(|| "HEAD".to_string(), |b| format!("refs/heads/{b}"));
        git::branch_create(&root, name, &start).await?;
        match git::worktree_add_existing(&project.path, &project.worktrees_root(), held, name).await
        {
            Ok((path, branch)) => Ok((path, branch, base)),
            Err(e) => {
                let _ = git::branch_delete(&root, name, true).await;
                Err(e)
            }
        }
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
        let wt = wt.ok_or_else(agent_without_worktree)?;
        let project = self.project(&pid)?;
        // Of several repositories (ESC-169): each one's, checked first, all or nothing.
        if wt.is_multi() {
            let out = self
                .merge_multi(&project, &name, &wt, squash, switch_to_base, None)
                .await;
            self.git.refresh(&pid);
            return out;
        }
        if git::has_tracked_changes_but_tickets(&project.path).await? {
            bail!(tr!(
                "Le dépôt principal a des modifications non commitées : commite-les ou mets-les de côté avant de merger.",
                "The main repository has uncommitted changes: commit or stash them before merging."
            ));
        }
        let dirty = git::status(&wt.path).await?.entries.len();
        if dirty > 0 {
            bail!(uncommitted_in_agent(i18n::ui(), dirty));
        }
        if !git::branch_exists(&project.path, &wt.base_branch).await {
            bail!(tr!(
                "La branche de base « {base} » n'existe plus : « {branch} » ne peut pas être mergée dedans.",
                "The base branch “{base}” no longer exists: “{branch}” can’t be merged into it.",
                base = wt.base_branch,
                branch = wt.branch
            ));
        }
        // Counted from the base, not from HEAD: the folder may be on another branch.
        if git::ahead_of(&project.path, &wt.base_branch, &wt.branch).await? == 0 {
            bail!(tr!(
                "Rien à merger : la branche {b} n'a pas de nouveau commit.",
                "Nothing to merge: the branch {b} has no new commit.",
                b = wt.branch
            ));
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

    /// Adds the folder as a project (`add_project`), whichever history it brings back: the way the
    /// tests add one (the window's command is told whether the project is a new one).
    #[cfg(test)]
    pub async fn create_project(
        self: &Arc<Self>,
        path: &str,
        name: &str,
        color: &str,
        worktree_per_agent: bool,
        first_agent: Option<String>,
        restore: bool,
    ) -> Result<Project> {
        let added = self
            .add_project(
                path,
                name,
                color,
                worktree_per_agent,
                first_agent,
                restore,
                None,
            )
            .await?;
        Ok(added.0)
    }

    /// Adds the folder as a project. With `restore`, the history of a project closed in this folder
    /// comes back with it, same id (`closed`): the one `history_id` names if it is one of this
    /// folder, else the latest closed; else, or when there is none, it is a new project. True
    /// when a history came back (the project is not a new one).
    #[allow(clippy::too_many_arguments)]
    pub async fn add_project(
        self: &Arc<Self>,
        path: &str,
        name: &str,
        color: &str,
        worktree_per_agent: bool,
        first_agent: Option<String>,
        restore: bool,
        history_id: Option<String>,
    ) -> Result<(Project, bool)> {
        let path = path.trim().trim_end_matches(['\\', '/']).to_string();
        if !Path::new(&path).is_dir() {
            bail!(tr!(
                "Le dossier {path} n'existe pas",
                "The folder {path} doesn’t exist"
            ));
        }
        // The history a project closed in this folder kept comes back with it, same id. Looked for
        // before the awaits below (the memory answers at once, the disk is asked only for another
        // spelling of the folder): the window asking twice is told of the project the first asking
        // brings back, whichever way the two interleave.
        let entry = if restore {
            self.closed_entry_for(&path, history_id.as_deref()).await
        } else {
            None
        };
        // Its file is read off the runtime's threads. Gone by now, if the first asking made the
        // project and settled the file already: the open project answers below.
        let closed = match &entry {
            Some(entry) => {
                let (data, id) = (self.data.clone(), entry.id.clone());
                tokio::task::spawn_blocking(move || closed::read_history(&data, &id))
                    .await
                    .ok()
                    .flatten()
            }
            None => None,
        };
        // A folder of repositories (ESC-169) is not made one: its repositories are its git.
        let several = !crate::repos::detect(Path::new(&path), &paths::worktrees_root(&path, ""))
            .await
            .is_empty();
        if !several && git::toplevel(&path).await.is_none() {
            git::init_repo(&path).await.context("git init")?;
        }
        let fresh = Project {
            id: new_id(),
            name: if name.trim().is_empty() {
                Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| tr!("projet", "project"))
            } else {
                name.trim().to_string()
            },
            color: color.to_string(),
            worktree_per_agent,
            created_at: now_ms(),
            // « Nouveaux projets privés »: a history brought back keeps its own.
            private: self.settings.read().private_projects,
            ..blank_project(&path)
        };
        let accounts: Vec<String> = self
            .settings
            .read()
            .accounts
            .iter()
            .map(|a| a.id.clone())
            .collect();
        let (project, back) = {
            let mut projects = self.projects.write();
            // The history's project is open by now (the window asked twice): it is the answer,
            // not a blank project beside it.
            let open = entry
                .as_ref()
                .and_then(|e| projects.iter().find(|p| p.id == e.id));
            if let Some(open) = open {
                return Ok((open.clone(), true));
            }
            let project = match &closed {
                Some(c) => c.reopened(&path, name, color, worktree_per_agent, &accounts),
                None => fresh,
            };
            projects.push(project.clone());
            (project, closed)
        };
        let reopened = back.is_some();
        let ticket_ids: Vec<String> = back
            .iter()
            .flat_map(|c| c.tickets.iter().map(|t| t.id.clone()))
            .collect();
        let remote = match back {
            Some(closed) => self.give_back(closed, &project),
            None => Vec::new(),
        };
        {
            let mut ui = self.ui.write();
            ui.active_project = Some(project.id.clone());
            ui.view = "project".into();
        }
        if reopened {
            // Saved before its file goes, and only if it is: a stop in between, or a state that
            // cannot be written, loses nothing (the file is ignored while its project is open,
            // and gives back what the state lacks at the next start).
            self.note_leftover(&project.id);
            match self.save_state() {
                Ok(()) => self.settle_history_file(&project.id),
                Err(e) => {
                    log::error!("history of project {} kept: {e:#}", project.id);
                    self.closed_index.lock().remove(&project.id);
                    self.request_save();
                }
            }
        } else {
            self.request_save();
        }
        // Its `.escouade.yml` sets it up, and says so (ESC-146): before its folders are watched, the
        // worktrees' one may come from it.
        let project = self.read_shared(&project.id, true).unwrap_or(project);
        // Its repositories, if it has several (ESC-169): found before anything reads its git.
        let _ = self.refresh_repos(&project.id, true).await;
        let project = self.project(&project.id).unwrap_or(project);
        self.watch_project(&project);
        // « Index du code sur tous les projets » on and followed: read like the others (ESC-293).
        if self.code_index_on(&project) {
            self.queue_code_index(vec![project.id.clone()]);
        }
        if reopened {
            // Its tickets may start, its agents are counted, its Remote Control ones are
            // reachable again as they are when the app starts.
            self.schedule();
            self.update_tray();
            self.start_remote(remote);
            // The syncs its tickets left waiting while it was closed go again.
            self.retry_syncs_of(&ticket_ids);
        }
        self.facts.emit(
            crate::facts::Body::ProjectAdded {
                path: project.path.clone(),
            },
            crate::facts::About::project(&project.id),
        );
        // The project is made whatever becomes of its first agent: told (no account serves a new
        // project, say), never an error that would have the window ask, and make it, again.
        if let Some(model) = first_agent {
            if let Err(e) = self.create_agent(&project.id, Some(model)).await {
                log::warn!("first agent of {}: {e:#}", project.name);
                // For want of an account: with « Ajouter un compte… » (ESC-128).
                self.hub.emit(UiEvent::Toast {
                    text: accounts::marked(&e, e.to_string()),
                });
            }
        }
        Ok((project, reopened))
    }

    pub fn update_project(self: &Arc<Self>, p: Project) -> Result<()> {
        // Where its worktrees go and how their branches are named: refused as the window does.
        let worktree_dir = p.worktree_dir.trim().to_string();
        let agent_prefix = p.agent_branch_prefix.trim().to_string();
        let ticket_prefix = p.ticket_branch_prefix.trim().to_string();
        let docs_branch = p.docs_branch.trim().to_string();
        let path = self.project(&p.id)?.path;
        if let Some(why) = paths::worktrees_root_problem(&path, &worktree_dir)
            .or_else(|| crate::docsbranch::branch_problem(&docs_branch))
            .or_else(|| git::branch_prefix_problem(&agent_prefix))
            .or_else(|| git::branch_prefix_problem(&ticket_prefix))
        {
            bail!(why);
        }
        // Its preferred account is one there is (the window's copy may be older than the
        // accounts' tab); an account switched off is kept, and ignored while it is.
        let account = {
            let settings = self.settings.read();
            let known = settings.accounts.iter().any(|a| a.id == p.account);
            if known {
                p.account.clone()
            } else {
                String::new()
            }
        };
        let mut projects = self.projects.write();
        let cur = projects
            .iter_mut()
            .find(|x| x.id == p.id)
            .ok_or_else(project_not_found)?;
        let before = cur.clone();
        let account_changed = cur.account != account;
        cur.docs_branch = docs_branch;
        let remote_changed = cur.remote_control != p.remote_control;
        let renamed = cur.name != p.name;
        cur.remote_control = p.remote_control;
        cur.name = p.name;
        cur.color = p.color;
        // Its logo is `set_project_logo`'s: the window's copy may be older.
        cur.tile_letters = crate::projectlogo::clean_letters(&p.tile_letters);
        cur.tile_label = p.tile_label;
        cur.tile_label_text = crate::projectlogo::clean_label(&p.tile_label_text);
        cur.worktree_per_agent = p.worktree_per_agent;
        cur.worktree_at_first_message = p.worktree_at_first_message;
        cur.worktree_base = p.worktree_base;
        cur.run_commands = p.run_commands;
        cur.worktree_copy = p.worktree_copy;
        cur.worktree_setup = p.worktree_setup;
        cur.worktree_teardown = p.worktree_teardown;
        cur.commit_mode = p.commit_mode;
        cur.account = account;
        cur.engine = p.engine;
        // « Projet privé »: the MCP server reads it at each call (`mcp::resolve::Sight`), the
        // harnesses at each start of an agent, before its next turn (`fence`).
        cur.private = p.private;
        cur.shared_on_limit = p.shared_on_limit;
        // Its workflows' corrections and when they are read again (ESC-195).
        cur.workflow_fixes = crate::workflowfix::cleaned(p.workflow_fixes);
        cur.workflows_auto_read = p.workflows_auto_read;
        cur.workflows_enabled = p.workflows_enabled;
        let moved_from = (cur.workflows_in_repo != p.workflows_in_repo)
            .then(|| crate::workflowfiles::places(cur, &self.data).repo);
        cur.workflows_in_repo = p.workflows_in_repo;
        let dir_changed = cur.worktree_dir != worktree_dir;
        cur.worktree_dir = worktree_dir;
        cur.agent_branch_prefix = agent_prefix;
        cur.ticket_branch_prefix = ticket_prefix;
        // What was imported is the backend's own: the window's copy may be older.
        let imported = std::mem::take(&mut cur.integrations.imported);
        cur.integrations = crate::integrations::checked_links(p.integrations);
        cur.integrations.imported = imported;
        // What the window set of its `.escouade.yml`: kept on this machine, ahead of the file.
        let kept = crate::projectfile::keep_local(&before, cur);
        let updated = cur.clone();
        drop(projects);
        self.request_save();
        if !kept.is_empty() {
            self.kept_on_machine(&updated, kept.len());
        }
        // Its next worktrees go elsewhere, maybe out of its folder: watched there too.
        if dir_changed {
            self.watch_project(&updated);
        }
        // Its workflows kept elsewhere, or corrected otherwise: their files follow, and its agents
        // are followed in them at once (ESC-195).
        if let Some(from) = moved_from {
            let to = crate::workflowfiles::places(&updated, &self.data).repo;
            if let Err(e) = crate::workflowfiles::move_all(&from, &to) {
                log::warn!("workflows of {} moved: {e:#}", updated.id);
            }
        }
        if before.workflow_fixes != updated.workflow_fixes
            || before.workflows_in_repo != updated.workflows_in_repo
            || before.workflows_enabled != updated.workflows_enabled
        {
            self.write_workflow_fixes(&updated);
            self.workflow_fixes_changed(&updated.id);
        }
        // The account its tickets go to may have changed, and so what holds them back.
        if account_changed {
            self.schedule();
        }
        // Its agents get Remote Control or lose it, on tasks of their own.
        if remote_changed {
            self.apply_remote_to(Some(&updated.id));
        }
        // Their sessions on claude.ai are called « projet · agent ».
        if renamed {
            self.rename_remote_sessions(&updated.id);
        }
        Ok(())
    }

    /// « Logo » (ESC-167): the image of the file `source` copied under the project in place of its
    /// letters, or (None) no logo. A file refused (too heavy, unreadable, of another format) leaves
    /// the project as it was. Returns the project.
    /// A project's shared memory changed (ESC-255): the window reads it again.
    pub fn memory_changed(&self, project_id: &str) {
        self.hub.emit(UiEvent::Memory {
            project_id: project_id.to_string(),
        });
    }

    /// The memories of the project's shared memory, by name.
    pub fn memories(&self, id: &str) -> Result<Vec<crate::memory::Memory>> {
        crate::memory::list(&crate::memory::dir_of(&self.project(id)?))
    }

    /// « Oublier » of the settings: a memory's file removed.
    pub fn forget_memory(&self, id: &str, name: &str) -> Result<()> {
        if !crate::memory::forget(&crate::memory::dir_of(&self.project(id)?), name)? {
            bail!(tr!(
                "Aucun souvenir « {name} » dans la mémoire du projet.",
                "No memory “{name}” in the project's memory."
            ));
        }
        self.facts.emit(
            crate::facts::Body::MemoryForgotten {
                memory: name.to_string(),
            },
            crate::facts::About::project(id),
        );
        self.memory_changed(id);
        Ok(())
    }

    /// The folders Claude Code keeps the memory of `p` in: one under each Claude account's configuration folder
    /// (`accounts::config_dir`, never the machine user's own in the tests). Read, never written.
    fn claude_memory_dirs(&self, p: &Project) -> Vec<PathBuf> {
        let settings = self.settings.read().clone();
        let mut out: Vec<PathBuf> = Vec::new();
        for a in &settings.accounts {
            let Some(dir) = crate::engine::by_id(a.engine)
                .memory_dir(&accounts::config_dir(a, &settings), &p.path)
            else {
                continue;
            };
            if !out.contains(&dir) {
                out.push(dir);
            }
        }
        out
    }

    /// What Claude Code remembers of the project, offered once to its shared memory: nothing once it was offered
    /// (taken or not), nor when Claude Code has none.
    pub fn claude_memory_offer(&self, id: &str) -> Result<Vec<crate::memory::Offered>> {
        let p = self.project(id)?;
        if p.claude_memory_asked {
            return Ok(Vec::new());
        }
        Ok(crate::memory::claude_offer(
            &self.claude_memory_dirs(&p),
            &crate::memory::dir_of(&p),
        ))
    }

    /// « Reprendre » of the window: the memories of Claude Code the user ticked, written into the project's; the
    /// offer is not made again.
    pub fn claude_memory_take(&self, id: &str, names: &[String]) -> Result<Project> {
        let p = self.project(id)?;
        if !names.is_empty() {
            let offered = crate::memory::claude_offer(
                &self.claude_memory_dirs(&p),
                &crate::memory::dir_of(&p),
            );
            crate::memory::claude_take(&offered, names, &crate::memory::dir_of(&p))?;
            self.memory_changed(id);
        }
        self.claude_memory_asked(id)
    }

    /// The offer of Claude Code's memory made: never again for this project.
    pub fn claude_memory_asked(&self, id: &str) -> Result<Project> {
        let updated = {
            let mut projects = self.projects.write();
            let cur = projects
                .iter_mut()
                .find(|x| x.id == id)
                .ok_or_else(project_not_found)?;
            cur.claude_memory_asked = true;
            cur.clone()
        };
        self.request_save();
        Ok(updated)
    }

    pub fn set_project_logo(&self, id: &str, source: Option<&str>) -> Result<Project> {
        self.project(id)?;
        let file = match source {
            Some(path) => {
                crate::projectlogo::import(i18n::ui(), &self.data, id, Path::new(path.trim()))?
            }
            None => String::new(),
        };
        let updated = {
            let mut projects = self.projects.write();
            let Some(cur) = projects.iter_mut().find(|x| x.id == id) else {
                drop(projects);
                // Closed meanwhile: its copy goes with it.
                crate::projectlogo::remove(&self.data, id);
                return Err(project_not_found());
            };
            cur.logo = file;
            cur.clone()
        };
        if updated.logo.is_empty() {
            crate::projectlogo::clear(&self.data, id);
        }
        self.request_save();
        Ok(updated)
    }

    /// The project's logo as a `data:` URL, for its tile; None when it has none (or its copy is
    /// gone).
    pub fn project_logo(&self, id: &str) -> Option<String> {
        let logo = self.project(id).ok()?.logo;
        (!logo.is_empty())
            .then(|| crate::projectlogo::url_of(&self.data, id, &logo))
            .flatten()
    }

    /// The sessions on claude.ai of the project's agents linked through their stream's process
    /// get their name again (« projet · agent »), on a task of their own. The interfaces in the
    /// Claude Code view keep the name they started with: nothing reaches them.
    fn rename_remote_sessions(&self, project_id: &str) {
        let renames: Vec<(Process, String)> = self
            .project_agents(project_id)
            .iter()
            .filter_map(|h| {
                let rt = h.lock();
                let proc = rt.proc.clone().filter(|_| rt.remote_linked)?;
                Some((proc, self.session_title(&rt)))
            })
            .collect();
        if renames.is_empty() {
            return;
        }
        tauri::async_runtime::spawn(async move {
            for (proc, title) in renames {
                if let Err(e) = proc.rename(&title).await {
                    log::warn!("session not renamed on claude.ai: {e:#}");
                }
            }
        });
    }

    pub fn reorder_projects(&self, ids: &[String]) {
        let mut projects = self.projects.write();
        projects.sort_by_key(|p| ids.iter().position(|i| *i == p.id).unwrap_or(usize::MAX));
        drop(projects);
        self.request_save();
    }

    /// Closes the project: its agents, terminals and launches stop, and it leaves the app. Its
    /// history (settings, agents, conversations, tickets, the accounts it was reserved to) is kept
    /// for the next project added in its folder (`closed`), unless `erase`: nothing of it stays.
    pub async fn remove_project(self: &Arc<Self>, id: &str, erase: bool) -> Result<()> {
        // One closing at a time: a second one would find the project half gone and write its
        // history again with what is left of it.
        let Some(_closing) = closed::Closing::start(&self.closing, id) else {
            return Ok(());
        };
        // A validation under way (its tests, its merge) is not cut half way: the closing waits
        // for its end. Looked for once the project is being closed, which no validation starts on
        // (`Refusal::ProjectClosing`): one that began before is seen, one that begins is refused.
        let approving = self
            .tickets
            .read()
            .iter()
            .find(|t| t.project_id == id && t.step.is_some())
            .map(|t| t.key.clone());
        if let Some(key) = approving {
            return Err(closed::approval_under_way(self.lang().ui, &key));
        }
        let project = self.project(id).ok();
        if let Some(p) = &project {
            // Said with its name and whether it is private: it is gone by the time the bus reads them.
            self.facts.emit(
                crate::facts::Body::ProjectClosed {},
                crate::facts::About {
                    project: Some(crate::facts::Named {
                        id: p.id.clone(),
                        name: p.name.clone(),
                    }),
                    private: Some(p.private),
                    ..Default::default()
                },
            );
        }
        let selected = self.ui.read().selected_agent.get(id).cloned();
        // Its history written before anything stops, as it is now, and again once all stopped
        // (below): an app stopped in between, its state saved without the project, still has it.
        // Nowhere to write it: the project stays open.
        if let (false, Some(p)) = (erase, &project) {
            let agents = self
                .project_agents(id)
                .iter()
                .map(|h| h.lock().meta.clone())
                .collect();
            let tickets = self
                .tickets
                .read()
                .iter()
                .filter(|t| t.project_id == id)
                .cloned()
                .collect();
            let first = self.write_history(p.clone(), agents, tickets, selected.clone());
            if let Err(e) = first {
                log::error!("history of project {id} not written, not closed: {e:#}");
                return Err(closed::history_refused(&e));
            }
        }
        let agents: Vec<String> = self
            .project_agents(id)
            .iter()
            .map(|h| h.lock().meta.id.clone())
            .collect();
        let mut interfaces = Vec::new();
        let mut kept: Vec<AgentMeta> = Vec::new();
        for aid in agents.iter().cloned() {
            // Wait for an in-flight start (a warm-up), as the deletion of an agent does: it would
            // otherwise start a process, with a token, for an agent that is gone.
            let lock = self.spawn_lock(&aid);
            let _guard = lock.lock().await;
            // Nor out in the middle of its naming (`apply_name_to`): the name and the branch it is
            // giving it are both set first, and kept; a name that comes later finds it gone and
            // sets nothing. Taken after its start's lock, as a start that makes its worktree does.
            let naming = self.spawn_lock(&format!("naming-{id}"));
            let _naming = naming.lock().await;
            // Bound first: the map guard must not live across the agent lock and the I/O below.
            let removed = self.agents.write().remove(&aid);
            let mut worktree = None;
            if let Some(h) = removed {
                let mut rt = h.lock();
                rt.gen += 1;
                let mut at_work = rt.meta.status.is_active() || rt.waiting_views().next().is_some();
                if let Some(p) = rt.proc.take() {
                    p.kill();
                    at_work = true;
                }
                // Its Claude Code interface is no terminal of the project's (it quits below).
                if let Some(t) = rt.tui.take() {
                    interfaces.push((t.term_id, crate::tui::quit_key(&rt.meta)));
                    at_work = true;
                }
                // What its turn left open is closed in the conversation the history keeps (its
                // process' exit, made stale above, no longer does it): a question or a permission
                // cancelled, a tool interrupted, a text no longer streaming. Only for an agent
                // that could have a turn open: finding out reads its whole log.
                if !erase && at_work {
                    let mut fx = Effects::default();
                    rt.clear_pending(&mut fx);
                    rt.close_open_items(&mut fx);
                }
                // Its log is let go of; the history keeps it, or erases it below.
                rt.conv.close_log();
                worktree = rt.meta.worktree.clone();
                kept.push(rt.meta.clone());
            }
            self.mcp.forget_agent(&aid);
            self.hooks.revoke(&aid);
            self.spawn_locks.lock().remove(&aid);
            self.quit_locks.lock().remove(&aid);
            self.remote_locks.lock().remove(&aid);
            // The setup of its worktree stops, with what it started (its log stays with the
            // history), and the images pasted in its interface go.
            if let Some(s) = self.setups.lock().remove(&aid) {
                s.task.abort();
            }
            self.forget_pastes(&aid);
            self.forget_sidechains(&aid);
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
        // The interfaces of its agents quit as their users would, all together.
        let quits: Vec<_> = interfaces
            .into_iter()
            .map(|(t, key)| {
                let pty = self.pty.clone();
                tauri::async_runtime::spawn(async move {
                    crate::tui::quit_interface(&pty, &t, key).await
                })
            })
            .collect();
        for quit in quits {
            let _ = quit.await;
        }
        self.pty.kill_project(id);
        self.git.unwatch(id);
        // The project as it is now, out of the list in one go: a ticket created or a setting
        // changed while the closing waited for a start above is in it, not in the copy read first.
        let latest = {
            let mut projects = self.projects.write();
            let at = projects.iter().position(|p| p.id == id);
            at.map(|at| projects.remove(at))
        };
        // No account stays kept to a project that is gone for good (the window could not untick
        // it): once it is out, so that an app stopped before still has it reserved where it is
        // open. One whose history is kept stays in their lists, until that is erased.
        if erase {
            self.forget_project_in_accounts(id);
        }
        // After the project is out: `ticket_create` checks the project under the projects' read
        // lock, which it holds while it adds the ticket, so a ticket it adds before this is dropped
        // here, and none is added after.
        let tickets = self.drop_project_tickets(id);
        // Recomputed from the files: not kept with a closed project.
        self.codeindex.delete(id);
        if erase {
            // Nothing of it stays, not even a history a stop left half way (see `closed_list`).
            let tickets: Vec<String> = tickets.iter().map(|t| t.id.clone()).collect();
            self.erase_history_files(&agents, &tickets);
            crate::projectlogo::remove(&self.data, id);
            if !self.forget_closed_file(id) {
                // Not listed again, tried again at the next start.
                self.note_leftover(id);
            }
        } else if let Some(project) = latest.or(project) {
            self.keep_history(project, kept, tickets, selected);
        }
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
        // When each project may be read again (`git::Pace`).
        let mut last: HashMap<String, Instant> = HashMap::new();
        let mut pace = crate::git::Pace::default();
        loop {
            let next = due.values().min().copied();
            tokio::select! {
                msg = rx.recv() => {
                    let Some(pid) = msg else { break };
                    let now = Instant::now();
                    let earliest = last.get(&pid).copied().unwrap_or(now);
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
                        let (wait, busy) = pace.read(&pid, now);
                        if let Some(line) = busy {
                            log::warn!("{line}");
                        }
                        last.insert(pid.clone(), now + wait);
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
        // Its folder changed: its `.escouade.yml` too, maybe (a pull, another branch).
        self.read_shared(project_id, false);
        // Its shared tickets' files too (ESC-190): read again, a teammate's change followed.
        self.sync_shared(project_id);
        // And its workflows' (ESC-195).
        self.sync_workflow_files(project_id);
        let Ok(project) = self.project(project_id) else {
            return;
        };
        // The branch each agent's worktree is really on: one the agent switched to by itself is
        // followed (`follow_agent_branch`).
        let mut heads: Vec<(String, String)> = Vec::new();
        let info = match self.toplevel(&project.path).await {
            // Of several repositories (ESC-169): each one's.
            _ if project.is_multi() => self.multi_git(&project).await,
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
                type Row = (String, Option<Worktree>, Option<String>, Vec<String>);
                let agents: Vec<Row> = self
                    .project_agents(project_id)
                    .iter()
                    .map(|h| {
                        let rt = h.lock();
                        (
                            rt.meta.id.clone(),
                            rt.meta.worktree.clone(),
                            rt.meta.found_worktree.as_ref().map(|f| f.path.clone()),
                            rt.meta.touched_files.clone(),
                        )
                    })
                    .collect();
                for (aid, wt, found, touched) in agents {
                    // The worktree it went into by itself (ESC-161): its branch, its files being
                    // counted in the project's folder as they were.
                    if let (None, Some(found)) = (&wt, found) {
                        if let Ok(fs) = git::status(&found).await {
                            info.agent_git.insert(
                                aid.clone(),
                                crate::agentbranch::agent_git_of(&fs, info.has_remote),
                            );
                        }
                    }
                    let count = match wt {
                        Some(wt) => match git::status(&wt.path).await {
                            Ok(ws) => {
                                for e in &ws.entries {
                                    tally(e.status, &mut info);
                                }
                                info.agent_git.insert(
                                    aid.clone(),
                                    crate::agentbranch::agent_git_of(&ws, info.has_remote),
                                );
                                heads.push((aid.clone(), ws.branch.clone()));
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
        for (aid, head) in heads {
            self.follow_agent_branch(&aid, &head).await;
        }
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

    /// Dirty files for the files panel. `agent_id` + scope "agent" restricts to one agent. A
    /// worktree git cannot read lists nothing here (the panel shows the others); see
    /// `git_files_checked`.
    pub async fn git_files(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<Vec<FileChange>> {
        self.dirty_files(project_id, agent_id, false).await
    }

    /// `git_files`, but a worktree git cannot read is git's error: for a commit, which must not
    /// take a failure for « nothing to commit ».
    async fn git_files_checked(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<Vec<FileChange>> {
        self.dirty_files(project_id, agent_id, true).await
    }

    async fn dirty_files(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        strict: bool,
    ) -> Result<Vec<FileChange>> {
        let project = self.project(project_id)?;
        if project.is_multi() {
            return self.multi_dirty_files(&project, agent_id, strict).await;
        }
        let root = self.toplevel(&project.path).await.ok_or_else(not_a_repo)?;
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
                let changes = match git::file_changes(&wt.path).await {
                    Ok(changes) => changes,
                    Err(e) if strict => return Err(e),
                    Err(_) => Vec::new(),
                };
                for mut f in changes {
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
            None => self.toplevel(&project.path).await.ok_or_else(not_a_repo),
        }
    }

    /// The folder the editor shows for `agent_id`: its worktree (with the branch it left), else
    /// the project's repository, else the project's folder when it is not one. Refused for an
    /// agent of another project, and for a worktree that is no longer there.
    pub async fn edit_root(
        &self,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<(String, Option<String>)> {
        let project = self.project(project_id)?;
        if let Some(a) = &agent_id {
            let h = self.agent(a)?;
            let (owner, worktree) = {
                let rt = h.lock();
                (rt.meta.project_id.clone(), rt.meta.worktree.clone())
            };
            // Its worktree is not this project's to edit, nor to open a shell in.
            if owner != project_id {
                bail!(tr!(
                    "agent introuvable dans ce projet",
                    "agent not found in this project"
                ));
            }
            if let Some(wt) = worktree {
                // Removed after a pull request or a push: said so, not in the system's words for a
                // missing folder (and a shell started there would open in the app's own folder, as
                // if it were the agent's).
                if !Path::new(&wt.path).is_dir() {
                    bail!(tr!(
                        "Le worktree de l'agent n'existe plus",
                        "The agent’s worktree no longer exists"
                    ));
                }
                return Ok((wt.path, Some(wt.base_branch)));
            }
        }
        let root = self.toplevel(&project.path).await.unwrap_or(project.path);
        Ok((root, None))
    }

    /// `edit_root`, with the folders of it that hold the agents' worktrees (see `worktrees_kept`):
    /// the editor makes nothing in them, and its search leaves them out.
    pub async fn edit_worktrees(
        &self,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<(String, Vec<String>)> {
        let (root, base) = self.edit_root(project_id, agent_id).await?;
        let project = self.project(project_id)?;
        let own = (!project.worktree_dir.trim().is_empty()).then(|| project.worktrees_root());
        let kept = worktrees_kept(&root, &project.path, base.is_some(), own.as_deref());
        Ok((root, kept))
    }

    /// The folders of the worktrees of every agent of the app, there or not: a new worktree never
    /// takes one (`git::worktree_add`), the agent's own coming back there when it is restored.
    fn held_worktrees(&self) -> Vec<PathBuf> {
        self.agents
            .read()
            .values()
            .filter_map(|h| {
                h.lock()
                    .meta
                    .worktree
                    .as_ref()
                    .map(|w| PathBuf::from(&w.path))
            })
            .collect()
    }

    /// The folders whose changes refresh the git state of `project`: its own, and those holding
    /// its agents' worktrees out of it (its « Dossier des worktrees » beside it), which a watch of
    /// its folder does not see.
    pub(crate) fn watched_folders(&self, project: &Project) -> Vec<String> {
        let mut holders = vec![project.worktrees_root()];
        holders.extend(self.project_agents(&project.id).iter().filter_map(|h| {
            let rt = h.lock();
            let wt = rt.meta.worktree.as_ref()?;
            Path::new(&wt.path).parent().map(Path::to_path_buf)
        }));
        let mut dirs = vec![project.path.clone()];
        for d in holders {
            let d = d.to_string_lossy().into_owned();
            if Path::new(&d).is_dir() && !dirs.iter().any(|w| self.git_watches_in(w, &d)) {
                dirs.push(d);
            }
        }
        dirs
    }

    /// `dir` is `watched`, or in it.
    fn git_watches_in(&self, watched: &str, dir: &str) -> bool {
        crate::tickets::same_dir(watched, dir) || paths::strip_base(watched, dir).is_some()
    }

    /// The watch of `project`'s folder sees the worktree at `path`.
    fn git_watches(&self, project: &Project, path: &str) -> bool {
        self.git_watches_in(&project.path, path)
    }

    /// Watches what refreshes the git state of `project` (`watched_folders`), in place of what
    /// was watched for it.
    pub(crate) fn watch_project(&self, project: &Project) {
        self.git.watch(&project.id, &self.watched_folders(project));
    }

    /// `edit_root`, with the folders of it that the editor's renames and deletions keep away from:
    /// those holding the agents' worktrees (`edit_worktrees`), every worktree of the repository
    /// (the agents' of every project sharing it, a validation's), every agent's, and every
    /// project's folder (renamed, its project would point at nothing).
    pub async fn edit_kept(
        &self,
        project_id: &str,
        agent_id: Option<String>,
    ) -> Result<(String, Vec<fsedit::Kept>)> {
        let (root, worktrees) = self.edit_worktrees(project_id, agent_id).await?;
        let kept_as = |what| move |path| fsedit::Kept { path, what };
        let mut kept: Vec<fsedit::Kept> = worktrees
            .into_iter()
            .map(kept_as(fsedit::Holds::Worktrees))
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
            (worktrees, fsedit::Holds::Worktree),
            (projects, fsedit::Holds::Project),
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
        // A base branch says the root is a worktree's (refused when it is gone).
        let (root, base) = self.edit_root(project_id, agent_id).await?;
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
            bail!(tr!(
                "Le dossier « {sub} » n'existe pas",
                "The folder “{sub}” doesn’t exist"
            ));
        }
        Ok(dir.to_string_lossy().into_owned())
    }

    /// What « Afficher dans l'Explorateur » shows of an editor source: the file or folder `path` of
    /// its tree selected, or, without one, the source's folder opened, where a terminal opens (the
    /// agent's worktree, else the project's folder). Refused for a worktree no longer there.
    pub async fn reveal_target(
        &self,
        project_id: &str,
        agent_id: Option<String>,
        path: Option<&str>,
    ) -> Result<fsedit::Reveal> {
        let (root, base) = self.edit_root(project_id, agent_id).await?;
        let folder = match base {
            Some(_) => root.clone(),
            None => self.project(project_id)?.path,
        };
        // A name is taken as written: one can start or end with a space.
        let rel = path.filter(|p| !p.trim().is_empty()).map(str::to_string);
        // The disk is looked at off the async workers: a `\\wsl.localhost` path wakes its
        // distribution up.
        tokio::task::spawn_blocking(move || match rel {
            Some(rel) => fsedit::reveal_path(Path::new(&root), &rel).map(fsedit::Reveal::Select),
            None => fsedit::reveal_folder(PathBuf::from(folder)),
        })
        .await?
    }

    pub async fn git_diff(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        paths: Vec<String>,
    ) -> Result<String> {
        let project = self.project(project_id)?;
        if let Some(checkouts) = self.multi_checkouts(&project, agent_id.as_deref())? {
            return Self::multi_diff(&checkouts, &paths).await;
        }
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
                let Some(diff) = self.worktree_diff(agent_id.as_deref()).await? else {
                    continue;
                };
                diff
            } else {
                // As for a worktree, no path to list for the checkout when no agent is credited
                // with some of its files; else each group lists its own paths.
                let listed: &[String] = if checkout_groups == 1 { &[] } else { &paths };
                let project = self.project(project_id)?;
                if project.is_multi() {
                    Self::multi_diff(&project.checkouts(), &paths).await?
                } else {
                    git::diff(&self.files_root(project_id, None).await?, listed).await?
                }
            };
            out.push(OwnedDiff {
                agent_id,
                in_worktree,
                diff,
            });
        }
        Ok(out)
    }

    /// The diff of an agent's worktree group of `git_project_diff`: all of the worktree's dirty
    /// files. None for an agent deleted (or its worktree or its folder removed) since the list was
    /// made: it has no file left to show, and the project's checkout must not be read in its
    /// place. Err when git cannot read the worktree: an empty diff would leave its files out
    /// without a word.
    pub(crate) async fn worktree_diff(&self, agent_id: Option<&str>) -> Result<Option<String>> {
        let worktree = agent_id.and_then(|a| self.agent(a).ok()).and_then(|h| {
            let rt = h.lock();
            let name = rt.meta.name.clone();
            rt.meta.worktree.clone().map(|w| (name, w))
        });
        let Some((name, worktree)) = worktree else {
            return Ok(None);
        };
        // Its folder gone in the meantime (a validation, an archive): no file left to show either.
        if !Path::new(&worktree.path).is_dir() {
            return Ok(None);
        }
        // Its group is all of the worktree's dirty files: no path to list. Said with the colon of
        // each language (the French space before it), which a context would not give.
        let diff = if worktree.is_multi() {
            Self::multi_diff(&worktree.checkouts(), &[]).await
        } else {
            git::diff(&worktree.path, &[]).await
        };
        diff.map(Some).map_err(|e| {
            anyhow!(tr!(
                "Diff du worktree de « {name} » non lu : {e:#}",
                "Diff of the worktree of “{name}” not read: {e:#}"
            ))
        })
    }

    /// Reverts a file of the files panel to HEAD (a new file is deleted).
    pub async fn git_discard(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        path: &str,
    ) -> Result<()> {
        let project = self.project(project_id)?;
        if let Some(checkouts) = self.multi_checkouts(&project, agent_id.as_deref())? {
            let (c, rest) = Self::checkout_of(&checkouts, path)?;
            git::discard(&c.dir, &rest).await?;
            self.git.refresh(project_id);
            return Ok(());
        }
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
        // Of several repositories (ESC-169): every checkout's, its paths from the project's folder.
        if let Some(checkouts) = self.multi_checkouts(&project, agent_id.as_deref())? {
            let (mut scope, unread) = self
                .multi_commit_scope(&project, &checkouts, in_worktree)
                .await?;
            // An agent without a worktree: the files it edited alone.
            if let (Some(a), false) = (&agent_id, in_worktree) {
                let touched = self.agent(a)?.lock().meta.touched_files.clone();
                scope
                    .files
                    .retain(|f| touched.iter().any(|t| t.eq_ignore_ascii_case(&f.path)));
            }
            return Ok(DirectScope {
                root: String::new(),
                scope,
                unread,
            });
        }
        let root = self.files_root(project_id, agent_id.clone()).await?;
        let listed = match agent_id {
            // The project's checkout, whoever edited its files.
            None => git::file_changes(&root).await?,
            // Its worktree, else the files it edited in the project's checkout.
            // (A worktree git cannot read is git's error, not a list of nothing.)
            Some(a) => self.git_files_checked(project_id, Some(a)).await?,
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
        // Asked of the account of the agent whose changes these are.
        let asker = agent_id.clone();
        let multi = self.multi_checkouts(&self.project(project_id)?, agent_id.as_deref())?;
        let DirectScope {
            root,
            scope,
            unread,
        } = self.commit_scope(project_id, agent_id).await?;
        let files = committed(scope, &paths);
        if files.is_empty() {
            return Err(nothing_to_commit());
        }
        // Of several repositories (ESC-169): one message per repository, in its style.
        let root = match &multi {
            Some(checkouts) => {
                let repo = &files[0].repo;
                if files.iter().any(|f| &f.repo != repo) {
                    bail!(tr!(
                        "Un message par dépôt : propose-le pour un dépôt à la fois.",
                        "One message per repository: propose it for one repository at a time."
                    ));
                }
                checkouts
                    .iter()
                    .find(|c| &c.repo == repo)
                    .map(|c| c.dir.clone())
                    .unwrap_or(root)
            }
            None => root,
        };
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
        } else if let Some(checkouts) = &multi {
            Self::multi_diff(checkouts, &read).await.unwrap_or_default()
        } else {
            git::diff(&root, &read).await.unwrap_or_default()
        };
        let lang = self.lang().claude;
        // « Comme l'interface » (the default) keeps imitating the language of the repository's
        // commits; a language named in the settings is the one asked for.
        let named = i18n::names_a_language(&self.settings.read().claude_language);
        let prompt = commit_proposal_prompt(lang, named, &subjects, &files, &diff);
        let answer = self
            .one_shot(
                &commit_proposal_system(lang),
                &prompt,
                asker.as_deref(),
                Some(project_id),
            )
            .await?;
        proposal_from_answer(&answer)
            .ok_or_else(|| anyhow!(tr!("Haiku n'a rien proposé", "Haiku proposed nothing")))
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
            bail!(tr!(
                "Écris le message du commit.",
                "Write the commit message."
            ));
        }
        // The app does not restart for an update in the middle of it (a hook may take its time).
        let _working = self.working();
        let multi = self.multi_checkouts(&self.project(project_id)?, agent_id.as_deref())?;
        let DirectScope { root, scope, .. } = self.commit_scope(project_id, agent_id).await?;
        let files: Vec<String> = committed(scope, &paths)
            .into_iter()
            .map(|f| f.path)
            .collect();
        if files.is_empty() {
            return Err(nothing_to_commit());
        }
        // Of several repositories (ESC-169): a commit in each repository of `files`, with the
        // message; one that fails stops none of the others, and is told.
        if let Some(checkouts) = multi {
            let repos: Vec<String> = checkouts.iter().map(|c| c.repo.clone()).collect();
            let (mut hashes, mut failed) = (Vec::new(), Vec::new());
            for (repo, rel) in crate::repos::by_repo(&repos, &files) {
                let Some(c) = checkouts.iter().find(|c| c.repo == repo) else {
                    continue;
                };
                match git::commit_paths(&c.dir, &rel, message).await {
                    Ok(()) => hashes.push(
                        git::text(&c.dir, &["rev-parse", "--short", "HEAD"])
                            .await
                            .unwrap_or_default(),
                    ),
                    Err(e) => failed.push(format!("{} : {e:#}", crate::repos::label(&repo))),
                }
            }
            self.git_event(project_id);
            if !failed.is_empty() {
                bail!(failed.join("\n"));
            }
            return Ok(hashes.join(", "));
        }
        let done = git::commit_paths(&root, &files, message).await;
        // Even after a refusal: a hook may have changed files (a formatter) before saying no.
        self.git_event(project_id);
        done?;
        git::text(&root, &["rev-parse", "--short", "HEAD"]).await
    }

    /// The repository graph (every branch, agents' worktree branches included) and the branch
    /// the agent works on.
    pub async fn git_log(
        self: &Arc<Self>,
        project_id: &str,
        agent_id: Option<String>,
        repo: Option<String>,
    ) -> Result<GitLog> {
        let project = self.project(project_id)?;
        let root = self.repo_root(&project, repo.as_deref()).await?;
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

    pub async fn git_show(
        self: &Arc<Self>,
        project_id: &str,
        hash: &str,
        repo: Option<String>,
    ) -> Result<String> {
        let project = self.project(project_id)?;
        let root = self.repo_root(&project, repo.as_deref()).await?;
        git::show(&root, hash).await
    }

    pub(crate) fn sync_lock(&self, root: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.sync_locks
            .lock()
            .entry(root.to_string())
            .or_default()
            .clone()
    }

    /// A git event of the project (a commit, a branch left): its code index reads what changed.
    fn git_event(&self, project_id: &str) {
        self.git.refresh(project_id);
        if let Some(me) = self.me.upgrade() {
            me.code_index_event(project_id);
        }
    }

    /// Refreshes the git state of every project in the repository at `root`.
    async fn refresh_repo(&self, root: &str) {
        let projects: Vec<(String, String)> = self
            .projects
            .read()
            .iter()
            .map(|p| (p.id.clone(), p.path.clone()))
            .collect();
        for (id, _) in projects {
            if self.project_roots_of(&id).await.iter().any(|r| r == root) {
                self.git_event(&id);
            }
        }
    }

    /// Fetches every project's repository that has a remote, one at a time and without ever
    /// asking for credentials, so that the commits to pull show up.
    pub(crate) async fn fetch_all(self: &Arc<Self>) {
        let ids: Vec<String> = self.projects.read().iter().map(|p| p.id.clone()).collect();
        let mut roots: Vec<String> = Vec::new();
        for id in ids {
            for root in self.project_roots_of(&id).await {
                if !roots.contains(&root) {
                    roots.push(root);
                }
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
    /// The project's own repository (the first of several): the tests' way in.
    #[cfg(test)]
    pub async fn git_sync(self: &Arc<Self>, project_id: &str, op: SyncOp) -> Result<String> {
        self.git_sync_in(project_id, None, op).await
    }

    /// `git_sync` of the repository `repo` of a project of several (ESC-169).
    pub async fn git_sync_in(
        self: &Arc<Self>,
        project_id: &str,
        repo: Option<&str>,
        op: SyncOp,
    ) -> Result<String> {
        let project = self.project(project_id)?;
        let root = self.repo_root(&project, repo).await?;
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

    // ---------- branches ----------

    /// The project and the repository its folder is in.
    async fn branch_repo(&self, project_id: &str, repo: Option<&str>) -> Result<(Project, String)> {
        let project = self.project(project_id)?;
        let root = self.repo_root(&project, repo).await?;
        Ok((project, root))
    }

    /// The refs a branch's work counts as merged into: the project's base (the board's target
    /// while it exists, else the branch the folder is on, HEAD when detached), and that base's
    /// upstream (a pull request merged on the remote, fetched but not pulled yet). The base's
    /// target is only read here, never fixed.
    async fn merge_bases(&self, project: &Project, root: &str) -> Vec<String> {
        let base = self.base_of(project, root).await;
        if base.is_empty() || !git::branch_exists(root, &base).await {
            return vec!["HEAD".into()];
        }
        let mut bases = vec![format!("refs/heads/{base}")];
        bases.extend(git::upstream_of(root, &base).await.map(|u| u.tracking));
        bases
    }

    /// The agents' worktrees, any project's: (agent id, agent name, folder).
    fn agent_worktrees(&self) -> Vec<(String, String, String)> {
        self.agents
            .read()
            .values()
            .filter_map(|h| {
                let rt = h.lock();
                let wt = rt.meta.worktree.as_ref()?;
                Some(
                    wt.checkouts()
                        .into_iter()
                        .map(|c| (rt.meta.id.clone(), rt.meta.name.clone(), c.dir))
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect()
    }

    /// True when the folder `root` itself has `branch` checked out; refused when another worktree
    /// has it: an agent's (`IN_WORKTREE`), or one of no agent (said in words).
    pub(crate) async fn checked_out_here(&self, root: &str, branch: &str) -> Result<bool> {
        let Some(path) = git::checkout_of(root, branch).await? else {
            return Ok(false);
        };
        if crate::tickets::same_dir(&path, root) {
            return Ok(true);
        }
        let agent = self
            .agent_worktrees()
            .into_iter()
            .find(|(_, _, wt)| crate::tickets::same_dir(wt, &path));
        if let Some((agent_id, agent, _)) = agent {
            return Err(BranchRefusal::InWorktree {
                branch: branch.to_string(),
                agent_id,
                agent,
            }
            .into());
        }
        bail!(tr!(
            "La branche « {branch} » est prise par le worktree {path}.",
            "The branch “{branch}” is checked out in the worktree {path}."
        ))
    }

    /// Refused while an agent without worktree has a turn under way (waiting for an answer
    /// included) in the repository at `root`: its folder is the checkout's. Any project's, as
    /// several may be open on one checkout (`refresh_repo` reads them the same way).
    async fn no_agent_at_work(&self, root: &str) -> Result<()> {
        let projects: Vec<(String, String)> = self
            .projects
            .read()
            .iter()
            .map(|p| (p.id.clone(), p.path.clone()))
            .collect();
        for (id, _) in projects {
            if !self.project_roots_of(&id).await.iter().any(|r| r == root) {
                continue;
            }
            let busy = self.project_agents(&id).iter().find_map(|h| {
                let rt = h.lock();
                (rt.meta.worktree.is_none() && rt.meta.status.is_active())
                    .then(|| (rt.meta.id.clone(), rt.meta.name.clone()))
            });
            if let Some((agent_id, agent)) = busy {
                return Err(BranchRefusal::AgentWorking { agent_id, agent }.into());
            }
        }
        Ok(())
    }

    /// Before the folder at `root` leaves where it is, refused (in words, before anything is put
    /// aside) in the middle of a merge, cherry-pick, revert or rebase, which a switch would
    /// scramble, and from a detached HEAD whose commits no branch has, which the switch would
    /// leave to the reflog. `head_kept`: the branch it goes to starts at HEAD, so it keeps them.
    pub(crate) async fn may_leave(&self, root: &str, head_kept: bool) -> Result<()> {
        if let Some(operation) = git::operation_in_progress(root).await {
            bail!(git::operation_refusal(i18n::ui(), operation));
        }
        if !head_kept {
            let commits = git::detached_commits(root).await?;
            if commits > 0 {
                bail!(git::detached_refusal(i18n::ui(), commits));
            }
        }
        Ok(())
    }

    /// Before the folder at `root` goes to `branch`: refused with uncommitted changes to tracked
    /// files (`DIRTY`), unless `stash`: then they are put aside in a stash named for `branch`,
    /// whose name is returned. Untracked files stay, as git carries them over (refusing what they
    /// would overwrite).
    pub(crate) async fn put_aside(
        &self,
        root: &str,
        branch: &str,
        stash: bool,
    ) -> Result<Option<Stash>> {
        if !git::has_tracked_changes(root).await? {
            return Ok(None);
        }
        if !stash {
            return Err(BranchRefusal::Dirty.into());
        }
        let message = stash_message(i18n::ui(), branch);
        let hash = git::stash_push(root, &message).await?;
        Ok(Some(Stash { message, hash }))
    }

    /// After a switch that failed (HEAD did not move): the changes put aside come back, from their
    /// own stash. When they cannot (the folder was written to meanwhile, the stash is gone), the
    /// error says where they are: the stash stays whenever git could not pop it.
    pub(crate) async fn take_back(&self, root: &str, stash: &Stash) -> Result<()> {
        git::stash_pop(root, &stash.hash).await.map_err(|cause| {
            anyhow::anyhow!(tr!(
                "Tes changements n’ont pas pu être remis en place. Ils sont dans le stash « {message} » : reprends-les avec « git stash apply {hash} ». ({cause})",
                "Your changes couldn’t be put back. They’re in the stash “{message}”: get them back with “git stash apply {hash}”. ({cause})",
                message = stash.message,
                hash = stash.hash,
                cause = format!("{cause:#}").trim()
            ))
        })
    }

    /// The project's branches, local then remote, each one an agent's worktree holds with that
    /// agent.
    /// The project's own repository (the first of several): the tests' way in.
    #[cfg(test)]
    pub async fn branches(&self, project_id: &str) -> Result<Vec<git::BranchInfo>> {
        self.branches_in(project_id, None).await
    }

    pub async fn branches_in(
        &self,
        project_id: &str,
        repo: Option<&str>,
    ) -> Result<Vec<git::BranchInfo>> {
        let (_, root) = self.branch_repo(project_id, repo).await?;
        let mut list = git::branch_list(&root).await?;
        let worktrees = self.agent_worktrees();
        for b in &mut list {
            let Some(path) = &b.worktree else { continue };
            b.agent = worktrees
                .iter()
                .find(|(_, _, wt)| crate::tickets::same_dir(wt, path))
                .map(|(id, _, _)| id.clone());
        }
        Ok(list)
    }

    /// Checks a name for a new branch of the project: as git does, not taken already, and not one
    /// that hides a branch of a remote.
    /// The project's own repository (the first of several): the tests' way in.
    #[cfg(test)]
    pub async fn branch_check(&self, project_id: &str, name: &str) -> Result<()> {
        self.branch_check_in(project_id, None, name).await
    }

    pub async fn branch_check_in(
        &self,
        project_id: &str,
        repo: Option<&str>,
        name: &str,
    ) -> Result<()> {
        let (_, root) = self.branch_repo(project_id, repo).await?;
        git::check_new_branch(&root, name).await
    }

    /// Switches the project's folder to `name`: a local branch, or a remote one (`origin/feat`)
    /// through the local branch that tracks it (made when there is none). Refused for a branch an
    /// agent's worktree holds (`IN_WORKTREE`), while an agent without worktree works in the folder
    /// (`AGENT_WORKING`, any project's on this checkout), in the middle of a merge, cherry-pick,
    /// revert or rebase, from a detached HEAD whose commits no branch has (in words), and with
    /// uncommitted changes (`DIRTY`) unless `stash`: they are put aside first, and back if the
    /// switch fails (else the error says which stash holds them). Returns the stash's name when
    /// one was made.
    /// The project's own repository (the first of several): the tests' way in.
    #[cfg(test)]
    pub async fn branch_switch(
        self: &Arc<Self>,
        project_id: &str,
        name: &str,
        stash: bool,
    ) -> Result<Option<String>> {
        self.branch_switch_in(project_id, None, name, stash).await
    }

    pub async fn branch_switch_in(
        self: &Arc<Self>,
        project_id: &str,
        repo: Option<&str>,
        name: &str,
        stash: bool,
    ) -> Result<Option<String>> {
        // The app does not restart for an update in the middle of it.
        let _working = self.working();
        let (_, root) = self.branch_repo(project_id, repo).await?;
        // Not under a pull or a push of the same checkout.
        let lock = self.sync_lock(&root);
        let _guard = lock.lock().await;
        if let Some(local) = git::local_of(&root, name).await? {
            if self.checked_out_here(&root, &local).await? {
                return Ok(None);
            }
        }
        self.no_agent_at_work(&root).await?;
        self.may_leave(&root, false).await?;
        let stashed = self.put_aside(&root, name, stash).await?;
        let switched = git::switch_to(&root, name).await;
        let mut restored = Ok(());
        if let (Err(_), Some(s)) = (&switched, &stashed) {
            restored = self.take_back(&root, s).await;
        }
        // Every project of the repository: the branch is the checkout's.
        self.refresh_repo(&root).await;
        match switched {
            Ok(_) => Ok(stashed.map(|s| s.message)),
            Err(e) => Err(failed_with(e, restored)),
        }
    }

    /// Creates the branch `name` at `start` (a branch, a remote branch, a commit; the folder's
    /// HEAD when empty), tracking nothing, and with `switch` switches the project's folder to it
    /// under the refusals of `branch_switch` (stash included). A start at HEAD changes no file of
    /// the folder: its uncommitted changes go along. Returns the stash's name when one was made.
    /// The project's own repository (the first of several): the tests' way in.
    #[cfg(test)]
    pub async fn branch_create(
        self: &Arc<Self>,
        project_id: &str,
        name: &str,
        start: &str,
        switch: bool,
        stash: bool,
    ) -> Result<Option<String>> {
        self.branch_create_in(project_id, None, name, start, switch, stash)
            .await
    }

    pub async fn branch_create_in(
        self: &Arc<Self>,
        project_id: &str,
        repo: Option<&str>,
        name: &str,
        start: &str,
        switch: bool,
        stash: bool,
    ) -> Result<Option<String>> {
        let _working = self.working();
        let (_, root) = self.branch_repo(project_id, repo).await?;
        let lock = self.sync_lock(&root);
        let _guard = lock.lock().await;
        // Everything that can be refused is, before anything is put aside.
        git::check_new_branch(&root, name).await?;
        let start = match start.trim() {
            "" => "HEAD",
            s => s,
        };
        let commit = git::commit_of(&root, start).await?;
        let mut stashed = None;
        if switch {
            self.no_agent_at_work(&root).await?;
            let head = git::commit_of(&root, "HEAD").await.ok();
            let from_head = head.as_deref() == Some(commit.as_str());
            self.may_leave(&root, from_head).await?;
            if !from_head {
                stashed = self.put_aside(&root, name, stash).await?;
            }
        }
        let mut done = git::branch_create(&root, name, &commit).await;
        if done.is_ok() && switch {
            done = git::switch(&root, name).await;
            if done.is_err() {
                // Not left behind half made: it has nothing of its own yet.
                let _ = git::branch_delete(&root, name, true).await;
            }
        }
        let mut restored = Ok(());
        if let (Err(_), Some(s)) = (&done, &stashed) {
            restored = self.take_back(&root, s).await;
        }
        self.refresh_repo(&root).await;
        match done {
            Ok(()) => Ok(stashed.map(|s| s.message)),
            Err(e) => Err(failed_with(e, restored)),
        }
    }

    /// Deletes the project's branch `name`, its remote copy too with `remote`: its upstream when
    /// that is the branch of the same name that no other local branch tracks, and neither the
    /// remote's default branch nor the base's upstream; otherwise only the local branch goes and
    /// the sentence returned says why. `name` may also be a remote branch alone (`origin/feat`),
    /// deleted on its remote, but for those two. Never the folder's own branch nor one another
    /// worktree holds (`IN_WORKTREE` for an agent's). A branch whose work the project's base has
    /// (contained or squash-merged) goes at once; another is refused (`UNMERGED:<n>`, its commits
    /// no other branch has) unless `force`.
    /// The project's own repository (the first of several): the tests' way in.
    #[cfg(test)]
    pub async fn branch_delete(
        self: &Arc<Self>,
        project_id: &str,
        name: &str,
        remote: bool,
        force: bool,
    ) -> Result<Option<String>> {
        self.branch_delete_in(project_id, None, name, remote, force)
            .await
    }

    pub async fn branch_delete_in(
        self: &Arc<Self>,
        project_id: &str,
        repo: Option<&str>,
        name: &str,
        remote: bool,
        force: bool,
    ) -> Result<Option<String>> {
        let _working = self.working();
        let (project, root) = self.branch_repo(project_id, repo).await?;
        let lock = self.sync_lock(&root);
        let _guard = lock.lock().await;
        let local = !name.starts_with('-') && git::branch_exists(&root, name).await;
        let bases = self.merge_bases(&project, &root).await;
        let mut note = None;
        // What goes: the local branch, and the remote one as (remote, its name there).
        let on_remote = if local {
            if self.checked_out_here(&root, name).await? {
                bail!(tr!(
                    "« {name} » est la branche du dossier du projet : passe sur une autre branche avant de la supprimer.",
                    "“{name}” is the branch of the project’s folder: switch to another branch before deleting it."
                ));
            }
            match remote {
                true => match git::remote_copy(&root, name, &bases).await? {
                    git::RemoteCopy::Delete { remote, branch } => Some((remote, branch)),
                    kept => {
                        note = kept.note(i18n::ui());
                        None
                    }
                },
                false => None,
            }
        } else {
            let split = git::split_remote(&root, name).await;
            if !git::ref_exists(&root, &format!("refs/remotes/{name}")).await || split.is_none() {
                bail!(tr!(
                    "La branche « {name} » est introuvable.",
                    "The branch “{name}” wasn’t found."
                ));
            }
            if let Some((r, b)) = &split {
                // `origin/HEAD` is a symbolic ref to the remote's default branch: deleting it as a
                // branch would follow it and delete that branch's copy.
                if b == "HEAD" {
                    bail!(tr!(
                        "« {name} » n’est pas une branche : c’est le renvoi vers la branche par défaut du dépôt distant.",
                        "“{name}” isn’t a branch: it points to the remote repository’s default branch."
                    ));
                }
                let tracking = format!("refs/remotes/{r}/{b}");
                if let Some(why) = git::remote_guard(&root, r, &tracking, &bases).await {
                    bail!(why.refusal(i18n::ui(), name));
                }
            }
            split
        };
        let mut refs: Vec<String> = Vec::new();
        if local {
            refs.push(format!("refs/heads/{name}"));
        }
        if let Some((r, b)) = &on_remote {
            refs.push(format!("refs/remotes/{r}/{b}"));
        }
        if !force {
            let mut merged = true;
            for r in &refs {
                merged = merged && merged_into_any(&root, r, &bases).await;
            }
            if !merged {
                let commits = git::unique_commits(&root, &refs).await?;
                let base = bases[0].trim_start_matches("refs/heads/").to_string();
                return Err(BranchRefusal::Unmerged {
                    branch: name.to_string(),
                    base,
                    commits,
                }
                .into());
            }
        }
        // The remote first: refused there (rights, network), nothing is deleted.
        let mut done = Ok(());
        if let Some((r, b)) = &on_remote {
            done = git::branch_delete_remote(&root, r, b).await;
        }
        if done.is_ok() && local {
            done = git::branch_delete(&root, name, true).await;
        }
        self.refresh_repo(&root).await;
        done.map(|_| note)
    }

    /// The project's local branches whose work its base has (contained or squash-merged), but the
    /// base, the folder's own branch and those a worktree holds: what « Branches mergées » offers
    /// to delete. By name.
    /// The project's own repository (the first of several): the tests' way in.
    #[cfg(test)]
    pub async fn branches_merged(&self, project_id: &str) -> Result<Vec<String>> {
        self.branches_merged_in(project_id, None).await
    }

    pub async fn branches_merged_in(
        &self,
        project_id: &str,
        repo: Option<&str>,
    ) -> Result<Vec<String>> {
        let (project, root) = self.branch_repo(project_id, repo).await?;
        let bases = self.merge_bases(&project, &root).await;
        let mut merged: Vec<String> = Vec::new();
        for b in &bases {
            for name in git::merged_into(&root, b).await? {
                if !merged.contains(&name) {
                    merged.push(name);
                }
            }
        }
        let held: Vec<String> = git::branch_list(&root)
            .await?
            .into_iter()
            .filter(|b| !b.remote && b.worktree.is_some())
            .map(|b| b.name)
            .collect();
        merged.retain(|n| !held.contains(n) && format!("refs/heads/{n}") != bases[0]);
        merged.sort();
        Ok(merged)
    }

    /// What changes from `a` to `b` in the project's repository (`git diff a b`).
    /// The project's own repository (the first of several): the tests' way in.
    #[cfg(test)]
    pub async fn diff_refs(&self, project_id: &str, a: &str, b: &str) -> Result<String> {
        self.diff_refs_in(project_id, None, a, b).await
    }

    pub async fn diff_refs_in(
        &self,
        project_id: &str,
        repo: Option<&str>,
        a: &str,
        b: &str,
    ) -> Result<String> {
        let (_, root) = self.branch_repo(project_id, repo).await?;
        git::diff_refs(&root, a, b).await
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

    /// What a reading of the account's quota found (a process of the account told it, or the
    /// endpoint did), the quotas settled again (`UsageSnapshot::settle`) and told to the window.
    #[cfg(test)]
    pub(crate) fn record_usage(&self, account: &str, reading: usage::Reading) {
        self.update_usage(|u| u.record(account, reading, now_ms()));
    }

    /// The quotas settled again as the settings now are, and told to the window.
    pub(crate) fn settle_usage(&self) {
        self.update_usage(|_| {});
    }

    /// `change` made to the quotas, which are then settled and told to the window.
    pub(crate) fn update_usage(&self, change: impl FnOnce(&mut UsageSnapshot)) {
        let settings = self.settings.read().clone();
        let mut u = self.usage.lock();
        let before = u.current.clone();
        change(&mut u);
        let now = now_ms();
        u.settle(&settings, now);
        // The day's cost goes with every state of the quotas, for the accounts the settings now list.
        u.set_today(&self.stats.today_by_account());
        // Told under the lock: two changes at once reach the window in the order they were made.
        self.hub.emit(UiEvent::Usage { usage: u.clone() });
        // The account new agents go to changed because the one before passed the threshold.
        let threshold = board::pick_threshold(&settings);
        // Each harness's own: new agents never go to another harness by themselves.
        for (engine, to) in &u.current {
            let Some(from) = before.get(engine) else {
                continue;
            };
            let told = accounts::switch_notice(
                i18n::ui(),
                &settings,
                &u.accounts,
                threshold,
                now,
                from,
                to,
            );
            if let Some(text) = told {
                self.hub.emit(UiEvent::Toast { text });
            }
        }
    }

    /// Every account's quota read again: from a process of the account when one runs
    /// (`get_usage`), else, for an active account, from the usage endpoint with its own sign-in,
    /// at most every five minutes (`usage::OAUTH_POLL_MS`).
    pub async fn refresh_usage(self: &Arc<Self>) {
        let settings = self.settings.read().clone();
        // One running process per account, its first.
        let mut procs: HashMap<String, Process> = HashMap::new();
        for h in self.agents.read().values() {
            let rt = h.lock();
            if let Some(p) = rt.proc.clone() {
                let account = accounts::get(&settings, &rt.meta.account).id;
                procs.entry(account).or_insert(p);
            }
        }
        let mut readings = Vec::new();
        for account in &settings.accounts {
            let proc = procs.get(&account.id).cloned();
            if let Some(r) = self.read_quota(&settings, account, proc).await {
                readings.push((account.id.clone(), r));
            }
        }
        let read = readings
            .iter()
            .any(|(_, r)| matches!(r, usage::Reading::Windows(_)));
        self.update_usage(|u| {
            for (account, reading) in readings {
                u.record(&account, reading, now_ms());
            }
        });
        if read {
            // Back under the threshold, the tickets go on at once.
            self.pause_tick();
        }
    }

    /// The quota of an account of a harness other than Claude Code none of whose agents runs, read
    /// by a process of its own (`Engine::read_quota`) when it was not in the last five minutes.
    /// Not signed in, it is looked at again at the next reading (no process was started).
    async fn read_quota_apart(
        &self,
        settings: &Settings,
        account: &Account,
    ) -> Option<usage::Reading> {
        let program = accounts::program(account, settings)?;
        let before = {
            let mut last = self.last_oauth_call.lock();
            if !usage::oauth_due(last.get(&account.id).copied(), now_ms()) {
                return None;
            }
            last.insert(account.id.clone(), now_ms())
        };
        let env = [settings.claude_env(), accounts::launch_env(account)].concat();
        let dir = accounts::config_dir(account, settings);
        match engine::by_id(account.engine)
            .read_quota(program, env, &dir)
            .await
        {
            Ok(Some(usage::Reading::NotSignedIn)) => {
                let mut last = self.last_oauth_call.lock();
                match before {
                    Some(t) => last.insert(account.id.clone(), t),
                    None => last.remove(&account.id),
                };
                Some(usage::Reading::NotSignedIn)
            }
            Ok(reading) => reading,
            Err(e) => {
                log::debug!("quota of account {}: {e:#}", account.id);
                None
            }
        }
    }

    /// One reading of the account's quota, if one is to be had: its process's, else the
    /// endpoint's when the account is active and was not asked in the last five minutes. None
    /// also for an error of the network or of the server: the last values stand.
    async fn read_quota(
        &self,
        settings: &Settings,
        account: &Account,
        proc: Option<Process>,
    ) -> Option<usage::Reading> {
        if let Some(p) = proc {
            if let Ok(Some(v)) = p.read_usage().await {
                return Some(usage::Reading::Windows(usage::parse_windows(&v)));
            }
        }
        if !account.active {
            return None;
        }
        // The usage endpoint is Claude Code's: another harness's accounts are read by a process of
        // their own (Codex: an app-server, `Engine::read_quota`), at most every five minutes, or not
        // at all (a harness without a quota).
        if !crate::engine::by_id(account.engine).oauth_sign_in() {
            return self.read_quota_apart(settings, account).await;
        }
        // Noted before asking: two readings at once ask the endpoint once.
        let before = {
            let mut last = self.last_oauth_call.lock();
            if !usage::oauth_due(last.get(&account.id).copied(), now_ms()) {
                return None;
            }
            last.insert(account.id.clone(), now_ms())
        };
        let api = self.usage_api.read().clone();
        match usage::read_oauth(settings, &api, &usage::sign_in(account, settings)).await {
            Ok(usage::Reading::NotSignedIn) => {
                // The endpoint was not asked: the next reading looks again, so that a sign-in
                // shows within a minute rather than five.
                let mut last = self.last_oauth_call.lock();
                match before {
                    Some(t) => last.insert(account.id.clone(), t),
                    None => last.remove(&account.id),
                };
                Some(usage::Reading::NotSignedIn)
            }
            Ok(reading) => Some(reading),
            Err(e) => {
                log::debug!("usage endpoint, account {}: {e:#}", account.id);
                None
            }
        }
    }
}

/// Where the plugins' commands run (`plugins::events`): the user's shell, the project's folder.
impl<R: Runtime> crate::plugins::events::Place for Core<R> {
    fn shell(&self) -> Option<crate::pty::ShellInfo> {
        let settings = self.settings.read().clone();
        crate::pty::detect_shells(&settings).into_iter().next()
    }

    fn project_path(&self, id: &str) -> Option<String> {
        self.project(id).ok().map(|p| p.path)
    }

    fn project_plugin(
        &self,
        id: &str,
        plugin: &str,
    ) -> Option<(bool, serde_json::Map<String, serde_json::Value>)> {
        let p = self.project(id).ok()?;
        let settings = p
            .shared
            .file
            .as_ref()
            .and_then(|f| crate::projectfile::plugin_settings(&f.values, plugin))
            .cloned()
            .unwrap_or_default();
        Some((p.private, settings))
    }
}

/// The names of what a fact is about, and whether its project is private (`facts`). Takes the
/// agents, then the projects, then the tickets, one at a time: the bus holds no other lock.
impl<R: Runtime> crate::facts::Resolve for Core<R> {
    fn resolve(&self, about: &mut crate::facts::About) -> Option<bool> {
        // The ticket an agent is made for, before the ticket links it.
        let mut own_ticket = None;
        if let Some(a) = about.agent.as_mut() {
            if let Ok(h) = self.agent(&a.id) {
                let rt = h.lock();
                own_ticket = rt.meta.ticket_id.clone();
                if a.name.is_empty() {
                    a.name = rt.meta.name.clone();
                }
                if about.project.is_none() {
                    about.project = Some(crate::facts::Named {
                        id: rt.meta.project_id.clone(),
                        name: String::new(),
                    });
                }
            }
        }
        {
            let tickets = self.tickets.read();
            let found = match (&about.ticket, &about.agent) {
                (Some(t), _) => tickets.iter().find(|x| x.id == t.id),
                // An agent's fact says its ticket, the one it works on.
                (None, Some(a)) => tickets.iter().find(|x| {
                    x.agent_id.as_deref() == Some(a.id.as_str())
                        || own_ticket.as_deref() == Some(x.id.as_str())
                }),
                (None, None) => None,
            };
            if let Some(t) = found {
                about.ticket = Some(crate::facts::TicketRef {
                    id: t.id.clone(),
                    key: t.key.clone(),
                    title: t.title.clone(),
                });
                if about.project.is_none() {
                    about.project = Some(crate::facts::Named {
                        id: t.project_id.clone(),
                        name: String::new(),
                    });
                }
                if about.agent.is_none() {
                    if let Some(a) = &t.agent_id {
                        about.agent = Some(crate::facts::Named {
                            id: a.clone(),
                            name: String::new(),
                        });
                    }
                }
            }
        }
        if let Some(a) = about.agent.as_mut().filter(|a| a.name.is_empty()) {
            if let Ok(h) = self.agent(&a.id) {
                a.name = h.lock().meta.name.clone();
            }
        }
        let p = about.project.as_mut()?;
        let projects = self.projects.read();
        let found = projects.iter().find(|x| x.id == p.id)?;
        if p.name.is_empty() {
            p.name = found.name.clone();
        }
        Some(found.private)
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

    #[test]
    fn an_agent_is_named_from_its_message_unless_that_is_a_command_without_words_of_its_own() {
        let named = |prompt: &str| naming_text(prompt);
        // Plain text names itself, trimmed.
        assert_eq!(
            named("Create the hello file"),
            Some("Create the hello file".into())
        );
        assert_eq!(
            named("  Crée le fichier \n"),
            Some("Crée le fichier".into())
        );
        assert_eq!(named(""), None);
        assert_eq!(named("  \n "), None);
        // A built-in command of Claude Code has nothing to name a task from, arguments or not.
        for builtin in ["/clear", "/model sonnet", "/compact keep the plan", "/init"] {
            assert_eq!(named(builtin), None, "{builtin}");
        }
        assert_eq!(named("  /resume  "), None);
        // Any other command is a skill or a command of the user's: without arguments, nothing to
        // name from; with some, they are the task (without the leading slash).
        for bare in [
            "/impeccable",
            "/gsd:quick",
            "/superpowers:brainstorming",
            "/",
        ] {
            assert_eq!(named(bare), None, "{bare}");
        }
        assert_eq!(
            named("/impeccable propose un design"),
            Some("impeccable propose un design".into())
        );
        assert_eq!(
            named("  /gsd:quick fix the login"),
            Some("gsd:quick fix the login".into())
        );
        assert_eq!(
            named("/impeccable\nrefais l'écran d'accueil"),
            Some("impeccable\nrefais l'écran d'accueil".into())
        );
        // A built-in's name is a whole word: a skill that starts with one is not that command.
        assert_eq!(named("/clearly"), None);
        assert_eq!(
            named("/clearly say what you mean"),
            Some("clearly say what you mean".into())
        );
    }

    fn change(path: &str, status: &str) -> FileChange {
        FileChange {
            repo: String::new(),
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
            crate::i18n::Lang::Fr,
            false,
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
        let first = commit_proposal_prompt(crate::i18n::Lang::Fr, false, &[], &files, "");
        assert!(!first.contains("<sujets-recents>"), "{first}");
        assert!(first.contains("Conventional Commits"), "{first}");
    }

    #[test]
    fn a_commit_proposal_reads_a_capped_diff_and_list_of_files() {
        let diff = format!("{}FIN-DU-DIFF", "+ligne\n".repeat(20_000));
        let files: Vec<FileChange> = (0..250)
            .map(|i| change(&format!("src/f{i}.ts"), "M"))
            .collect();
        let p = commit_proposal_prompt(crate::i18n::Lang::Fr, false, &[], &files, &diff);
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
    fn what_haiku_and_a_copy_are_told_reads_in_english() {
        use crate::i18n::check::french_in;
        use crate::i18n::Lang::{En, Fr};
        let subjects: Vec<String> =
            ["feat(board): ask before deleting a ticket".to_string()].into();
        let files: Vec<FileChange> = (0..201)
            .map(|i| change(&format!("src/f{i}.ts"), "M"))
            .collect();
        let diff = "diff --git a/src/f0.ts b/src/f0.ts\n+x\n";
        // Named in the settings, the language is said; else the repository's is imitated.
        let named = commit_proposal_prompt(En, true, &subjects, &files, diff);
        let imitated = commit_proposal_prompt(En, false, &subjects, &files, diff);
        let first = commit_proposal_prompt(En, true, &[], &files[..1], "");
        for p in [&named, &imitated, &first] {
            assert_eq!(french_in(p), None, "{p}");
        }
        assert!(
            named.contains("Write it in English, in the style of the repository’s latest commits below: their format, their length.")
                && named.contains("<recent-subjects>\nfeat(board): ask before deleting a ticket\n</recent-subjects>")
                && named.contains("<files>\nM src/f0.ts\n")
                && named.contains("… and 1 more file\n</files>")
                && named.contains("<diff>\ndiff --git a/src/f0.ts b/src/f0.ts\n+x\n</diff>"),
            "{named}"
        );
        assert!(
            imitated.contains("their language, their format, their length"),
            "{imitated}"
        );
        assert!(
            first.contains("Conventional Commits") && !first.contains("<recent-subjects>"),
            "{first}"
        );
        // A repository without a commit has no language to imitate: a language named is said all
        // the same, and none is when the settings leave it to the interface.
        let unnamed = commit_proposal_prompt(En, false, &[], &files[..1], "");
        assert!(first.contains("(“type(scope): description”). Write it in English."));
        assert!(!unnamed.contains("Write it in English"), "{unnamed}");
        assert!(commit_proposal_prompt(Fr, true, &[], &files[..1], "")
            .contains("(« type(portée): description »). Écris-le en français."));
        // In French, the language named the same way.
        assert!(commit_proposal_prompt(Fr, true, &subjects, &files, diff)
            .contains("Écris-le en français, dans le style des derniers commits du dépôt ci-dessous : leur format, leur longueur."));
        assert!(commit_proposal_prompt(Fr, false, &subjects, &files, diff)
            .contains("… et 1 autre fichier\n</fichiers>"));
        let p = naming_prompt(En, "Create a hello.txt file");
        assert_eq!(french_in(&p), None, "{p}");
        assert!(p.contains("<task>\nCreate a hello.txt file\n</task>") && p.contains("slug"));
        for system in [naming_system(En), commit_proposal_system(En)] {
            assert_eq!(french_in(&system), None, "{system}");
        }
        assert_eq!(
            copied_conversation(En, "C:/code/a", "C:/code/b"),
            "This conversation was copied from an agent that worked in C:/code/a. You now work in C:/code/b: only read and write in it."
        );
        assert_eq!(auto_resume(En), "continue");
    }

    #[test]
    fn the_task_is_framed_as_text_to_name() {
        let p = naming_prompt(crate::i18n::Lang::Fr, "Crée un fichier hello.txt");
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
    fn a_copys_branch_keeps_its_marker_within_a_slug() {
        assert_eq!(
            copy_branch("escouade/", "refacto-auth", "refacto-auth (copie)"),
            "escouade/refacto-auth-copie"
        );
        assert_eq!(
            copy_branch("escouade/", "refacto-auth", "refacto-auth (copie 2)"),
            "escouade/refacto-auth-copie-2"
        );
        // A ticket agent's name already takes the 40 characters of a slug: it gives way.
        let ticket = "dem-12-ajouter-la-pagination-aux-evenements";
        let slug = |b: &str| b.strip_prefix("escouade/").unwrap().to_string();
        for (copy, marker) in [
            (format!("{ticket} (copie)"), "-copie"),
            (format!("{ticket} (copie 12)"), "-copie-12"),
        ] {
            let branch = slug(&copy_branch("escouade/", ticket, &copy));
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
            copy_branch("escouade/", &word, &format!("{word} (copie)")),
            format!("escouade/{}-copie", "a".repeat(34))
        );
        // Nothing to make a slug of: the marker alone.
        assert_eq!(
            copy_branch("escouade/", "日本", "日本 (copie)"),
            "escouade/copie"
        );
        // The project's own prefix, or none.
        assert_eq!(copy_branch("gg/", "api", "api (copie)"), "gg/api-copie");
        assert_eq!(copy_branch("", "api", "api (copie)"), "api-copie");
    }

    #[test]
    fn a_copy_is_named_after_its_original_and_numbered_once_taken() {
        let taken = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            copy_name(i18n::Lang::Fr, "refacto-auth", &taken(&["refacto-auth"])),
            "refacto-auth (copie)"
        );
        assert_eq!(
            copy_name(
                i18n::Lang::Fr,
                "refacto-auth",
                &taken(&["refacto-auth", "refacto-auth (copie)"])
            ),
            "refacto-auth (copie 2)"
        );
        assert_eq!(
            copy_name(
                i18n::Lang::Fr,
                "refacto-auth",
                &taken(&["refacto-auth (copie)", "refacto-auth (copie 2)"])
            ),
            "refacto-auth (copie 3)"
        );
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
            worktrees_kept("C:/code/app", "C:\\code\\app", false, None),
            vec![".claude/worktrees"]
        );
        assert_eq!(
            worktrees_kept("C:/code/mono", "C:/code/mono/packages/web", false, None),
            vec![".claude/worktrees", "packages/web/.claude/worktrees"]
        );
        // An agent's worktree holds none of the project's.
        assert_eq!(
            worktrees_kept(
                "C:/code/mono/.claude/worktrees/dem-1",
                "C:/code/mono",
                true,
                None
            ),
            vec![".claude/worktrees"]
        );
        // The project's own « Dossier des worktrees », when it is in the source.
        let own = |p: &str| Some(PathBuf::from(p));
        assert_eq!(
            worktrees_kept(
                "C:/code/app",
                "C:/code/app",
                false,
                own("C:\\code\\app\\.worktrees").as_deref()
            ),
            vec![".claude/worktrees", ".worktrees"]
        );
        assert_eq!(
            worktrees_kept(
                "C:/code/mono",
                "C:/code/mono/packages/web",
                false,
                own("C:/code/mono/packages/web/wt").as_deref()
            ),
            vec![
                ".claude/worktrees",
                "packages/web/.claude/worktrees",
                "packages/web/wt"
            ]
        );
        // Beside the repository, or set to the usual one: nothing more.
        for elsewhere in ["C:/code/app-worktrees", "C:/code/app/.claude/worktrees"] {
            assert_eq!(
                worktrees_kept(
                    "C:/code/app",
                    "C:/code/app",
                    false,
                    own(elsewhere).as_deref()
                ),
                vec![".claude/worktrees"],
                "{elsewhere}"
            );
        }
    }

    #[test]
    fn what_the_core_tells_the_window_reads_in_english() {
        use crate::i18n::Lang::{En, Fr};
        // A copy named in English has its branch named so too.
        assert_eq!(
            copy_branch("escouade/", "refacto-auth", "refacto-auth (copy 2)"),
            "escouade/refacto-auth-copy-2"
        );
        // A validation the app's stop cut, told as the ticket's block when the app starts again.
        assert_eq!(approval_interrupted(En), "Approval interrupted");
        assert_eq!(approval_interrupted(Fr), "Validation interrompue");
        assert_eq!(
            [size_label(En, 18 * MB), size_label(En, 256 * KB)],
            ["18 MB", "256 KB"]
        );
        assert_eq!(size_label(Fr, 5 * MB), "5 Mo");
        let refused = |current: &str| NotOnBase {
            current: current.into(),
            base: "main".into(),
        };
        assert_eq!(
            refused("feature/x").text(En),
            "The project is on the branch “feature/x”: switch to “main” before merging."
        );
        assert_eq!(
            refused("").text(En),
            "The project isn’t on any branch (detached HEAD): switch to “main” before merging."
        );
        // The copy of an agent, named in the language of the moment it is made.
        let taken = vec!["refacto (copy)".to_string()];
        assert_eq!(copy_name(En, "refacto", &taken), "refacto (copy 2)");
        assert_eq!(copy_name(Fr, "refacto", &[]), "refacto (copie)");
        assert_eq!(
            [setup_done(En, 1, 3), setup_done(En, 2, 12)],
            [
                "Worktree set up (1 command, 3 s).",
                "Worktree set up (2 commands, 12 s)."
            ]
        );
        assert_eq!(
            setup_done(Fr, 2, 12),
            "Worktree préparé (2 commandes, 12 s)."
        );
        assert_eq!(
            uncommitted_in_agent(En, 1),
            "The agent has 1 uncommitted file: ask it to commit before merging."
        );
    }

    #[test]
    fn a_look_at_the_plan_files_is_asked_once_and_followed_by_one_more_if_asked_again() {
        let mut scan = PlanScan::default();
        assert!(scan.ask(), "none under way: the caller starts one");
        // Asked while it waits for its turn, and again: one more look is owed, not two.
        assert!(!scan.ask());
        assert!(!scan.ask());
        scan.begin();
        assert!(
            !scan.again,
            "the look that begins answers what was asked before"
        );
        // Asked while the files are being read: they may have moved after the read.
        assert!(!scan.ask());
        assert!(scan.end(), "another look is owed");
        assert!(scan.running);
        scan.begin();
        assert!(!scan.end(), "nothing asked since: it ends");
        assert!(!scan.running);
        assert!(scan.ask(), "the next ask starts a look again");
    }

    #[test]
    fn a_look_at_the_plan_files_waits_for_the_interval_since_the_last_began() {
        let mut scan = PlanScan::default();
        let every = Duration::from_secs(2);
        assert_eq!(scan.wait(every), Duration::ZERO, "the first is at once");
        scan.begin();
        let wait = scan.wait(every);
        assert!(
            wait > Duration::from_millis(1500) && wait <= every,
            "{wait:?}"
        );
        scan.last = Some(Instant::now() - Duration::from_secs(3));
        assert_eq!(scan.wait(every), Duration::ZERO);
        assert_eq!(PLAN_SCAN_EVERY, Duration::from_secs(2));
    }
}
