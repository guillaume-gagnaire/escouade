//! « Vue <harness> »: an agent run in its harness's own interface, in a terminal, in place of the
//! process that Escouade's conversation drives (`tui`). What that takes of a harness, behind one
//! interface (`TerminalView`, `Engine::terminal`):
//! - the launch: its interface's arguments, in the agent's folder, with its account's environment
//!   and Escouade's MCP server, and its hooks pointed at Escouade's hooks' server;
//! - the state: what each of its hooks tells (a turn starts or ends, a permission or a question
//!   waits for the user);
//! - the conversation: its session's log, read as it is written, each entry translated;
//! - the resume: the session found where the interface will look for it, and what the interface
//!   says when it does not find it;
//! - the paste of a message, and the keys that interrupt a turn and end the interface.
//!
//! A harness that calls its hooks as commands (not over HTTP) is given `escouade-hook`
//! (`src/bin/escouade-hook.rs`), which posts what it reads on its input to the same server.

use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::model::AgentMeta;
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::{mcp, BoxFuture, Engine, Escouade, Program};

/// Escouade's hooks' server, as an interface's hooks reach it. `url_var` and `command` are for
/// the harnesses that run their hooks as commands (`escouade-hook`): Claude Code calls them over
/// HTTP.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HookServer<'a> {
    /// Where they post what they tell (`http://127.0.0.1:<port>/hook`).
    pub url: &'a str,
    /// The variable of the interface's environment that holds its token: what the hooks send as
    /// `Authorization: Bearer <token>`.
    pub token_var: &'static str,
    /// The variable that gives `escouade-hook` the server's address (`url`).
    pub url_var: &'static str,
    /// `escouade-hook`, for a harness that runs its hooks as commands; None when it was not found
    /// beside the app.
    pub command: Option<&'a Path>,
}

/// What an interface is started from (`TerminalView::launch`).
pub struct TerminalLaunch<'a> {
    /// The agent: its model, effort, mode, added instructions, and the session it resumes or
    /// copies.
    pub meta: &'a AgentMeta,
    /// The session it starts (or copies into), named by Escouade, when it resumes none and its
    /// harness takes a name for it (`TerminalView::names_its_session`).
    pub session: Option<&'a str>,
    /// The name of its session on claude.ai, when it starts with Remote Control.
    pub remote: Option<&'a str>,
    /// The private file of the interface, which the core writes `TerminalArgs::file` to before it
    /// starts (and that stays its, the agent's own, from one start to the next).
    pub private_file: &'a Path,
    /// The configuration folder of the agent's own (`Engine::agent_home_var`), when its harness
    /// has one: where its interface keeps its sessions and reads its hooks (Mistral Vibe).
    pub home: Option<&'a Path>,
    pub hooks: HookServer<'a>,
    /// Escouade's MCP server given to it, or refused.
    pub access: &'a Escouade,
    /// The paths its tools are refused (« Projet privé »), and its program, which spells them.
    pub fence: &'a crate::fence::Fence,
    pub program: &'a Program,
}

/// How an interface is started.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TerminalArgs {
    pub args: Vec<String>,
    /// What its private file (`TerminalLaunch::private_file`) holds; None: no file.
    pub file: Option<Vec<u8>>,
    /// Other files of its own the core writes before it starts (their folders made): paths beside
    /// its private file, and what each holds.
    pub files: Vec<(PathBuf, Vec<u8>)>,
    /// The variables it gets besides its account's, the token's and the MCP server's.
    pub env: Vec<(String, String)>,
}

/// How an interface joins the process of its agent (`EngineProcess::attach`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Attach {
    pub args: Vec<String>,
    /// The variables it gets besides its account's: what is secret (a password) goes here, never
    /// in `args`, which the system shows to anyone who lists its processes.
    pub env: Vec<(String, String)>,
}

/// The interface of a harness in a terminal (« Vue <harness> »).
pub trait TerminalView: Send + Sync {
    // ----- the launch -----

