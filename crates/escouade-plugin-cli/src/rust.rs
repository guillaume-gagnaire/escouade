//! The compilation of the plugins in Rust (ESC-335, docs/PLUGINS.md §8.5): a `"run": { "wasm": "Cargo.toml" }` plugin to
//! its module, the same bytes on every system, as `run.js` by Javy.
//!
//! - **The pinned toolchain**: rustc, cargo and the std of `wasm32-wasip1`, downloaded the first time they are needed from
//!   static.rust-lang.org into `<data>/tools/rust-<version>`, each archive checked by its SHA-256, then every file at each
//!   use, against what was noted when the archives were checked. No rustup, and no `rust-src`: with it, rustc writes the
//!   paths of the standard library where it finds them on this machine. `ESCOUADE_RUST_TOOLCHAIN` names another folder
//!   of the same form (the store's image, which has it in a layer), which must be of this version.
//! - **The closed list** ([`check`], [`Native`]): what cargo would run in native before the sandbox has a say. `build.rs`
//!   and procedural macros are native code of the author; a plugin has none of its own, and those of its dependencies
//!   must be in `rust_native.json`, by name, version and checksum.
//! - **The compilation**, in a copy of the plugin's folder, so that nothing around it counts (a workspace, a
//!   `[patch.crates-io]` at the root of a repository, a `.cargo/config.toml`), with a `CARGO_HOME` of its own, `--locked`,
//!   the profile of [`crate::canon::PROFILE`] and [`crate::canon`] as its `RUSTC_WRAPPER`.

use crate::i18n::Lang;
use crate::{canon, wasm};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
pub use toml;

/// The rustc of the modules: the app's. Changing it changes every hash, and every module must be signed again.
pub const VERSION: &str = "1.98.0";
/// The target of the modules (docs/PLUGINS.md §8.5.1).
pub const TARGET: &str = "wasm32-wasip1";
/// `ESCOUADE_RUST_TOOLCHAIN`: a toolchain already there, in place of the one downloaded.
pub const TOOLCHAIN_VAR: &str = "ESCOUADE_RUST_TOOLCHAIN";

/// Where this version's archives are.
const DIST: &str = "https://static.rust-lang.org/dist/2026-08-20";
/// The SHA-256 of the std of the target, from the release's `channel-rust-1.98.0.toml`.
const TARGET_STD: &str = "ef91c40b5620afe76fffffbc8be01f740bf24a55a8fc570a4cba5fb6cd8caef8";
/// The SHA-256 of `rustc`, `cargo` and `rust-std` by host, from the same file.
const HOSTS: &[(&str, [&str; 3])] = &[
    (
        "x86_64-pc-windows-msvc",
        [
            "996ed3fa10dd6097149d5feaf727354b412c7da0e063d9326fbb58242def87e1",
            "c50b54520d2748c60cabbc50c3f45fadb522453f048f77ad6d5f4f1b7e3f300b",
            "e8810c2e914a1f557d8e3760653843afdbcba188714d9c5b7247f53bb049b592",
        ],
    ),
    (
        "aarch64-pc-windows-msvc",
        [
            "2e840fc986bc2768df9b7d954591bf01c88972ce2871158d77ffcfeef3926d31",
            "91145f927130f6a2c0eaf555725c3a83afe56a1fded4a0450d680b27323a633f",
            "9daec835fd585a27f8f6c0449fc36bbd46fd58a53a99f6213d77bf064d8584d4",
        ],
    ),
    (
        "x86_64-unknown-linux-gnu",
        [
            "18ed6559de1b8ea6b77474ea86992b9a507d3a3d134d9ee017d30cf3f406e3ee",
            "18bf1598891b30dd5eb52a337d08a92b4456255ddbe4c1ab996ffb578077031c",
            "8aa6405356392ce50160d1b286e86091c5e14adae3061115699c84ed4394d546",
        ],
    ),
    (
        "aarch64-unknown-linux-gnu",
        [
            "b6fdb37bc08e320bd92381e13d7311837f04a98423b8a5b678a5eb5e87c978ca",
            "e271c3c5d50336259a166e68b60ba792a64278ab117686183be3e51a0958d34b",
            "a2eece726a579de1f554a2c3bcc62410226b0be55736f57c2b5430c1bc71b98f",
        ],
    ),
    (
        "x86_64-apple-darwin",
        [
            "a0cf817df78615a10f4d0d29dbaafd657027e1a3665be728d0f19e341c8671bb",
            "7fdd094e131f98864fea06c6e0059fcecaba7b0010d07b90023f42215b6f33e8",
            "57a2c40ea919c06c4cd9ff4fc8b26e309c00c112538ea522227251e8ccc1f931",
        ],
    ),
    (
        "aarch64-apple-darwin",
        [
            "7b39cbbb11995584741b5821511b8706ab8ecab3bc7e12b11fb1ab50cab9d079",
            "ae556d0783f4ab2a6fccd4674158df08cf36526b73de03b7fde0e10b2bfc3d2e",
            "49dc82ff5b6fb033baa6e85635ed9cf07a27e5297a22314808ab4cf65a81335a",
        ],
    ),
];

