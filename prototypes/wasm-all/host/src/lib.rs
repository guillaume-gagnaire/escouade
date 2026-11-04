//! The host of a `run.wasm` module (ESC-330, docs/PLUGINS.md §8.3): one instance a plugin, on one thread, driven by the
//! events of the host. Nothing here is built by the app; it is what `src-tauri/src/plugins/wasm.rs` would change.
//!
//! - **The module** is a reactor (`crate-type = ["cdylib"]`): `_initialize`, then `escouade-turn`, called once per wake.
//! - **The thread** waits on one channel, whatever comes: a line of the core, the end of a fetch. It writes what came on
//!   the module's standard input, runs a turn, then serves what the module wrote. A fetch runs beside it, on the host's
//!   runtime, and comes back as an event: unlike `wasm.rs` today, a slow call holds nothing back.
//! - **The synchronous call** (`escouade.call`) is for what a state machine must have before it answers (`fs/read`).

use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use wasmtime::{Caller, Config, Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};
use wasmtime_wasi::cli::{IsTerminal, StdinStream, StdoutStream};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::{InputStream, OutputStream, Pollable, StreamResult};
use wasmtime_wasi::WasiCtxBuilder;

const TICK_MS: u64 = 10;
/// As in `wasm.rs`.
const REFUSED: i64 = -32003;
const FAILED: i64 = -32004;
const FETCH_BYTES: usize = 1 << 20;
const FILE_BYTES: u64 = 256 << 10;

pub fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut c = Config::new();
        c.epoch_interruption(true);
        let e = Engine::new(&c).expect("wasmtime's engine");
        let ticker = e.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(TICK_MS));
            ticker.increment_epoch();
        });
        e
    })
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime")
    })
}

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
        unimplemented!()
    }
}
impl StdoutStream for Outbox {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(self.clone())
    }
    fn async_stream(&self) -> Box<dyn tokio::io::AsyncWrite + Send + Sync> {
        unimplemented!()
    }
}

/// What the manifest of a plugin grants, as the host applies it.
#[derive(Clone, Default)]
pub struct Grants {
    /// `permissions.network`: an exact host here (the rest of the rule is `wasm.rs`'s `covers`).
    pub network: Vec<String>,
    /// A host served elsewhere: a fake, in http, past the checks of its addresses (`wasm::Net::routes`).
    pub routes: HashMap<String, String>,
    /// `permissions.files`: the folders it may read, resolved by the host.
    pub files: Vec<PathBuf>,
}

struct State {
    wasi: WasiP1Ctx,
    limits: StoreLimits,
    grants: Grants,
}

/// `fs/read`: a file under a folder it declares, judged after its links are resolved.
fn fs_read(grants: &Grants, params: &Value) -> Value {
    let path = Path::new(params["path"].as_str().unwrap_or_default());
    let real = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => return json!({ "error": format!("{}: {e}", path.display()) }),
    };
    let within = grants
        .files
        .iter()
        .filter_map(|d| d.canonicalize().ok())
        .any(|d| real.starts_with(d));
    if !within {
        return json!({ "error": format!("{} is not in permissions.files", path.display()) });
    }
    match std::fs::metadata(&real) {
        Ok(m) if m.len() > FILE_BYTES => json!({ "error": "over 256 KiB" }),
        Ok(_) => match std::fs::read_to_string(&real) {
            Ok(text) => json!({ "result": { "text": text } }),
            Err(e) => json!({ "error": e.to_string() }),
        },
        Err(e) => json!({ "error": e.to_string() }),
    }
}

/// The host's synchronous call: a JSON request read from the module, its JSON answer written back.
fn call(mut caller: Caller<'_, State>, req: i32, len: i32, out: i32, cap: i32) -> i64 {
    let Some(Extern::Memory(memory)) = caller.get_export("memory") else {
        return 0;
    };
    let mut bytes = vec![0u8; len as usize];
    if memory.read(&caller, req as usize, &mut bytes).is_err() {
        return 0;
    }
    let request: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    let answer = match request["method"].as_str() {
        Some("fs/read") => fs_read(&caller.data().grants, &request["params"]),
        other => json!({ "error": format!("no call {other:?}") }),
    }
    .to_string();
    if answer.len() > cap as usize {
        return -(answer.len() as i64);
    }
    if memory.write(&mut caller, out as usize, answer.as_bytes()).is_err() {
        return 0;
    }
    answer.len() as i64
}

/// An instance: a module, instantiated once, then a turn at a time.
pub struct Instance {
    store: Store<State>,
    turn: TypedFunc<(), ()>,
    memory: wasmtime::Memory,
    inbox: Inbox,
    outbox: Outbox,
}

