//! ESC-319, the prototype of the WASM runtime of Escouade's JS/TS plugins.
//!
//! A plugin's committed bundle (`dist/plugin.mjs`) is compiled by Javy (QuickJS in WASM) after the prelude
//! (`prelude.js`), and run by wasmtime in Escouade's process. The protocol does not change: the same JSON-RPC lines
//! (`initialize`, `events/fact`, `permissions/decide`, `secrets/get`, `settings/get`, `log`), carried by the module's
//! standard input and output instead of a process's. Escouade serves two more methods, which Node gave before:
//! `net/fetch` (only to the hosts of `permissions.network`) and the plugin's files (the first line, `escouade/boot`).
//!
//! What would go into `src-tauri/src/plugins/wasm.rs`: [`wrap`] and [`compile`] (the store and `escouade-plugin`),
//! [`Plugin`] (the runtime's other transport, beside the process of `runtime.rs`).

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};
use wasmtime_wasi::cli::{IsTerminal, StdinStream, StdoutStream};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::{InputStream, OutputStream, Pollable, StreamResult};
use wasmtime_wasi::WasiCtxBuilder;

pub const PRELUDE: &str = include_str!("prelude.js");
/// The Node modules a bundle may import, served by the prelude; any other import is refused.
const NODE: &[&str] = &["node:fs", "node:readline"];

/// The bundle made a script after the prelude: its `import … from "node:…"` taken from the prelude, its body run
/// once by `_start`, after the first line (`escouade/boot`).
pub fn wrap(bundle: &str) -> Result<String> {
    let mut body = String::new();
    for line in bundle.lines() {
        let t = line.trim();
        if t.starts_with("import ") && t.contains(" from ") {
            let (what, from) = t["import ".len()..].rsplit_once(" from ").expect("checked");
            let module = from
                .trim()
                .trim_end_matches(';')
                .trim_matches(|c| c == '"' || c == '\'');
            if !NODE.contains(&module) {
                bail!(
                    "the bundle imports {module}, which a WASM plugin does not have (only {})",
                    NODE.join(", ")
                );
            }
            let what = what.trim();
            let binding =
                if let Some(names) = what.strip_prefix('{').and_then(|w| w.strip_suffix('}')) {
                    let names: Vec<String> = names
                        .split(',')
                        .map(str::trim)
                        .filter(|n| !n.is_empty())
                        .map(|n| match n.split_once(" as ") {
                            Some((a, b)) => format!("{}: {}", a.trim(), b.trim()),
                            None => n.to_string(),
                        })
                        .collect();
                    format!("{{ {} }}", names.join(", "))
                } else if let Some(name) = what.strip_prefix("* as ") {
                    name.trim().to_string()
                } else {
                    what.to_string()
                };
            body.push_str(&format!("const {binding} = __esc.node({module:?});\n"));
        } else if t.starts_with("import ") || t.starts_with("export ") {
            bail!("the bundle must be one file without exports: {t}");
        } else {
            body.push_str(&line.replace("import.meta.url", "__esc.url"));
            body.push('\n');
        }
    }
    Ok(format!(
        "{PRELUDE}
function __bundle() {{
{body}}}

__esc.boot(__bundle);
"
    ))
}

/// What compiles a bundle: the pinned Javy CLI and Escouade's Javy plugin (`escouade-js`, QuickJS with the
/// `escouade-turn` export), the two things the empreinte depends on with the bundle.
pub struct Compiler {
    pub javy: PathBuf,
    pub plugin: PathBuf,
}

impl Compiler {
    /// The pinned tools of the README.
    pub fn pinned() -> Compiler {
        Compiler {
            javy: javy(),
            plugin: escouade_js(),
        }
    }

    /// `dist/plugin.mjs` to a `.wasm`, in `work` (a folder of its own). A deterministic build: fixed clocks and
    /// random during the pre-initialisation; no source kept in the module.
    pub fn compile(&self, bundle: &str, work: &Path) -> Result<Vec<u8>> {
        std::fs::create_dir_all(work)?;
        let js = work.join("plugin.js");
        let out = work.join("plugin.wasm");
        std::fs::write(&js, wrap(bundle)?)?;
        let status = std::process::Command::new(&self.javy)
            .args([
                "build",
                "-C",
                "deterministic=y",
                "-C",
                "source=omitted",
                "-C",
            ])
            .arg(format!("plugin={}", self.plugin.display()))
            .arg("-o")
            .arg(&out)
            .arg(&js)
            .output()
            .with_context(|| format!("javy at {}", self.javy.display()))?;
        if !status.status.success() {
            bail!("javy: {}", String::from_utf8_lossy(&status.stderr));
        }
        Ok(std::fs::read(&out)?)
    }
}