    /// The name of its terminal, as the window lists it (`claude`).
    fn shell(&self) -> &'static str;
    /// The program of its interface, from its account's (`program`, the one its agents' processes
    /// run): the same, unless its harness has its interface apart (Mistral Vibe: `vibe` beside
    /// `vibe-acp`).
    fn program(&self, program: &Program) -> Program {
        program.clone()
    }
    /// The arguments, private file and variables of its interface.
    fn launch(&self, l: &TerminalLaunch) -> TerminalArgs;
    /// The arguments that let it read the folder `dir` without a question.
    fn read_dir_args(&self, dir: &Path) -> Vec<String>;
    /// Escouade's MCP server for it (`Engine::escouade_launch` of its harness), the account's
    /// configuration folder `config_dir` at hand.
    fn escouade(&self, harness: &dyn Engine, access: &Escouade, _config_dir: &Path) -> mcp::Launch {
        harness.escouade_launch(access)
    }
    /// Whether its interface holds a private project's fence (its settings, its rules, its
    /// client): a view that does not is refused to a fenced agent (ESC-430).
    fn holds_fence(&self) -> bool {
        true
    }
    /// The variables set over those of its environment `env` (its account's folder among them),
    /// to keep from it what the app's own environment would give it: nothing, unless its harness
    /// would take another account from it (Copilot CLI: a GitHub token, for an account of its own).
    fn masked_env(&self, _env: &[(String, String)]) -> Vec<(String, String)> {
        Vec::new()
    }
    /// What is added to its arguments once they are known (`args`, with its environment `env`),
    /// just before it starts: nothing, unless its harness has to be asked first.
    fn prepare<'a>(
        &'a self,
        _program: &'a Program,
        _cwd: &'a str,
        _args: &'a [String],
        _env: &'a [(String, String)],
    ) -> BoxFuture<'a, Result<Vec<String>>> {
        Box::pin(async { Ok(Vec::new()) })
    }
    /// Its arguments `args` (with its environment `env`) once those its account would refuse are
    /// changed, just before it starts: as they are, unless its harness checks some first (Copilot
    /// CLI: a model its account does not list).
    fn for_account<'a>(
        &'a self,
        _program: &'a Program,
        _cwd: &'a str,
        args: Vec<String>,
        _env: &'a [(String, String)],
    ) -> BoxFuture<'a, Vec<String>> {
        Box::pin(async { args })
    }
    /// What its interface left of the agent once the agent is deleted: beside its private file
    /// `private_file`, and in the folder `config_dir` of its account. Nothing, unless its harness
    /// keeps something there.
    fn forget(&self, _private_file: &Path, _config_dir: &Path) {}
    /// The copy of the session `of` (up to its entry `at`, all of it with none) made before the
    /// interface starts, in `cwd` with `env`, when it cannot copy one itself: the copy's id. None:
    /// it copies a session itself (`TerminalView::launch`, from `AgentMeta::fork_of`).
    fn copy_first<'a>(
        &'a self,
        _program: &'a Program,
        _cwd: &'a str,
        _env: &'a [(String, String)],
        _of: &'a str,
        _at: Option<&'a str>,
    ) -> Option<BoxFuture<'a, Result<String>>> {
        None
    }
    /// Escouade names a new session before the interface starts (its log is then known before any
    /// hook); false: the interface names its own, which its first hook tells.
    fn names_its_session(&self) -> bool;
    /// Its interface opens any session it is given, a new one under that id when it has none
    /// (Cursor): the session to resume is not looked for first, nor ever lost.
    fn resumes_any(&self) -> bool {
        false
    }
    /// Where its interface, started in the agent's folder `cwd`, reads what Escouade tells it of
    /// the conversation so far, when it keeps the conversations of its interface apart from its
    /// session's (Cursor): a file in that folder, written as the view opens, kept out of git and
    /// taken away on the way back. None: the interface resumes the conversation itself.
    fn relay_path(&self, _cwd: &Path) -> Option<PathBuf> {
        None
    }
    /// What that file holds for the relay `text` (`handover::relay`).
    fn relay_file(&self, text: &str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }

    // ----- the state -----

    /// One of its hooks, the agent's state changed by what it tells.
    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome;
    /// The folder where it lists its running interfaces (`find_running`), in the account's
    /// config folder `config_dir`; None: it lists none.
    fn running_dir(&self, config_dir: &Path) -> Option<PathBuf>;
    /// Its interface of `session` among those `dir` lists, the file that lists it (`known`, once
    /// found) read alone: it is up, past what it asks before it reads a message. Never blocks for
    /// long nor fails.
    fn find_running(
        &self,
        dir: &Path,
        session: &str,
        known: Option<&Path>,
    ) -> Option<crate::tui::Registered>;

    // ----- the conversation -----

    /// The log of `session` in the account's config folder `config_dir` (the agent's own folder,
    /// when its harness gives it one: `Engine::agent_home_var`).
    fn find_log(&self, config_dir: &Path, session: &str) -> Option<PathBuf>;
    /// The log of a session goes on in another file while its interface runs (Mistral Vibe: a
    /// segment of its journal for each turn): `find_log` is asked again once the file read is read
    /// to its end, and the new file is read from its start.
    fn log_moves(&self) -> bool {
        false
    }
    /// One entry of its log, translated into the agent's conversation.
    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects);

    // ----- the resume -----

    /// Whether what the interface wrote before it quit says it did not find the session it was to
    /// resume.
    fn says_session_lost(&self, output: &[u8]) -> bool;
    /// Copies the session `session` from the account's config folder `from` into `to`'s.
    fn copy_session(&self, from: &Path, to: &Path, session: &str) -> Result<()>;

    // ----- the paste and the interruption -----

    /// Why a message may not be pasted in it now, though it is up (in the agent's folder `cwd`,
    /// its account's configuration folder `config_dir`; `spoke`: it told something since it
    /// started, a hook or an entry of its log): a question of its own on its screen, which a text
    /// typed would answer. None: it may.
    fn paste_refusal(&self, _config_dir: &Path, _cwd: &str, _spoke: bool) -> Option<String> {
        None
    }

    /// What is typed in it for a message `text`, before `submit` sends it: pasted, so that its
    /// line breaks stay in it.
    fn paste(&self, text: &str) -> Vec<u8>;
    /// The key that sends what was pasted.
    fn submit(&self) -> &'static [u8];
    /// The types of the images it takes as a path pasted in it (none: it takes no image so).
    fn image_types(&self) -> &'static [&'static str];
    /// The key that interrupts its turn.
    fn interrupt_key(&self) -> &'static [u8];
    /// When its interrupt key has to be pressed again to confirm (Copilot: « esc again to
    /// interrupt »), how long after the first: None, once is enough.
    fn interrupt_again(&self) -> Option<std::time::Duration> {
        None
    }
    /// The key that ends it for good when sent twice, `quit_gap` apart.
    fn quit_key(&self) -> &'static [u8];
    /// Its turn was interrupted from the window (`interrupt_key` sent): nothing to do, unless its
    /// interface tells nothing of a turn it stopped (Gemini CLI: no hook then), whose end is then
    /// the agent's.
    fn interrupted(&self, _rt: &mut AgentRt, _fx: &mut Effects) {}
}

