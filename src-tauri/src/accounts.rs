//! The accounts of the harnesses: each one is its harness run with a configuration folder of its
//! own (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`…), so with its own sign-in, sessions and quota. The
//! machine's account of a harness (its Principal) is the user's own installation, launched as it
//! always was: never given a `CLAUDE_CONFIG_DIR`, since setting one, even to `~/.claude`, changes
//! the name of its macOS keychain entry and signs the user out.
//!
//! Only the accounts the user added exist (ESC-128): a first installation has none, for any
//! harness, and « Ajouter un compte… » adds the machine's (`principal_of`) or a new one.
//!
//! The order of `Settings::accounts` is their priority: new agents go to the first one usable.

use crate::board;
use crate::core::Core;
use crate::engine::{self, Process};
use crate::i18n::Lang;
use crate::model::{Account, AccountUsage, EngineId, Project, Settings};
use crate::pace::{self, Standing};
use crate::pty::TermInfo;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::Runtime;

/// The id of the user's own account.
pub const PRINCIPAL: &str = "principal";

/// How far the accounts are brought (`Settings::accounts_version`): 1 since only the accounts
/// added exist (ESC-128, `migrate`).
pub const ACCOUNTS_VERSION: u32 = 1;

/// The variable that gives Claude Code its configuration folder.
pub(crate) const CONFIG_DIR_VAR: &str = "CLAUDE_CONFIG_DIR";

/// What a new account may share of Principal's folder: its settings and instructions (files),
/// then what extends Claude Code (folders). Never its sign-in (`.credentials.json`) nor its
/// `.claude.json` (the account's own state: its email, its MCP servers, its projects).
pub const SHARED_FILES: [&str; 2] = ["settings.json", "CLAUDE.md"];
pub const SHARED_DIRS: [&str; 6] = [
    "skills",
    "agents",
    "commands",
    "plugins",
    "hooks",
    "output-styles",
];

/// What a new account of Codex may share of its Principal's folder (`~/.codex`): its settings
/// (`config.toml`, which keeps its Windows sandbox and the trust of its folders) and instructions,
/// then its skills, prompts and rules. Never its sign-in (`auth.json`: its refresh token turns, two
/// folders using it would sign each other out), nor its sessions and state.
pub const CODEX_SHARED_FILES: [&str; 2] = ["config.toml", "AGENTS.md"];
pub const CODEX_SHARED_DIRS: [&str; 3] = ["skills", "prompts", "rules"];

/// What a new account of Copilot CLI may share of its Principal's folder (`~/.copilot`): its MCP
/// servers and instructions, then its agents and skills. Never its `config.json` (its sign-in: the
/// users signed in, a token there without a keychain), `settings.json` (its trusted folders), nor
/// its sessions.
pub const COPILOT_SHARED_FILES: [&str; 2] = ["mcp-config.json", "copilot-instructions.md"];
pub const COPILOT_SHARED_DIRS: [&str; 2] = ["agents", "skills"];

/// How the accounts of a harness keep their folder: where its Principal's is in the home folder,
/// the variable that moves it, what a new account may share of it, and the arguments of its
/// program that sign in.
pub struct Store {
    pub home: &'static str,
    pub var: &'static str,
    pub files: &'static [&'static str],
    pub dirs: &'static [&'static str],
    pub login: &'static [&'static str],
    /// Its Principal's folder under the app's `XDG_CONFIG_HOME` when it has one, in place of
    /// `home` (OpenCode's `opencode`, which goes by it under Windows too: tried on 1.19.3).
    pub xdg: Option<&'static str>,
}

impl Store {
    /// A harness that keeps nothing of its accounts in a folder Escouade knows (`Engine::store`).
    pub const NONE: Store = Store {
        home: "",
        var: "",
        files: &[],
        dirs: &[],
        login: &[],
        xdg: None,
    };
}

/// The `Store` of the accounts of `engine`, as its harness declares it (`Engine::store`).
pub fn store(engine: EngineId) -> Store {
    engine::by_id(engine).store()
}

/// Which accounts of a harness go by an API key of their own, kept in the keychain
/// (`Engine::keys`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyUse {
    /// None: each signs in.
    None,
    /// Every one but its Principal (Cursor CLI, Mistral Vibe, Antigravity CLI).
    Added,
    /// Those but its Principal that were added with one, or given one since (Gemini CLI, ESC-275:
    /// `Account::api_key`); the others are signed in by its interface.
    Chosen,
}

/// How a new account shares Principal's items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShareMode {
    /// One item for both accounts: a folder linked (a junction on Windows, which needs no
    /// administrator rights; a symbolic link elsewhere), a file too except on Windows, where it is
    /// copied (a link to a file there needs administrator rights).
    Link,
    /// A copy of Principal's, each account then changing its own.
    Copy,
}

/// Claude Code's Principal as it is added: the user's own Claude Code, active.
pub fn principal() -> Account {
    principal_of(EngineId::CLAUDE)
}

/// The id of the user's own account of `engine`: `principal` for Claude Code (the only one before
/// 1.8), `principal-<harness>` for another.
pub fn principal_id(engine: EngineId) -> String {
    match engine {
        EngineId::CLAUDE => PRINCIPAL.to_string(),
        e => format!("{PRINCIPAL}-{e}"),
    }
}

/// Whether the account is the user's own of its harness: launched as the harness always is,
/// without a configuration folder of Escouade's.
pub fn is_principal(account: &Account) -> bool {
    account.id == principal_id(account.engine)
}

/// The machine's account of `engine` (its Principal) as « Ajouter un compte… » adds it: the user's
/// own installation, active.
pub fn principal_of(engine: EngineId) -> Account {
    Account {
        id: principal_id(engine),
        name: tr!("Principal", "Main"),
        engine,
        ..Default::default()
    }
}

/// Puts the accounts right: every id unique; a Principal without a folder of its own, of the
/// harness its id says; at least one account of each harness that has some active; every harness
/// with accounts once in « Ordre des harness ». It adds none: only the accounts the user added
/// exist (ESC-128).
pub fn normalize(settings: &mut Settings) {
    let accounts = &mut settings.accounts;
    for a in accounts.iter_mut() {
        a.id = a.id.trim().to_string();
        // Exactly the folder its processes are given (`launch_env`).
        a.config_dir = a.config_dir.trim().to_string();
    }
    // An id given twice: the first keeps it, the others are told apart (their agents, if any,
    // stay with the first). Every id given counts as taken from the start: one given once keeps
    // it even when another is told apart before it comes.
    let mut taken: HashSet<String> = accounts
        .iter()
        .filter(|a| !a.id.is_empty())
        .map(|a| a.id.clone())
        .collect();
    let mut seen = HashSet::new();
    for a in accounts.iter_mut().filter(|a| !a.id.is_empty()) {
        if !seen.insert(a.id.clone()) {
            a.id = unique(&a.id, &taken);
            taken.insert(a.id.clone());
            seen.insert(a.id.clone());
        }
    }
    // None (written by hand): one after its name, never a Principal's.
    let written: Vec<EngineId> = engine::registry::all().iter().map(|e| e.id()).collect();
    taken.extend(written.iter().map(|e| principal_id(*e)));
    for a in accounts.iter_mut().filter(|a| a.id.is_empty()) {
        let base = match crate::core::slugify(&a.name) {
            s if s.is_empty() => "account".to_string(),
            s => s,
        };
        a.id = unique(&base, &taken);
        taken.insert(a.id.clone());
    }
    for a in accounts.iter_mut() {
        // A Principal, whatever a hand-edited file says: a folder would sign the user out of it,
        // and its id says its harness.
        if let Some(e) = written.iter().find(|e| principal_id(**e) == a.id) {
            a.engine = *e;
            a.config_dir.clear();
            if a.name.trim().is_empty() {
                a.name = principal().name;
            }
        }
    }
    // Each harness with accounts keeps one active, its first (one this version does not write,
    // saved by a later one, too), and its place in the order.
    let mut engines: Vec<EngineId> = Vec::new();
    for a in accounts.iter() {
        if !engines.contains(&a.engine) {
            engines.push(a.engine);
        }
    }
    for engine in &engines {
        if !accounts.iter().any(|a| a.active && a.engine == *engine) {
            if let Some(first) = accounts.iter_mut().find(|a| a.engine == *engine) {
                first.active = true;
            }
        }
    }
    let mut order = Vec::new();
    for e in settings.engine_order.iter().chain(&engines) {
        if !order.contains(e) {
            order.push(*e);
        }
    }
    settings.engine_order = order;
}

/// The settings saved before 1.8.1 (`accounts_version` 0) brought to the accounts added alone
/// (ESC-128), once: a Principal stays, as an account added (same id, same name, same settings: its
/// agents, tickets and statistics do not move), when `keep` says it is signed in or served; else it
/// goes. Claude Code's was there even when the settings did not list it (saved before the
/// accounts, `normalize` put it back at every load): it is looked at too. Each Claude Code account
/// kept takes the model, effort and mode new agents had (`Settings::default_model`…); another
/// harness's keeps its harness's defaults. Whether anything was done.
pub fn migrate(settings: &mut Settings, keep: impl Fn(&Account) -> bool) -> bool {
    if settings.accounts_version >= ACCOUNTS_VERSION {
        return false;
    }
    if !settings.accounts.iter().any(|a| a.id == PRINCIPAL) {
        settings.accounts.insert(0, principal());
    }
    settings.accounts.retain(|a| !is_principal(a) || keep(a));
    let (model, effort, mode) = (
        settings.default_model.trim().to_string(),
        settings.default_effort.trim().to_string(),
        settings.default_mode.trim().to_string(),
    );
    for a in settings
        .accounts
        .iter_mut()
        .filter(|a| a.engine == EngineId::CLAUDE)
    {
        if a.default_model.is_empty() {
            a.default_model = model.clone();
        }
        if a.default_effort.is_empty() {
            a.default_effort = effort.clone();
        }
        if a.default_mode.is_empty() {
            a.default_mode = mode.clone();
        }
    }
    settings.accounts_version = ACCOUNTS_VERSION;
    normalize(settings);
    true
}

/// What a new agent goes with unless it says its own: its account's model, effort and permission
/// mode (`Account::default_model`…), else its harness's defaults (`defaults`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defaults {
    pub model: String,
    pub effort: String,
    pub mode: String,
}

/// Claude Code's model of new agents, until an account says another.
pub const CLAUDE_DEFAULT_MODEL: &str = "sonnet";

/// The `Defaults` of `account`, else of its harness `engine` (none: no account yet): Claude
/// Code's Sonnet, every harness's effort `medium` and mode `auto`; no model for another harness,
/// whose own default it then runs (`Core::model_for`).
pub fn defaults(account: Option<&Account>, engine: EngineId) -> Defaults {
    let engine = account.map_or(engine, |a| a.engine);
    let own = |f: fn(&Account) -> &String| {
        account
            .map(|a| f(a).trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let model = own(|a| &a.default_model)
        .unwrap_or_else(|| crate::engine::by_id(engine).default_model().to_string());
    // A harness without the mode `auto` (Cursor CLI, Mistral Vibe, Gemini CLI…) starts its new
    // agents in « Demander », the mode they would run in anyway.
    let auto = crate::engine::by_id(engine)
        .caps()
        .modes
        .iter()
        .any(|m| m.id == "auto");
    Defaults {
        model,
        effort: own(|a| &a.default_effort).unwrap_or_else(|| "medium".into()),
        mode: own(|a| &a.default_mode)
            .unwrap_or_else(|| if auto { "auto" } else { "default" }.into()),
    }
}

/// `base`, else `base-2`, `base-3`… the first that is not `taken`.
fn unique(base: &str, taken: &HashSet<String>) -> String {
    let mut id = base.to_string();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// The account `id` names, if the settings have it: an id they do not know (an account removed,
/// a state saved by a later version) is none, never another account (ESC-128).
pub fn find<'a>(settings: &'a Settings, id: &str) -> Option<&'a Account> {
    settings.accounts.iter().find(|a| a.id == id)
}

/// `find`, as a copy that is always there, for what names or reads an account (its name, its
/// folder): one the settings do not know is the machine's of its harness for a Principal's id,
/// else an account named by its id, switched off, with no folder. Never what an agent is started
/// on: that is `Core::agent_account`, which refuses an account gone.
pub fn get(settings: &Settings, id: &str) -> Account {
    if let Some(a) = find(settings, id) {
        return a.clone();
    }
    if let Some(e) = engine::registry::all()
        .iter()
        .map(|e| e.id())
        .find(|e| principal_id(*e) == id)
    {
        return principal_of(e);
    }
    Account {
        id: id.to_string(),
        name: id.to_string(),
        active: false,
        ..Default::default()
    }
}

/// The verdict on the account's quota (`usage`) at `now`, `threshold` being the percentage at which
/// a short window is out (`pace::judge`): the one place that says whether an account is a good one
/// (PR #20). An account of a harness without a quota, or not read yet, has no meter: good.
pub(crate) fn verdict(usage: &[AccountUsage], id: &str, threshold: u32, now: i64) -> pace::Verdict {
    let meters = usage
        .iter()
        .find(|u| u.id == id)
        .map(|u| u.meters.as_slice());
    pace::judge(meters.unwrap_or_default(), threshold, now)
}

/// How the account stands for the choice (`verdict`): good, behind (only its pace is bad) or out.
pub(crate) fn standing(usage: &[AccountUsage], id: &str, threshold: u32, now: i64) -> Standing {
    verdict(usage, id, threshold, now).standing
}

/// Whether one of the account's quota meters (`usage`) is used `threshold` percent or more at
/// `now`, until an end still to come (as the autopilot counts it, `board::over_threshold`). A plain
/// reading of the percentages, for a question about a window being full; never the choice of an
/// account, which is `standing` (the pace of a long window counts there).
pub fn over_threshold(usage: &[AccountUsage], id: &str, threshold: u32, now: i64) -> bool {
    usage.iter().find(|u| u.id == id).is_some_and(|u| {
        u.meters
            .iter()
            .any(|m| board::over_threshold(&m.rate(), threshold, now))
    })
}

/// The account a new agent goes to. The one `preferred` names (a project's, or the one asked for)
/// when it is active; else the first good one (`standing`: see `pace`) in the settings' order; when
/// none is, the first whose only fault is its pace; when every active account is out, the first
/// active one (the autopilot then pauses). None when none is active (no account added, ESC-128).
/// Given the settings of one harness (`for_project`), it stays among its accounts: going on another
/// harness is a handover (ESC-88).
pub fn pick(
    settings: &Settings,
    usage: &[AccountUsage],
    threshold: u32,
    preferred: Option<&str>,
    now: i64,
) -> Option<String> {
    pick_ranked(settings, preferred, |id| {
        standing(usage, id, threshold, now)
    })
}

/// The account `preferred` names when it is active, else the first of the best active ones as
/// `standing` ranks them, in the settings' order (the order stays the priority: a better rank only
/// goes before a worse one); the first active one when all are out.
pub fn pick_ranked(
    settings: &Settings,
    preferred: Option<&str>,
    standing: impl Fn(&str) -> Standing,
) -> Option<String> {
    let active = settings.accounts.iter().filter(|a| a.active);
    if let Some(a) = preferred.and_then(|id| active.clone().find(|a| a.id == id)) {
        return Some(a.id.clone());
    }
    // `min_by_key` keeps the first of equals.
    active.min_by_key(|a| standing(&a.id)).map(|a| a.id.clone())
}

/// The settings as an agent of `engine` sees them: the accounts of the other harnesses taken as
/// switched off.
pub fn of_engine(settings: &Settings, engine: EngineId) -> Settings {
    let mut s = settings.clone();
    for a in s.accounts.iter_mut().filter(|a| a.engine != engine) {
        a.active = false;
    }
    s
}

/// The settings as a new agent of `project` (none: one of no project) on `engine` sees them: the
/// accounts of the other harnesses, and those reserved to other projects (« Réservé aux
/// projets »), taken as switched off. What `pick`, `preferred` and `candidates` are given for a
/// project: they then choose among the accounts of that harness only.
pub fn for_project(settings: &Settings, engine: EngineId, project: Option<&str>) -> Settings {
    let mut s = of_engine(settings, engine);
    for a in s.accounts.iter_mut().filter(|a| !a.serves(project)) {
        a.active = false;
    }
    s
}

/// Whether the account is reserved to `project` alone (« Réservé aux projets »): the only
/// accounts whose sessions, memory and history a private project shares with no other (ESC-141).
pub fn dedicated(account: &Account, project: &str) -> bool {
    matches!(account.projects.as_slice(), [p] if p == project)
}

/// Whether the agents of the project never go on their own to an account it shares with other
/// projects: a private project that did not allow it (« Comptes partagés à la limite »).
pub fn confined(project: &Project) -> bool {
    project.private && !project.shared_on_limit
}

/// `for_project` for `project` itself. A confined one (`confined`) with an active account of
/// `engine` reserved to it alone goes to those only, and to the one it prefers (« Compte
/// préféré », the user's own choice): the others are taken as switched off, so that an account
/// past the threshold sends neither its new agents, its tickets nor a handover to a shared one.
pub fn for_project_of(settings: &Settings, engine: EngineId, project: &Project) -> Settings {
    let mut s = for_project(settings, engine, Some(&project.id));
    let own = |a: &Account| dedicated(a, &project.id);
    if confined(project) && s.accounts.iter().any(|a| a.active && own(a)) {
        let preferred = project.account.trim();
        for a in s
            .accounts
            .iter_mut()
            .filter(|a| !own(a) && a.id != preferred)
        {
            a.active = false;
        }
    }
    s
}

/// Whether an agent of `project` may go on its own (at the usage limit: a switch of account, a
/// handover) to the account `id`: always, but for a confined project only to an account reserved
/// to it alone.
pub fn may_switch_to(settings: &Settings, project: &Project, id: &str) -> bool {
    !confined(project)
        || settings
            .accounts
            .iter()
            .any(|a| a.id == id && dedicated(a, &project.id))
}

/// Whether an active account of `engine` serves new agents of `project` (none: of no project).
/// None does while the harness has no account (ESC-128), or each is reserved to other projects:
/// nothing starts for it there.
pub fn serves_any(settings: &Settings, engine: EngineId, project: Option<&str>) -> bool {
    settings
        .accounts
        .iter()
        .any(|a| a.active && a.engine == engine && a.serves(project))
}

/// The harnesses a project may run on, in the order it goes to them (ESC-88 goes on to the next
/// when every account of one has used its quota): its default harness (`default`), then the
/// others in « Ordre des harness », only those with an active account serving it (`project`: its
/// id).
// Read by the handover to another harness (ESC-88).
#[cfg_attr(not(test), allow(dead_code))]
pub fn fallback_engines(settings: &Settings, default: EngineId, project: &str) -> Vec<EngineId> {
    std::iter::once(default)
        .chain(settings.engine_order.iter().copied())
        .fold(Vec::new(), |mut v, e| {
            if !v.contains(&e) && serves_any(settings, e, Some(project)) {
                v.push(e);
            }
            v
        })
}

/// The account a project prefers (`Project::account`), if it is one the settings have and it is
/// active: what `pick` and `candidates` are given.
pub fn preferred<'a>(settings: &Settings, id: &'a str) -> Option<&'a str> {
    let id = id.trim();
    let known = settings.accounts.iter().any(|a| a.active && a.id == id);
    (!id.is_empty() && known).then_some(id)
}

/// The accounts a project's new agents may go to: the one it prefers when that one is active,
/// else every active one, in order.
pub fn candidates(settings: &Settings, preferred: Option<&str>) -> Vec<String> {
    let active = || settings.accounts.iter().filter(|a| a.active);
    match preferred.and_then(|id| active().find(|a| a.id == id)) {
        Some(a) => vec![a.id.clone()],
        None => active().map(|a| a.id.clone()).collect(),
    }
}

/// What the account is called in `lang`: its name; a Principal still named by default, its
/// default name in `lang` (the one it was first saved in is not always the one the window is in).
pub fn name_in(lang: Lang, account: &Account) -> String {
    let name = account.name.trim();
    if is_principal(account) && (name.is_empty() || default_names().iter().any(|d| d == name)) {
        return tr_in!(lang, "Principal", "Main");
    }
    account.name.clone()
}

/// What Principal is called until it is renamed, in either language.
fn default_names() -> [String; 2] {
    [
        tr_in!(Lang::Fr, "Principal", "Main"),
        tr_in!(Lang::En, "Principal", "Main"),
    ]
}

/// What the window is told when the account new agents go to (the current one) changed from
/// `from` to `to`: only when `from` is an active account that stands worse than `to` now (it is why
/// they go elsewhere: a window at the threshold or at its limit, or a pace that would not last to
/// the reset, `pace`), so that with every account out the first one is the current again, and the
/// autopilot's pause says so; going back to the first good one is not told. None otherwise.
pub fn switch_notice(
    lang: Lang,
    settings: &Settings,
    usage: &[AccountUsage],
    threshold: u32,
    now: i64,
    from: &str,
    to: &str,
) -> Option<String> {
    if from == to || standing(usage, to, threshold, now) >= standing(usage, from, threshold, now) {
        return None;
    }
    let left = settings
        .accounts
        .iter()
        .find(|a| a.active && a.id == from)?;
    // The meter that holds the account back: the worst, and the most used of those.
    let held = verdict(usage, from, threshold, now).holding?;
    let (from, to) = (name_in(lang, left), name_in(lang, &get(settings, to)));
    let what = crate::switching::meter_phrase(lang, &held.meter);
    // Under its threshold, it is the pace that puts the account behind.
    Some(match held.projected.filter(|_| held.by_pace()) {
        Some(p) => {
            let pct = pace::written(p);
            tr_in!(
                lang,
                "Le compte {from} finirait à {pct} % de {what} au rythme actuel : les nouveaux agents partent sur {to}.",
                "At its current pace the {from} account would end at {pct}% of {what}: new agents now go to {to}."
            )
        }
        None => {
            let pct = held.meter.pct.round();
            tr_in!(
                lang,
                "Le compte {from} a atteint {pct} % : les nouveaux agents partent sur {to}.",
                "The {from} account has reached {pct}%: new agents now go to {to}."
            )
        }
    })
}

