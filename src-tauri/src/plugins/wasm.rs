//! The plugins in WASM (ESC-320, docs/PLUGINS.md §5 and §8): a plugin whose program is a bundle of its folder
//! (`"run": { "js": "dist/plugin.mjs" }`) or a Rust module (`"run": { "wasm": "Cargo.toml" }`, ESC-331) runs inside
//! Escouade, without Node, isolated. One host for both: only the first call differs (`_start`, or `_initialize` when
//! the module has one).
//!
//! - **Its module.** The bundle is compiled by `escouade_plugin_cli::wasm` (ESC-328: the pinned Javy, escouade-js and
//!   the prelude, the same in the store's image), deterministically. Then Cranelift precompiles it for this machine. Both are kept in the app's cache ([`Modules`]): the module by the
//!   SHA-256 of what made it, the precompiled one by the app's version too, made again after an update.
//! - **A Rust module** is a reactor compiled to `wasm32-wasip1` (`crate-type = ["cdylib"]`): no `_start`, and
//!   `escouade-turn` (rustc exports no `_initialize` from a cdylib unless told to: it is called when it is there). It is `plugin.wasm` beside the manifest: what the store published, or what the author built.
//!   Precompiled and kept as a bundle's, under the SHA-256 of the module itself. It gets no first line.
//! - **Its instance.** One per plugin, which keeps its state from a call to the next, as its process did: the same
//!   JSON-RPC lines, carried by the module's standard input and output, held in memory ([`Instance`]). The input
//!   never blocks; a turn (`escouade-turn`) hands the plugin what came, and ends when QuickJS has nothing left to
//!   run. 64 MiB of memory; 2 s a turn, 500 ms when it holds a decision (the epoch, ticked every 10 ms): past them
//!   the instance is done, and the runtime makes a new one with its budget of restarts (`runtime.rs`).
//! - **What it is served.** The runtime's methods (`secrets/get`, `settings/get`, `log`), and two that Node gave
//!   before: the JSON files at the root of its folder (the first line, `escouade/boot`), and `net/fetch`
//!   ([`Fetcher`]): https only, to a host of `permissions.network` or one of its subdomains, no redirection followed,
//!   10 s and 1 MiB at most unless its manifest asks for more (`limits`, 120 s and 32 MiB at most), and never to an
//!   address of the machine or of the local network, judged on the addresses the name resolves to, by the resolver the
//!   connection itself uses ([`forbidden`]). Two things open this (ESC-332): `{account}`, the host the user typed for one
//!   of its accounts and that host alone, in http or on the local network; and `local:shared`, in http, the port the core
//!   read in the output of the process it started for the plugin, while it lives. The proxy and the TLS of Réglages ›
//!   Réseau are followed: through a proxy only the name is judged, and the journal says so.
//! - **Its thread** ([`serve`]) waits on one channel, whatever comes: a line of the core, the end of a fetch, a piece of
//!   a stream, a timer. It writes all that came on the module's input, runs one turn, then serves what the module wrote.
//!   What lasts runs beside it ([`Beside`]), on the host's runtime ([`net`]), and comes back on the channel: a slow call
//!   holds back neither the next nor the thread.
//! - **Its synchronous call** (`escouade.call`, [`Calls`]): a JSON request, its JSON answer, served on the thread, in
//!   the time of the turn, for what a state machine must have before it answers.
//! - **Its folders and its programs** ([`Capabilities`], ESC-333): `fs/*` (`files.rs`) and `proc/*` (`procs.rs`), on
//!   `permissions.files` and `permissions.commands`, by `escouade.call` and by a line (`beside.rs`) alike.
//!
//! The transport is wired to the runtime's host in `runtime.rs`: same journal, same delays, same restarts.

use super::beside::{request_of, Beside, WS_MESSAGE};
use super::socket::Net as Settings;
use super::socket::{Io, Ws};
use crate::paths::DataDir;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use wasmtime::{
    Caller, Config, Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
    TypedFunc,
};
use wasmtime_wasi::cli::{IsTerminal, StdinStream, StdoutStream};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::{InputStream, OutputStream, Pollable, StreamResult};
use wasmtime_wasi::WasiCtxBuilder;

// The compiler is the crate's: what the rest of the app, the bench and the tests call here.
#[allow(unused_imports)]
pub use escouade_plugin_cli::wasm::{
    escouade_js, escouade_js_sha256, javy_asset, reactor_form, script_of, sha256, unpack_javy,
    wrap, ClientOf, Compiler, Refusal, JAVY_VERSION, MODULE_FILE, PRELUDE, SERVED_SINCE,
};

/// The epoch's tick.
const TICK_MS: u64 = 10;
/// The error of a fetch refused (JSON-RPC): a host, a scheme or an address the plugin may not reach.
pub const REFUSED: i64 = -32003;
/// The error of a fetch that failed on the way (no answer, an answer too big).
pub const FAILED: i64 = -32004;
/// How long a fetch may take, and how much it may read, unless its manifest asks for more (`limits`)...
const FETCH_TIME: Duration = Duration::from_secs(10);
const FETCH_BYTES: usize = 1 << 20;
/// ... within this.
const FETCH_TIME_MAX: Duration = Duration::from_secs(120);
const FETCH_BYTES_MAX: usize = 32 << 20;
/// How long a stream that is not `sse` stays without a byte.
const STREAM_IDLE: Duration = Duration::from_secs(120);
/// The words of `permissions.network` that are no host.
const ACCOUNT: &str = "{account}";
const LOCAL_SHARED: &str = "local:shared";
/// How many fetches of a plugin are under way at once; the others wait for their turn beside its thread.
const FETCHES_AT_ONCE: usize = 16;
/// The files of its folder a plugin is given: each, and all of them.
const FILE_BYTES: u64 = 256 << 10;
const FILES_BYTES: usize = 1 << 20;

/// The most linear memory `limits.memoryBytes` gets a plugin.
pub const MEMORY_MAX: u64 = 256 << 20;

/// The longest turn `limits.turnSeconds` gets a plugin.
pub const TURN_MAX: Duration = Duration::from_secs(10);

/// What a plugin's instance may take.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Its linear memory, in bytes.
    pub memory: usize,
    /// A turn.
    pub turn: Duration,
    /// A turn that holds a decision (`permissions/*`).
    pub decision: Duration,
}

impl Limits {
    /// These, with what a manifest asks: more memory and longer turns, up to the host's ceilings, never less. A turn that
    /// holds a decision keeps its time: the agent waits for it.
    pub fn asked(self, manifest: super::manifest::Limits) -> Self {
        let memory = manifest
            .memory_bytes
            .map_or(self.memory, |n| self.memory.max(n.min(MEMORY_MAX) as usize));
        let turn = manifest.turn_seconds.map_or(self.turn, |n| {
            self.turn.max(Duration::from_secs(n).min(TURN_MAX))
        });
        Self {
            memory,
            turn,
            ..self
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            memory: 64 << 20,
            turn: Duration::from_secs(2),
            decision: Duration::from_millis(500),
        }
    }
}