/// A text pasted in a terminal (bracketed paste), the sequence's own escape taken out of it, and
/// its line breaks as a terminal pastes them (xterm.js too): `\r`. Claude Code under Windows drops
/// a `\n` pasted (ESC-12).
pub fn bracketed(text: &str) -> Vec<u8> {
    let text = text
        .replace('\x1b', "")
        .replace("\r\n", "\r")
        .replace('\n', "\r");
    format!("\x1b[200~{text}\x1b[201~").into_bytes()
}

/// The name of the `escouade-hook` command on this platform.
pub const HOOK_COMMAND: &str = if cfg!(windows) {
    "escouade-hook.exe"
} else {
    "escouade-hook"
};

/// `escouade-hook`, beside the app's program (where the installers put it, and `cargo build`
/// too). In the tests, the app's program is the test binary, in `deps`: it is one folder up.
pub fn hook_command() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let found = [Some(dir), dir.parent()]
        .into_iter()
        .flatten()
        .map(|d| d.join(HOOK_COMMAND))
        .find(|p| p.is_file());
    found
}

/// The variable that keeps `cmd` from looking for a program named alone in its current folder
/// first (ESC-430): the agent's, where an `escouade-hook.cmd` would run at each hook.
pub(crate) const NO_CURRENT_DIR: &str = "NoDefaultCurrentDirectoryInExePath";

