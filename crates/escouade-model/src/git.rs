//! The git commands of the app, run by the `git` CLI within their time limits (the app's `git` re-exports them,
//! docs/CRATES.md): what the plugins' registry clones with as well as the app's worktrees and branches.

use anyhow::{bail, Result};
use std::process::Stdio;
use std::time::Duration;

pub fn command(cwd: &str, args: &[&str]) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("-C")
        .arg(cwd)
        .arg("--no-optional-locks")
        // Accented paths verbatim (UTF-8) instead of "\303\251" escapes in diff headers.
        .args(["-c", "core.quotepath=false"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    cmd
}

/// Runs a git command that reads (status, diff, log, rev-parse…) or moves little: killed past
/// `GIT_TIMEOUT`. One that runs the user's hooks goes through `run_hooks`.
pub async fn run(cwd: &str, args: &[&str]) -> Result<Vec<u8>> {
    run_within(cwd, args, Limit::Reads).await
}

/// `run`, for the commands that run the user's hooks (commit, merge, rebase, worktree add,
/// switch…): a pre-commit hook may run a whole test suite, a post-checkout one an install.
pub async fn run_hooks(cwd: &str, args: &[&str]) -> Result<Vec<u8>> {
    run_within(cwd, args, Limit::Hooks).await
}

pub async fn run_within(cwd: &str, args: &[&str], limit: Limit) -> Result<Vec<u8>> {
    let out = output_within(command(cwd, args), args, limit).await?;
    checked(out, args)
}

/// Time limit of the git commands that read.
pub const GIT_TIMEOUT: Duration = Duration::from_secs(120);
/// Time limit of the git commands that run the user's hooks.
pub const GIT_HOOKS_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// How long a git command may take.
#[derive(Clone, Copy)]
pub enum Limit {
    Reads,
    Hooks,
}

impl Limit {
    pub fn duration(self) -> Duration {
        #[cfg(any(test, feature = "testing"))]
        if let Some((reads, hooks)) = SHORT_LIMITS.with(std::cell::Cell::get) {
            return match self {
                Limit::Reads => reads,
                Limit::Hooks => hooks,
            };
        }
        match self {
            Limit::Reads => GIT_TIMEOUT,
            Limit::Hooks => GIT_HOOKS_TIMEOUT,
        }
    }
}

#[cfg(any(test, feature = "testing"))]
thread_local! {
    /// Shorter limits (reads, hooks) for the test running on this thread: nobody waits minutes.
    pub static SHORT_LIMITS: std::cell::Cell<Option<(Duration, Duration)>> =
        const { std::cell::Cell::new(None) };
}

/// What `cmd` (a git command that reads) prints, within `GIT_TIMEOUT` and without waiting for
/// the end of its pipes once it has ended (see `proc`).
pub async fn output(cmd: tokio::process::Command, args: &[&str]) -> Result<std::process::Output> {
    output_within(cmd, args, Limit::Reads).await
}

pub async fn output_within(
    cmd: tokio::process::Command,
    args: &[&str],
    limit: Limit,
) -> Result<std::process::Output> {
    let limit = limit.duration();
    match crate::proc::output_within(cmd, limit).await {
        Ok(out) => Ok(out),
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Err(stopped(args, limit)),
        Err(e) => Err(e.into()),
    }
}

/// A command killed for not being done within `limit`.
pub fn stopped(args: &[&str], limit: Duration) -> anyhow::Error {
    anyhow::anyhow!(stopped_in(crate::i18n::ui(), args, limit))
}

/// What `stopped` says: the git command (its subcommand) and the limit it ran into, in seconds up
/// to two minutes and in minutes past that (a commit running hooks has half an hour).
pub fn stopped_in(lang: crate::i18n::Lang, args: &[&str], limit: Duration) -> String {
    let secs = limit.as_secs();
    let after = if secs > 120 && secs.is_multiple_of(60) {
        format!("{} min", secs / 60)
    } else {
        format!("{secs} s")
    };
    let cmd = subcommand(args)
        .map(|c| format!(" {c}"))
        .unwrap_or_default();
    tr_in!(
        lang,
        "git{cmd} interrompu : toujours pas terminé après {after}",
        "git{cmd} stopped: still not done after {after}",
        cmd = cmd,
        after = after
    )
}

/// The git command `args` run: the first word that is no option (`--literal-pathspecs`,
/// `-c key=value`...).
pub fn subcommand<'a>(args: &[&'a str]) -> Option<&'a str> {
    let mut args = args.iter().copied();
    while let Some(arg) = args.next() {
        match arg {
            // These two take a value.
            "-c" | "-C" => {
                args.next();
            }
            _ if arg.starts_with('-') => {}
            _ => return Some(arg),
        }
    }
    None
}

