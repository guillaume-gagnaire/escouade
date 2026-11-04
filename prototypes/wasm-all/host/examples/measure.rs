//! The measures of `run.wasm` against the native plugin (ESC-330, docs/PLUGINS.md §8.9), as INT-0 reads them: throughput,
//! 99th percentile at the fastest and at the recorded pace, memory, start, size. The same lines to the same plugin code,
//! by a process's pipes and by a module's turns. It stops at the plugin's answer: what the core does of it is INT-0's.
//!
//!   cargo run --release --example measure -- [rounds]

use escouade_wasm_all::*;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use wasmtime::Module;

const LONGEST_GAP: u64 = 2_000;
const BATCH_BYTES: usize = 64 * 1024;

fn pct(sorted: &[f64], p: f64) -> f64 {
    sorted[((sorted.len() as f64 - 1.0) * p).round() as usize]
}

fn stats(mut d: Vec<f64>) -> (f64, f64, f64) {
    d.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (pct(&d, 0.5), pct(&d, 0.99), d[d.len() - 1])
}

fn said(file: &str) -> Vec<(u64, String)> {
    let text = std::fs::read_to_string(fixtures().join("codex").join(file)).expect("a recording");
    let all: Vec<(u64, String)> = text
        .lines()
        .filter_map(|l| {
            let v: Value = serde_json::from_str(l).ok()?;
            (v["dir"] == "s2c").then(|| (v["t"].as_u64().unwrap_or(0), v["msg"].to_string()))
        })
        .collect();
    let first = all.first().map_or(0, |l| l.0);
    all.into_iter().map(|(t, l)| (t - first, l)).collect()
}

fn attach(p: &mut dyn Plugin, id: u64, agent: &str) {
    let a = json!({ "agent": agent, "cwd": "C:\\wt", "model": "gpt-6-luna", "mode": "default", "lang": "fr" });
    let l = json!({ "agent": agent, "cwd": "C:\\wt", "model": "gpt-6-luna", "mode": "default", "lang": "fr", "start": { "kind": "new" } });
    call_plugin(p, id, "agent/attach", a);
    call_plugin(p, id + 1, "engine/launch", l);
}

fn frames(p: &mut dyn Plugin, id: u64, agent: &str, lines: &[&String]) {
    let a = call_plugin(p, id, "agent/frames", json!({ "agent": agent, "frames": lines }));
    assert!(a["error"].is_null(), "{a}");
}

/// Each line alone, as soon as the one before is answered.
fn fastest(p: &mut dyn Plugin, lines: &[(u64, String)], rounds: usize) -> Value {
    let (mut delays, mut bytes, mut id) = (Vec::new(), 0usize, 100u64);
    let started = Instant::now();
    for r in 0..rounds {
        let agent = format!("f{r}");
        attach(p, id, &agent);
        id += 2;
        for (_, l) in lines {
            let at = Instant::now();
            frames(p, id, &agent, &[l]);
            delays.push(at.elapsed().as_secs_f64() * 1e6);
            bytes += l.len();
            id += 1;
        }
        call_plugin(p, id, "agent/detach", json!({ "agent": agent }));
        id += 1;
    }
    let secs = started.elapsed().as_secs_f64();
    let n = delays.len();
    let (median, p99, max) = stats(delays);
    json!({ "lines": n, "linesPerSecond": n as f64 / secs, "mbPerSecond": bytes as f64 / 1e6 / secs, "medianUs": median, "p99Us": p99, "maxUs": max })
}

/// In batches of at most 64 KiB, as a harness at full speed comes (`BATCH_BYTES`).
fn batched(p: &mut dyn Plugin, lines: &[(u64, String)], rounds: usize) -> Value {
    let (mut n, mut id) = (0usize, 1_000_000u64);
    let started = Instant::now();
    for r in 0..rounds {
        let agent = format!("b{r}");
        attach(p, id, &agent);
        id += 2;
        let mut batch: Vec<&String> = Vec::new();
        let mut size = 0;
        for (_, l) in lines {
            if size + l.len() > BATCH_BYTES {
                frames(p, id, &agent, &batch);
                id += 1;
                batch.clear();
                size = 0;
            }
            size += l.len();
            batch.push(l);
            n += 1;
        }
        if !batch.is_empty() {
            frames(p, id, &agent, &batch);
            id += 1;
        }
        call_plugin(p, id, "agent/detach", json!({ "agent": agent }));
        id += 1;
    }
    json!({ "linesPerSecond": n as f64 / started.elapsed().as_secs_f64() })
}

/// Each line when it came, a silence longer than 2 s shortened to it (as INT-0).
fn paced(p: &mut dyn Plugin, lines: &[(u64, String)]) -> Value {
    attach(p, 5_000_000, "p");
    let (start, mut shift, mut last, mut delays) = (Instant::now(), 0u64, 0u64, Vec::new());
    for (i, (t, l)) in lines.iter().enumerate() {
        shift += t.saturating_sub(last).saturating_sub(LONGEST_GAP);
        last = *t;
        let due = start + Duration::from_millis(t - shift);
        while let Some(wait) = due.checked_duration_since(Instant::now()) {
            if wait > Duration::from_millis(3) {
                std::thread::sleep(wait - Duration::from_millis(2));
            } else {
                std::hint::spin_loop();
            }
        }
        let at = Instant::now();
        frames(p, 5_000_100 + i as u64, "p", &[l]);
        delays.push(at.elapsed().as_secs_f64() * 1e6);
    }
    let secs = start.elapsed().as_secs_f64();
    call_plugin(p, 5_000_000 - 1, "agent/detach", json!({ "agent": "p" }));
    let (median, p99, max) = stats(delays);
    json!({ "seconds": secs, "medianUs": median, "p99Us": p99, "maxUs": max })
}