/// The file of the sums of a toolchain, in its folder: `<sha256>  <path>`, one line a file.
const SUMS: &str = ".escouade-sums";
/// A lock older than this was left by a process that died.
const STALE: Duration = Duration::from_secs(20 * 60);
/// What is left out of the copy of a plugin: what no source reads, or what `build` itself writes.
const NOT_COPIED: &[&str] = &["target", ".git", "node_modules", "packs"];
/// The sources of a `Cargo.lock` that are crates.io.
const CRATES_IO: &[&str] = &[
    "registry+https://github.com/rust-lang/crates.io-index",
    "sparse+https://index.crates.io/",
];

/// One crate whose code may run in native while a plugin compiles (`rust_native.json`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Native {
    pub name: String,
    pub version: String,
    pub checksum: String,
}

/// The closed list (docs/PLUGINS.md §8.5.4): the crates read, by name, version and checksum.
pub fn native_list() -> Vec<Native> {
    serde_json::from_str(include_str!("rust_native.json")).expect("rust_native.json")
}

/// Why a plugin in Rust is not compiled, said to its author in their language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// `run.wasm` names no `Cargo.toml` of the folder.
    NoManifest(String),
    /// The `Cargo.toml` does not read: what cargo's format says.
    BadManifest(String),
    NoLock(String),
    /// A `Cargo.lock` that does not read.
    BadLock(String),
    /// A `Cargo.lock` that is not what `Cargo.toml` and crates.io say: what cargo said.
    StaleLock(String),
    /// `[patch]` or `[replace]`: its table.
    Patch(String),
    /// A `path` that leaves the folder: what names it, and the path.
    PathOutside(String, String),
    /// A dependency that does not come from crates.io by its manifest: its name.
    NotCratesIo(String),
    /// A package of the lock that does not come from crates.io: its name and source.
    LockSource(String, String),
    /// Something other than a `cdylib`: what.
    NotCdylib(String),
    /// A `build.rs` in the plugin's own crates: the crate.
    OwnBuildScript(String),
    /// A procedural macro in the plugin's own crates: the crate.
    OwnMacro(String),
    /// A crate that would run in native and is not on the closed list: `name version`, and how it runs.
    NotListed(String, Runs),
    /// A file that would change the compilation: `.cargo/config.toml`, `rust-toolchain`.
    Config(String),
    /// A `[profile.*.package.*]` or a `build-override`, which the compilation cannot pass over.
    Profile(String),
}

/// How a crate runs in native.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runs {
    Script,
    Macro,
    /// It is a dependency of a script or of a macro.
    Under,
}

impl Refusal {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            Refusal::NoManifest(file) => tr_in!(
                lang,
                "run.wasm nomme {file}, qui n'est pas dans le dossier du plugin",
                "run.wasm names {file}, which is not in the plugin's folder"
            ),
            Refusal::BadManifest(why) => tr_in!(
                lang,
                "Cargo.toml ne se lit pas : {why}",
                "Cargo.toml does not read: {why}"
            ),
            Refusal::NoLock(file) => tr_in!(
                lang,
                "pas de {file} : commite-le, le store compile exactement ce qu'il fixe (cargo generate-lockfile)",
                "no {file}: commit it, the store compiles exactly what it pins (cargo generate-lockfile)"
            ),
            Refusal::BadLock(why) => tr_in!(
                lang,
                "Cargo.lock ne se lit pas : {why}",
                "Cargo.lock does not read: {why}"
            ),
            Refusal::StaleLock(said) => tr_in!(
                lang,
                "Cargo.lock ne correspond pas à Cargo.toml ou à crates.io (cargo generate-lockfile, puis commite-le) : {said}",
                "Cargo.lock does not match Cargo.toml or crates.io (cargo generate-lockfile, then commit it): {said}"
            ),
            Refusal::Patch(table) => tr_in!(
                lang,
                "[{table}] n'est pas permis : le store compile les dépendances publiées, pas d'autres",
                "[{table}] is not allowed: the store compiles the published dependencies, no others"
            ),
            Refusal::PathOutside(what, path) => tr_in!(
                lang,
                "{what} : le chemin {path} sort du dossier du plugin, le seul que le store lit",
                "{what}: the path {path} leaves the plugin's folder, the only one the store reads"
            ),
            Refusal::NotCratesIo(name) => tr_in!(
                lang,
                "la dépendance {name} doit venir de crates.io : ni git, ni un autre registre",
                "the dependency {name} must come from crates.io: neither git nor another registry"
            ),
            Refusal::LockSource(name, source) => tr_in!(
                lang,
                "Cargo.lock : {name} vient de {source}, pas de crates.io",
                "Cargo.lock: {name} comes from {source}, not from crates.io"
            ),
            Refusal::NotCdylib(what) => tr_in!(
                lang,
                "{what} : un plugin en Rust est une bibliothèque, crate-type = [\"cdylib\"] ([\"cdylib\", \"rlib\"] pour la partager avec ses tests)",
                "{what}: a plugin in Rust is a library, crate-type = [\"cdylib\"] ([\"cdylib\", \"rlib\"] to share it with its tests)"
            ),
            Refusal::OwnBuildScript(name) => tr_in!(
                lang,
                "{name} a un build.rs : un script s'exécute en natif à la compilation, un plugin n'en a pas",
                "{name} has a build.rs: a script runs in native while compiling, a plugin has none"
            ),
            Refusal::OwnMacro(name) => tr_in!(
                lang,
                "{name} est une macro procédurale : elle s'exécute en natif à la compilation, un plugin n'en a pas",
                "{name} is a procedural macro: it runs in native while compiling, a plugin has none"
            ),
            Refusal::NotListed(name, runs) => {
                let how = match (runs, lang) {
                    (Runs::Script, Lang::Fr) => "a un build.rs",
                    (Runs::Script, Lang::En) => "has a build.rs",
                    (Runs::Macro, Lang::Fr) => "est une macro procédurale",
                    (Runs::Macro, Lang::En) => "is a procedural macro",
                    (Runs::Under, Lang::Fr) => "sert un build.rs ou une macro",
                    (Runs::Under, Lang::En) => "serves a build.rs or a macro",
                };
                tr_in!(
                    lang,
                    "{name} {how} et n'est pas sur la liste relue : son code s'exécuterait en natif à la compilation (docs/PLUGINS.md §8.5.4)",
                    "{name} {how} and is not on the reviewed list: its code would run in native while compiling (docs/PLUGINS.md §8.5.4)"
                )
            }
            Refusal::Config(file) => tr_in!(
                lang,
                "{file} changerait la compilation : le store l'impose, retire-le",
                "{file} would change the compilation: the store sets it, remove it"
            ),
            Refusal::Profile(table) => tr_in!(
                lang,
                "[{table}] n'est pas permis : le profil d'un module est imposé",
                "[{table}] is not allowed: the profile of a module is imposed"
            ),
        }
    }
}

