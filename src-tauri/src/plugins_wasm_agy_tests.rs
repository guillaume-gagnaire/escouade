//! Antigravity CLI in the host of the app (ESC-348): its module says what its native plugin said on the streams of `agy`
//! 1.4.9 (`tests/fixtures/agy/`), each with a transcript made from it in the agent's home, the cards answered. What the native
//! said is `tests/fixtures/plugins/parity/antigravity.json`: `ESCOUADE_PARITY_RECORD=1` and
//! `ESCOUADE_NATIVE_PLUGINS=<a program of before its port>` write it again.
//!
//! And what no native did: a transcript written late holds nothing inside a turn, what follows comes by `agent/items`.

use super::*;
use crate::plugins::files::{Access, AgentPlace, Agents};
use crate::plugins::manifest::{read as read_manifest, Hand};

const STREAMS: [&str; 13] = [
    "01-simple-turn",
    "02-stream-input-two-turns",
    "03-permission-denied-print",
    "04-write-denied-print",
    "05-tools-allowed-by-rules",
    "06-hook-allow-not-enough",
    "07-mcp-denied-print",
    "08-mcp-allowed-project-config",
    "09-ask-question-skipped-print",
    "10-accept-edits-write",
    "11-plan-mode-artifact",
    "12-subagent-tasks",
    "13-rule-hot-reload-two-turns",
];

