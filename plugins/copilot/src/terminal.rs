//! « Vue Copilot »: the launch of Copilot CLI's own interface in a terminal (`terminal/launch`), and what an account tells
//! by a server of its own (`account/read`): its models, its quota, the models it lists before its interface starts, the
//! copy of a session its interface cannot make.
//!
//! - Its hooks are commands (`escouade-hook <event>`), in a plugin of the agent's own given by `--plugin-dir`, beside its
//!   private file (`<plugin>/plugin.json`, `<plugin>/hooks/hooks.json`): for this process alone, never in the
//!   `COPILOT_HOME` of the account, which the user's own sessions read too. Their payloads do not say their event: its
//!   name is the command's argument.
//! - Escouade names a new session (`--session-id <uuid>`); a session is resumed by `--resume <id>`. Its interface cannot
//!   copy a session: a copy is made first by a server of its own (`sessions.fork`, the read `fork`).
//! - A model its account does not list goes as `auto` (ESC-122): Copilot CLI 1.0.98 takes it so, but says it in red.

use crate::words::{
    catalog, credits, server_args, signed_in, takes_effort, INHERITED_TOKENS, SECRETS, SERVER,
};
use escouade_plugin_protocol::engine::{
    AccountLines, AccountRead, AccountStep, EnvVar, FileToWrite, Launch, TerminalLaunch,
    TerminalParams,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

/// The hooks Escouade listens to.
pub const EVENTS: &[&str] = &[
    "sessionStart",
    "userPromptSubmitted",
    "preToolUse",
    "permissionRequest",
    "notification",
    "postToolUse",
    "postToolUseFailure",
    "agentStop",
    "sessionEnd",
];

/// The efforts its `--reasoning-effort` takes.
const EFFORTS: [&str; 7] = ["none", "minimal", "low", "medium", "high", "xhigh", "max"];

/// How long a hook may run, in seconds.
const HOOK_LIMIT_SECS: u64 = 10;

/// The models each account lists (by its folder), asked once for the plugin's run: a plan changed (Free to Pro) is seen
/// at the next start of the app.
static LISTED: Mutex<Option<HashMap<String, Vec<String>>>> = Mutex::new(None);

/// The plugin of the agent's interface, beside its private file: `<id>.copilot-plugin`.
pub fn plugin_dir(private_file: &str) -> String {
    match private_file.strip_suffix(".json") {
        Some(stem) => format!("{stem}.copilot-plugin"),
        None => format!("{private_file}.copilot-plugin"),
    }
}

/// `dir` and `name`, by the separator `dir` is written with.
fn joined(dir: &str, name: &str) -> String {
    let sep = if dir.contains('\\') { '\\' } else { '/' };
    format!(
        "{}{sep}{}",
        dir.trim_end_matches(['/', '\\']),
        name.replace('/', &sep.to_string())
    )
}

/// The quotes PowerShell reads as `'` in a string between them: each is doubled there.
const POWERSHELL_QUOTES: [char; 5] = ['\'', '\u{2018}', '\u{2019}', '\u{201A}', '\u{201B}'];

/// The line that runs `command` for `event`, in PowerShell (Windows) or in a POSIX shell: by its whole path, never by
/// its name, which a shell may look for in the agent's folder first.
fn hook_line(command: &str, event: &str, powershell: bool) -> String {
    if powershell {
        let mut quoted = String::new();
        for c in command.chars() {
            quoted.push(c);
            if POWERSHELL_QUOTES.contains(&c) {
                quoted.push(c);
            }
        }
        format!("& '{quoted}' {event}")
    } else {
        format!("'{}' {event}", command.replace('\'', r"'\''"))
    }
}

/// Its plugin's hooks (`hooks/hooks.json`): `escouade-hook <event>` for each event Escouade listens to.
pub fn hooks_file(command: &str) -> Value {
    let hooks: serde_json::Map<String, Value> = EVENTS
        .iter()
        .map(|e| {
            let entry = json!({
                "type": "command",
                "bash": hook_line(command, e, false),
                "powershell": hook_line(command, e, true),
                "timeoutSec": HOOK_LIMIT_SECS,
            });
            (e.to_string(), json!([entry]))
        })
        .collect();
    json!({ "version": 1, "hooks": hooks })
}

/// The arguments of an agent's interface: its session resumed or named, Escouade's options of its server (no built-in
/// MCP, nor the system's temporary folder, the tokens out of its commands), its model, effort and mode, and its plugin
/// (`plugin`) when there is `escouade-hook` to run.
pub fn args(p: &TerminalParams, plugin: Option<&str>) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    match (&p.session, &p.new_session) {
        (Some(s), _) => a.extend(["--resume".into(), s.clone()]),
        (None, Some(s)) => a.extend(["--session-id".into(), s.clone()]),
        (None, None) => {}
    }
    a.extend([
        "--no-auto-update".into(),
        "--disable-builtin-mcps".into(),
        "--disallow-temp-dir".into(),
        format!("--secret-env-vars={SECRETS}"),
    ]);
    // Never `auto`, which picks a model itself.
    if !p.model.is_empty() && p.model != "auto" {
        a.extend(["--model".into(), p.model.clone()]);
    }
    if EFFORTS.contains(&p.effort.as_str()) && takes_effort(&p.model) {
        a.extend(["--reasoning-effort".into(), p.effort.clone()]);
    }
    // No write goes without a question in « Édits auto » nor in « Auto »: `--allow-tool=write` would cover the `.git` of
    // the worktree too (a hook, `core.fsmonitor`, then a `git commit` with no question), and nothing proves a refusal
    // by path (`write(<dir>)`) covers the files under it. `git add` and `git commit` in « Auto », as the process.
    match p.mode.as_str() {
        _ if p.read_only => {}
        "bypassPermissions" => a.push("--allow-all".into()),
        "plan" => a.extend(["--mode".into(), "plan".into()]),
        "auto" => a.extend([
            "--mode".into(),
            "autopilot".into(),
            "--allow-tool=shell(git add)".into(),
            "--allow-tool=shell(git commit)".into(),
        ]),
        _ => {}
    }
    a.extend(denials(p));
    if let Some(dir) = plugin {
        a.extend(["--plugin-dir".into(), dir.to_string()]);
    }
    a
}