impl Instance {
    pub fn start(module: &Module, grants: Grants) -> Result<Instance, String> {
        let w = |e: wasmtime::Error| format!("{e:#}");
        let mut linker: Linker<State> = Linker::new(engine());
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |s: &mut State| &mut s.wasi).map_err(w)?;
        linker.func_wrap("escouade", "call", call).map_err(w)?;
        let inbox = Inbox::default();
        let outbox = Outbox::default();
        // No folder, no network, no environment, no argument: only the three streams.
        let wasi = WasiCtxBuilder::new()
            .stdin(inbox.clone())
            .stdout(outbox.clone())
            .stderr(Outbox::default())
            .build_p1();
        let state = State {
            wasi,
            limits: StoreLimitsBuilder::new().memory_size(64 << 20).instances(1).build(),
            grants,
        };
        let mut store = Store::new(engine(), state);
        store.limiter(|s| &mut s.limits);
        store.set_epoch_deadline(u64::MAX / 2);
        let instance = linker.instantiate(&mut store, module).map_err(w)?;
        if let Ok(init) = instance.get_typed_func::<(), ()>(&mut store, "_initialize") {
            init.call(&mut store, ()).map_err(w)?;
        }
        let turn = instance
            .get_typed_func::<(), ()>(&mut store, "escouade-turn")
            .map_err(w)?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or("no memory")?;
        Ok(Instance { store, turn, memory, inbox, outbox })
    }

    /// Its linear memory, in bytes.
    pub fn memory(&self) -> usize {
        self.memory.data_size(&self.store)
    }

    /// A turn on what is waiting, bounded by the epoch: what it wrote.
    pub fn turn(&mut self, bytes: &[u8], within: Duration) -> Result<Vec<u8>, String> {
        self.inbox.0.lock().unwrap().extend(bytes);
        self.store.set_epoch_deadline((within.as_millis() as u64 / TICK_MS).max(1));
        let r = self.turn.call(&mut self.store, ());
        self.store.set_epoch_deadline(u64::MAX / 2);
        r.map_err(|e| format!("{e:#}"))?;
        Ok(std::mem::take(&mut *self.outbox.0.lock().unwrap()))
    }
}

fn fetch_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("a client")
}

/// `net/fetch`, beside the thread of the plugin. As `wasm::Fetcher`, but for the checks of the resolved addresses.
async fn fetch(client: reqwest::Client, grants: Arc<Grants>, params: Value) -> Result<Value, (i64, String)> {
    let url = params["url"].as_str().unwrap_or_default();
    let parsed = reqwest::Url::parse(url).map_err(|_| (-32602, format!("not a URL: {url}")))?;
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    if parsed.scheme() != "https" {
        return Err((REFUSED, format!("only https: {url}")));
    }
    if !grants.network.iter().any(|h| *h == host) {
        return Err((REFUSED, format!("{host} is not in permissions.network")));
    }
    let target = match grants.routes.get(&host) {
        Some(base) => format!("{}{}", base.trim_end_matches('/'), parsed.path()),
        None => parsed.to_string(),
    };
    let method = params["method"].as_str().unwrap_or("GET");
    let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| (-32602, e.to_string()))?;
    let mut req = client.request(method, &target);
    for (k, v) in params["headers"].as_object().into_iter().flatten() {
        req = req.header(k, v.as_str().unwrap_or_default());
    }
    if let Some(b) = params["body"].as_str() {
        req = req.body(b.to_string());
    }
    let res = req.send().await.map_err(|e| (FAILED, e.to_string()))?;
    let status = res.status().as_u16();
    let mut headers = serde_json::Map::new();
    for (k, v) in res.headers() {
        headers.insert(k.to_string(), json!(v.to_str().unwrap_or_default()));
    }
    let body = res.bytes().await.map_err(|e| (FAILED, e.to_string()))?;
    if body.len() > FETCH_BYTES {
        return Err((FAILED, format!("{host}: an answer over 1 MiB")));
    }
    Ok(json!({ "status": status, "headers": headers, "body": String::from_utf8_lossy(&body) }))
}

enum Event {
    /// A line of the core, its new line included.
    Line(Vec<u8>),
    /// The end of a fetch: its answer's line.
    Answer(String),
    Close,
}

/// How the core speaks to a plugin, process or module: lines.
pub trait Plugin {
    fn send(&mut self, line: &str);
    fn recv(&mut self, within: Duration) -> Option<String>;
    /// Its memory as the system counts it, for a process; its linear memory is not the same thing.
    fn memory(&self) -> u64 {
        0
    }
}

/// A plugin in a module, on its own thread.
pub struct Wasm {
    events: Sender<Event>,
    lines: Receiver<String>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Its linear memory after its last turn.
    size: Arc<AtomicUsize>,
}

