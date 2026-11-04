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
//!   10 s and 1 MiB at most, and never to an address of the machine or of the local network, judged on the
//!   addresses the name resolves to, by the resolver the connection itself uses ([`forbidden`]).
//! - **Its thread** ([`serve`]) waits on one channel, whatever comes: a line of the core, the end of a fetch. It writes
//!   all that came on the module's input, runs one turn, then serves what the module wrote. A fetch runs beside it, on
//!   the host's runtime ([`net`]), and comes back on the channel: a slow call holds back neither the next nor the thread.
//! - **Its synchronous call** (`escouade.call`, [`Calls`]): a JSON request, its JSON answer, served on the thread, in
//!   the time of the turn, for what a state machine must have before it answers.
//!
//! The transport is wired to the runtime's host in `runtime.rs`: same journal, same delays, same restarts.

use crate::paths::DataDir;
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
/// How long a fetch may take, and how much it may read.
const FETCH_TIME: Duration = Duration::from_secs(10);
const FETCH_BYTES: usize = 1 << 20;
/// How many fetches of a plugin are under way at once; the others wait for their turn beside its thread.
const FETCHES_AT_ONCE: usize = 16;
/// The files of its folder a plugin is given: each, and all of them.
const FILE_BYTES: u64 = 256 << 10;
const FILES_BYTES: usize = 1 << 20;

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
        let native = self
            .cache
            .join(format!("{}-{}.cwasm", sha256(&bytes), self.app_version));
        let _one = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = precompiled(&native) {
            return Ok((m, Made::Cache));
        }
        let module = reactor(&bytes)?;
        std::fs::create_dir_all(&self.cache).map_err(|e| e.to_string())?;
        let pre = w(module.serialize())?;
        crate::paths::write_atomic(&native, &pre).map_err(|e| e.to_string())?;
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
}

/// What `escouade.call` serves: a method and its parameters, its result or why not. Served on the plugin's thread, in
/// the time of its turn: nothing here may wait for long.
pub type Calls = Arc<dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync>;

/// The import `escouade.call`: the JSON request at `request`, its JSON answer (`{"result":…}` or `{"error":"…"}`)
/// written at `out`. How many bytes it took, or the opposite of how many it needs when `cap` is too small. A pointer out
/// of its memory is a fault of the module, as any other trap.
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
    let asked: Value = serde_json::from_slice(
        memory
            .data(&caller)
            .get(request..request.saturating_add(len))
            .ok_or_else(out_of_memory)?,
    )
    .unwrap_or(Value::Null);
    let method = asked["method"].as_str().unwrap_or_default();
    let answer = match &caller.data().calls {
        Some(calls) => calls(method, &asked["params"]),
        None => Err(format!("no call {method}")),
    };
    let answer = match answer {
        Ok(result) => json!({ "result": result }),
        Err(error) => json!({ "error": error }),
    }
    .to_string();
    if answer.len() > cap as usize {
        return Ok(-(answer.len() as i64));
    }
    let out = out as usize;
    memory
        .data_mut(&mut caller)
        .get_mut(out..out.saturating_add(answer.len()))
        .ok_or_else(out_of_memory)?
        .copy_from_slice(answer.as_bytes());
    Ok(answer.len() as i64)
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
    /// Its instance, without `escouade.call` to serve.
    pub fn start(
        module: &Module,
        files: &BTreeMap<String, String>,
        limits: Limits,
    ) -> Result<Instance, Fault> {
        Self::start_with(module, files, limits, None)
    }

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

type Dns = Arc<dyn Fn(&str) -> Vec<IpAddr> + Send + Sync>;

/// The resolver of `net/fetch`'s connections: the system's, its forbidden addresses left out, and none at all a
/// refusal. The connection is made to what it gives, never to what another resolution would say.
struct Guarded {
    dns: Option<Dns>,
}