/// The app's `CLAUDE_CONFIG_DIR` (which Principal's processes inherit) and the home folder. In
/// unit tests, neither the variable nor the machine user's home: a folder that does not exist, so
/// that no test ever reads or writes the user's own Claude Code (tests may run inside a Claude
/// Code session, with its CLAUDE_CONFIG_DIR). A real run by hand (`tui_tests::real_claude_view`)
/// gives the machine user's home in `ESCOUADE_TEST_REAL_HOME`.
pub(crate) fn app_env() -> (Option<String>, PathBuf) {
    if cfg!(test) {
        let home = std::env::var_os("ESCOUADE_TEST_REAL_HOME").map(PathBuf::from);
        return (
            None,
            home.unwrap_or_else(|| std::env::temp_dir().join("escouade-tests-no-home")),
        );
    }
    (
        std::env::var(CONFIG_DIR_VAR).ok(),
        dirs::home_dir().unwrap_or_default(),
    )
}

/// The app's `CLAUDE_CONFIG_DIR` and the home folder as the account's Claude Code goes by them
/// (`app_env`); for a `claude` of WSL, no variable (none of the app's reach it) and its Linux home
/// folder seen from Windows (`\\wsl.localhost\<distro>\home\…`), where Principal's folder is.
pub(crate) fn claude_home(account: &Account, settings: &Settings) -> (Option<String>, PathBuf) {
    match program(account, settings)
        .as_ref()
        .and_then(engine::Program::wsl)
    {
        Some(wsl) => (None, wsl.windows_path(&wsl.home)),
        None => app_env(),
    }
}

/// The configuration folder the account's Claude Code reads, seen from the app: its own, or for
/// Principal (no folder) the app's `CLAUDE_CONFIG_DIR` when it has one, which its processes
/// inherit, else `~/.claude` (in Linux for a `claude` of WSL: `claude_home`). For another harness,
/// the same with its own variable and folder (Codex: `CODEX_HOME`, `~/.codex`).
pub fn config_dir(account: &Account, settings: &Settings) -> PathBuf {
    let harness = engine::by_id(account.engine);
    if let Some(dir) = harness.config_dir(account, settings) {
        return dir;
    }
    let store = harness.store();
    let (_, home) = app_env();
    // Never the machine user's in the tests (as `app_env`).
    let env = (!cfg!(test) && !store.var.is_empty())
        .then(|| std::env::var(store.var).ok())
        .flatten();
    match own_dir(account) {
        Some(dir) => PathBuf::from(dir),
        None => match env.filter(|v| !v.is_empty()) {
            Some(v) => PathBuf::from(v),
            None => xdg_config(&store).unwrap_or_else(|| home.join(store.home)),
        },
    }
}

/// Claude Code's `config_dir` (`Engine::config_dir`): the app's `CLAUDE_CONFIG_DIR` and home
/// folder as its `claude` goes by them (the Linux home folder for one of WSL).
pub(crate) fn claude_config_dir(account: &Account, settings: &Settings) -> PathBuf {
    let (env, home) = claude_home(account, settings);
    config_dir_with(account, env.as_deref(), &home)
}

/// The folder of a harness's Principal under the app's `XDG_CONFIG_HOME` when it has one and the
/// harness goes by it (`Store::xdg`), in place of `~/.config`. Never in the tests.
fn xdg_config(store: &Store) -> Option<PathBuf> {
    let name = store.xdg.filter(|_| !cfg!(test))?;
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(|v| PathBuf::from(v).join(name))
}

/// `config_dir` with the app's `CLAUDE_CONFIG_DIR` (`env`) and home folder given.
pub fn config_dir_with(account: &Account, env: Option<&str>, home: &Path) -> PathBuf {
    match own_dir(account) {
        Some(dir) => PathBuf::from(dir),
        None => match env.filter(|v| !v.is_empty()) {
            Some(v) => PathBuf::from(v),
            None => home.join(".claude"),
        },
    }
}

/// The account's own folder, the configuration folder its processes get (`CLAUDE_CONFIG_DIR` for
/// Claude Code): none for a Principal, whatever its settings say.
pub(crate) fn own_dir(account: &Account) -> Option<&str> {
    let dir = account.config_dir.trim();
    (!is_principal(account) && !dir.is_empty()).then_some(dir)
}

/// What the account's harness is launched with besides the network settings
/// (`Settings::claude_env`): the variable of its configuration folder its harness declares
/// (`Engine::accounts`: `CLAUDE_CONFIG_DIR`, `CODEX_HOME`), exactly the folder saved, when it
/// has one (never for a Principal), and, for every account, those that keep it from updating
/// itself (`Engine::no_self_update`, ESC-133).
pub fn launch_env(account: &Account) -> Vec<(String, String)> {
    launch_env_with(account, &|name| {
        // Unit tests may run inside a Claude Code session with identity variables of their own.
        !cfg!(test) && std::env::var_os(name).is_some()
    })
}

/// The variables of the app's environment (`inherited` tells whether one is set) that make a
/// `claude` sign in as someone else than its account's folder says (ESC-147): a token or an API key
/// takes the place of the login of `CLAUDE_CONFIG_DIR`, `CLAUDE_SECURESTORAGE_CONFIG_DIR` the place
/// of its folder for the sign-in. Every account with a folder of its own would then run as the
/// same identity, and bill its turns to it.
pub(crate) const IDENTITY_VARS: [&str; 3] = [
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
];
pub(crate) const SECURE_STORAGE_VAR: &str = "CLAUDE_SECURESTORAGE_CONFIG_DIR";

/// `launch_env`, the app's environment told by `inherited`. For an account of Claude Code with a
/// folder of its own, those of `IDENTITY_VARS` that are set are given empty (read as not set), and
/// `CLAUDE_SECURESTORAGE_CONFIG_DIR`, when set, its folder: its own sign-in is the one it uses.
/// Principal, which signs in as the machine's user does, keeps them.
pub fn launch_env_with(
    account: &Account,
    inherited: &dyn Fn(&str) -> bool,
) -> Vec<(String, String)> {
    let engine = engine::by_id(account.engine);
    let own = match engine.accounts() {
        engine::registry::AccountStore::ConfigDir { var } => {
            own_dir(account).map(|dir| (var.to_string(), dir.to_string()))
        }
    };
    // Its key, and the variables of the app's that would sign it in as another emptied: its
    // harness's (`Engine::account_env`).
    let env = engine.account_env(account, inherited);
    own.into_iter()
        .chain(env)
        .chain(engine::no_self_update_env(engine))
        .collect()
}

/// The program the account runs: its own when it names one, else the settings' path for Claude
/// Code (the only harness with one in the settings, and the harnesses not written yet, which run
/// its adapter), else where its harness usually is (found as `Engine::locate` finds it). None when
/// it cannot be found.
pub fn program(account: &Account, settings: &Settings) -> Option<engine::Program> {
    let own = account.claude_path.trim();
    let harness = engine::by_id(account.engine);
    let path = if own.is_empty() {
        harness.settings_path(settings)
    } else {
        own
    };
    harness.locate(path, &settings.wsl_distro)
}

/// The file Claude Code keeps the account's own state in (its email, its MCP servers): in its
/// folder; for Principal (no folder) in the app's `CLAUDE_CONFIG_DIR` (`env`) when it has one, else
/// in the home folder, beside `.claude` and not in it.
pub fn claude_json_with(account: &Account, env: Option<&str>, home: &Path) -> PathBuf {
    match own_dir(account).or(env.filter(|v| !v.is_empty())) {
        Some(dir) => Path::new(dir).join(".claude.json"),
        None => home.join(".claude.json"),
    }
}

/// `claude_json_with` as the account's Claude Code goes (`claude_home`).
pub fn claude_json(account: &Account, settings: &Settings) -> PathBuf {
    let (env, home) = claude_home(account, settings);
    claude_json_with(account, env.as_deref(), &home)
}

/// The email of the claude.ai account Claude Code signed in with, as its `.claude.json` (`file`)
/// keeps it (`oauthAccount.emailAddress`); None without one, or a file that cannot be read.
pub fn email_in(file: &Path) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v["oauthAccount"]["emailAddress"]
        .as_str()
        .filter(|e| !e.is_empty())
        .map(str::to_string)
}

/// The sign-in of a Codex account, as its `auth.json` keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexSignIn {
    /// The email of its ChatGPT account (in its `id_token`), none signed in with an API key.
    pub email: Option<String>,
    /// What tells one sign-in from another: a hash of its `id_token` (or key), never the token.
    pub stamp: Option<String>,
}

/// The sign-in of the Codex account of folder `dir` (`<dir>/auth.json`: tokens of ChatGPT, or an
/// API key); None when it has none.
pub fn codex_sign_in(dir: &Path) -> Option<CodexSignIn> {
    use base64::Engine as _;
    use sha2::Digest as _;
    let text = std::fs::read_to_string(dir.join("auth.json")).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let id_token = v["tokens"]["id_token"].as_str().filter(|t| !t.is_empty());
    let key = v["OPENAI_API_KEY"].as_str().filter(|k| !k.is_empty());
    let secret = id_token.or(key)?;
    let email = id_token.and_then(|t| {
        let payload = t.split('.').nth(1)?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload.trim_end_matches('='))
            .ok()?;
        let claims: Value = serde_json::from_slice(&bytes).ok()?;
        claims["email"]
            .as_str()
            .filter(|e| !e.is_empty())
            .map(str::to_string)
    });
    let hash = sha2::Sha256::digest(secret.as_bytes());
    Some(CodexSignIn {
        email,
        stamp: Some(hash.iter().take(8).map(|b| format!("{b:02x}")).collect()),
    })
}

/// The GitHub account a Copilot CLI folder `dir` is signed in to by `copilot login`, as its
/// `config.json` (JSON with comments) names it: its login (the last one signed in, else the first
/// listed), never a token; None when it names none. Its token is in the system's keychain, by user
/// (or in that file, without a keychain): a folder signed in to another user is another account.
pub fn copilot_sign_in(dir: &Path) -> Option<String> {
    copilot_login_in(dir)
}

/// Where Cursor CLI keeps its sign-in (`agent login`): `%APPDATA%\Cursor\auth.json` under Windows,
/// `~/.cursor/auth.json` under macOS, `$XDG_CONFIG_HOME/cursor/auth.json` elsewhere (docs/TODO-1.8.md,
/// point 0, « Cursor CLI »). `CURSOR_CONFIG_DIR` does not move it. In a debug build,
/// `ESCOUADE_CURSOR_AUTH` names another (the end-to-end tests: a file of theirs, or `none`); none in
/// the unit tests.
pub fn cursor_auth_file() -> Option<PathBuf> {
    // Never the machine user's in the unit tests.
    if cfg!(test) {
        return None;
    }
    if cfg!(debug_assertions) {
        if let Ok(forced) = std::env::var("ESCOUADE_CURSOR_AUTH") {
            return (forced != "none").then(|| PathBuf::from(forced));
        }
    }
    if cfg!(windows) {
        dirs::config_dir().map(|d| d.join("Cursor").join("auth.json"))
    } else if cfg!(target_os = "macos") {
        dirs::home_dir().map(|d| d.join(".cursor").join("auth.json"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
            .map(|d| d.join("cursor").join("auth.json"))
    }
}

/// Whether Cursor CLI is signed in on the machine (`agent login`): its sign-in file is there (its
/// tokens are never read). Under macOS, a sign-in in the keychain alone is not seen. A key of the
/// app's environment (`CURSOR_API_KEY`) does not count: its Principal is never given it
/// (`launch_env_with`).
pub fn cursor_signed_in() -> bool {
    cursor_auth_file().is_some_and(|f| f.is_file())
}

/// The variables that give Cursor CLI an identity other than its sign-in (`agent login`): an API
/// key, which an account added by its key goes by (ESC-43), and a token. Cursor takes a token
/// first, then a key, then its sign-in; an empty one counts as none (read in its code, 2026.10.22).
pub(crate) const CURSOR_KEY_VAR: &str = "CURSOR_API_KEY";
pub(crate) const CURSOR_TOKEN_VAR: &str = "CURSOR_AUTH_TOKEN";

/// The variable Mistral Vibe takes its API key from, before its keychain and its `.env` (read in its
/// code, 3.0.3): an account added by its key goes by it (ESC-267).
pub(crate) const MISTRAL_KEY_VAR: &str = "MISTRAL_API_KEY";

/// The variable Antigravity CLI takes its Gemini API key from with `modelProvider: gemini` (its
/// reference, antigravity.google/docs/cli/reference): an account added by its key goes by it
/// (ESC-271).
pub(crate) const AGY_KEY_VAR: &str = crate::engine::agy::KEY_VARS[0];

/// Whether Antigravity CLI is signed in on the machine: its keychain's entry (service `gemini`,
/// account `antigravity`, ESC-269) looked for by the system's own tool, its secret never read
/// (`cmdkey /list` lists names; `security` without `-g` and `secret-tool` with its output thrown
/// away say only whether it is there). None when it cannot be told (no such tool): signed in as far
/// as can be told.
pub fn agy_signed_in() -> Option<bool> {
    // Never the machine user's in the unit tests.
    if cfg!(test) {
        return None;
    }
    // The e2e's: a file of the test's stands for the keychain's entry (its fake `agy` writes it
    // when it is signed in in its terminal).
    if cfg!(debug_assertions) {
        if let Ok(forced) = std::env::var("ESCOUADE_AGY_SIGNED_IN") {
            return Some(Path::new(&forced).is_file());
        }
    }
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("cmdkey", &["/list:gemini:antigravity"])
    } else if cfg!(target_os = "macos") {
        (
            "security",
            &["find-generic-password", "-s", "gemini", "-a", "antigravity"],
        )
    } else {
        (
            "secret-tool",
            &["lookup", "service", "gemini", "username", "antigravity"],
        )
    };
    let mut cmd = std::process::Command::new(program);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if !cfg!(windows) {
        // The secret `secret-tool lookup` writes goes nowhere.
        cmd.stdout(std::process::Stdio::null());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    if cfg!(windows) {
        return Some(cmdkey_lists(&String::from_utf8_lossy(&out.stdout)));
    }
    Some(out.status.success())
}

/// Whether `cmdkey /list:gemini:antigravity` lists the entry: a line of its own ends in its name
/// (`Target: gemini:antigravity`, `Cible : LegacyGeneric:target=gemini:antigravity`, whatever the
/// language); its first line, which names the filter, ends in a colon, and `* NONE *` follows when
/// there is none.
fn cmdkey_lists(out: &str) -> bool {
    out.lines()
        .any(|l| l.trim_end().ends_with("gemini:antigravity"))
}

/// The API keys of the accounts that go by one (Cursor CLI, Mistral Vibe), by their folder (unique to each, the tests' too): read from
/// the keychain at the start (`Core::load_account_keys`) and when the user gives one, so that
/// `launch_env` gives each process its own.
static ACCOUNT_KEYS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, String>>,
> = std::sync::LazyLock::new(Default::default);

/// The keychain's entry of the API key of the account `id` of `engine` (`cursor:<id>`, `vibe:<id>`).
pub(crate) fn key_entry(engine: EngineId, id: &str) -> String {
    format!("{}:{id}", engine.as_str())
}

/// Whether the account is one that goes by its own API key: every one of Cursor CLI's and Mistral
/// Vibe's but their Principal (the machine's sign-in, or the key `vibe --setup` kept).
pub fn by_key(account: &Account) -> bool {
    match engine::by_id(account.engine).keys() {
        KeyUse::Added => !is_principal(account),
        // Gemini CLI: the one it was added with, its key or its Google sign-in (ESC-275).
        KeyUse::Chosen => account.api_key && !is_principal(account),
        KeyUse::None => false,
    }
}

/// Whether a key may be given to the account (« Clé d’API »): one that goes by its key, or a new
/// account of Gemini CLI, which then goes by it (ESC-275).
pub(crate) fn takes_key(account: &Account) -> bool {
    by_key(account)
        || (engine::by_id(account.engine).keys() == KeyUse::Chosen && !is_principal(account))
}

/// The status of an account of a harness whose accounts but its Principal go by their key
/// (Cursor CLI, ESC-43; Mistral Vibe, ESC-267; Antigravity CLI, ESC-271), shown by its end; its
/// Principal's as `machine` says (signed in, and which sign-in), never read.
pub(crate) fn key_status(
    account: &Account,
    settings: &Settings,
    machine: (bool, Option<String>),
) -> AccountStatus {
    let key = account_key(account).filter(|_| by_key(account));
    let (connected, stamp) = if by_key(account) {
        (key.is_some(), None)
    } else {
        machine
    };
    AccountStatus {
        connected,
        email: key.as_deref().map(key_hint),
        stamp,
        dir: config_dir(account, settings).to_string_lossy().into_owned(),
    }
}

/// The API key the account goes by, as the keychain gave it; None when it has none (yet).
pub(crate) fn account_key(account: &Account) -> Option<String> {
    let dir = own_dir(account)?;
    ACCOUNT_KEYS.read().get(dir).cloned()
}

/// The account's API key known (`None`: forgotten) for its processes from now on.
pub(crate) fn remember_key(account: &Account, key: Option<&str>) {
    let Some(dir) = own_dir(account) else { return };
    let mut keys = ACCOUNT_KEYS.write();
    match key {
        Some(k) => keys.insert(dir.to_string(), k.to_string()),
        None => keys.remove(dir),
    };
}

/// How a key is shown (« Connecté · clé …a1b2 »): its last four characters, never more.
pub(crate) fn key_hint(key: &str) -> String {
    let chars: Vec<char> = key.trim().chars().collect();
    let end: String = chars[chars.len().saturating_sub(4)..].iter().collect();
    tr!("clé …{end}", "key …{end}")
}

/// Why the account cannot start a process (none of Cursor's or Vibe's but their Principal goes
/// without its key: it would run as the machine's, another account); Ok for any other.
pub(crate) fn key_ready(account: &Account) -> Result<()> {
    let harness = engine::by_id(account.engine);
    // Gemini CLI's account signed in to Google by `gemini` in its folder (ESC-275): without it, its
    // agent would ask its sign-in where nobody sees it.
    if let Some(ready) = harness.sign_in_ready(account) {
        return ready;
    }
    if !by_key(account) || account_key(account).is_some() {
        return Ok(());
    }
    bail!(harness.key_missing(&account.name))
}

fn copilot_login_in(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("config.json")).ok()?;
    let json: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    let v: Value = serde_json::from_str(&json).ok()?;
    let login = |u: &Value| -> Option<String> {
        u.as_str()
            .or_else(|| u["login"].as_str())
            .filter(|l| !l.is_empty())
            .map(str::to_string)
    };
    let users = v
        .get("loggedInUsers")
        .or_else(|| v.get("logged_in_users"))
        .and_then(Value::as_array);
    let last = v
        .get("lastLoggedInUser")
        .or_else(|| v.get("last_logged_in_user"));
    last.and_then(login)
        .or_else(|| users.into_iter().flatten().find_map(login))
}

/// The items of `SHARED_FILES` and `SHARED_DIRS` that Principal's folder `dir` has, files first.
#[cfg(test)]
pub fn shareable(dir: &Path) -> Vec<String> {
    shareable_of(EngineId::CLAUDE, dir)
}

/// The items a new account of `engine` may share (`store`) that its Principal's folder `dir` has,
/// files first.
pub fn shareable_of(engine: EngineId, dir: &Path) -> Vec<String> {
    let store = store(engine);
    let files = store.files.iter().filter(|n| dir.join(n).is_file());
    let dirs = store.dirs.iter().filter(|n| dir.join(n).is_dir());
    files.chain(dirs).map(|n| n.to_string()).collect()
}

/// What keeps the accounts from being saved: an account other than a Principal needs a folder of
/// its own written in full (relative, its processes would each read another one, under the
/// folder they run in; empty, it would run as Principal).
pub fn check(settings: &Settings) -> Result<()> {
    for a in settings.accounts.iter().filter(|a| !is_principal(a)) {
        let dir = a.config_dir.trim();
        if dir.is_empty() {
            bail!(tr!(
                "Le compte « {name} » n’a pas de dossier de configuration.",
                "The account “{name}” has no configuration folder.",
                name = a.name
            ));
        }
        if !Path::new(dir).is_absolute() {
            bail!(tr!(
                "Le dossier du compte « {name} » doit être un chemin complet : {dir}",
                "The folder of the account “{name}” must be a full path: {dir}",
                name = a.name
            ));
        }
    }
    Ok(())
}

/// The names an account goes by: its own, and for Principal still named by default, its default
/// name in either language (the one it was first saved in is not the one the window shows).
fn names_of(a: &Account) -> Vec<String> {
    let name = a.name.trim().to_lowercase();
    let defaults = default_names().map(|n| n.to_lowercase());
    if is_principal(a) && defaults.contains(&name) {
        defaults.to_vec()
    } else {
        vec![name]
    }
}

/// A new account named `name`: its folder made under `root` (`<data folder>/claude`), named after
/// it and taken by no account (`settings`) nor folder there, with the items `share` of Principal's
/// folder (`principal_dir`) linked or copied into it (`mode`). Its sign-in and `.claude.json`
/// are its own: Claude Code makes them when it signs in.
#[cfg(test)]
pub fn create(
    settings: &Settings,
    root: &Path,
    principal_dir: &Path,
    name: &str,
    share: &[String],
    mode: ShareMode,
) -> Result<Account> {
    create_of(
        EngineId::CLAUDE,
        settings,
        root,
        principal_dir,
        name,
        share,
        mode,
    )
}

/// `create` for an account of `engine`, sharing what its harness lets share (`store`): its folder
/// made under `root` (`<data folder>/codex` for Codex). Its sign-in is its own: its harness makes
/// it when it signs in (Codex's `auth.json` is never shared).
pub fn create_of(
    engine: EngineId,
    settings: &Settings,
    root: &Path,
    principal_dir: &Path,
    name: &str,
    share: &[String],
    mode: ShareMode,
) -> Result<Account> {
    let store = store(engine);
    let name = name.trim();
    if name.is_empty() {
        bail!(tr!("Donne un nom au compte.", "Give the account a name."));
    }
    let lower = name.to_lowercase();
    if settings
        .accounts
        .iter()
        .any(|a| names_of(a).contains(&lower))
    {
        return Err(name_taken(name));
    }
    // Every item checked before anything is made: never the sign-in, nor a path out of the folder.
    if let Some(n) = share
        .iter()
        .find(|n| !store.files.contains(&n.as_str()) && !store.dirs.contains(&n.as_str()))
    {
        bail!(tr!(
            "« {n} » ne se partage pas entre comptes.",
            "“{n}” isn’t shared between accounts."
        ));
    }
    let base = match crate::core::slugify(name) {
        s if s.is_empty() => "account".to_string(),
        s => s,
    };
    let mut taken: HashSet<String> = settings.accounts.iter().map(|a| a.id.clone()).collect();
    taken.extend(engine::registry::all().iter().map(|e| principal_id(e.id())));
    // A folder an account removed left behind keeps its sign-in and its sessions: a new account
    // never takes it over. Named without regard to case, as the file systems of Windows and macOS.
    if let Ok(entries) = std::fs::read_dir(root) {
        taken.extend(
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_lowercase()),
        );
    }
    let id = unique(&base, &taken);
    std::fs::create_dir_all(root)?;
    let dir = std::path::absolute(root.join(&id))?;
    std::fs::create_dir(&dir)?;
    if let Err(e) = share_items(&store, principal_dir, &dir, share, mode) {
        // Its links go with it, never what they lead to.
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    Ok(Account {
        id,
        name: name.to_string(),
        engine,
        config_dir: dir.to_string_lossy().into_owned(),
        claude_path: String::new(),
        active: true,
        ..Default::default()
    })
}

/// The items `names` of `from` (Principal's folder) linked or copied into `to`: those it has.
fn share_items(
    store: &Store,
    from: &Path,
    to: &Path,
    names: &[String],
    mode: ShareMode,
) -> Result<()> {
    // A junction cannot lead into a distribution of WSL (a `claude` of WSL's folder).
    let mode = if crate::wsl::on_share(from) {
        ShareMode::Copy
    } else {
        mode
    };
    for name in names {
        let (src, dst) = (from.join(name), to.join(name));
        let done = if store.dirs.contains(&name.as_str()) {
            if !src.is_dir() {
                continue;
            }
            match mode {
                ShareMode::Link => link_dir(&src, &dst),
                ShareMode::Copy => copy_dir(&src, &dst, 0),
            }
        } else {
            if !src.is_file() {
                continue;
            }
            match mode {
                ShareMode::Link => link_file(&src, &dst),
                ShareMode::Copy => std::fs::copy(&src, &dst).map(|_| ()).map_err(Into::into),
            }
        };
        if let Err(e) = done {
            bail!(tr!(
                "« {name} » n’a pas pu être partagé : {e:#}",
                "“{name}” couldn’t be shared: {e:#}"
            ));
        }
    }
    Ok(())
}

/// `dst` made a symbolic link to the folder `src`.
#[cfg(unix)]
fn link_dir(src: &Path, dst: &Path) -> Result<()> {
    std::os::unix::fs::symlink(std::path::absolute(src)?, dst)?;
    Ok(())
}

/// The `mklink /J` command line `cmd /c` runs for the junction `dst` to the folder `src`, each path
/// quoted: inside quotes, `&`, `^` and parentheses are plain text (Rust quotes an argument only
/// when it has a space). Not `%`, which cmd.exe expands there too, nor a straight quote, which
/// cannot be quoted: such a path is refused.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn junction_line(dst: &Path, src: &Path) -> Result<String> {
    for path in [dst, src] {
        let shown = path.to_string_lossy();
        if shown.contains(['%', '"']) {
            bail!(tr!(
                "« {path} » contient « % » ou un guillemet droit, que cmd.exe ne sait pas lier : choisis « Copier ».",
                "“{path}” has a “%” or a straight quote, which cmd.exe can’t link: choose “Copy”.",
                path = shown
            ));
        }
    }
    Ok(format!(
        "mklink /J \"{}\" \"{}\"",
        dst.display(),
        src.display()
    ))
}

