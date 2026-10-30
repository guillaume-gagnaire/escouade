//! The registry of the harnesses: what each one declares (its program, its tested versions, its
//! capabilities) and what is found of it on this machine (its version, its state).
//!
//! The interface reads the capabilities (`EngineCaps`) to show a function or not, rather than
//! asking whether the agent runs on Claude Code: a function its harness lacks is hidden, not greyed
//! out. A version older than the tested range is refused, with its reason; a newer one is taken,
//! with a warning (docs/TODO-1.8.md, « Versions »).

use super::{Engine, Located, Program};
use crate::i18n::{self, Lang};
use crate::model::EngineId;
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// The harnesses written into the app, in the order the interface lists them: what the registry
/// starts from (INT-3). A harness gets its line with its adapter (docs/TODO-1.8.md, chapter 1).
pub const BUILT_IN: [&dyn Engine; 8] = [
    &super::CLAUDE,
    &super::CODEX,
    &super::COPILOT,
    &super::OPENCODE,
    &super::CURSOR,
    &super::VIBE,
    &super::AGY,
    &super::GEMINI,
];

/// The harnesses there are past the built-in ones, added by `register` (the plugins of a
/// harness, INT-9).
static ADDED: RwLock<Vec<&'static dyn Engine>> = RwLock::new(Vec::new());

/// Every harness there is, in the order the interface lists them: the built-in ones, then the
/// ones added.
pub fn all() -> Vec<&'static dyn Engine> {
    let mut out = BUILT_IN.to_vec();
    out.extend(ADDED.read().iter().copied());
    out
}

/// Adds `engine` to the registry, after the others; one added before under the same id gives it
/// its place. A built-in one keeps its own.
#[allow(dead_code)] // The plugins of a harness come with INT-9; the tests add one.
pub fn register(engine: &'static dyn Engine) {
    let mut added = ADDED.write();
    added.retain(|e| e.id() != engine.id());
    added.push(engine);
}

/// The harness of the registry that bears `id`, if any.
pub fn find(id: EngineId) -> Option<&'static dyn Engine> {
    BUILT_IN
        .iter()
        .copied()
        .find(|e| e.id() == id)
        .or_else(|| ADDED.read().iter().copied().find(|e| e.id() == id))
}

// A mode of Escouade a harness takes (`auto`, `default`, `plan`, `acceptEdits`,
/// `bypassPermissions`), and what it means there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeCap {
    pub id: &'static str,
    /// What it does on this harness, sandbox included, when it is not what Escouade's menu says of
    /// it (None: Claude Code's meaning, the menu's own text).
    pub meaning: Option<String>,
}

/// Up to where a copy of an agent (« Dupliquer la conversation ») keeps its conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
// Its other values are for the harnesses to come (docs/TODO-1.8.md, chapter 1).
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub enum ForkPoint {
    /// The harness cannot copy a conversation: the function is hidden.
    None,
    /// Up to its last entry (Claude Code: `--resume-session-at`).
    Entry,
    /// Up to its last turn (Codex: `thread/fork` with `lastTurnId`).
    Turn,
    /// Up to its last event (Copilot).
    Event,
    /// Up to its last message (OpenCode).
    Message,
}

/// What a message sent while a turn runs does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
// Its other values are for the harnesses to come (docs/TODO-1.8.md, chapter 1).
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub enum MidTurn {
    /// It waits for the end of the turn (Claude Code).
    Queued,
    /// The running turn reads it at once (Codex: `turn/steer`).
    Immediate,
    /// It cannot be sent until the turn ends.
    Refused,
}

/// The quota of the harness's accounts, as it can be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
// Its other values are for the harnesses to come (docs/TODO-1.8.md, chapter 1).
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub enum QuotaKind {
    /// Windows of time, in percent (Claude Code, Codex).
    Windows,
    /// Credits of the month (Copilot).
    Credits,
    /// Nothing to read.
    None,
}

/// What tells a harness's accounts apart (docs/ENGINES.md, « Les comptes »). Its Principal is
/// launched without it, as the harness always is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AccountStore {
    /// A configuration folder of their own, given to their processes by `var`: `CLAUDE_CONFIG_DIR`
    /// (Claude Code), `CODEX_HOME` (Codex). A key kept in the keychain and given by the
    /// environment (`CURSOR_API_KEY`) comes with the first harness that needs one.
    ConfigDir { var: &'static str },
}