impl reqwest::dns::Resolve for Guarded {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        let dns = self.dns.clone();
        Box::pin(async move {
            let all: Vec<IpAddr> = match dns {
                Some(f) => f(&host),
                None => tokio::net::lookup_host((host.as_str(), 0))
                    .await?
                    .map(|s| s.ip())
                    .collect(),
            };
            let (bad, good): (Vec<IpAddr>, Vec<IpAddr>) =
                all.into_iter().partition(|ip| forbidden(*ip));
            if good.is_empty() {
                return Err(Box::new(Forbidden { host, addrs: bad })
                    as Box<dyn std::error::Error + Send + Sync>);
            }
            let addrs: reqwest::dns::Addrs =
                Box::new(good.into_iter().map(|ip| SocketAddr::new(ip, 0)));
            Ok(addrs)
        })
    }
}

/// What the tests change of the network: a host served elsewhere (the fake Slack, in http, past the checks of the
/// addresses), and the answer of the resolver.
#[derive(Clone, Default)]
pub struct Net {
    pub routes: HashMap<String, String>,
    pub dns: Option<Dns>,
}

/// The host's runtime: what runs beside the plugins' threads (their `net/fetch`).
fn net() -> &'static tokio::runtime::Handle {
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

/// A plugin's `net/fetch`: only to its hosts, by https, through the guarded resolver.
pub struct Fetcher {
    id: String,
    network: Vec<String>,
    routes: HashMap<String, String>,
    client: Option<reqwest::Client>,
    at_once: tokio::sync::Semaphore,
}