fn codex(p: &mut dyn Plugin, rounds: usize) -> Value {
    let mut lines = said("full-turn.jsonl");
    let end = lines.last().map_or(0, |x| x.0);
    lines.extend(said("plan-mode.jsonl").into_iter().map(|(t, l)| (t + end, l)));
    call_plugin(p, 1, "initialize", json!({}));
    let fast = fastest(p, &lines, rounds);
    let batch = batched(p, &lines, rounds);
    let pace = paced(p, &lines);
    json!({ "fastest": fast, "batched": batch, "paced": pace, "memory": p.memory() })
}

/// `tickets/issues`, a page of 50: its two fetches and its secret.
fn linear(p: &mut dyn Plugin, native: Option<&str>, calls: usize) -> Value {
    call_plugin(p, 1, "initialize", json!({}));
    let params = linear_params(native);
    let (mut delays, started) = (Vec::new(), Instant::now());
    for i in 0..calls {
        let at = Instant::now();
        let a = call_plugin(p, 10 + i as u64, "tickets/issues", params.clone());
        assert!(a["error"].is_null(), "{a}");
        delays.push(at.elapsed().as_secs_f64() * 1e6);
    }
    let per = calls as f64 / started.elapsed().as_secs_f64();
    let (median, p99, max) = stats(delays);
    let mut paced_delays = Vec::new();
    for i in 0..calls / 10 {
        std::thread::sleep(Duration::from_millis(20));
        let at = Instant::now();
        call_plugin(p, 1_000_000 + i as u64, "tickets/issues", params.clone());
        paced_delays.push(at.elapsed().as_secs_f64() * 1e6);
    }
    let (pm, pp, _) = stats(paced_delays);
    json!({
        "callsPerSecond": per, "medianUs": median, "p99Us": p99, "maxUs": max,
        "pacedMedianUs": pm, "pacedP99Us": pp, "memory": p.memory(),
    })
}

fn start_native(program: &std::path::Path, n: usize) -> Value {
    let mut d = Vec::new();
    let mut memory = 0;
    for _ in 0..n {
        let at = Instant::now();
        let mut p = Process::start(program, &[]);
        call_plugin(&mut p, 1, "initialize", json!({}));
        d.push(at.elapsed().as_secs_f64() * 1e3);
        memory = p.memory();
    }
    let (median, p99, _) = stats(d);
    json!({ "medianMs": median, "p99Ms": p99, "idleMemory": memory })
}

fn start_wasm(name: &str, grants: Grants, n: usize) -> Value {
    let bytes = std::fs::read(wasm_file(name)).expect("the module");
    let at = Instant::now();
    let module = Module::new(engine(), &bytes).expect("a module");
    let precompile = at.elapsed().as_secs_f64() * 1e3;
    let pre = module.serialize().expect("serialized");
    let at = Instant::now();
    // SAFETY: the bytes this process made just above.
    let module = unsafe { Module::deserialize(engine(), &pre) }.expect("deserialized");
    let load = at.elapsed().as_secs_f64() * 1e3;
    let mut d = Vec::new();
    for _ in 0..n {
        let at = Instant::now();
        let mut p = Wasm::start(module.clone(), grants.clone());
        call_plugin(&mut p, 1, "initialize", json!({}));
        d.push(at.elapsed().as_secs_f64() * 1e3);
    }
    let (median, p99, _) = stats(d);
    // Ten instances more, alive: what the process gains.
    let me = std::process::id();
    let before = memory_of(me);
    let mut alive: Vec<Wasm> = (0..10).map(|_| Wasm::start(module.clone(), grants.clone())).collect();
    for p in &mut alive {
        call_plugin(p, 1, "initialize", json!({}));
    }
    let per = memory_of(me).saturating_sub(before) / 10;
    let mut i = Instance::start(&module, grants).expect("an instance");
    let _ = i.turn(b"", Duration::from_secs(2));
    json!({
        "wasmBytes": bytes.len(), "precompileMs": precompile, "cwasmBytes": pre.len(), "loadMs": load,
        "instanceMedianMs": median, "instanceP99Ms": p99, "workingSetPerInstance": per, "linearMemory": i.memory(),
    })
}

fn main() {
    let rounds: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(20);
    let fake = FakeLinear::start(50);
    let (codex_native, linear_native) = (native_file("native-codex"), native_file("native-linear"));

    let results = json!({
        "start": {
            "codexNative": start_native(&codex_native, 30),
            "codexWasm": start_wasm("codex", Grants::default(), 30),
            "linearNative": start_native(&linear_native, 30),
            "linearWasm": start_wasm("linear", linear_grants(&fake.base), 30),
        },
        "codex": {
            "native": codex(&mut Process::start(&codex_native, &[]), rounds),
            "wasm": codex(&mut Wasm::start(load("codex"), Grants::default()), rounds),
        },
        "linear": {
            "native": linear(&mut Process::start(&linear_native, &[]), Some(&fake.base), 2_000),
            "wasm": linear(&mut Wasm::start(load("linear"), linear_grants(&fake.base)), None, 2_000),
        },
        "size": {
            "codexNative": std::fs::metadata(&codex_native).map(|m| m.len()).unwrap_or(0),
            "linearNative": std::fs::metadata(&linear_native).map(|m| m.len()).unwrap_or(0),
        },
    });
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
}
