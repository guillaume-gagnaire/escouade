//! The JS/TS plugins in WASM (ESC-320, docs/PLUGINS.md §5): a plugin whose program is a bundle of its folder
//! (`"run": { "js": "dist/plugin.mjs" }`) runs inside Escouade, without Node, isolated.
//!
//! - **Its module.** The bundle, after Escouade's prelude (`wasm_prelude.js`), is compiled by the pinned Javy
//!   (9.1.0, downloaded once and checked by its SHA-256) with escouade-js, Escouade's Javy plugin (QuickJS and the
//!   `escouade-turn` export, built by scripts/escouade-js.sh and embedded in the app), deterministically. Then
//!   Cranelift precompiles it for this machine. Both are kept in the app's cache ([`Modules`]): the module by the
//!   SHA-256 of what made it, the precompiled one by the app's version too, made again after an update.
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
//!
//! The transport (the thread of a plugin, [`serve`]) is wired to the runtime's host in `runtime.rs`: same journal,
//! same delays, same restarts.

use crate::paths::DataDir;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};
use wasmtime_wasi::cli::{IsTerminal, StdinStream, StdoutStream};
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::p2::{InputStream, OutputStream, Pollable, StreamResult};
use wasmtime_wasi::WasiCtxBuilder;

/// What goes before the bundle: what Node gave it, served by Escouade.
pub const PRELUDE: &str = include_str!("wasm_prelude.js");
/// The Javy the bundles are compiled with (its CLI, `-C deterministic=y`).
pub const JAVY_VERSION: &str = "9.1.0";
/// The Node modules a bundle may import, served by the prelude; any other import refuses its compilation.
const NODE: &[&str] = &["node:fs", "node:readline"];
/// The epoch's tick.
const TICK_MS: u64 = 10;
/// The error of a fetch refused (JSON-RPC): a host, a scheme or an address the plugin may not reach.
pub const REFUSED: i64 = -32003;
/// The error of a fetch that failed on the way (no answer, an answer too big).
pub const FAILED: i64 = -32004;
/// How long a fetch may take, and how much it may read.
const FETCH_TIME: Duration = Duration::from_secs(10);
const FETCH_BYTES: usize = 1 << 20;
/// The files of its folder a plugin is given: each, and all of them.
const FILE_BYTES: u64 = 256 << 10;
const FILES_BYTES: usize = 1 << 20;

static ESCOUADE_JS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/escouade-js.wasm"));

/// escouade-js as the app embeds it (`build.rs`); why not, when this build has none.
pub fn escouade_js() -> Result<&'static [u8], String> {
    if ESCOUADE_JS.is_empty() {
        return Err(
            "this Escouade was built without escouade-js (scripts/escouade-js.sh): it runs no run.js plugin".into(),
        );
    }
    Ok(ESCOUADE_JS)
}

/// The SHA-256 of escouade-js, noted with the module it made.
pub fn escouade_js_sha256() -> &'static str {
    static SUM: OnceLock<String> = OnceLock::new();
    SUM.get_or_init(|| sha256(ESCOUADE_JS))
}

pub fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

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

/// Why a bundle cannot run in WASM (ESC-322: said by `escouade-plugin` in the author's language).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// It imports a module a plugin in WASM does not have.
    Import(String),
    /// It is not one file without exports: the line that says so.
    NotOneFile(String),
}

impl Refusal {
    pub fn message(&self, lang: crate::i18n::Lang) -> String {
        let only = NODE.join(", ");
        match self {
            Refusal::Import(module) => tr_in!(
                lang,
                "le bundle importe {module}, qu'un plugin en WASM n'a pas (seulement {only})",
                "the bundle imports {module}, which a plugin in WASM does not have (only {only})"
            ),
            Refusal::NotOneFile(line) => tr_in!(
                lang,
                "le bundle doit tenir en un fichier, sans export : {line}",
                "the bundle must be one file, without exports: {line}"
            ),
        }
    }
}

/// The bundle made a script after the prelude: its `import … from "node:…"` taken from the prelude, its body run
/// once by `_start`, after the first line (`escouade/boot`). A bundle that imports anything else, or exports, is
/// refused, with the reason.
pub fn wrap(bundle: &str) -> Result<String, String> {
    script_of(bundle).map_err(|r| r.message(crate::i18n::Lang::En))
}