// ---- The module ------------------------------------------------------------------------------------------------

/// The engine of every plugin: Cranelift, the epoch ticked by a thread of its own; one for the app.
pub fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut c = Config::new();
        c.epoch_interruption(true);
        let e = Engine::new(&c).expect("wasmtime's engine");
        let ticker = e.clone();
        std::thread::Builder::new()
            .name("plugins-epoch".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(TICK_MS));
                ticker.increment_epoch();
            })
            .expect("the epoch's thread");
        e
    })
}

/// wasmtime 50's own error, in words.
fn w<T>(r: wasmtime::Result<T>) -> Result<T, String> {
    r.map_err(|e| format!("{e:#}"))
}

/// Where a plugin's module came from, for its journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Made {
    /// Its precompiled module, from the cache.
    Cache,
    /// Its module, precompiled now (a new version of the app, a new `run.wasm`).
    Precompiled,
    /// Its bundle compiled by Javy, then precompiled.
    Compiled,
}

impl Made {
    pub fn describe(self) -> &'static str {
        match self {
            Made::Cache => "its precompiled module loaded from the cache",
            Made::Precompiled => "its module precompiled for this version of Escouade",
            Made::Compiled => "its bundle compiled by Javy, then precompiled",
        }
    }
}

/// The modules of the plugins in WASM, in the app's cache.
pub struct Modules {
    cache: PathBuf,
    app_version: String,
    compiler: Compiler,
    /// One compilation at a time: Javy is heavy, and two of the same bundle make the same bytes anyway.
    busy: Mutex<()>,
}

impl Modules {
    pub fn new(data: &DataDir, app_version: &str) -> Self {
        Self {
            cache: data.plugin_wasm_dir(),
            app_version: app_version.to_string(),
            compiler: Compiler::new(data.plugin_wasm_dir(), data.tools_dir()),
            busy: Mutex::new(()),
        }
    }

    pub fn client_with(&self, f: ClientOf) {
        self.compiler.client_with(f);
    }

    /// The cache's folder.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn dir(&self) -> &Path {
        &self.cache
    }

    /// The pinned Javy (`Compiler::javy`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn javy(&self) -> Result<PathBuf, String> {
        self.compiler.javy()
    }

    /// The pinned Javy of the app's tools, whatever `ESCOUADE_JAVY` says.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn pinned_javy(&self) -> Result<PathBuf, String> {
        self.compiler.pinned_javy()
    }

    /// The key of a bundle's module: what made it (Javy's version, escouade-js, the prelude and the bundle).
    fn key(script: &str) -> String {
        sha256(
            format!(
                "javy {JAVY_VERSION}\0escouade-js {}\0{script}",
                escouade_js_sha256()
            )
            .as_bytes(),
        )
    }

    /// The module of the bundle `js` of the plugin's `folder`: precompiled from the cache, or made now and kept.
    pub fn prepare(&self, folder: &Path, js: &str) -> Result<(Module, Made), String> {
        escouade_js()?;
        let path = folder.join(super::manifest::icon_path(js));
        let bundle = std::fs::read_to_string(&path)
            .map_err(|e| format!("its bundle {}: {e}", path.display()))?;
        let script = wrap(&bundle)?;
        let key = Self::key(&script);
        let wasm = self.cache.join(format!("{key}.wasm"));
        let native = self.cache.join(format!("{key}-{}.cwasm", self.app_version));
        let _one = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = precompiled(&native) {
            return Ok((m, Made::Cache));
        }
        let (bytes, made) = match std::fs::read(&wasm) {
            Ok(b) => (b, Made::Precompiled),
            Err(_) => (self.compiler.compile(&script)?, Made::Compiled),
        };
        let module = w(Module::new(engine(), &bytes))?;
        std::fs::create_dir_all(&self.cache).map_err(|e| e.to_string())?;
        if made == Made::Compiled {
            crate::paths::write_atomic(&wasm, &bytes).map_err(|e| e.to_string())?;
        }
        let pre = w(module.serialize())?;
        crate::paths::write_atomic(&native, &pre).map_err(|e| e.to_string())?;
        Ok((module, made))
    }

    /// The module of a `run.wasm` plugin (ESC-331), its `plugin.wasm`: precompiled from the cache, by the SHA-256 of the
    /// module and the app's version, or now. The module is not kept here: it stays in the plugin's folder.
    pub fn prepare_reactor(&self, folder: &Path) -> Result<(Module, Made), String> {
        let path = folder.join(MODULE_FILE);
        let bytes =
            std::fs::read(&path).map_err(|e| format!("its module {}: {e}", path.display()))?;
        let name = format!("{}-{}.cwasm", sha256(&bytes), self.app_version);
        let shared = of_the_build(folder);
        let native = shared.unwrap_or(&self.cache).join(name);
        let _one = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = precompiled(&native) {
            return Ok((m, Made::Cache));
        }
        let module = reactor(&bytes)?;
        let pre = w(module.serialize())?;
        match shared {
            // Another process may be writing the same bytes: it is only a cache.
            Some(_) => {
                let _ = crate::paths::write_atomic(&native, &pre);
            }
            None => {
                std::fs::create_dir_all(&self.cache).map_err(|e| e.to_string())?;
                crate::paths::write_atomic(&native, &pre).map_err(|e| e.to_string())?;
            }
        }
        Ok((module, Made::Precompiled))
    }

    /// The module of the program `run` of the plugin of `folder`, whichever of the two it is; a process has none.
    pub fn of(&self, folder: &Path, run: &super::manifest::Run) -> Result<(Module, Made), String> {
        match &run.js {
            Some(js) => self.prepare(folder, js),
            None => self.prepare_reactor(folder),
        }
    }

    /// Takes in the module the store compiled of `bundle` (ESC-323), once checked against its
    /// `version.json`: kept in the cache under the bundle's key, in place of any made here, so that
    /// `prepare` precompiles it instead of compiling the bundle again. Refused when it is no module.
    pub fn adopt(&self, bundle: &str, bytes: &[u8]) -> Result<(), String> {
        w(Module::validate(engine(), bytes))?;
        let key = Self::key(&wrap(bundle)?);
        let _one = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        std::fs::create_dir_all(&self.cache).map_err(|e| e.to_string())?;
        if std::fs::read(self.cache.join(format!("{key}.wasm")))
            .ok()
            .as_deref()
            == Some(bytes)
        {
            return Ok(());
        }
        // The precompiled modules of the one it replaces go with it.
        if let Ok(dir) = std::fs::read_dir(&self.cache) {
            for e in dir.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with(&format!("{key}-")) && name.ends_with(".cwasm") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        crate::paths::write_atomic(&self.cache.join(format!("{key}.wasm")), bytes)
            .map_err(|e| e.to_string())
    }

    /// The module of a bundle, compiled now, outside the cache: what `escouade-plugin build` writes, the same bytes
    /// on every system for the same bundle (ESC-322).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn build(&self, bundle: &str) -> Result<Vec<u8>, String> {
        self.compiler.build(bundle)
    }

    /// Takes the precompiled modules of another version of the app out of the cache.
    pub fn prune(&self) {
        let Ok(dir) = std::fs::read_dir(&self.cache) else {
            return;
        };
        let own = format!("-{}.cwasm", self.app_version);
        for e in dir.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".cwasm") && !name.ends_with(&own) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