impl Wasm {
    pub fn start(module: Module, grants: Grants) -> Self {
        let (events, inbox) = channel::<Event>();
        let (out, lines) = channel::<String>();
        let back = events.clone();
        let size = Arc::new(AtomicUsize::new(0));
        let seen = size.clone();
        let thread = std::thread::spawn(move || {
            let grants = Arc::new(grants);
            let mut instance = Instance::start(&module, (*grants).clone()).expect("an instance");
            let client = fetch_client();
            let mut bytes = Vec::with_capacity(1 << 16);
            loop {
                bytes.clear();
                let first = match inbox.recv() {
                    Ok(e) => e,
                    Err(_) => return,
                };
                let mut next = Some(first);
                // Everything that came: one turn for all of it.
                while let Some(e) = next.take().or_else(|| inbox.try_recv().ok()) {
                    match e {
                        Event::Line(b) => bytes.extend_from_slice(&b),
                        Event::Answer(line) => {
                            bytes.extend_from_slice(line.as_bytes());
                            bytes.push(b'\n');
                        }
                        Event::Close => return,
                    }
                }
                let wrote = match instance.turn(&bytes, Duration::from_secs(2)) {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("the instance is done: {e}");
                        return;
                    }
                };
                seen.store(instance.memory(), Ordering::Relaxed);
                for line in wrote.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
                    let line = String::from_utf8_lossy(line);
                    match request_of(&line, "net/fetch") {
                        Some((id, params)) => {
                            let (client, grants, back) = (client.clone(), grants.clone(), back.clone());
                            runtime().spawn(async move {
                                let answer = match fetch(client, grants, params).await {
                                    Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                                    Err((code, message)) => json!({
                                        "jsonrpc": "2.0", "id": id,
                                        "error": { "code": code, "message": message },
                                    }),
                                };
                                let _ = back.send(Event::Answer(answer.to_string()));
                            });
                        }
                        None => {
                            let _ = out.send(line.into_owned());
                        }
                    }
                }
            }
        });
        Self { events, lines, thread: Some(thread), size }
    }
}

fn request_of(line: &str, method: &str) -> Option<(Value, Value)> {
    if !line.contains(method) {
        return None;
    }
    let v: Value = serde_json::from_str(line).ok()?;
    (v.get("method")?.as_str()? == method).then(|| (v["id"].clone(), v["params"].clone()))
}

impl Plugin for Wasm {
    fn send(&mut self, line: &str) {
        let mut b = Vec::with_capacity(line.len() + 1);
        b.extend_from_slice(line.as_bytes());
        b.push(b'\n');
        let _ = self.events.send(Event::Line(b));
    }
    fn recv(&mut self, within: Duration) -> Option<String> {
        match self.lines.recv_timeout(within) {
            Ok(l) => Some(l),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => None,
        }
    }
    fn memory(&self) -> u64 {
        self.size.load(Ordering::Relaxed) as u64
    }
}

impl Drop for Wasm {
    fn drop(&mut self) {
        let _ = self.events.send(Event::Close);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// A plugin in a process of its own, as `runtime.rs` runs one: its lines on its pipes.
pub struct Process {
    child: std::process::Child,
    input: std::process::ChildStdin,
    lines: Receiver<String>,
}

impl Process {
    pub fn start(program: &Path, env: &[(&str, &str)]) -> Self {
        use std::io::BufRead;
        use std::process::{Command, Stdio};
        let mut child = Command::new(program)
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the program");
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(output).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    return;
                }
            }
        });
        Self { child, input, lines }
    }
}

impl Plugin for Process {
    fn send(&mut self, line: &str) {
        use std::io::Write;
        let mut b = Vec::with_capacity(line.len() + 1);
        b.extend_from_slice(line.as_bytes());
        b.push(b'\n');
        let _ = self.input.write_all(&b);
        let _ = self.input.flush();
    }
    fn recv(&mut self, within: Duration) -> Option<String> {
        self.lines.recv_timeout(within).ok()
    }
    fn memory(&self) -> u64 {
        memory_of(self.child.id())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The working set of a process.
pub fn memory_of(pid: u32) -> u64 {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let mut s = System::new();
    let pid = Pid::from_u32(pid);
    s.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_memory(),
    );
    s.process(pid).map_or(0, |p| p.memory())
}

/// A call to a plugin and its answer, the requests it makes meanwhile served as the core serves them (`secrets/get`).
pub fn call_plugin(p: &mut dyn Plugin, id: u64, method: &str, params: Value) -> Value {
    p.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string());
    wait_answer(p, id)
}