/// wasmtime 50 has an error of its own.
fn w<T>(r: wasmtime::Result<T>) -> Result<T> {
    r.map_err(|e| anyhow!("{e:#}"))
}

pub fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

// --- The runtime --------------------------------------------------------------------------------------------------

/// What the plugin may do, from its manifest and Escouade's state.
#[derive(Clone, Default)]
pub struct Grants {
    pub id: String,
    /// `permissions.network`: the hosts a fetch may reach (the host itself, or one of its subdomains).
    pub network: Vec<String>,
    /// Its own secrets, by key (`permissions.secrets`), as the keychain gives them.
    pub secrets: HashMap<String, String>,
    pub settings: Value,
    /// The plugin's folder, whose files it reads.
    pub folder: PathBuf,
    /// Tests only: where a host is really reached (the fake Slack), after the permission is checked on its name.
    pub resolve: HashMap<String, String>,
}

#[derive(Clone, Copy)]
pub struct Limits {
    /// The most its linear memory may grow to.
    pub memory: usize,
    /// How long one turn may run, before the instance is stopped (epoch).
    pub call: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            memory: 64 << 20,
            call: Duration::from_millis(2000),
        }
    }
}

/// A line of its journal.
#[derive(Debug, Clone)]
pub struct Logged {
    pub level: String,
    pub message: String,
}

/// The engine of every plugin: Cranelift, epoch interruption (a ticker of 10 ms), one per app.
pub fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut c = Config::new();
        c.epoch_interruption(true);
        let e = Engine::new(&c).expect("engine");
        let ticker = e.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(TICK_MS));
            ticker.increment_epoch();
        });
        e
    })
}
const TICK_MS: u64 = 10;

#[derive(Clone, Default)]
struct Inbox(Arc<Mutex<VecDeque<u8>>>);
#[derive(Clone, Default)]
struct Outbox(Arc<Mutex<Vec<u8>>>);

impl IsTerminal for Inbox {
    fn is_terminal(&self) -> bool {
        false
    }
}
impl IsTerminal for Outbox {
    fn is_terminal(&self) -> bool {
        false
    }
}
#[wasmtime_wasi::async_trait]
impl Pollable for Inbox {
    async fn ready(&mut self) {}
}
#[wasmtime_wasi::async_trait]
impl Pollable for Outbox {
    async fn ready(&mut self) {}
}
#[wasmtime_wasi::async_trait]
impl InputStream for Inbox {
    fn read(&mut self, size: usize) -> StreamResult<bytes::Bytes> {
        let mut q = self.0.lock().unwrap();
        let n = size.min(q.len());
        Ok(q.drain(..n).collect::<Vec<u8>>().into())
    }
    /// Never blocks: what is there, or nothing (0 bytes read, "nothing more for now" for the prelude).
    /// (WASI p1 reads an empty answer as EINTR, a closed stream as 0 bytes: the stream is not closed for all that.)
    async fn blocking_read(&mut self, size: usize) -> StreamResult<bytes::Bytes> {
        let b = self.read(size)?;
        if b.is_empty() {
            return Err(wasmtime_wasi::p2::StreamError::Closed);
        }
        Ok(b)
    }
}
#[wasmtime_wasi::async_trait]
impl OutputStream for Outbox {
    fn write(&mut self, bytes: bytes::Bytes) -> StreamResult<()> {
        self.0.lock().unwrap().extend_from_slice(&bytes);
        Ok(())
    }
    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }
    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(1 << 20)
    }
}
impl StdinStream for Inbox {
    fn p2_stream(&self) -> Box<dyn InputStream> {
        Box::new(self.clone())
    }
    fn async_stream(&self) -> Box<dyn tokio::io::AsyncRead + Send + Sync> {
        unimplemented!("the p1 stdin goes through p2_stream")
    }
}
impl StdoutStream for Outbox {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(self.clone())
    }
    fn async_stream(&self) -> Box<dyn tokio::io::AsyncWrite + Send + Sync> {
        unimplemented!("the p1 stdout goes through p2_stream")
    }
}

