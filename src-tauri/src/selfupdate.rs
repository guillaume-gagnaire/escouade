//! Escouade updates Claude Code itself (ESC-133). Every `claude` it launches has its own update
//! turned off (`Engine::no_self_update`): Escouade stops its processes with their whole tree, and an
//! update one of them had started (`npm install -g`) was cut half-way, the `claude` command gone
//! (a client's Mac, 2026-10-19). In its place, `claude update` at Escouade's start, then once a
//! day while it is open, in a process of its own: out of every job and process group of the app's,
//! its output in a file rather than a pipe, so that neither an agent stopped nor Escouade closed
//! cuts it. Not when « Mettre à jour Claude Code automatiquement » is off, nor when the user turned
//! Claude Code's own update off (`user_off`).

use crate::accounts;
use crate::engine::{self, Program};
use crate::model::Settings;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How often it is run while Escouade is open.
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);
/// How often the loop looks whether it is due (the switch turned on is told at once: `poke`).
const LOOK: Duration = Duration::from_secs(10 * 60);
/// How long after Escouade's start: the window and the agents first.
const AFTER_START: Duration = Duration::from_secs(5);

/// Where the last update stands, as « Réglages › Harness » shows it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    /// Not run yet since Escouade started.
    #[default]
    Waiting,
    Running,
    UpToDate,
    Updated,
    Failed,
    /// The switch of the settings is off.
    Off,
    /// The user turned Claude Code's own update off (`line` says how): Escouade leaves it too.
    UserOff,
    /// No `claude` found (`line` says what to do).
    Missing,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub state: State,
    /// The version it runs (the new one once updated).
    pub version: Option<String>,
    /// The version it was updated from.
    pub from: Option<String>,
    /// The error in a line, or why it is not run.
    pub line: Option<String>,
    /// When it ended, in milliseconds since the epoch.
    pub at: Option<i64>,
}

static LAST: Mutex<Option<Status>> = Mutex::new(None);
static RAN_AT: Mutex<Option<Instant>> = Mutex::new(None);
static POKE: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// Truthy as Claude Code reads its variables: `1`, `true`, `yes`, `on`.
fn truthy(v: &str) -> bool {
    ["1", "true", "yes", "on"].contains(&v.trim().to_lowercase().as_str())
}

