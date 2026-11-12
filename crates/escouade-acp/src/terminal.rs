//! The launch of the interface of a harness on this client in a terminal (`terminal/launch`): Cursor CLI's `agent`,
//! Mistral Vibe's `vibe`, Gemini CLI's `gemini`, by the extension its manifest names. The core checks it, writes its
//! files and keeps the PTY; what it writes in the agent's home for its agents (`BuiltinHarness::prepare`: Vibe's trust and
//! the refusals of a private project, Gemini's settings and rules) it writes for the interface too, before this launch
//! is asked, which reads them.

use escouade_plugin_protocol::engine::{
    EnvVar, FileToWrite, HarnessDecl, Launch, TerminalLaunch, TerminalParams,
};
use serde_json::{json, Value};
use std::path::Path;

/// How long a hook may run: in seconds for Vibe, in milliseconds for Gemini CLI (its unit).
const HOOK_LIMIT_SECS: u64 = 10;
const HOOK_LIMIT_MS: u64 = 10_000;

/// The events Gemini CLI's hooks tell Escouade.
pub const GEMINI_EVENTS: [&str; 6] = [
    "SessionStart",
    "BeforeAgent",
    "BeforeTool",
    "AfterTool",
    "Notification",
    "AfterAgent",
];

/// The variable that holds the token of Escouade's server for Vibe, named by its `api_key_env`.
pub const VIBE_TOKEN_VAR: &str = "ESCOUADE_MCP_TOKEN";

/// The hooks Escouade writes in the agent's `VIBE_HOME` for its interface.
pub const VIBE_HOOKS: &str = "hooks.toml";

/// The variables of Git Bash that would make Cursor take its shell (`if (!process.env.MSYSTEM)`): empty for it.
pub const GIT_BASH_VARS: [&str; 2] = ["MSYSTEM", "EXEPATH"];

/// Whether `path` is one of Windows: a drive, or a backslash.
fn windows_path(path: &str) -> bool {
    path.contains('\\') || path.as_bytes().get(1) == Some(&b':')
}

/// `dir` and `name`, by the separator `dir` is written with.
fn joined(dir: &str, name: &str) -> String {
    let sep = if dir.contains('\\') { "\\" } else { "/" };
    format!(
        "{}{sep}{}",
        dir.trim_end_matches(['/', '\\']),
        name.replace('/', sep)
    )
}

/// The variables of a launch.
fn vars(pairs: Vec<(String, String)>) -> Vec<EnvVar> {
    pairs
        .into_iter()
        .map(|(name, value)| EnvVar { name, value })
        .collect()
}

/// The `PATH` with the folder of `escouade-hook` first, under Windows, where a hook runs it by its name.
fn hook_path(p: &TerminalParams) -> Option<(String, String)> {
    let command = p.hook_command.as_deref().filter(|c| windows_path(c))?;
    let dir = &command[..command.rfind(['\\', '/'])?];
    let path = p.path.as_deref().unwrap_or_default();
    Some((
        "PATH".into(),
        match path.is_empty() {
            true => dir.to_string(),
            false => format!("{dir};{path}"),
        },
    ))
}

/// The line a hook runs: `escouade-hook` by its path quoted for a POSIX shell, or under Windows by its name (its folder
/// first on the `PATH`).
fn hook_line(command: &str, event: Option<&str>) -> String {
    let run = match windows_path(command) {
        true => "escouade-hook".to_string(),
        false => format!("'{}'", command.replace('\'', r"'\''")),
    };
    match event {
        Some(e) => format!("{run} {e}"),
        None => run,
    }
}

/// Its interface's launch, by the extension of its harness `own` (`cursor`, `vibe`, `gemini`); None for a harness with
/// no interface of its own here.
pub fn launch(own: &HarnessDecl, p: &TerminalParams) -> Option<TerminalLaunch> {
    let ext = own.acp.as_ref()?.extensions.first()?.clone();
    let launch = match ext.as_str() {
        "cursor" => cursor(p),
        "vibe" => vibe(p),
        "gemini" => gemini(p),
        _ => return None,
    };
    Some(TerminalLaunch {
        launch: Launch {
            read_only: p.read_only,
            ..launch
        },
        ..TerminalLaunch::default()
    })
}

