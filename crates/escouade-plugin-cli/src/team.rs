//! The checks of a team's private store (TS-14, `docs/TEAM-SERVER.md` §7): those of the public store that do not need
//! its database, played on the machine of who publishes, then again on the machine of the administrator who signs. The
//! team's server is blind (C10-A): it cannot play them.
//!
//! - **The archive** ([`pack`], [`unpack`]): the plugin's folder as a tar.gz made the same way on every system (its
//!   paths sorted, no date, no owner, no mode but 0644), its history, its `packs/` and its `target/` left out. Never a
//!   link, never a path that climbs out by `..` or starts at a root, never a special file, 50 MB at most packed and
//!   200 MB unpacked, no submodule, no Git LFS pointer.
//! - **The manifest** and what `escouade-plugin validate` says of it, for anyone's hand: a team never signs
//!   `dev.escouade.*`.
//! - **The version**: the tag's (`v1.2.0` or `1.2.0`), and above the last the team published.
//! - **The permissions** coherent with the contributions, **no binary** (no native program: everything runs in WASM).
//! - **The compilation, twice**, each in a folder of its own made for it: the two modules must be the same bytes. Their
//!   SHA-256, with the archive's, makes the report's [`Report::digest`], which the administrator who signs must find
//!   again on their machine.
//!
//! What compiles is a [`Compile`]: the app's toolchains (`Toolchains`: the pinned Javy, the private rustc), or a test's.

use crate::i18n::Lang;
use crate::manifest::{self, Manifest};
use crate::{rust, wasm};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// The largest archive (packed), as the public store's.
pub const ARCHIVE_MAX: u64 = 50 * 1024 * 1024;
/// The most an archive unpacks to.
pub const UNPACKED_MAX: u64 = 200 * 1024 * 1024;
/// The most files an archive holds.
pub const ENTRIES_MAX: usize = 20_000;
/// The folders [`pack`] leaves out: the history, the modules `build` writes, Cargo's and npm's.
pub const LEFT_OUT: &[&str] = &[".git", "packs", "target", "node_modules"];
/// The checks, in their order.
pub const CHECKS: &[&str] = &[
    "archive",
    "manifest",
    "version",
    "permissions",
    "binaries",
    "compile",
];

const LFS_POINTER: &[u8] = b"version https://git-lfs.github.com/spec/v1";

/// A check and what it found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub key: String,
    pub ok: bool,
    /// What it found, in words: why it fails, or what passed.
    pub detail: String,
}

/// The report of the checks of one archive.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// The plugin's id and version, as its manifest says (empty when it does not read).
    pub id: String,
    pub version: String,
    pub archive_sha256: String,
    /// The SHA-256 of the module both compilations made; None for a declarative plugin.
    pub module_sha256: Option<String>,
    /// The two things together: what the administrator who signs must find again.
    pub digest: String,
    pub checks: Vec<Check>,
    /// Every check passed.
    pub ok: bool,
    /// The module compiled (the same bytes twice), to publish beside the archive.
    #[serde(skip)]
    pub module: Option<Vec<u8>>,
    /// The manifest, when it reads.
    #[serde(skip)]
    pub manifest: Option<Manifest>,
}

impl Report {
    /// The first check that failed, in words.
    pub fn failure(&self) -> Option<String> {
        self.checks
            .iter()
            .find(|c| !c.ok)
            .map(|c| format!("{} : {}", c.key, c.detail))
    }
}

