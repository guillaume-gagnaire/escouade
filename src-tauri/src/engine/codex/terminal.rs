//! « Vue Codex »: Codex's own interface (`codex`, its TUI) in a terminal (`TerminalView`), as the
//! real run of Codex 0.163.0 found it (docs/TODO-1.8.md, point 0, « Vue terminal »).
//!
//! - Its hooks are commands (`escouade-hook`), declared for this process alone by `-c
//!   hooks.<Event>=…`, never in the user's `config.toml`. Codex runs none it was not told to
//!   trust: their trust is given for this process too (`-c hooks.state=…`), with the keys and the
//!   hashes that `hooks/list` of an app-server started with the same hooks gives (`prepare`). The
//!   hooks already trusted of the user's are trusted again with them.
//! - Its log is the session's rollout (`sessions/YYYY/MM/DD/rollout-<date>-<session>.jsonl`),
//!   translated by `AgentRt::codex_rollout`; its hooks by `AgentRt::codex_hook`.
//! - It names its own sessions: a new one is told by its first hook (`SessionStart`, which comes
//!   with the first message).
//! - A session is resumed by `codex resume <id>`, a copy by `codex fork <id>`, in the same
//!   `CODEX_HOME` as the app-server of Escouade's view (tried both ways).
//! - On a project it does not know yet, it asks whether to trust the folder before anything else:
//!   Escouade never answers for the user, and pastes no message meanwhile (`paste_refusal`).

use super::super::terminal::{bracketed, TerminalArgs, TerminalLaunch, TerminalView};
use super::super::{mcp, BoxFuture, Engine, Escouade, Program};
use super::{declares_escouade, escouade_launch, CODEX};
use crate::agent::{AgentRt, Effects, HookOutcome};
use crate::tui::Registered;
use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

/// The hooks Escouade listens to.
pub(crate) const EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "Stop",
    "Interrupt",
];

/// What Codex says, before it quits, of a session to resume that its folder does not have
/// (« No saved session found with ID <id>. », tried on 0.163.0).
const NO_SESSION: &str = "No saved session found";

/// The types of the images Codex takes as a path pasted in it.
const IMAGE_TYPES: [&str; 2] = ["image/png", "image/jpeg"];

/// How long the app-server asked for the trust of the hooks may take.
const TRUST_LIMIT: Duration = Duration::from_secs(20);

/// Codex's interface.
pub struct CodexTerminal;

pub static CODEX_TERMINAL: CodexTerminal = CodexTerminal;

/// A text as a TOML string (a JSON string is one).
fn toml_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// The hooks of an interface: `escouade-hook` for each event Escouade listens to, as options for
/// this process alone. Its path quoted when it has a space (Codex runs it through a shell).
pub(crate) fn hook_args(command: &Path) -> Vec<String> {
    let path = command.to_string_lossy();
    let line = if path.contains(' ') {
        format!("\"{path}\"")
    } else {
        path.into_owned()
    };
    EVENTS
        .iter()
        .flat_map(|e| {
            [
                "-c".to_string(),
                format!(
                    "hooks.{e}=[{{hooks=[{{type=\"command\",command={}}}]}}]",
                    toml_string(&line)
                ),
            ]
        })
        .collect()
}

/// The hooks' options among `args` (`hook_args`).
fn hooks_among(args: &[String]) -> Vec<String> {
    args.windows(2)
        .filter(|w| w[0] == "-c" && w[1].starts_with("hooks."))
        .flat_map(|w| w.to_vec())
        .collect()
}

/// The option that trusts the hooks `hooks/list` lists (its answer `list`): those of this process
/// (`sessionFlags`), and those the user trusts already. None when there is none.
pub(crate) fn trust_args(list: &Value) -> Option<Vec<String>> {
    let entries: Vec<String> = list["data"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|d| d["hooks"].as_array().into_iter().flatten())
        .filter(|h| h["source"] == "sessionFlags" || h["trustStatus"] == "trusted")
        .filter_map(|h| {
            let (key, hash) = (h["key"].as_str()?, h["currentHash"].as_str()?);
            Some(format!(
                "{}={{trusted_hash={}}}",
                toml_string(key),
                toml_string(hash)
            ))
        })
        .collect();
    (!entries.is_empty()).then(|| {
        vec![
            "-c".to_string(),
            format!("hooks.state={{{}}}", entries.join(",")),
        ]
    })
}

