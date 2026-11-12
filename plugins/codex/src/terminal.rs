//! « Vue Codex »: the launch of Codex's own interface in a terminal (`terminal/launch`), and what an account tells by an
//! `app-server` of its own (`account/read`): its models, its quota, and the trust of the hooks its interface is given.
//!
//! - Its hooks are commands (`escouade-hook`), declared for this process alone by `-c hooks.<Event>=…`, never in the
//!   user's `config.toml`. Codex runs none it was not told to trust: their trust is given for this process too
//!   (`-c hooks.state=…`), with the keys and hashes that `hooks/list` of an app-server started with the same hooks gives
//!   (the read `hooks`, which the core runs before the interface starts). The user's hooks already trusted stay so.
//! - A session is resumed by `codex resume <id>`, a copy by `codex fork <id>`, in the same `CODEX_HOME` as the
//!   app-server of Escouade's view.
//! - An app-server for a read starts no thread: nothing is spent, and it costs little (0,6 s and 62 Mo on Codex 0.163.0).

use crate::words::{
    self, catalog, denied_args, fence_args, fence_profile, granted_args, meters, said,
    sandbox_mode, sandboxed, toml_string, TOKEN_VAR,
};
use escouade_plugin_protocol::engine::{
    AccountLines, AccountRead, AccountStep, EnvVar, Launch, TerminalLaunch, TerminalParams,
};
use serde_json::{json, Value};
use std::path::Path;

/// The hooks Escouade listens to.
pub const EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PermissionRequest",
    "PostToolUse",
    "Stop",
    "Interrupt",
];

/// How many pages of `model/list` are read at most (a cursor that never ends ends there).
const MODEL_PAGES: u64 = 50;

