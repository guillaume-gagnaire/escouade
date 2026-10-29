//! The installed plugins (docs/PLUGINS.md §3.2, ESC-173): a plugin is installed from a local
//! folder or from a git repository at a tag, copied into the plugins' folder of the app's data
//! (`DataDir::plugins_dir`) and pinned there by its fingerprint (the commit, or the SHA-256 of the
//! folder's files); a folder in development is loaded in place, and read again when it changes.
//!
//! What is installed is listed in `DataDir::plugins_file`. Loading a plugin reads its manifest
//! (`manifest`) and nothing else: no plugin runs here (its process is `runtime`'s, PLG-2). A manifest
//! that does not hold loads nothing, and says why, field by field; a plugin of an API or for an
//! Escouade out of its range is loaded disabled, with the reason; a copy whose files changed
//! since it was installed is too.

// The journal and the runtime serve Réglages › Plugins (PLG-4) and the extension points (PLG-5 and
// after); until then, the app only starts and closes them.
#[cfg_attr(not(test), allow(dead_code))]
pub mod journal;
// The conformance bench and the fake Escouade, for `escouade-plugin` and the tests (PLG-3).
pub mod bench;
pub mod manifest;
#[cfg_attr(not(test), allow(dead_code))]
pub mod runtime;

use crate::i18n::Lang;
use crate::paths::DataDir;
use manifest::{Manifest, Problem};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Weak};
use std::time::Duration;

/// The version of the list of installed plugins.
const INDEX_VERSION: u32 = 1;
/// How long a folder in development stays quiet before it is read again.
const SETTLE: Duration = Duration::from_millis(200);

/// Where an installed plugin comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Origin {
    /// Copied from a local folder.
    Folder { path: String },
    /// Cloned from a git repository at a tag.
    Git { url: String, tag: String },
    /// A folder in development, loaded in place.
    Dev { path: String },
}

/// A plugin of the list (`plugins.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub id: String,
    pub origin: Origin,
    /// What pins it: `git:<commit>`, `sha256:<files>`; empty for a folder in development.
    pub fingerprint: String,
    /// The SHA-256 of its copy's files (`digest`), compared when it is loaded; empty in development.
    #[serde(default)]
    pub digest: String,
    pub installed_at: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    version: u32,
    plugins: Vec<Installed>,
}

/// Why a plugin was not installed: the fields of its manifest that are wrong, or another reason.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refusal {
    /// The whole reason, in the interface's language: a line per field that is wrong.
    pub message: String,
    pub problems: Vec<ProblemView>,
}

/// A field of a manifest that is wrong, with what is wrong in words.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemView {
    #[serde(flatten)]
    pub problem: Problem,
    pub message: String,
}

impl Refusal {
    fn other(message: String) -> Self {
        Self {
            message,
            problems: Vec::new(),
        }
    }