/// Asks an app-server started with the hooks `hooks` (in `cwd`, with `env`) how they are listed:
/// their keys and hashes (`hooks/list`).
pub(crate) async fn list_hooks(
    program: &Program,
    cwd: &str,
    hooks: &[String],
    env: &[(String, String)],
) -> Result<Value> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let mut args = vec!["app-server".to_string()];
    args.extend(hooks.iter().cloned());
    let launch = program.launch(cwd, &args, env)?;
    let mut cmd = tokio::process::Command::new(&launch.program);
    cmd.args(&launch.args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    for (k, v) in &launch.env {
        cmd.env(k, v);
    }
    #[cfg(windows)]
    cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
    crate::job::isolate(&mut cmd);
    crate::proc::no_inherited_fds(&mut cmd);
    let mut child = cmd.spawn().context("codex app-server")?;
    let job = crate::job::Job::for_child(&child);
    let mut stdin = child.stdin.take().context("stdin")?;
    let stdout = child.stdout.take().context("stdout")?;
    let asked = [
        json!({ "id": 1, "method": "initialize", "params": {
            "clientInfo": { "name": "escouade", "title": "Escouade", "version": env!("CARGO_PKG_VERSION") },
            "capabilities": {},
        } }),
        json!({ "method": "initialized" }),
        json!({ "id": 2, "method": "hooks/list", "params": { "cwds": [cwd] } }),
    ];
    for msg in asked {
        stdin.write_all(format!("{msg}\n").as_bytes()).await?;
    }
    stdin.flush().await?;
    let read = async {
        let mut lines = BufReader::new(stdout).lines();
        while let Some(line) = lines.next_line().await? {
            let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if msg["id"] == 2 {
                if let Some(e) = msg.get("error") {
                    bail!("hooks/list: {e}");
                }
                return Ok(msg["result"].clone());
            }
        }
        Err(anyhow!("codex app-server ended before it listed its hooks"))
    };
    let answer = tokio::time::timeout(TRUST_LIMIT, read)
        .await
        .map_err(|_| anyhow!("codex app-server did not list its hooks in time"))?;
    drop(stdin);
    let _ = child.start_kill();
    let _ = child.wait().await;
    drop(job);
    answer
}

/// The arguments of an agent's interface: its session resumed (`resume <id>`) or copied (`fork
/// <id>`), or a new one; its model, effort and mode; its instructions for a new session (a resumed
/// one keeps those it had, tried); the plan tool, as its app-server has it.
pub(crate) fn tui_args(l: &TerminalLaunch) -> Vec<String> {
    let m = l.meta;
    let mut a: Vec<String> = Vec::new();
    let new = match (&m.session_id, &m.fork_of) {
        (Some(s), _) => {
            a.extend(["resume".into(), s.clone()]);
            false
        }
        (None, Some(f)) => {
            a.extend(["fork".into(), f.clone()]);
            false
        }
        (None, None) => true,
    };
    // Its own process, never the shared one of a daemon: its options and hooks are its own.
    a.push("--no-daemon".into());
    if !m.model.is_empty() {
        a.extend(["-m".into(), m.model.clone()]);
    }
    if !m.effort.is_empty() && CODEX.supports_effort(&m.model) {
        a.extend([
            "-c".into(),
            format!("model_reasoning_effort={}", toml_string(&m.effort)),
        ]);
    }
    // « Projet privé »: Escouade's profile of its mode, in place of its sandbox (`-s`, which
    // Codex would take over the profile).
    let fence = crate::fence::spelled(l.fence, l.program);
    // A discussion (ESC-261): read only, whatever its mode.
    let sandboxed = super::sandboxed(&m.mode, m.discussion.is_some());
    let profile = (!fence.is_empty())
        .then(|| super::fence_profile(sandboxed))
        .flatten();
    match (sandboxed, profile) {
        ("auto", _) => a.push("--approve-for-me".into()),
        ("bypassPermissions", _) => a.push("--dangerously-bypass-approvals-and-sandbox".into()),
        (_, Some(_)) => a.extend(["-a".into(), "on-request".into()]),
        (mode, None) => a.extend([
            "-a".into(),
            "on-request".into(),
            "-s".into(),
            super::sandbox_mode(mode).into(),
        ]),
    }
    if let Some(profile) = profile {
        a.extend(super::fence_args(&fence, profile));
    }
    a.extend(["-c".into(), "tools.update_plan.enabled=true".into()]);
    if let Some(p) = m
        .append_prompt
        .as_deref()
        .filter(|p| new && !p.trim().is_empty())
    {
        let one_line = p.split_whitespace().collect::<Vec<_>>().join(" ");
        a.extend([
            "-c".into(),
            format!("developer_instructions={}", toml_string(&one_line)),
        ]);
    }
    if let Some(command) = l.hooks.command {
        a.extend(hook_args(command));
    }
    a
}