/// Its refusals, which win over every permission, `--allow-all` included: the writes in what changes what Copilot or
/// `git` run (`.git`, Copilot's settings and hooks of the repository) and in the folders it reads alone (the files
/// joined to the messages); for a discussion (`read_only`), every write and every command.
pub fn denials(p: &TerminalParams) -> Vec<String> {
    let mut d: Vec<String> = Vec::new();
    if p.read_only {
        d.extend(["--deny-tool=write".into(), "--deny-tool=shell".into()]);
    }
    d.extend(
        CONFIG_NAMES
            .iter()
            .map(|n| format!("--deny-tool=write({n})")),
    );
    if !p.cwd.is_empty() {
        d.extend(
            CONFIG_DIRS
                .iter()
                .map(|n| format!("--deny-tool=write({})", joined(&p.cwd, n))),
        );
    }
    d.extend(
        p.read_dirs
            .iter()
            .map(|r| format!("--deny-tool=write({r})")),
    );
    d
}

/// A fenced agent's interface is refused: nothing in Copilot's interface keeps it from the paths of a private project
/// (its rules have no kind for a read, and its hooks only look on), so its view would read them with no question in
/// « Bypass », and on the user's word elsewhere. Its conversation, whose rules Escouade answers, holds the fence.
pub fn refused(p: &TerminalParams) -> Option<String> {
    (!p.fence.is_empty()).then(|| {
        crate::words::said(
            &p.lang,
            "La vue Copilot CLI ne tient pas la clôture des projets privés : cet agent reste dans sa conversation, qui la tient.".into(),
            "Copilot CLI’s view doesn’t hold the fence of private projects: this agent stays in its conversation, which holds it.".into(),
        )
    })
}

/// The names refused to its writes wherever they are (« by trailing path components »).
pub const CONFIG_NAMES: [&str; 2] = [".git", ".copilot"];

/// The folders of the agent's refused to its writes, by their whole path: `.git`, and what Copilot reads of a
/// repository's settings and hooks (`copilot help config`, 1.0.98).
pub const CONFIG_DIRS: [&str; 4] = [".git", ".copilot", ".github/copilot", ".github/hooks"];

/// An account of its own (a `COPILOT_HOME` given) signs in by itself (`copilot login`): a GitHub token of the app's
/// environment would make it another one, Copilot preferring it.
fn own_home(names: &[String]) -> bool {
    names.iter().any(|n| n == "COPILOT_HOME")
}