/// The digest of an archive and its module: what both machines must find.
pub fn digest(archive_sha256: &str, module_sha256: Option<&str>) -> String {
    let text = format!(
        "escouade-team-plugin-v1\n{archive_sha256}\n{}\n",
        module_sha256.unwrap_or("-")
    );
    hex(&Sha256::digest(text.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// What compiles a plugin's module in a folder of its own.
pub trait Compile {
    /// The module of the plugin of manifest `m`, whose files are in `folder`.
    fn compile(&self, folder: &Path, m: &Manifest, lang: Lang) -> Result<Vec<u8>, String>;
}

/// The app's toolchains: the pinned Javy for a `run.js` plugin, the private rustc for a `run.wasm` one, in the data
/// folder `data` (their downloads, the copies of the compilations).
pub struct Toolchains {
    pub data: PathBuf,
}

impl Compile for Toolchains {
    fn compile(&self, folder: &Path, m: &Manifest, lang: Lang) -> Result<Vec<u8>, String> {
        let run = m.run.as_ref();
        if let Some(js) = run.and_then(|r| r.js.as_ref()) {
            let path = folder.join(js);
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            wasm::script_of(&text).map_err(|r| r.message(lang))?;
            return wasm::Compiler::of_data(&self.data).build(&text);
        }
        if let Some(cargo_file) = run.and_then(|r| r.wasm.as_ref()) {
            return crate::cli::module_of_crate(folder, cargo_file, &self.data, lang)
                .map(|b| b.bytes);
        }
        Err("nothing to compile".into())
    }
}

// === The archive ===

/// The plugin's folder `folder` as a tar.gz made the same way everywhere (its files under `plugin/`, as GitHub puts
/// a commit's under a folder of its own). A link, or a special file, is refused: the store follows none.
pub fn pack(folder: &Path, lang: Lang) -> Result<Vec<u8>, String> {
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    walk(folder, folder, &mut files, lang)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let gz = flate2::GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);
    tar.mode(tar::HeaderMode::Deterministic);
    let mut total: u64 = 0;
    for (rel, path) in &files {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        total += bytes.len() as u64;
        if total > UNPACKED_MAX {
            return Err(tr_in!(
                lang,
                "le dossier pèse plus de 200 Mo",
                "the folder weighs more than 200 MB"
            ));
        }
        let mut h = tar::Header::new_gnu();
        h.set_entry_type(tar::EntryType::Regular);
        h.set_size(bytes.len() as u64);
        h.set_mode(0o644);
        h.set_mtime(0);
        h.set_uid(0);
        h.set_gid(0);
        tar.append_data(&mut h, format!("plugin/{rel}"), bytes.as_slice())
            .map_err(|e| e.to_string())?;
    }
    let gz = tar.into_inner().map_err(|e| e.to_string())?;
    let out = gz.finish().map_err(|e| e.to_string())?;
    if out.len() as u64 > ARCHIVE_MAX {
        return Err(too_big(out.len() as u64, lang));
    }
    Ok(out)
}

fn too_big(size: u64, lang: Lang) -> String {
    let mb = size as f64 / 1_048_576.0;
    tr_in!(
        lang,
        "l'archive pèse {mb:.1} Mo, 50 Mo au plus",
        "the archive weighs {mb:.1} MB, 50 MB at most"
    )
}

fn walk(
    root: &Path,
    dir: &Path,
    out: &mut Vec<(String, PathBuf)>,
    lang: Lang,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let kind = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if kind.file_type().is_symlink() {
            return Err(tr_in!(
                lang,
                "{rel} est un lien symbolique : le store n'en suit aucun",
                "{rel} is a symbolic link: the store follows none"
            ));
        }
        if kind.is_dir() {
            if dir == root && LEFT_OUT.contains(&name.as_str()) {
                continue;
            }
            walk(root, &path, out, lang)?;
        } else if kind.is_file() {
            out.push((rel, path));
        } else {
            return Err(tr_in!(
                lang,
                "{rel} n'est pas un fichier ordinaire",
                "{rel} is no plain file"
            ));
        }
        if out.len() > ENTRIES_MAX {
            return Err(tr_in!(
                lang,
                "le dossier a plus de 20 000 fichiers",
                "the folder has more than 20,000 files"
            ));
        }
    }
    Ok(())
}

/// The regular files of an archive, by their path in the plugin (its top folder taken off), or every problem it has:
/// a link, a special file, a path absolute or that climbs out, too big, too many files, a submodule, an LFS pointer.
pub fn unpack(gz: &[u8], lang: Lang) -> Result<Vec<(String, Vec<u8>)>, Vec<String>> {
    if gz.len() as u64 > ARCHIVE_MAX {
        return Err(vec![too_big(gz.len() as u64, lang)]);
    }
    let reader = Bounded {
        inner: flate2::read::GzDecoder::new(gz),
        left: UNPACKED_MAX,
    };
    let mut ar = tar::Archive::new(reader);
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    let unreadable = |e: std::io::Error| {
        vec![tr_in!(
            lang,
            "l'archive ne se lit pas : {e}",
            "the archive does not read: {e}"
        )]
    };
    let entries = ar.entries().map_err(unreadable)?;
    for (n, entry) in entries.enumerate() {
        if n >= ENTRIES_MAX {
            return Err(vec![tr_in!(
                lang,
                "l'archive a plus de 20 000 fichiers",
                "the archive has more than 20,000 files"
            )]);
        }
        let mut entry = entry.map_err(unreadable)?;
        let kind = entry.header().entry_type();
        let raw = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        use tar::EntryType as T;
        match kind {
            T::XGlobalHeader | T::XHeader | T::GNULongName | T::GNULongLink => continue,
            T::Directory => {
                if let Err(p) = inner_path(&raw, lang) {
                    problems.push(p);
                }
                continue;
            }
            T::Regular | T::Continuous => {}
            T::Symlink => {
                problems.push(tr_in!(
                    lang,
                    "{raw} est un lien symbolique",
                    "{raw} is a symbolic link"
                ));
                continue;
            }
            T::Link => {
                problems.push(tr_in!(
                    lang,
                    "{raw} est un lien physique",
                    "{raw} is a hard link"
                ));
                continue;
            }
            _ => {
                problems.push(tr_in!(
                    lang,
                    "{raw} n'est pas un fichier ordinaire",
                    "{raw} is no plain file"
                ));
                continue;
            }
        }
        let path = match inner_path(&raw, lang) {
            Ok(p) => p,
            Err(p) => {
                problems.push(p);
                continue;
            }
        };
        if path.is_empty() {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(unreadable)?;
        if out.iter().any(|(p, _)| p == &path) {
            problems.push(tr_in!(
                lang,
                "{path} est deux fois dans l'archive",
                "{path} is twice in the archive"
            ));
            continue;
        }
        if path == ".gitmodules" {
            problems.push(tr_in!(
                lang,
                "le dépôt a des sous-modules : leur code n'est pas dans l'archive",
                "the repository has submodules: their code is not in the archive"
            ));
        }
        if bytes.len() < 1024 && bytes.starts_with(LFS_POINTER) {
            problems.push(tr_in!(
                lang,
                "{path} est un pointeur Git LFS : son contenu n'est pas dans l'archive",
                "{path} is a Git LFS pointer: its content is not in the archive"
            ));
        }
        out.push((path, bytes));
    }
    if problems.is_empty() {
        Ok(out)
    } else {
        Err(problems)
    }
}

/// A path of the archive inside the plugin, its top folder taken off; refused when it is absolute or climbs out.
fn inner_path(raw: &str, lang: Lang) -> Result<String, String> {
    let p = raw.replace('\\', "/");
    let b = p.as_bytes();
    if p.starts_with('/') || (b.len() >= 2 && b[1] == b':') || p.contains('\0') {
        return Err(tr_in!(
            lang,
            "{raw} est un chemin absolu",
            "{raw} is an absolute path"
        ));
    }
    let parts: Vec<&str> = p
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    if parts.contains(&"..") {
        return Err(tr_in!(
            lang,
            "{raw} sort du dossier par ..",
            "{raw} climbs out with .."
        ));
    }
    Ok(parts.iter().skip(1).copied().collect::<Vec<_>>().join("/"))
}

/// A reader that stops past `left` bytes: a gzip bomb is refused before it fills the memory.
struct Bounded<R> {
    inner: R,
    left: u64,
}

impl<R: Read> Read for Bounded<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n as u64 > self.left {
            return Err(std::io::Error::other("the archive unpacks past 200 MB"));
        }
        self.left -= n as u64;
        Ok(n)
    }
}