/// Cursor CLI's `agent`: its session resumed (it opens any it is given), its model by its id without its options, Plan
/// and « Run Everything » of its own modes. Its fence and its rules are those its agents' client holds in its home.
pub fn cursor(p: &TerminalParams) -> Launch {
    let mut a: Vec<String> = vec!["--disable-auto-update".into()];
    if let Some(s) = p.session.as_deref().or(p.new_session.as_deref()) {
        a.extend(["--resume".into(), s.to_string()]);
    }
    let model = match p.model.as_str() {
        "" => None,
        crate::cursor::AUTO => Some("auto".to_string()),
        m => Some(m.split('[').next().unwrap_or(m).to_string()),
    };
    if let Some(model) = model {
        a.extend(["--model".into(), model]);
    }
    match p.mode.as_str() {
        "plan" => a.extend(["--mode".into(), "plan".into()]),
        "bypassPermissions" => a.push("--force".into()),
        _ => {}
    }
    Launch {
        args: a,
        env: vars(
            GIT_BASH_VARS
                .iter()
                .map(|v| (v.to_string(), String::new()))
                .collect(),
        ),
        fence_by_client: true,
        ..Launch::default()
    }
}

/// Vibe's agent for Escouade's `mode`: no Plan, which asks as « Demander »; never `auto-approve` with a private project's
/// paths refused, which passes over them.
fn vibe_agent(mode: &str, fenced: bool) -> String {
    let agent = match mode {
        "acceptEdits" => "accept-edits",
        "bypassPermissions" => "auto-approve",
        _ => "ask",
    };
    crate::vibe::agent_of(agent, fenced)
}

/// Its `hooks.toml`: `escouade-hook` before and after each tool, and at the end of each turn.
pub fn vibe_hooks(command: &str) -> String {
    // A JSON string is a basic string of TOML.
    let line = serde_json::to_string(&hook_line(command, None)).unwrap_or_default();
    let mut out = String::from(
        "# Escouade: the hooks of the agent's interface (« Vue Vibe »), taken away when it is gone.\n",
    );
    for kind in ["pre_tool", "post_tool", "post_agent"] {
        out.push_str(&format!(
            "\n[[hooks]]\nname = \"escouade-{}\"\ntype = \"{kind}\"\ncommand = {line}\ntimeout = {HOOK_LIMIT_SECS}\n",
            kind.replace('_', "-"),
        ));
    }
    out
}

/// Mistral Vibe's `vibe`: its session resumed when its store has it, its agent, the folders it reads; its model and
/// Escouade's server by its variables, its hooks in the agent's `VIBE_HOME`. Its trust and fence are the core's
/// (`prepare`), in the same home.
pub fn vibe(p: &TerminalParams) -> Launch {
    let fenced = !p.fence.is_empty();
    let mut a: Vec<String> = Vec::new();
    if let Some(s) = p.session.as_deref().filter(|_| p.resumes) {
        a.extend(["--resume".into(), s.to_string()]);
    }
    a.extend(["--agent".into(), vibe_agent(&p.mode, fenced)]);
    a.extend(p.read_dirs.iter().map(|d| format!("--add-dir={d}")));
    let mut env = vec![
        ("VIBE_ENABLE_TELEMETRY".to_string(), "false".to_string()),
        (p.hook_url_var.clone(), p.hook_url.clone()),
    ];
    if !p.model.is_empty() {
        env.push(("VIBE_ACTIVE_MODEL".into(), p.model.clone()));
    }
    if let Some(g) = &p.escouade {
        let servers = json!([{
            "name": crate::SERVER, "transport": "http", "url": g.url,
            "api_key_env": VIBE_TOKEN_VAR,
        }]);
        env.push(("VIBE_MCP_SERVERS".into(), servers.to_string()));
        env.push((VIBE_TOKEN_VAR.into(), g.token.clone()));
    }
    let mut files = Vec::new();
    if let (Some(home), Some(command)) = (&p.home, &p.hook_command) {
        files.push(FileToWrite {
            path: joined(home, VIBE_HOOKS),
            text: vibe_hooks(command),
        });
        env.extend(hook_path(p));
    }
    Launch {
        args: a,
        env: vars(env),
        files,
        ..Launch::default()
    }
}

