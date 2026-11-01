//! ESC-319: the measures of Javy + wasmtime, and of Node for reference, on team-rules and slack.
//! `cargo run --release --example bench` (Javy of the README, Node on the PATH).
use escouade_wasm_host::*;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn stats(mut v: Vec<Duration>) -> String {
    v.sort();
    let n = v.len();
    format!(
        "median {:.1} µs, p99 {:.1} µs, max {:.1} µs (n={n})",
        v[n / 2].as_secs_f64() * 1e6,
        v[n * 99 / 100].as_secs_f64() * 1e6,
        v[n - 1].as_secs_f64() * 1e6
    )
}

fn ms(d: Duration) -> String {
    format!("{:.2} ms", d.as_secs_f64() * 1e3)
}

fn hello(id: &str) -> Value {
    json!({ "protocolVersion": 1, "escouade": { "version": "2.2.0" }, "plugin": { "id": id, "version": "2.0.0" },
            "capabilities": { "batch": true, "secrets": true, "settings": true, "log": true } })
}

fn ask(command: &str) -> Value {
    json!({ "project": { "id": "p1", "name": "Escouade" },
            "request": { "id": "r1", "kind": "command", "tool": "Bash", "input": { "command": command } } })
}

fn ignored_fact() -> Value {
    json!({ "name": "agent.status", "project": { "id": "p1", "name": "Escouade" }, "agent": { "name": "a" },
            "data": { "status": "running" } })
}

fn gz(bytes: &[u8]) -> usize {
    // The size a download would weigh: deflate by the store's HTTP (approximated by flate in PowerShell is not
    // portable; the bench counts the raw size, and gzip -9 by the git bash).
    let tmp = std::env::temp_dir().join(format!("esc319-gz-{}", std::process::id()));
    std::fs::write(&tmp, bytes).unwrap();
    let out = Command::new("gzip").arg("-9c").arg(&tmp).output();
    let _ = std::fs::remove_file(&tmp);
    out.map(|o| o.stdout.len()).unwrap_or(0)
}

