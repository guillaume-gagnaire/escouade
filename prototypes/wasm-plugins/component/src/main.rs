//! ESC-319: the same bundles as components (componentize-js 0.23, StarlingMonkey/SpiderMonkey), run by wasmtime 50's
//! Component Model, measured as Javy is. `cargo run --release -p escouade-wasm-component -- <name>.component.wasm…`
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, Instant};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({ path: "wit", world: "plugin" });

struct State {
    wasi: WasiCtx,
    table: ResourceTable,
    sent: Vec<String>,
    limits: StoreLimits,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl escouade::plugin::host::Host for State {
    fn send(&mut self, line: String) {
        self.sent.push(line);
    }
}

struct Run {
    store: Store<State>,
    plugin: Plugin,
    next: u64,
}

impl Run {
    fn turn(&mut self, input: &str) -> Vec<Value> {
        self.plugin.call_turn(&mut self.store, input).unwrap();
        let out = std::mem::take(&mut self.store.data_mut().sent);
        out.concat()
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let line = json!({ "jsonrpc": "2.0", "id": self.next, "method": method, "params": params })
            .to_string()
            + "\n";
        let out = self.turn(&line);
        out.into_iter()
            .find(|m| m["id"] == self.next)
            .map(|m| m["result"].clone())
            .unwrap_or(Value::Null)
    }
}

fn stats(mut v: Vec<Duration>) -> String {
    v.sort();
    let n = v.len();
    format!(
        "median {:.1} µs, p99 {:.1} µs (n={n})",
        v[n / 2].as_secs_f64() * 1e6,
        v[n * 99 / 100].as_secs_f64() * 1e6
    )
}

fn fact() -> String {
    json!({ "jsonrpc": "2.0", "method": "events/fact", "params": { "name": "agent.status", "data": {} } }).to_string() + "\n"
}

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let engine = Engine::new(&Config::new()).unwrap();
    let mut linker: Linker<State> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    Plugin::add_to_linker::<State, wasmtime::component::HasSelf<State>>(&mut linker, |s| s)
        .unwrap();
    for path in std::env::args().skip(1) {
        let name = if path.contains("team-rules") {
            "team-rules"
        } else {
            "slack"
        };
        println!("\n## componentize-js {name}");
        let bytes = std::fs::read(&path).unwrap();
        println!("component {} B", bytes.len());
        let t = Instant::now();
        let component = Component::new(&engine, &bytes).unwrap();
        let compile = t.elapsed();
        let cwasm = component.serialize().unwrap();
        let t = Instant::now();
        let pre = unsafe { Component::deserialize(&engine, &cwasm).unwrap() };
        println!(
            "Cranelift compile {:.0} ms; precompiled {} B, loaded in {:.2} ms",
            compile.as_secs_f64() * 1e3,
            cwasm.len(),
            t.elapsed().as_secs_f64() * 1e3
        );
        let files: serde_json::Map<String, Value> = ["rules.json", "channels.json"]
            .iter()
            .filter_map(|f| {
                std::fs::read_to_string(repo.join("plugins").join(name).join(f))
                    .ok()
                    .map(|t| (f.to_string(), json!(t)))
            })
            .collect();
        let start = |component: &Component| {
            let wasi = WasiCtxBuilder::new().build();
            let state = State {
                wasi,
                table: ResourceTable::new(),
                sent: vec![],
                limits: StoreLimitsBuilder::new().memory_size(256 << 20).build(),
            };
            let mut store = Store::new(&engine, state);
            store.limiter(|s| &mut s.limits);
            let plugin = Plugin::instantiate(&mut store, component, &linker).unwrap();
            let mut r = Run {
                store,
                plugin,
                next: 0,
            };
            r.turn(&(json!({ "escouade/boot": { "files": files } }).to_string() + "\n"));
            r
        };
        let t = Instant::now();
        let mut r = start(&pre);
        let init = r.request("initialize", json!({ "protocolVersion": 1 }));
        println!(
            "start from the precompiled component + initialize {:.1} ms ({})",
            t.elapsed().as_secs_f64() * 1e3,
            init["protocolVersion"]
        );
        let mut warm = vec![];
        for _ in 0..20 {
            let t = Instant::now();
            let mut q = start(&pre);
            q.request("initialize", json!({ "protocolVersion": 1 }));
            warm.push(t.elapsed());
        }
        println!("warm start (instance + initialize) {}", stats(warm));
        let w0 = ws();
        let ten: Vec<_> = (0..10)
            .map(|_| {
                let mut q = start(&pre);
                q.request("initialize", json!({}));
                q
            })
            .collect();
        println!(
            "working set {w0} MiB, then {} MiB with 10 more instances",
            ws()
        );
        drop(ten);
        if name == "team-rules" {
            let mut v = vec![];
            for i in 0..2000 {
                let command = if i % 2 == 0 {
                    "git status && git diff --stat"
                } else {
                    "rm -rf build"
                };
                let p = json!({ "request": { "id": "r", "kind": "command", "tool": "Bash", "input": { "command": command } } });
                let t = Instant::now();
                let a = r.request("permissions/decide", p);
                v.push(t.elapsed());
                assert!(a["decision"] == "allow" || a["decision"] == "deny", "{a}");
            }
            println!("permissions/decide {}", stats(v));
        }
        let mut v = vec![];
        for _ in 0..2000 {
            let t = Instant::now();
            r.turn(&fact());
            v.push(t.elapsed());
        }
        println!("events/fact (ignored) {}", stats(v));
        for _ in 0..20000 {
            r.turn(&fact());
        }
        println!(
            "still answers after 24000 calls: {}",
            r.request("initialize", json!({}))["protocolVersion"]
        );
    }
}

/// The working set of this process, in MiB (PowerShell, as for Node).
fn ws() -> u64 {
    let pid = std::process::id();
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("(Get-Process -Id {pid}).WorkingSet64"),
        ])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<u64>()
                .ok()
        })
        .unwrap_or(0)
        >> 20
}