/// The rollout of `session` in Codex's folder `config_dir`: `sessions/<y>/<m>/<d>/rollout-<date>-
/// <session>.jsonl`, the one written last if several have it.
pub fn find_rollout(config_dir: &Path, session: &str) -> Option<PathBuf> {
    let suffix = format!("-{session}.jsonl");
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    let dirs = |p: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(p)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect()
    };
    for year in dirs(&config_dir.join("sessions")) {
        for month in dirs(&year) {
            for day in dirs(&month) {
                for file in std::fs::read_dir(&day).into_iter().flatten().flatten() {
                    let name = file.file_name().to_string_lossy().into_owned();
                    if name.starts_with("rollout-") && name.ends_with(&suffix) {
                        let at = file.metadata().and_then(|m| m.modified()).ok();
                        found.push((at.unwrap_or(std::time::UNIX_EPOCH), file.path()));
                    }
                }
            }
        }
    }
    found.into_iter().max_by_key(|(at, _)| *at).map(|(_, p)| p)
}

/// A folder as Codex's `config.toml` names it, to compare: forward slashes, no trailing one, and
/// (Windows) no case.
fn folder_key(p: &str) -> String {
    let s = p
        .trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string();
    let s = s.strip_prefix("//?/").unwrap_or(&s).to_string();
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

/// The folders a `config.toml` of Codex has a trust level for (`[projects.'<folder>']` with a
/// `trust_level`, trusted or not: Codex asks for neither again).
pub(crate) fn trusted_folders(config: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut table: Option<String> = None;
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            table = line
                .strip_prefix("[projects.")
                .and_then(|r| r.strip_suffix(']'))
                .map(|r| r.trim().trim_matches(['\'', '"']).replace("\\\\", "\\"));
            continue;
        }
        if let Some(t) = &table {
            if line.split('=').next().map(str::trim) == Some("trust_level") {
                out.push(folder_key(t));
            }
        }
    }
    out
}

/// The repository root of the folder `cwd` (that of the project for a worktree), as Codex asks
/// the trust of it.
pub(crate) fn repo_root(cwd: &Path) -> Option<PathBuf> {
    let mut dir = Some(cwd);
    while let Some(d) = dir {
        let git = d.join(".git");
        if git.is_dir() {
            return Some(d.to_path_buf());
        }
        if git.is_file() {
            let text = std::fs::read_to_string(&git).ok()?;
            let target = text
                .trim()
                .strip_prefix("gitdir:")?
                .trim()
                .replace('\\', "/");
            return match target.split_once("/.git/worktrees/") {
                Some((root, _)) => Some(PathBuf::from(root)),
                None => Some(d.to_path_buf()),
            };
        }
        dir = d.parent();
    }
    None
}

/// Whether Codex, in `cwd` with the folder `config_dir`, opens on its question about the trust of
/// the folder: no trust level for it, its repository's root, nor a folder above it.
pub(crate) fn asks_trust(config_dir: &Path, cwd: &str) -> bool {
    let config = std::fs::read_to_string(config_dir.join("config.toml")).unwrap_or_default();
    let known = trusted_folders(&config);
    let cwd_path = Path::new(cwd);
    let mut candidates = vec![folder_key(cwd)];
    candidates.extend(repo_root(cwd_path).map(|r| folder_key(&r.to_string_lossy())));
    !candidates.iter().any(|c| {
        known
            .iter()
            .any(|k| c == k || c.starts_with(&format!("{k}/")))
    })
}