/// What `cmd.exe` is started with to run the `mklink` line: no AutoRun commands (`/d`), no delayed
/// expansion (`/v:off`: a machine that turns it on would have `!name!` in a path read as a
/// variable), then the line (`/c`, which takes all that follows).
#[cfg_attr(not(windows), allow(dead_code))]
const CMD_SWITCHES: [&str; 3] = ["/v:off", "/d", "/c"];

/// `dst` made a junction to the folder `src`: unlike a symbolic link, it needs no administrator
/// rights (nor the developer mode).
#[cfg(windows)]
fn link_dir(src: &Path, dst: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt;
    let cmd = std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into());
    // The line as cmd.exe reads it, not as Rust would quote it (only what has a space).
    let line = junction_line(dst, &std::path::absolute(src)?)?;
    let out = std::process::Command::new(cmd)
        .args(CMD_SWITCHES)
        .raw_arg(line)
        .creation_flags(crate::proc::CREATE_NO_WINDOW)
        .output()?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let said = if said.is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            said
        };
        bail!("mklink /J: {said}");
    }
    Ok(())
}

/// `dst` made a symbolic link to the file `src`.
#[cfg(unix)]
fn link_file(src: &Path, dst: &Path) -> Result<()> {
    std::os::unix::fs::symlink(std::path::absolute(src)?, dst)?;
    Ok(())
}

/// A copy: on Windows, a link to a file needs administrator rights (or the developer mode).
#[cfg(windows)]
fn link_file(src: &Path, dst: &Path) -> Result<()> {
    std::fs::copy(src, dst)?;
    Ok(())
}

/// A copy of the folder `src` at `dst`, what its links lead to copied in their place (a link gone
/// nowhere left out); `depth` keeps a link that leads back up from going on for ever.
fn copy_dir(src: &Path, dst: &Path, depth: u32) -> Result<()> {
    if depth > 32 {
        bail!("{}: too deep", src.display());
    }
    std::fs::create_dir(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let (from, to) = (entry.path(), dst.join(entry.file_name()));
        match std::fs::metadata(&from) {
            Ok(m) if m.is_dir() => copy_dir(&from, &to, depth + 1)?,
            Ok(_) => {
                std::fs::copy(&from, &to)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            // A link Windows cannot follow (one of a WSL distribution's folder): left out.
            Err(e) if std::fs::symlink_metadata(&from).is_ok_and(|m| m.is_symlink()) => {
                log::warn!("{}: link left out of the copy: {e}", from.display());
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// A session id as Claude Code makes them (a UUID): letters, digits, `-` and `_`, so that it
/// names a file and nothing else.
pub(crate) fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The session `session_id` of the account whose folder is `from`, copied into the folder `to` of
/// another account, where its `--resume` finds it: Claude Code looks a session up in the account's
/// `projects/*/<id>.jsonl`. Its transcript and the folder `<id>/` beside it (what its subagents and
/// tools keep) go to the same project folder of `to`. A file `to` already has is replaced only by a
/// more recent one: the session may have been there before, and gone on.
///
/// Nobody may write the session while it is copied: the agent's process is stopped (and gone)
/// before.
pub fn copy_session(from: &Path, to: &Path, session_id: &str) -> Result<()> {
    if !valid_session_id(session_id) {
        bail!(tr!(
            "Identifiant de session invalide : {id}",
            "Invalid session id: {id}",
            id = session_id
        ));
    }
    let file = format!("{session_id}.jsonl");
    // The project folder that holds it; if several do, the one written last.
    let mut found: Option<(std::time::SystemTime, std::ffi::OsString)> = None;
    if let Ok(folders) = std::fs::read_dir(from.join("projects")) {
        for folder in folders.flatten() {
            let Ok(meta) = std::fs::metadata(folder.path().join(&file)) else {
                continue;
            };
            let at = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            if meta.is_file() && found.as_ref().is_none_or(|(best, _)| at > *best) {
                found = Some((at, folder.file_name()));
            }
        }
    }
    let Some((_, folder)) = found else {
        bail!(tr!(
            "Session « {id} » introuvable dans le dossier du compte : {dir}",
            "Session “{id}” not found in the account’s folder: {dir}",
            id = session_id,
            dir = from.display()
        ));
    };
    let (src, dst) = (
        from.join("projects").join(&folder),
        to.join("projects").join(&folder),
    );
    std::fs::create_dir_all(&dst)?;
    // What it keeps first, its transcript last: a transcript that is there has all that goes with it.
    let kept = src.join(session_id);
    if kept.is_dir() {
        copy_newer_dir(&kept, &dst.join(session_id), 0)?;
    }
    copy_newer(&src.join(&file), &dst.join(&file))
}

/// The folder `src` copied into `dst`, file by file (`copy_newer`); a link is left where it is.
fn copy_newer_dir(src: &Path, dst: &Path, depth: u32) -> Result<()> {
    if depth > 32 {
        bail!("{}: too deep", src.display());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let (from, to) = (entry.path(), dst.join(entry.file_name()));
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_newer_dir(&from, &to, depth + 1)?;
        } else if kind.is_file() {
            copy_newer(&from, &to)?;
        }
    }
    Ok(())
}

/// The file `src` copied to `dst`, with its modification time, unless `dst` is there and at least
/// as recent. Written beside and renamed: never a half-written transcript where a resume reads.
fn copy_newer(src: &Path, dst: &Path) -> Result<()> {
    let modified = std::fs::metadata(src)?.modified()?;
    if let Ok(there) = std::fs::metadata(dst) {
        if there.modified().is_ok_and(|t| t >= modified) {
            return Ok(());
        }
    }
    let mut name = dst.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    let part = dst.with_file_name(name);
    let copied = std::fs::copy(src, &part)
        .and_then(|_| {
            // The copy is as old as what it copies: the more recent of two is told by it.
            let f = std::fs::OpenOptions::new().write(true).open(&part)?;
            f.set_modified(modified)
        })
        .and_then(|_| std::fs::rename(&part, dst));
    if let Err(e) = copied {
        let _ = std::fs::remove_file(&part);
        return Err(e.into());
    }
    Ok(())
}

/// Whether an account is signed in to claude.ai, as its tab shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    /// Claude Code keeps a sign-in for it (an expired one too: it gets a new token when it runs).
    pub connected: bool,
    /// The email of the claude.ai account, when signed in.
    pub email: Option<String>,
    /// Its configuration folder (Principal's: the app's `CLAUDE_CONFIG_DIR`, else `~/.claude`).
    pub dir: String,
    /// Which sign-in it is (a fingerprint, never the token): the same while the sign-in is, even
    /// out of date, another once the user signs in again. None when not signed in.
    pub stamp: Option<String>,
}

/// What « Ajouter un compte… » says of the machine's account of a harness (`Core::machine_account`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineAccount {
    /// Its harness's program is found.
    pub found: bool,
    /// It is among the accounts already.
    pub added: bool,
    /// Its harness is signed in there (OpenCode, which signs in to none: found).
    pub connected: bool,
    /// Its harness signs in to an account of its own (not OpenCode, whose providers are its
    /// config's).
    pub signs_in: bool,
    /// It may be added before it is signed in there, then signed in by its program in a terminal
    /// (Antigravity CLI, whose sign-in is the machine's alone, ESC-271).
    pub signs_in_later: bool,
    /// Its sign-in is out of Escouade's reach (Gemini CLI's Google sign-in, in `~/.gemini`): a new
    /// account is to add instead (ESC-275).
    pub apart: bool,
    /// Who it is signed in as, when the harness says: an email, a GitHub `@login`.
    pub email: Option<String>,
    /// Its folder (`~/.claude`, the app's `CLAUDE_CONFIG_DIR`, `~/.codex`…).
    pub dir: String,
}

/// Refused: no account has the id the window gave (removed meanwhile).
fn unknown(id: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "compte « {id} » introuvable",
        "account “{id}” not found"
    ))
}

/// What a move of an agent's session left behind: the account it left, and the resume it waited
/// for there, which the move dropped.
pub(crate) struct Moved {
    pub(crate) from: Account,
    pub(crate) resume_at: Option<i64>,
}

/// Refused: the account is reserved to other projects (« Réservé aux projets »).
fn reserved(account: &Account) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "Le compte « {name} » est réservé à d’autres projets.",
        "The account “{name}” is reserved to other projects.",
        name = name_in(crate::i18n::ui(), account)
    ))
}

/// Why the account `id` may not be the one a ticket of `project` on `engine` is set to (« Compte »
/// in its form, ESC-153): one the settings do not know, of another harness, switched off, reserved
/// to other projects, or shared while the private project goes to its own accounts alone. None
/// when it may.
pub(crate) fn ticket_refusal(
    settings: &Settings,
    project: &Project,
    engine: EngineId,
    id: &str,
) -> Option<anyhow::Error> {
    let Some(account) = find(settings, id) else {
        return Some(unknown(id));
    };
    let lang = crate::i18n::ui();
    let name = name_in(lang, account);
    if account.engine != engine {
        let (theirs, ours) = (engine::by_name(account.engine), engine::by_name(engine));
        return Some(anyhow::anyhow!(tr_in!(
            lang,
            "Le compte « {name} » est un compte {theirs}, et ce ticket tourne sur {ours}.",
            "The account “{name}” is a {theirs} account, and this ticket runs on {ours}."
        )));
    }
    if !account.active {
        return Some(switched_off(account));
    }
    if !account.serves(Some(&project.id)) {
        return Some(reserved(account));
    }
    let usable = for_project_of(settings, engine, project)
        .accounts
        .iter()
        .any(|a| a.active && a.id == id);
    (!usable).then(|| {
        anyhow::anyhow!(tr_in!(
            lang,
            "Le projet privé « {project} » n’utilise que ses propres comptes : « {name} » est partagé avec d’autres projets.",
            "The private project “{project}” uses its own accounts alone: “{name}” is shared with other projects.",
            project = project.name
        ))
    })
}

/// Refused: the harness `engine` has no account (ESC-128): nothing starts on it before one is
/// added. The window offers « Ajouter un compte… » on that harness with it (`commands::err`).
#[derive(Debug)]
pub struct NoAccount {
    pub engine: EngineId,
    text: String,
}

impl std::fmt::Display for NoAccount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl std::error::Error for NoAccount {}

/// What the window is told of the refusal `e` (said as `text`): one for want of an account comes
/// as `no-account:<harness>|<text>`, which the window shows with « Ajouter un compte… » on that
/// harness (`refusal.ts`).
pub fn marked(e: &anyhow::Error, text: String) -> String {
    match e.downcast_ref::<NoAccount>() {
        Some(no) => format!("no-account:{}|{text}", no.engine),
        None => text,
    }
}

/// `NoAccount` for `engine`, said in the interface's language.
pub(crate) fn no_account_of(engine: EngineId) -> anyhow::Error {
    let harness = engine::by_name(engine);
    anyhow::Error::new(NoAccount {
        engine,
        text: tr!(
            "Aucun compte {harness} : ajoute-en un dans Réglages › Comptes.",
            "No {harness} account: add one in Settings › Accounts."
        ),
    })
}

/// Whether the settings have an account of `engine`, active or not.
pub fn has_any(settings: &Settings, engine: EngineId) -> bool {
    settings.accounts.iter().any(|a| a.engine == engine)
}

/// Refused: no active account of `engine` serves the project `name` (all are reserved to others,
/// or off; none at all: `no_account_of`): nothing starts for it on that harness.
pub(crate) fn no_account(name: &str, engine: EngineId) -> anyhow::Error {
    let harness = engine::by_name(engine);
    anyhow::anyhow!(tr!(
        "Aucun compte {harness} actif ne sert le projet « {name} » : coche-le sous « Réservé aux projets » d’un compte {harness} actif, dans Réglages › Comptes.",
        "No active {harness} account serves the project “{name}”: tick it under “Reserved to projects” on an active {harness} account, in Settings › Accounts."
    ))
}

/// Refused: no active account of `engine` serves agents of no project (every one is reserved).
pub(crate) fn no_account_outside_projects(engine: EngineId) -> anyhow::Error {
    let harness = engine::by_name(engine);
    anyhow::anyhow!(tr!(
        "Aucun compte {harness} actif ne sert les agents hors projet : chacun est réservé à des projets.",
        "No active {harness} account serves agents outside a project: each is reserved to projects."
    ))
}

/// Why a project's board starts nothing while no account of its default harness serves it
/// (`no_account`).
pub(crate) fn no_account_issue(engine: EngineId) -> String {
    let harness = engine::by_name(engine);
    tr!(
        "Aucun compte {harness} pour ce projet — aucun ticket ne démarre",
        "No {harness} account for this project — no tickets will start"
    )
}

/// Refused, in `lang`: `account` runs Claude Code on the other side of WSL than the agent's
/// (`in_wsl`: it runs it in WSL, the agent's account on Windows).
fn other_side(lang: Lang, account: &Account, in_wsl: bool) -> anyhow::Error {
    let name = name_in(lang, account);
    anyhow::anyhow!(if in_wsl {
        tr_in!(
            lang,
            "Le compte « {name} » fait tourner Claude Code dans WSL, celui de l’agent sous Windows : sa session ne passe pas de l’un à l’autre.",
            "The “{name}” account runs Claude Code in WSL, the agent’s on Windows: its session doesn’t go from one to the other."
        )
    } else {
        tr_in!(
            lang,
            "Le compte « {name} » fait tourner Claude Code sous Windows, celui de l’agent dans WSL : sa session ne passe pas de l’un à l’autre.",
            "The “{name}” account runs Claude Code on Windows, the agent’s in WSL: its session doesn’t go from one to the other."
        )
    })
}

/// Refused: the account is switched off, nothing goes to it.
fn switched_off(account: &Account) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "Le compte « {name} » est désactivé.",
        "The account “{name}” is switched off.",
        name = name_in(crate::i18n::ui(), account)
    ))
}

/// How long a process is given to end once its input is closed, and once it is killed.
const STOP_GRACE: Duration = Duration::from_secs(10);
const KILL_GRACE: Duration = Duration::from_secs(5);

/// The process stopped (its input closed: it finishes what it does and ends; killed if it does not
/// within `STOP_GRACE`) and gone. A session is never copied before: the process may write it as
/// it ends, and the account it goes to would resume a session another still writes.
async fn stop_and_wait(p: &Process) -> Result<()> {
    if ended(p).await {
        return Ok(());
    }
    bail!(tr!(
        "Le process de l’agent ne s’arrête pas : la session n’a pas été copiée.",
        "The agent’s process won’t stop: the session wasn’t copied."
    ))
}

/// The process stopped (its input closed, killed if it does not end within `STOP_GRACE`): whether
/// it is gone.
pub(crate) async fn ended(p: &Process) -> bool {
    p.close_input();
    if p.wait_exit(STOP_GRACE).await {
        return true;
    }
    p.kill();
    p.wait_exit(KILL_GRACE).await
}

/// Refused: an account named as another one.
fn name_taken(name: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "Un compte s’appelle déjà « {name} ».",
        "An account is already named “{name}”."
    ))
}

// The accounts' tab: each change saved at once, through `save_settings` (normalized, checked, the
// quotas settled again), one at a time (`claude_accounts_lock`).
impl<R: Runtime> Core<R> {
    /// The settings the window saves, but for the accounts (and « Ordre des harness ») and for « Claude et les agents peuvent
    /// piloter Escouade »: those are the backend's, which their tabs change on their own (the window's copy
    /// may be older).
    pub fn save_window_settings(self: &Arc<Self>, mut s: Settings) -> Result<()> {
        let _one = self.claude_accounts_lock.lock();
        s.accounts = self.settings.read().accounts.clone();
        s.engine_order = self.settings.read().engine_order.clone();
        s.accounts_version = self.settings.read().accounts_version;
        s.mcp_enabled = self.settings.read().mcp_enabled;
        self.save_settings(s)
    }

    /// The settings changed by `change` and saved, with the accounts as they then are.
    pub(crate) fn change_claude_accounts<T>(
        self: &Arc<Self>,
        change: impl FnOnce(&mut Settings) -> Result<T>,
    ) -> Result<(T, Vec<Account>)> {
        let _one = self.claude_accounts_lock.lock();
        let mut s = self.settings.read().clone();
        let out = change(&mut s)?;
        self.save_settings(s)?;
        let accounts = self.settings.read().accounts.clone();
        Ok((out, accounts))
    }

    /// The project `id` taken out of every account's « Réservé aux projets » (it is gone, its
    /// history too: a project closed with its history kept stays in them). An account kept to it
    /// alone then serves every project, as one kept to none does.
    pub(crate) fn forget_project_in_accounts(self: &Arc<Self>, id: &str) {
        let kept = |s: &Settings| {
            s.accounts
                .iter()
                .any(|a| a.projects.iter().any(|p| p == id))
        };
        if !kept(&self.settings.read()) {
            return;
        }
        let forgotten = self.change_claude_accounts(|s| {
            for a in &mut s.accounts {
                a.projects.retain(|p| p != id);
            }
            Ok(())
        });
        if let Err(e) = forgotten {
            log::warn!("accounts: project {id} not forgotten: {e:#}");
        }
    }

    /// The account `id` names, as the settings now have it.
    fn claude_account(&self, id: &str) -> Result<Account> {
        let settings = self.settings.read();
        let found = settings.accounts.iter().find(|a| a.id == id).cloned();
        found.ok_or_else(|| unknown(id))
    }

    /// The items of Principal's folder a new account may share.
    #[cfg(test)]
    pub fn claude_shareable(&self) -> Vec<String> {
        self.shareable(EngineId::CLAUDE)
    }

    /// The items of the folder of the Principal of `engine` a new account of it may share.
    pub fn shareable(&self, engine: EngineId) -> Vec<String> {
        let settings = self.settings.read().clone();
        shareable_of(
            engine,
            &config_dir(&get(&settings, &principal_id(engine)), &settings),
        )
    }

    /// « Créer et se connecter »: a new account (`create`), its folder in the data folder, last in
    /// the order and active.
    #[cfg(test)]
    pub fn create_claude_account(
        self: &Arc<Self>,
        name: &str,
        share: &[String],
        mode: ShareMode,
    ) -> Result<Account> {
        self.create_account(EngineId::CLAUDE, name, share, mode)
    }

