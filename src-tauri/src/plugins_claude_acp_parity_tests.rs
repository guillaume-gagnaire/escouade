//! The parity of the harnesses' modules (ESC-347): Claude Code and ACP, played in the host of the app, answer what their
//! native plugins answered, line by line, on the recorded streams of their tests. `tests/fixtures/plugins/parity/` holds
//! what the natives of 6dbd1223 answered: each call whole, and for each line of a stream the kinds of the items and a digest
//! of all of it. `ESCOUADE_PARITY_RECORD=1` and `ESCOUADE_NATIVE_PLUGINS=<a program of then>` write them again.
//!
//! The files of ACP need an agent the host knows: `plugins_acp_tests` plays them, through the runtime.

use crate::plugins::wasm::{Fetcher, Net};
use crate::plugins_wasm_parity_tests::{call, hello, module, root, Hosted, Native, Plugin};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// What a harness said in the recording `file`, one line after the other, as the core gives it to its plugin.
fn said(file: &Path) -> Vec<String> {
    std::fs::read_to_string(file)
        .unwrap_or_else(|e| panic!("{}: {e}", file.display()))
        .lines()
        .filter_map(|l| {
            let v: Value = serde_json::from_str(l).ok()?;
            let by_it = matches!(v["dir"].as_str(), None | Some("s2c" | "in"));
            let msg = if v["msg"].is_null() { &v } else { &v["msg"] };
            by_it.then(|| msg.to_string())
        })
        .collect()
}

/// What a harness's plugin answered the lines `lines` of its agent `agent`: a line each, the kinds of its items and a
/// digest of the whole answer.
fn played(p: &mut dyn Plugin, id: &mut u64, agent: &str, lines: &[String]) -> Value {
    let mut out = Vec::new();
    for line in lines {
        *id += 1;
        let v = call(
            p,
            *id,
            "agent/frames",
            json!({ "agent": agent, "frames": [line] }),
        );
        assert!(v["error"].is_null(), "{line}: {v}");
        let kinds: Vec<&str> = v["result"]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|i| i["op"].as_str().unwrap_or("?"))
            .collect();
        let digest = Sha256::digest(v["result"].to_string().as_bytes());
        let digest: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
        out.push(json!({ "kinds": kinds, "digest": digest }));
    }
    json!(out)
}

/// The calls `calls`, each answered whole.
fn called(p: &mut dyn Plugin, id: &mut u64, calls: &[(&str, Value)]) -> Value {
    let mut out = Vec::new();
    for (method, params) in calls {
        *id += 1;
        let mut v = call(p, *id, method, params.clone());
        // What an interruption answers comes from a map, in no order, in either plugin.
        if let Some(writes) = v["result"]["writes"].as_array_mut() {
            writes.sort_by_key(Value::to_string);
        }
        out.push(json!({ "method": method, "result": v["result"], "error": v["error"] }));
    }
    json!(out)
}

fn recorded(name: &str) -> PathBuf {
    root()
        .join("tests/fixtures/plugins/parity")
        .join(format!("{name}.json"))
}

/// The native's transcript written, or the module's compared to it, line by line.
fn check(name: &str, transcript: impl Fn(&mut dyn Plugin) -> Value) {
    let path = recorded(name);
    if std::env::var_os("ESCOUADE_PARITY_RECORD").is_some() {
        let mut native = Native::start(name);
        let text = serde_json::to_string_pretty(&transcript(&mut native)).unwrap();
        std::fs::write(&path, text + "\n").unwrap();
        return;
    }
    let Some(m) = module(name) else {
        return;
    };
    let net = Net::default();
    let id = format!("dev.escouade.{name}");
    let mut hosted = Hosted::start(m, Fetcher::new(&id, &[], &net));
    let ours = transcript(&mut hosted);
    let theirs: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let flat = |v: &Value| -> Vec<Value> {
        let calls = v["calls"].as_array().unwrap();
        calls
            .iter()
            .flat_map(|c| c.as_array().unwrap().clone())
            .collect()
    };
    let (mine, recorded) = (flat(&ours), flat(&theirs));
    assert_eq!(mine.len(), recorded.len(), "{name}: the calls");
    for (x, y) in mine.iter().zip(&recorded) {
        assert_eq!(x, y, "{name}: {}", y["method"]);
    }
    let (a, b) = (
        ours["streams"].as_object().unwrap(),
        theirs["streams"].as_object().unwrap(),
    );
    assert_eq!(a.len(), b.len());
    for (file, lines) in b {
        let ours = a[file].as_array().unwrap();
        let theirs = lines.as_array().unwrap();
        assert_eq!(ours.len(), theirs.len(), "{file}");
        for (i, (x, y)) in ours.iter().zip(theirs).enumerate() {
            assert_eq!(x, y, "{name}, {file}, line {i}");
        }
    }
}