/// How the user turned Claude Code's own update off, as Claude Code itself decides it (2.1.302):
/// `DISABLE_UPDATES` or `DISABLE_AUTOUPDATER` (in the environment, `var`, or in the `env` of its
/// `settings.json`), `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` set, or `autoUpdates: false` in its
/// `.claude.json`, which a native installation goes past when it wrote it itself
/// (`autoUpdatesProtectedForNative`). None: it is on.
pub fn user_off(
    var: &dyn Fn(&str) -> Option<String>,
    settings_json: Option<&Value>,
    claude_json: Option<&Value>,
) -> Option<String> {
    let set_in = |name: &str| -> Option<String> {
        var(name).or_else(|| {
            let v = settings_json?.get("env")?.get(name)?;
            Some(match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
        })
    };
    for name in ["DISABLE_UPDATES", "DISABLE_AUTOUPDATER"] {
        if set_in(name).is_some_and(|v| truthy(&v)) {
            return Some(name.to_string());
        }
    }
    const TRAFFIC: &str = "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC";
    if set_in(TRAFFIC).is_some_and(|v| !v.is_empty()) {
        return Some(TRAFFIC.to_string());
    }
    let c = claude_json?;
    let native = c.get("installMethod").and_then(Value::as_str) == Some("native");
    let protected = c.get("autoUpdatesProtectedForNative") == Some(&Value::Bool(true));
    (c.get("autoUpdates") == Some(&Value::Bool(false)) && !(native && protected))
        .then(|| "autoUpdates: false".to_string())
}

/// `user_off` for the `claude` a terminal runs (Principal's): the app's environment, its folder.
fn user_off_here(settings: &Settings) -> Option<String> {
    let principal = accounts::principal();
    let read = |p: PathBuf| -> Option<Value> {
        serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()
    };
    let settings_json = read(accounts::config_dir(&principal, settings).join("settings.json"));
    let claude_json = read(accounts::claude_json(&principal, settings));
    let var = |name: &str| -> Option<String> {
        if cfg!(test) {
            return None;
        }
        std::env::var(name).ok()
    };
    user_off(&var, settings_json.as_ref(), claude_json.as_ref())
}

/// What `claude update` said, in `out` (its output and its errors), and how it ended.
pub fn outcome(success: bool, code: Option<i32>, out: &str) -> Status {
    let lines: Vec<String> = out
        .lines()
        .map(|l| strip_ansi(l).trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    for l in &lines {
        // « Successfully updated from 2.1.300 to version 2.1.301 »
        if let Some(rest) = l.strip_prefix("Successfully updated from ") {
            if let Some((from, to)) = rest.split_once(" to version ") {
                return Status {
                    state: State::Updated,
                    from: Some(from.trim().to_string()),
                    version: Some(to.trim().trim_end_matches('.').to_string()),
                    ..Default::default()
                };
            }
        }
        // « Claude Code is up to date (2.1.301) »; Homebrew, WinGet: « Claude is up to date! ».
        if l.contains("is up to date") {
            let version = l
                .split_once('(')
                .and_then(|(_, r)| r.split_once(')'))
                .map(|(v, _)| v.trim().to_string());
            return Status {
                state: State::UpToDate,
                version,
                ..Default::default()
            };
        }
    }
    if success {
        return Status {
            state: State::UpToDate,
            ..Default::default()
        };
    }
    let line = lines
        .last()
        .map(|l| l.to_string())
        .unwrap_or_else(|| match code {
            Some(c) => format!("exit code {c}"),
            None => "stopped".to_string(),
        });
    Status {
        state: State::Failed,
        line: Some(line),
        ..Default::default()
    }
}

/// `line` without its colors (`ESC [ … letter`).
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// `launch` started apart from the app: out of its job when it is in one that lets it go (under
/// Windows), in a process group of its own (Unix), its output and errors in `out` (a pipe would
/// break once the app is gone), nothing on its input. Neither an agent stopped (its own job, its own
/// group) nor Escouade closed ends it.
pub fn spawn_apart(
    launch: &engine::Launch,
    cwd: &Path,
    out: &Path,
) -> std::io::Result<std::process::Child> {
    let file = std::fs::File::create(out)?;
    let mut cmd = std::process::Command::new(&launch.program);
    cmd.args(&launch.args)
        .envs(launch.env.iter().map(|(k, v)| (k, v)))
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(file);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        let flags = crate::proc::CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP;
        // A job that does not let it go refuses the start: then it stays in it.
        if let Ok(child) = cmd
            .creation_flags(flags | CREATE_BREAKAWAY_FROM_JOB)
            .spawn()
        {
            return Ok(child);
        }
        cmd.creation_flags(flags);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.spawn()
}

/// Where the last update stands, the settings and the user's own choice told as they are now.
pub fn status(settings: &Settings) -> Status {
    let last = LAST.lock().clone().unwrap_or_default();
    if last.state == State::Running {
        return last;
    }
    if !settings.claude_self_update {
        return Status {
            state: State::Off,
            ..last
        };
    }
    if let Some(how) = user_off_here(settings) {
        return Status {
            state: State::UserOff,
            line: Some(how),
            ..last
        };
    }
    last
}

fn set(status: Status) {
    *LAST.lock() = Some(status);
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// The switch turned on: the loop looks at once.
pub fn poke() {
    POKE.notify_one();
}

/// `claude update` once, unless off or a run is under way; `data` is Escouade's data folder (the
/// output's file). What it said goes to `app.log` and to `status`.
pub async fn run_once(settings: &Settings, data: &Path) {
    if !settings.claude_self_update {
        return;
    }
    if let Some(how) = user_off_here(settings) {
        log::info!("claude update: not run, Claude Code's own update is off ({how})");
        return;
    }
    {
        let mut last = LAST.lock();
        if last.as_ref().is_some_and(|s| s.state == State::Running) {
            return;
        }
        let before = last.clone().unwrap_or_default();
        *last = Some(Status {
            state: State::Running,
            ..before
        });
    }
    *RAN_AT.lock() = Some(Instant::now());
    let principal = accounts::principal();
    let program = {
        let (p, s) = (principal.clone(), settings.clone());
        tokio::task::spawn_blocking(move || accounts::program(&p, &s))
            .await
            .ok()
            .flatten()
    };
    let Some(program) = program else {
        let line = engine::claude_missing_line(crate::i18n::ui());
        log::warn!("claude update: not run, {line}");
        set(Status {
            state: State::Missing,
            line: Some(line),
            at: Some(now_ms()),
            ..Default::default()
        });
        return;
    };
    let status = update(&program, settings, data).await;
    match status.state {
        State::Updated => log::info!(
            "claude update: {} updated from {} to {}",
            program.describe(),
            status.from.as_deref().unwrap_or("?"),
            status.version.as_deref().unwrap_or("?")
        ),
        State::Failed => log::warn!(
            "claude update: {} failed: {}",
            program.describe(),
            status.line.as_deref().unwrap_or("")
        ),
        _ => log::info!(
            "claude update: {} up to date ({})",
            program.describe(),
            status.version.as_deref().unwrap_or("?")
        ),
    }
    set(status);
}

/// `claude update` by `program`, apart (`spawn_apart`), waited for: what it said.
async fn update(program: &Program, settings: &Settings, data: &Path) -> Status {
    // From the home folder, as the user would (the tests' is none: the temporary folder).
    let (_, home) = accounts::app_env();
    let home = if home.is_dir() {
        home
    } else {
        std::env::temp_dir()
    };
    // Its network settings; none of the variables that turn its update off.
    let env = settings.claude_env();
    let args = vec!["update".to_string()];
    let out = data.join("claude-update.log");
    let failed = |line: String| Status {
        state: State::Failed,
        line: Some(line),
        at: Some(now_ms()),
        ..Default::default()
    };
    let launch = match program.launch(&home.to_string_lossy(), &args, &env) {
        Ok(l) => l,
        Err(e) => return failed(e.to_string()),
    };
    let child = match spawn_apart(&launch, &home, &out) {
        Ok(c) => c,
        Err(e) => return failed(e.to_string()),
    };
    log::info!(
        "claude update: started by {} (pid {})",
        program.describe(),
        child.id()
    );
    let mut child = child;
    let ended = tokio::task::spawn_blocking(move || child.wait()).await;
    let said = std::fs::read(&out)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let mut status = match ended {
        Ok(Ok(exit)) => outcome(exit.success(), exit.code(), &said),
        Ok(Err(e)) => failed(e.to_string()),
        Err(e) => failed(e.to_string()),
    };
    // A new version, or one it did not tell: read again.
    engine::registry::forget();
    if status.version.is_none() && status.state == State::UpToDate {
        status.version =
            engine::registry::version(engine::by_id(crate::model::EngineId::CLAUDE), program).await;
    }
    status.at = Some(now_ms());
    status
}

/// At Escouade's start, then once a day while it is open (and at once when the switch is turned
/// on): `run_once`.
pub fn watch(core: Arc<crate::core::Core>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(AFTER_START).await;
        loop {
            let due = RAN_AT.lock().is_none_or(|at| at.elapsed() >= EVERY);
            if due {
                let settings = core.settings.read().clone();
                run_once(&settings, core.data.root()).await;
            }
            let _ = tokio::time::timeout(LOOK, POKE.notified()).await;
        }
    });
}

#[cfg(test)]
#[path = "selfupdate_tests.rs"]
mod tests;