struct State {
    wasi: WasiP1Ctx,
    limits: StoreLimits,
}

/// Compiles a module (Cranelift), or loads one precompiled at the installation (`Module::serialize`).
pub fn load(wasm: &[u8]) -> Result<Module> {
    w(Module::new(engine(), wasm))
}

/// Precompiled for this machine and this wasmtime, what the installation would keep beside the `.wasm`.
pub fn precompile(module: &Module) -> Result<Vec<u8>> {
    w(module.serialize())
}

/// # Safety
/// `bytes` must come from [`precompile`] of this same build (wasmtime checks its version and its flags, not more).
pub unsafe fn load_precompiled(bytes: &[u8]) -> Result<Module> {
    w(unsafe { Module::deserialize(engine(), bytes) })
}

/// Why a call failed.
#[derive(Debug)]
pub enum Fault {
    /// Past its time: stopped by the epoch, the instance is done.
    Timeout,
    /// A trap, a memory refused, a panic of QuickJS: the instance is done; Escouade makes a new one.
    Crashed(String),
    /// No answer to a request although the plugin is idle.
    NoAnswer,
}

/// A plugin running in WASM: one instance, which keeps its state from a call to the next.
pub struct Plugin {
    store: Store<State>,
    turn: TypedFunc<(), ()>,
    memory: wasmtime::Memory,
    inbox: Inbox,
    outbox: Outbox,
    errbox: Outbox,
    grants: Grants,
    limits: Limits,
    next: u64,
    pub journal: Vec<Logged>,
    pub stderr: String,
    /// What it asked Escouade, in order (`net/fetch`…), with the answer's error when refused.
    pub calls: Vec<(String, Option<String>)>,
    dead: bool,
}

/// Text files at the root of the plugin's folder, given to it at the start: its own files only (`rules.json`,
/// `channels.json`), 256 KiB each and 1 MiB in all; never its manifest's secrets, never a file outside its folder.
pub fn files_of(folder: &Path) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    let Ok(dir) = std::fs::read_dir(folder) else {
        return files;
    };
    let mut entries: Vec<_> = dir.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let Ok(meta) = std::fs::symlink_metadata(e.path()) else {
            continue;
        };
        let name = e.file_name().to_string_lossy().to_string();
        if !meta.is_file()
            || name.starts_with('.')
            || !name.ends_with(".json")
            || name == "escouade-plugin.json"
        {
            continue;
        }
        if meta.len() > 256 << 10 || total + meta.len() as usize > 1 << 20 {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(e.path()) {
            total += text.len();
            files.insert(name, text);
        }
    }
    files
}

impl Plugin {
    pub fn start(module: &Module, grants: Grants, limits: Limits) -> Result<Plugin> {
        Self::start_with(None, module, grants, limits)
    }

