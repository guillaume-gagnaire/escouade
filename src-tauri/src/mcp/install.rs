//! Escouade's server declared in Claude Code, account by account (« Claude peut piloter
//! Escouade »): `claude mcp add --scope user --transport http escouade <url> --header
//! "Authorization: Bearer <token>"`, run as the account runs Claude Code (its `claude`, its
//! `CLAUDE_CONFIG_DIR`), so that the entry lands in that account's `.claude.json`.
//!
//! The CLI only writes. What is declared is read from `.claude.json` itself: `claude mcp get`
//! connects to the server, which is slow, and a failed connection says nothing of the entry.

use super::activity::clip;
use crate::accounts;
use crate::core::Core;
use crate::model::{Account, Settings};
use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::Runtime;

/// The name Claude Code knows the server by: its tools are `mcp__escouade__<tool>`.
pub const NAME: &str = "escouade";

/// How long one `claude mcp` command may take.
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

/// What stands in the place of a token that must not be shown or kept in a log.
pub const MASK: &str = "••••••••";

/// What the server is declared as in Claude Code's `.claude.json`.
#[derive(Clone, PartialEq, Eq)]
pub struct Entry {
    pub url: String,
    pub token: String,
}

// By hand: the token is not printed by a `{:?}`, a failed assertion or a log line.
impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("url", &self.url)
            .field("token", &MASK)
            .finish()
    }
}

/// `text` with the bearer tokens it holds hidden (`Bearer <token>`, up to a blank or a quote):
/// what the window shows of a command.
pub fn mask_bearer(text: &str) -> String {
    const BEARER: &str = "Bearer ";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(BEARER) {
        let (head, tail) = rest.split_at(at + BEARER.len());
        out.push_str(head);
        out.push_str(MASK);
        let end = tail
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
            .unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// `text` with every one of `secrets` (a token, whole) hidden, and then any bearer token.
fn mask_secrets(text: &str, secrets: &[&str]) -> String {
    let hidden = secrets
        .iter()
        .filter(|s| !s.is_empty())
        .fold(text.to_string(), |t, s| t.replace(*s, MASK));
    mask_bearer(&hidden)
}

/// How an account's Claude Code is run, and where it keeps its state.
#[derive(Debug, Clone)]
pub struct Target {
    /// Its `claude`.
    pub program: PathBuf,
    /// What its processes get besides the environment of the app: the network settings, and the
    /// account's `CLAUDE_CONFIG_DIR` (never Principal's).
    pub env: Vec<(String, String)>,
    /// Where the command runs.
    pub cwd: PathBuf,
    /// Its `.claude.json`, which holds the user scope's servers.
    pub claude_json: PathBuf,
    pub timeout: Duration,
}

/// What `declare` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Declared {
    /// There was no entry: added.
    Added,
    /// There was another one: replaced.
    Replaced,
    /// It was there as wanted: left as it was.
    Unchanged,
}

/// The entry that reaches the server on `port` with `token`.
pub fn wanted(port: u16, token: &str) -> Entry {
    Entry {
        url: format!("http://127.0.0.1:{port}{}", super::PATH),
        token: token.to_string(),
    }
}

/// What a `.claude.json` says of the server.
enum Standing {
    /// There is no such file.
    NoFile,
    /// There is a file that cannot be read or is no JSON: nothing can be told of the server.
    Unreadable,
    /// The file has no entry of that name.
    Absent,
    /// It has one. One that is not an HTTP server with a bearer token has an empty url or token: it
    /// is there, and it is not the one wanted.
    Declared(Entry),
}

fn standing(claude_json: &Path) -> Standing {
    let text = match std::fs::read_to_string(claude_json) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Standing::NoFile,
        Err(_) => return Standing::Unreadable,
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return Standing::Unreadable;
    };
    let Some(found) = json.get("mcpServers").and_then(|s| s.get(NAME)) else {
        return Standing::Absent;
    };
    let http = found["type"] == "http";
    let url = found["url"].as_str().filter(|_| http).unwrap_or_default();
    let token = found["headers"]["Authorization"]
        .as_str()
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or_default();
    Standing::Declared(Entry {
        url: url.to_string(),
        token: token.to_string(),
    })
}

/// The entry `.claude.json` (`claude_json`) holds under the server's name, if it holds one. A file
/// that is not there or not JSON holds none.
pub fn current(claude_json: &Path) -> Option<Entry> {
    match standing(claude_json) {
        Standing::Declared(entry) => Some(entry),
        _ => None,
    }
}

/// The arguments of `claude mcp add`: the header last, since Claude Code lets it take every
/// argument that follows.
fn add_args(entry: &Entry) -> Vec<String> {
    let header = format!("Authorization: Bearer {}", entry.token);
    [
        "mcp",
        "add",
        "--scope",
        "user",
        "--transport",
        "http",
        NAME,
        &entry.url,
        "--header",
        &header,
    ]
    .map(String::from)
    .to_vec()
}