/// Its interface's launch: its plugin of hooks beside its private file, Escouade's server, and the model its account
/// lists, read before it starts (`listed`).
pub fn launch(p: &TerminalParams) -> TerminalLaunch {
    let plugin = plugin_dir(&p.private_file);
    let files = match &p.hook_command {
        Some(command) => vec![
            FileToWrite {
                path: joined(&plugin, "plugin.json"),
                text: serde_json::to_string_pretty(&json!({
                    "name": "escouade",
                    "description": "Escouade follows the agent of this interface.",
                    "version": env!("CARGO_PKG_VERSION"),
                }))
                .unwrap_or_default(),
            },
            FileToWrite {
                path: joined(&plugin, "hooks/hooks.json"),
                text: serde_json::to_string_pretty(&hooks_file(command)).unwrap_or_default(),
            },
        ],
        None => Vec::new(),
    };
    let mut a = args(p, (!files.is_empty()).then_some(plugin.as_str()));
    a.extend(p.read_dirs.iter().map(|d| format!("--add-dir={d}")));
    // `escouade-hook` posts to the hooks' server at this address.
    let mut env = vec![EnvVar {
        name: p.hook_url_var.clone(),
        value: p.hook_url.clone(),
    }];
    // Escouade's server by the core's private file, which holds its token itself: the token is in no
    // variable its commands would inherit, and is among those it strips (`SECRETS`).
    match (&p.escouade, &p.escouade_file) {
        (Some(_), Some(file)) => {
            a.extend(["--additional-mcp-config".into(), format!("@{file}")]);
        }
        _ => a.extend(["--disable-mcp-server".into(), SERVER.into()]),
    }
    if own_home(&p.env_names) {
        env.extend(INHERITED_TOKENS.iter().map(|v| EnvVar {
            name: v.to_string(),
            value: String::new(),
        }));
    }
    let named = a
        .windows(2)
        .any(|w| w[0] == "--model" && takes_effort(&w[1]));
    let warn = files.is_empty().then(|| {
        crate::words::said(
            &p.lang,
            "Escouade ne suivra pas l’état de cet agent dans son terminal : escouade-hook est introuvable à côté de l’app.".into(),
            "Escouade won’t follow this agent’s state in its terminal: escouade-hook is not found beside the app.".into(),
        )
    });
    TerminalLaunch {
        launch: Launch {
            args: a,
            env,
            files,
            // Held by its refusals (`denials`).
            read_only: p.read_only,
            // Never: a fenced agent's view is refused (`refused`).
            fence_by_client: false,
            ..Launch::default()
        },
        before: named.then(|| AccountRead {
            read: "listed".into(),
            ..AccountRead::default()
        }),
        warn,
    }
}

/// The arguments `args` of an interface, its model `auto` (and no effort, which `auto` does not take) when the account
/// does not list it (`listed`).
pub fn model_of_account(mut args: Vec<String>, listed: &[String]) -> Vec<String> {
    let Some(i) = args.iter().position(|a| a == "--model") else {
        return args;
    };
    let Some(model) = args.get(i + 1) else {
        return args;
    };
    if !takes_effort(model) || listed.iter().any(|m| m == model) {
        return args;
    }
    args[i + 1] = "auto".into();
    if let Some(e) = args.iter().position(|a| a == "--reasoning-effort") {
        args.drain(e..(e + 2).min(args.len()));
    }
    args
}

/// A request of its server.
fn request(id: u64, method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| a.as_str().map(str::to_string))
        .collect()
}