/// The precompiled module of `path`, in the cache, when it is one wasmtime can load.
/// The folder `folder` itself when it holds a module of `scripts/plugins-wasm.sh`: its precompiled one is kept there, in a
/// build of development and the tests alone. Each test has a data folder of its own, and Cranelift, unoptimized in such a
/// build, would take seconds a module in each of them.
fn of_the_build(folder: &Path) -> Option<&Path> {
    let built = folder.parent()?.file_name()? == "plugins-wasm";
    (cfg!(any(debug_assertions, test)) && built).then_some(folder)
}

fn precompiled(path: &Path) -> Option<Module> {
    // Read whole, not mapped: under Windows a mapped file could be neither replaced nor taken out of the cache.
    let pre = std::fs::read(path).ok()?;
    // SAFETY: a file this app wrote itself (`Module::serialize`) into its own cache, never one received;
    // wasmtime checks it was made by this same version and configuration.
    match unsafe { Module::deserialize(engine(), &pre) } {
        Ok(m) => Some(m),
        Err(e) => {
            log::warn!("{}: {e:#}; precompiled again", path.display());
            None
        }
    }
}

/// Whether the module exports `name`.
fn exports(module: &Module, name: &str) -> bool {
    module.exports().any(|e| e.name() == name)
}

/// `bytes` compiled, when they are the reactor a `run.wasm` plugin must be: no `_start`, and `escouade-turn`.
pub fn reactor(bytes: &[u8]) -> Result<Module, String> {
    let module = w(Module::new(engine(), bytes))?;
    reactor_form(bytes)?;
    Ok(module)
}

// ---- The instance ----------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Inbox(Arc<Mutex<VecDeque<u8>>>);
#[derive(Clone, Default)]
struct Outbox(Arc<Mutex<Vec<u8>>>);

impl Outbox {
    fn take(&self) -> Vec<u8> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

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
        let mut q = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let n = size.min(q.len());
        Ok(q.drain(..n).collect::<Vec<u8>>().into())
    }
    /// Never blocks: what is there, or nothing. WASI p1 reads an empty answer as EINTR and a closed stream as
    /// 0 bytes, which the prelude takes for "nothing more for now": the stream stays usable for all that.
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
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend_from_slice(&bytes);
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
        unimplemented!("WASI p1's standard input goes through p2_stream")
    }
}
impl StdoutStream for Outbox {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(self.clone())
    }
    fn async_stream(&self) -> Box<dyn tokio::io::AsyncWrite + Send + Sync> {
        unimplemented!("WASI p1's standard output goes through p2_stream")
    }
}

struct State {
    wasi: WasiP1Ctx,
    limits: StoreLimits,
    calls: Option<Calls>,
    /// The last answer that did not fit, and the request it answers: the same request comes for it, not to be served
    /// twice (a write).
    too_big: Option<(Vec<u8>, String)>,
}

/// What `escouade.call` serves: a method and its parameters, its result or why not (a code of the protocol, and its
/// words). Served on the plugin's thread, in the time of its turn: nothing here may wait for long.
pub type Calls = Arc<dyn Fn(&str, &Value) -> Result<Value, (i64, String)> + Send + Sync>;

/// The import `escouade.call`: the JSON request at `request`, its JSON answer (`{"result":…}` or
/// `{"error":"…","code":…}`) written at `out`. How many bytes it took, or the opposite of how many it needs when `cap` is
/// too small: the same request then gets that answer, without being served again. A pointer out of its memory is a fault
/// of the module, as any other trap.
fn escouade_call(
    mut caller: Caller<'_, State>,
    request: u32,
    len: u32,
    out: u32,
    cap: u32,
) -> wasmtime::Result<i64> {
    let out_of_memory = || wasmtime::Error::msg("escouade.call: out of the module's memory");
    let Some(Extern::Memory(memory)) = caller.get_export("memory") else {
        return Err(wasmtime::Error::msg(
            "escouade.call: a module without memory",
        ));
    };
    let (request, len) = (request as usize, len as usize);
    let bytes = memory
        .data(&caller)
        .get(request..request.saturating_add(len))
        .ok_or_else(out_of_memory)?
        .to_vec();
    let answer = match caller.data_mut().too_big.take() {
        Some((asked, answer)) if asked == bytes => answer,
        _ => {
            let asked: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            let method = asked["method"].as_str().unwrap_or_default();
            let answer = match &caller.data().calls {
                Some(calls) => calls(method, &asked["params"]),
                None => Err((
                    super::runtime::METHOD_NOT_FOUND,
                    format!("no call {method}"),
                )),
            };
            match answer {
                Ok(result) => json!({ "result": result }),
                Err((code, error)) => json!({ "error": error, "code": code }),
            }
            .to_string()
        }
    };
    if answer.len() > cap as usize {
        let needs = answer.len() as i64;
        caller.data_mut().too_big = Some((bytes, answer));
        return Ok(-needs);
    }
    let out = out as usize;
    memory
        .data_mut(&mut caller)
        .get_mut(out..out.saturating_add(answer.len()))
        .ok_or_else(out_of_memory)?
        .copy_from_slice(answer.as_bytes());
    Ok(answer.len() as i64)
}

/// What a plugin in WASM reaches of the machine (ESC-333): its folders (`fs/*`) and its programs (`proc/*`). Served by
/// `escouade.call` and by a line (`Beside`) alike, the same refusals, the same journal. One for an instance: its programs
/// end with it.
pub struct Capabilities {
    files: super::files::Access,
    procs: super::procs::Procs,
    told: super::procs::Told,
}

impl Capabilities {
    pub fn new(
        files: super::files::Access,
        procs: super::procs::Procs,
        told: super::procs::Told,
    ) -> Arc<Self> {
        Arc::new(Self { files, procs, told })
    }

    /// Whether `method` is one of its methods.
    pub fn serves(method: &str) -> bool {
        method.starts_with("fs/") || method.starts_with("proc/")
    }

    pub fn call(&self, method: &str, params: &Value) -> Result<Value, (i64, String)> {
        if method.starts_with("fs/") {
            self.files
                .serve(method, params, &|kind, line| (self.told)(kind, line))
        } else if method.starts_with("proc/") {
            self.procs.serve(method, params)
        } else {
            Err((
                super::runtime::METHOD_NOT_FOUND,
                format!("no call {method}"),
            ))
        }
    }

    /// What `escouade.call` serves.
    pub fn calls(self: &Arc<Self>) -> Calls {
        let me = self.clone();
        Arc::new(move |method, params| me.call(method, params))
    }

    /// Its instance ended.
    pub fn end(&self) {
        self.procs.end();
    }
}

