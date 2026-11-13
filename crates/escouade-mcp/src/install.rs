//! The parts of Escouade’s declaration in a harness that stand without the app: how an account’s
//! program is run (`Target`, `run`), the entry wanted (`wanted`), what a declaration did
//! (`Declared`) and where the server stands in an account (`Declaration`). The app declares it
//! harness by harness (`mcp::install`).

use crate::activity::clip;
use anyhow::{anyhow, bail, Result};
use escouade_model::model::EngineId;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

/// How long one `claude mcp` command may take.
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

// The server's entry, what a user config says of it and how a token is hidden: `escouade-engines`, which the
// harnesses' user configs (`engine::userconfig`) read too.
pub use escouade_engines::mcp::{mask_bearer, Entry, Standing, MASK};

/// `text` with every one of `secrets` (a token, whole) hidden, and then any bearer token.
pub fn mask_secrets(text: &str, secrets: &[&str]) -> String {
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
    pub program: escouade_engines::program::Program,
    /// What its processes get besides the environment of the app: the network settings, and the
    /// account's `CLAUDE_CONFIG_DIR` (never Principal's).
    pub env: Vec<(String, String)>,
    /// Where the command runs.
    pub cwd: PathBuf,
    /// Its user config, which holds the servers it reaches in every folder (Claude Code's
    /// `.claude.json`).
    pub config: PathBuf,
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
        url: format!("http://127.0.0.1:{port}{}", crate::PATH),
        token: token.to_string(),
    }
}

/// Runs the account's program with `args` as the account does, within its time. Err with what Claude Code
/// said when it fails, any of `secrets` (the tokens in play) hidden: the error goes to the window
/// and the logs.
pub async fn run(target: &Target, args: &[String], secrets: &[&str]) -> Result<()> {
    let launch = target
        .program
        .launch(&target.cwd.to_string_lossy(), args, &target.env)?;
    let mut cmd = tokio::process::Command::new(&launch.program);
    cmd.args(&launch.args)
        .current_dir(&target.cwd)
        .envs(launch.env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(escouade_model::proc::CREATE_NO_WINDOW);
    escouade_model::job::isolate(&mut cmd);
    let child = cmd.spawn().map_err(|e| {
        anyhow!(tr!(
            "claude n’a pas pu être lancé : {e}",
            "claude could not be started: {e}"
        ))
    })?;
    // Dropped when this returns: whatever the command left running ends with it.
    let _job = escouade_model::job::Job::for_child(&child);
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

/// Where the server stands in one account of a harness, as the window shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Declaration {
    /// The id of the account.
    pub account: String,
    /// Its harness.
    pub engine: EngineId,
    /// The server is declared in it (as wanted).
    pub ok: bool,
    /// Why it is not.
    pub error: Option<String>,
    /// The command that declares it in this account by hand (for a harness with a user config
    /// Escouade writes itself, the entry to paste in `file`), its token hidden (`MASK`): the real
    /// one is built when the user asks to copy it (`Core::mcp_manual_command`), and is never kept
    /// nor sent with the state.
    pub command: String,
    /// The user config the entry is in, with the token in plain text (for a link, the file it
    /// leads to): `.claude.json`, `config.toml`, `opencode.json`, `mcp-config.json`.
    pub file: String,
    /// `command` is the entry to paste in `file` (a harness Escouade writes it in itself); else a
    /// command to run.
    pub paste: bool,
}
