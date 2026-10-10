//! The Claude accounts: each one is Claude Code run with a configuration folder of its own
//! (`CLAUDE_CONFIG_DIR`), so with its own sign-in, sessions and quota. Principal is the user's own
//! Claude Code, launched as it always was: never given a `CLAUDE_CONFIG_DIR`, since setting one,
//! even to `~/.claude`, changes the name of its macOS keychain entry and signs the user out.
//!
//! The order of `Settings::accounts` is their priority: new agents go to the first one usable.

use crate::board;
use crate::claude;
use crate::core::Core;
use crate::i18n::Lang;
use crate::model::{Account, AccountUsage, Settings};
use crate::pty::TermInfo;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Runtime;

/// The id of the user's own account.
pub const PRINCIPAL: &str = "principal";

/// The variable that gives Claude Code its configuration folder.
const CONFIG_DIR_VAR: &str = "CLAUDE_CONFIG_DIR";

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

/// Principal as it is made the first time: the user's own Claude Code, active.
pub fn principal() -> Account {
    Account {
        id: PRINCIPAL.into(),
        name: tr!("Principal", "Main"),
        config_dir: String::new(),
        claude_path: String::new(),
        active: true,
    }
}

/// Puts the accounts right: Principal among them (first when it was missing, as on the first
/// load), with no folder of its own; every id unique; at least one account active.
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
    // None (written by hand): one after its name, never Principal's.
    taken.insert(PRINCIPAL.to_string());
    for a in accounts.iter_mut().filter(|a| a.id.is_empty()) {
        let base = match crate::core::slugify(&a.name) {
            s if s.is_empty() => "account".to_string(),
            s => s,
        };
        a.id = unique(&base, &taken);
        taken.insert(a.id.clone());
    }
    let at = match accounts.iter().position(|a| a.id == PRINCIPAL) {
        Some(at) => at,
        None => {
            accounts.insert(0, principal());
            0
        }
    };
    let main = &mut accounts[at];
    // Whatever a hand-edited file says: a folder would sign the user out of it.
    main.config_dir.clear();
    if main.name.trim().is_empty() {
        main.name = principal().name;
    }
    if !accounts.iter().any(|a| a.active) {
        accounts[at].active = true;
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

/// The account `id` names, else Principal (an id unknown to the settings: an account removed, a
/// state saved by a later version), with a line in the log. None only when the settings have no
/// Principal, which `normalize` prevents.
pub fn find<'a>(settings: &'a Settings, id: &str) -> Option<&'a Account> {
    if let Some(a) = settings.accounts.iter().find(|a| a.id == id) {
        return Some(a);
    }
    // Empty: an agent made by code that names none.
    if !id.is_empty() && id != PRINCIPAL {
        log::warn!("Claude account {id:?} unknown: Principal instead");
    }
    settings.accounts.iter().find(|a| a.id == PRINCIPAL)
}

/// `find`, as a copy that is always there: settings never normalized (written whole by a test)
/// give Principal as it is made.
pub fn get(settings: &Settings, id: &str) -> Account {
    find(settings, id).cloned().unwrap_or_else(principal)
}

/// Whether one of the account's quota windows (`usage`) is used `threshold` percent or more at
/// `now`, until an end still to come (as the autopilot counts it, `board::over_threshold`).
pub fn over_threshold(usage: &[AccountUsage], id: &str, threshold: u32, now: i64) -> bool {
    usage.iter().find(|u| u.id == id).is_some_and(|u| {
        [u.five_hour, u.seven_day]
            .iter()
            .flatten()
            .any(|w| board::over_threshold(w, threshold, now))
    })
}

/// The account a new agent goes to. The one `preferred` names (a project's, or the one asked for)
/// when it is active; else the first active one, in the settings' order, none of whose quota
/// windows (`usage`) is used `threshold` percent or more at `now`; when every active account is,
/// the first active one (the autopilot then pauses). Principal when none is active, which
/// `normalize` prevents. With no `preferred` it is the account the status bar calls the current one.
pub fn pick(
    settings: &Settings,
    usage: &[AccountUsage],
    threshold: u32,
    preferred: Option<&str>,
    now: i64,
) -> String {
    pick_where(settings, preferred, |id| {
        !over_threshold(usage, id, threshold, now)
    })
}