impl TerminalView for CodexTerminal {
    fn shell(&self) -> &'static str {
        "codex"
    }

    fn launch(&self, l: &TerminalLaunch) -> TerminalArgs {
        TerminalArgs {
            args: tui_args(l),
            file: None,
            // `escouade-hook` posts to the hooks' server at this address.
            env: vec![(l.hooks.url_var.to_string(), l.hooks.url.to_string())],
            files: Vec::new(),
        }
    }

    /// Refused, an entry of the user's is turned off; with none, nothing is passed (an entry of
    /// `enabled` alone keeps Codex from starting, as for its app-server).
    fn escouade(&self, _harness: &dyn Engine, access: &Escouade, config_dir: &Path) -> mcp::Launch {
        let declared = || {
            std::fs::read_to_string(config_dir.join("config.toml"))
                .is_ok_and(|text| declares_escouade(&text))
        };
        if matches!(access, Escouade::Denied { .. }) && !declared() {
            return mcp::Launch::default();
        }
        escouade_launch(access)
    }

    fn prepare<'a>(
        &'a self,
        program: &'a Program,
        cwd: &'a str,
        args: &'a [String],
        env: &'a [(String, String)],
    ) -> BoxFuture<'a, Result<Vec<String>>> {
        Box::pin(async move {
            let hooks = hooks_among(args);
            if hooks.is_empty() {
                bail!(tr!(
                    "escouade-hook est introuvable à côté de l’app",
                    "escouade-hook is not found beside the app"
                ));
            }
            let list = list_hooks(program, cwd, &hooks, env).await?;
            trust_args(&list).ok_or_else(|| anyhow!("hooks/list: {list}"))
        })
    }

    /// Codex reads any file without asking: nothing to add.
    fn read_dir_args(&self, _dir: &Path) -> Vec<String> {
        Vec::new()
    }

    fn names_its_session(&self) -> bool {
        false
    }

    fn on_hook(&self, rt: &mut AgentRt, hook: &Value, fx: &mut Effects) -> HookOutcome {
        rt.codex_hook(hook, fx)
    }

    fn running_dir(&self, _config_dir: &Path) -> Option<PathBuf> {
        None
    }

    fn find_running(&self, _: &Path, _: &str, _: Option<&Path>) -> Option<Registered> {
        None
    }

    fn find_log(&self, config_dir: &Path, session: &str) -> Option<PathBuf> {
        find_rollout(config_dir, session)
    }

    fn on_log_entry(&self, rt: &mut AgentRt, entry: &Value, fx: &mut Effects) {
        rt.view_line(entry, fx);
    }

    fn says_session_lost(&self, output: &[u8]) -> bool {
        String::from_utf8_lossy(output).contains(NO_SESSION)
    }

    /// Its rollout, at the same place in the other folder.
    fn copy_session(&self, from: &Path, to: &Path, session: &str) -> Result<()> {
        let file = find_rollout(from, session)
            .ok_or_else(|| anyhow!("no rollout of {session} in {}", from.display()))?;
        let rel = file.strip_prefix(from)?;
        let target = to.join(rel);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(&file, &target)?;
        Ok(())
    }

    fn paste_refusal(&self, config_dir: &Path, cwd: &str, _spoke: bool) -> Option<String> {
        asks_trust(config_dir, cwd).then(|| {
            tr!(
                "Codex demande si tu fais confiance à ce dossier : réponds dans son terminal, puis renvoie ton message.",
                "Codex asks whether you trust this folder: answer in its terminal, then send your message again."
            )
        })
    }

    fn paste(&self, text: &str) -> Vec<u8> {
        bracketed(text)
    }

    fn submit(&self) -> &'static [u8] {
        b"\r"
    }

    fn image_types(&self) -> &'static [&'static str] {
        &IMAGE_TYPES
    }

    /// Escape (« esc to interrupt »), which its `Interrupt` hook confirms.
    fn interrupt_key(&self) -> &'static [u8] {
        b"\x1b"
    }

    fn quit_key(&self) -> &'static [u8] {
        b"\x03"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trust_of_the_hooks_is_their_keys_and_hashes_and_the_users_trusted_ones() {
        let list = json!({ "data": [{ "cwd": "/w", "hooks": [
            { "key": "C:\\<session-flags>\\config.toml:stop:0:0", "currentHash": "sha256:aa",
              "source": "sessionFlags", "trustStatus": "untrusted" },
            { "key": "/home/u/.codex/config.toml:stop:0:0", "currentHash": "sha256:bb",
              "source": "user", "trustStatus": "trusted" },
            { "key": "/home/u/.codex/config.toml:pre_tool_use:0:0", "currentHash": "sha256:cc",
              "source": "user", "trustStatus": "untrusted" },
        ] }] });
        assert_eq!(
            trust_args(&list).unwrap(),
            [
                "-c".to_string(),
                r#"hooks.state={"C:\\<session-flags>\\config.toml:stop:0:0"={trusted_hash="sha256:aa"},"/home/u/.codex/config.toml:stop:0:0"={trusted_hash="sha256:bb"}}"#.to_string(),
            ]
        );
        assert_eq!(trust_args(&json!({ "data": [] })), None);
    }

    #[test]
    fn each_hook_runs_escouade_hook_quoted_when_its_path_has_a_space() {
        let args = hook_args(Path::new("C:/Program Files/Escouade/escouade-hook.exe"));
        assert_eq!(args.len(), EVENTS.len() * 2);
        assert_eq!(args[0], "-c");
        assert_eq!(
            args[1],
            r#"hooks.SessionStart=[{hooks=[{type="command",command="\"C:/Program Files/Escouade/escouade-hook.exe\""}]}]"#
        );
        assert_eq!(hooks_among(&args), args);
        let plain = hook_args(Path::new("/usr/bin/escouade-hook"));
        assert!(plain[1].contains(r#"command="/usr/bin/escouade-hook""#));
    }

    #[test]
    fn a_folder_known_to_codex_is_not_asked_about_again() {
        let config = "model = \"x\"\n[projects.'C:\\Users\\u\\proj']\ntrust_level = \"trusted\"\n\n[projects.\"/home/u/other\"]\ntrust_level = \"untrusted\"\n[mcp_servers.x]\nurl = \"y\"\n";
        let known = trusted_folders(config);
        assert_eq!(known.len(), 2, "{known:?}");
        assert!(known.contains(&folder_key("C:/Users/u/proj")));
        assert!(known.contains(&folder_key("/home/u/other")));
    }

    #[test]
    fn the_root_of_a_worktree_is_its_projects() {
        let dir =
            std::env::temp_dir().join(format!("escouade-codex-root-{}", crate::model::new_id()));
        let wt = dir.join("wt");
        std::fs::create_dir_all(dir.join("proj").join(".git")).unwrap();
        std::fs::create_dir_all(&wt).unwrap();
        let gitdir = dir.join("proj").join(".git").join("worktrees").join("wt");
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();
        let root = repo_root(&wt).unwrap();
        assert_eq!(
            folder_key(&root.to_string_lossy()),
            folder_key(&dir.join("proj").to_string_lossy())
        );
        assert_eq!(
            folder_key(&repo_root(&dir.join("proj")).unwrap().to_string_lossy()),
            folder_key(&dir.join("proj").to_string_lossy())
        );
        // Codex asks about it until its config has the project.
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        assert!(asks_trust(&home, &wt.to_string_lossy()));
        std::fs::write(
            home.join("config.toml"),
            format!(
                "[projects.'{}']\ntrust_level = \"trusted\"\n",
                dir.join("proj").display()
            ),
        )
        .unwrap();
        assert!(!asks_trust(&home, &wt.to_string_lossy()));
    }

    #[test]
    fn the_rollout_of_a_session_is_found_in_its_dated_folders() {
        let home = std::env::temp_dir().join(format!(
            "escouade-codex-rollouts-{}",
            crate::model::new_id()
        ));
        let day = home.join("sessions").join("2026").join("10").join("17");
        std::fs::create_dir_all(&day).unwrap();
        let file = day.join("rollout-2026-10-17T10-00-00-s1.jsonl");
        std::fs::write(&file, "").unwrap();
        std::fs::write(day.join("rollout-2026-10-17T10-00-00-s2.jsonl"), "").unwrap();
        assert_eq!(find_rollout(&home, "s1"), Some(file.clone()));
        assert_eq!(find_rollout(&home, "s3"), None);
        // Copied into another account's folder, at the same place.
        let other = home.join("other");
        CODEX_TERMINAL.copy_session(&home, &other, "s1").unwrap();
        assert!(find_rollout(&other, "s1").is_some());
    }
}