/// What a turn costs, as the harness tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
// Its other values are for the harnesses to come (docs/TODO-1.8.md, chapter 1).
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub enum CostKind {
    /// Told by the harness (Claude Code: `total_cost_usd`).
    Exact,
    /// Estimated by Escouade from the tokens.
    Estimated,
    /// Unknown: nothing shown.
    None,
}

/// What a harness can do, as the interface reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineCaps {
    /// The modes it takes, in the menu's order.
    pub modes: Vec<ModeCap>,
    /// Its models take an effort (each model may still have none: `Engine::supports_effort`).
    pub effort: bool,
    /// Images and PDF joined to a message.
    pub images: bool,
    pub pdf: bool,
    /// Copying an agent, and up to where.
    pub fork: ForkPoint,
    /// A conversation resumed on another account of the same harness.
    pub resume_other_account: bool,
    /// A message sent while a turn runs.
    pub mid_turn: MidTurn,
    /// Its `/` commands in the composer.
    pub slash_commands: bool,
    pub quota: QuotaKind,
    /// The lengths of the windows of its quota, in minutes (`QuotaMeter::minutes`): what the
    /// status bar shows before their first reading (Claude Code: 5 hours and a week). A reading
    /// may give fewer (Codex's free plan has one).
    pub quota_windows: Vec<u32>,
    pub cost: CostKind,
    /// Its own interface in a terminal (« Vue Claude Code »).
    pub terminal_view: bool,
    /// Remote Control (claude.ai).
    pub remote_control: bool,
    /// Background tasks and workflows, and stopping a subagent.
    pub background_tasks: bool,
    /// The task tools it may be given (`CLAUDE_CODE_ENABLE_TODO_TOOLS`).
    pub todo_tools: bool,
    /// It tells which requests it would refuse by default (« Claude Code refuserait par défaut »).
    pub default_refusal: bool,
    /// Its commands run in a sandbox, which the modes set (Codex): « Bac à sable » in the agent's header.
    pub sandbox: bool,
    /// Where the usage of its accounts is seen, when it tells Escouade neither quota nor cost
    /// (Cursor CLI: its dashboard, ESC-43): linked in place of the gauges and the cost of the day.
    pub usage_page: Option<&'static str>,
    /// Its accounts but the machine's go by an API key, kept in the keychain (Cursor CLI), rather
    /// than a sign-in in a folder of their own.
    pub accounts_by_key: bool,
    /// Escouade can keep it from writing a file or running a command, in any mode
    /// (`Spawn::read_only`): a discussion of the Docs tab runs on it (ESC-261). Not one that
    /// writes the files of its folder without going through Escouade (Cursor CLI, Mistral Vibe).
    pub read_only: bool,
}

/// The versions of a harness Escouade was tested with, oldest and newest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tested {
    pub oldest: &'static str,
    pub newest: &'static str,
}

/// The command that installs a harness, as « Aucun harness trouvé » gives it: on Windows, and on
/// macOS and Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Install {
    pub windows: &'static str,
    pub unix: &'static str,
}

/// A model the composer's menu offers for a harness, by the id its harness takes (`opus`), with
/// what it takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDecl {
    pub id: &'static str,
    /// Its name in the menu, until the harness tells which version it runs (`Opus`).
    pub label: &'static str,
    /// It takes an effort (`Engine::supports_effort`).
    pub effort: bool,
    /// It takes the mode `auto` (`Engine::supports_auto`).
    pub auto: bool,
}

/// A model as its harness lists it for the account (Codex: `model/list`), which the composer's
/// menu of its agents offers in place of the declared ones (`ModelDecl`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToldModel {
    /// The id its harness takes.
    pub id: String,
    /// Its name in the menu (`GPT-6-Luna`).
    pub label: String,
    /// The efforts it takes, in its harness's order (none: no menu « Effort »).
    pub efforts: Vec<String>,
    pub default_effort: Option<String>,
    /// It reads images.
    pub images: bool,
    /// The one its harness runs unless told otherwise: the model of a new agent of it.
    pub default: bool,
    /// The provider it comes from, when its harness has several (OpenCode: `Ollama (local)`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// What it costs beside the others, as its harness bills it (Copilot: `billing.multiplier`,
    /// `1` the base rate, written `×3` beside it in the menu).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiplier: Option<String>,
    /// Its tier of price, when its harness tells no multiplier (Copilot: `low`, `medium`,
    /// `high`, `very_high`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    /// Not listed by its harness, but the model the account's config names (Codex's `model` of
    /// its `config.toml`, ESC-186): offered after the listed ones, marked as coming from there.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_config: bool,
}