/// Its settings `before` with what its interface needs of Escouade: its hooks (`command`, none without it), Escouade's
/// server (`escouade`: its address and token) or none, the variables of its hooks kept from its redaction.
pub fn gemini_settings(
    before: &str,
    command: Option<&str>,
    escouade: Option<(&str, &str)>,
    hook_vars: &[&str],
) -> Value {
    let mut doc: Value = serde_json::from_str(before)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    if let Some(command) = command {
        let mut hooks = json!({});
        for event in GEMINI_EVENTS {
            hooks[event] = json!([{ "hooks": [{
                "name": "escouade", "type": "command", "command": hook_line(command, Some(event)),
                "timeout": HOOK_LIMIT_MS,
            }] }]);
        }
        doc["hooks"] = hooks;
        doc["hooksConfig"] = json!({ "enabled": true });
        if !doc["security"].is_object() {
            doc["security"] = json!({});
        }
        if !doc["security"]["environmentVariableRedaction"].is_object() {
            doc["security"]["environmentVariableRedaction"] = json!({});
        }
        let mut allowed: Vec<Value> = doc["security"]["environmentVariableRedaction"]["allowed"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for v in hook_vars {
            if !allowed.iter().any(|a| a == v) {
                allowed.push(json!(v));
            }
        }
        doc["security"]["environmentVariableRedaction"]["allowed"] = json!(allowed);
    }
    if !doc["mcpServers"].is_object() {
        doc["mcpServers"] = json!({});
    }
    match escouade {
        Some((url, token)) => {
            doc["mcpServers"][crate::SERVER] = json!({
                "httpUrl": url, "headers": { "Authorization": format!("Bearer {token}") },
                "trust": false,
            });
        }
        None => {
            if let Some(o) = doc["mcpServers"].as_object_mut() {
                o.remove(crate::SERVER);
            }
        }
    }
    doc
}

/// Its `--approval-mode` for Escouade's mode.
fn approval_mode(mode: &str) -> &'static str {
    match mode {
        "plan" => "plan",
        "acceptEdits" => "auto_edit",
        "bypassPermissions" => "yolo",
        _ => "default",
    }
}

/// Gemini CLI's `gemini`: its session resumed when its store has it, else started under Escouade's id; its model unless
/// Auto, its approval mode, the folders it reads; its settings in the agent's home with its hooks and Escouade's server.
/// Its rules (`--policy`) and how it signs in are the core's (`prepare`), which adds their option.
pub fn gemini(p: &TerminalParams) -> Launch {
    let session = p.session.as_deref().or(p.new_session.as_deref());
    let mut a: Vec<String> = Vec::new();
    if let Some(s) = session {
        let flag = if p.resumes {
            "--resume"
        } else {
            "--session-id"
        };
        a.extend([flag.into(), s.to_string()]);
    }
    let model = p.model.trim();
    if !model.is_empty() && model != "auto" && !model.contains(char::is_whitespace) {
        a.extend(["--model".into(), model.to_string()]);
    }
    a.extend(["--approval-mode".into(), approval_mode(&p.mode).into()]);
    a.extend(
        p.read_dirs
            .iter()
            .flat_map(|d| ["--include-directories".to_string(), d.clone()]),
    );
    let mut env = vec![
        ("GEMINI_CLI_TRUST_WORKSPACE".to_string(), "true".to_string()),
        (p.hook_url_var.clone(), p.hook_url.clone()),
    ];
    let mut files = Vec::new();
    if let Some(home) = &p.home {
        let file = joined(home, ".gemini/settings.json");
        let before = crate::plugin::agent_file(&p.agent, Path::new(&file)).unwrap_or_default();
        let escouade = p
            .escouade
            .as_ref()
            .map(|g| (g.url.as_str(), g.token.as_str()));
        let doc = gemini_settings(
            &before,
            p.hook_command.as_deref(),
            escouade,
            &[&p.hook_url_var, &p.hook_token_var],
        );
        files.push(FileToWrite {
            path: file,
            text: serde_json::to_string_pretty(&doc).unwrap_or_default(),
        });
        env.extend(hook_path(p));
    }
    Launch {
        args: a,
        env: vars(env),
        files,
        ..Launch::default()
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