    fn invalid(lang: Lang, problems: &[Problem]) -> Self {
        let views: Vec<ProblemView> = problems
            .iter()
            .map(|p| ProblemView {
                problem: p.clone(),
                message: p.message(lang),
            })
            .collect();
        let mut message = tr_in!(
            lang,
            "Le manifeste ({}) n'est pas valide ; rien n'a été chargé :",
            "The manifest ({}) does not hold; nothing was loaded:",
            manifest::FILE
        );
        for v in &views {
            message.push_str("\n- ");
            message.push_str(&v.message);
        }
        Self {
            message,
            problems: views,
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Where a plugin stands once loaded.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum State {
    /// Loaded.
    Ready,
    /// Loaded, but disabled: an API or an Escouade out of its range, its copy changed.
    Disabled { reason: String },
    /// Nothing loaded: its manifest does not hold, or is gone.
    Invalid {
        reason: String,
        problems: Vec<ProblemView>,
    },
}

/// A plugin as the interface shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginView {
    pub id: String,
    /// Its name in the interface's language (its id when its manifest does not hold).
    pub name: String,
    pub version: String,
    pub description: String,
    pub origin: Origin,
    pub fingerprint: String,
    /// The folder it is loaded from.
    pub folder: String,
    pub declarative: bool,
    pub state: State,
    /// The fields of its manifest Escouade does not know, ignored.
    pub ignored: Vec<String>,
    pub manifest: Option<Manifest>,
}

/// A plugin as last read from its folder.
#[derive(Debug, Clone)]
struct Loaded {
    installed: Installed,
    folder: PathBuf,
    read: Result<manifest::Read, Fault>,
    /// The SHA-256 of the copy's files as found, when it is not the one installed.
    changed: bool,
}

/// What kept a plugin's manifest from being read.
#[derive(Debug, Clone)]
enum Fault {
    /// Its folder or its manifest is gone.
    Missing,
}

/// The installed plugins of the app.
pub struct Plugins {
    me: Weak<Self>,
    data: DataDir,
    /// This Escouade's version, which a plugin's range is compared to.
    app_version: String,
    loaded: RwLock<Vec<Loaded>>,
    /// One install, removal or reload at a time.
    writing: Mutex<()>,
    watchers: Mutex<HashMap<String, RecommendedWatcher>>,
    /// Told the id of a folder in development that changed.
    changes: Mutex<Option<mpsc::Sender<String>>>,
    /// Counts the reloads of the folders in development (for the tests).
    revision: AtomicU64,
}

impl Plugins {
    /// The plugins of `data`, read once, the folders in development watched.
    pub fn load(data: DataDir, app_version: &str) -> Arc<Self> {
        let me = Arc::new_cyclic(|me| Self {
            me: me.clone(),
            data,
            app_version: app_version.to_string(),
            loaded: RwLock::new(Vec::new()),
            writing: Mutex::new(()),
            watchers: Mutex::new(HashMap::new()),
            changes: Mutex::new(None),
            revision: AtomicU64::new(0),
        });
        let index = me.index();
        let loaded: Vec<Loaded> = index.plugins.into_iter().map(|i| me.read(i)).collect();
        for l in &loaded {
            if let Origin::Dev { path } = &l.installed.origin {
                me.watch(&l.installed.id, Path::new(path));
            }
        }
        *me.loaded.write() = loaded;
        me
    }

    fn index(&self) -> Index {
        std::fs::read(self.data.plugins_file())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save(&self) -> std::io::Result<()> {
        let index = Index {
            version: INDEX_VERSION,
            plugins: self
                .loaded
                .read()
                .iter()
                .map(|l| l.installed.clone())
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&index).map_err(std::io::Error::other)?;
        std::fs::create_dir_all(self.data.root())?;
        crate::paths::write_atomic(&self.data.plugins_file(), &bytes)
    }

    /// The folder a plugin is loaded from.
    fn folder_of(&self, i: &Installed) -> PathBuf {
        match &i.origin {
            Origin::Dev { path } => PathBuf::from(path),
            _ => self.data.plugins_dir().join(&i.id),
        }
    }

    fn read(&self, installed: Installed) -> Loaded {
        let folder = self.folder_of(&installed);
        let read = std::fs::read_to_string(folder.join(manifest::FILE))
            .map(|t| manifest::read_in(&t, false, &folder))
            .map_err(|_| Fault::Missing);
        let changed = read.is_ok()
            && !installed.digest.is_empty()
            && digest(&folder).ok().as_deref() != Some(installed.digest.as_str());
        Loaded {
            installed,
            folder,
            read,
            changed,
        }
    }

    /// Every installed plugin, as last read, in `lang`.
    pub fn list(&self, lang: Lang) -> Vec<PluginView> {
        self.loaded
            .read()
            .iter()
            .map(|l| self.view(l, lang))
            .collect()
    }

    /// One installed plugin, as last read, in `lang`.
    pub fn get(&self, id: &str, lang: Lang) -> Option<PluginView> {
        self.loaded
            .read()
            .iter()
            .find(|l| l.installed.id == id)
            .map(|l| self.view(l, lang))
    }

    /// The icon of the installed plugin `id`, as a `data:image/png` address the window shows:
    /// taken by its id alone, read in its folder and never out of it (`manifest::icon_file`), its
    /// header checked again (a folder in development may have changed it). `Ok(None)` for a
    /// plugin without one, or whose manifest does not hold (the window draws its letters); an
    /// error for an id that is not installed.
    pub fn icon(&self, id: &str) -> Result<Option<String>, String> {
        use base64::Engine as _;
        let (folder, rel) = {
            let loaded = self.loaded.read();
            let l = loaded
                .iter()
                .find(|l| l.installed.id == id)
                .ok_or_else(|| format!("no plugin {id} is installed"))?;
            let rel = match &l.read {
                Ok(manifest::Read {
                    manifest: Ok(m), ..
                }) => m.icon.clone(),
                _ => None,
            };
            (l.folder.clone(), rel)
        };
        let Some(bytes) = rel.and_then(|r| manifest::icon_file(&folder, &r)) else {
            return Ok(None);
        };
        if !manifest::icon_problems(&bytes).is_empty() {
            return Ok(None);
        }
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(Some(format!("data:image/png;base64,{b64}")))
    }

    fn view(&self, l: &Loaded, lang: Lang) -> PluginView {
        let i = &l.installed;
        let (manifest, ignored, state) = match &l.read {
            Err(Fault::Missing) => (
                None,
                Vec::new(),
                State::Invalid {
                    reason: tr_in!(
                        lang,
                        "Son manifeste ({}) est introuvable dans {}.",
                        "Its manifest ({}) is not found in {}.",
                        manifest::FILE,
                        l.folder.display()
                    ),
                    problems: Vec::new(),
                },
            ),
            Ok(read) => match &read.manifest {
                Err(problems) => {
                    let r = Refusal::invalid(lang, problems);
                    (
                        None,
                        read.ignored.clone(),
                        State::Invalid {
                            reason: r.message,
                            problems: r.problems,
                        },
                    )
                }
                Ok(m) => {
                    let state = if m.id != i.id {
                        State::Disabled {
                            reason: tr_in!(
                                lang,
                                "Son id a changé ({} devenu {}) : charge le dossier à nouveau.",
                                "Its id changed ({} became {}): load the folder again.",
                                i.id,
                                m.id
                            ),
                        }
                    } else if l.changed {
                        State::Disabled {
                            reason: tr_in!(
                                lang,
                                "Ses fichiers ont changé depuis son installation (empreinte {}) : installe-le à nouveau.",
                                "Its files changed since it was installed (fingerprint {}): install it again.",
                                i.fingerprint
                            ),
                        }
                    } else {
                        match m.refusal(&self.app_version, lang) {
                            Some(reason) => State::Disabled { reason },
                            None => State::Ready,
                        }
                    };
                    (Some(m.clone()), read.ignored.clone(), state)
                }
            },
        };
        PluginView {
            id: i.id.clone(),
            name: manifest
                .as_ref()
                .map(|m| m.name.get(lang).to_string())
                .unwrap_or_else(|| i.id.clone()),
            version: manifest
                .as_ref()
                .map(|m| m.version.clone())
                .unwrap_or_default(),
            description: manifest
                .as_ref()
                .and_then(|m| m.description.as_ref())
                .map(|d| d.get(lang).to_string())
                .unwrap_or_default(),
            origin: i.origin.clone(),
            fingerprint: i.fingerprint.clone(),
            folder: l.folder.display().to_string(),
            declarative: manifest.as_ref().is_some_and(Manifest::declarative),
            state,
            ignored,
            manifest,
        }
    }

    /// The manifest of the folder `dir`, when it holds.
    fn checked(dir: &Path, lang: Lang) -> Result<Manifest, Refusal> {
        let text = std::fs::read_to_string(dir.join(manifest::FILE)).map_err(|_| {
            Refusal::other(tr_in!(
                lang,
                "Aucun {} dans {} : ce n'est pas le dossier d'un plugin.",
                "No {} in {}: this is not a plugin's folder.",
                manifest::FILE,
                dir.display()
            ))
        })?;
        manifest::read_in(&text, false, dir)
            .manifest
            .map_err(|p| Refusal::invalid(lang, &p))
    }

    /// Installs the plugin of the local folder `src`: copied into the plugins' folder, pinned by
    /// the SHA-256 of its files. In place of an installed plugin of the same id.
    pub fn install_folder(&self, src: &Path, lang: Lang) -> Result<PluginView, Refusal> {
        let m = Self::checked(src, lang)?;
        let _w = self.writing.lock();
        let digest = self.copy_in(src, &m.id, lang)?;
        let path = std::path::absolute(src)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| src.display().to_string());
        self.put(
            Installed {
                id: m.id.clone(),
                origin: Origin::Folder { path },
                fingerprint: format!("sha256:{digest}"),
                digest,
                installed_at: now(),
            },
            lang,
        )
    }

    /// Installs the plugin of the git repository `url` at the tag `tag`: cloned at that tag alone,
    /// copied without its history, pinned by the tag's commit.
    pub async fn install_git(
        &self,
        url: &str,
        tag: &str,
        lang: Lang,
    ) -> Result<PluginView, Refusal> {
        let (url, tag) = (url.trim(), tag.trim());
        if url.is_empty() || url.starts_with('-') {
            return Err(Refusal::other(tr_in!(
                lang,
                "Donne l'adresse du dépôt git du plugin.",
                "Give the address of the plugin's git repository."
            )));
        }
        if tag.is_empty() || tag.starts_with('-') {
            return Err(Refusal::other(tr_in!(
                lang,
                "Donne l'étiquette de la version à installer (v1.2.0) : un plugin est fixé à une version.",
                "Give the tag of the version to install (v1.2.0): a plugin is pinned to a version."
            )));
        }
        let root = self.data.plugins_dir();
        std::fs::create_dir_all(&root).map_err(|e| Refusal::other(e.to_string()))?;
        let clone = root.join(format!(".clone-{}", uuid::Uuid::new_v4().simple()));
        let clone_s = clone.display().to_string();
        let root_s = root.display().to_string();
        let cloned = crate::git::run(
            &root_s,
            &[
                "-c",
                "advice.detachedHead=false",
                "clone",
                "--quiet",
                "--depth",
                "1",
                "--branch",
                tag,
                "--",
                url,
                &clone_s,
            ],
        )
        .await;
        let result = async {
            cloned.map_err(|e| {
                Refusal::other(tr_in!(
                    lang,
                    "Le dépôt {url} n'a pas pu être cloné à l'étiquette {tag} : {e}",
                    "The repository {url} could not be cloned at the tag {tag}: {e}"
                ))
            })?;
            let commit = crate::git::text(&clone_s, &["rev-parse", "HEAD"])
                .await
                .map_err(|e| Refusal::other(e.to_string()))?;
            let m = Self::checked(&clone, lang)?;
            let _w = self.writing.lock();
            let digest = self.copy_in(&clone, &m.id, lang)?;
            self.put(
                Installed {
                    id: m.id.clone(),
                    origin: Origin::Git {
                        url: url.to_string(),
                        tag: tag.to_string(),
                    },
                    fingerprint: format!("git:{}", commit.trim()),
                    digest,
                    installed_at: now(),
                },
                lang,
            )
        }
        .await;
        remove_dir(&clone);
        result
    }

    /// Loads the folder in development `dir` in place: read again each time it changes.
    pub fn load_dev(&self, dir: &Path, lang: Lang) -> Result<PluginView, Refusal> {
        let m = Self::checked(dir, lang)?;
        let path = std::path::absolute(dir)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| dir.display().to_string());
        let _w = self.writing.lock();
        // A copy of the same id goes: the folder takes its place (unless it is that copy).
        let copy = self.data.plugins_dir().join(&m.id);
        if std::fs::canonicalize(&copy).ok() != std::fs::canonicalize(dir).ok() {
            remove_dir(&copy);
        }
        let view = self.put(
            Installed {
                id: m.id.clone(),
                origin: Origin::Dev { path: path.clone() },
                fingerprint: String::new(),
                digest: String::new(),
                installed_at: now(),
            },
            lang,
        )?;
        self.watch(&m.id, Path::new(&path));
        Ok(view)
    }

    /// Uninstalls a plugin: its copy removed; a folder in development is left where it is.
    pub fn remove(&self, id: &str) -> std::io::Result<bool> {
        let _w = self.writing.lock();
        let gone = {
            let mut loaded = self.loaded.write();
            let at = loaded.iter().position(|l| l.installed.id == id);
            at.map(|at| loaded.remove(at))
        };
        let Some(gone) = gone else {
            return Ok(false);
        };
        self.watchers.lock().remove(id);
        if !matches!(gone.installed.origin, Origin::Dev { .. }) {
            remove_dir(&gone.folder);
        }
        self.save()?;
        Ok(true)
    }

    /// Puts `installed` in the list in place of its id's, read from its folder, the list saved.
    fn put(&self, installed: Installed, lang: Lang) -> Result<PluginView, Refusal> {
        let id = installed.id.clone();
        self.watchers.lock().remove(&id);
        let l = self.read(installed);
        {
            let mut loaded = self.loaded.write();
            loaded.retain(|o| o.installed.id != id);
            loaded.push(l);
        }
        self.save().map_err(|e| Refusal::other(e.to_string()))?;
        Ok(self.get(&id, lang).expect("just put"))
    }

    /// Copies the folder `src` to the plugin `id`'s, in place of its old copy: its history
    /// (`.git`) and its links left out. Gives the SHA-256 of the copy's files.
    fn copy_in(&self, src: &Path, id: &str, lang: Lang) -> Result<String, Refusal> {
        let root = self.data.plugins_dir();
        let fail = |e: std::io::Error| {
            Refusal::other(tr_in!(
                lang,
                "Le plugin n'a pas pu être copié dans {} : {e}",
                "The plugin could not be copied to {}: {e}",
                root.display()
            ))
        };
        std::fs::create_dir_all(&root).map_err(fail)?;
        let incoming = root.join(format!(".incoming-{}", uuid::Uuid::new_v4().simple()));
        let made = copy_tree(src, &incoming).and_then(|_| digest(&incoming));
        let digest = match made {
            Ok(d) => d,
            Err(e) => {
                remove_dir(&incoming);
                return Err(fail(e));
            }
        };
        let to = root.join(id);
        remove_dir(&to);
        if let Err(e) = std::fs::rename(&incoming, &to) {
            remove_dir(&incoming);
            return Err(fail(e));
        }
        Ok(digest)
    }

    /// Watches the folder in development of the plugin `id`.
    fn watch(&self, id: &str, dir: &Path) {
        let tx = self.changes_tx();
        let pid = id.to_string();
        let handler = move |res: notify::Result<notify::Event>| {
            // Reading the folder again opens its files: under Linux, that would read it again (ESC-290).
            if res.is_ok_and(|ev| !crate::git::is_read(&ev)) {
                let _ = tx.send(pid.clone());
            }
        };
        match notify::recommended_watcher(handler) {
            Ok(mut w) => match w.watch(dir, RecursiveMode::Recursive) {
                Ok(()) => {
                    self.watchers.lock().insert(id.to_string(), w);
                }
                Err(e) => log::warn!("cannot watch the plugin folder {}: {e}", dir.display()),
            },
            Err(e) => log::warn!("plugin watcher error: {e}"),
        }
    }

    /// Where the watchers tell what changed: a thread that reads again, once quiet, the folders in
    /// development that changed.
    fn changes_tx(&self) -> mpsc::Sender<String> {
        let mut changes = self.changes.lock();
        if let Some(tx) = changes.as_ref() {
            return tx.clone();
        }
        let (tx, rx) = mpsc::channel::<String>();
        let me = self.me.clone();
        std::thread::Builder::new()
            .name("plugins-dev".into())
            .spawn(move || {
                while let Ok(first) = rx.recv() {
                    let mut ids = vec![first];
                    while let Ok(id) = rx.recv_timeout(SETTLE) {
                        if !ids.contains(&id) {
                            ids.push(id);
                        }
                    }
                    let Some(me) = me.upgrade() else { return };
                    me.reload(&ids);
                }
            })
            .ok();
        *changes = Some(tx.clone());
        tx
    }

    /// Reads again the folders in development of `ids`.
    fn reload(&self, ids: &[String]) {
        let _w = self.writing.lock();
        let mut loaded = self.loaded.write();
        for l in loaded.iter_mut() {
            if ids.contains(&l.installed.id) && matches!(l.installed.origin, Origin::Dev { .. }) {
                *l = self.read(l.installed.clone());
            }
        }
        drop(loaded);
        self.revision.fetch_add(1, Ordering::AcqRel);
    }

    /// How many times the folders in development were read again.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Removes a folder, read-only files included; what cannot be removed stays.
fn remove_dir(dir: &Path) {
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(dir) {
            log::warn!("cannot remove {}: {e}", dir.display());
        }
    }
}

/// Whether a file of a plugin's folder is left out of its copy and its fingerprint: its git
/// history.
fn left_out(name: &std::ffi::OsStr) -> bool {
    name == ".git"
}

/// Copies the folder `src` to `to` (made), its history and its links left out.
fn copy_tree(src: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        if left_out(&entry.file_name()) {
            continue;
        }
        let kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// The files of a folder, by their path in it (`/` between folders), sorted; its history and its
/// links left out.
fn files_of(dir: &Path, at: &str, out: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if left_out(&name) {
            continue;
        }
        let rel = format!("{at}{}", name.to_string_lossy());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            files_of(&entry.path(), &format!("{rel}/"), out)?;
        } else if kind.is_file() {
            out.push((rel, entry.path()));
        }
    }
    Ok(())
}

/// The SHA-256 of a folder's files: their paths and their contents, in the order of their paths.
pub fn digest(dir: &Path) -> std::io::Result<String> {
    let mut files = Vec::new();
    files_of(dir, "", &mut files)?;
    files.sort();
    let mut h = Sha256::new();
    for (rel, path) in files {
        let bytes = std::fs::read(path)?;
        h.update(rel.as_bytes());
        h.update([0]);
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(&bytes);
    }
    Ok(format!("{:x}", h.finalize()))
}