/// Time limits of the commands that reach a remote.
pub const NET_TIMEOUT: Duration = Duration::from_secs(180);
pub const NET_TIMEOUT_BACKGROUND: Duration = Duration::from_secs(60);

/// Nothing may ask for credentials in the background: no Git Credential Manager window, no
/// askpass program (GIT_ASKPASS set but empty also skips core.askPass and SSH_ASKPASS), no ssh
/// passphrase prompt.
pub fn never_prompt(cmd: &mut tokio::process::Command) {
    cmd.env("GCM_INTERACTIVE", "never")
        .env("GIT_ASKPASS", "")
        .env_remove("SSH_ASKPASS")
        .env("SSH_ASKPASS_REQUIRE", "never");
}

/// Runs a command that reaches a remote, killed past its time limit. A background one never
/// asks for credentials and dies with everything it started (credential helper, ssh…). The
/// user's may ask: Git Credential Manager can then start a browser, which must outlive the
/// command, so no job object for those.
pub async fn run_net(cwd: &str, args: &[&str], background: bool) -> Result<Vec<u8>> {
    run_net_command(command(cwd, args), args, background).await
}

/// `cmd` with git's messages in English, whatever the system's language: for the commands whose
/// refusals are told apart by their text.
pub fn in_english(mut cmd: tokio::process::Command) -> tokio::process::Command {
    cmd.env("LC_ALL", "C");
    cmd
}

/// `run_hooks`, with git's messages in English (`in_english`): switch and merge run hooks.
pub async fn run_english_hooks(cwd: &str, args: &[&str]) -> Result<Vec<u8>> {
    let out = output_within(in_english(command(cwd, args)), args, Limit::Hooks).await?;
    checked(out, args)
}

/// `run_net`, with git's messages in English (`in_english`).
pub async fn run_net_english(cwd: &str, args: &[&str], background: bool) -> Result<Vec<u8>> {
    run_net_command(in_english(command(cwd, args)), args, background).await
}

pub async fn run_net_command(
    mut cmd: tokio::process::Command,
    args: &[&str],
    background: bool,
) -> Result<Vec<u8>> {
    cmd.kill_on_drop(true);
    if background {
        never_prompt(&mut cmd);
        crate::job::isolate(&mut cmd);
    }
    let child = cmd.spawn()?;
    let job = if background {
        crate::job::Job::for_child(&child)
    } else {
        None
    };
    let limit = if background {
        NET_TIMEOUT_BACKGROUND
    } else {
        NET_TIMEOUT
    };
    match tokio::time::timeout(limit, crate::proc::wait_output(child)).await {
        Ok(out) => checked(out?, args),
        Err(_) => {
            if let Some(j) = &job {
                j.terminate();
            }
            Err(stopped(args, limit))
        }
    }
}

pub fn checked(out: std::process::Output, args: &[&str]) -> Result<Vec<u8>> {
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if err.is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            err
        };
        bail!(if msg.is_empty() {
            tr_in!(
                crate::i18n::ui(),
                "git {a} a échoué",
                "git {a} failed",
                a = args.join(" ")
            )
        } else {
            msg
        });
    }
    Ok(out.stdout)
}

pub async fn text(cwd: &str, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8_lossy(&run(cwd, args).await?)
        .trim()
        .to_string())
}

/// A file opened, or closed unwritten: nothing changed in it (the watchers of the repositories and of the plugins).
pub fn is_read(ev: &::notify::Event) -> bool {
    matches!(ev.kind, ::notify::EventKind::Access(_))
}