/// Why nothing was built: the plugin is refused, or the tools failed (a download, rustc on the author's code).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Refused(Refusal),
    Tools(String),
}

impl Failure {
    pub fn message(&self, lang: Lang) -> String {
        match self {
            Failure::Refused(r) => r.message(lang),
            Failure::Tools(why) => why.clone(),
        }
    }
}

impl From<Refusal> for Failure {
    fn from(r: Refusal) -> Self {
        Failure::Refused(r)
    }
}

/// A module, and what the store notes of how it was made.
#[derive(Debug, Clone)]
pub struct Built {
    pub bytes: Vec<u8>,
    /// The SHA-256 of `Cargo.lock`, in its LF form.
    pub lock: String,
}

/// `compiler` of `--json`, then of `version.json` (docs/PLUGINS.md §8.5.3).
pub fn compiler(lock: &str) -> Value {
    serde_json::json!({
        "rust": VERSION,
        "target": TARGET,
        "canon": canon::sha256(),
        "lock": lock,
    })
}

/// The triple of this system, when the toolchain has one.
fn host() -> Option<(&'static str, &'static [&'static str; 3])> {
    let triple = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        _ => return None,
    };
    HOSTS
        .iter()
        .find(|(t, _)| *t == triple)
        .map(|(t, sums)| (*t, sums))
}

/// The private toolchain's folder in the data folder `data`.
pub fn toolchain_dir(data: &Path) -> PathBuf {
    crate::paths::tools_dir(data).join(format!("rust-{VERSION}"))
}

/// A lock across processes, held until it is dropped: two compilations of a folder, or two downloads, wait for each
/// other. One left by a process that died is taken after [`STALE`].
struct Held(PathBuf);

impl Held {
    fn take(path: PathBuf) -> Result<Self, String> {
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_) => return Ok(Self(path)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let old = std::fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| SystemTime::now().duration_since(t).ok());
                    if old.is_some_and(|age| age > STALE) {
                        let _ = std::fs::remove_file(&path);
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
                // Windows says "access denied" while the other process removes it.
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && cfg!(windows) => {
                    std::thread::sleep(Duration::from_millis(250));
                }
                Err(e) => return Err(format!("{}: {e}", path.display())),
            }
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The folder of a toolchain's programs.
fn program(root: &Path, name: &str) -> PathBuf {
    root.join("bin").join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    })
}

/// Every file under `dir`, relative to it, with `/`.
fn files_under(dir: &Path, rest: &str, out: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if rest.is_empty() {
            name
        } else {
            format!("{rest}/{name}")
        };
        if entry.file_type()?.is_dir() {
            files_under(&entry.path(), &rel, out)?;
        } else {
            out.push(rel);
        }
    }
    Ok(())
}

/// The SHA-256 of a file, read in pieces.
fn sum_of(path: &Path) -> std::io::Result<String> {
    use sha2::Digest;
    let mut file = std::fs::File::open(path)?;
    let mut hash = sha2::Sha256::new();
    std::io::copy(&mut file, &mut hash)?;
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// `files` of `root`, each with its SHA-256, on every core: what a toolchain is checked by at each use.
fn sums(root: &Path, files: &[String]) -> Result<Vec<String>, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, usize::from);
    let parts: Vec<Result<Vec<(usize, String)>, String>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                s.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(rel) = files.get(i) else {
                            return Ok(done);
                        };
                        let sum = sum_of(&root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
                        done.push((i, sum));
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err("a sum's thread".into())))
            .collect()
    });
    let mut all = vec![String::new(); files.len()];
    for part in parts {
        for (i, sum) in part? {
            all[i] = sum;
        }
    }
    Ok(all)
}

