//! The compilation of the JS/TS plugins (ESC-320, ESC-328, docs/PLUGINS.md §5): the bundle of a plugin
//! (`"run": { "js": "dist/plugin.mjs" }`), after Escouade's prelude (`wasm_prelude.js`), is compiled by the pinned Javy
//! (9.1.0, downloaded once and checked by its SHA-256) with escouade-js, Escouade's Javy plugin (QuickJS and the
//! `escouade-turn` export, built by scripts/escouade-js.sh and embedded here), deterministically.
//!
//! What runs the module (the app's cache, wasmtime, its instance and what it is served) is in the app:
//! `src-tauri/src/plugins/wasm.rs`. The store's image has this part alone. So is what a `run.wasm` module must look like
//! ([`reactor_form`], ESC-331): its exports are read here, without wasmtime, and the app compiles it.

use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// What goes before the bundle: what Node gave it, served by Escouade.
pub const PRELUDE: &str = include_str!("wasm_prelude.js");
/// The first version of Escouade that runs `run.js` plugins in WASM (what `escouade.min` of such a plugin names):
/// `escouade-plugin --version` says it, and `plugins/ci.mjs` judges the plugins of the repository against it.
pub const SERVED_SINCE: &str = "2.2.0";
/// The Javy the bundles are compiled with (its CLI, `-C deterministic=y`).
pub const JAVY_VERSION: &str = "9.1.0";
/// The Node modules a bundle may import, served by the prelude; any other import refuses its compilation.
const NODE: &[&str] = &["node:fs", "node:readline"];
/// The module of a `run.wasm` plugin, in its folder: what the store published, or what its author built.
pub const MODULE_FILE: &str = "plugin.wasm";
/// Windows's flag for a program without a console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

/// A number of a module (LEB128) at `at`, which it passes.
fn number(bytes: &[u8], at: &mut usize) -> Option<usize> {
    let (mut value, mut shift) = (0usize, 0u32);
    loop {
        let byte = *bytes.get(*at)?;
        *at += 1;
        value |= usize::from(byte & 0x7f).checked_shl(shift)?;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
        if shift > 28 {
            return None;
        }
    }
}

/// The names a module exports, from its binary form; none when it is no module.
fn exports_of(bytes: &[u8]) -> Option<Vec<String>> {
    let mut at = 8;
    if bytes.get(..8)? != b"\0asm\x01\0\0\0" {
        return None;
    }
    let mut names = Vec::new();
    while at < bytes.len() {
        let id = bytes[at];
        at += 1;
        let size = number(bytes, &mut at)?;
        let end = at.checked_add(size).filter(|e| *e <= bytes.len())?;
        // The export section: a count, then each name, its kind and its index.
        if id == 7 {
            let mut p = at;
            for _ in 0..number(bytes, &mut p)? {
                let len = number(bytes, &mut p)?;
                let stop = p.checked_add(len).filter(|e| *e <= end)?;
                names.push(String::from_utf8_lossy(&bytes[p..stop]).into_owned());
                p = stop + 1;
                number(bytes, &mut p)?;
            }
        }
        at = end;
    }
    Some(names)
}

/// Whether `bytes` have the form of the reactor a `run.wasm` plugin must be: no `_start`, and `escouade-turn` (rustc
/// exports no `_initialize` from a cdylib unless told to, so none is asked for). Why not, in words.
pub fn reactor_form(bytes: &[u8]) -> Result<(), String> {
    let form = "a plugin in Rust is a reactor (crate-type = [\"cdylib\"], target wasm32-wasip1) with its turn";
    let exports = exports_of(bytes).ok_or_else(|| format!("no WASM module: {form}"))?;
    if exports.iter().any(|e| e == "_start") {
        return Err(format!("it exports _start: {form}"));
    }
    if !exports.iter().any(|e| e == "escouade-turn") {
        return Err(format!("no export escouade-turn: {form}"));
    }
    Ok(())
}

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

/// How the download of Javy gets its client.
pub type ClientOf = Arc<dyn Fn() -> reqwest::Client + Send + Sync>;

/// What makes a `run.js` plugin's module: the pinned Javy and escouade-js.
pub struct Compiler {
    work: PathBuf,
    tools: PathBuf,
    /// The client of Javy's download (Réglages › Réseau's proxy), once the core is made.
    client: RwLock<Option<ClientOf>>,
}

impl Compiler {
    /// A compiler that works in `work` and keeps its tools in `tools`.
    pub fn new(work: PathBuf, tools: PathBuf) -> Self {
        Self {
            work,
            tools,
            client: RwLock::new(None),
        }
    }

    /// The compiler of the data folder `data`.
    pub fn of_data(data: &Path) -> Self {
        Self::new(
            crate::paths::plugin_wasm_dir(data),
            crate::paths::tools_dir(data),
        )
    }

    pub fn client_with(&self, f: ClientOf) {
        *self.client.write() = Some(f);
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
            // GitHub's releases answer 502 to 504 now and then (2026-11-08, three runs of the CI in a row): a server error
            // or a cut connection is tried again, further apart each time; anything else is said at once.
            let mut wait = Duration::from_secs(5);
            let mut tries = 1;
            loop {
                let got = async {
                    let r = client
                        .get(&url)
                        .timeout(Duration::from_secs(300))
                        .send()
                        .await?
                        .error_for_status()?;
                    r.bytes().await
                }
                .await;
                match got {
                    Ok(b) => break Ok(b.to_vec()),
                    Err(e) if tries < 4 && e.status().is_none_or(|s| s.is_server_error()) => {
                        log::warn!("{url}: {e}; tried again in {} s", wait.as_secs());
                        tokio::time::sleep(wait).await;
                        wait *= 3;
                        tries += 1;
                    }
                    Err(e) => break Err(format!("{url}: {e}")),
                }
            }
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

    /// The module of a bundle, compiled now, outside the cache: what `escouade-plugin build` writes, the same bytes
    /// on every system for the same bundle (ESC-322).
    pub fn build(&self, bundle: &str) -> Result<Vec<u8>, String> {
        escouade_js()?;
        self.compile(&wrap(bundle)?)
    }

    /// The bundle's script to a module, by the pinned Javy, in a folder of its own.
    pub fn compile(&self, script: &str) -> Result<Vec<u8>, String> {
        let javy = self.javy()?;
        let plugin = self.engine_file()?;
        let work = self
            .work
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
                cmd.creation_flags(CREATE_NO_WINDOW);
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
pub(crate) fn block_on<T: Send>(f: impl std::future::Future<Output = T> + Send) -> T {
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