/// A version number (`2.1.301`), compared number by number (a missing one counts as 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version(Vec<u64>);

impl Version {
    /// The first version number of `text` (`2.1.301 (Claude Code)`, `codex-cli 0.163.0`).
    pub fn find(text: &str) -> Option<Version> {
        text.split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .map(|w| w.trim_matches('.'))
            .find(|w| w.contains('.') && w.split('.').all(|n| !n.is_empty()))
            .and_then(|w| w.split('.').map(|n| n.parse().ok()).collect::<Option<_>>())
            .map(Version)
    }

    pub fn parse(text: &str) -> Version {
        Version::find(text).unwrap_or(Version(Vec::new()))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<String> = self.0.iter().map(u64::to_string).collect();
        f.write_str(&parts.join("."))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let n = self.0.len().max(other.0.len());
        let at = |v: &Version, i: usize| v.0.get(i).copied().unwrap_or(0);
        (0..n)
            .map(|i| at(self, i).cmp(&at(other, i)))
            .find(|o| o.is_ne())
            .unwrap_or(Ordering::Equal)
    }
}

/// Where a version stands against the tested range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    Older,
    Tested,
    Newer,
}

impl Tested {
    pub fn place(&self, version: &Version) -> Range {
        if *version < Version::parse(self.oldest) {
            Range::Older
        } else if *version > Version::parse(self.newest) {
            Range::Newer
        } else {
            Range::Tested
        }
    }
}

/// The state of a harness on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineState {
    /// Its program is not found.
    Absent,
    /// Its version is older than the tested ones: refused.
    TooOld,
    /// A line of it Escouade does not drive (OpenCode 2): refused.
    Unsupported,
    /// Found, but no account is signed in.
    NotConnected,
    /// Taken: a version newer than the tested ones too, without a warning (ESC-159).
    Ready,
}

/// A harness as the settings show it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub id: crate::model::EngineId,
    pub name: &'static str,
    /// Where its program is.
    pub located: Located,
    /// Its version, as it tells it (None: not found, or it did not say).
    pub version: Option<String>,
    pub tested: Tested,
    pub state: EngineState,
    /// What its state means for the user: the refusal of an older version and its reason.
    pub note: Option<String>,
    pub caps: EngineCaps,
}

/// What every harness declares, without looking for it on the machine: what the window gets at
/// its start, to show a function or not.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineDecl {
    pub id: crate::model::EngineId,
    pub name: &'static str,
    /// Its agents' name, as the texts speak of them (« Claude attend ta réponse »).
    pub speaker: &'static str,
    pub tested: Tested,
    pub caps: EngineCaps,
    /// How to install it (« Aucun harness trouvé »).
    pub install: Install,
    /// The models of the composer's menu, in its order.
    pub models: Vec<ModelDecl>,
    /// The efforts of the composer's menu, in its order (empty when `caps.effort` is false).
    pub efforts: Vec<&'static str>,
    /// The model that writes the texts (« Modèle des textes rédigés »: `Claude Haiku`).
    pub writer: &'static str,
    /// Its texts go to the model chosen in the settings (`Engine::writer_takes_model`: OpenCode).
    pub writer_takes_model: bool,
}

pub fn declared() -> Vec<EngineDecl> {
    all()
        .into_iter()
        .map(|e| EngineDecl {
            id: e.id(),
            name: e.name(),
            speaker: e.speaker(),
            tested: e.tested(),
            caps: e.caps(),
            install: e.install(),
            models: e.models(),
            efforts: e.efforts().to_vec(),
            writer: e.writer(super::Weight::Light),
            writer_takes_model: e.writer_takes_model(),
        })
        .collect()
}