/// Notes the sums of the files of the toolchain in `root`, once its archives are checked.
pub fn note_sums(root: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    files_under(root, "", &mut files).map_err(|e| e.to_string())?;
    files.retain(|f| f != SUMS);
    files.sort();
    let lines: Vec<String> = sums(root, &files)?
        .iter()
        .zip(&files)
        .map(|(sum, file)| format!("{sum}  {file}\n"))
        .collect();
    std::fs::write(root.join(SUMS), lines.concat()).map_err(|e| e.to_string())
}

/// Whether the toolchain in `root` is what was noted when its archives were checked.
pub fn same_as_noted(root: &Path) -> bool {
    let Ok(noted) = std::fs::read_to_string(root.join(SUMS)) else {
        return false;
    };
    let lines: Vec<(&str, &str)> = noted.lines().filter_map(|l| l.split_once("  ")).collect();
    let files: Vec<String> = lines.iter().map(|(_, f)| (*f).to_string()).collect();
    if files.is_empty() {
        return false;
    }
    sums(root, &files).is_ok_and(|got| got.iter().zip(&lines).all(|(sum, (want, _))| sum == want))
}

/// One archive of the toolchain to the file `to`, refused when it is not the pinned one.
fn download(url: &str, sum: &str, to: &Path) -> Result<(), String> {
    use sha2::Digest;
    wasm::block_on(async {
        let mut r = reqwest::Client::default()
            .get(url)
            .timeout(Duration::from_secs(30 * 60))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("{url}: {e}"))?;
        let mut file = std::fs::File::create(to).map_err(|e| e.to_string())?;
        let mut hash = sha2::Sha256::new();
        while let Some(piece) = r.chunk().await.map_err(|e| format!("{url}: {e}"))? {
            hash.update(&piece);
            file.write_all(&piece).map_err(|e| e.to_string())?;
        }
        let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if got != sum {
            return Err(format!(
                "{url}: its SHA-256 is {got}, not {sum}: not the pinned toolchain, refused"
            ));
        }
        Ok(())
    })
}

/// What an archive of the toolchain holds under `<package>/<component>/`, its `bin` and `lib`, into `root`. An archive
/// is read only once its SHA-256 is the pinned one, and only these paths come out of it.
pub fn unpack(archive: &Path, root: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    for entry in tar.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let parts: Vec<_> = path.components().collect();
        if parts.iter().any(|c| !matches!(c, Component::Normal(_))) {
            return Err(format!("{}: {}", archive.display(), path.display()));
        }
        let rest: PathBuf = parts.iter().skip(2).collect();
        if !(rest.starts_with("bin") || rest.starts_with("lib"))
            || entry.header().entry_type().is_dir()
        {
            continue;
        }
        let to = root.join(rest);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        entry
            .unpack(&to)
            .map_err(|e| format!("{}: {e}", to.display()))?;
    }
    Ok(())
}

