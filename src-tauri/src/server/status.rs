//! What a running `escouade-server` says of itself, for the commands beside it (ESC-515): `server-status.json` in its
//! data folder, written again every [`EVERY`] and when what it does changes, removed at its clean stop. `health` (the
//! image's `HEALTHCHECK`) reads that it is fresh and its process alive; `update` reads its version and whether a restart
//! would cut someone's work (`updates::busy`, as the app's automatic restart: a turn, a question, a background task, a
//! terminal used lately…), to wait for it.
//!
//! It holds no secret: a process id, versions, an address of the loopback or the one `--listen` asked for, a reason.

use crate::paths::DataDir;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// The file, in the data folder.
pub const FILE: &str = "server-status.json";
/// How often it is written again, whatever happens.
pub const EVERY: Duration = Duration::from_secs(5);
/// Older than this, the server is taken for stuck (`health`).
pub const STALE_MS: i64 = 30_000;

/// What runs the server, as far as it can tell: a restart after `update` comes from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Supervisor {
    /// A systemd service (it sets `INVOCATION_ID`): `Restart=always` starts it again.
    Systemd,
    /// The image of Escouade (`ESCOUADE_SERVER_IMAGE`): it is updated by a new image, never in place.
    Docker,
}

/// The image of Escouade sets it (`deploy/Dockerfile`): the server knows it runs in its container.
pub const IMAGE_VAR: &str = "ESCOUADE_SERVER_IMAGE";

/// What supervises this process.
pub fn supervisor() -> Option<Supervisor> {
    supervisor_of(|name| std::env::var(name).ok())
}

/// The same, from the variables `var` gives.
pub fn supervisor_of(var: impl Fn(&str) -> Option<String>) -> Option<Supervisor> {
    let set = |name: &str| var(name).is_some_and(|v| !v.trim().is_empty());
    if set(IMAGE_VAR) {
        Some(Supervisor::Docker)
    } else if set("INVOCATION_ID") {
        Some(Supervisor::Systemd)
    } else {
        None
    }
}

/// What the server says of itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub pid: u32,
    pub version: String,
    /// Where its API listens.
    pub api: String,
    pub started_at: i64,
    /// When it was written (ms): a server stuck stops writing it.
    pub at: i64,
    /// Why a restart now would cut someone's work (`updates::Busy`, in words: `turn`, `question`…), none at rest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub busy: Option<String>,
    /// Agents in a turn.
    #[serde(default)]
    pub turns: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supervisor: Option<Supervisor>,
}

pub fn path(data: &DataDir) -> PathBuf {
    data.root().join(FILE)
}

/// Written whole, then put in place: a reader never sees half of it.
pub fn write(data: &DataDir, status: &Status) -> Result<()> {
    let file = path(data);
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(status)?)?;
    std::fs::rename(&tmp, &file)?;
    Ok(())
}

/// What the server last said, if it said anything.
pub fn read(data: &DataDir) -> Option<Status> {
    serde_json::from_slice(&std::fs::read(path(data)).ok()?).ok()
}

pub fn remove(data: &DataDir) {
    let _ = std::fs::remove_file(path(data));
}

/// The process `pid` is alive (and this user may signal it).
pub fn alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: signal 0 sends nothing; it only says whether the process exists.
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// The server behind `status` is running, and still writes it (at `now`).
pub fn healthy(
    status: &Status,
    now: i64,
    alive: impl Fn(u32) -> bool,
) -> std::result::Result<(), String> {
    if !alive(status.pid) {
        return Err(tr!(
            "le processus {} du serveur n’est plus là",
            "the server's process {} is gone",
            status.pid
        ));
    }
    let age = now - status.at;
    if age > STALE_MS {
        return Err(tr!(
            "le serveur n’a rien dit depuis {} s",
            "the server has said nothing for {} s",
            age / 1000
        ));
    }
    Ok(())
}

/// `updates::Busy` in a word, for the file.
pub fn busy_word(busy: crate::updates::Busy) -> &'static str {
    use crate::updates::Busy;
    match busy {
        Busy::Turn => "turn",
        Busy::Question => "question",
        Busy::Setup => "setup",
        Busy::Validation => "validation",
        Busy::Work => "work",
        Busy::Background => "background",
        Busy::Launch => "launch",
        Busy::Terminal => "terminal",
        Busy::Testing => "testing",
        Busy::Unsaved => "unsaved",
        Busy::Modal => "modal",
        Busy::Activity => "activity",
    }
}

/// The word of the file, as the user reads it.
pub fn busy_text(word: &str) -> String {
    match word {
        "turn" => tr!("un agent est en plein tour", "an agent is in a turn"),
        "question" => tr!(
            "un agent attend une réponse",
            "an agent waits for an answer"
        ),
        "setup" => tr!(
            "le worktree d’un agent se prépare",
            "an agent's worktree is being set up"
        ),
        "validation" => tr!("un ticket se valide", "a ticket is being validated"),
        "work" => tr!("un travail est en cours", "work is under way"),
        "background" => tr!(
            "une tâche de fond d’un agent tourne",
            "an agent's background task runs"
        ),
        "launch" => tr!("un lancement tourne", "a launch runs"),
        "terminal" => tr!(
            "un terminal a servi il y a peu",
            "a terminal was used lately"
        ),
        "unsaved" => tr!(
            "l’éditeur garde des fichiers non enregistrés",
            "the editor holds unsaved files"
        ),
        _ => tr!("le serveur est occupé", "the server is busy"),
    }
}