fn remove_args() -> Vec<String> {
    ["mcp", "remove", NAME, "--scope", "user"]
        .map(String::from)
        .to_vec()
}

/// Runs `claude` with `args` as the account does, within its time. Err with what Claude Code
/// said when it fails, any of `secrets` (the tokens in play) hidden: the error goes to the window
/// and the logs.
async fn run(target: &Target, args: &[String], secrets: &[&str]) -> Result<()> {
    let mut cmd = tokio::process::Command::new(&target.program);
    cmd.args(args)
        .current_dir(&target.cwd)
        .envs(target.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(crate::claude::CREATE_NO_WINDOW);
    crate::job::isolate(&mut cmd);
    let child = cmd.spawn().map_err(|e| {
        anyhow!(tr!(
            "claude n’a pas pu être lancé : {e}",
            "claude could not be started: {e}"
        ))
    })?;
    // Dropped when this returns: whatever the command left running ends with it.
    let _job = crate::job::Job::for_child(&child);
    let out = tokio::time::timeout(target.timeout, child.wait_with_output())
        .await
        .map_err(|_| {
            anyhow!(tr!(
                "pas de réponse de claude en {s} s",
                "no answer from claude in {s} s",
                s = target.timeout.as_secs()
            ))
        })??;
    if out.status.success() {
        return Ok(());
    }
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_string();
    let said = [text(&out.stderr), text(&out.stdout)]
        .into_iter()
        .find(|t| !t.is_empty());
    match said {
        Some(said) => bail!("{}", clip(&mask_secrets(&said, secrets))),
        None => bail!(tr!(
            "claude s’est terminé avec le code {code}",
            "claude exited with code {code}",
            code = out.status.code().unwrap_or(-1)
        )),
    }
}

/// The server declared in the account of `target` as `entry` says: added; replaced (removed, then
/// added) when the entry there is another; left alone, with no command run, when it is the same.
pub async fn declare(target: &Target, entry: &Entry) -> Result<Declared> {
    let had = current(&target.claude_json);
    if had.as_ref() == Some(entry) {
        return Ok(Declared::Unchanged);
    }
    let secrets = [entry.token.as_str(), had.as_ref().map_or("", |h| &h.token)];
    if had.is_some() {
        // `add` of a name already there fails.
        run(target, &remove_args(), &secrets).await?;
    }
    run(target, &add_args(entry), &secrets).await?;
    Ok(if had.is_some() {
        Declared::Replaced
    } else {
        Declared::Added
    })
}

/// The server's entry removed from the account of `target`; whether there was one. No file or no
/// entry in it, no command; a file that cannot be read may hold one, so the command is run, and its
/// failure is a failure. A command that fails with the entry gone all the same is not one.
pub async fn withdraw(target: &Target) -> Result<bool> {
    let had = standing(&target.claude_json);
    let token = match &had {
        Standing::NoFile | Standing::Absent => return Ok(false),
        Standing::Declared(entry) => entry.token.as_str(),
        Standing::Unreadable => "",
    };
    if let Err(e) = run(target, &remove_args(), &[token]).await {
        // Removed meanwhile: Claude Code answers an absent entry with an error too.
        if matches!(
            standing(&target.claude_json),
            Standing::NoFile | Standing::Absent
        ) {
            return Ok(true);
        }
        return Err(e);
    }
    Ok(true)
}

/// `s` as one word of PowerShell.
fn powershell_word(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// `s` as one word of a POSIX shell.
fn posix_word(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The command that declares `entry` by hand, in the shell of the platform (`windows`: PowerShell,
/// else a POSIX shell), for an account of folder `config_dir` (none for Principal). `program` is
/// the `claude` as the user named it (`claude` by default). `replace`: an entry is there, which is
/// removed first (the shell goes on whether or not it was). The folder is set for the command
/// alone: not left set in the user's shell, where it would send the next `claude` to this account.
pub fn manual_command_in(
    windows: bool,
    program: &str,
    config_dir: Option<&str>,
    entry: &Entry,
    replace: bool,
) -> String {
    let word = |s: &str| {
        if windows {
            powershell_word(s)
        } else {
            posix_word(s)
        }
    };
    let claude = if program == "claude" {
        program.to_string()
    } else if windows {
        format!("& {}", word(program))
    } else {
        word(program)
    };
    let header = format!("Authorization: Bearer {}", entry.token);
    let add = format!(
        "{claude} mcp add --scope user --transport http {NAME} {} --header \"{header}\"",
        entry.url
    );
    let remove = format!("{claude} mcp remove {NAME} --scope user");
    let commands = if replace {
        vec![remove, add]
    } else {
        vec![add]
    };
    let Some(dir) = config_dir else {
        return commands.join("; ");
    };
    if windows {
        let dir = word(dir);
        format!(
            "$old = $env:CLAUDE_CONFIG_DIR; $env:CLAUDE_CONFIG_DIR = {dir}; {}; $env:CLAUDE_CONFIG_DIR = $old",
            commands.join("; ")
        )
    } else {
        let set = format!("CLAUDE_CONFIG_DIR={}", word(dir));
        commands
            .iter()
            .map(|c| format!("{set} {c}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Where the server stands in one account's Claude Code, as the window shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Declaration {
    /// The id of the Claude account.
    pub account: String,
    /// The server is declared in it (as wanted).
    pub ok: bool,
    /// Why it is not.
    pub error: Option<String>,
    /// The command that declares it in this account by hand, its token hidden (`MASK`): the real
    /// one is built when the user asks to copy it (`Core::mcp_manual_command`), and is never kept
    /// nor sent with the state.
    pub command: String,
}

// The declaration in each account, kept in step with the settings: « Claude peut piloter Escouade »
// turned on or off, an account added, switched on or off, the server on another port.
impl<R: Runtime> Core<R> {
    /// The app's `CLAUDE_CONFIG_DIR` and home folder, which Principal's `claude` goes by
    /// (`accounts::app_env`). In unit tests every core has a home of its own: the fake `claude`
    /// is told it and never goes by the real one.
    fn claude_app_env(&self) -> (Option<String>, PathBuf) {
        if cfg!(test) {
            return (None, self.data.root().join("claude-home"));
        }
        accounts::app_env()
    }

    /// How the account's Claude Code is run to declare the server: its `claude`, its network
    /// settings, its folder (never Principal's), from the data folder.
    fn install_target(&self, settings: &Settings, account: &Account) -> Result<Target> {
        let program = accounts::program(account, settings).ok_or_else(|| {
            anyhow!(tr!(
                "Claude Code introuvable. Installe-le ou indique son chemin dans les réglages.",
                "Claude Code not found. Install it or give its path in the settings."
            ))
        })?;
        let (env, home) = self.claude_app_env();
        let mut vars = [settings.claude_env(), accounts::launch_env(account)].concat();
        if cfg!(test) {
            // Tests may run inside a Claude Code session with a `CLAUDE_CONFIG_DIR` of its own, which
            // Principal's processes would inherit: blank, the fake `claude` goes by its home alone.
            if vars.iter().all(|(k, _)| k != "CLAUDE_CONFIG_DIR") {
                vars.push(("CLAUDE_CONFIG_DIR".into(), String::new()));
            }
            vars.push((
                "FAKE_CLAUDE_HOME".into(),
                home.to_string_lossy().into_owned(),
            ));
        }
        Ok(Target {
            program,
            env: vars,
            cwd: self.data.root().to_path_buf(),
            claude_json: accounts::claude_json_with(account, env.as_deref(), &home),
            timeout: COMMAND_TIMEOUT,
        })
    }

    /// What the account would be told to run by hand to declare `entry`.
    fn manual_command(&self, settings: &Settings, account: &Account, entry: &Entry) -> String {
        let (env, home) = self.claude_app_env();
        let claude_json = accounts::claude_json_with(account, env.as_deref(), &home);
        let typed = [account.claude_path.trim(), settings.claude_path.trim()]
            .into_iter()
            .find(|p| !p.is_empty())
            .unwrap_or("claude");
        manual_command_in(
            cfg!(windows),
            typed,
            accounts::own_dir(account),
            entry,
            current(&claude_json).is_some(),
        )
    }

    /// The server declared in `account`: how that went.
    async fn declare_in(
        &self,
        settings: &Settings,
        account: &Account,
        entry: &Entry,
    ) -> Declaration {
        let done = match self.install_target(settings, account) {
            Ok(target) => declare(&target, entry).await.map(|_| ()),
            Err(e) => Err(e),
        };
        if let Err(e) = &done {
            log::warn!("mcp: not declared for account {}: {e:#}", account.id);
        }
        Declaration {
            account: account.id.clone(),
            ok: done.is_ok(),
            error: done.err().map(|e| format!("{e:#}")),
            command: mask_bearer(&self.manual_command(settings, account, entry)),
        }
    }

    /// The command that declares the server by hand in the account `account`, with the real token
    /// (« Copier la commande »). Only while the switch is on and the server runs: there is a token
    /// and a port to give.
    pub fn mcp_manual_command(&self, account: &str) -> Result<String> {
        let settings = self.settings.read().clone();
        let account = settings
            .accounts
            .iter()
            .find(|a| a.id == account)
            .ok_or_else(|| {
                anyhow!(tr!(
                    "compte Claude « {account} » introuvable",
                    "Claude account “{account}” not found"
                ))
            })?;
        let status = self.mcp.status();
        if !settings.mcp_enabled || !status.running {
            bail!(tr!(
                "Le serveur MCP ne tourne pas : active « Claude peut piloter Escouade ».",
                "The MCP server is not running: turn on “Claude can drive Escouade”."
            ));
        }
        let token = self.mcp.external_token()?;
        Ok(self.manual_command(&settings, account, &wanted(status.port, &token)))
    }

    /// « Claude peut piloter Escouade » turned on or off: saved like any setting, which starts or
    /// stops the server and declares it in the accounts (or takes it out of them). One change of
    /// the settings at a time, with those of the Claude accounts.
    pub fn set_mcp_enabled(self: &Arc<Self>, enabled: bool) -> Result<()> {
        let _one = self.claude_accounts_lock.lock();
        let mut settings = self.settings.read().clone();
        settings.mcp_enabled = enabled;
        self.save_settings(settings)
    }

    /// The declaration in Claude Code is to be brought in step with the settings: on its own task
    /// (the commands are slow), unless nothing is to be done (off, and nothing declared to take out).
    pub(crate) fn declare_in_claude(&self) {
        if !self.settings.read().mcp_enabled && !self.mcp.withdraw.load(Ordering::Acquire) {
            return;
        }
        let Some(core) = self.weak().upgrade() else {
            return;
        };
        tauri::async_runtime::spawn(async move { core.declare_now().await });
    }

    /// A Claude account was removed from the settings: its entry leaves its `.claude.json` (the
    /// folder stays on the disk, and the token would open the server from there for as long as the
    /// switch is on), whether the switch is on or off (a leftover goes too). On its own task, after
    /// the declaration under way; a failure is only logged. The token is not changed: the other
    /// accounts still hold it.
    pub(crate) fn withdraw_removed(&self, gone: Account) {
        let Some(core) = self.weak().upgrade() else {
            return;
        };
        tauri::async_runtime::spawn(async move {
            {
                let _one = core.mcp.declaring.lock().await;
                if !core.quitting.load(Ordering::Acquire) {
                    let settings = core.settings.read().clone();
                    let done = match core.install_target(&settings, &gone) {
                        Ok(target) => withdraw(&target).await.map(|_| ()),
                        Err(e) => Err(e),
                    };
                    if let Err(e) = done {
                        log::warn!(
                            "mcp: not taken out of the removed account {}: {e:#}",
                            gone.id
                        );
                    }
                }
            }
            #[cfg(test)]
            core.mcp.runs.fetch_add(1, Ordering::AcqRel);
        });
    }

    /// Brings Claude Code in step with the settings, one run at a time:
    /// - on, with the server running: the server declared in every active account (left alone when
    ///   it is there as wanted, replaced when it is another, added when it is absent), each account's
    ///   result told to the window;
    /// - off, after it was on: taken out of every account, whether active or not (the token was
    ///   changed when it was turned off, `McpServer::turned_off`, so that an entry that stays — a
    ///   refusal, a `claude` that is gone, a crash, the app quitting before this run — opens
    ///   nothing).
    pub(crate) async fn declare_now(&self) {
        self.declare_run().await;
        #[cfg(test)]
        self.mcp.runs.fetch_add(1, Ordering::AcqRel);
    }

    async fn declare_run(&self) {
        let _one = self.mcp.declaring.lock().await;
        let withdrawing = self.mcp.withdraw.swap(false, Ordering::AcqRel);
        if self.quitting.load(Ordering::Acquire) {
            return;
        }
        let settings = self.settings.read().clone();
        if !settings.mcp_enabled {
            if withdrawing {
                for account in &settings.accounts {
                    let Ok(target) = self.install_target(&settings, account) else {
                        continue;
                    };
                    if let Err(e) = withdraw(&target).await {
                        log::warn!("mcp: not taken out of account {}: {e:#}", account.id);
                    }
                }
            }
            self.mcp.tell_declared(Vec::new());
            return;
        }
        // Turned off and on again before the run: the token changed when it was turned off, and the
        // entries are replaced below, with the new one.
        let status = self.mcp.status();
        let token = self.mcp.external_token();
        let (true, Ok(token)) = (status.running, token) else {
            // Nothing to reach (the window says why: the server's own error).
            self.mcp.tell_declared(Vec::new());
            return;
        };
        let wanted = wanted(status.port, &token);
        let mut declared = Vec::new();
        for account in settings.accounts.iter().filter(|a| a.active) {
            declared.push(self.declare_in(&settings, account, &wanted).await);
        }
        self.mcp.tell_declared(declared);
    }
}