/// The pinned toolchain, downloaded when it is not there; `say` tells it, as it takes a while.
fn pinned_toolchain(data: &Path, say: &dyn Fn()) -> Result<PathBuf, String> {
    let (triple, host_sums) = host().ok_or_else(|| {
        format!(
            "no rustc {VERSION} for {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let root = toolchain_dir(data);
    let tools = crate::paths::tools_dir(data);
    std::fs::create_dir_all(&tools).map_err(|e| e.to_string())?;
    let _held = Held::take(tools.join(format!("rust-{VERSION}.lock")))?;
    if root.is_dir() {
        if same_as_noted(&root) {
            return Ok(root);
        }
        log::warn!(
            "{}: not the toolchain checked at its download; downloaded again",
            root.display()
        );
        std::fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    }
    say();
    let work = tools.join(format!(".rust-{}", uuid::Uuid::new_v4().simple()));
    let unpacked = work.join("toolchain");
    std::fs::create_dir_all(&unpacked).map_err(|e| e.to_string())?;
    let result = (|| {
        let archives = [
            (format!("rustc-{VERSION}-{triple}"), host_sums[0]),
            (format!("cargo-{VERSION}-{triple}"), host_sums[1]),
            (format!("rust-std-{VERSION}-{triple}"), host_sums[2]),
            (format!("rust-std-{VERSION}-{TARGET}"), TARGET_STD),
        ];
        for (name, sum) in archives {
            let archive = work.join(format!("{name}.tar.gz"));
            download(&format!("{DIST}/{name}.tar.gz"), sum, &archive)?;
            unpack(&archive, &unpacked)?;
            std::fs::remove_file(&archive).map_err(|e| e.to_string())?;
        }
        note_sums(&unpacked)?;
        std::fs::rename(&unpacked, &root).map_err(|e| e.to_string())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result?;
    Ok(root)
}

/// The toolchain named by [`TOOLCHAIN_VAR`], once it is one that makes the same modules: this version, the target, and
/// no `rust-src`.
fn named_toolchain(root: PathBuf) -> Result<PathBuf, String> {
    let said = Command::new(program(&root, "rustc"))
        .arg("-V")
        .output()
        .map_err(|e| format!("{TOOLCHAIN_VAR}={}: {e}", root.display()))?;
    let said = String::from_utf8_lossy(&said.stdout);
    if !said.starts_with(&format!("rustc {VERSION} ")) {
        return Err(format!(
            "{TOOLCHAIN_VAR}={}: {}, not rustc {VERSION}",
            root.display(),
            said.trim()
        ));
    }
    let rustlib = root.join("lib").join("rustlib");
    if !rustlib.join(TARGET).is_dir() {
        return Err(format!("{TOOLCHAIN_VAR}={}: no {TARGET}", root.display()));
    }
    if rustlib.join("src").exists() {
        return Err(format!(
            "{TOOLCHAIN_VAR}={}: it has rust-src, whose paths enter the module",
            root.display()
        ));
    }
    Ok(root)
}

/// The toolchain of the data folder `data`.
pub fn toolchain(data: &Path, say: &dyn Fn()) -> Result<PathBuf, String> {
    match std::env::var_os(TOOLCHAIN_VAR).filter(|v| !v.is_empty()) {
        Some(named) => named_toolchain(PathBuf::from(named)),
        None => pinned_toolchain(data, say),
    }
}

/// The value of a key of a table, as a table.
fn table<'a>(of: &'a toml::Table, key: &str) -> Option<&'a toml::Table> {
    of.get(key).and_then(toml::Value::as_table)
}

/// `path` as `from`, a folder under `root`, names it: whether it stays in `root`, by its words alone (`root` need not
/// be on this machine for the store, and nothing here follows a link).
fn stays_in(path: &str, from: &Path, root: &Path) -> bool {
    let rel = Path::new(path);
    if rel.is_absolute() || path.starts_with('/') || path.starts_with('\\') {
        return false;
    }
    let mut depth = from
        .strip_prefix(root)
        .map_or(0, |r| r.components().count());
    for part in rel.components() {
        match part {
            Component::ParentDir => match depth.checked_sub(1) {
                Some(d) => depth = d,
                None => return false,
            },
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            _ => return false,
        }
    }
    true
}

/// A table of dependencies: what comes from elsewhere than crates.io or leaves the folder.
fn check_dependencies(deps: Option<&toml::Table>, from: &Path, root: &Path) -> Result<(), Refusal> {
    for (name, spec) in deps.into_iter().flatten() {
        let Some(spec) = spec.as_table() else {
            continue;
        };
        if ["git", "registry", "registry-index"]
            .iter()
            .any(|k| spec.contains_key(*k))
        {
            return Err(Refusal::NotCratesIo(name.clone()));
        }
        if let Some(path) = spec.get("path").and_then(toml::Value::as_str) {
            if !stays_in(path, from, root) {
                return Err(Refusal::PathOutside(name.clone(), path.to_string()));
            }
        }
    }
    Ok(())
}

/// Every table of dependencies of a manifest.
fn dependency_tables(manifest: &toml::Table) -> Vec<&toml::Table> {
    const KINDS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let mut found: Vec<&toml::Table> = KINDS.iter().filter_map(|k| table(manifest, k)).collect();
    for platform in table(manifest, "target")
        .into_iter()
        .flat_map(|t| t.values())
    {
        if let Some(platform) = platform.as_table() {
            found.extend(KINDS.iter().filter_map(|k| table(platform, k)));
        }
    }
    found.extend(table(manifest, "workspace").and_then(|w| table(w, "dependencies")));
    found
}

/// The crate's targets: a `cdylib` alone, or with an `rlib`. Nothing else is built, and no bin.
fn check_targets(manifest: &toml::Table, dir: &Path, name: &str) -> Result<(), Refusal> {
    let lib = table(manifest, "lib");
    if lib.is_some_and(|l| l.get("proc-macro").and_then(toml::Value::as_bool) == Some(true)) {
        return Err(Refusal::OwnMacro(name.to_string()));
    }
    let types: Vec<&str> = lib
        .and_then(|l| l.get("crate-type").or_else(|| l.get("crate_type")))
        .and_then(toml::Value::as_array)
        .map(|a| a.iter().filter_map(toml::Value::as_str).collect())
        .unwrap_or_default();
    if let Some(other) = types.iter().find(|t| !matches!(**t, "cdylib" | "rlib")) {
        return Err(Refusal::NotCdylib(format!("crate-type {other}")));
    }
    if !types.contains(&"cdylib") {
        return Err(Refusal::NotCdylib(format!("[lib] of {name}")));
    }
    let auto = table(manifest, "package")
        .and_then(|p| p.get("autobins"))
        .and_then(toml::Value::as_bool)
        != Some(false);
    if manifest.contains_key("bin") {
        return Err(Refusal::NotCdylib("[[bin]]".into()));
    }
    if auto && dir.join("src").join("main.rs").is_file() {
        return Err(Refusal::NotCdylib("src/main.rs".into()));
    }
    if auto && dir.join("src").join("bin").is_dir() {
        return Err(Refusal::NotCdylib("src/bin".into()));
    }
    Ok(())
}

/// A `.cargo/config*` or a `rust-toolchain*` under `dir`, as a path from `root`.
fn config_under(dir: &Path, root: &Path) -> Option<String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
    entries.sort_by_key(std::fs::DirEntry::path);
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        let shown = || {
            path.strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
        };
        if path.is_dir() {
            if NOT_COPIED.contains(&name.as_str()) {
                continue;
            }
            let config = name == ".cargo"
                && (path.join("config.toml").is_file() || path.join("config").is_file());
            if config {
                return Some(shown());
            }
            if let Some(found) = config_under(&path, root) {
                return Some(found);
            }
        } else if name == "rust-toolchain" || name == "rust-toolchain.toml" {
            return Some(shown());
        }
    }
    None
}

/// What reads a plugin's folder without running anything: why it would not be compiled. Gives its `Cargo.lock`, read.
/// `validate` says the same, sooner. The rest of the closed list needs cargo ([`check_packages`]).
pub fn check(folder: &Path, cargo_file: &str) -> Result<toml::Table, Refusal> {
    let path = folder.join(cargo_file);
    let dir = path.parent().unwrap_or(folder).to_path_buf();
    if !stays_in(cargo_file, folder, folder) {
        return Err(Refusal::NoManifest(cargo_file.to_string()));
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Err(Refusal::NoManifest(cargo_file.to_string()));
    };
    let manifest: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| Refusal::BadManifest(e.message().to_string()))?;
    if let Some(file) = config_under(folder, folder) {
        return Err(Refusal::Config(file));
    }
    for key in ["patch", "replace"] {
        if manifest.contains_key(key) {
            return Err(Refusal::Patch(key.to_string()));
        }
    }
    for (profile, values) in table(&manifest, "profile").into_iter().flatten() {
        let values = values.as_table();
        for key in ["package", "build-override"] {
            if values.is_some_and(|v| v.contains_key(key)) {
                return Err(Refusal::Profile(format!("profile.{profile}.{key}")));
            }
        }
    }
    for deps in dependency_tables(&manifest) {
        check_dependencies(Some(deps), &dir, folder)?;
    }
    if let Some(members) = table(&manifest, "workspace")
        .and_then(|w| w.get("members"))
        .and_then(toml::Value::as_array)
    {
        for member in members.iter().filter_map(toml::Value::as_str) {
            if !stays_in(member, &dir, folder) {
                return Err(Refusal::PathOutside(
                    "workspace.members".into(),
                    member.to_string(),
                ));
            }
        }
    }
    let package = table(&manifest, "package");
    if let Some(root) = package
        .and_then(|p| p.get("workspace"))
        .and_then(toml::Value::as_str)
    {
        if !stays_in(root, &dir, folder) {
            return Err(Refusal::PathOutside(
                "package.workspace".into(),
                root.to_string(),
            ));
        }
    }
    let name = package
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str)
        .unwrap_or("plugin");
    check_targets(&manifest, &dir, name)?;
    let build = package.and_then(|p| p.get("build"));
    let no_script = build.is_some_and(|b| b.as_bool() == Some(false));
    if !no_script && (build.is_some() || dir.join("build.rs").is_file()) {
        return Err(Refusal::OwnBuildScript(name.to_string()));
    }
    let lock_file = dir.join("Cargo.lock");
    let shown = Path::new(cargo_file)
        .with_file_name("Cargo.lock")
        .to_string_lossy()
        .replace('\\', "/");
    let lock = std::fs::read_to_string(&lock_file).map_err(|_| Refusal::NoLock(shown))?;
    let lock: toml::Table = lock
        .parse()
        .map_err(|e: toml::de::Error| Refusal::BadLock(e.message().to_string()))?;
    for package in lock
        .get("package")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
    {
        let source = package.get("source").and_then(toml::Value::as_str);
        if let Some(source) = source.filter(|s| !CRATES_IO.contains(s)) {
            let name = package
                .get("name")
                .and_then(toml::Value::as_str)
                .unwrap_or("?");
            return Err(Refusal::LockSource(name.to_string(), source.to_string()));
        }
    }
    Ok(lock)
}