    /// A module of `javy build -C dynamic`: QuickJS comes from `provider` (Escouade's Javy plugin), shared by all.
    pub fn start_with(
        provider: Option<&Module>,
        module: &Module,
        grants: Grants,
        limits: Limits,
    ) -> Result<Plugin> {
        let mut linker: Linker<State> = Linker::new(engine());
        w(wasmtime_wasi::p1::add_to_linker_sync(
            &mut linker,
            |s: &mut State| &mut s.wasi,
        ))?;
        let inbox = Inbox::default();
        let outbox = Outbox::default();
        let errbox = Outbox::default();
        // No folder, no network, no environment, no clock of the machine but WASI's: only the three streams.
        let wasi = WasiCtxBuilder::new()
            .stdin(inbox.clone())
            .stdout(outbox.clone())
            .stderr(errbox.clone())
            .build_p1();
        let mut store = Store::new(
            engine(),
            State {
                wasi,
                limits: StoreLimitsBuilder::new()
                    .memory_size(limits.memory)
                    .instances(2)
                    .build(),
            },
        );
        store.limiter(|s| &mut s.limits);
        store.set_epoch_deadline(u64::MAX / 2);
        let mut shared = None;
        if let Some(provider) = provider {
            let namespace = module
                .imports()
                .map(|i| i.module().to_string())
                .find(|m| m.starts_with("escouade-js-"))
                .ok_or_else(|| anyhow!("not a dynamic module of Javy"))?;
            let p = w(linker.instantiate(&mut store, provider))?;
            shared = Some(p);
            w(linker.instance(&mut store, &namespace, p))?;
        }
        let instance = w(linker.instantiate(&mut store, module))?;
        let owner = shared.unwrap_or(instance);
        let turn = w(owner.get_typed_func::<(), ()>(&mut store, "escouade-turn"))?;
        let start = w(instance.get_typed_func::<(), ()>(&mut store, "_start"))?;
        let memory = owner
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("no memory"))?;
        let files = files_of(&grants.folder);
        let mut p = Plugin {
            store,
            turn,
            memory,
            inbox,
            outbox,
            errbox,
            grants,
            limits,
            next: 1,
            journal: vec![],
            stderr: String::new(),
            calls: vec![],
            dead: false,
        };
        p.push(&json!({ "escouade/boot": { "files": files } }));
        // The module evaluated once: the prelude, then the bundle's top level (its `createPlugin()…start()`).
        p.call(start)
            .map_err(|f| anyhow!("its start failed: {f:?}"))?;
        Ok(p)
    }

    /// The bytes of its linear memory now.
    pub fn memory(&self) -> usize {
        self.memory.data_size(&self.store)
    }

    fn push(&mut self, v: &Value) {
        let mut q = self.inbox.0.lock().unwrap();
        q.extend(serde_json::to_vec(v).unwrap());
        q.push_back(b'\n');
    }

    /// One call of `f`, bounded by the epoch; a trap ends the instance.
    fn call(&mut self, f: TypedFunc<(), ()>) -> Result<(), Fault> {
        if self.dead {
            return Err(Fault::Crashed("the instance is done".into()));
        }
        let ticks = (self.limits.call.as_millis() as u64 / TICK_MS).max(1);
        self.store.set_epoch_deadline(ticks);
        let r = f.call(&mut self.store, ());
        self.store.set_epoch_deadline(u64::MAX / 2);
        let err = std::mem::take(&mut *self.errbox.0.lock().unwrap());
        self.stderr.push_str(&String::from_utf8_lossy(&err));
        match r {
            Ok(()) => Ok(()),
            // `_start` ends by exit(0), the instance stays.
            Err(e)
                if e.downcast_ref::<wasmtime_wasi::I32Exit>()
                    .is_some_and(|x| x.0 == 0) =>
            {
                Ok(())
            }
            Err(e) => {
                self.dead = true;
                Err(match e.downcast_ref::<wasmtime::Trap>() {
                    Some(wasmtime::Trap::Interrupt) => Fault::Timeout,
                    _ => Fault::Crashed(format!(
                        "{e:#}
{}",
                        String::from_utf8_lossy(&err)
                    )),
                })
            }
        }
    }

    /// A turn of the plugin (`escouade-turn`): the lines written go to it; then what it wrote, line by line.
    fn turn(&mut self) -> Result<Vec<Value>, Fault> {
        self.call(self.turn.clone())?;
        let out = std::mem::take(&mut *self.outbox.0.lock().unwrap());
        Ok(out
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .filter_map(|l| serde_json::from_slice(l).ok())
            .collect())
    }

    /// Sends a request and runs the plugin until it answers, serving what it asks meanwhile.
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, Fault> {
        let id = self.next;
        self.next += 1;
        self.push(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        self.run(Some(id))
    }

    /// Sends a notification (`events/fact`), and runs the plugin until it is idle.
    pub fn notify(&mut self, method: &str, params: Value) -> Result<(), Fault> {
        self.push(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
        self.run(None).map(|_| ())
    }

    fn run(&mut self, wanted: Option<u64>) -> Result<Value, Fault> {
        let mut answer = None;
        loop {
            let lines = self.turn()?;
            let mut asked = false;
            for m in lines {
                match (m.get("method").and_then(Value::as_str), m.get("id")) {
                    (Some(method), Some(id)) => {
                        let reply = match self
                            .serve(method, m.get("params").unwrap_or(&Value::Null))
                        {
                            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                            Err((code, message)) => {
                                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
                            }
                        };
                        self.calls.push((
                            method.to_string(),
                            reply
                                .pointer("/error/message")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                        ));
                        self.push(&reply);
                        asked = true;
                    }
                    (Some("log"), None) => self.journal.push(Logged {
                        level: m
                            .pointer("/params/level")
                            .and_then(Value::as_str)
                            .unwrap_or("info")
                            .into(),
                        message: m
                            .pointer("/params/message")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                    }),
                    (Some(_), None) => {}
                    (None, Some(id)) if Some(id.as_u64().unwrap_or(0)) == wanted => {
                        answer = Some(m)
                    }
                    _ => {}
                }
            }
            if !asked {
                break;
            }
        }
        match (wanted, answer) {
            (None, _) => Ok(Value::Null),
            (Some(_), Some(m)) => Ok(m.get("result").cloned().unwrap_or(m)),
            (Some(_), None) => Err(Fault::NoAnswer),
        }
    }

    /// What Escouade serves the plugin: the same methods as for a process, and `net/fetch`.
    fn serve(&mut self, method: &str, params: &Value) -> Result<Value, (i64, String)> {
        match method {
            "secrets/get" => {
                let key = params
                    .get("key")
                    .and_then(Value::as_str)
                    .ok_or((-32602, "secrets/get takes a key".into()))?;
                match self.grants.secrets.get(key) {
                    Some(s) => Ok(json!(s)),
                    None => Err((
                        -32001,
                        format!("{key} is not a secret of {}", self.grants.id),
                    )),
                }
            }
            "settings/get" => Ok(self.grants.settings.clone()),
            "net/fetch" => self.fetch(params),
            _ => Err((-32601, format!("unknown method {method}"))),
        }
    }

    /// A fetch, only to the hosts of `permissions.network`, in https (http for none), 1 MiB of answer at most.
    fn fetch(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let url = params.get("url").and_then(Value::as_str).unwrap_or("");
        let (scheme, rest) = url
            .split_once("://")
            .ok_or((-32602, format!("not a URL: {url}")))?;
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
        let name = host.split(':').next().unwrap_or("").to_ascii_lowercase();
        if scheme != "https" {
            return Err((-32003, format!("only https: {url}")));
        }
        let allowed = self
            .grants
            .network
            .iter()
            .any(|h| name == *h || name.ends_with(&format!(".{h}")));
        if !allowed {
            return Err((
                -32003,
                format!("{name} is not in permissions.network of {}", self.grants.id),
            ));
        }
        let target = match self.grants.resolve.get(&name) {
            Some(base) => format!("{base}{}", &rest[host.len()..]),
            None => url.to_string(),
        };
        let method = params
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("GET");
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .http_status_as_error(false)
            // A redirection is never followed: the plugin gets the 3xx, and a fetch to its Location is judged again.
            .max_redirects(0)
            .build()
            .into();
        let mut req = ureq::http::Request::builder().method(method).uri(&target);
        if let Some(h) = params.get("headers").and_then(Value::as_object) {
            for (k, v) in h {
                req = req.header(k, v.as_str().unwrap_or(""));
            }
        }
        let body = params
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let req = req.body(body).map_err(|e| (-32602, e.to_string()))?;
        let mut res = agent
            .run(req)
            .map_err(|e| (-32004, format!("{name}: {e}")))?;
        let status = res.status().as_u16();
        let headers: serde_json::Map<String, Value> = res
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), json!(v.to_str().unwrap_or(""))))
            .collect();
        let text = res
            .body_mut()
            .with_config()
            .limit(1 << 20)
            .read_to_string()
            .map_err(|e| (-32004, e.to_string()))?;
        Ok(json!({ "status": status, "headers": headers, "body": text }))
    }
}

/// How long `f` takes.
pub fn timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let t = Instant::now();
    let v = f();
    (v, t.elapsed())
}

/// Where Escouade's Javy plugin is, initialized (`build-escouade-js.sh`): `ESCOUADE_JS`, else `target/escouade-js.wasm`.
pub fn escouade_js() -> PathBuf {
    std::env::var_os("ESCOUADE_JS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/escouade-js.wasm"))
}

/// Where the pinned Javy is: `ESCOUADE_JAVY`, else `%TEMP%\esc319\dl-javy\javy.exe` (README).
pub fn javy() -> PathBuf {
    std::env::var_os("ESCOUADE_JAVY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir()
                .join("esc319")
                .join("dl-javy")
                .join(if cfg!(windows) { "javy.exe" } else { "javy" })
        })
}