/// [`wrap`], its refusal kept to be said in another language.
pub fn script_of(bundle: &str) -> Result<String, Refusal> {
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
                return Err(Refusal::Import(module.to_string()));
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
        } else if t.starts_with("import ") || t.starts_with("import(") || t.starts_with("export ") {
            return Err(Refusal::NotOneFile(t.to_string()));
        } else {
            body.push_str(&line.replace("import.meta.url", "__esc.url"));
            body.push('\n');
        }
    }
    Ok(format!(
        "{PRELUDE}\nfunction __bundle() {{\n{body}}}\n\n__esc.boot(__bundle);\n"
    ))
}

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
    /// Its module from the cache, precompiled now (a new version of the app).
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

type ClientOf = Arc<dyn Fn() -> reqwest::Client + Send + Sync>;

/// The modules of the `run.js` plugins, in the app's cache.
pub struct Modules {
    cache: PathBuf,
    tools: PathBuf,
    app_version: String,
    /// The client of Javy's download (Réglages › Réseau's proxy), once the core is made.
    client: parking_lot::RwLock<Option<ClientOf>>,
    /// One compilation at a time: Javy is heavy, and two of the same bundle make the same bytes anyway.
    busy: Mutex<()>,
}

impl Modules {
    pub fn new(data: &DataDir, app_version: &str) -> Self {
        Self {
            cache: data.plugin_wasm_dir(),
            tools: data.tools_dir(),
            app_version: app_version.to_string(),
            client: parking_lot::RwLock::new(None),
            busy: Mutex::new(()),
        }
    }

    pub fn client_with(&self, f: ClientOf) {
        *self.client.write() = Some(f);
    }

