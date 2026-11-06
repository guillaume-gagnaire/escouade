//! The parity of the harnesses in modules (ESC-346, ESC-350, docs/PLUGINS.md §8.26): played by the host of the app, Codex,
//! Copilot CLI, Cursor CLI, Gemini CLI and Mistral Vibe answer what their native plugins answered, call by call, on the recordings of their
//! harness: the description of their launches, the items of each line, what they write back, every call of the contract.
//!
//! `tests/fixtures/plugins/parity/<short>.jsonl` holds, one step a line, what each native plugin answered before its
//! port. `ESCOUADE_PARITY_RECORD=1` and `ESCOUADE_NATIVE_PLUGINS=<a program of then>` write them again. Their modules come
//! from `sh scripts/plugins-wasm.sh`: without them a test says so and passes, with `ESCOUADE_PARITY=1` it fails.

use crate::plugins::wasm::{Fetcher, Net};
use crate::plugins_wasm_parity_tests::{call, hello, module, root, Hosted, Native, Plugin};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

/// A harness, and how its recordings speak.
struct Harness {
    /// Its folder under `plugins/`, and its entry in `escouade-plugins`.
    short: &'static str,
    id: &'static str,
    /// The folder of its recordings under `tests/fixtures`.
    fixtures: &'static str,
    files: &'static [&'static str],
    /// What the client sends when the user speaks, and when they stop it.
    message: &'static str,
    interrupt: &'static str,
    model: &'static str,
    /// What this client asks that its recordings do not answer (its recordings are of a client that did not ask): a
    /// method, and the result its harness gives.
    answers: &'static [(&'static str, &'static str)],
}

const CODEX: Harness = Harness {
    short: "codex",
    id: "dev.escouade.codex",
    fixtures: "codex",
    files: &[
        "full-turn",
        "plan-mode",
        "failed-turn",
        "resume",
        "account",
        "hooks-appserver",
    ],
    message: "turn/start",
    interrupt: "turn/interrupt",
    model: "gpt-6-luna",
    answers: &[],
};

