//! « Vue Antigravity »: the launch of Antigravity CLI's own interface (`agy` without `-p`) in a terminal
//! (`terminal/launch`), and the models of an account (`account/read`).
//!
//! - Its interface and `agy -p` keep their conversations in the agent's own home (`ESCOUADE_AGY_HOME`, its `USERPROFILE`
//!   and `HOME`): `--conversation <id>` opens the agent's when its store has it (`resumes`); a conversation `-p` opened
//!   and never wrote has none, and would be lost.
//! - Its hooks (`.gemini/config/hooks.json` of its home) run `escouade-hook <event>`: under Windows by its name, its
//!   folder first on the `PATH`.
//! - Escouade's server is in its home's MCP config, its token in the file (`agy` expands no variable there).
//! - Its first screens are passed: its colours (`cache/onboarding.json`) and the trust of the agent's folder
//!   (`trustedWorkspaces`), with the permission rules of the agent's mode and fence, those of its agents.
//! - The account's address is kept off its screen (`AGY_CLI_HIDE_ACCOUNT_INFO`).

use crate::words::{
    agy_mode, gemini_dir, grant_doc, home_env, mcp_file, set_provider, settings_doc, settings_file,
    supports_effort, temp_mark, AgyRules, DEFAULT, EFFORTS, KEY_VARS,
};
use escouade_plugin_protocol::engine::{
    AccountLines, AccountRead, AccountStep, EnvVar, FileToWrite, Launch, OneShotLaunch,
    OneShotOutput, OneShotParams, OneShotText, TerminalLaunch, TerminalParams,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// How long a hook may run, in seconds.
const HOOK_LIMIT_SECS: u64 = 10;

/// The events its hooks tell Escouade.
pub const EVENTS: [&str; 4] = ["SessionStart", "PreToolUse", "PostToolUse", "Stop"];

/// Its hooks in the agent's home.
pub fn hooks_file(home: &Path) -> PathBuf {
    gemini_dir(home).join("config").join("hooks.json")
}

/// Where it keeps whether its first screens were passed.
pub fn onboarding_file(home: &Path) -> PathBuf {
    gemini_dir(home)
        .join("antigravity-cli")
        .join("cache")
        .join("onboarding.json")
}

/// Whether `path` is one of Windows: a drive, or a backslash.
fn windows_path(path: &str) -> bool {
    path.contains('\\') || path.as_bytes().get(1) == Some(&b':')
}

/// The line its hooks run for `event`: `escouade-hook` by its path quoted for a POSIX shell, or under Windows by its name
/// (its folder first on the `PATH`), which `cmd /c` finds.
fn hook_line(command: &str, event: &str) -> String {
    if windows_path(command) {
        format!("escouade-hook {event}")
    } else {
        format!("'{}' {event}", command.replace('\'', r"'\''"))
    }
}

/// Its `hooks.json`: `escouade-hook <event>` for each event Escouade reads.
pub fn hooks_json(command: &str) -> Value {
    let mut spec = json!({});
    for event in EVENTS {
        let handler = json!({
            "type": "command", "command": hook_line(command, event), "timeout": HOOK_LIMIT_SECS,
        });
        spec[event] = if event.ends_with("ToolUse") {
            json!([{ "matcher": "*", "hooks": [handler] }])
        } else {
            json!([handler])
        };
    }
    json!({ "escouade": spec })
}

/// Its settings `doc` with the agent's folder `cwd` among its trusted workspaces.
pub fn trusted(mut doc: Value, cwd: &str) -> Value {
    if cwd.is_empty() {
        return doc;
    }
    let mut list: Vec<Value> = doc["trustedWorkspaces"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if !list.iter().any(|v| v.as_str() == Some(cwd)) {
        list.push(json!(cwd));
    }
    doc["trustedWorkspaces"] = json!(list);
    doc
}

/// The folder of `command`, put first on the `PATH` `path` (Windows: `;` between its folders).
pub fn path_with(command: &str, path: &str) -> Option<String> {
    let cut = command.rfind(['\\', '/'])?;
    let dir = &command[..cut];
    Some(match path.is_empty() {
        true => dir.to_string(),
        false => format!("{dir};{path}"),
    })
}

/// The arguments of an agent's interface: its conversation when its store has it, its model and effort, its mode.
pub fn args(p: &TerminalParams) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    if let Some(s) = p.session.as_deref().filter(|_| p.resumes) {
        a.extend(["--conversation".into(), s.to_string()]);
    }
    let model = p.model.trim();
    if !model.is_empty() && !model.contains(char::is_whitespace) {
        a.extend(["--model".into(), model.to_string()]);
        if supports_effort(model) && EFFORTS.contains(&p.effort.as_str()) {
            a.extend(["--effort".into(), p.effort.clone()]);
        }
    }
    if let Some(mode) = agy_mode(&p.mode) {
        a.extend(["--mode".into(), mode.into()]);
    }
    a.extend(
        p.read_dirs
            .iter()
            .flat_map(|d| ["--add-dir".to_string(), d.clone()]),
    );
    a
}

/// A JSON file's text, as `agy` writes them.
fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_default()
}