/// Why an instance is done.
#[derive(Debug, Clone, PartialEq)]
pub enum Fault {
    /// A turn past its time, stopped by the epoch.
    Timeout(Duration),
    /// A trap, a memory refused, an exception QuickJS did not catch.
    Crashed(String),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::Timeout(d) => write!(
                f,
                "a turn ran past {} ms: Escouade stopped its instance",
                d.as_millis()
            ),
            Fault::Crashed(why) => write!(f, "its instance crashed: {why}"),
        }
    }
}

/// A plugin's instance: its module started once, then a turn at a time.
pub struct Instance {
    store: Store<State>,
    turn: TypedFunc<(), ()>,
    inbox: Inbox,
    outbox: Outbox,
    errbox: Outbox,
    limits: Limits,
}

impl Instance {
    /// Its instance. A bundle gets the files of its folder (the first line), then its top level runs (`_start`); a
    /// reactor gets no line, and its `_initialize` runs if it has one. The rest is the same.
    pub fn start_with(
        module: &Module,
        files: &BTreeMap<String, String>,
        limits: Limits,
        calls: Option<Calls>,
    ) -> Result<Instance, Fault> {
        let crashed = Fault::Crashed;
        let mut linker: Linker<State> = Linker::new(engine());
        w(wasmtime_wasi::p1::add_to_linker_sync(
            &mut linker,
            |s: &mut State| &mut s.wasi,
        ))
        .map_err(crashed)?;
        w(linker.func_wrap("escouade", "call", escouade_call)).map_err(crashed)?;
        let inbox = Inbox::default();
        let outbox = Outbox::default();
        let errbox = Outbox::default();
        // No folder, no network, no environment, no argument: only the three streams.
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
                    .instances(1)
                    .build(),
                calls,
                too_big: None,
            },
        );
        store.limiter(|s| &mut s.limits);
        store.set_epoch_deadline(u64::MAX / 2);
        let instance = w(linker.instantiate(&mut store, module)).map_err(crashed)?;
        let turn =
            w(instance.get_typed_func::<(), ()>(&mut store, "escouade-turn")).map_err(crashed)?;
        let bundle = exports(module, "_start");
        let first = if bundle { "_start" } else { "_initialize" };
        let first = if exports(module, first) {
            Some(w(instance.get_typed_func::<(), ()>(&mut store, first)).map_err(crashed)?)
        } else {
            None
        };
        let mut me = Instance {
            store,
            turn,
            inbox,
            outbox,
            errbox,
            limits,
        };
        if bundle {
            let boot = json!({ "escouade/boot": { "files": files } });
            me.push(format!("{boot}\n").as_bytes());
        }
        if let Some(first) = first {
            me.call(first, limits.turn)?;
        }
        Ok(me)
    }

    /// Bytes for its standard input.
    pub fn push(&self, bytes: &[u8]) {
        self.inbox
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .extend(bytes);
    }

    /// What it wrote on its standard error since the last time.
    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.errbox.take()).into_owned()
    }

    fn call(&mut self, f: TypedFunc<(), ()>, within: Duration) -> Result<(), Fault> {
        let ticks = (within.as_millis() as u64 / TICK_MS).max(1);
        self.store.set_epoch_deadline(ticks);
        let r = f.call(&mut self.store, ());
        self.store.set_epoch_deadline(u64::MAX / 2);
        match r {
            Ok(()) => Ok(()),
            // `_start` ends by exit(0); the instance stays.
            Err(e)
                if e.downcast_ref::<wasmtime_wasi::I32Exit>()
                    .is_some_and(|x| x.0 == 0) =>
            {
                Ok(())
            }
            Err(e) => Err(match e.downcast_ref::<wasmtime::Trap>() {
                Some(wasmtime::Trap::Interrupt) => Fault::Timeout(within),
                _ => {
                    let said = String::from_utf8_lossy(
                        &self.errbox.0.lock().unwrap_or_else(|e| e.into_inner()),
                    )
                    .trim()
                    .lines()
                    // The error's own line, not a frame of the stack QuickJS prints after it.
                    .rfind(|l| !l.trim().is_empty() && !l.trim_start().starts_with("at "))
                    .unwrap_or("")
                    .trim()
                    .to_string();
                    let what = format!("{e:#}").lines().next().unwrap_or("").to_string();
                    Fault::Crashed(if said.is_empty() {
                        what
                    } else {
                        format!("{what}: {said}")
                    })
                }
            }),
        }
    }

    /// A turn (`escouade-turn`): what waits goes to the plugin, then every job it queued runs. Its lines.
    pub fn turn(&mut self, within: Duration) -> Result<Vec<String>, Fault> {
        let f = self.turn.clone();
        self.call(f, within)?;
        let out = self.outbox.take();
        Ok(out
            .split(|b| *b == b'\n')
            .map(|l| String::from_utf8_lossy(l).trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }
}

/// The JSON files at the root of the plugin's folder, given at its start (`rules.json`, `channels.json`):
/// 256 KiB each, 1 MiB in all; never its manifest, never a link, never anything outside it.
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
            || name == super::manifest::FILE
            || meta.len() > FILE_BYTES
            || total + meta.len() as usize > FILES_BYTES
        {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(e.path()) {
            total += text.len();
            files.insert(name, text);
        }
    }
    files
}

// ---- net/fetch -------------------------------------------------------------------------------------------------

/// Whether `ip` is an address a plugin may never reach: the machine (loopback, unspecified), the local network
/// (private, link-local, the carrier's shared space), or many at once (multicast, broadcast); an IPv4 mapped or
/// translated into IPv6 is judged as itself.
pub fn forbidden(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => forbidden_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return forbidden_v4(v4);
            }
            let s = v6.segments();
            // NAT64 (64:ff9b::/96): an IPv4 behind it.
            if s[..6] == [0x64, 0xff9b, 0, 0, 0, 0] {
                let [a, b] = s[6].to_be_bytes();
                let [c, d] = s[7].to_be_bytes();
                return forbidden_v4(Ipv4Addr::new(a, b, c, d));
            }
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                // unique local (fc00::/7), link-local (fe80::/10), the old site-local (fec0::/10)
                || (s[0] & 0xfe00) == 0xfc00
                || (s[0] & 0xffc0) == 0xfe80
                || (s[0] & 0xffc0) == 0xfec0
                // IPv4-compatible (::a.b.c.d, ::1 included above)
                || s[..6] == [0, 0, 0, 0, 0, 0]
        }
    }
}

fn forbidden_v4(v4: Ipv4Addr) -> bool {
    let [a, b, ..] = v4.octets();
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_unspecified()
        || v4.is_multicast()
        || v4.is_broadcast()
        // 0.0.0.0/8 ("this network"), 100.64.0.0/10 (the carrier's shared space), 240.0.0.0/4 (reserved)
        || a == 0
        || (a == 100 && (64..128).contains(&b))
        || a >= 240
}

/// The addresses a name resolves to, all forbidden: what the connection was refused for.
#[derive(Debug)]
struct Forbidden {
    host: String,
    addrs: Vec<IpAddr>,
}