/// Every stream a test of Claude Code holds: real runs, and its long turn.
fn claude_streams() -> Vec<PathBuf> {
    let fixtures = root().join("tests/fixtures");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&fixtures)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.starts_with("real-run-") && name.ends_with(".jsonl")
        })
        .collect();
    files.sort();
    files.push(fixtures.join("claude/long-turn.jsonl"));
    assert!(files.len() >= 9);
    files
}

fn name_of(file: &Path) -> String {
    file.file_name().unwrap().to_string_lossy().into_owned()
}

fn claude_transcript(p: &mut dyn Plugin) -> Value {
    let a = "a";
    let mut greeting = hello();
    greeting["lang"] = json!("fr");
    let attach = json!({ "agent": a, "cwd": "C:\\wt", "model": "opus", "effort": "high", "mode": "default", "lang": "fr" });
    let mut launch = attach.clone();
    launch["start"] = json!({ "kind": "new" });
    launch["appendPrompt"] = json!("Le protocole\ndu ticket");
    launch["readDirs"] = json!(["C:\\files"]);
    let mut id = 0;
    let mut calls = vec![called(
        p,
        &mut id,
        &[
            ("initialize", greeting),
            ("agent/attach", attach.clone()),
            ("engine/launch", launch),
            (
                "agent/sendMessage",
                json!({ "agent": a, "uuid": "u1", "text": "bonjour" }),
            ),
            ("agent/setModel", json!({ "agent": a, "value": "sonnet" })),
            ("agent/setEffort", json!({ "agent": a, "value": "low" })),
            ("agent/setMode", json!({ "agent": a, "value": "plan" })),
            (
                "agent/interrupt",
                json!({ "agent": a, "cancelQueued": true }),
            ),
            ("agent/readUsage", json!({ "agent": a })),
            ("agent/rename", json!({ "agent": a, "value": "Un titre" })),
            ("agent/stopTask", json!({ "agent": a, "task": "t1" })),
            ("agent/taskOutput", json!({ "agent": a, "task": "t1" })),
            (
                "agent/answerPermission",
                json!({ "agent": a, "request": "r1", "allow": true, "call": "c1", "input": { "x": 1 } }),
            ),
            (
                "agent/answerQuestion",
                json!({ "agent": a, "request": "r2", "call": "c2", "input": { "x": 1 }, "answers": {} }),
            ),
            ("agent/nothing", json!({ "agent": a })),
            ("agent/setModel", json!({ "agent": "nobody", "value": "x" })),
        ],
    )];
    let mut streams = serde_json::Map::new();
    // The answers to what it asked: `ccm_` and the order of the calls above.
    let answers: Vec<String> = [
        json!({ "subtype": "success", "request_id": "ccm_1", "response": { "commands": [{ "name": "compact" }], "models": [{ "value": "opus" }] } }),
        json!({ "subtype": "success", "request_id": "ccm_2", "response": {} }),
        json!({ "subtype": "error", "request_id": "ccm_3", "error": "refusé" }),
        json!({ "subtype": "success", "request_id": "ccm_6", "response": { "rate_limits": { "five_hour": { "utilization": 0.5 } } } }),
    ]
    .iter()
    .map(|r| json!({ "type": "control_response", "response": r }).to_string())
    .collect();
    streams.insert("answers".into(), played(p, &mut id, a, &answers));
    for file in claude_streams() {
        streams.insert(name_of(&file), played(p, &mut id, a, &said(&file)));
    }
    calls.push(called(
        p,
        &mut id,
        &[("agent/detach", json!({ "agent": a }))],
    ));
    json!({ "calls": calls, "streams": streams })
}

/// The harness a plugin of the repository declares.
fn decl_of(folder: &str) -> Value {
    let file = root()
        .join("plugins")
        .join(folder)
        .join("escouade-plugin.json");
    let v: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    v["contributes"]["harnesses"][0].clone()
}

