//! ESC-319: the same bundles in Extism (extism 1.41, its JS PDK 1.7, QuickJS-ng), measured as Javy is.
//! `cargo run --release -p escouade-wasm-extism`, binaryen on the PATH (README).
use extism::*;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PRELUDE: &str = include_str!("../../host/src/prelude.js");

/// The prelude's Javy.IO over Extism: the turn's input is the standard input, each line written goes to the host
/// function `send` at once (the jobs a turn queues may run after its function returned).
const GLUE: &str = r#"
const { send } = Host.getFunctions();
const __dec2 = new TextDecoder();
globalThis.Javy = { IO: {
  readSync(fd, buf) {
    const b = globalThis.__in;
    if (!b || !b.length) return 0;
    const n = Math.min(b.length, buf.length);
    buf.set(b.subarray(0, n));
    globalThis.__in = b.subarray(n);
    return n;
  },
  writeSync(fd, bytes) {
    if (fd === 1) send(Memory.fromString(__dec2.decode(bytes)).offset);
    return bytes.length;
  },
} };
function turn() {
  globalThis.__in = new Uint8Array(Host.inputBytes());
  if (!__esc.booted) __esc.boot(__bundle);
  else __esc.pump();
  return 0;
}
module.exports = { turn };
"#;

const DTS: &str = "declare module 'main' {\n  export function turn(): I32;\n}\ndeclare module 'extism:host' {\n  interface user {\n    send(ptr: I64): void;\n  }\n}\n";

fn wrap(bundle: &str) -> String {
    // The same rewriting as the Javy prototype's (host/src/lib.rs), done here simply for the two bundles.
    let mut body = String::new();
    for line in bundle.lines() {
        let t = line.trim();
        if t.starts_with("import ") && t.contains(" from ") {
            let (what, from) = t["import ".len()..].rsplit_once(" from ").unwrap();
            let module = from
                .trim()
                .trim_end_matches(';')
                .trim_matches(|c| c == '"' || c == '\'');
            body.push_str(&format!(
                "const {} = __esc.node({module:?});\n",
                what.trim()
            ));
        } else {
            body.push_str(&line.replace("import.meta.url", "__esc.url"));
            body.push('\n');
        }
    }
    format!("{PRELUDE}\nfunction __bundle() {{\n{body}}}\n{GLUE}")
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

struct Run {
    plugin: extism::Plugin,
    sent: Arc<Mutex<Vec<String>>>,
    next: u64,
}

impl Run {
    fn turn(&mut self, input: &str) -> Vec<Value> {
        self.plugin.call::<&str, Vec<u8>>("turn", input).unwrap();
        let out: Vec<String> = std::mem::take(&mut *self.sent.lock().unwrap());
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

fn fact() -> String {
    json!({ "jsonrpc": "2.0", "method": "events/fact", "params": { "name": "agent.status", "data": {} } }).to_string() + "\n"
}

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let tmp = std::env::temp_dir().join("esc319").join("extism");
    std::fs::create_dir_all(&tmp).unwrap();
    let extism_js = std::env::var("EXTISM_JS").unwrap_or_else(|_| {
        std::env::temp_dir()
            .join("esc319/dl-extism/extism-js.exe")
            .display()
            .to_string()
    });
    for name in ["team-rules", "slack"] {
        println!("\n## Extism {name}");
        let bundle =
            std::fs::read_to_string(repo.join("plugins").join(name).join("dist/plugin.mjs"))
                .unwrap();
        let js = tmp.join(format!("{name}.js"));
        let dts = tmp.join(format!("{name}.d.ts"));
        std::fs::write(&js, wrap(&bundle)).unwrap();
        std::fs::write(&dts, DTS).unwrap();
        let mut builds = vec![];
        for k in 0..2 {
            let out = tmp.join(format!("{name}-{k}.wasm"));
            let t = Instant::now();
            let st = std::process::Command::new(&extism_js)
                .arg(&js)
                .arg("-i")
                .arg(&dts)
                .arg("-o")
                .arg(&out)
                .output()
                .unwrap();
            assert!(
                st.status.success(),
                "{}",
                String::from_utf8_lossy(&st.stderr)
            );
            if k == 0 {
                println!("compiled in {:.0} ms", t.elapsed().as_secs_f64() * 1e3);
            }
            builds.push(std::fs::read(&out).unwrap());
        }
        let wasm = builds[0].clone();
        println!(
            "module {} B; two compilations {}",
            wasm.len(),
            if builds[0] == builds[1] {
                "same"
            } else {
                "DIFFERENT"
            }
        );

        let files: serde_json::Map<String, Value> = ["rules.json", "channels.json"]
            .iter()
            .filter_map(|f| {
                std::fs::read_to_string(repo.join("plugins").join(name).join(f))
                    .ok()
                    .map(|t| (f.to_string(), json!(t)))
            })
            .collect();
        let start = |wasm: &[u8]| {
            let sent = Arc::new(Mutex::new(Vec::<String>::new()));
            let kept = sent.clone();
            let send = Function::new(
                "send",
                [PTR],
                [],
                UserData::new(()),
                move |plugin, inputs, _out, _u: UserData<()>| {
                    let s: String = plugin.memory_get_val(&inputs[0])?;
                    kept.lock().unwrap().push(s);
                    Ok(())
                },
            );
            let manifest = Manifest::new([Wasm::data(wasm.to_vec())])
                .with_memory_max(1024)
                .with_timeout(Duration::from_secs(2));
            let plugin = extism::Plugin::new(&manifest, [send], true).unwrap();
            let mut r = Run {
                plugin,
                sent,
                next: 0,
            };
            r.turn(&(json!({ "escouade/boot": { "files": files } }).to_string() + "\n"));
            r
        };
        let t = Instant::now();
        let mut r = start(&wasm);
        let init = r.request(
            "initialize",
            json!({ "protocolVersion": 1, "plugin": { "id": name } }),
        );
        println!(
            "cold start (compile + instance + initialize) {:.1} ms; initialize answered {}",
            t.elapsed().as_secs_f64() * 1e3,
            init["protocolVersion"]
        );
        let mut warm = vec![];
        for _ in 0..20 {
            let t = Instant::now();
            let mut q = start(&wasm);
            q.request("initialize", json!({ "protocolVersion": 1 }));
            warm.push(t.elapsed());
        }
        println!("Plugin::new again + initialize {}", stats(warm));
        let w0 = ws();
        let ten: Vec<_> = (0..10)
            .map(|_| {
                let mut q = start(&wasm);
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