impl std::fmt::Display for Forbidden {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let addrs: Vec<String> = self.addrs.iter().map(IpAddr::to_string).collect();
        write!(
            f,
            "{} resolves to {}, an address of this machine or of the local network",
            self.host,
            if addrs.is_empty() {
                "nothing".to_string()
            } else {
                addrs.join(", ")
            }
        )
    }
}

impl std::error::Error for Forbidden {}

pub type Dns = Arc<dyn Fn(&str) -> Vec<IpAddr> + Send + Sync>;
/// Whether a name's addresses are not judged: the host the user typed, and the proxy of the settings.
type Open = Arc<dyn Fn(&str) -> bool + Send + Sync>;
/// The hosts the user typed for the accounts of a plugin (`{account}`), by its id; asked at each call.
pub type HostsOf = Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>;
/// The ports the core read for a plugin's shared processes, while they live (`local:shared`), by its id.
pub type PortsOf = Arc<dyn Fn(&str) -> Vec<u16> + Send + Sync>;

/// What `name` resolves to, its forbidden addresses left out unless it is open; none at all is a refusal.
async fn guarded_lookup(
    dns: Option<&Dns>,
    open: &Open,
    name: &str,
) -> Result<Vec<IpAddr>, Box<dyn std::error::Error + Send + Sync>> {
    let all: Vec<IpAddr> = match dns {
        Some(f) => f(name),
        None => tokio::net::lookup_host((name, 0))
            .await?
            .map(|s| s.ip())
            .collect(),
    };
    if open(name) {
        return Ok(all);
    }
    let (bad, good): (Vec<IpAddr>, Vec<IpAddr>) = all.into_iter().partition(|ip| forbidden(*ip));
    if good.is_empty() {
        return Err(Box::new(Forbidden {
            host: name.to_string(),
            addrs: bad,
        }));
    }
    Ok(good)
}

/// The resolver of `net/fetch`'s connections: the system's, its forbidden addresses left out, and none at all a
/// refusal. The connection is made to what it gives, never to what another resolution would say.
struct Guarded {
    dns: Option<Dns>,
    open: Open,
}

impl reqwest::dns::Resolve for Guarded {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        let (dns, open) = (self.dns.clone(), self.open.clone());
        Box::pin(async move {
            let good = guarded_lookup(dns.as_ref(), &open, &host).await?;
            let addrs: reqwest::dns::Addrs =
                Box::new(good.into_iter().map(|ip| SocketAddr::new(ip, 0)));
            Ok(addrs)
        })
    }
}

/// The network of the plugins in WASM: Réglages › Réseau, who tells the hosts of `{account}` and the ports of
/// `local:shared`; and what the tests change: a host served elsewhere (the fake Slack, in http, past the checks of the
/// addresses), the answer of the resolver, and how long a stream stays silent.
#[derive(Clone, Default)]
pub struct Net {
    pub routes: HashMap<String, String>,
    pub dns: Option<Dns>,
    /// Réglages › Réseau, read at each call: a change reaches the instances that run.
    pub network: Arc<RwLock<Settings>>,
    pub accounts: Option<HostsOf>,
    pub shared: Option<PortsOf>,
    /// In place of the 120 s without a byte that end a stream.
    pub idle: Option<Duration>,
}

/// The host's runtime: what runs beside the plugins' threads (their network, their timers, their programs).
pub(super) fn net() -> &'static tokio::runtime::Handle {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("plugins-net")
                .enable_all()
                .build()
                .expect("the host's runtime")
        })
        .handle()
}

/// What a URL was judged to be, and where its request goes.
struct Judged {
    /// The host it names, in lower case: all the journal says of it (a query may hold a key).
    name: String,
    target: reqwest::Url,
    /// Straight to it, whatever the proxy: the local server of its harness, a route of the tests.
    direct: bool,
    routed: bool,
}

/// What a URL must be for the call that opens it.
#[derive(Clone, Copy, PartialEq)]
enum Way {
    Http,
    Socket,
}

/// The clients of the settings they were made for.
struct Clients {
    settings: Settings,
    by_settings: Option<reqwest::Client>,
    direct: Option<reqwest::Client>,
}

/// A plugin's way out: only to its hosts, by https (the hosts the user typed, and the local server of its harness, by
/// http too), through the guarded resolver, or the proxy of the settings, which judges by the name alone.
pub struct Fetcher {
    id: String,
    network: Vec<String>,
    routes: HashMap<String, String>,
    dns: Option<Dns>,
    live: Arc<RwLock<Settings>>,
    accounts: Option<HostsOf>,
    shared: Option<PortsOf>,
    idle: Duration,
    seconds: Duration,
    bytes: usize,
    clients: Mutex<Option<Clients>>,
    at_once: tokio::sync::Semaphore,
}

impl Fetcher {
    pub fn new(id: &str, network: &[String], net: &Net) -> Self {
        Self {
            id: id.to_string(),
            network: network.to_vec(),
            routes: net.routes.clone(),
            dns: net.dns.clone(),
            live: net.network.clone(),
            accounts: net.accounts.clone(),
            shared: net.shared.clone(),
            idle: net.idle.unwrap_or(STREAM_IDLE),
            seconds: FETCH_TIME,
            bytes: FETCH_BYTES,
            clients: Mutex::new(None),
            at_once: tokio::sync::Semaphore::new(FETCHES_AT_ONCE),
        }
    }

    /// With the limits its manifest declares, within the host's.
    pub fn limited(mut self, limits: super::manifest::Limits) -> Self {
        if let Some(s) = limits.fetch_seconds {
            self.seconds = Duration::from_secs(s).min(FETCH_TIME_MAX);
        }
        if let Some(b) = limits.fetch_bytes {
            self.bytes = usize::try_from(b)
                .unwrap_or(FETCH_BYTES_MAX)
                .min(FETCH_BYTES_MAX);
        }
        self
    }

    /// How long a stream may stay silent.
    pub(super) fn idle(&self) -> Duration {
        self.idle
    }

    /// How long a call may take, and a stream or a socket to open.
    pub fn seconds(&self) -> Duration {
        self.seconds
    }