/// A file of the agent's home, read through the host (`agentHome`); none when it is not there.
fn home_file(agent: &str, path: &Path) -> String {
    escouade_acp::plugin::agent_file(agent, path).unwrap_or_default()
}

/// Its interface's launch: its files in the agent's home (its settings, its first screens passed, its hooks, Escouade's
/// server), its home as `HOME` and `USERPROFILE`, a key of the machine's kept from taking the account's place.
pub fn launch(p: &TerminalParams) -> TerminalLaunch {
    let mut files = Vec::new();
    let mut env = vec![
        ("AGY_CLI_HIDE_ACCOUNT_INFO".to_string(), "1".to_string()),
        (p.hook_url_var.clone(), p.hook_url.clone()),
    ];
    if let Some(home) = p.home.as_deref().map(Path::new) {
        env = home_env(&env, home);
        // Its own temporary folder: `agy` writes there with no rule nor question (ESC-389).
        files.push(FileToWrite {
            path: temp_mark(home).to_string_lossy().into_owned(),
            text: "Antigravity's temporary folder for this agent (TEMP, TMP), made by Escouade.\n"
                .into(),
        });
        let rules = AgyRules {
            cwd: p.cwd.clone(),
            read_dirs: p.read_dirs.clone(),
            fence: p.fence.clone(),
            read_only: p.read_only,
        };
        let file = settings_file(home);
        let doc = settings_doc(&home_file(&p.agent, &file), &rules, &p.mode, &[]);
        files.push(FileToWrite {
            path: file.to_string_lossy().into_owned(),
            text: pretty(&trusted(doc, &p.cwd)),
        });
        let onboarding = onboarding_file(home);
        let passed = serde_json::from_str::<Value>(&home_file(&p.agent, &onboarding))
            .is_ok_and(|v| v["onboardingComplete"] == true);
        if !passed {
            let done = json!({
                "consumerOnboardingComplete": true, "enterpriseOnboardingComplete": true,
                "onboardingComplete": true,
            });
            files.push(FileToWrite {
                path: onboarding.to_string_lossy().into_owned(),
                text: pretty(&done),
            });
        }
        let escouade = p
            .escouade
            .as_ref()
            .map(|g| (g.url.as_str(), g.token.as_str()));
        let mcp = mcp_file(home);
        files.push(FileToWrite {
            path: mcp.to_string_lossy().into_owned(),
            text: pretty(&grant_doc(&home_file(&p.agent, &mcp), escouade)),
        });
        if let Some(command) = &p.hook_command {
            files.push(FileToWrite {
                path: hooks_file(home).to_string_lossy().into_owned(),
                text: pretty(&hooks_json(command)),
            });
            if windows_path(command) {
                if let Some(path) = path_with(command, p.path.as_deref().unwrap_or_default()) {
                    env.push(("PATH".into(), path));
                }
            }
        }
    }
    // A key of the machine's would take the account's place.
    env.extend(
        KEY_VARS
            .iter()
            .filter(|k| !p.env_names.iter().any(|n| n == *k))
            .map(|k| (k.to_string(), String::new())),
    );
    TerminalLaunch {
        launch: Launch {
            args: args(p),
            env: env
                .into_iter()
                .map(|(name, value)| EnvVar { name, value })
                .collect(),
            files,
            fence_applied: !p.fence.is_empty(),
            read_only: p.read_only,
            ..Launch::default()
        },
        ..TerminalLaunch::default()
    }
}

/// The models `agy models` lists (a line each: its id, a tab, its name), as `ToldModel`s: a Gemini model's efforts
/// (`gemini-3.8-flash-high`) under its base id, `medium` its default; the default model marked.
pub fn models_of(listed: &str) -> Vec<Value> {
    let mut out: Vec<(String, String, Vec<String>)> = Vec::new();
    for line in listed.lines() {
        let Some((id, label)) = line.split_once('\t') else {
            continue;
        };
        let (id, label) = (id.trim(), label.trim());
        if id.is_empty() {
            continue;
        }
        match crate::words::effort_of(id).filter(|(base, _)| base.starts_with("gemini-")) {
            Some((base, effort)) => {
                let name = label.rsplit_once(" (").map_or(label, |(n, _)| n);
                let at = match out.iter().position(|m| m.0 == base) {
                    Some(at) => at,
                    None => {
                        out.push((base.to_string(), name.to_string(), Vec::new()));
                        out.len() - 1
                    }
                };
                out[at].2.push(effort.to_string());
            }
            None => out.push((id.to_string(), label.to_string(), Vec::new())),
        }
    }
    let first = out
        .iter()
        .position(|m| m.0 == DEFAULT)
        .or((!out.is_empty()).then_some(0));
    out.into_iter()
        .enumerate()
        .map(|(i, (id, label, mut efforts))| {
            efforts.sort_by_key(|e| EFFORTS.iter().position(|x| x == e).unwrap_or(9));
            let default_effort =
                (!efforts.is_empty()).then(|| match efforts.iter().any(|e| e == "medium") {
                    true => "medium".to_string(),
                    false => efforts[0].clone(),
                });
            json!({
                "id": id, "label": label, "efforts": efforts, "defaultEffort": default_effort,
                "images": false, "default": Some(i) == first,
            })
        })
        .collect()
}