    /// « Nouveau compte » of « Ajouter un compte… » for `engine` (`create_of`): its folder in the
    /// data folder's of its harness (`claude`, `codex`), last in the order and active. Alone: its
    /// harness's Principal is added only when the user asks (`add_machine_account`, ESC-128).
    pub fn create_account(
        self: &Arc<Self>,
        engine: EngineId,
        name: &str,
        share: &[String],
        mode: ShareMode,
    ) -> Result<Account> {
        let root = self.data.accounts_of(engine);
        let (made, _) = self.change_claude_accounts(|s| {
            let from = config_dir(&get(s, &principal_id(engine)), s);
            let made = create_of(engine, s, &root, &from, name, share, mode)?;
            s.accounts.push(made.clone());
            Ok(made)
        })?;
        self.account_added(&made);
        Ok(made)
    }

    /// « Le compte de la machine » of « Ajouter un compte… »: the user's own installation of
    /// `engine` (its Principal: its usual folder, `~/.claude`, `~/.codex`…), added last in the
    /// order and active, as it is. Refused when it is there already.
    pub fn add_machine_account(self: &Arc<Self>, engine: EngineId) -> Result<Account> {
        let (made, _) = self.change_claude_accounts(|s| {
            let id = principal_id(engine);
            if s.accounts.iter().any(|a| a.id == id) {
                bail!(tr!(
                    "Le compte de la machine est déjà ajouté.",
                    "The machine’s account is added already."
                ));
            }
            let made = principal_of(engine);
            s.accounts.push(made.clone());
            Ok(made)
        })?;
        self.account_added(&made);
        Ok(made)
    }

    /// « Clé d’API » of a Cursor or Mistral Vibe account other than its Principal (ESC-43,
    /// ESC-267): `key` kept in the system's keychain, read back to check, and given to its
    /// processes from now on by `CURSOR_API_KEY` or `MISTRAL_API_KEY` (never on a command line,
    /// never in a file of Escouade's). Its status as it then is.
    pub async fn set_account_key(self: &Arc<Self>, id: &str, key: &str) -> Result<AccountStatus> {
        let account = find(&self.settings.read(), id)
            .cloned()
            .ok_or_else(|| unknown(id))?;
        if !takes_key(&account) {
            bail!(tr!(
                "Ce compte ne prend pas de clé d’API.",
                "This account takes no API key."
            ));
        }
        let key = key.trim();
        let malformed = key.len() < 16 || key.chars().any(char::is_whitespace);
        if malformed {
            bail!(engine::by_id(account.engine).key_malformed());
        }
        let store = self.secrets.clone();
        let (entry, secret) = (key_entry(account.engine, id), key.to_string());
        let harness = account.engine.as_str();
        tokio::task::spawn_blocking(move || {
            crate::integrations::secrets::keep_secret(&*store, &entry, &secret)
        })
        .await?
        .map_err(|e| {
            log::warn!("{harness} account {id}: the system keychain refused its key: {e:#}");
            anyhow::anyhow!(tr!(
                "Le trousseau du système n’a pas gardé la clé : {e:#}",
                "The system keychain didn’t keep the key: {e:#}"
            ))
        })?;
        remember_key(&account, Some(key));
        // Gemini CLI: the account goes by its key from now on (ESC-275).
        let account = if engine::by_id(account.engine).keys() == KeyUse::Chosen && !account.api_key
        {
            let (made, _) = self.change_claude_accounts(|s| {
                let a = s
                    .accounts
                    .iter_mut()
                    .find(|a| a.id == id)
                    .ok_or_else(|| unknown(id))?;
                a.api_key = true;
                Ok(a.clone())
            })?;
            // The window says it by its key from now on.
            let (accounts, engine_order) = {
                let s = self.settings.read();
                (s.accounts.clone(), s.engine_order.clone())
            };
            self.hub.emit(crate::model::UiEvent::Accounts {
                accounts,
                engine_order,
            });
            made
        } else {
            account
        };
        log::info!("{} account {id}: API key given", account.engine.as_str());
        Ok(self.status_of(&account).await)
    }

    /// At the start: the keys of the Cursor and Vibe accounts read from the keychain (`set_account_key`).
    /// One the keychain does not give leaves its account not signed in, said so in the log.
    pub(crate) fn load_account_keys(&self) {
        let accounts: Vec<Account> = self
            .settings
            .read()
            .accounts
            .iter()
            .filter(|a| by_key(a))
            .cloned()
            .collect();
        for a in accounts {
            let harness = a.engine.as_str();
            match self.secrets.get(&key_entry(a.engine, &a.id)) {
                Ok(Some(key)) if !key.is_empty() => remember_key(&a, Some(&key)),
                Ok(_) => log::warn!(
                    "{harness} account {}: no API key in the system keychain",
                    a.id
                ),
                Err(e) => log::warn!(
                    "{harness} account {}: the system keychain did not give its key: {e:#}",
                    a.id
                ),
            }
        }
    }

    /// An account added: the window told the accounts (and « Ordre des harness », which gains its
    /// harness), the models of a Claude Code or Codex one read (ESC-124, ESC-186).
    fn account_added(self: &Arc<Self>, made: &Account) {
        let (accounts, engine_order) = {
            let s = self.settings.read();
            (s.accounts.clone(), s.engine_order.clone())
        };
        self.hub.emit(crate::model::UiEvent::Accounts {
            accounts,
            engine_order,
        });
        let (c, id) = (self.clone(), made.id.clone());
        tauri::async_runtime::spawn(async move { c.read_account_models(&id).await });
    }

    /// What « Ajouter un compte… » says of the machine's account of `engine`: whether its harness
    /// is signed in there (with its identity when the harness gives it), its folder, and whether it
    /// is added already.
    pub async fn machine_account(&self, engine: EngineId) -> MachineAccount {
        let id = principal_id(engine);
        let settings = self.settings.read().clone();
        let account = get(&settings, &id);
        let added = find(&settings, &id).is_some();
        let found = {
            let (account, settings) = (account.clone(), settings.clone());
            tokio::task::spawn_blocking(move || program(&account, &settings).is_some())
                .await
                .unwrap_or(false)
        };
        let status = self.status_of(&account).await;
        let harness = engine::by_id(engine);
        let machine_key = harness.machine_key();
        MachineAccount {
            found,
            added,
            // OpenCode signs in to no account: its own installation, once found. Gemini CLI's
            // machine's account is a key or Vertex AI of the app's alone (ESC-275).
            connected: if !harness.signs_in() {
                found
            } else {
                machine_key.unwrap_or(status.connected)
            },
            signs_in: harness.signs_in(),
            signs_in_later: harness.signs_in_later(),
            // Gemini CLI's is its key or Vertex AI of the app's alone: its Google sign-in is in
            // `~/.gemini`, never given (ESC-275).
            apart: machine_key == Some(false),
            email: status.email,
            dir: status.dir,
        }
    }

    /// The account's name, `claude`, whether new agents may go to it and what they go with, as the
    /// tab gives them (its folder stays its own); the accounts as they then are.
    pub fn update_claude_account(self: &Arc<Self>, account: Account) -> Result<Vec<Account>> {
        let (_, accounts) = self.change_claude_accounts(|s| {
            if !s.accounts.iter().any(|a| a.id == account.id) {
                return Err(unknown(&account.id));
            }
            let name = account.name.trim();
            if name.is_empty() {
                bail!(tr!("Donne un nom au compte.", "Give the account a name."));
            }
            let lower = name.to_lowercase();
            let others = || s.accounts.iter().filter(|a| a.id != account.id);
            // Each harness has its Principal, all named « Principal »: one does not take the name
            // from the others.
            let principal = s
                .accounts
                .iter()
                .find(|a| a.id == account.id)
                .is_some_and(is_principal);
            let both_principals = |a: &Account| principal && is_principal(a);
            if others()
                .filter(|a| !both_principals(a))
                .any(|a| names_of(a).contains(&lower))
            {
                return Err(name_taken(name));
            }
            // Each harness keeps one: its agents go to no other harness's.
            let engine = s
                .accounts
                .iter()
                .find(|a| a.id == account.id)
                .map(|a| a.engine);
            if !account.active && !others().any(|a| a.active && Some(a.engine) == engine) {
                bail!(tr!(
                    "Il faut au moins un compte actif par harness.",
                    "Each harness needs at least one active account."
                ));
            }
            let Some(a) = s.accounts.iter_mut().find(|a| a.id == account.id) else {
                return Err(unknown(&account.id));
            };
            a.name = name.to_string();
            a.claude_path = account.claude_path.trim().to_string();
            a.active = account.active;
            // What its new agents go with (ESC-128); empty: its harness's default.
            a.default_model = account.default_model.trim().to_string();
            a.default_effort = account.default_effort.trim().to_string();
            a.default_mode = account.default_mode.trim().to_string();
            a.daily_budget = match account.daily_budget {
                Some(b) if !b.is_finite() || b < 0.0 => bail!(tr!(
                    "Le budget par jour est un montant en dollars, positif.",
                    "The daily budget is an amount in dollars, positive."
                )),
                // 0 or none: no budget.
                b => b.filter(|b| *b > 0.0),
            };
            // Principal too: a project none serves then starts nothing (`serves_any`).
            a.projects.clear();
            for p in account
                .projects
                .iter()
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
            {
                if !a.projects.iter().any(|x| x == p) {
                    a.projects.push(p.to_string());
                }
            }
            Ok(())
        })?;
        Ok(accounts)
    }

    /// The accounts put in the order of `ids` (each account's once).
    pub fn reorder_claude_accounts(self: &Arc<Self>, ids: &[String]) -> Result<Vec<Account>> {
        let (_, accounts) = self.change_claude_accounts(|s| {
            let mut given: Vec<&str> = ids.iter().map(String::as_str).collect();
            let mut have: Vec<&str> = s.accounts.iter().map(|a| a.id.as_str()).collect();
            given.sort_unstable();
            have.sort_unstable();
            if given != have {
                bail!(tr!(
                    "La liste des comptes a changé : ferme les réglages et rouvre-les.",
                    "The list of accounts changed: close the settings and open them again."
                ));
            }
            s.accounts
                .sort_by_key(|a| ids.iter().position(|id| *id == a.id));
            Ok(())
        })?;
        Ok(accounts)
    }

    /// « Ordre des harness » set to `order` (each harness once, the ones it leaves out after it);
    /// the order as it then is.
    pub fn set_engine_order(self: &Arc<Self>, order: &[EngineId]) -> Result<Vec<EngineId>> {
        self.change_claude_accounts(|s| {
            s.engine_order = order.to_vec();
            Ok(())
        })?;
        Ok(self.settings.read().engine_order.clone())
    }

    /// The account removed from the settings, its folder left on the disk (its sign-in and its
    /// sessions with it), but for the MCP server's entry in its `.claude.json` (`withdraw_removed`).
    /// The machine's too (ESC-128): its harness stays signed in there. Never an account an agent
    /// not archived still runs on.
    pub fn remove_claude_account(self: &Arc<Self>, id: &str) -> Result<Vec<Account>> {
        let (gone, accounts) = self.change_claude_accounts(|s| {
            let at = s
                .accounts
                .iter()
                .position(|a| a.id == id)
                .ok_or_else(|| unknown(id))?;
            // An archived agent restored later goes to another account of its harness
            // (`Core::agent_account`).
            let n = self
                .agents
                .read()
                .values()
                .filter(|h| {
                    let rt = h.lock();
                    rt.meta.account == id && !rt.meta.archived
                })
                .count();
            if n > 0 {
                bail!(tr_n!(
                    n,
                    "Le compte sert encore à {n} agent.",
                    "Le compte sert encore à {n} agents.",
                    "The account is still used by {n} agent.",
                    "The account is still used by {n} agents.",
                    n = n
                ));
            }
            let gone = s.accounts.remove(at);
            Ok(gone)
        })?;
        // No project prefers it any more.
        let changed: Vec<crate::model::Project> = self
            .projects
            .write()
            .iter_mut()
            .filter(|p| p.account == id)
            .map(|p| {
                p.account.clear();
                p.clone()
            })
            .collect();
        if !changed.is_empty() {
            for project in changed {
                self.hub.emit(crate::model::UiEvent::Project { project });
            }
            self.request_save();
            // What held their tickets back is theirs no more.
            self.schedule();
        }
        // Its folder stays on the disk, and with it the entry that holds the server's token.
        // A Cursor or Vibe account's key leaves the keychain with it (its folder stays, as any
        // account's).
        if by_key(&gone) {
            remember_key(&gone, None);
            if let Err(e) = self.secrets.delete(&key_entry(gone.engine, &gone.id)) {
                log::warn!(
                    "{} account {}: the system keychain did not forget its key: {e:#}",
                    gone.engine.as_str(),
                    gone.id
                );
            }
        }
        self.withdraw_removed(gone);
        Ok(accounts)
    }

    /// The Claude account of an agent that has not started: `account` (an active one), or the
    /// one a new agent of its project goes to when empty (« Automatique »). Its warm process, which
    /// runs on the old one, is stopped and started again on the new. Once it has a first message
    /// its conversation is filed in its account: it stays there.
    pub async fn set_agent_account(self: &Arc<Self>, id: &str, account: &str) -> Result<()> {
        let h = self.agent(id)?;
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        let settings = self.settings.read().clone();
        let (project, engine) = {
            let rt = h.lock();
            (rt.meta.project_id.clone(), rt.meta.engine)
        };
        let target = match account.trim() {
            "" => self.account_for_new(&project, engine)?,
            wanted => {
                let found = settings.accounts.iter().find(|a| a.id == wanted);
                // An account of another harness is not one this agent can run on.
                let found = found
                    .filter(|a| a.engine == engine)
                    .ok_or_else(|| unknown(wanted))?;
                if !found.active {
                    return Err(switched_off(found));
                }
                if !found.serves(Some(&project)) {
                    return Err(reserved(found));
                }
                found.id.clone()
            }
        };
        let stopped = {
            let mut rt = h.lock();
            // A copy resumes its original's session, filed in the original's account.
            let started = rt.meta.session_id.is_some()
                || rt.meta.fork_of.is_some()
                || rt.meta.prompts > 0
                || rt.meta.status.is_active();
            if started {
                bail!(tr!(
                    "Le compte d’un agent ne change plus après son premier message.",
                    "An agent’s account can’t change after its first message."
                ));
            }
            if rt.meta.account == target {
                return Ok(());
            }
            rt.meta.account = target;
            rt.detach()
        };
        if let Some(p) = stopped {
            p.close_input();
        }
        self.emit_agent(&h);
        self.request_save();
        self.warm(id);
        Ok(())
    }

    /// « Reprendre sur <compte> »: the agent stopped by the usage limit goes on, with its session,
    /// on the account `account` (an active one, other than its own), sent "continue" as the
    /// automatic resume sends it. The resume it waited for is dropped. If the turn that follows
    /// fails there, the agent remembers where it came from (`back_to_previous_account`).
    pub async fn resume_on_account(self: &Arc<Self>, id: &str, account: &str) -> Result<()> {
        let Moved { from, resume_at } = self.move_session(id, account).await?;
        let to = get(&self.settings.read(), account);
        self.with_agent(id, |rt, fx| {
            rt.meta.moved_from = Some(from.id.clone());
            // Where it waits again if it goes back (the move dropped it).
            rt.meta.moved_resume_at = resume_at;
            let lang = crate::i18n::ui();
            rt.notice(
                "info",
                tr!(
                    "L’agent reprend sur le compte {to} (il était sur {from}).",
                    "The agent resumes on the {to} account (it was on {from}).",
                    to = name_in(lang, &to),
                    from = name_in(lang, &from)
                ),
                fx,
            );
            Ok(())
        })?;
        self.send_continue(id).await
    }

    /// « Revenir sur <compte> »: a resume on another account failed, and the agent goes back to
    /// the one it came from, its session with it (what the failed turn added included). It waits
    /// there for the reset of the usage limit it met, as it did.
    pub async fn back_to_previous_account(self: &Arc<Self>, id: &str) -> Result<()> {
        let h = self.agent(id)?;
        let previous = h.lock().meta.moved_from.clone();
        let Some(previous) = previous else {
            bail!(tr!(
                "Cet agent n’a pas changé de compte.",
                "This agent hasn’t changed account."
            ));
        };
        self.move_session(id, &previous).await?;
        let to = get(&self.settings.read(), &previous);
        // The reset it waited for, as it was: the account's windows may no longer say when it is.
        let waited = h.lock().meta.moved_resume_at.take();
        let waited = waited.filter(|_| self.settings.read().auto_resume);
        self.with_agent(id, |rt, fx| {
            rt.meta.moved_from = None;
            rt.meta.resume_at = waited;
            rt.notice(
                "info",
                tr!(
                    "L’agent revient sur le compte {to}.",
                    "The agent goes back to the {to} account.",
                    to = name_in(crate::i18n::ui(), &to)
                ),
                fx,
            );
            Ok(())
        })?;
        // Not remembered (the automatic resume was off, the reset unknown): as the windows say.
        if waited.is_none() {
            self.plan_resume(id, None);
        }
        // No reset to wait for (turned off, none known): a ticket would wait for ever.
        if h.lock().meta.resume_at.is_none() {
            self.resume_lost(id);
        }
        Ok(())
    }

    /// The agent's session moved to the account `target`, an active one other than its own: its
    /// process is stopped, and gone (never two on one session), then the session is copied into
    /// the folder of `target` (`copy_session`), then the agent's account is changed and the resume
    /// it planned dropped. Nothing is changed when it fails before. The account it left, and the
    /// resume it dropped.
    pub(crate) async fn move_session(self: &Arc<Self>, id: &str, target: &str) -> Result<Moved> {
        let h = self.agent(id)?;
        // Its process is not started again meanwhile (a message, a warm-up wait for the lock).
        let lock = self.spawn_lock(id);
        let _guard = lock.lock().await;
        let settings = self.settings.read().clone();
        let to = settings
            .accounts
            .iter()
            .find(|a| a.id == target)
            .ok_or_else(|| unknown(target))?;
        if !to.active {
            return Err(switched_off(to));
        }
        if !to.serves(Some(&h.lock().meta.project_id)) {
            return Err(reserved(to));
        }
        // Claude Code finds a session by the folder it runs in, which WSL names in its own way
        // (`/mnt/c/…`): one does not go on from Windows to WSL, nor back.
        let in_wsl = |a: &Account| program(a, &settings).is_some_and(|p| p.wsl().is_some());
        // The agent's account read first: no lock held while a `claude` of WSL is looked for.
        let current = get(&settings, &h.lock().meta.account);
        if in_wsl(&current) != in_wsl(to) {
            return Err(other_side(crate::i18n::ui(), to, in_wsl(to)));
        }
        let (from, session, stopped) = {
            let mut rt = h.lock();
            let from = get(&settings, &rt.meta.account);
            if from.id == to.id {
                bail!(tr!(
                    "L’agent tourne déjà sur le compte « {name} ».",
                    "The agent already runs on the “{name}” account.",
                    name = name_in(crate::i18n::ui(), &from)
                ));
            }
            if rt.meta.status.is_active() {
                bail!(tr!(
                    "L’agent travaille encore : reprends quand son tour est fini.",
                    "The agent is still working: resume once its turn is over."
                ));
            }
            let Some(session) = rt.meta.session_id.clone() else {
                bail!(tr!(
                    "Cette conversation n’a pas encore de session à reprendre.",
                    "This conversation has no session to resume yet."
                ));
            };
            (from, session, rt.detach())
        };
        if let Some(p) = stopped {
            stop_and_wait(&p).await?;
        }
        // Mistral Vibe keeps the session in the agent's own `VIBE_HOME`, whatever its account (only
        // its key changes): nothing to copy (ESC-267).
        // Antigravity CLI the same, in the agent's own home (ESC-271).
        if !engine::by_id(from.engine).sessions_in_agent_home() {
            let (from_dir, to_dir) = (config_dir(&from, &settings), config_dir(to, &settings));
            tokio::task::spawn_blocking(move || copy_session(&from_dir, &to_dir, &session))
                .await
                .map_err(|e| anyhow::anyhow!(e))??;
        }
        let resume_at = {
            let mut rt = h.lock();
            rt.meta.account = to.id.clone();
            let lang = crate::i18n::ui();
            rt.fact(crate::facts::Body::AccountSwitched {
                from: name_in(lang, &from),
                to: name_in(lang, to),
            });
            // It goes on now: nothing left to wait for.
            rt.meta.resume_at.take()
        };
        self.emit_agent(&h);
        self.request_save();
        Ok(Moved { from, resume_at })
    }

    /// Each active account's `claude` looked for once, at the start (see `wsl::find`).
    pub fn warm_claude(&self) {
        let settings = self.settings.read().clone();
        for account in settings.accounts.iter().filter(|a| a.active) {
            let _ = program(account, &settings);
        }
    }

    /// The account new agents of `engine` would go to (the current one of the harness, as the
    /// status bar shows it), else its Principal.
    pub(crate) fn current_account(&self, engine: EngineId) -> Account {
        let settings = self.settings.read().clone();
        let id = self.usage.lock().current.get(&engine).cloned();
        let id = id.unwrap_or_else(|| principal_id(engine));
        match settings.accounts.iter().find(|a| a.id == id) {
            Some(a) => a.clone(),
            None => principal_of(engine),
        }
    }