    /// How much an answer may hold.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    fn declares(&self, word: &str) -> bool {
        self.network.iter().any(|w| w == word)
    }

    /// Whether `permissions.network` covers `host`: one of its hosts, or a subdomain of one (`slack.com` covers
    /// `api.slack.com`, not `evilslack.com`); `*.exemple.dev` its subdomains alone.
    pub fn covers(&self, host: &str) -> bool {
        let h = host.trim_end_matches('.').to_ascii_lowercase();
        !h.is_empty()
            && self.network.iter().any(|w| {
                let p = w.trim().trim_end_matches('.').to_ascii_lowercase();
                match p.strip_prefix("*.") {
                    Some(d) => !d.is_empty() && h.ends_with(&format!(".{d}")),
                    None => {
                        !p.is_empty()
                            && !p.contains('*')
                            && (h == p || h.ends_with(&format!(".{p}")))
                    }
                }
            })
    }

    /// The hosts the user typed for its accounts, when it declares `{account}`.
    fn account_hosts(&self) -> Vec<String> {
        match &self.accounts {
            Some(hosts) if self.declares(ACCOUNT) => hosts(&self.id),
            _ => Vec::new(),
        }
    }

    /// Whether `url` is a host the user typed for one of its accounts, and on its port: that host alone, none of its
    /// subdomains; no port typed is the scheme's own.
    fn is_account(&self, url: &reqwest::Url) -> bool {
        let Some(name) = url.host_str().map(str::to_ascii_lowercase) else {
            return false;
        };
        self.account_hosts().iter().any(|typed| {
            let typed = typed.to_ascii_lowercase();
            let bare = crate::forge::bare_host(&typed);
            let port = typed[bare.len()..].trim_start_matches(':');
            bare == name
                && match port.parse::<u16>() {
                    Ok(p) => url.port_or_known_default() == Some(p),
                    Err(_) => url.port().is_none(),
                }
        })
    }

    /// Whether `url` is the local server of its harness: http, 127.0.0.1, on a port the core read while it lives.
    fn is_local(&self, url: &reqwest::Url) -> bool {
        url.scheme() == "http"
            && self.declares(LOCAL_SHARED)
            && url.host_str() == Some("127.0.0.1")
            && matches!(
                (&self.shared, url.port()),
                (Some(ports), Some(p)) if ports(&self.id).contains(&p)
            )
    }

    /// A call to a fake service of the tests by its own address, as `account.api` gives it to a plugin: the answer of a
    /// route, which only the tests and a build of development have. A route without a port stands for every port of its
    /// host, by http or ws: the fake services of the voice open one each.
    fn fake(&self, url: &reqwest::Url) -> Option<Judged> {
        let origin = url.origin();
        let faked = self.routes.values().any(|base| {
            reqwest::Url::parse(base).is_ok_and(|b| {
                b.origin() == origin
                    || (b.port().is_none()
                        && matches!(url.scheme(), "http" | "ws")
                        && b.host_str() == url.host_str())
            })
        });
        faked.then(|| Judged {
            name: url.host_str().unwrap_or_default().to_ascii_lowercase(),
            target: url.clone(),
            direct: true,
            routed: true,
        })
    }

    /// The URL of a call, judged: the local server of its harness; a host of an account (its address not judged); or a
    /// host of `permissions.network` by https (`wss` for a socket), never an address written.
    fn judge(&self, url: &str, way: Way) -> Result<Judged, (i64, String)> {
        let invalid = || (super::runtime::INVALID_PARAMS, format!("not a URL: {url}"));
        let parsed = reqwest::Url::parse(url).map_err(|_| invalid())?;
        if let Some(judged) = self.fake(&parsed) {
            return Ok(judged);
        }
        let local = way == Way::Http && self.is_local(&parsed);
        let account = !local && self.is_account(&parsed);
        let scheme = parsed.scheme();
        let wanted = if way == Way::Socket { "wss" } else { "https" };
        let fine = local
            || scheme == wanted
            || (account && way == Way::Http && matches!(scheme, "http" | "https"));
        if !fine {
            return Err((
                REFUSED,
                format!("only {wanted}: {}", parsed.origin().ascii_serialization()),
            ));
        }
        let name = parsed
            .host_str()
            .filter(|h| !h.is_empty())
            .ok_or_else(invalid)?
            .to_ascii_lowercase();
        if !local && !account {
            // An address written in the URL: never one of permissions.network, which names hosts.
            let written = name.trim_matches(['[', ']']).parse::<IpAddr>().is_ok();
            if written || !self.covers(&name) {
                return Err((
                    REFUSED,
                    format!("{name} is not in permissions.network of {}", self.id),
                ));
            }
            // A proxy would take it for itself: no name of the machine, whoever declares it.
            if (name == "localhost" || name.ends_with(".localhost")) && self.proxied(&name) {
                return Err((REFUSED, format!("{name} is an address of this machine")));
            }
        }
        let route = self.routes.get(&name);
        let target = match route {
            Some(base) => {
                let mut rest = parsed.path().to_string();
                if let Some(q) = parsed.query() {
                    rest.push('?');
                    rest.push_str(q);
                }
                reqwest::Url::parse(&format!("{}{rest}", base.trim_end_matches('/')))
                    .map_err(|_| invalid())?
            }
            None => parsed.clone(),
        };
        Ok(Judged {
            direct: local || route.is_some(),
            routed: route.is_some(),
            name,
            target,
        })
    }

    /// Whether a call to `name` goes through the proxy of the settings.
    fn proxied(&self, name: &str) -> bool {
        super::socket::proxy_for(&self.live.read(), name).is_some()
    }

    /// What is told of a call: its method and its host, never its path nor its query, which may hold a key; and, through
    /// the proxy, that its address was not judged.
    pub fn describe(&self, params: &Value, method: &str) -> String {
        let url = params.get("url").and_then(Value::as_str).unwrap_or("");
        let parsed = reqwest::Url::parse(url).ok();
        let host = parsed
            .as_ref()
            .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
            .unwrap_or_default();
        let direct =
            self.routes.contains_key(&host) || parsed.as_ref().is_some_and(|u| self.is_local(u));
        let by = if !direct && self.proxied(&host) {
            " (by the proxy: its name judged, not its addresses)"
        } else {
            ""
        };
        format!("{method} {host}{by}")
    }

    /// Who may skip the judgment of the addresses: the hosts of its accounts, and the proxy of the settings.
    fn open(&self) -> Open {
        let (hosts, id, live) = (self.accounts.clone(), self.id.clone(), self.live.clone());
        let declared = self.declares(ACCOUNT);
        Arc::new(move |name: &str| {
            let name = name.to_ascii_lowercase();
            let proxy = super::socket::proxy_url(&live.read());
            if proxy.is_some_and(|p| p.host_str().is_some_and(|h| h.eq_ignore_ascii_case(&name))) {
                return true;
            }
            declared
                && hosts.as_ref().is_some_and(|hosts| {
                    hosts(&id)
                        .iter()
                        .any(|h| crate::forge::bare_host(&h.to_ascii_lowercase()) == name)
                })
        })
    }

    fn build(&self, settings: &Settings, by_proxy: bool) -> Option<reqwest::Client> {
        let mut b = reqwest::Client::builder()
            .dns_resolver(Arc::new(Guarded {
                dns: self.dns.clone(),
                open: self.open(),
            }))
            // The plugin gets the 3xx: a fetch to its Location is judged again.
            .redirect(reqwest::redirect::Policy::none())
            // The settings' proxy alone, never the system's.
            .no_proxy()
            .danger_accept_invalid_certs(settings.insecure)
            .user_agent(concat!("escouade/", env!("CARGO_PKG_VERSION")));
        if by_proxy && !settings.proxy.is_empty() {
            // A proxy that is no address stops the call: going around it would leave by a way the user closed.
            let url = super::socket::proxy_url(settings)?;
            let proxy = reqwest::Proxy::all(url.as_str())
                .ok()?
                .no_proxy(reqwest::NoProxy::from_string(&settings.no_proxy));
            b = b.proxy(proxy);
        }
        b.build().ok()
    }

    /// The client of the settings of now: `direct` around the proxy.
    fn client(&self, direct: bool) -> Option<reqwest::Client> {
        let settings = self.live.read().clone();
        let mut clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        if clients.as_ref().is_none_or(|c| c.settings != settings) {
            *clients = Some(Clients {
                by_settings: self.build(&settings, true),
                direct: self.build(&settings, false),
                settings,
            });
        }
        let c = clients.as_ref()?;
        if direct {
            c.direct.clone()
        } else {
            c.by_settings.clone()
        }
    }

    /// The request of `params` (`url`, `method`, `headers`, `body` or `bodyBase64`) to what `judged` says.
    fn request(
        &self,
        judged: &Judged,
        params: &Value,
    ) -> Result<reqwest::RequestBuilder, (i64, String)> {
        let invalid = |m: String| (super::runtime::INVALID_PARAMS, m);
        let client = self.client(judged.direct).ok_or((
            FAILED,
            "no network client: is the proxy of Réglages › Réseau an address?".to_string(),
        ))?;
        let method = params
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("GET");
        let method = reqwest::Method::from_bytes(method.to_ascii_uppercase().as_bytes())
            .map_err(|_| invalid(format!("no HTTP method: {method}")))?;
        let mut req = client.request(method, judged.target.clone());
        if let Some(h) = params.get("headers").and_then(Value::as_object) {
            for (k, v) in h {
                if k.eq_ignore_ascii_case("host") {
                    continue;
                }
                req = req.header(k, v.as_str().unwrap_or(""));
            }
        }
        let text = params.get("body").and_then(Value::as_str);
        let binary = params.get("bodyBase64").and_then(Value::as_str);
        match (text, binary) {
            (Some(_), Some(_)) => return Err(invalid("body or bodyBase64, not both".into())),
            (Some(b), None) => req = req.body(b.to_string()),
            (None, Some(b)) => {
                let bytes = BASE64
                    .decode(b)
                    .map_err(|_| invalid("bodyBase64 is not base64".into()))?;
                req = req.body(bytes);
            }
            (None, None) => {}
        }
        Ok(req)
    }

    /// `{ url, method, headers, body | bodyBase64 }` → `{ status, headers, body | bodyBase64 }`, or why not (`REFUSED`,
    /// `FAILED`). An answer that is not UTF-8 comes in `bodyBase64`, whole.
    pub async fn fetch(&self, params: &Value) -> Result<Value, (i64, String)> {
        let url = params.get("url").and_then(Value::as_str).unwrap_or("");
        let judged = self.judge(url, Way::Http)?;
        let name = judged.name.clone();
        let req = self.request(&judged, params)?.timeout(self.seconds);
        let _turn = self
            .at_once
            .acquire()
            .await
            .map_err(|e| (FAILED, e.to_string()))?;
        let mut res = req
            .send()
            .await
            .map_err(|e| refusal_of(&name, e, self.seconds))?;
        let status = res.status().as_u16();
        let headers = headers_of(res.headers());
        let mut body = Vec::new();
        loop {
            let chunk = res
                .chunk()
                .await
                .map_err(|e| refusal_of(&name, e, self.seconds))?;
            let Some(chunk) = chunk else { break };
            if body.len() + chunk.len() > self.bytes {
                return Err((
                    FAILED,
                    format!("{name}: an answer over {}", in_words(self.bytes)),
                ));
            }
            body.extend_from_slice(&chunk);
        }
        let mut answer = json!({ "status": status, "headers": headers });
        match String::from_utf8(body) {
            Ok(text) => answer["body"] = json!(text),
            Err(e) => answer["bodyBase64"] = json!(BASE64.encode(e.as_bytes())),
        }
        Ok(answer)
    }

    /// The response of `net/stream`, its headers read: what `fetch` judges, within the time of a call.
    pub(super) async fn open_stream(
        &self,
        params: &Value,
    ) -> Result<reqwest::Response, (i64, String)> {
        let url = params.get("url").and_then(Value::as_str).unwrap_or("");
        let judged = self.judge(url, Way::Http)?;
        let req = self.request(&judged, params)?;
        match tokio::time::timeout(self.seconds, req.send()).await {
            Ok(r) => r.map_err(|e| refusal_of(&judged.name, e, self.seconds)),
            Err(_) => Err((
                FAILED,
                format!(
                    "{}: no answer within {} s",
                    judged.name,
                    self.seconds.as_secs()
                ),
            )),
        }
    }

    /// The connection of a socket to what `judged` says: through the proxy, or to an address the name resolves to.
    async fn tcp_of(&self, judged: &Judged) -> Result<tokio::net::TcpStream, (i64, String)> {
        let name = &judged.name;
        let failed = |e: String| (FAILED, format!("{name}: {e}"));
        let host = judged.target.host_str().unwrap_or_default().to_string();
        let port = judged.target.port_or_known_default().unwrap_or(443);
        if judged.routed {
            return tokio::net::TcpStream::connect((host.as_str(), port))
                .await
                .map_err(|e| failed(e.to_string()));
        }
        let settings = self.live.read().clone();
        if let Some(proxy) = super::socket::proxy_for(&settings, &host) {
            return super::socket::tunnel(&proxy, &host, port)
                .await
                .map_err(failed);
        }
        let bare = host.trim_matches(['[', ']']);
        let addrs = match bare.parse::<IpAddr>() {
            Ok(ip) => vec![ip],
            Err(_) => guarded_lookup(self.dns.as_ref(), &self.open(), bare)
                .await
                .map_err(|e| match e.downcast_ref::<Forbidden>() {
                    Some(f) => (REFUSED, f.to_string()),
                    None => failed(e.to_string()),
                })?,
        };
        let mut last = "it resolves to nothing".to_string();
        for ip in addrs {
            match tokio::net::TcpStream::connect((ip, port)).await {
                Ok(tcp) => return Ok(tcp),
                Err(e) => last = e.to_string(),
            }
        }
        Err(failed(last))
    }

    /// The socket of `ws/open`: `wss` to a host of `net/fetch`, the proxy and the TLS of the settings. What fails says
    /// neither its query nor its headers, which hold the key of the service.
    pub(super) async fn open_socket(&self, params: &Value) -> Result<Ws, (i64, String)> {
        use tokio_tungstenite::tungstenite::{
            client::IntoClientRequest,
            http::{HeaderName, HeaderValue},
            protocol::WebSocketConfig,
        };
        let url = params.get("url").and_then(Value::as_str).unwrap_or("");
        let judged = self.judge(url, Way::Socket)?;
        let name = judged.name.clone();
        let failed = |e: String| (FAILED, format!("{name}: {e}"));
        let tcp = self.tcp_of(&judged).await?;
        let _ = tcp.set_nodelay(true);
        let io: Box<dyn Io> = if judged.target.scheme() == "wss" {
            let host = judged.target.host_str().unwrap_or_default();
            let host = host.trim_matches(['[', ']']);
            let tls_name = rustls::pki_types::ServerName::try_from(host.to_string())
                .map_err(|e| failed(e.to_string()))?;
            let insecure = self.live.read().insecure;
            let connector = tokio_rustls::TlsConnector::from(super::socket::tls_config(insecure));
            Box::new(
                connector
                    .connect(tls_name, tcp)
                    .await
                    .map_err(|e| failed(format!("TLS: {e}")))?,
            )
        } else {
            Box::new(tcp)
        };
        let mut req = judged
            .target
            .as_str()
            .into_client_request()
            .map_err(|_| failed("not a socket's address".into()))?;
        let headers = params.get("headers").and_then(Value::as_object);
        if let Some(h) = headers {
            let invalid = |k: &str| {
                (
                    super::runtime::INVALID_PARAMS,
                    format!("the header {k} does not hold"),
                )
            };
            for (k, v) in h {
                if k.eq_ignore_ascii_case("host") {
                    continue;
                }
                let key = HeaderName::from_bytes(k.as_bytes()).map_err(|_| invalid(k))?;
                let value =
                    HeaderValue::from_str(v.as_str().unwrap_or("")).map_err(|_| invalid(k))?;
                req.headers_mut().insert(key, value);
            }
        }
        let config = WebSocketConfig::default()
            .max_message_size(Some(WS_MESSAGE))
            .max_frame_size(Some(WS_MESSAGE));
        match tokio_tungstenite::client_async_with_config(req, io, Some(config)).await {
            Ok((ws, _)) => Ok(ws),
            // What it answered, for the plugin to say: a service says here why it refuses. Its headers' values, which hold
            // its key, never: this goes to the journal too.
            Err(tokio_tungstenite::tungstenite::Error::Http(r)) => {
                let body = r.body().as_deref().unwrap_or_default();
                let mut said = String::from_utf8_lossy(body).into_owned();
                // Its last word: `Bearer <key>` is sent, and a service quotes the key alone. Before it is cut: the
                // start of a key across the cut would stay.
                for v in headers.iter().flat_map(|h| h.values()) {
                    let key = v.as_str().and_then(|v| v.split_whitespace().last());
                    if let Some(key) = key.filter(|k| k.len() >= 4) {
                        said = said.replace(key, "***");
                    }
                }
                let said: String = said.chars().take(512).collect();
                Err(failed(format!("HTTP {}: {said}", r.status().as_u16())))
            }
            Err(e) => Err(failed(e.to_string())),
        }
    }
}