/// The home of an agent of the test, wherever the system puts it.
fn home_of(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("esc348-{name}-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

/// The module, the home `home` its agent `a`'s, its permissions those of its manifest.
fn hosted(module: wasmtime::Module, home: &Path) -> Hosted {
    let text = std::fs::read_to_string(root().join("plugins/antigravity/escouade-plugin.json"));
    let manifest = read_manifest(&text.unwrap(), Hand::Builtin)
        .manifest
        .expect("its manifest");
    let agents: Agents = Arc::default();
    agents.lock().insert(
        "a".into(),
        AgentPlace {
            cwd: home.to_path_buf(),
            home: Some(home.to_path_buf()),
            project: "p".into(),
            ..AgentPlace::default()
        },
    );
    let access = Access::new(&manifest, home, home.join("temp"), agents);
    let calls: wasm::Calls = Arc::new(move |m, p| access.serve(m, p, &|_, _| {}));
    let fetcher = Fetcher::new("dev.escouade.antigravity-cli", &[], &Net::default());
    Hosted::start_with(module, fetcher, Some(calls))
}

/// What the agent is given: a folder that reads the same on every machine, as a string.
fn attach(home: &Path, mode: &str) -> Value {
    json!({
        "agent": "a", "cwd": "C:\\wt", "model": "gemini-3.8-flash", "effort": "high", "mode": mode, "lang": "fr",
        "home": home.to_string_lossy(),
    })
}

fn lines_of(stream: &str, home: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(root().join(format!("tests/fixtures/agy/{stream}.jsonl")));
    let home = serde_json::to_string(&home.to_string_lossy()).unwrap();
    let home = home.trim_matches('"').to_string();
    text.unwrap()
        .lines()
        .map(|l| l.replace("C:\\\\tmp\\\\esc269\\\\home1", &home))
        .map(|l| under_home(&l, &home, if cfg!(windows) { "\\\\" } else { "/" }))
        .collect()
}

/// `line` with what follows `home` by `separator`, as JSON writes it: the streams were recorded under Windows, and
/// elsewhere the plan would be written and read beside the home (the CI's Linux, 2026-11-06). Under Windows the home is a
/// `\\?\` path, which takes no forward slash. The folder of the work stays as it was recorded: it is only words, which
/// the recording holds.
fn under_home(line: &str, home: &str, separator: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(at) = rest.find(home) {
        let (before, after) = rest.split_at(at + home.len());
        let end = after.find('"').unwrap_or(after.len());
        out.push_str(before);
        out.push_str(&after[..end].replace("\\\\", separator));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

#[test]
fn a_stream_of_windows_reads_its_home_by_the_slashes_of_this_system_and_nothing_else() {
    let line = r#"{"Path":"/home/x\\.gemini\\brain\\p.md","cwd":"C:\\tmp\\work"}"#;
    assert_eq!(
        under_home(line, "/home/x", "/"),
        r#"{"Path":"/home/x/.gemini/brain/p.md","cwd":"C:\\tmp\\work"}"#
    );
    assert_eq!(under_home(line, "/home/x", "\\\\"), line);
    assert_eq!(under_home(line, "/elsewhere", "/"), line);
}

/// The conversation of a stream, and its transcript as `agy` would have written it beside it: for each step that ends a
/// text, its reasoning and the calls that follow; for a call, how it ended, a write of the stream `05` refused there (ESC-269).
fn transcript_of(lines: &[String], home: &Path, stream: &str) {
    let events: Vec<Value> = lines
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let id = events[0]["conversation_id"].as_str().unwrap();
    let mut out = String::new();
    for (at, e) in events.iter().enumerate() {
        let s = &e["step_update"];
        if s["state"] != "DONE" {
            continue;
        }
        if s["step_type"] == "agent_response" {
            let calls: Vec<Value> = events[at + 1..]
                .iter()
                .map(|e| &e["step_update"])
                .take_while(|s| s["step_type"] != "agent_response")
                .filter(|s| s["state"] == "ACTIVE" && s["tool_name"].is_string())
                .map(|s| json!({ "name": s["tool_name"], "args": s["tool_info"]["parameters"] }))
                .collect();
            let thinking = format!("pensée {}", s["step_index"]);
            let line =
                json!({ "step_index": s["step_index"], "thinking": thinking, "tool_calls": calls });
            out.push_str(&format!("{line}\n"));
        } else if s["tool_name"].is_string() {
            let refused = s["tool_name"] == "write_file" && stream.starts_with("05");
            let status = if refused { "ERROR" } else { "DONE" };
            let line = json!({ "step_index": s["step_index"], "status": status, "error": "denied permission" });
            out.push_str(&format!(
                "{line}
"
            ));
        }
        let path = s["tool_info"]["parameters"]["Path"].as_str();
        if let Some(plan) = path.filter(|p| p.contains("brain") && p.ends_with(".md")) {
            std::fs::create_dir_all(Path::new(plan).parent().unwrap()).unwrap();
            std::fs::write(plan, "# Le plan\n").unwrap_or_else(|e| panic!("{plan:?}: {e}"));
        }
    }
    let file = home.join(".gemini/antigravity-cli/brain").join(id);
    let file = file.join(".system_generated/logs");
    std::fs::create_dir_all(&file).unwrap();
    std::fs::write(file.join("transcript_full.jsonl"), out).unwrap();
}

/// `v` without the tools of the user, wherever a launch says them: the core's, in a module.
fn without_tools(v: &mut Value) {
    match v {
        Value::Object(o) => {
            if let Some(Value::Array(env)) = o.get_mut("env") {
                let tool = |e: &Value| {
                    ["GIT_CONFIG_GLOBAL", "CARGO_HOME", "RUSTUP_HOME"]
                        .contains(&e["name"].as_str().unwrap_or_default())
                };
                env.retain(|e| !tool(e));
            }
            o.values_mut().for_each(without_tools);
        }
        Value::Array(a) => a.iter_mut().for_each(without_tools),
        _ => {}
    }
}

/// `v` without what differs by machine: its home, its slashes, the tools of the user.
fn plain(v: &Value, home: &Path) -> Value {
    let mut v = v.clone();
    without_tools(&mut v);
    let home = serde_json::to_string(&home.to_string_lossy()).unwrap();
    let text = v.to_string().replace(home.trim_matches('"'), "<home>");
    // A file in JSON in JSON doubles them twice.
    let text = text.replace("\\\\", "/").replace("\\", "/");
    serde_json::from_str(&text).unwrap_or(Value::String(text))
}

/// A stream played whole, what the plugin answers to each call, in its order; its cards answered, allowed.
fn played(p: &mut dyn Plugin, stream: &str, home: &Path) -> Value {
    let lines = lines_of(stream, home);
    transcript_of(&lines, home, stream);
    let mode = if stream.starts_with("11") {
        "plan"
    } else {
        "default"
    };
    let mut id = 1;
    let mut said = Vec::new();
    let mut ask = |p: &mut dyn Plugin, method: &str, params: Value| {
        id += 1;
        let a = call(p, id, method, params);
        assert!(a["error"].is_null(), "{stream}: {method}: {a}");
        let result = plain(&a["result"], home);
        said.push(json!({ "method": method, "result": result }));
        a["result"].clone()
    };
    ask(p, "initialize", hello());
    let at = attach(home, mode);
    ask(p, "agent/attach", at.clone());
    let mut launch = at;
    launch["start"] = json!({ "kind": "new" });
    ask(p, "engine/launch", launch);
    let frames = |l: &String| json!({ "agent": "a", "frames": [l] });
    ask(p, "agent/frames", frames(&lines[0]));
    let text = json!({ "agent": "a", "text": "va", "uuid": "u1" });
    ask(p, "agent/sendMessage", text);
    for l in &lines[1..] {
        let items = ask(p, "agent/frames", frames(l));
        for i in items["items"].as_array().into_iter().flatten() {
            let request = i["request"].as_str().unwrap_or_default();
            match i["op"].as_str().unwrap_or_default() {
                "permission" | "permissionCard" => {
                    let a =
                        json!({ "agent": "a", "request": request, "allow": true, "always": true });
                    ask(p, "agent/answerPermission", a);
                }
                "question" | "questionCard" => {
                    let a = json!({ "agent": "a", "request": request, "answers": {} });
                    ask(p, "agent/answerQuestion", a);
                }
                _ => {}
            }
        }
    }
    ask(p, "agent/interrupt", json!({ "agent": "a" }));
    Value::Array(said)
}

#[tokio::test(flavor = "multi_thread")]
async fn antigravity_says_what_its_native_plugin_said_on_its_streams() {
    let Some(agy) = module("antigravity") else {
        return;
    };
    let path = recording("antigravity");
    if std::env::var_os("ESCOUADE_PARITY_RECORD").is_some() {
        let mut was = serde_json::Map::new();
        for stream in STREAMS {
            let home = home_of("native");
            let mut native = Native::start("agy");
            was.insert(stream.into(), played(&mut native, stream, &home));
        }
        std::fs::write(&path, serde_json::to_string_pretty(&was).unwrap()).unwrap();
    }
    let was: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut items = 0;
    for stream in STREAMS {
        let home = home_of("module");
        let mut plugin = hosted(agy.clone(), &home);
        let now = played(&mut plugin, stream, &home);
        for (a, b) in now
            .as_array()
            .unwrap()
            .iter()
            .zip(was[stream].as_array().unwrap())
        {
            assert_eq!(a, b, "{stream}: {}", a["method"]);
            items += a["result"]["items"].as_array().map_or(0, Vec::len);
        }
        assert_eq!(now, was[stream], "{stream}");
    }
    assert!(items > 100, "{items} items");
}

fn step_done(step: u64) -> String {
    json!({ "event": "step_update", "step_update": {
        "step_index": step, "state": "DONE", "step_type": "agent_response", "usage": { "input_tokens": 1 },
    } })
    .to_string()
}

fn tool(step: u64, state: &str) -> String {
    json!({ "event": "step_update", "step_update": {
        "step_index": step, "state": state, "step_type": "tool", "tool_name": "run_command",
        "tool_info": { "parameters": {}, "output": "ok" },
    } })
    .to_string()
}

fn opened(p: &mut dyn Plugin, home: &Path) {
    assert!(call(p, 1, "initialize", hello())["result"].is_object());
    assert!(call(p, 2, "agent/attach", attach(home, "default"))["error"].is_null());
    let mut launch = attach(home, "default");
    launch["start"] = json!({ "kind": "new" });
    assert!(call(p, 3, "engine/launch", launch)["error"].is_null());
    let init = json!({ "event": "init", "conversation_id": "conv-1" }).to_string();
    call(
        p,
        4,
        "agent/frames",
        json!({ "agent": "a", "frames": [init] }),
    );
    call(
        p,
        5,
        "agent/sendMessage",
        json!({ "agent": "a", "text": "va", "uuid": "u1" }),
    );
}

/// The items its plugin says unasked, up to the end of the turn.
fn pushed(p: &mut dyn Plugin, within: Duration) -> Vec<Value> {
    let until = Instant::now() + within;
    let mut items = Vec::new();
    while !items.iter().any(|i: &Value| i["op"] == "turn") {
        let left = until.saturating_duration_since(Instant::now());
        let line = p
            .recv(left)
            .expect("agent/items before the end of its patience");
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["method"], "agent/items", "{v}");
        assert_eq!(v["params"]["agent"], "a");
        items.extend(v["params"]["items"].as_array().unwrap().iter().cloned());
    }
    items
}

#[test]
fn a_transcript_written_late_comes_behind_what_it_held_by_agent_items() {
    let Some(agy) = module("antigravity") else {
        return;
    };
    let home = home_of("late");
    let mut plugin = hosted(agy, &home);
    opened(&mut plugin, &home);
    let result = json!({ "event": "result", "result": { "status": "SUCCESS" } }).to_string();
    let frames = json!({ "agent": "a", "frames": [step_done(3), tool(4, "ACTIVE"), tool(4, "DONE"), result] });
    let at = Instant::now();
    let answer = call(&mut plugin, 6, "agent/frames", frames);
    assert!(
        at.elapsed() < Duration::from_millis(300),
        "it waited inside its turn"
    );
    assert_eq!(answer["result"]["items"], json!([]), "{answer}");
    // Each batch that comes meanwhile goes behind it.
    std::thread::sleep(Duration::from_millis(120));
    let dir = home.join(".gemini/antigravity-cli/brain/conv-1/.system_generated/logs");
    std::fs::create_dir_all(&dir).unwrap();
    let line = json!({ "step_index": 3, "tool_calls": [{ "name": "run_command", "args": { "CommandLine": "echo tard" } }] });
    std::fs::write(dir.join("transcript_full.jsonl"), format!("{line}\n")).unwrap();
    let items = pushed(&mut plugin, Duration::from_secs(5));
    let text = serde_json::to_string(&items).unwrap();
    assert!(text.contains("echo tard"), "the arguments it read: {text}");
    assert_eq!(items.last().unwrap()["op"], "turn", "{text}");
}

#[test]
fn a_transcript_never_written_ends_the_wait_after_its_patience() {
    let Some(agy) = module("antigravity") else {
        return;
    };
    let home = home_of("never");
    let mut plugin = hosted(agy, &home);
    opened(&mut plugin, &home);
    let result = json!({ "event": "result", "result": { "status": "SUCCESS" } }).to_string();
    let frames = json!({ "agent": "a", "frames": [step_done(3), result] });
    let at = Instant::now();
    call(&mut plugin, 6, "agent/frames", frames);
    let items = pushed(&mut plugin, Duration::from_secs(5));
    assert!(
        at.elapsed() >= Duration::from_millis(500),
        "{:?}",
        at.elapsed()
    );
    assert_eq!(items.last().unwrap()["op"], "turn");
}