/// A path of Windows written with `/` and no quote, when a line of `cmd`, PowerShell or Git Bash
/// holds it so (tried on Windows 11), and a program in Go passes it to `cmd` as it is (a quote
/// becomes `\"`, which `cmd` does not read); None for a path with a space, an accent or a sign a
/// shell reads.
pub(crate) fn plain_windows_path(path: &str) -> Option<String> {
    let plain = !path.is_empty()
        && path.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '\\' | '/' | ':' | '.' | '_' | '-' | '~')
        });
    plain.then(|| path.replace('\\', "/"))
}

/// How a hook's line runs `escouade-hook` (`command`): by its whole path, quoted for a POSIX
/// shell, under Windows (`windows`) with `/` and no quote; by its name only when a line cannot
/// hold its path so (its folder first on the `PATH`, `hook_env`).
pub(crate) fn hook_program(command: &Path, windows: bool) -> String {
    let path = command.to_string_lossy();
    match windows {
        true => plain_windows_path(&path).unwrap_or_else(|| "escouade-hook".into()),
        false => format!("'{}'", path.replace('\'', r"'\''")),
    }
}

/// Under Windows, the variables of an interface whose hooks run `escouade-hook` (`command`): the
/// app's `PATH` with its folder first (`path`), for a line that calls it by its name, and
/// `NoDefaultCurrentDirectoryInExePath`, so that `cmd` never runs one of its current folder.
pub(crate) fn hook_env(command: &Path, path: &std::ffi::OsStr) -> Vec<(String, String)> {
    let mut env = Vec::new();
    if let Some(dir) = command.parent() {
        let joined = std::env::join_paths(
            std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(path)),
        )
        .unwrap_or_else(|_| path.to_os_string());
        env.push(("PATH".into(), joined.to_string_lossy().into_owned()));
    }
    env.push((NO_CURRENT_DIR.into(), "1".into()));
    env
}

#[cfg(test)]
mod hook_tests {
    use super::*;

    #[test]
    fn a_hook_runs_escouade_hook_by_its_whole_path_and_never_one_of_its_current_folder() {
        let plain = Path::new(r"C:\Users\ana\AppData\Local\Escouade\escouade-hook.exe");
        assert_eq!(
            hook_program(plain, true),
            "C:/Users/ana/AppData/Local/Escouade/escouade-hook.exe"
        );
        // What a line cannot hold without quotes: by its name, its folder first on the PATH.
        for odd in [
            r"C:\Program Files\Escouade\escouade-hook.exe",
            r"C:\Users\Zoé\escouade-hook.exe",
            r"C:\a&b\escouade-hook.exe",
            r"C:\a%PATH%\escouade-hook.exe",
            r"C:\O'Brien\escouade-hook.exe",
            r"C:\a(1)\escouade-hook.exe",
        ] {
            assert_eq!(hook_program(Path::new(odd), true), "escouade-hook", "{odd}");
        }
        assert_eq!(
            hook_program(Path::new("/opt/it's/escouade-hook"), false),
            r"'/opt/it'\''s/escouade-hook'"
        );
        let env = hook_env(plain, std::ffi::OsStr::new(""));
        assert!(env.iter().any(|(k, v)| k == NO_CURRENT_DIR && v == "1"));
        if cfg!(windows) {
            assert!(env
                .iter()
                .any(|(k, v)| k == "PATH" && v.contains("Escouade")));
        }
    }
}