    /// Every harness with what is found of it here, for the account new agents of it would go
    /// to: its program, its version, whether it is signed in, its state (`engine::registry`).
    pub async fn engines(&self) -> Vec<engine::registry::EngineInfo> {
        use engine::registry;
        let settings = self.settings.read().clone();
        let mut out = Vec::new();
        for harness in registry::all() {
            let account = self.current_account(harness.id());
            let found = {
                let (account, settings) = (account.clone(), settings.clone());
                tokio::task::spawn_blocking(move || program(&account, &settings))
                    .await
                    .ok()
                    .flatten()
            };
            let version = match &found {
                Some(p) => registry::version(harness, p).await,
                None => None,
            };
            let connected = found.is_some()
                && self
                    .claude_account_status(&account.id)
                    .await
                    .is_ok_and(|s| s.connected);
            let (state, mut note) = registry::state_of(
                crate::i18n::ui(),
                harness,
                found.is_some(),
                version.as_deref(),
                connected,
            );
            // Claude Code not found: its installation cut half-way, or what installs it (ESC-133).
            if found.is_none() {
                let repair =
                    tokio::task::spawn_blocking(move || harness.missing_repair(crate::i18n::ui()))
                        .await
                        .ok()
                        .flatten();
                if repair.is_some() {
                    note = repair;
                }
            }
            out.push(registry::EngineInfo {
                id: harness.id(),
                name: harness.name(),
                located: engine::Located::of(found),
                version,
                tested: harness.tested(),
                state,
                note,
                caps: harness.caps(),
            });
        }
        out
    }

    /// What stopped the sign-in of account `id` before it was signed in, when it is known
    /// (ESC-133): its `claude` not found, or for Claude Code an installation of it cut half-way,
    /// each with what to do. None: nothing more to say than that it stopped.
    pub fn sign_in_ended(&self, id: &str) -> Option<String> {
        let settings = self.settings.read().clone();
        let account = settings.accounts.iter().find(|a| a.id == id)?.clone();
        let lang = crate::i18n::ui();
        if program(&account, &settings).is_none() {
            return Some(engine::missing_line(account.engine, lang));
        }
        engine::by_id(account.engine).interrupted_line(lang)
    }

    /// Whether Claude Code is found for the account new agents would go to (the current one): its
    /// own `claude`, else the settings'.
    pub fn claude_found(&self) -> bool {
        self.engine_found(EngineId::CLAUDE)
    }

    /// Whether the program of `engine` is found for the account its new agents would go to.
    pub fn engine_found(&self, engine: EngineId) -> bool {
        let settings = self.settings.read().clone();
        program(&self.current_account(engine), &settings).is_some()
    }

    /// The harnesses whose program is found, in the registry's order: those the interface offers
    /// (a harness that is not found shows nowhere but in Réglages › Harness).
    pub fn engines_found(&self) -> Vec<crate::model::EngineId> {
        engine::registry::all()
            .into_iter()
            .filter(|h| self.engine_found(h.id()))
            .map(|h| h.id())
            .collect()
    }

    /// Whether the account is signed in, and with which email.
    pub async fn claude_account_status(&self, id: &str) -> Result<AccountStatus> {
        let account = self.claude_account(id)?;
        Ok(self.status_of(&account).await)
    }

    /// Whether `account` (one of the settings, or the machine's not added yet) is signed in.
    pub(crate) async fn status_of(&self, account: &Account) -> AccountStatus {
        let settings = self.settings.read().clone();
        engine::by_id(account.engine)
            .account_status(account, &settings)
            .await
    }

