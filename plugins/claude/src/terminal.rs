//! « Vue Claude Code »: the launch of Claude Code's own interface in a terminal (`terminal/launch`), and what an account
//! tells by a process of its own (`account/read`). The core checks the launch, writes its files and keeps the PTY.
//!
//! Its hooks are HTTP hooks to Escouade's server, given with `--settings` (its private file): the core adds there the
//! fence of a private project and the refusal of Escouade's server, which are its own (`Engine::fence_settings`). Its
//! models are what `initialize` tells, by a process given no turn: it asks nothing of the API (ESC-121, ESC-124), and no
//! session is written.

use crate::client::{initialize_line, initialized, supports_effort, BASE_ARGS, ESCOUADE_TOOLS};
use escouade_plugin_protocol::engine::{
    AccountLines, AccountRead, AccountStep, FileToWrite, Launch, TerminalLaunch, TerminalParams,
};
use serde_json::{json, Value};

/// The hooks Escouade listens to.
pub const EVENTS: &[&str] = &[
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "PostToolUseFailure",
    "Notification",
    "Stop",
    "StopFailure",
];

/// The id of the request a read of its models asks.
const INITIALIZE: &str = "ccm_1";

/// The settings an interface is started with (`--settings <file>`): its hooks, each an HTTP call to `url` with the
/// token of its variable `token_var`.
pub fn settings(url: &str, token_var: &str) -> Value {
    let hook = json!([{ "hooks": [{
        "type": "http",
        "url": url,
        "headers": { "Authorization": format!("Bearer ${token_var}") },
        "allowedEnvVars": [token_var],
        "timeout": 5,
    }] }]);
    let hooks: serde_json::Map<String, Value> = EVENTS
        .iter()
        .map(|e| (e.to_string(), hook.clone()))
        .collect();
    json!({ "hooks": hooks })
}

/// The arguments of an agent's interface, as its stream's are less the stream's own: its model, effort and mode, its
/// added instructions, where it starts from, its link to claude.ai, its settings.
pub fn args(p: &TerminalParams) -> Vec<String> {
    let mut a: Vec<String> = vec!["--model".into(), p.model.clone()];
    if supports_effort(&p.model) {
        a.extend(["--effort".into(), p.effort.clone()]);
    }
    a.extend([
        "--permission-mode".into(),
        p.mode.clone(),
        "--allow-dangerously-skip-permissions".into(),
    ]);
    if let Some(text) = p.append_prompt.as_deref().filter(|t| !t.trim().is_empty()) {
        let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
        a.extend(["--append-system-prompt".into(), one_line]);
    }
    match (&p.session, &p.fork_of) {
        (Some(s), _) => a.push(format!("--resume={s}")),
        (None, Some(f)) => {
            a.extend([format!("--resume={f}"), "--fork-session".into()]);
            if let Some(at) = &p.fork_at {
                a.push(format!("--resume-session-at={at}"));
            }
            a.extend(p.new_session.as_ref().map(|s| format!("--session-id={s}")));
        }
        (None, None) => a.extend(p.new_session.as_ref().map(|s| format!("--session-id={s}"))),
    }
    // One argument, the name glued to the flag: Claude Code takes it so, even one that starts with « - ».
    if let Some(name) = &p.remote {
        a.push(format!("--remote-control={name}"));
    }
    a.extend(["--settings".into(), p.private_file.clone()]);
    a
}

/// Its interface's launch: its arguments, the folders it reads without asking, Escouade's server (`--mcp-config` of the
/// file the core wrote, or its tools refused), its settings in its private file.
pub fn launch(p: &TerminalParams) -> TerminalLaunch {
    let mut a = args(p);
    a.extend(p.read_dirs.iter().map(|d| format!("--add-dir={d}")));
    match (&p.escouade, &p.escouade_file) {
        (Some(_), Some(file)) => a.extend(["--mcp-config".into(), file.clone()]),
        (Some(_), None) => {}
        (None, _) => a.extend(["--disallowedTools".into(), ESCOUADE_TOOLS.into()]),
    }
    let text =
        serde_json::to_string_pretty(&settings(&p.hook_url, &p.hook_token_var)).unwrap_or_default();
    TerminalLaunch {
        launch: Launch {
            args: a,
            files: vec![FileToWrite {
                path: p.private_file.clone(),
                text,
            }],
            // The core writes the fence in its settings (`fenceBy` the core): said as applied for it.
            fence_applied: !p.fence.is_empty(),
            read_only: p.read_only,
            ..Launch::default()
        },
        ..TerminalLaunch::default()
    }
}

/// The first step of a read of its account: its models (`initialize`, by a process given no turn).
pub fn read(r: &AccountRead) -> AccountStep {
    match r.read.as_str() {
        "models" => {
            let mut args: Vec<String> = BASE_ARGS.iter().map(|s| s.to_string()).collect();
            args.extend([
                "--no-session-persistence".into(),
                "--strict-mcp-config".into(),
            ]);
            AccountStep::start(
                Launch {
                    args,
                    ..Launch::default()
                },
                vec![initialize_line(INITIALIZE)],
                Value::Null,
            )
        }
        other => AccountStep::failed(format!("Claude Code reads no {other}")),
    }
}

/// What its process wrote: the models `initialize` tells, or why it refused.
pub fn lines(l: &AccountLines) -> AccountStep {
    for line in &l.lines {
        let Ok(frame) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        match initialized(&frame, INITIALIZE) {
            Some(Ok(told)) => return AccountStep::done(told["models"].clone()),
            Some(Err(why)) => return AccountStep::failed(why),
            None => {}
        }
    }
    match l.ended {
        true => AccountStep::failed("process exited"),
        false => AccountStep::more(Vec::new(), l.state.clone()),
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