/// The state of `engine`, from its program, its version and whether an account is signed in, and
/// the text that goes with it, in `lang`.
pub fn state_of(
    lang: Lang,
    engine: &dyn Engine,
    found: bool,
    version: Option<&str>,
    connected: bool,
) -> (EngineState, Option<String>) {
    if !found {
        return (EngineState::Absent, None);
    }
    if let Some(why) = version
        .and_then(Version::find)
        .and_then(|v| engine.unsupported_line(lang, &v))
    {
        return (EngineState::Unsupported, Some(why));
    }
    // Only an older version is refused: a newer one is taken as a tested one is.
    if let Some(v) = version
        .filter(|v| Version::find(v).is_some_and(|v| engine.tested().place(&v) == Range::Older))
    {
        return (EngineState::TooOld, Some(too_old(lang, engine, v)));
    }
    let state = if connected {
        EngineState::Ready
    } else {
        EngineState::NotConnected
    };
    (state, None)
}

/// Refused, in `lang`: the version `found` of `engine` is older than the tested ones.
pub fn too_old(lang: Lang, engine: &dyn Engine, found: &str) -> String {
    let (name, oldest) = (engine.name(), engine.tested().oldest);
    let found = Version::parse(found);
    let why = engine.oldest_reason(lang);
    let update = engine.update_command();
    tr_in!(
        lang,
        "{name} {found} est trop ancien pour Escouade, qui demande au moins la version {oldest} : {why}. Mets-le à jour ({update}).",
        "{name} {found} is too old for Escouade, which needs version {oldest} or later: {why}. Update it ({update})."
    )
}

/// How long `--version` may take (a program of WSL starts its distribution first).
const VERSION_LIMIT: Duration = Duration::from_secs(20);

/// The versions already read, by program: a native one read again once its file changed.
/// A version read: the time its program's file was changed when it was read, and the version.
type Read = (Option<SystemTime>, Option<String>);

static VERSIONS: Mutex<Option<HashMap<String, Read>>> = Mutex::new(None);

fn modified(program: &Program) -> Option<SystemTime> {
    match program {
        Program::Native(path) => std::fs::metadata(path).and_then(|m| m.modified()).ok(),
        Program::Wsl(_) => None,
    }
}

/// Forgets the versions read: asked again when the user looks (maybe just updated).
pub fn forget() {
    *VERSIONS.lock() = None;
}

/// The version of `program`, as `engine` reads it: from what it already said while its file is
/// the same. None when it says none.
pub async fn version(engine: &dyn Engine, program: &Program) -> Option<String> {
    let key = program.describe();
    let stamp = modified(program);
    if let Some((at, v)) = VERSIONS.lock().as_ref().and_then(|m| m.get(&key)) {
        if *at == stamp {
            return v.clone();
        }
    }
    let read = ask_version(engine, program).await;
    VERSIONS
        .lock()
        .get_or_insert_with(HashMap::new)
        .insert(key, (stamp, read.clone()));
    read
}

async fn ask_version(engine: &dyn Engine, program: &Program) -> Option<String> {
    let cwd = std::env::temp_dir();
    let args: Vec<String> = engine
        .version_args()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let env: Vec<_> = super::no_self_update_env(engine).collect();
    let launch = program.launch(&cwd.to_string_lossy(), &args, &env).ok()?;
    let mut cmd = tokio::process::Command::new(&launch.program);
    cmd.args(&launch.args).envs(launch.env).current_dir(&cwd);
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    crate::proc::no_inherited_fds(&mut cmd);
    let out = crate::proc::output_within(cmd, VERSION_LIMIT).await.ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    Version::find(&text).map(|v| v.to_string())
}

/// Refused, with its reason in the interface's language, when the version of `program` is older
/// than the ones `engine` was tested with; taken when it is tested, newer, or unknown.
pub async fn check(engine: &dyn Engine, program: &Program) -> anyhow::Result<()> {
    let read = version(engine, program).await;
    if let Some(why) = read
        .as_deref()
        .and_then(Version::find)
        .and_then(|v| engine.unsupported_line(i18n::ui(), &v))
    {
        return Err(anyhow::anyhow!(why));
    }
    match read {
        Some(v) if engine.tested().place(&Version::parse(&v)) == Range::Older => {
            Err(anyhow::anyhow!(too_old(i18n::ui(), engine, &v)))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