fn javy_bench(plugin: &str) {
    println!("\n## Javy {plugin}");
    let bundle =
        std::fs::read_to_string(repo().join("plugins").join(plugin).join("dist/plugin.mjs"))
            .unwrap();
    let compiler = Compiler::pinned();
    let work = std::env::temp_dir().join(format!("esc319-bench-{plugin}"));
    let (wasm, took) = timed(|| compiler.compile(&bundle, &work.join("a")).unwrap());
    let again = compiler.compile(&bundle, &work.join("b")).unwrap();
    println!(
        "bundle {} B; compiled in {}; module {} B ({} B gzip -9)",
        bundle.len(),
        ms(took),
        wasm.len(),
        gz(&wasm)
    );
    println!(
        "sha256 {} / again {} : {}",
        &sha256(&wasm)[..16],
        &sha256(&again)[..16],
        if wasm == again { "same" } else { "DIFFERENT" }
    );

    // Dynamic: the same bundle against Escouade's Javy plugin (QuickJS shared by all plugins, shipped with the app).
    let js = work.join("a").join("plugin.js");
    let dynamic = work.join("dynamic.wasm");
    let ok = Command::new(javy())
        .args([
            "build",
            "-C",
            "dynamic",
            "-C",
            "deterministic=y",
            "-C",
            "source=omitted",
            "-C",
        ])
        .arg(format!("plugin={}", escouade_js().display()))
        .arg("-o")
        .arg(&dynamic)
        .arg(&js)
        .status()
        .unwrap()
        .success();
    let provider = std::fs::read(escouade_js()).unwrap();
    if ok {
        let d = std::fs::read(&dynamic).unwrap();
        println!(
            "dynamic module {} B ({} B gzip), escouade-js {} B ({} B gzip)",
            d.len(),
            gz(&d),
            provider.len(),
            gz(&provider)
        );
        let pm = load(&provider).unwrap();
        let dm = load(&d).unwrap();
        let t = Instant::now();
        let g = Grants {
            id: "x".into(),
            folder: repo().join("plugins").join(plugin),
            ..Default::default()
        };
        let mut p = Plugin::start_with(Some(&pm), &dm, g, Limits::default()).unwrap();
        p.request("initialize", hello(plugin)).unwrap();
        println!(
            "dynamic: start (both compiled) + initialize {}",
            ms(t.elapsed())
        );
    }

    let (module, cold_compile) = timed(|| load(&wasm).unwrap());
    let cwasm = precompile(&module).unwrap();
    let (_m2, deser) = timed(|| unsafe { load_precompiled(&cwasm).unwrap() });
    println!(
        "Cranelift compile {}; precompiled {} B, loaded in {}",
        ms(cold_compile),
        cwasm.len(),
        ms(deser)
    );

    let grants = || Grants {
        id: format!("dev.escouade.{plugin}"),
        folder: repo().join("plugins").join(plugin),
        secrets: [("token".to_string(), "xoxb-x".to_string())].into(),
        network: vec!["slack.com".into()],
        ..Default::default()
    };
    // Cold: from the bytes of the .wasm to the answer to initialize.
    let t = Instant::now();
    let m = load(&wasm).unwrap();
    let mut p = Plugin::start(&m, grants(), Limits::default()).unwrap();
    p.request("initialize", hello(plugin)).unwrap();
    println!(
        "cold start (compile + instance + initialize) {}",
        ms(t.elapsed())
    );
    let t = Instant::now();
    let m = unsafe { load_precompiled(&cwasm).unwrap() };
    let mut p = Plugin::start(&m, grants(), Limits::default()).unwrap();
    p.request("initialize", hello(plugin)).unwrap();
    println!("start from the precompiled module {}", ms(t.elapsed()));
    let mut warm = vec![];
    for _ in 0..50 {
        let t = Instant::now();
        let mut q = Plugin::start(&module, grants(), Limits::default()).unwrap();
        q.request("initialize", hello(plugin)).unwrap();
        warm.push(t.elapsed());
    }
    println!(
        "warm start (module in memory: instance + initialize) {}",
        stats(warm)
    );
    println!("memory after initialize {} KiB", p.memory() >> 10);
    let w0 = ws();
    let ten: Vec<_> = (0..10)
        .map(|_| {
            let mut q = Plugin::start(&module, grants(), Limits::default()).unwrap();
            q.request("initialize", hello(plugin)).unwrap();
            q
        })
        .collect();
    println!(
        "working set {w0} MiB, then {} MiB with 10 more instances",
        ws()
    );
    drop(ten);

    if plugin == "team-rules" {
        let mut v = vec![];
        for i in 0..2000 {
            let line = if i % 2 == 0 {
                "git status && git diff --stat"
            } else {
                "rm -rf build"
            };
            let t = Instant::now();
            let a = p.request("permissions/decide", ask(line)).unwrap();
            v.push(t.elapsed());
            assert!(a["decision"] == "allow" || a["decision"] == "deny");
        }
        println!("permissions/decide {}", stats(v));
    }
    let mut v = vec![];
    for _ in 0..2000 {
        let t = Instant::now();
        p.notify("events/fact", ignored_fact()).unwrap();
        v.push(t.elapsed());
    }
    println!("events/fact (ignored) {}", stats(v));
    println!("memory after 4000 calls {} KiB", p.memory() >> 10);
    for _ in 0..20000 {
        p.notify("events/fact", ignored_fact()).unwrap();
    }
    println!("memory after 24000 calls {} KiB", p.memory() >> 10);
}

/// Node, as today: the process of run, the same lines on its stdin and stdout.
fn node_bench(plugin: &str) {
    println!("\n## Node {plugin}");
    let dir = repo().join("plugins").join(plugin);
    let t = Instant::now();
    let mut child = Command::new("node")
        .arg("dist/plugin.mjs")
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut id = 0;
    let mut call = |method: &str, params: Value, answer: bool| {
        id += 1;
        let m = if answer {
            json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
        } else {
            json!({ "jsonrpc": "2.0", "method": method, "params": params })
        };
        writeln!(stdin, "{m}").unwrap();
        stdin.flush().unwrap();
        if answer {
            let mut l = String::new();
            out.read_line(&mut l).unwrap();
            serde_json::from_str::<Value>(&l).unwrap()
        } else {
            Value::Null
        }
    };
    call("initialize", hello(plugin), true);
    println!("cold start (process + initialize) {}", ms(t.elapsed()));
    if plugin == "team-rules" {
        let mut v = vec![];
        for i in 0..2000 {
            let line = if i % 2 == 0 {
                "git status && git diff --stat"
            } else {
                "rm -rf build"
            };
            let t = Instant::now();
            call("permissions/decide", ask(line), true);
            v.push(t.elapsed());
        }
        println!("permissions/decide {}", stats(v));
    }
    let pid = child.id();
    let ws = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("(Get-Process -Id {pid}).WorkingSet64"),
        ])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    println!(
        "working set of node {} MiB",
        ws.parse::<u64>().map(|b| b >> 20).unwrap_or(0)
    );
    let _ = child.kill();
}

fn main() {
    let only = std::env::args().nth(1);
    for plugin in ["team-rules", "slack"] {
        if only.as_deref().is_none_or(|o| o == "javy") {
            javy_bench(plugin);
        }
        if only.as_deref().is_none_or(|o| o == "node") {
            node_bench(plugin);
        }
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