/// The headers of a response, by name; a repeated one joined.
pub(super) fn headers_of(headers: &reqwest::header::HeaderMap) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    for (k, v) in headers {
        let v = v.to_str().unwrap_or("");
        out.entry(k.as_str().to_string())
            .and_modify(|old| {
                *old = json!(format!("{}, {v}", old.as_str().unwrap_or("")));
            })
            .or_insert_with(|| json!(v));
    }
    out
}

/// A size, as a limit says it.
fn in_words(bytes: usize) -> String {
    if bytes.is_multiple_of(1 << 20) {
        format!("{} MiB", bytes >> 20)
    } else {
        format!("{bytes} bytes")
    }
}

/// A failed request: refused when the guarded resolver refused its addresses, failed otherwise. Without its URL: a query
/// may hold a key, and this goes to the journal.
pub(super) fn refusal_of(name: &str, e: reqwest::Error, within: Duration) -> (i64, String) {
    let e = e.without_url();
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&e);
    while let Some(s) = source {
        if let Some(f) = s.downcast_ref::<Forbidden>() {
            return (REFUSED, f.to_string());
        }
        source = s.source();
    }
    let what = if e.is_timeout() {
        format!("no answer within {} s", within.as_secs())
    } else {
        let mut s = e.to_string();
        let mut src = std::error::Error::source(&e);
        while let Some(x) = src {
            s = format!("{s}: {x}");
            src = x.source();
        }
        s
    };
    (FAILED, format!("{name}: {what}"))
}