    /// The cache's folder.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn dir(&self) -> &Path {
        &self.cache
    }

    /// The pinned Javy: `ESCOUADE_JAVY`, else the one downloaded into the app's tools, its archive checked by its
    /// SHA-256 (the release's, noted here), then the program itself at each use, against the SHA-256 noted beside it
    /// once its archive was checked (ESC-322): a Javy changed since is never run, and is downloaded again.
    pub fn javy(&self) -> Result<PathBuf, String> {
        if let Some(p) = std::env::var_os("ESCOUADE_JAVY") {
            return Ok(PathBuf::from(p));
        }
        self.pinned_javy()
    }

    /// The pinned Javy of the app's tools, whatever `ESCOUADE_JAVY` says.
    pub fn pinned_javy(&self) -> Result<PathBuf, String> {
        let (asset, sum) = javy_asset().ok_or_else(|| {
            format!(
                "no Javy {JAVY_VERSION} for {}-{}",
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        })?;
        let dir = self.tools.join(format!("javy-{JAVY_VERSION}"));
        let exe = dir.join(if cfg!(windows) { "javy.exe" } else { "javy" });
        let noted = dir.join(format!("javy-{JAVY_VERSION}.sha256"));
        if exe.is_file() {
            let same = match (std::fs::read(&exe), std::fs::read_to_string(&noted)) {
                (Ok(bytes), Ok(sum)) => sha256(&bytes) == sum.trim(),
                _ => false,
            };
            if same {
                return Ok(exe);
            }
            log::warn!(
                "{}: not the Javy checked at its download; downloaded again",
                exe.display()
            );
        }
        let url = format!(
            "https://github.com/bytecodealliance/javy/releases/download/v{JAVY_VERSION}/{asset}"
        );
        let client = self.client.read().as_ref().map(|f| f()).unwrap_or_default();
        let gz = block_on(async {
            let r = client
                .get(&url)
                .timeout(Duration::from_secs(300))
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(|e| format!("{url}: {e}"))?;
            r.bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| format!("{url}: {e}"))
        })?;
        let bytes = unpack_javy(&gz, asset, sum)?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&noted);
        let tmp = dir.join(format!(".javy-{}", uuid::Uuid::new_v4().simple()));
        std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
        }
        let _ = std::fs::rename(&tmp, &exe);
        let _ = std::fs::remove_file(&tmp);
        // What is there now must be what was checked (another process may have put the same there meanwhile).
        let sum = sha256(&bytes);
        if std::fs::read(&exe).map(|b| sha256(&b)).ok().as_deref() != Some(sum.as_str()) {
            return Err(format!("could not keep Javy at {}", exe.display()));
        }
        crate::paths::write_atomic(&noted, sum.as_bytes()).map_err(|e| e.to_string())?;
        Ok(exe)
    }

    /// escouade-js as a file, which Javy is given (`-C plugin=`).
    fn engine_file(&self) -> Result<PathBuf, String> {
        let bytes = escouade_js()?;
        let path = self
            .tools
            .join(format!("escouade-js-{}.wasm", &escouade_js_sha256()[..16]));
        if std::fs::read(&path).ok().as_deref() != Some(bytes) {
            std::fs::create_dir_all(&self.tools).map_err(|e| e.to_string())?;
            crate::paths::write_atomic(&path, bytes).map_err(|e| e.to_string())?;
        }
        Ok(path)
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
        // Read whole, not mapped: under Windows a mapped file could be neither replaced nor taken out of the cache.
        if let Ok(pre) = std::fs::read(&native) {
            // SAFETY: a file this app wrote itself (`Module::serialize`) into its own cache, never one received;
            // wasmtime checks it was made by this same version and configuration.
            match unsafe { Module::deserialize(engine(), &pre) } {
                Ok(m) => return Ok((m, Made::Cache)),
                Err(e) => log::warn!("{}: {e:#}; precompiled again", native.display()),
            }
        }
        let (bytes, made) = match std::fs::read(&wasm) {
            Ok(b) => (b, Made::Precompiled),
            Err(_) => (self.compile(&script)?, Made::Compiled),
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
    pub fn build(&self, bundle: &str) -> Result<Vec<u8>, String> {
        escouade_js()?;
        self.compile(&wrap(bundle)?)
    }

    /// The bundle's script to a module, by the pinned Javy, in a folder of its own.
    fn compile(&self, script: &str) -> Result<Vec<u8>, String> {
        let javy = self.javy()?;
        let plugin = self.engine_file()?;
        let work = self
            .cache
            .join(format!(".build-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let js = work.join("plugin.js");
        let out = work.join("plugin.wasm");
        let result = (|| {
            std::fs::write(&js, script).map_err(|e| e.to_string())?;
            let mut cmd = std::process::Command::new(&javy);
            cmd.args([
                "build",
                "-C",
                "deterministic=y",
                "-C",
                "source=omitted",
                "-C",
            ])
            .arg(format!("plugin={}", plugin.display()))
            .arg("-o")
            .arg(&out)
            .arg(&js)
            .stdin(std::process::Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(crate::proc::CREATE_NO_WINDOW);
            }
            let o = cmd
                .output()
                .map_err(|e| format!("Javy at {}: {e}", javy.display()))?;
            if !o.status.success() {
                return Err(format!(
                    "Javy refused the bundle: {}",
                    String::from_utf8_lossy(&o.stderr).trim()
                ));
            }
            std::fs::read(&out).map_err(|e| e.to_string())
        })();
        let _ = std::fs::remove_dir_all(&work);
        result
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

/// The program in Javy's archive `asset` (gzip), once the archive's SHA-256 is `sum`; a changed archive is refused.
pub fn unpack_javy(gz: &[u8], asset: &str, sum: &str) -> Result<Vec<u8>, String> {
    let got = sha256(gz);
    if got != sum {
        return Err(format!(
            "{asset}: its SHA-256 is {got}, not {sum}: not the pinned Javy, refused"
        ));
    }
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut flate2::read::GzDecoder::new(gz), &mut bytes)
        .map_err(|e| format!("{asset}: {e}"))?;
    Ok(bytes)
}

/// The pinned Javy's archive for this system, and its SHA-256 (the release's own digests).
pub fn javy_asset() -> Option<(&'static str, &'static str)> {
    Some(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => (
            "javy-x86_64-windows-v9.1.0.gz",
            "7148baab85d7426e8c18e2cc4deed4e9f03270cf3245dba5617a5be25a1de836",
        ),
        ("windows", "aarch64") => (
            "javy-arm-windows-v9.1.0.gz",
            "9d6b9d5a17285e88a2b9aa7d91bee910d6da8e918f185b7fbd199933a07369d2",
        ),
        ("linux", "x86_64") => (
            "javy-x86_64-linux-v9.1.0.gz",
            "a68b122d48eb3dfc1b801d4e14c39271fde3638243d3272d206e376ac9189e39",
        ),
        ("linux", "aarch64") => (
            "javy-arm-linux-v9.1.0.gz",
            "826962f0e82354cf97d6e928ea36777872bbae76c626e326948e0367c4b55cb5",
        ),
        ("macos", "aarch64") => (
            "javy-arm-macos-v9.1.0.gz",
            "99e9ec6a8e8c98e119d137c08a921d2443d3b873c675a5571e1800f4451e6294",
        ),
        ("macos", "x86_64") => (
            "javy-x86_64-macos-v9.1.0.gz",
            "6eed2927575dc2b3fb5a1563eee1ce0874e6da91c9cec39a728c9377a8cc7b5a",
        ),
        _ => return None,
    })
}

/// A future run to its end on a runtime of its own (the plugin's thread, a compilation), wherever it is called from.
fn block_on<T: Send>(f: impl std::future::Future<Output = T> + Send) -> T {
    std::thread::scope(|s| {
        s.spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime")
                .block_on(f)
        })
        .join()
        .expect("the download's thread")
    })
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

/// A plugin's instance: its module evaluated once, then a turn at a time.
pub struct Instance {
    store: Store<State>,
    turn: TypedFunc<(), ()>,
    inbox: Inbox,
    outbox: Outbox,
    errbox: Outbox,
    limits: Limits,
}

impl Instance {
    /// Its instance, with the files of its folder (the first line), its top level run (`_start`).
    pub fn start(
        module: &Module,
        files: &BTreeMap<String, String>,
        limits: Limits,
    ) -> Result<Instance, Fault> {
        let crashed = Fault::Crashed;
        let mut linker: Linker<State> = Linker::new(engine());
        w(wasmtime_wasi::p1::add_to_linker_sync(
            &mut linker,
            |s: &mut State| &mut s.wasi,
        ))
        .map_err(crashed)?;
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
            },
        );
        store.limiter(|s| &mut s.limits);
        store.set_epoch_deadline(u64::MAX / 2);
        let instance = w(linker.instantiate(&mut store, module)).map_err(crashed)?;
        let turn =
            w(instance.get_typed_func::<(), ()>(&mut store, "escouade-turn")).map_err(crashed)?;
        let start = w(instance.get_typed_func::<(), ()>(&mut store, "_start")).map_err(crashed)?;
        let mut me = Instance {
            store,
            turn,
            inbox,
            outbox,
            errbox,
            limits,
        };
        let boot = json!({ "escouade/boot": { "files": files } });
        me.push(format!("{boot}\n").as_bytes());
        me.call(start, limits.turn)?;
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

    /// Whether something waits on its input.
    pub fn waiting(&self) -> bool {
        !self
            .inbox
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
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

/// A plugin's `net/fetch`: only to its hosts, by https, through the guarded resolver.
pub struct Fetcher {
    id: String,
    network: Vec<String>,
    routes: HashMap<String, String>,
    client: Option<reqwest::Client>,
    rt: Option<tokio::runtime::Runtime>,
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
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok();
        Self {
            id: id.to_string(),
            network: network.to_vec(),
            routes: net.routes.clone(),
            client,
            rt,
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
    pub fn fetch(&self, params: &Value) -> Result<Value, (i64, String)> {
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
        let (Some(client), Some(rt)) = (&self.client, &self.rt) else {
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
        rt.block_on(async {
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
        })
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

/// A line for its input, and who waits for it to be written (`runtime::Frame`).
pub type Frame = (Vec<u8>, Option<tokio::sync::oneshot::Sender<()>>);

/// What the thread of a plugin tells its host.
pub struct Wiring {
    /// What the host writes to it.
    pub rx: tokio::sync::mpsc::UnboundedReceiver<Frame>,
    /// A line it wrote, but `net/fetch`'s requests (served here).
    pub line: Box<dyn FnMut(&str) + Send>,
    /// A line of its standard error.
    pub stderr: Box<dyn FnMut(&str) + Send>,
    /// A fetch refused or failed, for its journal.
    pub refused: Box<dyn FnMut(&str) + Send>,
}

/// Serves the instance until its host closes its input (None) or it is done (why).
pub fn serve(mut instance: Instance, fetcher: Fetcher, mut wiring: Wiring) -> Option<Fault> {
    let limits = instance.limits();
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
        // What the host wrote: wait for it, unless the plugin has answers waiting already (a fetch).
        let mut frames = Vec::new();
        if !instance.waiting() {
            frames.push(wiring.rx.blocking_recv()?);
        }
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
                    let reply = match fetcher.fetch(&params) {
                        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                        Err((code, message)) => {
                            (wiring.refused)(&format!("net/fetch: {message}"));
                            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
                        }
                    };
                    instance.push(format!("{reply}\n").as_bytes());
                }
                None => (wiring.line)(&line),
            }
        }
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