/// The rest of the closed list, on `cargo metadata`: what a package of the plugin holds, and every crate that would run
/// in native, which are those with a build script, the procedural macros, and all that these two depend on. What
/// compiles to the module runs in WASM alone: it needs no list.
pub fn check_packages(
    metadata: &Value,
    lock: &toml::Table,
    root: &Path,
    list: &[Native],
) -> Result<(), Refusal> {
    let packages: HashMap<&str, &Value> = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| Some((p["id"].as_str()?, p)))
        .collect();
    let nodes: HashMap<&str, &Value> = metadata["resolve"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| Some((n["id"].as_str()?, n)))
        .collect();
    let checksums: HashMap<(&str, &str), &str> = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| {
            Some((
                (p.get("name")?.as_str()?, p.get("version")?.as_str()?),
                p.get("checksum")?.as_str()?,
            ))
        })
        .collect();
    let has = |package: &Value, kind: &str| {
        package["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|t| {
                t["kind"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|k| k.as_str() == Some(kind))
            })
    };
    let Some(start) = metadata["resolve"]["root"].as_str() else {
        return Err(Refusal::BadManifest("cargo metadata: no root".into()));
    };
    let (mut seen, mut stack) = (HashSet::new(), vec![(start, false)]);
    while let Some((id, under)) = stack.pop() {
        if !seen.insert((id, under)) {
            continue;
        }
        let (Some(package), Some(node)) = (packages.get(id), nodes.get(id)) else {
            continue;
        };
        let (name, version) = (
            package["name"].as_str().unwrap_or("?"),
            package["version"].as_str().unwrap_or("?"),
        );
        let (script, proc_macro) = (has(package, "custom-build"), has(package, "proc-macro"));
        let local = package["source"].is_null();
        if local {
            let inside = package["manifest_path"]
                .as_str()
                .is_some_and(|m| Path::new(m).starts_with(root));
            if !inside {
                return Err(Refusal::PathOutside(
                    name.to_string(),
                    package["manifest_path"].as_str().unwrap_or("?").to_string(),
                ));
            }
            if script {
                return Err(Refusal::OwnBuildScript(name.to_string()));
            }
            if proc_macro || under {
                return Err(Refusal::OwnMacro(name.to_string()));
            }
        } else if script || proc_macro || under {
            let listed = checksums.get(&(name, version)).is_some_and(|sum| {
                list.iter()
                    .any(|n| n.name == name && n.version == version && n.checksum == *sum)
            });
            if !listed {
                let runs = if proc_macro {
                    Runs::Macro
                } else if script {
                    Runs::Script
                } else {
                    Runs::Under
                };
                return Err(Refusal::NotListed(format!("{name} {version}"), runs));
            }
        }
        for dep in node["deps"].as_array().into_iter().flatten() {
            let Some(next) = dep["pkg"].as_str() else {
                continue;
            };
            let kinds = dep["dep_kinds"].as_array().into_iter().flatten();
            let (mut normal, mut build) = (false, false);
            for kind in kinds {
                match kind["kind"].as_str() {
                    None | Some("normal") => normal = true,
                    Some("build") => build = true,
                    _ => {}
                }
            }
            if normal {
                stack.push((next, under || proc_macro));
            }
            if build {
                stack.push((next, true));
            }
        }
    }
    Ok(())
}