// ---- The plugin's thread ---------------------------------------------------------------------------------------

/// What comes for a plugin, on the one channel its thread waits on: a line for its input, and who waits for it to be
/// written (`runtime::Frame`). A line of the core, the end of a fetch, a piece of a stream: to the module, it is all lines.
pub type Frame = (Vec<u8>, Option<tokio::sync::oneshot::Sender<()>>);

/// What the thread of a plugin tells its host.
pub struct Wiring {
    /// What comes for it. Closed by the host: the end of the thread, what runs beside it dropped.
    pub rx: tokio::sync::mpsc::UnboundedReceiver<Frame>,
    /// The way back to `rx` of what runs beside the thread, which does not keep it open.
    pub back: tokio::sync::mpsc::WeakUnboundedSender<Frame>,
    /// A line it wrote, but those of the host's own capabilities (served here).
    pub line: Box<dyn FnMut(&str) + Send>,
    /// A line of its standard error.
    pub stderr: Box<dyn FnMut(&str) + Send>,
    /// Something refused or failed, for its journal: said from beside the thread.
    pub refused: Arc<dyn Fn(&str) + Send + Sync>,
    /// What it reached, for its journal: the method and the host, never a header nor a query.
    pub said: Arc<dyn Fn(&str) + Send + Sync>,
    /// Its folders and its programs, when it asks for them by a line.
    pub capabilities: Option<Arc<Capabilities>>,
}

/// Serves the instance until its host closes its input (None) or it is done (why).
pub fn serve(mut instance: Instance, fetcher: Fetcher, mut wiring: Wiring) -> Option<Fault> {
    let limits = instance.limits();
    // What runs beside the thread: it goes with it, dropped here.
    let mut beside = Beside::new(
        Arc::new(fetcher),
        wiring.back.clone(),
        wiring.refused.clone(),
        wiring.said.clone(),
        wiring.capabilities.clone(),
    );
    let mut errors = String::new();
    let flush_stderr = |instance: &Instance, wiring: &mut Wiring, errors: &mut String| {
        errors.push_str(&instance.stderr());
        while let Some(i) = errors.find('\n') {
            let line: String = errors.drain(..=i).collect();
            (wiring.stderr)(line.trim_end());
        }
    };
    flush_stderr(&instance, &mut wiring, &mut errors);
    loop {
        // Whatever comes first, then all that came with it: one turn for all of it.
        let mut frames = vec![wiring.rx.blocking_recv()?];
        while let Ok(f) = wiring.rx.try_recv() {
            frames.push(f);
        }
        let mut within = limits.turn;
        for (bytes, written) in frames {
            if bytes
                .windows(br#""method":"permissions/"#.len())
                .any(|w| w == br#""method":"permissions/"#)
            {
                within = within.min(limits.decision);
            }
            instance.push(&bytes);
            if let Some(w) = written {
                let _ = w.send(());
            }
        }
        let lines = instance.turn(within);
        flush_stderr(&instance, &mut wiring, &mut errors);
        let lines = match lines {
            Ok(l) => l,
            Err(f) => {
                if !errors.trim().is_empty() {
                    (wiring.stderr)(errors.trim_end());
                }
                return Some(f);
            }
        };
        for line in lines {
            match request_of(&line) {
                Some(request) => beside.serve(request),
                None => (wiring.line)(&line),
            }
        }
        beside.reap();
    }
}