/// `pick`, with the accounts that may take an agent now told by `usable`: for a choice that goes
/// by more than the quota windows (a pause after a usage limit, an agent waiting for its reset).
pub fn pick_where(
    settings: &Settings,
    preferred: Option<&str>,
    usable: impl Fn(&str) -> bool,
) -> String {
    let mut active = settings.accounts.iter().filter(|a| a.active);
    if let Some(a) = preferred.and_then(|id| active.clone().find(|a| a.id == id)) {
        return a.id.clone();
    }
    let first = active.clone().next();
    active
        .find(|a| usable(&a.id))
        .or(first)
        .map_or_else(|| PRINCIPAL.to_string(), |a| a.id.clone())
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

/// What the account is called in `lang`: its name; Principal still named by default, its default
/// name in `lang` (the one it was first saved in is not always the one the window is in).
pub fn name_in(lang: Lang, account: &Account) -> String {
    let name = account.name.trim();
    if account.id == PRINCIPAL && (name.is_empty() || default_names().iter().any(|d| d == name)) {
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
/// `from` to `to`: only when `from` is an active account that passed the threshold (it is why they
/// go elsewhere now) and `to` is one that did not (with every account past it the first one is
/// the current again, and the autopilot's pause says so); going back to the first usable one is
/// not told. None otherwise.
pub fn switch_notice(
    lang: Lang,
    settings: &Settings,
    usage: &[AccountUsage],
    threshold: u32,
    now: i64,
    from: &str,
    to: &str,
) -> Option<String> {
    if from == to || over_threshold(usage, to, threshold, now) {
        return None;
    }
    let left = settings
        .accounts
        .iter()
        .find(|a| a.active && a.id == from)?;
    let used = usage.iter().find(|u| u.id == from)?;
    // The window that went the furthest of those that hold the account back.
    let pct = [used.five_hour, used.seven_day]
        .iter()
        .flatten()
        .filter(|w| board::over_threshold(w, threshold, now))
        .map(|w| w.pct)
        .reduce(f64::max)?;
    let (from, to) = (name_in(lang, left), name_in(lang, &get(settings, to)));
    let pct = pct.round();
    Some(tr_in!(
        lang,
        "Le compte {from} a atteint {pct} % : les nouveaux agents partent sur {to}.",
        "The {from} account has reached {pct}%: new agents now go to {to}."
    ))
}

/// The app's `CLAUDE_CONFIG_DIR` (which Principal's processes inherit) and the home folder. In
/// unit tests, neither the variable nor the machine user's home: a folder that does not exist, so
/// that no test ever reads or writes the user's own Claude Code (tests may run inside a Claude
/// Code session, with its CLAUDE_CONFIG_DIR).
pub(crate) fn app_env() -> (Option<String>, PathBuf) {
    if cfg!(test) {
        return (None, std::env::temp_dir().join("escouade-tests-no-home"));
    }
    (
        std::env::var(CONFIG_DIR_VAR).ok(),
        dirs::home_dir().unwrap_or_default(),
    )
}

/// The configuration folder the account's Claude Code reads: its own, or for Principal (no folder)
/// the app's `CLAUDE_CONFIG_DIR` when it has one, which its processes inherit, else `~/.claude`.
pub fn config_dir(account: &Account) -> PathBuf {
    let (env, home) = app_env();
    config_dir_with(account, env.as_deref(), &home)
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

/// The account's own folder, the `CLAUDE_CONFIG_DIR` its processes get: none for Principal,
/// whatever its settings say.
pub(crate) fn own_dir(account: &Account) -> Option<&str> {
    let dir = account.config_dir.trim();
    (account.id != PRINCIPAL && !dir.is_empty()).then_some(dir)
}

/// What the account's Claude Code is launched with besides the network settings
/// (`Settings::claude_env`): its `CLAUDE_CONFIG_DIR`, exactly the folder saved, when it has one.
/// Never for Principal.
pub fn launch_env(account: &Account) -> Vec<(String, String)> {
    own_dir(account)
        .map(|dir| (CONFIG_DIR_VAR.to_string(), dir.to_string()))
        .into_iter()
        .collect()
}

/// The `claude` the account runs: its own when it names one, else the settings' (found as
/// `claude::resolve_binary` finds it). None when it cannot be found.
pub fn program(account: &Account, settings: &Settings) -> Option<PathBuf> {
    let own = account.claude_path.trim();
    claude::resolve_binary(if own.is_empty() {
        &settings.claude_path
    } else {
        own
    })
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

/// `claude_json_with` in the app's environment (`app_env`).
pub fn claude_json(account: &Account) -> PathBuf {
    let (env, home) = app_env();
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

/// The items of `SHARED_FILES` and `SHARED_DIRS` that Principal's folder `dir` has, files first.
pub fn shareable(dir: &Path) -> Vec<String> {
    let files = SHARED_FILES.iter().filter(|n| dir.join(n).is_file());
    let dirs = SHARED_DIRS.iter().filter(|n| dir.join(n).is_dir());
    files.chain(dirs).map(|n| n.to_string()).collect()
}

/// What keeps the accounts from being saved: an account other than Principal needs a folder of
/// its own written in full (relative, its processes would each read another one, under the
/// folder they run in; empty, it would run as Principal).
pub fn check(settings: &Settings) -> Result<()> {
    for a in settings.accounts.iter().filter(|a| a.id != PRINCIPAL) {
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
    if a.id == PRINCIPAL && defaults.contains(&name) {
        defaults.to_vec()
    } else {
        vec![name]
    }
}

/// A new account named `name`: its folder made under `root` (`<data folder>/claude`), named after
/// it and taken by no account (`settings`) nor folder there, with the items `share` of Principal's
/// folder (`principal_dir`) linked or copied into it (`mode`). Its sign-in and `.claude.json`
/// are its own: Claude Code makes them when it signs in.
pub fn create(
    settings: &Settings,
    root: &Path,
    principal_dir: &Path,
    name: &str,
    share: &[String],
    mode: ShareMode,
) -> Result<Account> {
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
        .find(|n| !SHARED_FILES.contains(&n.as_str()) && !SHARED_DIRS.contains(&n.as_str()))
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
    taken.insert(PRINCIPAL.to_string());
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
    if let Err(e) = share_items(principal_dir, &dir, share, mode) {
        // Its links go with it, never what they lead to.
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    Ok(Account {
        id,
        name: name.to_string(),
        config_dir: dir.to_string_lossy().into_owned(),
        claude_path: String::new(),
        active: true,
    })
}

/// The items `names` of `from` (Principal's folder) linked or copied into `to`: those it has.
fn share_items(from: &Path, to: &Path, names: &[String], mode: ShareMode) -> Result<()> {
    for name in names {
        let (src, dst) = (from.join(name), to.join(name));
        let done = if SHARED_DIRS.contains(&name.as_str()) {
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

/// `dst` made a junction to the folder `src`: unlike a symbolic link, it needs no administrator
/// rights (nor the developer mode).
#[cfg(windows)]
fn link_dir(src: &Path, dst: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt;
    let cmd = std::env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into());
    // The line as cmd.exe reads it, not as Rust would quote it (only what has a space).
    let line = junction_line(dst, &std::path::absolute(src)?)?;
    let out = std::process::Command::new(cmd)
        .args(["/d", "/c"])
        .raw_arg(line)
        .creation_flags(claude::CREATE_NO_WINDOW)
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
            Err(e) => return Err(e.into()),
        }
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

/// Refused: no account has the id the window gave (removed meanwhile).
fn unknown(id: &str) -> anyhow::Error {
    anyhow::anyhow!(tr!(
        "compte Claude « {id} » introuvable",
        "Claude account “{id}” not found"
    ))
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
    /// The settings the window saves, but for the accounts and for « Claude peut piloter
    /// Escouade »: those are the backend's, which their tabs change on their own (the window's copy
    /// may be older).
    pub fn save_window_settings(self: &Arc<Self>, mut s: Settings) -> Result<()> {
        let _one = self.claude_accounts_lock.lock();
        s.accounts = self.settings.read().accounts.clone();
        s.mcp_enabled = self.settings.read().mcp_enabled;
        self.save_settings(s)
    }

    /// The settings changed by `change` and saved, with the accounts as they then are.
    fn change_claude_accounts<T>(
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

    /// The account `id` names, as the settings now have it.
    fn claude_account(&self, id: &str) -> Result<Account> {
        let settings = self.settings.read();
        let found = settings.accounts.iter().find(|a| a.id == id).cloned();
        found.ok_or_else(|| unknown(id))
    }

    /// The items of Principal's folder a new account may share.
    pub fn claude_shareable(&self) -> Vec<String> {
        let principal = get(&self.settings.read(), PRINCIPAL);
        shareable(&config_dir(&principal))
    }

    /// « Créer et se connecter »: a new account (`create`), its folder in the data folder, last in
    /// the order and active.
    pub fn create_claude_account(
        self: &Arc<Self>,
        name: &str,
        share: &[String],
        mode: ShareMode,
    ) -> Result<Account> {
        let root = self.data.claude_accounts();
        let (made, _) = self.change_claude_accounts(|s| {
            let from = config_dir(&get(s, PRINCIPAL));
            let made = create(s, &root, &from, name, share, mode)?;
            s.accounts.push(made.clone());
            Ok(made)
        })?;
        Ok(made)
    }

    /// The account's name, `claude` and whether new agents may go to it, as the tab gives them
    /// (its folder stays its own); the accounts as they then are.
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
            if others().any(|a| names_of(a).contains(&lower)) {
                return Err(name_taken(name));
            }
            if !account.active && !others().any(|a| a.active) {
                bail!(tr!(
                    "Il faut au moins un compte actif.",
                    "At least one account must be active."
                ));
            }
            let Some(a) = s.accounts.iter_mut().find(|a| a.id == account.id) else {
                return Err(unknown(&account.id));
            };
            a.name = name.to_string();
            a.claude_path = account.claude_path.trim().to_string();
            a.active = account.active;
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

    /// The account removed from the settings, its folder left on the disk (its sign-in and its
    /// sessions with it), but for the MCP server's entry in its `.claude.json` (`withdraw_removed`).
    /// Never Principal, nor an account an agent not archived still runs on.
    pub fn remove_claude_account(self: &Arc<Self>, id: &str) -> Result<Vec<Account>> {
        if id == PRINCIPAL {
            bail!(tr!(
                "Le compte Principal ne se supprime pas.",
                "The Main account can’t be removed."
            ));
        }
        let (gone, accounts) = self.change_claude_accounts(|s| {
            let at = s
                .accounts
                .iter()
                .position(|a| a.id == id)
                .ok_or_else(|| unknown(id))?;
            // An archived agent restored later runs on Principal (`find`).
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
        let target = match account.trim() {
            "" => {
                let project = h.lock().meta.project_id.clone();
                self.account_for_new(&project)
            }
            wanted => {
                let found = settings.accounts.iter().find(|a| a.id == wanted);
                let found = found.ok_or_else(|| unknown(wanted))?;
                if !found.active {
                    bail!(tr!(
                        "Le compte « {name} » est désactivé.",
                        "The account “{name}” is switched off.",
                        name = name_in(crate::i18n::ui(), found)
                    ));
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

    /// Whether Claude Code is found for the account new agents would go to (the current one): its
    /// own `claude`, else the settings'.
    pub fn claude_found(&self) -> bool {
        let settings = self.settings.read().clone();
        let current = self.usage.lock().current.clone();
        program(&get(&settings, &current), &settings).is_some()
    }

    /// Whether the account is signed in, and with which email.
    pub async fn claude_account_status(&self, id: &str) -> Result<AccountStatus> {
        let account = self.claude_account(id)?;
        let sign_in = crate::usage::sign_in(&account);
        let stored = match crate::usage::read_stored(&sign_in.dir, &sign_in.service).await {
            Ok(stored) => Some(stored),
            // A file being written, the keychain refused: not signed in as far as can be told.
            Err(e) => {
                log::debug!("sign-in of account {id}: {e:#}");
                None
            }
        };
        let connected = stored
            .as_ref()
            .is_some_and(|s| s.credentials != crate::usage::Credentials::Missing);
        Ok(AccountStatus {
            connected,
            stamp: stored.and_then(|s| s.stamp),
            email: connected
                .then(|| email_in(&claude_json(&account)))
                .flatten(),
            dir: config_dir(&account).to_string_lossy().into_owned(),
        })
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
            anyhow::anyhow!(tr!(
                "Claude Code introuvable. Installe-le ou indique son chemin dans les réglages.",
                "Claude Code not found. Install it or give its path in the settings."
            ))
        })?;
        let info = TermInfo {
            id: crate::model::new_id(),
            // No project's: the window lists it nowhere.
            project_id: String::new(),
            name: account.name.clone(),
            shell: "claude".into(),
        };
        // Run from the home folder, as the user would.
        let (_, home) = app_env();
        let env = [settings.claude_env(), launch_env(&account)].concat();
        let (me, term) = (Arc::downgrade(self), info.id.clone());
        self.pty.spawn_program(
            info.clone(),
            &program,
            &home.to_string_lossy(),
            size,
            env,
            on_data,
            move |code| {
                if let Some(core) = me.upgrade() {
                    core.hub
                        .emit(crate::model::UiEvent::TerminalExit { id: term, code });
                }
            },
        )?;
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;

    fn account(id: &str, dir: &str) -> Account {
        Account {
            id: id.into(),
            name: id.to_uppercase(),
            config_dir: dir.into(),
            claude_path: String::new(),
            active: true,
        }
    }

    fn ids(s: &Settings) -> Vec<&str> {
        s.accounts.iter().map(|a| a.id.as_str()).collect()
    }

    #[test]
    fn settings_without_accounts_get_principal_active_with_no_folder() {
        let mut s = Settings::default();
        normalize(&mut s);
        assert_eq!(
            s.accounts,
            vec![Account {
                id: "principal".into(),
                name: "Principal".into(),
                config_dir: String::new(),
                claude_path: String::new(),
                active: true,
            }]
        );
        assert_eq!(principal(), s.accounts[0]);
        // Normalized again, nothing changes.
        let before = s.accounts.clone();
        normalize(&mut s);
        assert_eq!(s.accounts, before);
    }

    #[test]
    fn a_list_without_principal_gets_it_first_and_keeps_the_others_as_they_are() {
        let pro = account("pro", r"C:\Users\x\.escouade\claude\pro");
        let mut s = Settings {
            accounts: vec![pro.clone()],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(ids(&s), ["principal", "pro"]);
        assert_eq!(s.accounts[1], pro);
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
        assert_eq!(ids(&s), ["principal", "x", "x-3", "x-2"]);
        let dirs: Vec<&str> = s.accounts.iter().map(|a| a.config_dir.as_str()).collect();
        assert_eq!(dirs, ["", "/a", "/b", "/c"]);
    }

    #[test]
    fn principal_is_made_active_when_no_account_is() {
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
        assert_eq!(active, [false, true]);
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
            serde_json::json!({ "id": "pro", "name": "PRO", "configDir": "/a",
                                "claudePath": "", "active": true })
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
        assert!(launch_env(&principal()).is_empty());
        // Even with a folder written by hand in its settings.
        assert!(launch_env(&account("principal", "/Users/x/.claude")).is_empty());
        let dir = r"C:\Users\x\.escouade\claude\pro";
        assert_eq!(
            launch_env(&account("pro", dir)),
            vec![("CLAUDE_CONFIG_DIR".to_string(), dir.to_string())]
        );
        // Another account with no folder runs as Principal does.
        assert!(launch_env(&account("pro", "")).is_empty());
        assert!(launch_env(&account("pro", "  ")).is_empty());
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
        assert_eq!(program(&with(path(&own)), &settings), Some(own.clone()));
        assert_eq!(program(&with(String::new()), &settings), Some(set.clone()));
        assert_eq!(program(&with("  ".into()), &settings), Some(set));
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
                id: "equipe-pro".into(),
                name: "Équipe Pro".into(),
                config_dir: root.join("equipe-pro").to_string_lossy().into(),
                claude_path: String::new(),
                active: true,
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
            accounts: vec![Account {
                name: "Pro".into(),
                ..account("pro", "/a")
            }],
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
    fn an_unknown_account_is_principal() {
        let mut s = Settings {
            accounts: vec![account("pro", "/a")],
            ..Default::default()
        };
        normalize(&mut s);
        assert_eq!(find(&s, "pro").map(|a| a.id.as_str()), Some("pro"));
        assert_eq!(
            find(&s, "principal").map(|a| a.id.as_str()),
            Some("principal")
        );
        assert_eq!(find(&s, "parti").map(|a| a.id.as_str()), Some("principal"));
        assert_eq!(find(&s, "").map(|a| a.id.as_str()), Some("principal"));
        // Settings never normalized have none.
        assert_eq!(find(&Settings::default(), "pro"), None);
        // A copy, always there.
        assert_eq!(get(&s, "pro"), s.accounts[1]);
        assert_eq!(get(&s, "parti"), s.accounts[0]);
        assert_eq!(get(&Settings::default(), "pro"), principal());
    }

    const NOW: i64 = 1_790_000_000_000;

    /// The account's 5-hour window, used `pct` percent until `ends`.
    fn used(id: &str, pct: f64, ends: i64) -> AccountUsage {
        AccountUsage {
            id: id.into(),
            five_hour: Some(crate::model::RateWindow {
                pct,
                resets_at: Some(ends),
            }),
            ..Default::default()
        }
    }

    /// Principal, Pro and Team, in that order, all active.
    fn three() -> Settings {
        let mut s = Settings {
            accounts: vec![account("pro", "/pro"), account("team", "/team")],
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
        assert_eq!(pick_at(&s, &[]), "principal");
        let end = NOW + 60_000;
        // Under the threshold, whatever its use.
        assert_eq!(pick_at(&s, &[used("principal", 94.0, end)]), "principal");
        // At it (as the autopilot counts it): the next one.
        let first_over = [used("principal", 95.0, end)];
        assert_eq!(pick_at(&s, &first_over), "pro");
        // Its week counts as well as its 5 hours.
        let week = AccountUsage {
            id: "principal".into(),
            seven_day: Some(crate::model::RateWindow {
                pct: 99.0,
                resets_at: Some(end),
            }),
            ..Default::default()
        };
        assert_eq!(pick_at(&s, &[week]), "pro");
        // The next one over as well: the one after.
        let two_over = [used("principal", 100.0, end), used("pro", 96.0, end)];
        assert_eq!(pick_at(&s, &two_over), "team");
        // The order is the settings', not the order of the readings.
        let reversed = [used("pro", 96.0, end), used("principal", 100.0, end)];
        assert_eq!(pick_at(&s, &reversed), "team");
        // A window already over (and the margin after it) holds nothing back.
        let ended = [used("principal", 100.0, NOW - 31_000)];
        assert_eq!(pick_at(&s, &ended), "principal");
        // An account switched off is skipped, over or not.
        deactivate(&mut s, "principal");
        assert_eq!(pick_at(&s, &[]), "pro");
        assert_eq!(pick_at(&s, &[used("pro", 100.0, end)]), "team");
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
        assert_eq!(pick(&s, &all, 95, None, NOW), "principal");
        deactivate(&mut s, "principal");
        assert_eq!(pick(&s, &all, 95, None, NOW), "pro");
        // The threshold is the user's: at 100 % nothing at 99 % is past it.
        let under = [used("principal", 99.0, end), used("pro", 99.0, end)];
        assert_eq!(pick(&three(), &under, 100, None, NOW), "principal");
    }

    #[test]
    fn an_account_asked_for_is_taken_when_active_whatever_its_quota() {
        let mut s = three();
        let end = NOW + 60_000;
        let over = [used("pro", 100.0, end)];
        assert_eq!(pick(&s, &over, 95, Some("pro"), NOW), "pro");
        assert_eq!(pick(&s, &[], 95, Some("team"), NOW), "team");
        // Not one to ask for: the automatic choice (the first usable account).
        let first_over = [used("principal", 100.0, end)];
        for asked in [Some(""), Some("parti"), None] {
            assert_eq!(pick(&s, &first_over, 95, asked, NOW), "pro", "{asked:?}");
        }
        // An account switched off cannot be asked for either.
        deactivate(&mut s, "pro");
        assert_eq!(pick(&s, &first_over, 95, Some("pro"), NOW), "team");
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
        // The use told is that of the window that went the furthest.
        let week = AccountUsage {
            seven_day: Some(crate::model::RateWindow {
                pct: 98.6,
                resets_at: Some(end),
            }),
            ..used("principal", 96.0, end)
        };
        assert_eq!(
            told(Fr, &s, &[week], "principal", "team"),
            Some(
                "Le compte Principal a atteint 99 % : les nouveaux agents partent sur Team.".into()
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
        let only = |ok: &'static str| move |id: &str| id == ok;
        assert_eq!(pick_where(&s, None, only("team")), "team");
        assert_eq!(pick_where(&s, None, only("principal")), "principal");
        assert_eq!(pick_where(&s, None, |_| false), "principal");
        // Asked for, it is taken even if it is not usable.
        assert_eq!(pick_where(&s, Some("pro"), only("team")), "pro");
    }
}