/// Copies the plugin, without what no compilation reads.
fn copy_plugin(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if NOT_COPIED.iter().any(|n| name == *n) {
            continue;
        }
        let path = entry.path();
        if std::fs::metadata(&path)?.is_dir() {
            copy_plugin(&path, &to.join(&name))?;
        } else {
            std::fs::copy(&path, to.join(&name))?;
        }
    }
    Ok(())
}

/// Cargo in `dir`, with nothing of the machine's own: no flags, no wrapper, no profile of the environment. A network's
/// settings stay.
fn cargo(root: &Path, home: &Path, dir: &Path, canon_root: &Path, wrapper: &Path) -> Command {
    let mut cmd = Command::new(program(root, "cargo"));
    for (key, _) in std::env::vars_os() {
        let upper = key.to_string_lossy().to_ascii_uppercase();
        let network = upper.starts_with("CARGO_HTTP_") || upper.starts_with("CARGO_NET_");
        if (upper.starts_with("CARGO") && !network) || upper.starts_with("RUST") {
            cmd.env_remove(&key);
        }
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![root.join("bin")];
    paths.extend(std::env::split_paths(&path));
    if let Ok(joined) = std::env::join_paths(paths) {
        cmd.env("PATH", joined);
    }
    cmd.env("CARGO_HOME", home)
        .env("RUSTC", program(root, "rustc"))
        .env("RUSTC_WRAPPER", wrapper)
        .env("CARGO_ENCODED_RUSTFLAGS", "")
        .env("CARGO_TERM_COLOR", "never")
        .env(canon::ROOT, canon_root)
        .current_dir(dir)
        .stdin(std::process::Stdio::null());
    for (key, value) in canon::PROFILE {
        cmd.env(format!("CARGO_PROFILE_RELEASE_{key}"), value);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

/// What the author's compilation said, cut to its end.
fn tail(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    let start = text.len().saturating_sub(4000);
    let start = (start..text.len())
        .find(|i| text.is_char_boundary(*i))
        .unwrap_or(0);
    text[start..].to_string()
}

/// The name of the module cargo writes, from the manifest in `dir`.
fn module_name(dir: &Path) -> Result<String, Failure> {
    let text = std::fs::read_to_string(dir.join("Cargo.toml"))
        .map_err(|e| Failure::Tools(e.to_string()))?;
    let manifest: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| Failure::Tools(e.to_string()))?;
    let name = table(&manifest, "lib")
        .and_then(|l| l.get("name"))
        .or_else(|| table(&manifest, "package").and_then(|p| p.get("name")))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| Failure::Refused(Refusal::BadManifest("no package.name".into())))?;
    Ok(format!("{}.wasm", name.replace('-', "_")))
}

/// The copy of a plugin a compilation works on, held until it is dropped.
struct Copy {
    _held: Held,
    work: PathBuf,
    /// The copy's root.
    src: PathBuf,
    /// The folder of its `Cargo.toml`.
    dir: PathBuf,
}

/// `folder` copied to where it compiles, by its path and the compiler, for nothing around it to count.
fn copy_of(folder: &Path, cargo_file: &str, data: &Path) -> Result<Copy, Failure> {
    let tool = |e: std::io::Error| Failure::Tools(e.to_string());
    let absolute = std::path::absolute(folder).map_err(tool)?;
    let key = wasm::sha256(absolute.to_string_lossy().as_bytes());
    // By the compiler too: a target made by another rustc or wrapper gives a module that says nothing.
    let work = crate::paths::plugin_rust_dir(data)
        .join(format!("{VERSION}-{}", &canon::sha256()[..12]))
        .join(&key[..16]);
    std::fs::create_dir_all(&work).map_err(tool)?;
    let _held = Held::take(work.join("lock")).map_err(Failure::Tools)?;
    let src = work.join("src");
    if src.exists() {
        std::fs::remove_dir_all(&src).map_err(tool)?;
    }
    copy_plugin(folder, &src).map_err(tool)?;
    let dir = src.join(cargo_file);
    let dir = dir.parent().unwrap_or(&src).to_path_buf();
    Ok(Copy {
        _held,
        work,
        src,
        dir,
    })
}

/// Whether what `cargo fetch --locked` said is about the plugin's lock, not about the network.
pub fn lock_at_fault(said: &str) -> bool {
    [
        "--locked",
        "lock file",
        "failed to select a version",
        "no matching package",
        "checksum",
    ]
    .iter()
    .any(|word| said.contains(word))
}

/// Downloads the dependencies of the `run.wasm` plugin of `folder` from crates.io into the `CARGO_HOME` of the data
/// folder `data`, where `build` finds them without a network (docs/PLUGINS.md §8.5.3). Nothing of the plugin runs, and
/// [`check`] comes first: a `.cargo/config.toml` or a `[patch]` would otherwise choose where cargo goes.
pub fn fetch(
    folder: &Path,
    cargo_file: &str,
    data: &Path,
    wrapper: &Path,
    say: &dyn Fn(),
) -> Result<(), Failure> {
    check(folder, cargo_file)?;
    let root = toolchain(data, say).map_err(Failure::Tools)?;
    let copy = copy_of(folder, cargo_file, data)?;
    let home = crate::paths::tools_dir(data).join("cargo-home");
    let o = cargo(&root, &home, &copy.dir, &copy.src, wrapper)
        .args(["fetch", "--locked"])
        .output()
        .map_err(|e| Failure::Tools(e.to_string()))?;
    if o.status.success() {
        return Ok(());
    }
    let said = tail(&o.stderr);
    Err(if said.contains("manifest") {
        Failure::Refused(Refusal::BadManifest(said))
    } else if lock_at_fault(&said) {
        Failure::Refused(Refusal::StaleLock(said))
    } else {
        Failure::Tools(said)
    })
}

/// Compiles the `run.wasm` plugin of `folder` (`cargo_file` is its `run.wasm`), with the tools of the data folder `data`.
/// `wrapper` is the `escouade-plugin` that serves as `rustc-canon`; `say` is told when the toolchain must be downloaded.
pub fn build(
    folder: &Path,
    cargo_file: &str,
    data: &Path,
    wrapper: &Path,
    say: &dyn Fn(),
) -> Result<Built, Failure> {
    let tool = |e: std::io::Error| Failure::Tools(e.to_string());
    let lock = check(folder, cargo_file)?;
    let root = toolchain(data, say).map_err(Failure::Tools)?;
    let Copy {
        _held,
        work,
        src,
        dir,
    } = copy_of(folder, cargo_file, data)?;
    let home = crate::paths::tools_dir(data).join("cargo-home");
    let command = || cargo(&root, &home, &dir, &src, wrapper);
    let target = work.join("target");

    let mut metadata = command();
    metadata.args([
        "metadata",
        "--locked",
        "--format-version=1",
        "--filter-platform",
        TARGET,
    ]);
    let o = metadata.output().map_err(tool)?;
    if !o.status.success() {
        return Err(Failure::Tools(tail(&o.stderr)));
    }
    let read: Value =
        serde_json::from_slice(&o.stdout).map_err(|e| Failure::Tools(e.to_string()))?;
    check_packages(&read, &lock, &src, &native_list())?;

    let mut compile = command();
    compile
        .args([
            "build",
            "--release",
            "--locked",
            "--quiet",
            "--target",
            TARGET,
        ])
        .env("CARGO_TARGET_DIR", &target);
    let o = compile.output().map_err(tool)?;
    if !o.status.success() {
        return Err(Failure::Tools(tail(&o.stderr)));
    }
    let name = module_name(&dir)?;
    let bytes = std::fs::read(target.join(TARGET).join("release").join(&name)).map_err(tool)?;
    wasm::reactor_form(&bytes).map_err(Failure::Tools)?;
    let text = std::fs::read_to_string(dir.join("Cargo.lock")).map_err(tool)?;
    let lock = wasm::sha256(text.replace("\r\n", "\n").as_bytes());
    Ok(Built { bytes, lock })
}
