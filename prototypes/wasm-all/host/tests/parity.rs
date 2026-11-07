//! The two modules answer as their native plugin does, and the host applies what the manifest grants (ESC-330).
//! `sh ../reproducible.sh` first, and `cargo build --release --bins` for the natives: `cargo test --release`.

use escouade_wasm_all::*;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const INIT: &str = "initialize";

fn start_linear(fake: &FakeLinear) -> (Wasm, Process) {
    let mut wasm = Wasm::start(load("linear"), linear_grants(&fake.base));
    let mut native = Process::start(&native_file("native-linear"), &[]);
    assert!(call_plugin(&mut wasm, 1, INIT, json!({}))["result"].is_object());
    assert!(call_plugin(&mut native, 1, INIT, json!({}))["result"].is_object());
    (wasm, native)
}

#[test]
fn linear_answers_as_its_native_plugin() {
    let fake = FakeLinear::start(50);
    let (mut wasm, mut native) = start_linear(&fake);
    for method in ["account/me", "tickets/issues", "tickets/states"] {
        let extra = |mut p: Value| {
            p["point"] = json!("ticketSource");
            p
        };
        let a = call_plugin(&mut wasm, 2, method, extra(linear_params(None)));
        let b = call_plugin(&mut native, 2, method, extra(linear_params(Some(&fake.base))));
        assert!(a["error"].is_null(), "{method}: {a}");
        assert_eq!(a["result"], b["result"], "{method}");
    }
}

#[test]
fn a_slow_fetch_holds_back_neither_the_next_nor_the_thread() {
    let fake = FakeLinear::start(1);
    let (mut wasm, _native) = start_linear(&fake);
    // The first is held 400 ms by the fake (`slow400` in its body, a search text), the second is not.
    let mut p = linear_params(None);
    p["query"]["text"] = json!("slow400");
    let started = Instant::now();
    let write = |id: u64, params: &Value| json!({ "jsonrpc": "2.0", "id": id, "method": "tickets/issues", "params": params }).to_string();
    wasm.send(&write(10, &p));
    wasm.send(&write(11, &linear_params(None)));
    let mut order = Vec::new();
    while order.len() < 2 {
        let line = wasm.recv(Duration::from_secs(5)).expect("an answer");
        let v: Value = serde_json::from_str(&line).unwrap();
        if v["method"] == "secrets/get" {
            wasm.send(&json!({ "jsonrpc": "2.0", "id": v["id"], "result": "k" }).to_string());
        } else if v.get("method").is_none() {
            order.push((v["id"].as_u64().unwrap(), started.elapsed()));
        }
    }
    assert_eq!(order[0].0, 11, "the quick one first: {order:?}");
    assert!(order[0].1 < Duration::from_millis(300), "{order:?}");
    assert!(order[1].1 >= Duration::from_millis(400), "{order:?}");
}

#[test]
fn a_host_outside_the_manifest_is_refused() {
    let fake = FakeLinear::start(1);
    let mut grants = linear_grants(&fake.base);
    grants.network.clear();
    let mut wasm = Wasm::start(load("linear"), grants);
    call_plugin(&mut wasm, 1, INIT, json!({}));
    let a = call_plugin(&mut wasm, 2, "tickets/issues", linear_params(None));
    let said = a["error"]["message"].as_str().unwrap_or_default();
    assert!(said.contains("not in permissions.network"), "{a}");
}

/// The server-to-client lines of a recording, as the harness wrote them.
fn said(file: &str) -> Vec<(u64, String)> {
    let path = fixtures().join("codex").join(file);
    std::fs::read_to_string(path)
        .expect("a recording")
        .lines()
        .filter_map(|l| {
            let v: Value = serde_json::from_str(l).ok()?;
            (v["dir"] == "s2c").then(|| (v["t"].as_u64().unwrap_or(0), v["msg"].to_string()))
        })
        .collect()
}

/// The ids of a plugin's own requests start at a number of its own: what is compared is the rest.
fn without_ids(v: &Value) -> Value {
    let text = v.to_string();
    let v: Value = serde_json::from_str(&text).unwrap();
    fn walk(v: Value) -> Value {
        match v {
            Value::Object(o) => o.into_iter().map(|(k, v)| (k.clone(), if k == "line" { Value::Null } else { walk(v) })).collect(),
            Value::Array(a) => a.into_iter().map(walk).collect(),
            other => other,
        }
    }
    walk(v)
}

#[test]
fn codex_translates_a_whole_turn_as_its_native_plugin() {
    let mut wasm = Wasm::start(load("codex"), Grants::default());
    let mut native = Process::start(&native_file("native-codex"), &[]);
    let attach = json!({ "agent": "a", "cwd": "C:\\wt", "model": "gpt-6-luna", "mode": "default", "lang": "fr" });
    let launch = json!({ "agent": "a", "cwd": "C:\\wt", "model": "gpt-6-luna", "mode": "default", "lang": "fr", "start": { "kind": "new" } });
    for p in [&mut wasm as &mut dyn Plugin, &mut native] {
        assert!(call_plugin(p, 1, INIT, json!({}))["result"].is_object());
        assert!(call_plugin(p, 2, "agent/attach", attach.clone())["error"].is_null());
        assert!(call_plugin(p, 3, "engine/launch", launch.clone())["error"].is_null());
    }
    let (mut n, mut items) = (0, 0);
    for file in ["full-turn.jsonl", "plan-mode.jsonl"] {
        for (i, (_, line)) in said(file).iter().enumerate() {
            let p = json!({ "agent": "a", "frames": [line] });
            let a = call_plugin(&mut wasm, 10 + i as u64, "agent/frames", p.clone());
            let b = call_plugin(&mut native, 10 + i as u64, "agent/frames", p);
            assert!(a["error"].is_null(), "{a}");
            assert_eq!(without_ids(&a["result"]), without_ids(&b["result"]), "{file}, line {i}: {line}");
            n += 1;
            items += a["result"]["items"].as_array().map_or(0, Vec::len);
        }
    }
    assert!(n > 300 && items > 200, "{n} lines, {items} items");
}

#[test]
fn a_file_is_read_in_the_declared_folder_alone() {
    let dir = std::env::temp_dir().join(format!("esc330-{}", std::process::id()));
    let (granted, other) = (dir.join("codex-home"), dir.join("elsewhere"));
    std::fs::create_dir_all(&granted).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    for d in [&granted, &other] {
        std::fs::write(d.join("config.toml"), "[mcp_servers.escouade]\nurl = \"x\"\n").unwrap();
    }
    let off = |home: &std::path::Path, grants: Grants| {
        let mut wasm = Wasm::start(load("codex"), grants);
        call_plugin(&mut wasm, 1, INIT, json!({}));
        call_plugin(&mut wasm, 2, "agent/attach", json!({ "agent": "a", "cwd": "x" }));
        let launch = json!({
            "agent": "a", "cwd": "x", "model": "m", "mode": "default", "start": { "kind": "new" },
            "home": home.to_string_lossy(),
        });
        let a = call_plugin(&mut wasm, 3, "engine/launch", launch);
        a["result"]["args"].to_string().contains("mcp_servers.escouade.enabled=false")
    };
    let grants = Grants { files: vec![granted.clone()], ..Grants::default() };
    assert!(off(&granted, grants.clone()), "read in its folder");
    assert!(!off(&other, grants), "refused outside it");
    let _ = std::fs::remove_dir_all(&dir);
}