/// The composer's menu of the models `told`, else Antigravity's default one.
pub fn catalog(told: Vec<Value>) -> Vec<Value> {
    if !told.is_empty() {
        return told;
    }
    models_of(&format!(
        "{DEFAULT}-high\tGemini 3.8 Flash (High)\n{DEFAULT}-medium\tGemini 3.8 Flash (Medium)\n{DEFAULT}-low\tGemini 3.8 Flash (Low)"
    ))
}

/// The first step of a read of its account: its models, by `agy models` in a home of its own the core made for the read
/// (`home`), the provider of its models as the account's (the Gemini API for an account by its key).
pub fn read(r: &AccountRead) -> AccountStep {
    if r.read != "models" {
        return AccountStep::failed(format!("Antigravity CLI reads no {}", r.read));
    }
    let Some(home) = r.home.as_deref().map(Path::new) else {
        return AccountStep::failed("no home to read its models in");
    };
    let mut doc = json!({});
    set_provider(&mut doc, r.env_names.iter().any(|n| n == KEY_VARS[0]));
    let env = home_env(&[], home)
        .into_iter()
        .map(|(name, value)| EnvVar { name, value })
        .collect();
    AccountStep::start(
        Launch {
            args: vec!["models".into()],
            env,
            files: vec![FileToWrite {
                path: settings_file(home).to_string_lossy().into_owned(),
                text: pretty(&doc),
            }],
            ..Launch::default()
        },
        Vec::new(),
        json!({ "listed": "" }),
    )
}

/// What `agy models` wrote, gathered until it ends.
pub fn lines(l: &AccountLines) -> AccountStep {
    let mut listed = l.state["listed"].as_str().unwrap_or_default().to_string();
    for line in &l.lines {
        listed.push_str(line);
        listed.push('\n');
    }
    if !l.ended {
        return AccountStep::more(Vec::new(), json!({ "listed": listed }));
    }
    let told = models_of(&listed);
    match told.is_empty() {
        true => AccountStep::failed("agy models: no model listed"),
        false => AccountStep::done(Value::Array(catalog(told))),
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;

/// The model of a text of `weight` (`light`, else `reading` or `given`, ESC-66): every tool is refused whatever it is.
pub fn text_model(weight: &str) -> String {
    match weight {
        "light" => format!("{DEFAULT}-low"),
        _ => format!("{DEFAULT}-medium"),
    }
}

/// One question to `agy -p` (`engine/oneShot`), in the home the core made for it and removes once answered, on its
/// fastest Gemini, every tool refused (ESC-271): its settings there (the provider of its models as the account's), its
/// home as `HOME` and `USERPROFILE`, the question on its input.
pub fn one_shot(p: &OneShotParams) -> Result<OneShotLaunch, String> {
    let Some(home) = p.home.as_deref().map(Path::new) else {
        return Err("Antigravity CLI writes a text in a home of its own: none given".into());
    };
    let mut doc = json!({});
    set_provider(&mut doc, p.env_names.iter().any(|n| n == KEY_VARS[0]));
    doc["permissions"] = json!({
        "allow": [],
        "deny": ["command(*)", "write_file(*)", "read_file(*)", "mcp(*)", "read_url(*)", "execute_url(*)"],
        "ask": [],
    });
    let mut args = crate::words::launch_args(&crate::words::Settings::default(), None);
    args.extend(["--model".to_string(), text_model(&p.weight)]);
    let question = match p.system.trim().is_empty() {
        true => p.prompt.clone(),
        false => format!(
            "<instructions>\n{}\n</instructions>\n\n{}",
            p.system, p.prompt
        ),
    };
    Ok(OneShotLaunch {
        args,
        env: home_env(&[], home)
            .into_iter()
            .map(|(name, value)| EnvVar { name, value })
            .collect(),
        input: Some(format!(
            "{}\n",
            json!({ "event": "user", "message": { "content": question } })
        )),
        files: vec![FileToWrite {
            path: settings_file(home).to_string_lossy().into_owned(),
            text: pretty(&doc),
        }],
        ..OneShotLaunch::default()
    })
}

/// What `agy` answered in stream-json (its `result`), or why not.
pub fn one_shot_read(o: &OneShotOutput, lang: &str) -> Result<OneShotText, String> {
    let result = o
        .stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["event"] == "result")
        .ok_or_else(|| match lang {
            "en" => "Antigravity CLI gave no answer".to_string(),
            _ => "Antigravity CLI n’a rien répondu".to_string(),
        })?;
    let r = &result["result"];
    if r["status"] == "ERROR" {
        return Err(r["error"].as_str().unwrap_or("error").to_string());
    }
    Ok(OneShotText {
        text: r["response"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string(),
    })
}