pub fn wait_answer(p: &mut dyn Plugin, id: u64) -> Value {
    loop {
        let line = p.recv(Duration::from_secs(20)).expect("an answer");
        let v: Value = serde_json::from_str(&line).expect("JSON");
        if v["method"] == "secrets/get" {
            let answer = json!({ "jsonrpc": "2.0", "id": v["id"], "result": "lin_api_fake" });
            p.send(&answer.to_string());
        } else if v["id"] == json!(id) && v.get("method").is_none() {
            return v;
        }
    }
}

/// The folder of the repository's fixtures.
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures")
}

/// The modules of `reproducible.sh`, or of a plain build.
pub fn wasm_file(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target/reproducible/wasm32-wasip1/release")
        .join(format!("{name}.wasm"))
}

/// A native plugin of this workspace's own build, beside the test or the bench.
pub fn native_file(name: &str) -> PathBuf {
    let exe = std::env::current_exe().expect("this program");
    let mut dir = exe.parent().expect("its folder").to_path_buf();
    if dir.ends_with("deps") || dir.ends_with("examples") {
        dir.pop();
    }
    dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

/// A fake Linear: GraphQL answers on a socket of its own, keep-alive, one thread a connection. `slow` in a request's
/// body holds its answer back that long.
pub struct FakeLinear {
    pub base: String,
}

fn answer_of(body: &str, issues: usize) -> String {
    if body.contains("query Me") {
        return json!({ "data": { "viewer": { "id": "u1", "name": "Ada", "displayName": "ada", "email": "ada@example.dev" } } }).to_string();
    }
    if body.contains("states { nodes") {
        let states: Vec<Value> = ["triage", "backlog", "unstarted", "started", "completed", "canceled"]
            .iter()
            .enumerate()
            .map(|(i, t)| json!({ "id": format!("s{i}"), "name": format!("State {i}"), "type": t, "position": i }))
            .collect();
        return json!({ "data": { "team": { "states": { "nodes": states } } } }).to_string();
    }
    let nodes: Vec<Value> = (0..issues)
        .map(|i| {
            json!({
                "id": format!("id-{i}"), "identifier": format!("ENG-{i}"),
                "title": format!("Une issue de démonstration, la numéro {i}"),
                "description": "- [ ] Un critère\n- [ ] Un autre critère\n\nUn long texte de description, comme il y en a. ".repeat(4),
                "url": format!("https://linear.app/acme/issue/ENG-{i}"), "priorityLabel": "High",
                "assignee": { "displayName": "ada", "name": "Ada" },
                "state": { "id": "s3", "name": "State 3" },
                "labels": { "nodes": [{ "name": "bug" }, { "name": "backend" }] },
            })
        })
        .collect();
    json!({ "data": { "issues": { "nodes": nodes, "pageInfo": { "hasNextPage": false, "endCursor": null } } } }).to_string()
}

impl FakeLinear {
    pub fn start(issues: usize) -> Self {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let base = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || {
                    let _ = stream.set_nodelay(true);
                    let mut writer = stream.try_clone().unwrap();
                    let mut reader = BufReader::new(stream);
                    loop {
                        let mut length = 0usize;
                        loop {
                            let mut line = String::new();
                            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                                return;
                            }
                            if line == "\r\n" {
                                break;
                            }
                            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                                length = v.trim().parse().unwrap_or(0);
                            }
                        }
                        let mut body = vec![0u8; length];
                        if reader.read_exact(&mut body).is_err() {
                            return;
                        }
                        let body = String::from_utf8_lossy(&body);
                        if let Some(rest) = body.split("slow").nth(1) {
                            let ms: u64 = rest.chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0);
                            std::thread::sleep(Duration::from_millis(ms));
                        }
                        let answer = answer_of(&body, issues);
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                            answer.len()
                        );
                        if writer.write_all(head.as_bytes()).is_err() || writer.write_all(answer.as_bytes()).is_err() {
                            return;
                        }
                    }
                });
            }
        });
        Self { base }
    }
}

/// The module read and compiled by Cranelift.
pub fn load(name: &str) -> Module {
    let bytes = std::fs::read(wasm_file(name)).unwrap_or_else(|e| panic!("{name}.wasm: {e}: sh reproducible.sh first"));
    Module::new(engine(), &bytes).expect("a module")
}

/// What Linear is granted against the fake.
pub fn linear_grants(base: &str) -> Grants {
    Grants {
        network: vec!["api.linear.app".into()],
        routes: HashMap::from([("api.linear.app".to_string(), base.to_string())]),
        files: Vec::new(),
    }
}

/// A call of the source Linear, as `ProcessTicketSource` writes it. The fake is reached by `account.api` natively, by
/// the route of `api.linear.app` in a module.
pub fn linear_params(native_base: Option<&str>) -> Value {
    json!({
        "source": "linear",
        "account": { "api": native_base },
        "container": "team-1",
        "query": { "text": "", "filters": [], "label": null, "page": null },
    })
}