fn acp_transcript(p: &mut dyn Plugin) -> Value {
    let mut id = 0;
    let mut greeting = hello();
    greeting["lang"] = json!("fr");
    let mut calls = vec![called(p, &mut id, &[("initialize", greeting)])];
    let mut streams = serde_json::Map::new();
    // Goose has no recording: Cursor's, through its description, which knows none of Cursor's own.
    for (folder, harness) in [
        ("goose", "cursor"),
        ("cursor", "cursor"),
        ("mistral", "vibe"),
        ("gemini", "gemini"),
    ] {
        let a = format!("agent-{folder}");
        let attach = json!({
            "agent": a, "cwd": "C:\\wt", "model": "default", "mode": "default", "lang": "fr", "described": decl_of(folder),
        });
        let mut launch = attach.clone();
        launch["start"] = json!({ "kind": "new" });
        calls.push(called(
            p,
            &mut id,
            &[("agent/attach", attach), ("engine/launch", launch)],
        ));
        let cases = crate::engine::bench::replay::cases_of(harness);
        let mut names: Vec<PathBuf> = std::fs::read_dir(&cases)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
        names.sort();
        // Gemini's plan is read at the path of the recording: what a disk holds there is no parity.
        names.retain(|n| !n.ends_with("plan-exit-approval.json"));
        for case in names {
            let c: Value = serde_json::from_slice(&std::fs::read(&case).unwrap()).unwrap();
            let file = case.parent().unwrap().join(c["flux"].as_str().unwrap());
            let key = format!("{folder}/{}", name_of(&case));
            streams.insert(key, played(p, &mut id, &a, &said(&file)));
        }
        calls.push(called(
            p,
            &mut id,
            &[
                (
                    "agent/sendMessage",
                    json!({ "agent": a, "uuid": "u1", "text": "bonjour" }),
                ),
                ("agent/setModel", json!({ "agent": a, "value": "sonnet" })),
                (
                    "agent/setMode",
                    json!({ "agent": a, "value": "acceptEdits" }),
                ),
                ("agent/setEffort", json!({ "agent": a, "value": "low" })),
                ("agent/interrupt", json!({ "agent": a })),
                ("agent/readUsage", json!({ "agent": a })),
                (
                    "agent/answerPermission",
                    json!({ "agent": a, "request": "nothing", "allow": true }),
                ),
                ("agent/nothing", json!({ "agent": a })),
                ("agent/detach", json!({ "agent": a })),
            ],
        ));
    }
    json!({ "calls": calls, "streams": streams })
}

#[test]
fn claude_answers_as_its_native_plugin_line_by_line() {
    check("claude", claude_transcript);
}

#[test]
fn acp_answers_as_its_native_plugin_line_by_line() {
    check("acp", acp_transcript);
}

/// What a message may carry (`core::MAX_TOTAL`, in base64): sent to Claude Code, then replayed by it (`--replay-user-messages`).
#[test]
fn the_biggest_message_passes_through_the_module() {
    let Some(m) = module("claude") else {
        return;
    };
    let net = Net::default();
    let manifest: Value = serde_json::from_str(escouade_plugin_claude::MANIFEST).unwrap();
    let asked = crate::plugins::manifest::Limits {
        memory_bytes: manifest["limits"]["memoryBytes"].as_u64(),
        ..Default::default()
    };
    let limits = crate::plugins::wasm::Limits::default().asked(asked);
    let fetcher = Fetcher::new("dev.escouade.claude-code", &[], &net);
    let mut p = Hosted::within(m, fetcher, limits);
    let attach =
        json!({ "agent": "a", "cwd": "C:\\wt", "model": "opus", "mode": "default", "lang": "fr" });
    assert!(call(&mut p, 1, "initialize", hello())["result"].is_object());
    assert!(call(&mut p, 2, "agent/attach", attach)["error"].is_null());
    let data = "A".repeat(crate::core::MAX_TOTAL / 3 * 4);
    let content = json!([
        { "type": "document", "source": { "type": "base64", "media_type": "application/pdf", "data": data } },
        { "type": "text", "text": "lis ce PDF" },
    ]);
    let begun = std::time::Instant::now();
    let sent = call(
        &mut p,
        3,
        "agent/sendMessage",
        json!({ "agent": "a", "uuid": "u1", "text": "lis ce PDF", "content": content }),
    );
    let line = sent["result"]["writes"][0].as_str().expect("its line");
    assert!(line.len() > data.len(), "{}", line.len());
    println!("sent in {:?}", begun.elapsed());
    let begun = std::time::Instant::now();
    let told = call(
        &mut p,
        4,
        "agent/frames",
        json!({ "agent": "a", "frames": [line] }),
    );
    assert!(told["error"].is_null(), "{}", told["error"]);
    println!("replayed in {:?}", begun.elapsed());
}