    /// « Se connecter… »: the account's `claude`, in an interactive terminal of `size`, with its
    /// folder (`launch_env`): the user signs in there. Its output goes to `on_data`; its end is
    /// told to the window (`TerminalExit`). Killed like any terminal (`term_kill`).
    pub fn claude_login(
        self: &Arc<Self>,
        id: &str,
        size: (u16, u16),
        on_data: impl Fn(Vec<u8>) + Send + 'static,
    ) -> Result<TermInfo> {
        let account = self.claude_account(id)?;
        let settings = self.settings.read().clone();
        let program = program(&account, &settings).ok_or_else(|| {
            anyhow::anyhow!(engine::missing_line(account.engine, crate::i18n::ui()))
        })?;
        // Its own command of sign-in (`codex login`), else the program itself (Claude Code asks).
        let args: Vec<String> = store(account.engine)
            .login
            .iter()
            .map(|a| a.to_string())
            .collect();
        let info = TermInfo {
            id: crate::model::new_id(),
            // No project's: the window lists it nowhere.
            project_id: String::new(),
            name: account.name.clone(),
            shell: engine::by_id(account.engine).login_shell(),
        };
        // Run from the home folder, as the user would.
        let (_, home) = app_env();
        let mut env = [settings.claude_env(), launch_env(&account)].concat();
        // What its harness sets first (Gemini CLI: its interface straight to the Google sign-in).
        engine::by_id(account.engine).before_login(&account, &mut env);
        if let Err(e) = crate::pty::open_browser_apart(&self.data.browser_opener(), &mut env) {
            log::warn!(
                "sign-in of account {id}: no browser opener, its end closes the browser: {e}"
            );
        }
        let (me, term) = (Arc::downgrade(self), info.id.clone());
        let signed = account.id.clone();
        self.pty.spawn_program(
            info.clone(),
            &program,
            &args,
            &home.to_string_lossy(),
            size,
            env,
            // The browser it opened for the sign-in stays open when the sign-in ends.
            true,
            on_data,
            move |code| {
                if let Some(core) = me.upgrade() {
                    core.hub
                        .emit(crate::model::UiEvent::TerminalExit { id: term, code });
                    // Signed in: the models of its account (ESC-124, ESC-186).
                    tauri::async_runtime::spawn(
                        async move { core.read_account_models(&signed).await },
                    );
                }
            },
        )?;
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::QuotaMeter;
    use crate::paths::test_dir;

    fn account(id: &str, dir: &str) -> Account {
        Account {
            engine: EngineId::CLAUDE,
            id: id.into(),
            name: id.to_uppercase(),
            config_dir: dir.into(),
            claude_path: String::new(),
            active: true,
            projects: Vec::new(),
            daily_budget: None,
            ..Default::default()
        }
    }

    fn ids(s: &Settings) -> Vec<&str> {
        s.accounts.iter().map(|a| a.id.as_str()).collect()
    }

    #[test]
    fn a_first_installation_has_no_account_for_any_harness() {
        // ESC-128: no Principal made, for Claude Code either; the settings start brought up to date.
        let mut s = Settings::default();
        assert_eq!(s.accounts_version, ACCOUNTS_VERSION);
        normalize(&mut s);
        assert!(s.accounts.is_empty());
        assert!(s.engine_order.is_empty());
        for e in engine::registry::all() {
            assert!(!has_any(&s, e.id()), "{}", e.id());
            assert!(!serves_any(&s, e.id(), None), "{}", e.id());
        }
        assert_eq!(pick(&s, &[], 95, None, NOW), None);
        // Read from a file that says nothing of them: the same, but to be brought up to date.
        let mut read: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(read.accounts_version, 0);
        normalize(&mut read);
        assert!(read.accounts.is_empty());
    }

    #[test]
    fn a_list_without_principal_stays_as_it_is() {
        let pro = account("pro", r"C:\Users\x\.escouade\claude\pro");
        let mut s = Settings {
            accounts: vec![pro.clone()],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(ids(&s), ["pro"]);
        assert_eq!(s.accounts[0], pro);
        assert_eq!(s.engine_order, [EngineId::CLAUDE]);
    }

    #[test]
    fn a_gemini_account_goes_by_its_google_sign_in_or_its_key_never_by_the_apps() {
        // ESC-275: none by a key or Vertex AI of the app's.
        let inherited = |v: &str| {
            matches!(
                v,
                "GEMINI_API_KEY" | "GOOGLE_GENAI_USE_VERTEXAI" | "GOOGLE_APPLICATION_CREDENTIALS"
            )
        };
        let dir = test_dir("esc275-gemini-google");
        let google = Account {
            id: "gemini-perso".into(),
            name: "Perso".into(),
            engine: EngineId::GEMINI,
            config_dir: dir.to_string_lossy().into_owned(),
            ..Default::default()
        };
        assert!(!by_key(&google) && takes_key(&google));
        let env = launch_env_with(&google, &inherited);
        let creds = crate::engine::gemini::sign_in_file(&dir)
            .to_string_lossy()
            .into_owned();
        assert!(env.contains(&("GOOGLE_APPLICATION_CREDENTIALS".into(), creds.clone())));
        assert!(env.contains(&("GEMINI_API_KEY".into(), String::new())));
        assert!(env.contains(&("GOOGLE_GENAI_USE_VERTEXAI".into(), String::new())));
        assert!(env.contains(&("GEMINI_CLI_HOME".into(), dir.to_string_lossy().into_owned())));
        assert!(!env.contains(&("GOOGLE_APPLICATION_CREDENTIALS".into(), String::new())));
        // Not signed in until `gemini` wrote its sign-in in its folder (never read).
        let refused = key_ready(&google).unwrap_err().to_string();
        assert!(
            refused.contains("Se connecter") || refused.contains("Sign in"),
            "{refused}"
        );
        std::fs::create_dir_all(dir.join(".gemini")).unwrap();
        std::fs::write(&creds, "{}").unwrap();
        assert!(key_ready(&google).is_ok());
        // By its key: the key alone.
        let key_dir = test_dir("esc275-gemini-key");
        let keyed = Account {
            id: "gemini-cle".into(),
            engine: EngineId::GEMINI,
            config_dir: key_dir.to_string_lossy().into_owned(),
            api_key: true,
            ..Default::default()
        };
        assert!(by_key(&keyed) && key_ready(&keyed).is_err());
        remember_key(&keyed, Some("AIzaCleGemini0000000abcd"));
        let env = launch_env_with(&keyed, &inherited);
        assert!(env.contains(&("GEMINI_API_KEY".into(), "AIzaCleGemini0000000abcd".into())));
        assert!(env.contains(&("GOOGLE_APPLICATION_CREDENTIALS".into(), String::new())));
        assert!(!env.contains(&("GEMINI_API_KEY".into(), String::new())));
        assert!(key_ready(&keyed).is_ok());
        assert_eq!(
            key_entry(EngineId::GEMINI, "gemini-cle"),
            "gemini:gemini-cle"
        );
        remember_key(&keyed, None);
        // The machine's account: what the app has, as it is.
        let machine = principal_of(EngineId::GEMINI);
        assert!(!by_key(&machine) && !takes_key(&machine));
        let env = launch_env_with(&machine, &inherited);
        assert!(!env
            .iter()
            .any(|(k, _)| k.starts_with("GEMINI_API") || k.starts_with("GOOGLE_")));
        assert!(key_ready(&machine).is_ok());
        // Saved only when it goes by its key.
        let saved = serde_json::to_value(&google).unwrap();
        assert!(saved.get("apiKey").is_none());
        assert_eq!(serde_json::to_value(&keyed).unwrap()["apiKey"], true);
    }

    #[test]
    fn the_antigravity_sign_in_is_its_keychains_entry_by_name_alone() {
        // `cmdkey /list:gemini:antigravity`, its names only (ESC-271), in French and English.
        assert!(cmdkey_lists(
            "\r\nInformations d'identification actuellement stockées pour gemini:antigravity :\r\n\r\n    Cible : gemini:antigravity\r\n    Type : Générique \r\n"
        ));
        assert!(cmdkey_lists(
            "Currently stored credentials for gemini:antigravity:\n\n    Target: LegacyGeneric:target=gemini:antigravity\n"
        ));
        assert!(!cmdkey_lists(
            "Informations d'identification actuellement stockées pour gemini:antigravity :\r\n\r\n* AUCUN *\r\n"
        ));
        assert!(!cmdkey_lists(
            "Currently stored credentials for gemini:antigravity:\n\n* NONE *\n"
        ));
        // Never looked at in the unit tests: as far as can be told.
        assert_eq!(agy_signed_in(), None);
        // Its Principal by the machine's Google sign-in, never a key of the app's; another account
        // by its own key alone.
        let machine = principal_of(EngineId::AGY);
        let inherited = |v: &str| v == "GEMINI_API_KEY" || v == "GOOGLE_API_KEY";
        let env = launch_env_with(&machine, &inherited);
        assert!(env.contains(&("GEMINI_API_KEY".into(), String::new())));
        assert!(env.contains(&("GOOGLE_API_KEY".into(), String::new())));
        assert!(!by_key(&machine));
        let pro = Account {
            id: "agy-pro".into(),
            engine: EngineId::AGY,
            config_dir: test_dir("esc271-agy-pro").to_string_lossy().into_owned(),
            ..Default::default()
        };
        assert!(by_key(&pro) && key_ready(&pro).is_err());
        remember_key(&pro, Some("AIzaCleEssai00000000abcd"));
        let env = launch_env_with(&pro, &inherited);
        assert!(env.contains(&("GEMINI_API_KEY".into(), "AIzaCleEssai00000000abcd".into())));
        assert!(env.contains(&("GOOGLE_API_KEY".into(), String::new())));
        assert!(!env.contains(&("GEMINI_API_KEY".into(), String::new())));
        assert!(key_ready(&pro).is_ok());
        assert_eq!(key_entry(EngineId::AGY, "agy-pro"), "agy:agy-pro");
        remember_key(&pro, None);
    }

    #[test]
    fn principal_keeps_its_place_and_never_has_a_folder_of_its_own() {
        let mut s = Settings {
            accounts: vec![
                account("pro", "/Users/x/.escouade/claude/pro"),
                Account {
                    name: "  ".into(),
                    ..account("principal", "/Users/x/.claude")
                },
            ],
            ..Default::default()
        };
        normalize(&mut s);
        // The order is the user's priority.
        assert_eq!(ids(&s), ["pro", "principal"]);
        assert_eq!(s.accounts[1].config_dir, "");
        assert_eq!(s.accounts[1].name, "Principal");
        // A name of the user's own stays.
        s.accounts[1].name = "Perso".into();
        normalize(&mut s);
        assert_eq!(s.accounts[1].name, "Perso");
    }

    #[test]
    fn ids_given_twice_or_none_are_made_unique() {
        let mut s = Settings {
            accounts: vec![
                account("principal", ""),
                account("pro", "/a"),
                account("pro", "/b"),
                account("principal", "/c"),
                Account {
                    name: "Équipe".into(),
                    ..account("", "/d")
                },
                Account {
                    name: "Principal".into(),
                    ..account(" ", "/e")
                },
            ],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(
            ids(&s),
            [
                "principal",
                "pro",
                "pro-2",
                "principal-2",
                "equipe",
                "principal-3"
            ]
        );
        // The first one keeps the id; the others keep their folder.
        let dirs: Vec<&str> = s.accounts.iter().map(|a| a.config_dir.as_str()).collect();
        assert_eq!(dirs, ["", "/a", "/b", "/c", "/d", "/e"]);
    }

    #[test]
    fn an_id_given_once_keeps_it_when_another_is_told_apart() {
        // `x-2` is already some account's: the second `x` takes the next one free, and `x-2`
        // stays as it is (its agents with it).
        let mut s = Settings {
            accounts: vec![account("x", "/a"), account("x", "/b"), account("x-2", "/c")],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(ids(&s), ["x", "x-3", "x-2"]);
        let dirs: Vec<&str> = s.accounts.iter().map(|a| a.config_dir.as_str()).collect();
        assert_eq!(dirs, ["/a", "/b", "/c"]);
    }

    #[test]
    fn the_first_account_of_a_harness_is_made_active_when_none_is() {
        let mut s = Settings {
            accounts: vec![
                Account {
                    active: false,
                    ..account("pro", "/a")
                },
                Account {
                    active: false,
                    ..account("principal", "")
                },
            ],
            ..Default::default()
        };
        normalize(&mut s);
        let active: Vec<bool> = s.accounts.iter().map(|a| a.active).collect();
        assert_eq!(active, [true, false]);
        // Another account active: Principal may rest.
        s.accounts[0].active = true;
        s.accounts[1].active = false;
        normalize(&mut s);
        let active: Vec<bool> = s.accounts.iter().map(|a| a.active).collect();
        assert_eq!(active, [true, false]);
    }

    #[test]
    fn an_account_saved_with_only_part_of_its_fields_is_active() {
        let a: Account = serde_json::from_value(serde_json::json!({ "id": "pro" })).unwrap();
        assert_eq!(a, account_named("pro", ""));
        let v = serde_json::to_value(account("pro", "/a")).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "id": "pro", "name": "PRO", "engine": "claude", "configDir": "/a",
                                "claudePath": "", "active": true, "projects": [] })
        );
    }

    fn account_named(id: &str, name: &str) -> Account {
        Account {
            name: name.into(),
            ..account(id, "")
        }
    }

    #[test]
    fn principal_is_launched_without_claude_config_dir_another_account_with_its_folder() {
        // Every claude, whatever its account, without its own update (ESC-133).
        let no_update = ("DISABLE_AUTOUPDATER".to_string(), "1".to_string());
        assert_eq!(launch_env(&principal()), vec![no_update.clone()]);
        // Even with a folder written by hand in its settings.
        assert_eq!(
            launch_env(&account("principal", "/Users/x/.claude")),
            vec![no_update.clone()]
        );
        let dir = r"C:\Users\x\.escouade\claude\pro";
        assert_eq!(
            launch_env(&account("pro", dir)),
            vec![
                ("CLAUDE_CONFIG_DIR".to_string(), dir.to_string()),
                no_update.clone()
            ]
        );
        // Another account with no folder runs as Principal does.
        assert_eq!(launch_env(&account("pro", "")), vec![no_update.clone()]);
        assert_eq!(launch_env(&account("pro", "  ")), vec![no_update]);
    }

    #[test]
    fn inherited_identity_never_replaces_the_login_of_an_accounts_own_folder() {
        // ESC-147: a token or key in the app's environment took the place of the secondary's login.
        let all = |_: &str| true;
        let dir = "/a/pro";
        let env = launch_env_with(&account("pro", dir), &all);
        for var in IDENTITY_VARS {
            assert!(env.contains(&(var.to_string(), String::new())), "{var}");
        }
        assert!(env.contains(&(SECURE_STORAGE_VAR.to_string(), dir.to_string())));
        assert!(env.contains(&("CLAUDE_CONFIG_DIR".to_string(), dir.to_string())));
        // Nothing set, nothing added; Principal and an account with no folder keep theirs.
        assert_eq!(launch_env_with(&account("pro", dir), &|_| false).len(), 2);
        assert_eq!(launch_env_with(&principal(), &all).len(), 1);
        assert_eq!(launch_env_with(&account("pro", ""), &all).len(), 1);
    }

    /// ESC-43: a Cursor account added by its key goes by it, never by a token of the app's; the
    /// machine's by its sign-in, never by a key or token of the app's (empty: none, for Cursor).
    #[test]
    fn a_cursor_account_goes_by_its_own_key_and_the_machines_by_its_sign_in() {
        let all = |_: &str| true;
        let empty = |v: &str| (v.to_string(), String::new());
        let dir = crate::paths::test_dir("esc43-launch-env");
        let pro = Account {
            id: "pro".into(),
            engine: EngineId::CURSOR,
            config_dir: dir.to_string_lossy().into_owned(),
            ..Default::default()
        };
        // No key known: none given, the app's emptied.
        let env = launch_env_with(&pro, &all);
        assert!(env.contains(&empty(CURSOR_KEY_VAR)) && env.contains(&empty(CURSOR_TOKEN_VAR)));
        assert!(key_ready(&pro).is_err());
        remember_key(&pro, Some("cle-essai-0000000000abcd"));
        let env = launch_env_with(&pro, &all);
        assert!(env.contains(&(CURSOR_KEY_VAR.into(), "cle-essai-0000000000abcd".into())));
        assert!(env.contains(&empty(CURSOR_TOKEN_VAR)));
        assert!(!env.contains(&empty(CURSOR_KEY_VAR)));
        assert!(key_ready(&pro).is_ok());
        assert_eq!(key_hint("cle-essai-0000000000abcd"), "clé …abcd");
        // Its folder, as the variable of the harness says.
        assert!(env.contains(&("CURSOR_CONFIG_DIR".into(), pro.config_dir.clone())));
        // The machine's: the app's key and token emptied, nothing else; none set, nothing.
        let machine = principal_of(EngineId::CURSOR);
        let env = launch_env_with(&machine, &all);
        assert!(env.contains(&empty(CURSOR_KEY_VAR)) && env.contains(&empty(CURSOR_TOKEN_VAR)));
        assert!(launch_env_with(&machine, &|_| false).is_empty());
        assert!(key_ready(&machine).is_ok() && !by_key(&machine) && by_key(&pro));
        remember_key(&pro, None);
        assert_eq!(account_key(&pro), None);
    }

    #[test]
    fn principals_folder_is_the_apps_claude_config_dir_else_the_home_one() {
        let home = Path::new("/Users/x");
        assert_eq!(
            config_dir_with(&principal(), None, home),
            home.join(".claude")
        );
        assert_eq!(
            config_dir_with(&principal(), Some("/tmp/claude-e2e"), home),
            PathBuf::from("/tmp/claude-e2e")
        );
        // Set empty: as if it were not.
        assert_eq!(
            config_dir_with(&principal(), Some(""), home),
            home.join(".claude")
        );
        // Another account's folder is its own, whatever the app's.
        let pro = account("pro", "/Users/x/.escouade/claude/pro");
        assert_eq!(
            config_dir_with(&pro, Some("/tmp/claude-e2e"), home),
            PathBuf::from("/Users/x/.escouade/claude/pro")
        );
        assert_eq!(
            config_dir_with(&pro, None, home),
            PathBuf::from("/Users/x/.escouade/claude/pro")
        );
    }

    #[test]
    fn with_a_claude_of_wsl_principal_keeps_its_folder_in_linux() {
        // Only where WSL has a distribution (not on the CI's Windows runners).
        let machine = crate::wsl::Machine::this();
        let (Some(_), Some(distro)) = (&machine.exe, machine.default.clone()) else {
            eprintln!("no WSL distribution: skipped");
            return;
        };
        // Any program of the distribution will do: `sh` is in all of them.
        let settings = Settings {
            claude_path: "/bin/sh".into(),
            ..Default::default()
        };
        let principal = get(&settings, PRINCIPAL);
        let wsl = program(&principal, &settings)
            .and_then(|p| p.wsl().cloned())
            .expect("a program of WSL");
        let home = wsl.windows_path(&wsl.home);
        assert!(
            home.to_string_lossy()
                .starts_with(&format!(r"\\wsl.localhost\{distro}\")),
            "{home:?}"
        );
        assert_eq!(config_dir(&principal, &settings), home.join(".claude"));
        assert_eq!(
            claude_json(&principal, &settings),
            home.join(".claude.json")
        );
        assert_eq!(
            crate::usage::sign_in(&principal, &settings).dir,
            home.join(".claude")
        );
        // An account of its own keeps its folder, on Windows (its Linux path at the launch).
        let pro = account("pro", r"C:\Users\me\.escouade\claude\pro");
        assert_eq!(
            config_dir(&pro, &settings),
            PathBuf::from(r"C:\Users\me\.escouade\claude\pro")
        );
    }

    #[test]
    fn a_folder_of_wsl_is_copied_without_the_links_windows_cannot_follow() {
        // Only where WSL has a distribution (not on the CI's Windows runners).
        let machine = crate::wsl::Machine::this();
        let (Some(exe), Some(distro)) = (machine.exe.clone(), machine.default.clone()) else {
            eprintln!("no WSL distribution: skipped");
            return;
        };
        let linux = format!("/var/tmp/escouade-copy-{}", std::process::id());
        let made = std::process::Command::new(&exe)
            .args(["-d", &distro, "--exec", "/bin/sh", "-c"])
            .arg(format!(
                "rm -rf {linux} && mkdir -p {linux}/skills/a && echo x > {linux}/skills/a/SKILL.md && ln -s /etc/hostname {linux}/skills/linked && ln -s a/SKILL.md {linux}/skills/relative && ln -s a {linux}/skills/folder"
            ))
            .status()
            .unwrap();
        assert!(made.success());
        let to = test_dir("accounts-copy-wsl").join("skills");
        let from = crate::wsl::Program {
            exe: exe.clone(),
            distro: distro.clone(),
            path: String::new(),
            home: String::new(),
            mount_root: "/mnt/".into(),
        }
        .windows_path(&format!("{linux}/skills"));
        copy_dir(&from, &to, 0).unwrap();
        assert_eq!(
            std::fs::read_to_string(to.join("a").join("SKILL.md"))
                .unwrap()
                .trim(),
            "x"
        );
        assert!(!to.join("linked").exists());
        assert!(!to.join("relative").exists());
        assert!(!to.join("folder").exists());
        let _ = std::process::Command::new(&exe)
            .args(["-d", &distro, "--exec", "rm", "-rf", &linux])
            .status();
    }

    #[test]
    fn a_session_does_not_cross_between_windows_and_wsl_in_english() {
        let pro = account("pro", "/a");
        assert_eq!(
            other_side(Lang::En, &pro, true).to_string(),
            "The “PRO” account runs Claude Code in WSL, the agent’s on Windows: its session doesn’t go from one to the other."
        );
        assert_eq!(
            other_side(Lang::En, &pro, false).to_string(),
            "The “PRO” account runs Claude Code on Windows, the agent’s in WSL: its session doesn’t go from one to the other."
        );
    }

    #[test]
    fn an_account_runs_its_own_claude_else_the_settings_one() {
        let dir = test_dir("accounts-program");
        let (own, set) = (dir.join("claude-pro.cmd"), dir.join("claude.cmd"));
        std::fs::write(&own, "").unwrap();
        std::fs::write(&set, "").unwrap();
        let settings = Settings {
            claude_path: set.to_string_lossy().into(),
            ..Default::default()
        };
        let path = |p: &Path| p.to_string_lossy().to_string();
        let with = |claude_path: String| Account {
            claude_path,
            ..account("pro", "/a")
        };
        let native = |p: &PathBuf| Some(engine::Program::Native(p.clone()));
        assert_eq!(program(&with(path(&own)), &settings), native(&own));
        assert_eq!(program(&with(String::new()), &settings), native(&set));
        assert_eq!(program(&with("  ".into()), &settings), native(&set));
        // Another harness never runs the settings' claude: without a program of its own, none (the
        // unit tests find no Cursor CLI on the machine).
        let cursor = Account {
            engine: EngineId::CURSOR,
            ..with(String::new())
        };
        assert_eq!(program(&cursor, &settings), None);
        // Its own named and not there: not found, rather than another one.
        let gone = path(&dir.join("absent.cmd"));
        assert_eq!(program(&with(gone), &settings), None);
    }

    /// A Principal folder as Claude Code leaves it: what may be shared, and what never is.
    fn principal_folder(dir: &Path) -> PathBuf {
        let p = dir.join("principal");
        std::fs::create_dir_all(p.join("skills").join("revue")).unwrap();
        std::fs::write(p.join("skills").join("revue").join("SKILL.md"), "revue").unwrap();
        std::fs::create_dir_all(p.join("agents")).unwrap();
        std::fs::write(p.join("agents").join("testeur.md"), "testeur").unwrap();
        std::fs::create_dir_all(p.join("projects").join("x")).unwrap();
        std::fs::write(p.join("settings.json"), r#"{"model":"opus"}"#).unwrap();
        std::fs::write(p.join("CLAUDE.md"), "Réponds en français.").unwrap();
        std::fs::write(p.join(".credentials.json"), r#"{"claudeAiOauth":{}}"#).unwrap();
        std::fs::write(p.join(".claude.json"), r#"{"oauthAccount":{}}"#).unwrap();
        p
    }

    fn all_items() -> Vec<String> {
        SHARED_FILES
            .iter()
            .chain(SHARED_DIRS.iter())
            .map(|n| n.to_string())
            .collect()
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    fn is_link(p: &Path) -> bool {
        std::fs::symlink_metadata(p)
            .unwrap()
            .file_type()
            .is_symlink()
    }

    #[test]
    fn the_items_principal_has_are_offered_files_first() {
        let dir = test_dir("accounts-shareable");
        let p = principal_folder(&dir);
        assert_eq!(
            shareable(&p),
            ["settings.json", "CLAUDE.md", "skills", "agents"]
        );
        std::fs::create_dir_all(p.join("output-styles")).unwrap();
        // A file where a folder is expected is not one.
        std::fs::write(p.join("hooks"), "").unwrap();
        assert_eq!(
            shareable(&p),
            [
                "settings.json",
                "CLAUDE.md",
                "skills",
                "agents",
                "output-styles"
            ]
        );
        assert!(shareable(&dir.join("absent")).is_empty());
    }

    #[test]
    fn a_new_account_gets_a_folder_of_its_own_named_after_it_and_free() {
        let dir = test_dir("accounts-create");
        let root = dir.join("claude");
        let p = principal_folder(&dir);
        let mut s = Settings {
            accounts: vec![Account {
                name: "Boulot".into(),
                ..account("travail", "/a")
            }],
            ..Default::default()
        };
        normalize(&mut s);
        let made = create(&s, &root, &p, "  Équipe Pro ", &[], ShareMode::Link).unwrap();
        assert_eq!(
            made,
            Account {
                engine: EngineId::CLAUDE,
                id: "equipe-pro".into(),
                name: "Équipe Pro".into(),
                config_dir: root.join("equipe-pro").to_string_lossy().into(),
                claude_path: String::new(),
                active: true,
                projects: Vec::new(),
                daily_budget: None,
                ..Default::default()
            }
        );
        assert!(Path::new(&made.config_dir).is_absolute());
        // Made empty: nothing asked to be shared.
        assert!(names_in(&root.join("equipe-pro")).is_empty());
        // An id another account has, a folder an account removed left: taken.
        let made = create(&s, &root, &p, "Travail", &[], ShareMode::Copy).unwrap();
        assert_eq!(made.id, "travail-2");
        std::fs::create_dir_all(root.join("perso")).unwrap();
        std::fs::write(root.join("perso").join(".credentials.json"), "{}").unwrap();
        let made = create(&s, &root, &p, "Perso", &[], ShareMode::Copy).unwrap();
        assert_eq!(made.id, "perso-2");
        // The folder left keeps its sign-in.
        assert_eq!(names_in(&root.join("perso")), [".credentials.json"]);
        // Never Principal's id; a name without a letter for a slug.
        assert_eq!(
            create(&s, &root, &p, "principal 2", &[], ShareMode::Copy)
                .unwrap()
                .id,
            "principal-2"
        );
        assert_eq!(
            create(&s, &root, &p, "!!", &[], ShareMode::Copy)
                .unwrap()
                .id,
            "account"
        );
    }

    #[test]
    fn a_new_account_needs_a_name_no_other_has_and_shares_only_what_may_be() {
        let dir = test_dir("accounts-create-refused");
        let root = dir.join("claude");
        let p = principal_folder(&dir);
        let mut s = Settings {
            accounts: vec![
                principal(),
                Account {
                    name: "Pro".into(),
                    ..account("pro", "/a")
                },
            ],
            ..Default::default()
        };
        normalize(&mut s);
        let refused = |name: &str, share: &[&str]| {
            let share: Vec<String> = share.iter().map(|n| n.to_string()).collect();
            create(&s, &root, &p, name, &share, ShareMode::Link)
                .unwrap_err()
                .to_string()
        };
        assert_eq!(refused("  ", &[]), "Donne un nom au compte.");
        assert_eq!(refused("pro", &[]), "Un compte s’appelle déjà « pro ».");
        // Principal's name, in either language.
        assert_eq!(refused("Main", &[]), "Un compte s’appelle déjà « Main ».");
        assert_eq!(
            refused("Perso", &[".credentials.json"]),
            "« .credentials.json » ne se partage pas entre comptes."
        );
        assert_eq!(
            refused("Perso", &["skills", "../.claude.json"]),
            "« ../.claude.json » ne se partage pas entre comptes."
        );
        // Nothing made for an account refused.
        assert!(!root.join("perso").exists());
    }

    #[test]
    fn principals_items_are_linked_one_change_for_both_accounts() {
        let dir = test_dir("accounts-share-link");
        let root = dir.join("claude");
        let p = principal_folder(&dir);
        let mut s = Settings::default();
        normalize(&mut s);
        let made = create(&s, &root, &p, "Pro", &all_items(), ShareMode::Link).unwrap();
        let own = PathBuf::from(&made.config_dir);
        // Only what Principal has; never its sign-in, its .claude.json nor its sessions.
        assert_eq!(
            names_in(&own),
            ["CLAUDE.md", "agents", "settings.json", "skills"]
        );
        // The folders are Principal's own: a skill added there is the account's too.
        assert!(is_link(&own.join("skills")));
        assert!(is_link(&own.join("agents")));
        std::fs::write(p.join("skills").join("nouveau.md"), "x").unwrap();
        assert!(own.join("skills").join("nouveau.md").is_file());
        assert_eq!(
            std::fs::read_to_string(own.join("skills").join("revue").join("SKILL.md")).unwrap(),
            "revue"
        );
        // The files: linked too, except on Windows, where a link to a file needs administrator
        // rights: copied.
        std::fs::write(p.join("CLAUDE.md"), "Réponds en anglais.").unwrap();
        let text = std::fs::read_to_string(own.join("CLAUDE.md")).unwrap();
        if cfg!(windows) {
            assert!(!is_link(&own.join("settings.json")));
            assert_eq!(text, "Réponds en français.");
        } else {
            assert!(is_link(&own.join("settings.json")));
            assert_eq!(text, "Réponds en anglais.");
        }
        // Principal's folder is as it was.
        assert_eq!(
            names_in(&p),
            [
                ".claude.json",
                ".credentials.json",
                "CLAUDE.md",
                "agents",
                "projects",
                "settings.json",
                "skills"
            ]
        );
    }

    #[test]
    fn a_folder_with_spaces_accents_and_cmd_symbols_in_its_path_is_linked_all_the_same() {
        let dir = test_dir("accounts-link-odd");
        // Spaces and accents are quoted by Rust's own rules; `&` and `^` (with no space beside
        // them to make Rust quote the path) are what cmd.exe reads as commands.
        for (parent, from, to) in [
            ("Équipe à 100 pour cent", "source dossier", "lien é"),
            ("A&B^C(1)", "src&dir", "lien^&é"),
            // `!` is read by cmd.exe when the machine turns delayed expansion on.
            ("Bravo !HOME! !", "src!dir!", "lien!é"),
        ] {
            let src = dir.join(parent).join(from);
            std::fs::create_dir_all(&src).unwrap();
            std::fs::write(src.join("a.txt"), "dedans").unwrap();
            let dst = dir.join(parent).join(to);
            link_dir(&src, &dst).unwrap();
            assert!(std::fs::metadata(&dst).unwrap().is_dir(), "{parent}");
            assert_eq!(
                std::fs::read_to_string(dst.join("a.txt")).unwrap(),
                "dedans"
            );
            // A change of the folder is the link's.
            std::fs::write(src.join("b.txt"), "aussi").unwrap();
            assert!(dst.join("b.txt").is_file());
        }
    }

    #[test]
    fn the_command_line_of_a_junction_quotes_each_path_and_refuses_what_cmd_reads_inside_quotes() {
        let line = junction_line(
            Path::new(r"C:\Users\ada\Équipe & Co ^ (x)\dst"),
            Path::new(r"C:\Mes docs\src"),
        );
        assert_eq!(
            line.unwrap(),
            r#"mklink /J "C:\Users\ada\Équipe & Co ^ (x)\dst" "C:\Mes docs\src""#
        );
        // cmd.exe expands %NAME% inside quotes too, and a straight quote cannot be quoted.
        for bad in [r"C:\100%\x", r"C:\%TEMP%\x", "C:\\a\"b\\x"] {
            let want = format!(
                "« {bad} » contient « % » ou un guillemet droit, que cmd.exe ne sait pas lier : choisis « Copier »."
            );
            let (ok, other) = (Path::new(r"C:\ok\x"), Path::new(bad));
            assert_eq!(junction_line(other, ok).unwrap_err().to_string(), want);
            assert_eq!(junction_line(ok, other).unwrap_err().to_string(), want);
        }
    }

    #[test]
    fn the_cmd_that_makes_a_junction_reads_no_delayed_expansion_whatever_the_machine_says() {
        // With it on (a registry setting of the machine or the user), cmd.exe eats the `!` of a
        // path, even inside quotes.
        let at = |switch: &str| {
            CMD_SWITCHES
                .iter()
                .position(|s| s.eq_ignore_ascii_case(switch))
        };
        assert!(at("/v:off").is_some(), "{CMD_SWITCHES:?}");
        assert!(at("/v:off") < at("/c"), "{CMD_SWITCHES:?}");
        // `/c` takes the rest of the line as the command: it comes last.
        assert_eq!(CMD_SWITCHES.last(), Some(&"/c"));
    }

    #[cfg(windows)]
    #[test]
    fn a_folder_with_a_percent_in_its_path_is_not_linked_on_windows_and_nothing_is_made() {
        let dir = test_dir("accounts-link-percent");
        let src = dir.join("100%").join("src");
        std::fs::create_dir_all(&src).unwrap();
        let dst = dir.join("lien");
        let e = link_dir(&src, &dst).unwrap_err();
        assert!(e.to_string().ends_with("choisis « Copier »."), "{e}");
        assert!(!dst.exists());
    }

    #[test]
    fn principals_items_are_copied_each_account_then_changes_its_own() {
        let dir = test_dir("accounts-share-copy");
        let root = dir.join("claude");
        let p = principal_folder(&dir);
        let mut s = Settings::default();
        normalize(&mut s);
        let share = vec!["skills".to_string(), "settings.json".to_string()];
        let made = create(&s, &root, &p, "Pro", &share, ShareMode::Copy).unwrap();
        let own = PathBuf::from(&made.config_dir);
        assert_eq!(names_in(&own), ["settings.json", "skills"]);
        assert!(!is_link(&own.join("skills")));
        assert_eq!(
            std::fs::read_to_string(own.join("skills").join("revue").join("SKILL.md")).unwrap(),
            "revue"
        );
        assert_eq!(
            std::fs::read_to_string(own.join("settings.json")).unwrap(),
            r#"{"model":"opus"}"#
        );
        // Changed on one side only.
        std::fs::write(p.join("skills").join("nouveau.md"), "x").unwrap();
        assert!(!own.join("skills").join("nouveau.md").exists());
    }

    #[test]
    fn claude_code_keeps_an_accounts_email_in_its_claude_json() {
        let home = Path::new("/Users/x");
        let principal = principal();
        assert_eq!(
            claude_json_with(&principal, None, home),
            home.join(".claude.json")
        );
        assert_eq!(
            claude_json_with(&principal, Some(""), home),
            home.join(".claude.json")
        );
        assert_eq!(
            claude_json_with(&principal, Some("/tmp/claude-e2e"), home),
            Path::new("/tmp/claude-e2e").join(".claude.json")
        );
        let pro = account("pro", "/Users/x/.escouade/claude/pro");
        assert_eq!(
            claude_json_with(&pro, Some("/tmp/claude-e2e"), home),
            Path::new("/Users/x/.escouade/claude/pro").join(".claude.json")
        );
        let dir = test_dir("accounts-email");
        let file = dir.join(".claude.json");
        assert_eq!(email_in(&file), None);
        std::fs::write(
            &file,
            r#"{"numStartups":3,"oauthAccount":{"emailAddress":"ada@atlas.dev","organizationName":"Atlas"}}"#,
        )
        .unwrap();
        assert_eq!(email_in(&file).as_deref(), Some("ada@atlas.dev"));
        std::fs::write(&file, r#"{"numStartups":3}"#).unwrap();
        assert_eq!(email_in(&file), None);
        std::fs::write(&file, "{ pas du json").unwrap();
        assert_eq!(email_in(&file), None);
    }

    #[test]
    fn an_account_other_than_principal_needs_a_folder_written_in_full() {
        let dir = test_dir("accounts-check");
        let full = dir.join("claude").join("pro").to_string_lossy().to_string();
        let with = |config_dir: &str| {
            let mut s = Settings {
                accounts: vec![account("pro", config_dir)],
                ..Default::default()
            };
            normalize(&mut s);
            check(&s).map_err(|e| e.to_string())
        };
        assert_eq!(with(&full), Ok(()));
        assert_eq!(
            with(""),
            Err("Le compte « PRO » n’a pas de dossier de configuration.".into())
        );
        assert_eq!(
            with("claude/pro"),
            Err("Le dossier du compte « PRO » doit être un chemin complet : claude/pro".into())
        );
        // Principal has none, by design.
        let mut s = Settings::default();
        normalize(&mut s);
        assert!(check(&s).is_ok());
    }

    #[test]
    fn an_unknown_account_is_none_never_principal() {
        // ESC-128: an id the settings do not know falls back on no other account.
        let mut s = Settings {
            accounts: vec![principal(), account("pro", "/a")],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(find(&s, "pro").map(|a| a.id.as_str()), Some("pro"));
        assert_eq!(
            find(&s, "principal").map(|a| a.id.as_str()),
            Some("principal")
        );
        assert_eq!(find(&s, "parti"), None);
        assert_eq!(find(&s, ""), None);
        assert_eq!(find(&Settings::default(), "principal"), None);
        // A copy, always there, to name it: never another account in its place.
        assert_eq!(get(&s, "pro"), s.accounts[1]);
        let gone = get(&s, "parti");
        assert_eq!((gone.id.as_str(), gone.name.as_str()), ("parti", "parti"));
        assert!(!gone.active);
        assert!(own_dir(&gone).is_none());
        // A Principal not added: the machine's account of its harness, as it would be added.
        assert_eq!(get(&Settings::default(), "principal"), principal());
    }

    const NOW: i64 = 1_790_000_000_000;

    /// The account's 5-hour window, used `pct` percent until `ends`.
    fn used(id: &str, pct: f64, ends: i64) -> AccountUsage {
        AccountUsage {
            id: id.into(),
            meters: crate::model::claude_meters(
                Some(crate::model::RateWindow {
                    pct,
                    resets_at: Some(ends),
                }),
                None,
            ),
            ..Default::default()
        }
    }

    /// Principal, Pro and Team, in that order, all active.
    fn three() -> Settings {
        let mut s = Settings {
            accounts: vec![
                principal(),
                account("pro", "/pro"),
                account("team", "/team"),
            ],
            ..Default::default()
        };
        normalize(&mut s);
        s.accounts[1].name = "Pro".into();
        s.accounts[2].name = "Team".into();
        s
    }

    fn deactivate(s: &mut Settings, id: &str) {
        s.accounts.iter_mut().find(|a| a.id == id).unwrap().active = false;
    }

    #[test]
    fn a_new_agent_goes_to_the_first_active_account_under_the_threshold_in_the_order_of_the_accounts(
    ) {
        let mut s = three();
        let pick_at = |s: &Settings, usage: &[AccountUsage]| pick(s, usage, 95, None, NOW);
        // Nothing read yet: the first.
        assert_eq!(pick_at(&s, &[]).as_deref(), Some("principal"));
        let end = NOW + 60_000;
        // Under the threshold, whatever its use.
        assert_eq!(
            pick_at(&s, &[used("principal", 94.0, end)]).as_deref(),
            Some("principal")
        );
        // At it (as the autopilot counts it): the next one.
        let first_over = [used("principal", 95.0, end)];
        assert_eq!(pick_at(&s, &first_over).as_deref(), Some("pro"));
        // Its week counts as well as its 5 hours, once it is at its limit.
        let week = |pct| AccountUsage {
            id: "principal".into(),
            meters: crate::model::claude_meters(
                None,
                Some(crate::model::RateWindow {
                    pct,
                    resets_at: Some(end),
                }),
            ),
            ..Default::default()
        };
        assert_eq!(pick_at(&s, &[week(100.0)]).as_deref(), Some("pro"));
        // A week at 99 % that starts again in a minute, on pace: too near its limit, behind the
        // others (`pace::NEAR_LIMIT`); 94 %, it is not wasted.
        assert_eq!(pick_at(&s, &[week(99.0)]).as_deref(), Some("pro"));
        assert_eq!(pick_at(&s, &[week(94.0)]).as_deref(), Some("principal"));
        // The next one over as well: the one after.
        let two_over = [used("principal", 100.0, end), used("pro", 96.0, end)];
        assert_eq!(pick_at(&s, &two_over).as_deref(), Some("team"));
        // The order is the settings', not the order of the readings.
        let reversed = [used("pro", 96.0, end), used("principal", 100.0, end)];
        assert_eq!(pick_at(&s, &reversed).as_deref(), Some("team"));
        // A window already over (and the margin after it) holds nothing back.
        let ended = [used("principal", 100.0, NOW - 31_000)];
        assert_eq!(pick_at(&s, &ended).as_deref(), Some("principal"));
        // An account switched off is skipped, over or not.
        deactivate(&mut s, "principal");
        assert_eq!(pick_at(&s, &[]).as_deref(), Some("pro"));
        assert_eq!(
            pick_at(&s, &[used("pro", 100.0, end)]).as_deref(),
            Some("team")
        );
    }

    #[test]
    fn with_every_account_past_the_threshold_it_is_the_first_active_one() {
        let mut s = three();
        let end = NOW + 60_000;
        let all = [
            used("principal", 100.0, end),
            used("pro", 100.0, end),
            used("team", 100.0, end),
        ];
        assert_eq!(pick(&s, &all, 95, None, NOW).as_deref(), Some("principal"));
        deactivate(&mut s, "principal");
        assert_eq!(pick(&s, &all, 95, None, NOW).as_deref(), Some("pro"));
        // The threshold is the user's: at 100 % nothing at 99 % is past it.
        let under = [used("principal", 99.0, end), used("pro", 99.0, end)];
        assert_eq!(
            pick(&three(), &under, 100, None, NOW).as_deref(),
            Some("principal")
        );
    }

    #[test]
    fn an_account_asked_for_is_taken_when_active_whatever_its_quota() {
        let mut s = three();
        let end = NOW + 60_000;
        let over = [used("pro", 100.0, end)];
        assert_eq!(
            pick(&s, &over, 95, Some("pro"), NOW).as_deref(),
            Some("pro")
        );
        assert_eq!(
            pick(&s, &[], 95, Some("team"), NOW).as_deref(),
            Some("team")
        );
        // Not one to ask for: the automatic choice (the first usable account).
        let first_over = [used("principal", 100.0, end)];
        for asked in [Some(""), Some("parti"), None] {
            assert_eq!(
                pick(&s, &first_over, 95, asked, NOW).as_deref(),
                Some("pro"),
                "{asked:?}"
            );
        }
        // An account switched off cannot be asked for either.
        deactivate(&mut s, "pro");
        assert_eq!(
            pick(&s, &first_over, 95, Some("pro"), NOW).as_deref(),
            Some("team")
        );
    }

    #[test]
    fn an_account_reserved_to_projects_goes_to_those_alone() {
        let mut s = three();
        s.accounts[1].projects = vec!["client".into()];
        let client = for_project(&s, EngineId::CLAUDE, Some("client"));
        assert_eq!(candidates(&client, None), ["principal", "pro", "team"]);
        assert_eq!(preferred(&client, "pro"), Some("pro"));
        let other = for_project(&s, EngineId::CLAUDE, Some("other"));
        assert_eq!(candidates(&other, None), ["principal", "team"]);
        assert_eq!(preferred(&other, "pro"), None);
        assert_eq!(
            candidates(&for_project(&s, EngineId::CLAUDE, None), None),
            ["principal", "team"]
        );
        // Principal over the threshold: the other project goes to Team, never to Pro.
        let end = NOW + 60_000;
        let over = [used("principal", 100.0, end)];
        assert_eq!(pick(&other, &over, 95, None, NOW).as_deref(), Some("team"));
        assert_eq!(pick(&client, &over, 95, None, NOW).as_deref(), Some("pro"));
    }

    #[test]
    fn a_private_project_with_an_account_of_its_own_goes_to_it_alone_unless_it_allowed_shared_ones()
    {
        let mut s = three();
        // Pro is the client's alone, Team the client's and another's, Principal everyone's.
        s.accounts[1].projects = vec!["client".into()];
        s.accounts[2].projects = vec!["client".into(), "other".into()];
        assert!(dedicated(&s.accounts[1], "client"));
        assert!(!dedicated(&s.accounts[2], "client"));
        assert!(!dedicated(&s.accounts[0], "client"));
        let mut client = crate::core::blank_project("C:/client");
        client.id = "client".into();
        // Not private: every account that serves it, as before.
        let of =
            |s: &Settings, p: &Project| candidates(&for_project_of(s, EngineId::CLAUDE, p), None);
        assert_eq!(of(&s, &client), ["principal", "pro", "team"]);
        assert!(may_switch_to(&s, &client, "team"));
        client.private = true;
        assert_eq!(of(&s, &client), ["pro"]);
        assert!(may_switch_to(&s, &client, "pro"));
        assert!(!may_switch_to(&s, &client, "team"));
        assert!(!may_switch_to(&s, &client, "principal"));
        // The account it prefers, though shared, stays the user's choice.
        client.account = "team".into();
        assert_eq!(of(&s, &client), ["pro", "team"]);
        // « Comptes partagés à la limite »: as any project.
        client.account.clear();
        client.shared_on_limit = true;
        assert_eq!(of(&s, &client), ["principal", "pro", "team"]);
        assert!(may_switch_to(&s, &client, "team"));
        // Private with no account of its own: it shares them all already, none is taken away.
        client.shared_on_limit = false;
        s.accounts[1].projects.clear();
        assert_eq!(of(&s, &client), ["principal", "pro", "team"]);
        assert!(!may_switch_to(&s, &client, "pro"));
    }

    #[test]
    fn principal_may_be_reserved_too_and_a_project_may_be_left_without_an_account() {
        let mut s = three();
        s.accounts[0].projects = vec!["client".into()];
        s.accounts[1].projects = vec!["client".into()];
        normalize(&mut s);
        // Principal keeps the projects it is reserved to, as any account does.
        assert_eq!(s.accounts[0].projects, ["client"]);
        let client = for_project(&s, EngineId::CLAUDE, Some("client"));
        assert_eq!(candidates(&client, None), ["principal", "pro", "team"]);
        assert_eq!(
            candidates(&for_project(&s, EngineId::CLAUDE, Some("other")), None),
            ["team"]
        );
        // Team kept to the client as well: no account is left to the other project, nor to none.
        s.accounts[2].projects = vec!["client".into()];
        assert!(candidates(&for_project(&s, EngineId::CLAUDE, Some("other")), None).is_empty());
        assert!(candidates(&for_project(&s, EngineId::CLAUDE, None), None).is_empty());
        assert!(serves_any(&s, EngineId::CLAUDE, Some("client")));
        assert!(!serves_any(&s, EngineId::CLAUDE, Some("other")));
        assert!(!serves_any(&s, EngineId::CLAUDE, None));
        // A switched off account serves none either.
        s.accounts[2].projects.clear();
        deactivate(&mut s, "team");
        assert!(!serves_any(&s, EngineId::CLAUDE, Some("other")));
    }

    /// `three`, with two accounts of Codex after them: its Principal and Work.
    fn with_codex() -> Settings {
        let mut s = three();
        for (id, dir) in [("principal-codex", ""), ("work", "/work")] {
            s.accounts.push(Account {
                engine: EngineId::CODEX,
                ..account(id, dir)
            });
        }
        s
    }

    #[test]
    fn the_accounts_of_a_harness_serve_its_agents_alone() {
        let s = with_codex();
        let claude = for_project(&s, EngineId::CLAUDE, None);
        let codex = for_project(&s, EngineId::CODEX, None);
        assert_eq!(candidates(&claude, None), ["principal", "pro", "team"]);
        assert_eq!(candidates(&codex, None), ["principal-codex", "work"]);
        // A project that prefers an account of another harness: any of this one's.
        assert_eq!(preferred(&codex, "pro"), None);
        assert_eq!(preferred(&codex, "work"), Some("work"));
        // A harness with no account serves nothing.
        assert!(!serves_any(&s, EngineId::COPILOT, None));
        assert!(serves_any(&s, EngineId::CODEX, Some("p")));
    }

    #[test]
    fn the_automatic_switch_between_accounts_stays_in_the_harness() {
        let s = with_codex();
        let end = NOW + 60_000;
        // Every Claude account over the threshold, the Codex ones free: a new Claude agent still
        // goes to a Claude account (the first; the autopilot then pauses), never to Codex.
        let over = [
            used("principal", 100.0, end),
            used("pro", 100.0, end),
            used("team", 100.0, end),
        ];
        let claude = for_project(&s, EngineId::CLAUDE, None);
        assert_eq!(
            pick(&claude, &over, 95, None, NOW).as_deref(),
            Some("principal")
        );
        // On Codex, its first free one, whatever the Claude accounts read.
        let codex = for_project(&s, EngineId::CODEX, None);
        let codex_over = [used("principal-codex", 100.0, end)];
        assert_eq!(
            pick(&codex, &codex_over, 95, None, NOW).as_deref(),
            Some("work")
        );
        // Asked for by id, an account of another harness is not taken.
        assert_eq!(
            pick(&codex, &[], 95, Some("pro"), NOW).as_deref(),
            Some("principal-codex")
        );
    }

    #[test]
    fn an_account_of_any_harness_is_reserved_to_projects_as_a_claude_one() {
        let mut s = with_codex();
        let work = s.accounts.iter_mut().find(|a| a.id == "work").unwrap();
        work.projects = vec!["client".into()];
        let client = for_project(&s, EngineId::CODEX, Some("client"));
        assert_eq!(candidates(&client, None), ["principal-codex", "work"]);
        let other = for_project(&s, EngineId::CODEX, Some("other"));
        assert_eq!(candidates(&other, None), ["principal-codex"]);
        // Its Principal reserved too: the other project has no Codex account, and starts nothing on
        // Codex, while Claude still serves it.
        let main = s
            .accounts
            .iter_mut()
            .find(|a| a.id == "principal-codex")
            .unwrap();
        main.projects = vec!["client".into()];
        assert!(!serves_any(&s, EngineId::CODEX, Some("other")));
        assert!(serves_any(&s, EngineId::CLAUDE, Some("other")));
        assert!(serves_any(&s, EngineId::CODEX, Some("client")));
    }

    #[test]
    fn the_fallback_harnesses_of_a_project_follow_the_order_and_its_accounts() {
        let mut s = with_codex();
        s.engine_order = vec![EngineId::CODEX, EngineId::CLAUDE];
        // Its default harness first, then the others in the order.
        assert_eq!(
            fallback_engines(&s, EngineId::CLAUDE, "p"),
            [EngineId::CLAUDE, EngineId::CODEX]
        );
        assert_eq!(
            fallback_engines(&s, EngineId::CODEX, "p"),
            [EngineId::CODEX, EngineId::CLAUDE]
        );
        // Only those with an active account serving it.
        for a in s
            .accounts
            .iter_mut()
            .filter(|a| a.engine == EngineId::CODEX)
        {
            a.projects = vec!["client".into()];
        }
        assert_eq!(
            fallback_engines(&s, EngineId::CLAUDE, "p"),
            [EngineId::CLAUDE]
        );
        assert_eq!(
            fallback_engines(&s, EngineId::CLAUDE, "client"),
            [EngineId::CLAUDE, EngineId::CODEX]
        );
        // A harness out of the order (none of its accounts) is not one.
        assert_eq!(
            fallback_engines(&s, EngineId::COPILOT, "client"),
            [EngineId::CODEX, EngineId::CLAUDE]
        );
    }

    /// Settings as 1.8.0 saved them: Claude Code's Principal (renamed, reserved to a project), Pro,
    /// the Principal of Codex and one of its accounts, the defaults of new agents of the time.
    fn saved_by_1_8_0() -> Settings {
        Settings {
            accounts_version: 0,
            accounts: vec![
                Account {
                    name: "Perso".into(),
                    projects: vec!["p1".into()],
                    ..principal()
                },
                account("pro", "/pro"),
                principal_of(EngineId::CODEX),
                Account {
                    engine: EngineId::CODEX,
                    ..account("work", "/work")
                },
            ],
            default_model: "opus".into(),
            default_effort: "high".into(),
            default_mode: "default".into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_update_keeps_a_principal_signed_in_or_that_served_as_an_account_added() {
        // ESC-128, case by case. Kept: as it was, same id, same name, same settings.
        let mut s = saved_by_1_8_0();
        let before = s.accounts[0].clone();
        assert!(migrate(&mut s, |_| true));
        assert_eq!(ids(&s), ["principal", "pro", "principal-codex", "work"]);
        assert_eq!(s.accounts[0].name, "Perso");
        assert_eq!(s.accounts[0].projects, before.projects);
        assert_eq!(s.accounts_version, ACCOUNTS_VERSION);
        // Brought once: never again, whatever `keep` would say.
        assert!(!migrate(&mut s, |_| panic!("asked again")));
        assert_eq!(ids(&s), ["principal", "pro", "principal-codex", "work"]);
    }

    #[test]
    fn the_update_drops_a_principal_never_signed_in_nor_used() {
        // Claude Code's kept, Codex's dropped: Codex keeps the account the user made.
        let mut s = saved_by_1_8_0();
        migrate(&mut s, |a| a.engine == EngineId::CLAUDE);
        assert_eq!(ids(&s), ["principal", "pro", "work"]);
        // Both dropped: the accounts made stay, in their order.
        let mut s = saved_by_1_8_0();
        migrate(&mut s, |_| false);
        assert_eq!(ids(&s), ["pro", "work"]);
        // Its harness keeps an active account: the first left, when none of them was.
        let mut s = saved_by_1_8_0();
        s.accounts[1].active = false;
        migrate(&mut s, |_| false);
        assert!(s.accounts[0].active);
        // Who had only Claude Code's Principal, never signed in nor used: no account left.
        let mut s = Settings {
            accounts_version: 0,
            accounts: vec![principal()],
            ..Default::default()
        };
        migrate(&mut s, |_| false);
        assert!(s.accounts.is_empty());
        // Signed in: nothing else changes for them.
        let mut s = Settings {
            accounts_version: 0,
            accounts: vec![principal()],
            ..Default::default()
        };
        migrate(&mut s, |_| true);
        assert_eq!(ids(&s), ["principal"]);
        assert_eq!(s.accounts[0].name, "Principal");
    }

    #[test]
    fn the_update_gives_the_defaults_of_new_agents_to_each_claude_code_account_kept() {
        let mut s = saved_by_1_8_0();
        s.accounts[1].default_model = "haiku".into();
        migrate(&mut s, |_| true);
        let of = |id: &str| {
            let d = defaults(find(&s, id), EngineId::CLAUDE);
            (d.model, d.effort, d.mode)
        };
        let said = |m: &str, e: &str, o: &str| (m.to_string(), e.to_string(), o.to_string());
        assert_eq!(of("principal"), said("opus", "high", "default"));
        // Its own model stays; what it did not say comes from the settings.
        assert_eq!(of("pro"), said("haiku", "high", "default"));
        // Another harness's: its harness's own defaults (no model: the one Codex runs).
        assert_eq!(of("work"), said("", "medium", "auto"));
        assert_eq!(s.accounts[3].default_model, "");
        // Never written back to the settings.
        let v = serde_json::to_value(&s).unwrap();
        assert!(v.get("defaultModel").is_none());
        assert_eq!(v["accountsVersion"], 1);
    }

    #[test]
    fn a_new_agent_goes_with_its_accounts_defaults_else_its_harnesss() {
        let said = |d: Defaults| (d.model, d.effort, d.mode);
        let s = |m: &str, e: &str, o: &str| (m.to_string(), e.to_string(), o.to_string());
        assert_eq!(
            said(defaults(None, EngineId::CLAUDE)),
            s("sonnet", "medium", "auto")
        );
        assert_eq!(
            said(defaults(None, EngineId::CODEX)),
            s("", "medium", "auto")
        );
        let codex = Account {
            engine: EngineId::CODEX,
            default_model: " gpt-6-luna ".into(),
            default_mode: "default".into(),
            ..account("work", "/w")
        };
        // The account's harness, whatever harness is said.
        assert_eq!(
            said(defaults(Some(&codex), EngineId::CLAUDE)),
            s("gpt-6-luna", "medium", "default")
        );
    }

    #[test]
    fn the_order_of_the_harnesses_lists_each_one_once() {
        let mut s = Settings {
            accounts: vec![principal()],
            engine_order: vec![EngineId::CODEX, EngineId::CODEX],
            ..Default::default()
        };
        normalize(&mut s);
        // The one known kept, every harness with accounts added after it.
        assert_eq!(s.engine_order, [EngineId::CODEX, EngineId::CLAUDE]);
        // No account: no harness in the order yet (ESC-128).
        let mut fresh = Settings::default();
        normalize(&mut fresh);
        assert!(fresh.engine_order.is_empty());
    }

    #[test]
    fn the_principal_of_another_harness_missing_from_the_settings_is_still_that_harnesss() {
        let s = Settings::default();
        let codex = get(&s, "principal-codex");
        assert_eq!(
            (codex.engine, codex.id.as_str()),
            (EngineId::CODEX, "principal-codex")
        );
        assert_eq!(get(&s, "gone").engine, EngineId::CLAUDE);
    }

    #[test]
    fn no_harness_has_a_principal_until_it_is_added() {
        // ESC-128: no harness has a Principal until the user adds it: nothing of Codex in the
        // accounts, nor in the order of the harnesses (it is no harness to go on on).
        let mut s = Settings {
            accounts: vec![principal()],
            ..Default::default()
        };
        normalize(&mut s);
        assert!(s.accounts.iter().all(|a| a.engine == EngineId::CLAUDE));
        assert_eq!(s.engine_order, [EngineId::CLAUDE]);
        // Added (`Core::add_machine_account`), it stays.
        s.accounts.push(principal_of(EngineId::CODEX));
        normalize(&mut s);
        normalize(&mut s);
        assert_eq!(ids(&s), ["principal", "principal-codex"]);
        assert_eq!(s.engine_order, [EngineId::CLAUDE, EngineId::CODEX]);
    }

    #[test]
    fn each_harness_keeps_one_account_active_and_its_principal_runs_without_a_folder() {
        let mut s = with_codex();
        for a in s
            .accounts
            .iter_mut()
            .filter(|a| a.engine == EngineId::CODEX)
        {
            a.active = false;
        }
        s.accounts[3].config_dir = "/written/by/hand".into();
        normalize(&mut s);
        let codex: Vec<_> = s
            .accounts
            .iter()
            .filter(|a| a.engine == EngineId::CODEX)
            .map(|a| (a.id.as_str(), a.active, a.config_dir.as_str()))
            .collect();
        // Its first one active again; its Principal never runs with a folder of Escouade's.
        assert_eq!(
            codex,
            [("principal-codex", true, ""), ("work", false, "/work")]
        );
        assert_eq!(own_dir(&s.accounts[3]), None);
        assert_eq!(own_dir(&s.accounts[4]), Some("/work"));
        // Claude Code's accounts as they were.
        assert!(s.accounts[..3].iter().all(|a| a.active));
        assert!(is_principal(&s.accounts[0]) && is_principal(&s.accounts[3]));
        assert!(!is_principal(&s.accounts[1]));
        assert_eq!(principal_id(EngineId::CODEX), "principal-codex");
        // Their place in the order of the harnesses.
        assert_eq!(s.engine_order, [EngineId::CLAUDE, EngineId::CODEX]);
    }

    #[test]
    fn a_project_may_start_its_tickets_on_the_account_it_prefers_else_on_any_active_one() {
        let mut s = three();
        let all = ["principal", "pro", "team"];
        assert_eq!(candidates(&s, None), all);
        assert_eq!(candidates(&s, Some("")), all);
        assert_eq!(candidates(&s, Some("pro")), ["pro"]);
        // One that is gone, or switched off: any active one.
        assert_eq!(candidates(&s, Some("parti")), all);
        deactivate(&mut s, "pro");
        assert_eq!(candidates(&s, Some("pro")), ["principal", "team"]);
        assert_eq!(candidates(&s, None), ["principal", "team"]);
    }

    #[test]
    fn the_switch_of_the_current_account_is_told_only_when_the_one_left_passed_the_threshold() {
        use crate::i18n::Lang::{En, Fr};
        let mut s = three();
        let end = NOW + 60_000;
        let told = |lang, s: &Settings, usage: &[AccountUsage], from: &str, to: &str| {
            switch_notice(lang, s, usage, 95, NOW, from, to)
        };
        let over = [used("principal", 96.2, end), used("pro", 10.0, end)];
        assert_eq!(
            told(Fr, &s, &over, "principal", "pro"),
            Some(
                "Le compte Principal a atteint 96 % : les nouveaux agents partent sur Pro.".into()
            )
        );
        assert_eq!(
            told(En, &s, &over, "principal", "pro"),
            Some("The Main account has reached 96%: new agents now go to Pro.".into())
        );
        // The use told is that of the window that holds the account back (the most used of those).
        let week = |pct| AccountUsage {
            meters: crate::model::claude_meters(
                Some(crate::model::RateWindow {
                    pct: 96.0,
                    resets_at: Some(end),
                }),
                Some(crate::model::RateWindow {
                    pct,
                    resets_at: Some(end),
                }),
            ),
            ..used("principal", 96.0, end)
        };
        assert_eq!(
            told(Fr, &s, &[week(100.0)], "principal", "team"),
            Some(
                "Le compte Principal a atteint 100 % : les nouveaux agents partent sur Team."
                    .into()
            )
        );
        // A week at 98.6 % that starts again in a minute holds nothing back: it is the 5 h.
        assert_eq!(
            told(Fr, &s, &[week(98.6)], "principal", "team"),
            Some(
                "Le compte Principal a atteint 96 % : les nouveaux agents partent sur Team.".into()
            )
        );
        // No switch, no word; nor when there was no account before.
        assert_eq!(told(Fr, &s, &over, "principal", "principal"), None);
        assert_eq!(told(Fr, &s, &over, "", "pro"), None);
        // Back to an account that is the first usable again: it is not told.
        assert_eq!(told(Fr, &s, &over, "pro", "principal"), None);
        // The account left was not over the threshold (switched off, taken out): not told either.
        let under = [used("principal", 94.0, end)];
        assert_eq!(told(Fr, &s, &under, "principal", "pro"), None);
        assert_eq!(told(Fr, &s, &over, "parti", "pro"), None);
        deactivate(&mut s, "principal");
        assert_eq!(told(Fr, &s, &over, "principal", "pro"), None);
        // An account is called by its name; Principal by default, in the language told.
        let mut named = three();
        named.accounts[0].name = "Perso".into();
        assert_eq!(
            told(Fr, &named, &over, "principal", "pro"),
            Some("Le compte Perso a atteint 96 % : les nouveaux agents partent sur Pro.".into())
        );
    }

    #[test]
    fn principal_is_called_by_its_default_name_in_the_language_asked_until_it_is_renamed() {
        use crate::i18n::Lang::{En, Fr};
        let mut main = principal();
        assert_eq!(
            (name_in(Fr, &main), name_in(En, &main)),
            ("Principal".into(), "Main".into())
        );
        // Saved in the other language: still its default name.
        main.name = "Main".into();
        assert_eq!(name_in(Fr, &main), "Principal");
        main.name = " ".into();
        assert_eq!(name_in(En, &main), "Main");
        main.name = "Perso".into();
        assert_eq!(name_in(Fr, &main), "Perso");
        // Another account's name is its own, whatever it says.
        assert_eq!(name_in(En, &account("pro", "/pro")), "PRO");
        let mut other = account("pro", "/pro");
        other.name = "Main".into();
        assert_eq!(name_in(Fr, &other), "Main");
    }

    #[test]
    fn a_choice_is_made_among_the_accounts_that_are_usable_else_the_first_active_one() {
        let s = three();
        let only = |ok: &'static str| {
            move |id: &str| match id == ok {
                true => Standing::Good,
                false => Standing::Out,
            }
        };
        assert_eq!(pick_ranked(&s, None, only("team")).as_deref(), Some("team"));
        assert_eq!(
            pick_ranked(&s, None, only("principal")).as_deref(),
            Some("principal")
        );
        assert_eq!(
            pick_ranked(&s, None, |_| Standing::Out).as_deref(),
            Some("principal")
        );
        // Asked for, it is taken even if it is not usable.
        assert_eq!(
            pick_ranked(&s, Some("pro"), only("team")).as_deref(),
            Some("pro")
        );
        // The best rank first, the order of the settings among equals: a better account later in
        // the list goes before a worse one earlier, never before an equal one.
        let rank = |id: &str| match id {
            "principal" => Standing::Out,
            "pro" => Standing::Behind,
            _ => Standing::Good,
        };
        assert_eq!(pick_ranked(&s, None, rank).as_deref(), Some("team"));
        let rank = |id: &str| match id {
            "principal" => Standing::Out,
            _ => Standing::Behind,
        };
        assert_eq!(pick_ranked(&s, None, rank).as_deref(), Some("pro"));
    }

    /// A meter of `minutes` (none: the month's credits) used `pct` percent, starting again in `min`
    /// minutes from `NOW`.
    fn win(minutes: Option<u32>, pct: f64, min: i64) -> QuotaMeter {
        QuotaMeter {
            minutes,
            pct,
            resets_at: Some(NOW + min * 60_000),
        }
    }

    const FIVE: Option<u32> = Some(crate::model::FIVE_HOURS);
    const WEEK: Option<u32> = Some(crate::model::WEEK);

    fn read(id: &str, meters: Vec<QuotaMeter>) -> AccountUsage {
        AccountUsage {
            id: id.into(),
            meters,
            ..Default::default()
        }
    }

    /// A week 2 days gone (5 days left): `pct` percent used.
    fn day_two(pct: f64) -> QuotaMeter {
        win(WEEK, pct, 5 * 24 * 60)
    }

    #[test]
    fn a_week_on_pace_that_starts_again_soon_is_not_wasted_even_past_the_threshold() {
        let s = three();
        // A (first in the list): 88 % of its week, starting again in 3 h; B at 10 % of a week at its
        // second day (35 % at the reset).
        let usage = [
            read(
                "principal",
                vec![win(FIVE, 30.0, 120), win(WEEK, 88.0, 180)],
            ),
            read("pro", vec![day_two(10.0)]),
        ];
        for threshold in [80, 90, 95, 100] {
            assert_eq!(
                pick(&s, &usage, threshold, None, NOW).as_deref(),
                Some("principal"),
                "at {threshold}"
            );
        }
        // Past the threshold in percent all the same: what the old rule sent away.
        assert!(over_threshold(&usage, "principal", 80, NOW));
        assert_eq!(standing(&usage, "principal", 80, NOW), Standing::Good);
    }

    #[test]
    fn a_week_that_would_not_last_to_its_reset_goes_behind_the_others_even_under_the_threshold() {
        let s = three();
        // A at 60 % at its second day: 210 % at the reset. B at 20 %: 70 %.
        let usage = [
            read("principal", vec![day_two(60.0)]),
            read("pro", vec![day_two(20.0)]),
        ];
        assert_eq!(standing(&usage, "principal", 95, NOW), Standing::Behind);
        for threshold in [80, 95, 100] {
            assert_eq!(
                pick(&s, &usage, threshold, None, NOW).as_deref(),
                Some("pro"),
                "at {threshold}"
            );
        }
        // Behind is still behind the account that is good, but before the order of the list: the
        // first of the good ones, whichever it is.
        let third = [
            read("principal", vec![day_two(60.0)]),
            read("pro", vec![day_two(60.0)]),
            read("team", vec![day_two(20.0)]),
        ];
        assert_eq!(pick(&s, &third, 95, None, NOW).as_deref(), Some("team"));
    }

    #[test]
    fn a_5_hour_window_at_the_threshold_is_a_short_pause_that_leaves_no_trace() {
        let s = three();
        // A at 96 % of its 5 h (threshold 95) with a week at 20 % (second day: 70 %): B.
        let before = [
            read("principal", vec![win(FIVE, 96.0, 120), day_two(20.0)]),
            read("pro", vec![]),
        ];
        assert_eq!(pick(&s, &before, 95, None, NOW).as_deref(), Some("pro"));
        // Its 5 h started again (its end and the margin after it are past): A again, with the week
        // as it was.
        let after = [
            read("principal", vec![win(FIVE, 96.0, -30), day_two(20.0)]),
            read("pro", vec![]),
        ];
        assert_eq!(
            pick(&s, &after, 95, None, NOW).as_deref(),
            Some("principal")
        );
        // The 5 h of a fresh window, used 3 % : nothing.
        let fresh = [read("principal", vec![win(FIVE, 3.0, 290), day_two(20.0)])];
        assert_eq!(
            pick(&s, &fresh, 95, None, NOW).as_deref(),
            Some("principal")
        );
        // A 5 h that is full weighs on the choice for its own length only, not on the week.
        let full = [
            read("principal", vec![win(FIVE, 100.0, 60), day_two(20.0)]),
            read("pro", vec![day_two(20.0)]),
        ];
        assert_eq!(pick(&s, &full, 100, None, NOW).as_deref(), Some("pro"));
    }

    #[test]
    fn a_week_at_its_limit_is_out_until_its_reset() {
        let s = three();
        let usage = [
            read("principal", vec![win(WEEK, 100.0, 24 * 60)]),
            read("pro", vec![]),
        ];
        assert_eq!(standing(&usage, "principal", 100, NOW), Standing::Out);
        assert_eq!(pick(&s, &usage, 100, None, NOW).as_deref(), Some("pro"));
        // Reset (its end and the margin after it are past): a candidate again.
        let reset = [read("principal", vec![win(WEEK, 100.0, -1)])];
        assert_eq!(
            pick(&s, &reset, 100, None, NOW).as_deref(),
            Some("principal")
        );
    }

    #[test]
    fn with_no_projection_the_percentage_decides_as_before() {
        let s = three();
        // The first day of a week (12 h gone): too early to project.
        let early = [
            read("principal", vec![win(WEEK, 88.0, 7 * 24 * 60 - 12 * 60)]),
            read("pro", vec![]),
        ];
        assert_eq!(pick(&s, &early, 80, None, NOW).as_deref(), Some("pro"));
        assert_eq!(
            pick(&s, &early, 95, None, NOW).as_deref(),
            Some("principal")
        );
        // A week whose end is not known holds nothing back.
        let unknown = [read(
            "principal",
            vec![QuotaMeter {
                minutes: WEEK,
                pct: 99.0,
                resets_at: None,
            }],
        )];
        assert_eq!(
            pick(&s, &unknown, 80, None, NOW).as_deref(),
            Some("principal")
        );
    }

    #[test]
    fn with_no_good_account_it_is_the_first_without_a_window_at_its_limit_else_the_first_active() {
        let s = three();
        // A is out (5 h at 100 %), B only behind (210 %), C out: B.
        let usage = [
            read("principal", vec![win(FIVE, 100.0, 60)]),
            read("pro", vec![day_two(60.0)]),
            read("team", vec![win(FIVE, 99.0, 60)]),
        ];
        assert_eq!(pick(&s, &usage, 95, None, NOW).as_deref(), Some("pro"));
        // Both behind: the first in the order.
        let both = [
            read("principal", vec![day_two(60.0)]),
            read("pro", vec![day_two(60.0)]),
            read("team", vec![win(FIVE, 99.0, 60)]),
        ];
        assert_eq!(pick(&s, &both, 95, None, NOW).as_deref(), Some("principal"));
        // Every one out: the first active (the autopilot then pauses).
        let all = [
            read("principal", vec![win(FIVE, 100.0, 60)]),
            read("pro", vec![win(WEEK, 100.0, 60)]),
            read("team", vec![win(FIVE, 99.0, 60)]),
        ];
        assert_eq!(pick(&s, &all, 95, None, NOW).as_deref(), Some("principal"));
        let mut s = s;
        deactivate(&mut s, "principal");
        assert_eq!(pick(&s, &all, 95, None, NOW).as_deref(), Some("pro"));
    }

    #[test]
    fn the_order_stays_the_priority_asked_then_preferred_then_the_first_good_one() {
        let s = three();
        let usage = [
            read("principal", vec![win(FIVE, 100.0, 60)]),
            read("pro", vec![day_two(60.0)]),
            read("team", vec![day_two(20.0)]),
        ];
        // The first good one, not the best by percentage.
        assert_eq!(pick(&s, &usage, 95, None, NOW).as_deref(), Some("team"));
        // The account preferred (or asked for) comes first, whatever its quota.
        assert_eq!(
            pick(&s, &usage, 95, Some("principal"), NOW).as_deref(),
            Some("principal")
        );
        assert_eq!(
            pick(&s, &usage, 95, Some("pro"), NOW).as_deref(),
            Some("pro")
        );
        // Two good ones: the order of the settings, even when the later one has used less.
        let two = [
            read("principal", vec![day_two(25.0)]),
            read("pro", vec![day_two(5.0)]),
        ];
        assert_eq!(pick(&s, &two, 95, None, NOW).as_deref(), Some("principal"));
    }

    #[test]
    fn codex_has_a_short_5_hour_window_and_a_week_and_thirty_days_judged_by_their_pace() {
        let s = with_codex();
        let codex = for_project(&s, EngineId::CODEX, None);
        let thirty = Some(43_200);
        let day = 24 * 60;
        let usage = [
            // 5 h at 96 %: out for its own length.
            read(
                "principal-codex",
                vec![
                    win(FIVE, 96.0, 100),
                    win(WEEK, 20.0, 5 * day),
                    win(thirty, 10.0, 20 * day),
                ],
            ),
            // 5 h free, the week on pace at 92 %, the 30 days behind (40 % after 5 days: 240 %).
            read(
                "work",
                vec![
                    win(FIVE, 10.0, 100),
                    win(WEEK, 92.0, 600),
                    win(thirty, 40.0, 25 * day),
                ],
            ),
        ];
        let pick_at = |usage: &[AccountUsage]| pick(&codex, usage, 90, None, NOW);
        assert_eq!(standing(&usage, "principal-codex", 90, NOW), Standing::Out);
        assert_eq!(standing(&usage, "work", 90, NOW), Standing::Behind);
        // Out against behind: the one that is only behind.
        assert_eq!(pick_at(&usage).as_deref(), Some("work"));
        // Its 5 h started again: the first account, whose week and 30 days are on pace.
        let after = [
            read(
                "principal-codex",
                vec![
                    win(FIVE, 96.0, -31),
                    win(WEEK, 20.0, 5 * day),
                    win(thirty, 10.0, 20 * day),
                ],
            ),
            usage[1].clone(),
        ];
        assert_eq!(standing(&after, "principal-codex", 90, NOW), Standing::Good);
        assert_eq!(pick_at(&after).as_deref(), Some("principal-codex"));
        // Never an account of another harness.
        assert_eq!(
            pick(
                &for_project(&s, EngineId::CLAUDE, None),
                &usage,
                90,
                None,
                NOW
            )
            .as_deref(),
            Some("principal")
        );
    }

    #[test]
    fn copilot_credits_are_a_window_of_the_month_before_their_reset() {
        let mut s = Settings::default();
        for id in ["first", "second"] {
            s.accounts.push(Account {
                engine: EngineId::COPILOT,
                ..account(id, "")
            });
        }
        let reset = |ms: i64| QuotaMeter {
            minutes: None,
            pct: 0.0,
            resets_at: Some(ms),
        };
        // November 1st, 00:00 UTC: October has 31 days.
        let nov = chrono::DateTime::parse_from_rfc3339("2026-11-01T00:00:00Z")
            .unwrap()
            .timestamp_millis();
        let at = |iso: &str| {
            chrono::DateTime::parse_from_rfc3339(iso)
                .unwrap()
                .timestamp_millis()
        };
        let credits = |pct: f64| QuotaMeter { pct, ..reset(nov) };
        let usage = [
            read("first", vec![credits(60.0)]),
            read("second", vec![credits(10.0)]),
        ];
        // 5 days of October gone: 60 % is 372 % of the month, 10 % is 62 %.
        let day5 = at("2026-10-06T00:00:00Z");
        assert_eq!(standing(&usage, "first", 80, day5), Standing::Behind);
        assert_eq!(pick(&s, &usage, 80, None, day5).as_deref(), Some("second"));
        // Two days before the reset, 90 % is on pace (the month before is October, 31 days long),
        // past the threshold or not.
        let late = at("2026-10-30T12:00:00Z");
        let usage = [read("first", vec![credits(90.0)]), read("second", vec![])];
        assert_eq!(pick(&s, &usage, 80, None, late).as_deref(), Some("first"));
        // At 100 %: out until the reset.
        let usage = [read("first", vec![credits(100.0)]), read("second", vec![])];
        assert_eq!(pick(&s, &usage, 80, None, late).as_deref(), Some("second"));
        // The length of the month before: a reset on March 1st (28 days of February, 2026).
        let mar = at("2026-03-01T00:00:00Z");
        let usage = [
            read(
                "first",
                vec![QuotaMeter {
                    pct: 50.0,
                    ..reset(mar)
                }],
            ),
            read("second", vec![]),
        ];
        // 14 days of the 28 of February gone with 50 % used: 100 % at the reset, on pace; 11 days: 127 %.
        assert_eq!(
            standing(&usage, "first", 80, at("2026-02-15T00:00:00Z")),
            Standing::Good
        );
        assert_eq!(
            standing(&usage, "first", 80, at("2026-02-12T00:00:00Z")),
            Standing::Behind
        );
    }

    #[test]
    fn a_harness_without_a_quota_keeps_the_order_alone() {
        let mut s = Settings::default();
        for id in ["one", "two"] {
            s.accounts.push(Account {
                engine: EngineId::OPENCODE,
                ..account(id, "")
            });
        }
        // No meter, read or not: good, the first.
        assert_eq!(pick(&s, &[], 80, None, NOW).as_deref(), Some("one"));
        let usage = [read("one", vec![]), read("two", vec![])];
        assert_eq!(standing(&usage, "one", 80, NOW), Standing::Good);
        assert_eq!(pick(&s, &usage, 80, None, NOW).as_deref(), Some("one"));
        assert_eq!(
            pick(&s, &usage, 80, Some("two"), NOW).as_deref(),
            Some("two")
        );
    }

    /// A session as Claude Code files it in an account's folder: its transcript in the project's
    /// folder, with the folder of what it keeps beside it.
    fn keep_session(account: &Path, project: &str, id: &str, text: &str) -> PathBuf {
        let folder = account.join("projects").join(project);
        std::fs::create_dir_all(folder.join(id).join("subagents")).unwrap();
        std::fs::write(
            folder.join(id).join("subagents").join("agent-1.jsonl"),
            text,
        )
        .unwrap();
        let file = folder.join(format!("{id}.jsonl"));
        std::fs::write(&file, text).unwrap();
        file
    }

    fn modified(file: &Path, secs_ago: u64) {
        let at = std::time::SystemTime::now() - std::time::Duration::from_secs(secs_ago);
        let f = std::fs::OpenOptions::new().write(true).open(file).unwrap();
        f.set_modified(at).unwrap();
    }

    #[test]
    fn a_session_is_copied_with_its_folder_into_the_same_project_folder_of_the_other_account() {
        let dir = test_dir("accounts-copy-session");
        let (from, to) = (dir.join("principal"), dir.join("pro"));
        let kept = keep_session(&from, "C--code-demo", "s-1", "bonjour\n");
        // Another session of the account, and another project's: not copied.
        keep_session(&from, "C--code-demo", "s-2", "autre\n");
        keep_session(&from, "C--code-autre", "s-3", "autre\n");
        copy_session(&from, &to, "s-1").unwrap();
        let folder = to.join("projects").join("C--code-demo");
        assert_eq!(
            std::fs::read_to_string(folder.join("s-1.jsonl")).unwrap(),
            "bonjour\n"
        );
        assert_eq!(
            std::fs::read_to_string(folder.join("s-1").join("subagents").join("agent-1.jsonl"))
                .unwrap(),
            "bonjour\n"
        );
        assert_eq!(names_in(&to.join("projects")), ["C--code-demo"]);
        assert_eq!(names_in(&folder), ["s-1", "s-1.jsonl"]);
        // Copied, not moved; no half-written file left beside it.
        assert!(kept.is_file());
        assert!(!names_in(&folder).iter().any(|n| n.ends_with(".part")));
        // The copy is as old as the transcript: the more recent of the two is told by it.
        assert_eq!(
            std::fs::metadata(folder.join("s-1.jsonl"))
                .unwrap()
                .modified()
                .unwrap(),
            std::fs::metadata(&kept).unwrap().modified().unwrap()
        );
    }

    #[test]
    fn a_session_the_account_does_not_have_is_refused_with_where_it_looked() {
        let dir = test_dir("accounts-copy-session-missing");
        let (from, to) = (dir.join("principal"), dir.join("pro"));
        keep_session(&from, "C--code-demo", "s-1", "bonjour\n");
        let e = copy_session(&from, &to, "s-9").unwrap_err().to_string();
        assert_eq!(
            e,
            format!(
                "Session « s-9 » introuvable dans le dossier du compte : {}",
                from.display()
            )
        );
        // An account that has no `projects` folder yet says the same, and nothing is made.
        let e = copy_session(&dir.join("vide"), &to, "s-1").unwrap_err();
        assert!(
            e.to_string().starts_with("Session « s-1 » introuvable"),
            "{e}"
        );
        assert!(!to.exists());
    }

    #[test]
    fn a_session_id_that_is_a_path_is_refused_before_anything_is_read() {
        let dir = test_dir("accounts-copy-session-id");
        let (from, to) = (dir.join("principal"), dir.join("pro"));
        keep_session(&from, "C--code-demo", "s-1", "bonjour\n");
        std::fs::write(from.join("secret.jsonl"), "secret").unwrap();
        for bad in ["", "..", "../secret", "a/b", "a\\b", "s-1.jsonl", "s 1"] {
            let e = copy_session(&from, &to, bad).unwrap_err().to_string();
            assert_eq!(
                e,
                format!("Identifiant de session invalide : {bad}"),
                "{bad}"
            );
        }
        assert!(!to.exists());
    }

    #[test]
    fn the_more_recent_copy_of_a_session_wins_file_by_file() {
        let dir = test_dir("accounts-copy-session-newest");
        let (from, to) = (dir.join("principal"), dir.join("pro"));
        let old = keep_session(&from, "C--code-demo", "s-1", "ancien\n");
        let annexed = |account: &Path| {
            let folder = account.join("projects").join("C--code-demo");
            folder.join("s-1").join("subagents").join("agent-1.jsonl")
        };
        modified(&old, 600);
        modified(&annexed(&from), 600);
        // The other account has this session too, and it went on there: it is the newer.
        let newer = keep_session(&to, "C--code-demo", "s-1", "plus récent\n");
        modified(&newer, 60);
        modified(&annexed(&to), 60);
        copy_session(&from, &to, "s-1").unwrap();
        assert_eq!(std::fs::read_to_string(&newer).unwrap(), "plus récent\n");
        // The folder beside it is of the same rule: a file only the older one has comes over.
        let annex = to.join("projects").join("C--code-demo").join("s-1");
        std::fs::write(
            from.join("projects")
                .join("C--code-demo")
                .join("s-1")
                .join("tool-results.txt"),
            "résultat",
        )
        .unwrap();
        copy_session(&from, &to, "s-1").unwrap();
        assert_eq!(
            std::fs::read_to_string(annex.join("tool-results.txt")).unwrap(),
            "résultat"
        );
        assert_eq!(
            std::fs::read_to_string(annex.join("subagents").join("agent-1.jsonl")).unwrap(),
            "plus récent\n"
        );
        // The other way round: the copy here is the older one, it is replaced.
        modified(&newer, 3600);
        copy_session(&from, &to, "s-1").unwrap();
        assert_eq!(std::fs::read_to_string(&newer).unwrap(), "ancien\n");
    }

    #[test]
    fn a_session_filed_in_several_project_folders_is_copied_from_the_most_recent_one() {
        let dir = test_dir("accounts-copy-session-twice");
        let (from, to) = (dir.join("principal"), dir.join("pro"));
        let old = keep_session(&from, "C--code-demo", "s-1", "ancien\n");
        modified(&old, 600);
        keep_session(&from, "C--code-demo-wt", "s-1", "récent\n");
        copy_session(&from, &to, "s-1").unwrap();
        assert_eq!(names_in(&to.join("projects")), ["C--code-demo-wt"]);
    }
}