/// The hooks of an interface: `escouade-hook` for each event Escouade listens to, as options for this process alone. Its
/// path quoted when it has a space (Codex runs it through a shell).
pub fn hook_args(command: &str) -> Vec<String> {
    let line = if command.contains(' ') {
        format!("\"{command}\"")
    } else {
        command.to_string()
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
pub fn hooks_among(args: &[String]) -> Vec<String> {
    args.windows(2)
        .filter(|w| w[0] == "-c" && w[1].starts_with("hooks."))
        .flat_map(|w| w.to_vec())
        .collect()
}

/// The option that trusts the hooks `hooks/list` lists (its answer `list`): those of this process (`sessionFlags`), and
/// those the user trusts already. None when there is none.
pub fn trust_args(list: &Value) -> Option<Vec<String>> {
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

/// The arguments of an agent's interface: its session resumed (`resume <id>`) or copied (`fork <id>`), or a new one; its
/// model, effort and mode; its instructions for a new session (a resumed one keeps those it had); the plan tool, as its
/// app-server has it; its hooks.
pub fn args(p: &TerminalParams) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    let new = match (&p.session, &p.fork_of) {
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
    if !p.model.is_empty() {
        a.extend(["-m".into(), p.model.clone()]);
    }
    if !p.effort.is_empty() {
        a.extend([
            "-c".into(),
            format!("model_reasoning_effort={}", toml_string(&p.effort)),
        ]);
    }
    // « Projet privé »: Escouade's profile of its mode, in place of its sandbox (`-s`, which Codex would take over the
    // profile). A discussion (ESC-261): read only, whatever its mode.
    let mode = sandboxed(&p.mode, p.read_only);
    let profile = (!p.fence.is_empty()).then(|| fence_profile(mode)).flatten();
    match (mode, profile) {
        ("auto", _) => a.push("--approve-for-me".into()),
        ("bypassPermissions", _) => a.push("--dangerously-bypass-approvals-and-sandbox".into()),
        (_, Some(_)) => a.extend(["-a".into(), "on-request".into()]),
        (mode, None) => a.extend([
            "-a".into(),
            "on-request".into(),
            "-s".into(),
            sandbox_mode(mode).into(),
        ]),
    }
    if let Some(profile) = profile {
        a.extend(fence_args(&p.fence, profile));
    }
    a.extend(["-c".into(), "tools.update_plan.enabled=true".into()]);
    if let Some(text) = p
        .append_prompt
        .as_deref()
        .filter(|t| new && !t.trim().is_empty())
    {
        let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
        a.extend([
            "-c".into(),
            format!("developer_instructions={}", toml_string(&one_line)),
        ]);
    }
    if let Some(command) = &p.hook_command {
        a.extend(hook_args(command));
    }
    a
}

/// Its interface's launch. Refused, Escouade's server is turned off only when the account's `config.toml` declares it
/// (an entry of `enabled` alone keeps Codex from starting); its hooks' trust read before it starts (`hooks`).
pub fn launch(p: &TerminalParams) -> TerminalLaunch {
    let mut a = args(p);
    let mut env = vec![EnvVar {
        name: p.hook_url_var.clone(),
        value: p.hook_url.clone(),
    }];
    match &p.escouade {
        Some(g) => {
            a.extend(granted_args(&g.url));
            env.push(EnvVar {
                name: TOKEN_VAR.into(),
                value: g.token.clone(),
            });
        }
        None => {
            let declared = p.config_dir.as_deref().is_some_and(|dir| {
                escouade_acp::plugin::agent_file(&p.agent, &Path::new(dir).join("config.toml"))
                    .is_some_and(|text| words::declares_escouade(&text))
            });
            if declared {
                a.extend(denied_args());
            }
        }
    }
    let lang = p.lang.as_str();
    let (before, warn) = match p.hook_command {
        Some(_) => (
            Some(AccountRead {
                read: "hooks".into(),
                input: json!({
                    "cwd": p.cwd,
                    "failed": said(lang,
                        "Escouade ne suivra pas l’état de cet agent dans son terminal : Codex n’a pas dit comment approuver ses hooks ({error}).",
                        "Escouade won’t follow this agent’s state in its terminal: Codex did not tell how to trust its hooks ({error})."),
                }),
                ..AccountRead::default()
            }),
            None,
        ),
        None => (
            None,
            Some(said(lang,
                "Escouade ne suivra pas l’état de cet agent dans son terminal : escouade-hook est introuvable à côté de l’app.",
                "Escouade won’t follow this agent’s state in its terminal: escouade-hook is not found beside the app.")),
        ),
    };
    TerminalLaunch {
        launch: Launch {
            args: a,
            env,
            fence_applied: !p.fence.is_empty(),
            read_only: p.read_only,
            ..Launch::default()
        },
        before,
        warn,
    }
}

/// A request of the app-server, as a line.
fn request(id: u64, method: &str, params: Value) -> String {
    json!({ "id": id, "method": method, "params": params }).to_string()
}

/// Its first lines: hello, then `method`, of id 2.
fn hello_then(method: &str, params: Value) -> Vec<String> {
    vec![
        request(
            1,
            "initialize",
            json!({
                "clientInfo": { "name": "escouade", "title": "Escouade", "version": env!("CARGO_PKG_VERSION") },
                "capabilities": {},
            }),
        ),
        json!({ "method": "initialized" }).to_string(),
        request(2, method, params),
    ]
}

/// The first step of a read of its account, by an app-server of its own: its models (`model/list`, every page), its quota
/// (`account/rateLimits/read`), the trust of an interface's hooks (`hooks/list`, the app-server started with them).
pub fn read(r: &AccountRead) -> AccountStep {
    let server = |more: Vec<String>| {
        let mut args = vec!["app-server".to_string()];
        args.extend(more);
        Launch {
            args,
            ..Launch::default()
        }
    };
    match r.read.as_str() {
        "models" => AccountStep::start(
            server(Vec::new()),
            hello_then("model/list", json!({})),
            json!({ "data": [], "page": 1 }),
        ),
        "quota" => AccountStep::start(
            server(Vec::new()),
            hello_then("account/rateLimits/read", json!({})),
            Value::Null,
        ),
        "hooks" => {
            let args: Vec<String> = r.input["args"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|a| a.as_str().map(str::to_string))
                .collect();
            let hooks = hooks_among(&args);
            if hooks.is_empty() {
                return AccountStep::failed("escouade-hook is not among its arguments");
            }
            let cwd = r.input["cwd"].as_str().unwrap_or_default();
            AccountStep::start(
                server(hooks),
                hello_then("hooks/list", json!({ "cwds": [cwd] })),
                json!({ "args": args }),
            )
        }
        other => AccountStep::failed(format!("Codex reads no {other}")),
    }
}

/// What its app-server wrote: the answer to the request of the read, a page after another for its models.
pub fn lines(l: &AccountLines) -> AccountStep {
    let mut state = l.state.clone();
    for line in &l.lines {
        let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        let id = msg["id"].as_u64();
        if !msg["method"].is_null() || id.is_none_or(|id| id < 2) {
            continue;
        }
        if let Some(e) = msg.get("error").filter(|e| !e.is_null()) {
            let why = e["message"]
                .as_str()
                .map_or_else(|| e.to_string(), str::to_string);
            return AccountStep::failed(format!("{}: {why}", l.read));
        }
        let result = &msg["result"];
        match l.read.as_str() {
            "quota" => {
                return AccountStep::done(json!({ "meters": meters(&result["rateLimits"]) }))
            }
            "hooks" => {
                return match trust_args(result) {
                    Some(trust) => {
                        let mut args = state["args"].as_array().cloned().unwrap_or_default();
                        args.extend(trust.into_iter().map(Value::String));
                        AccountStep::done(json!({ "args": args }))
                    }
                    None => AccountStep::failed(format!("hooks/list: {result}")),
                };
            }
            "models" => {
                if let (Some(all), Some(page)) =
                    (state["data"].as_array_mut(), result["data"].as_array())
                {
                    all.extend(page.iter().cloned());
                }
                let page = state["page"].as_u64().unwrap_or(1);
                match result["nextCursor"].as_str() {
                    Some(cursor) if page < MODEL_PAGES => {
                        state["page"] = json!(page + 1);
                        let next = request(page + 2, "model/list", json!({ "cursor": cursor }));
                        return AccountStep::more(vec![next], state);
                    }
                    _ => {
                        let list = json!({ "data": state["data"] });
                        return AccountStep::done(catalog(&list).unwrap_or(Value::Null));
                    }
                }
            }
            other => return AccountStep::failed(format!("Codex reads no {other}")),
        }
    }
    match l.ended {
        true => AccountStep::failed(format!("{}: process exited", l.read)),
        false => AccountStep::more(Vec::new(), state),
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