/// A folder of its own, gone when dropped.
struct Fresh(PathBuf);

impl Fresh {
    fn new(files: &[(String, Vec<u8>)]) -> std::io::Result<Fresh> {
        let dir = std::env::temp_dir().join(format!(
            "escouade-team-check-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&dir)?;
        let fresh = Fresh(dir);
        for (rel, bytes) in files {
            let to = fresh.0.join(rel);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::File::create(&to)?.write_all(bytes)?;
        }
        Ok(fresh)
    }
}

impl Drop for Fresh {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// === The checks ===

/// What the archive is checked against.
pub struct Against<'a> {
    /// The tag of the version (`v1.2.0` or `1.2.0`).
    pub tag: &'a str,
    /// The last version of this plugin the team published, if any.
    pub last: Option<&'a str>,
    /// The version of Escouade it is judged for.
    pub app_version: &'a str,
}

/// Plays the checks on the archive `gz`: what the machine of who publishes does, then the administrator's before
/// signing. The compilation runs twice, each in a folder of its own made for it.
pub fn check(gz: &[u8], against: &Against, compiler: &dyn Compile, lang: Lang) -> Report {
    let archive_sha256 = hex(&Sha256::digest(gz));
    let mut checks: Vec<Check> = Vec::new();
    let (m, module) = play(gz, against, compiler, lang, &mut checks);
    let module_sha256 = module.as_deref().map(|b| hex(&Sha256::digest(b)));
    let ok = checks.len() == CHECKS.len() && checks.iter().all(|c| c.ok);
    Report {
        id: m.as_ref().map(|m| m.id.clone()).unwrap_or_default(),
        version: m.as_ref().map(|m| m.version.clone()).unwrap_or_default(),
        digest: digest(&archive_sha256, module_sha256.as_deref()),
        archive_sha256,
        module_sha256,
        checks,
        ok,
        module,
        manifest: m,
    }
}

fn push(checks: &mut Vec<Check>, key: &str, r: Result<String, String>) {
    let ok = r.is_ok();
    checks.push(Check {
        key: key.into(),
        ok,
        detail: r.unwrap_or_else(|e| e),
    });
}

/// The checks one after the other: the manifest and the module, when they hold.
fn play(
    gz: &[u8],
    against: &Against,
    compiler: &dyn Compile,
    lang: Lang,
    checks: &mut Vec<Check>,
) -> (Option<Manifest>, Option<Vec<u8>>) {
    // 1. The archive.
    let files = match unpack(gz, lang) {
        Ok(f) => f,
        Err(problems) => {
            push(checks, "archive", Err(problems.join(" ; ")));
            return (None, None);
        }
    };
    let size = gz.len() as f64 / 1_048_576.0;
    push(
        checks,
        "archive",
        Ok(tr_in!(
            lang,
            "{size:.1} Mo, {} fichiers, ni lien ni chemin qui sorte",
            "{size:.1} MB, {} files, no link nor path that leaves",
            files.len()
        )),
    );
    let first = match Fresh::new(&files) {
        Ok(f) => f,
        Err(e) => {
            push(checks, "manifest", Err(e.to_string()));
            return (None, None);
        }
    };
    // 2. The manifest, for anyone's hand.
    let m = match manifest_of(&first.0, against.app_version, lang) {
        Ok(m) => m,
        Err(why) => {
            push(checks, "manifest", Err(why));
            return (None, None);
        }
    };
    push(
        checks,
        "manifest",
        Ok(tr_in!(
            lang,
            "{} {} : le manifeste est valide",
            "{} {}: the manifest holds",
            m.id,
            m.version
        )),
    );
    // 3. The version, 4. the permissions, 5. no binary.
    push(checks, "version", check_version(&m, against, lang));
    push(checks, "permissions", check_permissions(&m, lang));
    push(checks, "binaries", check_binaries(&m, lang));
    // 6. The compilation, twice.
    let module = match check_compile(&m, &files, &first, compiler, lang) {
        Ok((detail, module)) => {
            push(checks, "compile", Ok(detail));
            module
        }
        Err(why) => {
            push(checks, "compile", Err(why));
            None
        }
    };
    (Some(m), module)
}

fn manifest_of(dir: &Path, app_version: &str, lang: Lang) -> Result<Manifest, String> {
    let path = dir.join(manifest::FILE);
    let text = std::fs::read_to_string(&path).map_err(|_| {
        tr_in!(
            lang,
            "pas de {} à la racine de l'archive",
            "no {} at the root of the archive",
            manifest::FILE
        )
    })?;
    let read = manifest::read_in(&text, manifest::Hand::Third, dir);
    let m = read.manifest.map_err(|problems| {
        problems
            .iter()
            .map(|p| p.message(lang))
            .collect::<Vec<_>>()
            .join(" ; ")
    })?;
    if let Some(why) = m.refusal(app_version, lang) {
        return Err(why);
    }
    if let Some(js) = m.run.as_ref().and_then(|r| r.js.as_ref()) {
        let text = std::fs::read_to_string(dir.join(js)).map_err(|_| {
            tr_in!(
                lang,
                "{js} n'est pas dans l'archive",
                "{js} is not in the archive"
            )
        })?;
        wasm::script_of(&text).map_err(|r| r.message(lang))?;
    }
    if let Some(cargo_file) = m.run.as_ref().and_then(|r| r.wasm.as_ref()) {
        rust::check(dir, cargo_file).map_err(|r| r.message(lang))?;
    }
    Ok(m)
}

/// The tag a version takes: `v1.2.0` or `1.2.0`.
pub fn tag_names(tag: &str, version: &str) -> bool {
    let tag = tag.trim();
    tag == version || tag.strip_prefix('v') == Some(version)
}

fn check_version(m: &Manifest, against: &Against, lang: Lang) -> Result<String, String> {
    if !tag_names(against.tag, &m.version) {
        return Err(tr_in!(
            lang,
            "l'étiquette {} n'est pas la version {} du manifeste",
            "the tag {} is not the manifest's version {}",
            against.tag,
            m.version
        ));
    }
    if let Some(last) = against.last {
        if manifest::Version::parse(&m.version) <= manifest::Version::parse(last) {
            return Err(tr_in!(
                lang,
                "la version {} n'est pas plus grande que la dernière publiée ({last})",
                "version {} is not above the last one published ({last})",
                m.version
            ));
        }
        return Ok(tr_in!(
            lang,
            "{}, après la {last}",
            "{}, after {last}",
            m.version
        ));
    }
    Ok(tr_in!(lang, "{}, la première", "{}, the first", m.version))
}

fn ids_of(c: &serde_json::Map<String, Value>, point: &str, field: &str) -> Vec<String> {
    c.get(point)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.get(field).and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn host_name(host: &str) -> bool {
    let host = host.strip_prefix("*.").unwrap_or(host);
    let (name, port) = match host.rsplit_once(':') {
        Some((n, p)) => (n, Some(p)),
        None => (host, None),
    };
    let labels: Vec<&str> = name.split('.').collect();
    labels.len() >= 2
        && labels
            .iter()
            .all(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        && port.is_none_or(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// The permissions coherent with the contributions, as the public store's ninth check.
fn check_permissions(m: &Manifest, lang: Lang) -> Result<String, String> {
    let p = &m.permissions;
    let c = &m.contributes;
    let runs = m.run.is_some();
    let webhooks = c
        .get("webhooks")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty());
    let declared = ids_of(c, "secrets", "key");
    let accounts = ids_of(c, "accounts", "id");
    let of_account = |key: &str| {
        key.strip_prefix("account:").is_some_and(|rest| {
            !accounts.is_empty() && (rest == "*" || accounts.iter().any(|a| a == rest))
        })
    };
    let mut problems: Vec<String> = Vec::new();
    if !runs && !webhooks {
        for (field, list) in [
            ("network", &p.network),
            ("events", &p.events),
            ("secrets", &p.secrets),
        ] {
            if !list.is_empty() {
                problems.push(tr_in!(
                    lang,
                    "permissions.{field} : un plugin sans programme ni webhook n'en a pas besoin",
                    "permissions.{field}: a plugin with no program and no webhook does not need it"
                ));
            }
        }
    }
    if !runs && !p.commands.is_empty() {
        problems.push(tr_in!(
            lang,
            "permissions.commands : un plugin sans programme ne lance pas de commande",
            "permissions.commands: a plugin with no program runs no command"
        ));
    }
    for host in &p.network {
        if host != "{account}" && host != "local:shared" && !host_name(host) {
            problems.push(tr_in!(
                lang,
                "permissions.network : {host} n'est pas un nom d'hôte",
                "permissions.network: {host} is no host name"
            ));
        }
    }
    for key in &p.secrets {
        if !declared.contains(key) && !of_account(key) {
            problems.push(tr_in!(
                lang,
                "permissions.secrets : {key} n'est pas un secret de contributes.secrets, ni le compte d'un genre de contributes.accounts",
                "permissions.secrets: {key} is no secret of contributes.secrets, nor the account of a kind of contributes.accounts"
            ));
        }
    }
    if !problems.is_empty() {
        return Err(problems.join(" ; "));
    }
    let mut asks: Vec<String> = Vec::new();
    if runs {
        asks.push(tr_in!(
            lang,
            "tourne dans Escouade, isolé",
            "runs inside Escouade, sandboxed"
        ));
    }
    if !p.network.is_empty() {
        asks.push(tr_in!(
            lang,
            "se connecte à {}",
            "connects to {}",
            p.network.join(", ")
        ));
    }
    if !p.events.is_empty() {
        asks.push(tr_in!(lang, "reçoit des événements", "gets events"));
    }
    if !p.secrets.is_empty() {
        asks.push(tr_in!(lang, "garde un secret", "keeps a secret"));
    }
    if asks.is_empty() {
        Ok(tr_in!(lang, "ne demande rien", "asks for nothing"))
    } else {
        Ok(asks.join(" ; "))
    }
}

/// No native program: a plugin runs in WASM, or not at all (decision 72).
fn check_binaries(m: &Manifest, lang: Lang) -> Result<String, String> {
    let Some(run) = &m.run else {
        return Ok(tr_in!(
            lang,
            "aucun : le plugin ne lance pas de programme",
            "none: the plugin runs no program"
        ));
    };
    if run.in_wasm().is_some() && run.by_system.is_empty() {
        return Ok(tr_in!(
            lang,
            "aucun à fournir : le plugin est compilé en WASM",
            "none to bring: the plugin is compiled to WASM"
        ));
    }
    Err(tr_in!(
        lang,
        "run lancerait un programme natif : un plugin d'équipe tourne en WASM (run.js ou run.wasm)",
        "run would start a native program: a team's plugin runs in WASM (run.js or run.wasm)"
    ))
}

/// The compilation twice, each in a folder of its own: the same bytes, or a refusal.
fn check_compile(
    m: &Manifest,
    files: &[(String, Vec<u8>)],
    first: &Fresh,
    compiler: &dyn Compile,
    lang: Lang,
) -> Result<(String, Option<Vec<u8>>), String> {
    if m.run.as_ref().and_then(|r| r.in_wasm()).is_none() {
        return Ok((
            tr_in!(
                lang,
                "rien à compiler : ni run.js ni run.wasm",
                "nothing to compile: neither run.js nor run.wasm"
            ),
            None,
        ));
    }
    let one = compiler.compile(&first.0, m, lang)?;
    let second = Fresh::new(files).map_err(|e| e.to_string())?;
    let two = compiler.compile(&second.0, m, lang)?;
    let (a, b) = (hex(&Sha256::digest(&one)), hex(&Sha256::digest(&two)));
    if a != b {
        return Err(tr_in!(
            lang,
            "les deux compilations diffèrent ({} puis {}) : la compilation n'est pas reproductible",
            "the two compilations differ ({} then {}): the compilation is not reproducible",
            &a[..12],
            &b[..12]
        ));
    }
    if m.run.as_ref().is_some_and(|r| r.wasm.is_some()) {
        wasm::reactor_form(&one).map_err(|e| {
            tr_in!(
                lang,
                "le module n'a pas la forme d'un réacteur : {e}",
                "the module does not have the form of a reactor: {e}"
            )
        })?;
    }
    Ok((
        tr_in!(
            lang,
            "deux fois le même module, sha256 {}",
            "twice the same module, sha256 {}",
            &a[..12]
        ),
        Some(one),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn plugin(dir: &Path, manifest: Value) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join(manifest::FILE),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    fn declarative(id: &str, version: &str) -> Value {
        serde_json::json!({
            "id": id,
            "name": { "fr": "Règles", "en": "Rules" },
            "version": version,
            "escouade": { "api": "1", "min": "2.0.0" },
            "contributes": {}
        })
    }

    fn against<'a>(tag: &'a str, last: Option<&'a str>) -> Against<'a> {
        Against {
            tag,
            last,
            app_version: "2.3.0",
        }
    }

    struct Fixed(Vec<u8>);
    impl Compile for Fixed {
        fn compile(&self, _: &Path, _: &Manifest, _: Lang) -> Result<Vec<u8>, String> {
            Ok(self.0.clone())
        }
    }

    /// A compiler whose output changes at each run: a build that is not reproducible.
    struct Drifting(Cell<u8>);
    impl Compile for Drifting {
        fn compile(&self, _: &Path, _: &Manifest, _: Lang) -> Result<Vec<u8>, String> {
            self.0.set(self.0.get() + 1);
            let mut m = module();
            m.push(self.0.get());
            Ok(m)
        }
    }

    /// A module, as a compiler would make it (its bytes alone matter here).
    fn module() -> Vec<u8> {
        b"\0asm-module".to_vec()
    }

    #[test]
    fn the_archive_is_the_same_bytes_wherever_it_is_made() {
        let t = tempfile();
        plugin(&t.0.join("a"), declarative("fr.acme.rules", "1.0.0"));
        std::fs::write(t.0.join("a/README.md"), "# Règles").unwrap();
        std::fs::create_dir_all(t.0.join("a/.git")).unwrap();
        std::fs::write(t.0.join("a/.git/HEAD"), "ref").unwrap();
        let one = pack(&t.0.join("a"), Lang::Fr).unwrap();
        plugin(&t.0.join("b"), declarative("fr.acme.rules", "1.0.0"));
        std::fs::write(t.0.join("b/README.md"), "# Règles").unwrap();
        assert_eq!(one, pack(&t.0.join("b"), Lang::Fr).unwrap());
        let files = unpack(&one, Lang::Fr).unwrap();
        let names: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(names, vec!["README.md", manifest::FILE], "no .git");
    }

    #[test]
    fn a_declarative_plugin_passes_every_check() {
        let t = tempfile();
        plugin(&t.0, declarative("fr.acme.rules", "1.1.0"));
        let gz = pack(&t.0, Lang::Fr).unwrap();
        let r = check(
            &gz,
            &against("v1.1.0", Some("1.0.0")),
            &Fixed(vec![]),
            Lang::Fr,
        );
        assert!(r.ok, "{:?}", r.checks);
        assert_eq!(r.checks.len(), CHECKS.len());
        assert_eq!(r.module_sha256, None);
        assert_eq!(r.digest, digest(&r.archive_sha256, None));
        // The same archive, checked on another machine: the same digest.
        let again = check(
            &gz,
            &against("1.1.0", Some("1.0.0")),
            &Fixed(vec![]),
            Lang::En,
        );
        assert_eq!(again.digest, r.digest);
    }

    #[test]
    fn the_version_is_the_tag_and_above_the_last() {
        let t = tempfile();
        plugin(&t.0, declarative("fr.acme.rules", "1.0.0"));
        let gz = pack(&t.0, Lang::Fr).unwrap();
        let failed = |r: &Report| r.checks.iter().find(|c| !c.ok).map(|c| c.key.clone());
        let r = check(&gz, &against("v1.0.1", None), &Fixed(vec![]), Lang::Fr);
        assert_eq!(failed(&r).as_deref(), Some("version"));
        let r = check(
            &gz,
            &against("v1.0.0", Some("1.0.0")),
            &Fixed(vec![]),
            Lang::Fr,
        );
        assert_eq!(failed(&r).as_deref(), Some("version"));
        assert!(!r.ok);
        let r = check(
            &gz,
            &against("v1.0.0", Some("0.9.3")),
            &Fixed(vec![]),
            Lang::Fr,
        );
        assert!(r.ok);
    }

    #[test]
    fn a_first_party_id_or_a_native_program_is_refused() {
        let t = tempfile();
        plugin(&t.0, declarative("dev.escouade.rules", "1.0.0"));
        let gz = pack(&t.0, Lang::Fr).unwrap();
        let r = check(&gz, &against("v1.0.0", None), &Fixed(vec![]), Lang::Fr);
        assert!(!r.ok);
        assert_eq!(r.checks.last().unwrap().key, "manifest");
        let mut m = declarative("fr.acme.tool", "1.0.0");
        m["run"] = serde_json::json!({ "command": "tool.exe" });
        let t = tempfile();
        plugin(&t.0, m);
        let r = check(
            &pack(&t.0, Lang::Fr).unwrap(),
            &against("v1.0.0", None),
            &Fixed(vec![]),
            Lang::Fr,
        );
        let binaries = r.checks.iter().find(|c| c.key == "binaries").unwrap();
        assert!(!binaries.ok, "{:?}", r.checks);
    }

    #[test]
    fn a_permission_without_its_contribution_is_refused() {
        let mut m = declarative("fr.acme.rules", "1.0.0");
        m["permissions"] = serde_json::json!({ "network": ["hooks.slack.com"] });
        let t = tempfile();
        plugin(&t.0, m);
        let r = check(
            &pack(&t.0, Lang::Fr).unwrap(),
            &against("v1.0.0", None),
            &Fixed(vec![]),
            Lang::Fr,
        );
        let p = r.checks.iter().find(|c| c.key == "permissions").unwrap();
        assert!(!p.ok);
        assert!(p.detail.contains("permissions.network"), "{}", p.detail);
    }

    fn with_bundle() -> TempDir {
        let mut m = declarative("fr.acme.bot", "1.0.0");
        m["run"] = serde_json::json!({ "js": "dist/plugin.mjs" });
        let t = tempfile();
        plugin(&t.0, m);
        std::fs::create_dir_all(t.0.join("dist")).unwrap();
        std::fs::write(
            t.0.join("dist/plugin.mjs"),
            "const plugin = { activate() {} };\n",
        )
        .unwrap();
        t
    }

    #[test]
    fn the_compilation_runs_twice_and_must_give_the_same_module() {
        let t = with_bundle();
        let gz = pack(&t.0, Lang::Fr).unwrap();
        let r = check(&gz, &against("v1.0.0", None), &Fixed(module()), Lang::Fr);
        assert!(r.ok, "{:?}", r.checks);
        let sha = hex(&Sha256::digest(module()));
        assert_eq!(r.module_sha256.as_deref(), Some(sha.as_str()));
        assert_eq!(r.digest, digest(&r.archive_sha256, Some(&sha)));
        assert_eq!(r.module.as_deref(), Some(module().as_slice()));

        let drifting = Drifting(Cell::new(0));
        let r = check(&gz, &against("v1.0.0", None), &drifting, Lang::Fr);
        assert_eq!(drifting.0.get(), 2, "compiled twice");
        let c = r.checks.iter().find(|c| c.key == "compile").unwrap();
        assert!(!c.ok);
        assert!(c.detail.contains("diffèrent"), "{}", c.detail);
        assert!(!r.ok);
        assert_eq!(r.module, None);
    }

    /// A tar.gz of raw entries: (path, kind, bytes), kind `0` a file, `2` a link, `5` a folder.
    fn tarball(entries: &[(&str, u8, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let gz = flate2::write::GzEncoder::new(&mut out, flate2::Compression::default());
            let mut b = tar::Builder::new(gz);
            for (path, kind, bytes) in entries {
                let mut h = tar::Header::new_gnu();
                {
                    let raw = h.as_gnu_mut().unwrap();
                    raw.name[..path.len()].copy_from_slice(path.as_bytes());
                }
                h.set_entry_type(tar::EntryType::new(b'0' + kind));
                h.set_size(bytes.len() as u64);
                h.set_mode(0o644);
                if *kind == 2 {
                    h.set_link_name("/etc/passwd").unwrap();
                }
                h.set_cksum();
                b.append(&h, *bytes).unwrap();
            }
            b.into_inner().unwrap().finish().unwrap();
        }
        out
    }

    #[test]
    fn an_unsound_archive_is_refused() {
        type Entries<'a> = &'a [(&'a str, u8, &'a [u8])];
        let cases: &[(Entries, &str)] = &[
            (&[("p/link", 2, b"")], "lien symbolique"),
            (&[("p/../../evil", 0, b"x")], ".."),
            (&[("/etc/x", 0, b"x")], "absolu"),
            (&[("p/.gitmodules", 0, b"[submodule]")], "sous-modules"),
            (&[("p/big.bin", 0, LFS_POINTER)], "LFS"),
        ];
        for (entries, said) in cases {
            let gz = tarball(entries);
            let problems = unpack(&gz, Lang::Fr).unwrap_err();
            assert!(
                problems.iter().any(|p| p.contains(said)),
                "{said}: {problems:?}"
            );
            let r = check(&gz, &against("v1.0.0", None), &Fixed(vec![]), Lang::Fr);
            assert!(!r.ok);
            assert_eq!(r.checks[0].key, "archive");
        }
        let big = vec![0u8; (ARCHIVE_MAX + 1) as usize];
        assert!(unpack(&big, Lang::Fr).unwrap_err()[0].contains("50 Mo"));
    }

    #[cfg(unix)]
    #[test]
    fn a_link_in_the_folder_is_refused_when_it_is_packed() {
        let t = tempfile();
        plugin(&t.0, declarative("fr.acme.rules", "1.0.0"));
        std::os::unix::fs::symlink("/etc/passwd", t.0.join("passwd")).unwrap();
        assert!(pack(&t.0, Lang::Fr).unwrap_err().contains("lien"));
    }

    struct TempDir(PathBuf);
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn tempfile() -> TempDir {
        let d = std::env::temp_dir().join(format!("team-pack-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&d).unwrap();
        TempDir(d)
    }
}