impl Fetcher {
    pub fn new(id: &str, network: &[String], net: &Net) -> Self {
        let client = reqwest::Client::builder()
            .dns_resolver(Arc::new(Guarded {
                dns: net.dns.clone(),
            }))
            // The plugin gets the 3xx: a fetch to its Location is judged again.
            .redirect(reqwest::redirect::Policy::none())
            // Straight to the host: a proxy would resolve the name where its address is not judged.
            .no_proxy()
            .timeout(FETCH_TIME)
            .user_agent(concat!("escouade/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok();
        Self {
            id: id.to_string(),
            network: network.to_vec(),
            routes: net.routes.clone(),
            client,
            at_once: tokio::sync::Semaphore::new(FETCHES_AT_ONCE),
        }
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

    /// `{ url, method, headers, body }` → `{ status, headers, body }`, or why not (`REFUSED`, `FAILED`).
    pub async fn fetch(&self, params: &Value) -> Result<Value, (i64, String)> {
        let url = params.get("url").and_then(Value::as_str).unwrap_or("");
        let parsed = reqwest::Url::parse(url)
            .map_err(|_| (super::runtime::INVALID_PARAMS, format!("not a URL: {url}")))?;
        if parsed.scheme() != "https" {
            return Err((REFUSED, format!("only https: {url}")));
        }
        let name = parsed
            .host_str()
            .filter(|h| !h.is_empty())
            .ok_or_else(|| (super::runtime::INVALID_PARAMS, format!("not a URL: {url}")))?
            .to_ascii_lowercase();
        // An address written in the URL: never one of permissions.network, which names hosts.
        if name.trim_matches(['[', ']']).parse::<IpAddr>().is_ok() {
            return Err((
                REFUSED,
                format!("{name} is not in permissions.network of {}", self.id),
            ));
        }
        if !self.covers(&name) {
            return Err((
                REFUSED,
                format!("{name} is not in permissions.network of {}", self.id),
            ));
        }
        let target = match self.routes.get(&name) {
            Some(base) => {
                let mut rest = parsed.path().to_string();
                if let Some(q) = parsed.query() {
                    rest.push('?');
                    rest.push_str(q);
                }
                format!("{}{rest}", base.trim_end_matches('/'))
            }
            None => parsed.to_string(),
        };
        let Some(client) = &self.client else {
            return Err((FAILED, "no network client".into()));
        };
        let method = params
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("GET");
        let method =
            reqwest::Method::from_bytes(method.to_ascii_uppercase().as_bytes()).map_err(|_| {
                (
                    super::runtime::INVALID_PARAMS,
                    format!("no HTTP method: {method}"),
                )
            })?;
        let mut req = client.request(method, &target);
        if let Some(h) = params.get("headers").and_then(Value::as_object) {
            for (k, v) in h {
                if k.eq_ignore_ascii_case("host") {
                    continue;
                }
                req = req.header(k, v.as_str().unwrap_or(""));
            }
        }
        if let Some(b) = params.get("body").and_then(Value::as_str) {
            req = req.body(b.to_string());
        }
        let _turn = self
            .at_once
            .acquire()
            .await
            .map_err(|e| (FAILED, e.to_string()))?;
        let mut res = req.send().await.map_err(|e| refusal_of(&name, &e))?;
        let status = res.status().as_u16();
        let mut headers = serde_json::Map::new();
        for (k, v) in res.headers() {
            let v = v.to_str().unwrap_or("");
            headers
                .entry(k.as_str().to_string())
                .and_modify(|old| {
                    *old = json!(format!("{}, {v}", old.as_str().unwrap_or("")));
                })
                .or_insert_with(|| json!(v));
        }
        let mut body = Vec::new();
        while let Some(chunk) = res
            .chunk()
            .await
            .map_err(|e| (FAILED, format!("{name}: {e}")))?
        {
            if body.len() + chunk.len() > FETCH_BYTES {
                return Err((FAILED, format!("{name}: an answer over 1 MiB")));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(json!({
            "status": status,
            "headers": headers,
            "body": String::from_utf8_lossy(&body),
        }))
    }
}

/// A failed request: refused when the guarded resolver refused its addresses, failed otherwise.
fn refusal_of(name: &str, e: &reqwest::Error) -> (i64, String) {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(e);
    while let Some(s) = source {
        if let Some(f) = s.downcast_ref::<Forbidden>() {
            return (REFUSED, f.to_string());
        }
        source = s.source();
    }
    let what = if e.is_timeout() {
        format!("no answer within {} s", FETCH_TIME.as_secs())
    } else {
        let mut s = e.to_string();
        let mut src = std::error::Error::source(e);
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
/// written (`runtime::Frame`). A line of the core, the end of a fetch: to the module, it is all lines.
pub type Frame = (Vec<u8>, Option<tokio::sync::oneshot::Sender<()>>);

/// What the thread of a plugin tells its host.
pub struct Wiring {
    /// What comes for it. Closed by the host: the end of the thread, a fetch under way or not.
    pub rx: tokio::sync::mpsc::UnboundedReceiver<Frame>,
    /// The way back to `rx` of what runs beside the thread, which does not keep it open.
    pub back: tokio::sync::mpsc::WeakUnboundedSender<Frame>,
    /// A line it wrote, but `net/fetch`'s requests (served here).
    pub line: Box<dyn FnMut(&str) + Send>,
    /// A line of its standard error.
    pub stderr: Box<dyn FnMut(&str) + Send>,
    /// A fetch refused or failed, for its journal: said from beside the thread.
    pub refused: Arc<dyn Fn(&str) + Send + Sync>,
}

/// Serves the instance until its host closes its input (None) or it is done (why).
pub fn serve(mut instance: Instance, fetcher: Fetcher, mut wiring: Wiring) -> Option<Fault> {
    let limits = instance.limits();
    let fetcher = Arc::new(fetcher);
    // What runs beside the thread: it goes with it, dropped here.
    let mut beside = tokio::task::JoinSet::new();
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
            match fetch_request(&line) {
                Some((id, params)) => {
                    let (fetcher, back) = (fetcher.clone(), wiring.back.clone());
                    let refused = wiring.refused.clone();
                    beside.spawn_on(
                        async move {
                            let reply = match fetcher.fetch(&params).await {
                                Ok(result) => {
                                    json!({ "jsonrpc": "2.0", "id": id, "result": result })
                                }
                                Err((code, message)) => {
                                    refused(&format!("net/fetch: {message}"));
                                    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
                                }
                            };
                            if let Some(tx) = back.upgrade() {
                                let _ = tx.send((format!("{reply}\n").into_bytes(), None));
                            }
                        },
                        net(),
                    );
                }
                None => (wiring.line)(&line),
            }
        }
        while beside.try_join_next().is_some() {}
    }
}

/// A `net/fetch` request of the plugin: its id and parameters.
fn fetch_request(line: &str) -> Option<(Value, Value)> {
    if !line.contains("net/fetch") {
        return None;
    }
    let v: Value = serde_json::from_str(line).ok()?;
    (v.get("method")?.as_str()? == "net/fetch").then(|| {
        (
            v.get("id").cloned().unwrap_or(Value::Null),
            v.get("params").cloned().unwrap_or(Value::Null),
        )
    })
}