/// The ids of the models an answer to `models.list` lists.
fn ids(listed: &Value) -> Vec<String> {
    listed["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["id"].as_str().map(str::to_string))
        .collect()
}

/// The first step of a read of its account, by a server of its own (`copilot --server --stdio`, which opens no session
/// and asks no model): `models` (`models.list`, one page), `quota` (`account.getQuota`, when it is signed in as the
/// account), `listed` (the models listed, to fit an interface's arguments), `fork` (`sessions.fork`).
pub fn read(r: &AccountRead) -> AccountStep {
    let mut args = server_args();
    let remove_env = match own_home(&r.env_names) {
        true => INHERITED_TOKENS.map(str::to_string).to_vec(),
        false => Vec::new(),
    };
    let home = r.home.clone().unwrap_or_default();
    let connect = |params: Value| request(1, "connect", params);
    let (writes, state) = match r.read.as_str() {
        "models" => (
            vec![connect(json!({})), request(2, "models.list", json!({}))],
            json!({ "step": "models" }),
        ),
        "listed" => {
            let args_of = strings(&r.input["args"]);
            let known = LISTED
                .lock()
                .ok()
                .and_then(|l| l.as_ref().and_then(|l| l.get(&home).cloned()));
            if let Some(listed) = known {
                return AccountStep::done(json!({ "args": model_of_account(args_of, &listed) }));
            }
            (
                vec![connect(json!({})), request(2, "models.list", json!({}))],
                json!({ "step": "listed", "args": args_of, "home": home }),
            )
        }
        "quota" => {
            args.extend(["--disable-mcp-server".into(), SERVER.into()]);
            (
                vec![
                    connect(json!({ "supportedTaskKinds": ["agent", "client", "shell"] })),
                    request(2, "auth.getStatus", json!({})),
                ],
                json!({ "step": "auth", "ownHome": own_home(&r.env_names) }),
            )
        }
        "fork" => {
            let (of, at) = (&r.input["of"], &r.input["at"]);
            let first = match at.as_str() {
                Some(_) => request(
                    2,
                    "sessions.readPersistedEvents",
                    json!({ "sessionId": of }),
                ),
                None => request(
                    3,
                    "sessions.fork",
                    json!({ "sessionId": of, "name": "Escouade" }),
                ),
            };
            (
                vec![connect(json!({})), first],
                json!({ "step": "fork", "of": of, "at": at }),
            )
        }
        other => return AccountStep::failed(format!("Copilot CLI reads no {other}")),
    };
    AccountStep::start(
        Launch {
            args,
            remove_env,
            ..Launch::default()
        },
        writes,
        state,
    )
}

/// What its server wrote: the answers of the read's requests, one after the other.
pub fn lines(l: &AccountLines) -> AccountStep {
    let state = &l.state;
    for line in &l.lines {
        let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        let Some(id) = msg["id"].as_u64().filter(|_| msg.get("method").is_none()) else {
            continue;
        };
        if id < 2 {
            continue;
        }
        if let Some(e) = msg.get("error").filter(|e| !e.is_null()) {
            let why = e["message"].as_str().unwrap_or("error");
            return AccountStep::failed(format!("{}: {why}", l.read));
        }
        let result = &msg["result"];
        match (state["step"].as_str().unwrap_or_default(), id) {
            ("models", 2) => {
                let told = catalog(Some(result), None);
                return AccountStep::done(match told.is_empty() {
                    true => Value::Null,
                    false => Value::Array(told),
                });
            }
            ("listed", 2) => {
                let listed = ids(result);
                let home = state["home"].as_str().unwrap_or_default().to_string();
                if let Ok(mut l) = LISTED.lock() {
                    l.get_or_insert_with(HashMap::new)
                        .insert(home, listed.clone());
                }
                let args = model_of_account(strings(&state["args"]), &listed);
                return AccountStep::done(json!({ "args": args }));
            }
            ("auth", 2) => {
                if !signed_in(result, state["ownHome"] == true) {
                    return AccountStep::done(json!({ "notSignedIn": true }));
                }
                let mut next = state.clone();
                next["step"] = json!("quota");
                return AccountStep::more(vec![request(3, "account.getQuota", json!({}))], next);
            }
            ("quota", 3) => {
                let now = chrono::Utc::now().timestamp_millis();
                let meters: Vec<Value> = credits(&result["quotaSnapshots"], now)
                    .map(|c| c.meter())
                    .into_iter()
                    .collect();
                return AccountStep::done(json!({ "meters": meters }));
            }
            ("fork", 2) => {
                // `toEventId` keeps what is before it: the event after `at`.
                let list = result["events"].as_array().cloned().unwrap_or_default();
                let after = list
                    .iter()
                    .position(|e| e["id"] == state["at"])
                    .and_then(|i| list.get(i + 1))
                    .and_then(|e| e["id"].as_str());
                let mut fork = json!({ "sessionId": state["of"], "name": "Escouade" });
                if let Some(after) = after {
                    fork["toEventId"] = json!(after);
                }
                return AccountStep::more(vec![request(3, "sessions.fork", fork)], state.clone());
            }
            ("fork", 3) => {
                return match result["sessionId"].as_str() {
                    Some(s) => AccountStep::done(json!({ "session": s })),
                    None => AccountStep::failed("sessions.fork: no session"),
                };
            }
            _ => {}
        }
    }
    match l.ended {
        true => AccountStep::failed(format!("{}: copilot ended before it answered", l.read)),
        false => AccountStep::more(Vec::new(), state.clone()),
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
