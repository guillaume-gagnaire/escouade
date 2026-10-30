//! The one-shot questions (« textes rédigés »: an agent's name, a commit message, a pull request's
//! description, the commands of a worktree): asked of the harness the settings name, each one
//! within its limit and with its answer read up to `ANSWER_MAX`, whatever the harness.

use super::{Engine, OneShot, Program};
use crate::model::EngineId;
use anyhow::{anyhow, bail, Result};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

/// The most an answer may weigh as it is read: a process that says more is killed, and its answer
/// refused (a name or a commit message is a few lines; this leaves room for a description).
pub const ANSWER_MAX: usize = 1 << 20;

/// How long a question may take by default, its writing included (`Core::one_shot`).
pub const LIMIT: Duration = Duration::from_secs(90);

/// The harness the one-shot questions go to, among those found (`found`, in the registry's order):
/// the one the settings name (`chosen`) while it is found; else Claude Code when it is (its Haiku
/// writes them by default); else the first found. None when no harness is found.
pub fn writer(chosen: Option<EngineId>, found: &[EngineId]) -> Option<EngineId> {
    chosen
        .filter(|id| found.contains(id))
        .or_else(|| {
            found
                .contains(&EngineId::CLAUDE)
                .then_some(EngineId::CLAUDE)
        })
        .or_else(|| found.first().copied())
}

/// `engine`'s answer to `ask`, within `ask.limit` whatever it does: past it, its question is
/// dropped (its process killed with all it started) and the answer refused.
pub async fn ask(engine: &dyn Engine, ask: OneShot<'_>) -> Result<String> {
    let (limit, weight) = (ask.limit, ask.weight);
    let answer = tokio::time::timeout(limit, engine.one_shot(ask))
        .await
        .map_err(|_| {
            let who = engine.writer(weight);
            anyhow!(tr!(
                "pas de réponse de {who} en {s} s",
                "no answer from {who} in {s} s",
                s = limit.as_secs()
            ))
        })??;
    // An engine that read its process otherwise is held to the same bound.
    if answer.len() > ANSWER_MAX {
        bail!(too_long(engine.writer(weight)));
    }
    Ok(answer)
}

fn too_long(who: &str) -> String {
    tr!(
        "réponse de {who} trop longue (plus de {n} Mo)",
        "answer from {who} too long (more than {n} MB)",
        n = ANSWER_MAX >> 20
    )
}

/// What `reader` says, up to `max` bytes; None when it says more (the rest is not read).
pub async fn read_bounded(
    reader: impl AsyncRead + Unpin,
    max: usize,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut out = Vec::new();
    // One byte more than allowed tells an answer too long from one exactly as long.
    reader.take(max as u64 + 1).read_to_end(&mut out).await?;
    Ok((out.len() <= max).then_some(out))
}

/// What `program` writes on its standard output once given `prompt` on its standard input, run in
/// `cwd` with `args` and `env` (its standard error dropped), read up to `ANSWER_MAX`: a process
/// that says more is killed and its answer refused. `who` names it in that refusal. Whatever it
/// started ends with it, when this returns or is dropped (`ask`'s limit).
pub async fn run(
    program: &Program,
    cwd: &Path,
    args: &[String],
    env: &[(String, String)],
    prompt: &str,
    who: &str,
) -> Result<Vec<u8>> {
    let launch = program.launch(&cwd.to_string_lossy(), args, env)?;
    let mut cmd = tokio::process::Command::new(&launch.program);
    cmd.args(&launch.args)
        .current_dir(cwd)
        .envs(launch.env)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    crate::job::isolate(&mut cmd);
    let mut child = cmd.spawn()?;
    // Dropped when this returns: whatever the process left running ends with it.
    let _job = crate::job::Job::for_child(&child);
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;
    // Written while the answer is read: a process that answers before reading all its question
    // (or never reads it) does not hold it.
    let write = async move {
        if let Some(mut stdin) = stdin {
            // A process that stopped reading has its answer read all the same.
            let _ = stdin.write_all(prompt.as_bytes()).await;
            // Dropped here: the input ends, and the CLI answers.
        }
    };
    let (_, read) = tokio::join!(write, read_bounded(stdout, ANSWER_MAX));
    let Some(out) = read? else {
        let _ = child.kill().await;
        bail!(too_long(who));
    };
    child.wait().await?;
    Ok(out)
}

#[cfg(test)]
#[path = "oneshot_tests.rs"]
mod tests;