const COPILOT: Harness = Harness {
    short: "copilot",
    id: "dev.escouade.copilot-cli",
    fixtures: "copilot",
    files: &[
        "full-turn",
        "instructions-modes-abort",
        "resume-fork",
        "quota-models-tools",
        "sandbox-server",
        "byok-ollama",
    ],
    message: "session.send",
    interrupt: "session.abort",
    model: "gpt-6-luna",
    answers: &[
        (
            "auth.getStatus",
            r#"{ "isAuthenticated": true, "authType": "user" }"#,
        ),
        ("session.permissions.setMode", "{}"),
        ("session.mode.set", "{}"),
        ("session.mcp.disable", "{}"),
        ("session.model.switchTo", "{}"),
        ("models.list", r#"{ "models": [] }"#),
        ("session.model.list", "{}"),
    ],
};

const CURSOR: Harness = Harness {
    short: "cursor",
    id: "dev.escouade.cursor-cli",
    fixtures: "cursor",
    files: &[
        "full-turn",
        "init-session",
        "load-modes-model",
        "mcp-allow-always",
        "midturn-ask",
        "sandbox-flag-commit",
    ],
    message: "session/prompt",
    interrupt: "session/cancel",
    model: "default",
    answers: &[],
};

const GEMINI: Harness = Harness {
    short: "gemini",
    id: "dev.escouade.gemini-cli",
    fixtures: "gemini",
    files: &[
        "02-simple-turn",
        "03-permission-write",
        "04-permission-reject",
        "05-permission-shell",
        "06-question-unavailable",
        "08-plan-enter",
        "09-plan-exit-approval",
        "10-cancel",
        "11-load-replay",
        "12-mcp-session-new",
        "13-rate-limit-429",
        "14-modes-model-and-missing-methods",
        "16-policy-fence-deny",
        "17-thoughts-and-chunks",
        "18-subagent-yolo",
    ],
    message: "session/prompt",
    interrupt: "session/cancel",
    model: "auto",
    answers: &[],
};

const VIBE: Harness = Harness {
    short: "mistral",
    id: "dev.escouade.mistral",
    fixtures: "vibe",
    files: &[
        "cancel-config-options",
        "error-invalid-key",
        "full-turn",
        "hooks-deny",
        "init-session",
        "load-fork-reject",
        "mcp-session-new",
        "permission-allow-always",
        "question-elicitation",
        "todo-subagent",
        "trust-project-hooks",
    ],
    message: "session/prompt",
    interrupt: "session/cancel",
    model: "mistral-medium-3.5",
    answers: &[],
};

/// The lines of a recording, as a client saw them: (`c2s` or `s2c`, the message).
fn recorded(h: &Harness, file: &str) -> Vec<(String, Value)> {
    let path = root()
        .join("tests/fixtures")
        .join(h.fixtures)
        .join(format!("{file}.jsonl"));
    std::fs::read_to_string(path)
        .expect("a recording")
        // A file of the machine it was recorded on, which a plan names: another machine reads none, nor does a module
        // here. `gemini_tests` and the e2e read the plan.
        .replace("esc273", "esc346-nowhere")
        .lines()
        .filter_map(|l| {
            let mut v: Value = serde_json::from_str(l).ok()?;
            let dir = v.as_object_mut()?.remove("dir")?.as_str()?.to_string();
            // Gemini's are the messages themselves, the others' hold them in `msg`.
            let msg = v.get("msg").cloned().unwrap_or(v);
            // Vibe's say `out` and `in`, seen from the client.
            let dir = match dir.as_str() {
                "out" => "c2s".to_string(),
                "in" => "s2c".to_string(),
                _ => dir,
            };
            matches!(dir.as_str(), "c2s" | "s2c").then_some((dir, msg))
        })
        .collect()
}

/// What may differ from one instance to the next: the ids of a plugin's own requests start at a number of its own
/// (`turn::instance_id`). The lines it writes are read too, for the rest of them to count.
fn steady(v: Value) -> Value {
    match v {
        Value::Object(o) => o
            .into_iter()
            .map(|(k, v)| {
                let kept = match (k.as_str(), v.as_u64()) {
                    ("id", Some(n)) => json!(n & 0xF_FFFF),
                    // Codex says the version of its crate, which its port changed.
                    ("clientInfo", _) if v["title"] == "Escouade" => Value::Null,
                    // Those of a map of the client, in the order of its hash.
                    ("writes", _) => {
                        let mut writes: Vec<Value> = v.as_array().cloned().unwrap_or_default();
                        writes = writes.into_iter().map(steady).collect();
                        writes.sort_by_key(Value::to_string);
                        Value::Array(writes)
                    }
                    _ => steady(v),
                };
                (k, kept)
            })
            .collect(),
        Value::Array(a) => a.into_iter().map(steady).collect(),
        Value::String(s) if s.starts_with('{') => match serde_json::from_str::<Value>(&s) {
            Ok(line) => Value::String(steady(line).to_string()),
            Err(_) => Value::String(s),
        },
        other => other,
    }
}

/// The requests a plugin wrote in `v`, whatever holds them: their ids by method. A recording's answers name the ids of
/// the client that made it: they are given to the requests of this one, for its client to take them as answers.
fn requests_in(v: &Value, ours: &mut HashMap<String, VecDeque<Value>>) {
    match v {
        Value::Object(o) => o.values().for_each(|v| requests_in(v, ours)),
        Value::Array(a) => a.iter().for_each(|v| requests_in(v, ours)),
        Value::String(s) if s.starts_with('{') => {
            let Ok(line) = serde_json::from_str::<Value>(s) else {
                return;
            };
            if let (Some(method), false) = (line["method"].as_str(), line["id"].is_null()) {
                let ids = ours.entry(method.to_string()).or_default();
                ids.push_back(line["id"].clone());
            }
        }
        _ => {}
    }
}

/// What a whole answer says: its result or its error, an empty one taken for nothing.
fn said(v: Value) -> Option<Value> {
    let answer = if v["error"].is_null() {
        v["result"].clone()
    } else {
        json!({ "error": v["error"] })
    };
    let answer = steady(answer);
    let empty = answer == json!({})
        || answer == json!({ "items": [] })
        || answer == json!({ "writes": [] })
        || answer.is_null();
    (!empty).then_some(answer)
}

/// One way of launching: what it adds to `engine/launch`.
fn launches() -> Vec<(&'static str, Value)> {
    vec![
        ("new", json!({ "mode": "default" })),
        ("plan", json!({ "mode": "plan" })),
        ("edits", json!({ "mode": "acceptEdits", "effort": "high" })),
        ("all", json!({ "mode": "bypassPermissions" })),
        ("read only", json!({ "readOnly": true })),
        (
            "fence",
            json!({ "fence": ["C:\\prive"], "appendPrompt": "Les consignes." }),
        ),
        (
            "resumed",
            json!({ "start": { "kind": "resume", "session": "la-session" } }),
        ),
        (
            "with Escouade",
            json!({ "escouade": { "url": "http://127.0.0.1:1/mcp", "token": "le-jeton" }, "home": "C:\\compte" }),
        ),
    ]
}

/// What `plugin` answers along the recordings of `h`, step by step: nothing is kept of a step that says nothing.
fn transcript(plugin: &mut dyn Plugin, h: &Harness) -> Vec<Value> {
    let mut steps: Vec<Value> = Vec::new();
    let mut id = 0;
    let mut ours = HashMap::new();
    let mut ask = |p: &mut dyn Plugin,
                   ours: &mut HashMap<String, VecDeque<Value>>,
                   m: &str,
                   params: Value,
                   label: &str| {
        id += 1;
        let whole = call(p, id, m, params);
        requests_in(&whole, ours);
        if let Some(answer) = said(whole) {
            steps.push(json!({ "at": label, "call": m, "answer": answer }));
        }
    };
    ask(plugin, &mut ours, "initialize", hello(), "start");
    let base = |agent: &str| {
        json!({
            "agent": agent, "cwd": "C:\\wt", "model": h.model, "mode": "default", "lang": "fr",
            "start": { "kind": "new" },
        })
    };
    for (i, (label, more)) in launches().into_iter().enumerate() {
        let agent = format!("launch{i}");
        let mut params = base(&agent);
        for (k, v) in more.as_object().unwrap() {
            params[k] = v.clone();
        }
        ask(plugin, &mut ours, "agent/attach", params.clone(), label);
        ask(plugin, &mut ours, "engine/launch", params, label);
        ask(
            plugin,
            &mut ours,
            "agent/detach",
            json!({ "agent": agent }),
            label,
        );
    }
    for file in h.files {
        let agent = *file;
        let params = base(agent);
        ours.clear();
        ask(plugin, &mut ours, "agent/attach", params.clone(), file);
        ask(plugin, &mut ours, "engine/launch", params, file);
        let mut asked = HashSet::new();
        let mut theirs: HashMap<String, String> = HashMap::new();
        for (n, (dir, msg)) in recorded(h, file).into_iter().enumerate() {
            let label = format!("{file}:{n}");
            let method = msg["method"].as_str().unwrap_or_default().to_string();
            if dir == "c2s" && !method.is_empty() && !msg["id"].is_null() {
                theirs.insert(msg["id"].to_string(), method.to_string());
            }
            if dir == "s2c" {
                let mut msg = msg;
                if !method.is_empty() && !msg["id"].is_null() {
                    asked.insert(msg["id"].to_string());
                } else if let Some(mine) = theirs
                    .remove(&msg["id"].to_string())
                    .and_then(|m| ours.get_mut(&m)?.pop_front())
                {
                    msg["id"] = mine;
                }
                let frames = json!({ "agent": agent, "frames": [msg.to_string()] });
                ask(plugin, &mut ours, "agent/frames", frames, &label);
            } else if method == h.message {
                let m = json!({ "agent": agent, "text": "La demande." });
                ask(plugin, &mut ours, "agent/sendMessage", m, &label);
            } else if method == h.interrupt {
                ask(
                    plugin,
                    &mut ours,
                    "agent/interrupt",
                    json!({ "agent": agent }),
                    &label,
                );
            } else if method.is_empty() && asked.remove(&msg["id"].to_string()) {
                let request = msg["id"]
                    .as_str()
                    .map_or_else(|| msg["id"].to_string(), str::to_string);
                let answer =
                    json!({ "agent": agent, "request": request, "allow": true, "always": false });
                ask(plugin, &mut ours, "agent/answerPermission", answer, &label);
            }
            while let Some((mine, result)) = h.answers.iter().find_map(|(m, result)| {
                let mine = ours.get_mut(*m)?.pop_front()?;
                Some((mine, *result))
            }) {
                let result: Value = serde_json::from_str(result).unwrap();
                let answer = json!({ "jsonrpc": "2.0", "id": mine, "result": result });
                let frames = json!({ "agent": agent, "frames": [answer.to_string()] });
                ask(plugin, &mut ours, "agent/frames", frames, &label);
            }
        }
        for (m, more) in [
            ("agent/setModel", json!({ "value": "un-autre" })),
            ("agent/setEffort", json!({ "value": "low" })),
            ("agent/setMode", json!({ "value": "plan" })),
            ("agent/readUsage", json!({})),
            ("agent/compact", json!({})),
            ("agent/rename", json!({ "value": "Un titre" })),
            ("agent/interrupt", json!({})),
        ] {
            let mut params = json!({ "agent": agent });
            for (k, v) in more.as_object().unwrap() {
                params[k] = v.clone();
            }
            ask(plugin, &mut ours, m, params, file);
        }
        ask(
            plugin,
            &mut ours,
            "agent/detach",
            json!({ "agent": agent }),
            file,
        );
    }
    ask(
        plugin,
        &mut ours,
        "agent/frames",
        json!({ "agent": "nobody", "frames": [] }),
        "end",
    );
    ask(plugin, &mut ours, "agent/unknown", json!({}), "end");
    steps
}

fn recording(short: &str) -> PathBuf {
    root()
        .join("tests/fixtures/plugins/parity")
        .join(format!("{short}.jsonl"))
}

fn parity_of(h: &Harness) {
    if std::env::var_os("ESCOUADE_PARITY_RECORD").is_some() {
        let mut native = Native::start(h.short);
        let text: Vec<String> = transcript(&mut native, h)
            .iter()
            .map(Value::to_string)
            .collect();
        std::fs::write(recording(h.short), text.join("\n") + "\n").unwrap();
        return;
    }
    let Some(wasm) = module(h.short) else {
        return;
    };
    let net = Net::default();
    let mut hosted = Hosted::start(wasm, Fetcher::new(h.id, &[], &net));
    let steps = transcript(&mut hosted, h);
    let text = std::fs::read_to_string(recording(h.short)).expect("a recording of the native");
    let native: Vec<Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    for (a, b) in steps.iter().zip(&native) {
        assert_eq!(a, b);
    }
    assert_eq!(steps.len(), native.len());
    assert!(steps.len() > 30, "{} steps", steps.len());
}

#[test]
fn vibe_answers_as_its_native_plugin() {
    parity_of(&VIBE);
}

#[test]
fn codex_answers_as_its_native_plugin() {
    parity_of(&CODEX);
}

#[test]
fn copilot_answers_as_its_native_plugin() {
    parity_of(&COPILOT);
}

#[test]
fn cursor_answers_as_its_native_plugin() {
    parity_of(&CURSOR);
}

#[test]
fn gemini_answers_as_its_native_plugin() {
    parity_of(&GEMINI);
}
