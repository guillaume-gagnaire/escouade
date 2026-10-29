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
// The seal of a version from the store, checked with the keys the app embeds (STORE-8; STORE-9 shows it).
pub mod store_keys;

use crate::i18n::Lang;
use crate::paths::DataDir;
use manifest::{Manifest, Problem};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
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
    #[serde(flatten)]
    pub choices: Choices,
}

/// What the user chose for an installed plugin (PLG-4): kept across its updates, and across an
/// uninstall that keeps its data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choices {
    /// The app's switch: off, nothing of it runs or reaches a project.
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Its switch in a project, by the project's id, where the user set it; elsewhere, on unless
    /// the project is private (`active_in`).
    #[serde(default)]
    pub projects: BTreeMap<String, bool>,
    /// The permissions taken back (`asks`' words): it goes on without them.
    #[serde(default)]
    pub revoked: Vec<String>,
    /// The values of its settings (`contributes.settings`), by key.
    #[serde(default)]
    pub settings: Map<String, Value>,
}

impl Default for Choices {
    fn default() -> Self {
        Self {
            enabled: true,
            projects: BTreeMap::new(),
            revoked: Vec::new(),
            settings: Map::new(),
        }
    }
}

fn yes() -> bool {
    true
}

/// Whether a plugin of `choices` reaches the project `project`: where the user set it, as set;
/// elsewhere, unless the project is private (§3.3); never with the app's switch off.
// The events reach a project by it (PLG-5); Réglages › Plugins applies the same rule in the window.
#[cfg_attr(not(test), allow(dead_code))]
pub fn active_in(choices: &Choices, project: &str, private: bool) -> bool {
    choices.enabled && choices.projects.get(project).copied().unwrap_or(!private)
}

/// What a plugin may do, a word each, the strongest first (§3.4): `run` (it runs a program),
/// `decide` (it decides the agents' permissions), then `commands:<c>`, `events:<e>`,
/// `network:<host>` and `secrets:<key>`.
pub fn asks(m: &Manifest) -> Vec<String> {
    let p = &m.permissions;
    let mut out = Vec::new();
    if m.run.is_some() {
        out.push("run".to_string());
    }
    if p.decide {
        out.push("decide".to_string());
    }
    for (kind, words) in [
        ("commands", &p.commands),
        ("events", &p.events),
        ("network", &p.network),
        ("secrets", &p.secrets),
    ] {
        out.extend(words.iter().map(|w| format!("{kind}:{w}")));
    }
    out
}

/// Whether the permission `word` can be taken back: all but the program itself (switched off
/// with the plugin).
pub fn revocable(word: &str) -> bool {
    word != "run"
}

/// `m` without the permissions of `revoked`: what its process is given.
pub fn granted(m: &Manifest, revoked: &[String]) -> Manifest {
    let mut m = m.clone();
    if revoked.is_empty() {
        return m;
    }
    let keep = |kind: &str, words: &mut Vec<String>| {
        words.retain(|w| !revoked.contains(&format!("{kind}:{w}")));
    };
    let p = &mut m.permissions;
    keep("commands", &mut p.commands);
    keep("events", &mut p.events);
    keep("network", &mut p.network);
    keep("secrets", &mut p.secrets);
    if revoked.iter().any(|w| w == "decide") {
        p.decide = false;
    }
    m
}

/// The keys of a plugin's secrets: those it declares fields for (`contributes.secrets`) and those
/// it may ask for by name (`permissions.secrets`, a pattern left out).
pub fn secret_keys(m: &Manifest) -> Vec<String> {
    let mut keys: Vec<String> = m
        .contributes
        .get("secrets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|s| s.get("key").and_then(Value::as_str).map(str::to_string))
        .collect();
    for k in &m.permissions.secrets {
        if !k.ends_with('*') && !keys.contains(k) {
            keys.push(k.clone());
        }
    }
    keys
}

/// A newer version found for an installed plugin: offered, never installed by itself (§3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub version: String,
    /// The tag it is at, for a plugin of a git repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
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
    pub installed_at: String,
    /// What the user chose: its switches, the permissions taken back, its settings.
    pub choices: Choices,
    /// What it may do (`asks`), the strongest first.
    pub asks: Vec<String>,
    /// The keys of its secrets (`secret_keys`).
    pub secret_keys: Vec<String>,
    /// A newer version found, offered.
    pub update: Option<Update>,
}

/// A plugin as last read from its folder.
#[derive(Debug, Clone)]
struct Loaded {
    installed: Installed,
    folder: PathBuf,
    read: Result<manifest::Read, Fault>,
    /// The SHA-256 of the copy's files as found, when it is not the one installed.
    changed: bool,
    /// A folder in development's files when it was read: an event of its watcher that changed
    /// nothing (one delivered late, a file written again as it was) reads nothing again (`reload`).
    seen: Option<Seen>,
}

/// A folder in development as last read: its stamp, cheap to take again at each event, and the
/// SHA-256 of its files, taken again only when the stamp moved.
#[derive(Debug, Clone, PartialEq)]
struct Seen {
    stamp: String,
    digest: String,
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
    /// The newer versions found (`check_updates`), by id.
    updates: RwLock<HashMap<String, Update>>,
    /// What an inspection set aside, by its token, until it is installed or discarded.
    staged: Mutex<HashMap<String, Staged>>,
    /// Told the id of each folder in development read again (the runtime forgets its host).
    reloaded: Mutex<Vec<ReloadHook>>,
}

type ReloadHook = Box<dyn Fn(&str) + Send + Sync>;

/// A plugin read before it is installed (the install window, §3.4): who made it and where it
/// comes from, what it adds and what it may do, and what grew since the version installed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    /// What `install` and `discard` take.
    pub token: String,
    /// Its icon, read in what was set aside (`Plugins::icon` once it is installed).
    pub icon: Option<String>,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub origin: Origin,
    pub fingerprint: String,
    pub declarative: bool,
    /// Why this Escouade would load it disabled (an API or a version out of its range).
    pub refusal: Option<String>,
    pub asks: Vec<String>,
    /// The version of the same id installed now: this is an update.
    pub previous: Option<Previous>,
    /// What it may do that the version installed may not: the user's consent is asked again.
    pub grown: Vec<String>,
    pub manifest: Manifest,
}

/// The installed version an inspection would replace.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Previous {
    pub version: String,
    pub fingerprint: String,
    pub asks: Vec<String>,
}

/// What an inspection set aside: its copy (none for a folder in development).
struct Staged {
    installed: Installed,
    dir: Option<PathBuf>,
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
            updates: RwLock::new(HashMap::new()),
            staged: Mutex::new(HashMap::new()),
            reloaded: Mutex::new(Vec::new()),
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
        let seen = matches!(installed.origin, Origin::Dev { .. })
            .then(|| {
                Some(Seen {
                    stamp: stamp(&folder).ok()?,
                    digest: digest(&folder).ok()?,
                })
            })
            .flatten();
        Loaded {
            installed,
            folder,
            read,
            changed,
            seen,
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
        Ok(rel.and_then(|r| icon_data(&folder, &r)))
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
            installed_at: i.installed_at.clone(),
            choices: i.choices.clone(),
            asks: manifest.as_ref().map(asks).unwrap_or_default(),
            secret_keys: manifest.as_ref().map(secret_keys).unwrap_or_default(),
            update: self.updates.read().get(&i.id).cloned(),
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

    /// Installs the plugin of the local folder `src` at once (`inspect_folder`, then `install`).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn install_folder(&self, src: &Path, lang: Lang) -> Result<PluginView, Refusal> {
        let p = self.inspect_folder(src, lang)?;
        self.install(&p.token, lang)
    }

    /// Installs the plugin of the git repository `url` at the tag `tag` at once.
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn install_git(
        &self,
        url: &str,
        tag: &str,
        lang: Lang,
    ) -> Result<PluginView, Refusal> {
        let p = self.inspect_git(url, tag, lang).await?;
        self.install(&p.token, lang)
    }

    /// Loads the folder in development `dir` in place at once.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn load_dev(&self, dir: &Path, lang: Lang) -> Result<PluginView, Refusal> {
        let p = self.inspect_dev(dir, lang)?;
        self.install(&p.token, lang)
    }

    /// What installing the plugin of the local folder `src` would do, before anything is
    /// installed: its copy is set aside, pinned by the SHA-256 of its files, until `install` or
    /// `discard`.
    pub fn inspect_folder(&self, src: &Path, lang: Lang) -> Result<Preview, Refusal> {
        let m = Self::checked(src, lang)?;
        let (dir, digest) = self.set_aside(src, lang)?;
        let path = std::path::absolute(src)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| src.display().to_string());
        Ok(self.stage(
            Installed {
                id: m.id.clone(),
                origin: Origin::Folder { path },
                fingerprint: format!("sha256:{digest}"),
                digest,
                installed_at: String::new(),
                choices: Choices::default(),
            },
            Some(dir),
            m,
            lang,
        ))
    }

    /// What installing the plugin of the git repository `url` at the tag `tag` would do: cloned at
    /// that tag alone, set aside without its history, pinned by the tag's commit.
    pub async fn inspect_git(&self, url: &str, tag: &str, lang: Lang) -> Result<Preview, Refusal> {
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
            let (dir, digest) = self.set_aside(&clone, lang)?;
            Ok(self.stage(
                Installed {
                    id: m.id.clone(),
                    origin: Origin::Git {
                        url: url.to_string(),
                        tag: tag.to_string(),
                    },
                    fingerprint: format!("git:{}", commit.trim()),
                    digest,
                    installed_at: String::new(),
                    choices: Choices::default(),
                },
                Some(dir),
                m,
                lang,
            ))
        }
        .await;
        remove_dir(&clone);
        result
    }

    /// What loading the folder in development `dir` in place would do: nothing is copied.
    pub fn inspect_dev(&self, dir: &Path, lang: Lang) -> Result<Preview, Refusal> {
        let m = Self::checked(dir, lang)?;
        let path = std::path::absolute(dir)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| dir.display().to_string());
        Ok(self.stage(
            Installed {
                id: m.id.clone(),
                origin: Origin::Dev { path },
                fingerprint: String::new(),
                digest: String::new(),
                installed_at: String::new(),
                choices: Choices::default(),
            },
            None,
            m,
            lang,
        ))
    }

    /// Keeps what an inspection read until `install` or `discard`, and says it.
    fn stage(
        &self,
        installed: Installed,
        dir: Option<PathBuf>,
        m: Manifest,
        lang: Lang,
    ) -> Preview {
        let token = uuid::Uuid::new_v4().simple().to_string();
        let wanted = asks(&m);
        let previous = self.get(&m.id, lang).map(|v| Previous {
            version: v.version.clone(),
            fingerprint: v.fingerprint.clone(),
            asks: v.asks.clone(),
        });
        let grown = previous
            .as_ref()
            .map(|p| {
                wanted
                    .iter()
                    .filter(|w| !p.asks.contains(w))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let read_from = dir.clone().unwrap_or_else(|| self.folder_of(&installed));
        let preview = Preview {
            icon: m.icon.as_deref().and_then(|r| icon_data(&read_from, r)),
            token: token.clone(),
            id: m.id.clone(),
            name: m.name.get(lang).to_string(),
            version: m.version.clone(),
            description: m
                .description
                .as_ref()
                .map(|d| d.get(lang).to_string())
                .unwrap_or_default(),
            origin: installed.origin.clone(),
            fingerprint: installed.fingerprint.clone(),
            declarative: m.declarative(),
            refusal: m.refusal(&self.app_version, lang),
            asks: wanted,
            previous,
            grown,
            manifest: m,
        };
        let mut staged = self.staged.lock();
        // What was set aside and never answered goes: one inspection at a time is enough.
        for (_, old) in staged.drain() {
            if let Some(d) = old.dir {
                remove_dir(&d);
            }
        }
        staged.insert(token, Staged { installed, dir });
        preview
    }

    /// Forgets what the inspection `token` set aside.
    pub fn discard(&self, token: &str) {
        if let Some(s) = self.staged.lock().remove(token) {
            if let Some(d) = s.dir {
                remove_dir(&d);
            }
        }
    }

    /// Installs what the inspection `token` set aside, in place of an installed plugin of the same
    /// id (an update): the user's choices of that one kept (a permission taken back stays so), or
    /// those an uninstall kept.
    pub fn install(&self, token: &str, lang: Lang) -> Result<PluginView, Refusal> {
        let staged = self.staged.lock().remove(token).ok_or_else(|| {
            Refusal::other(tr_in!(
                lang,
                "Cette installation n'est plus d'actualité : choisis le plugin à nouveau.",
                "This install is no longer current: choose the plugin again."
            ))
        })?;
        let _w = self.writing.lock();
        let mut installed = staged.installed;
        installed.installed_at = now();
        let id = installed.id.clone();
        let before = self
            .loaded
            .read()
            .iter()
            .find(|l| l.installed.id == id)
            .map(|l| l.installed.choices.clone());
        installed.choices = before.or_else(|| self.kept(&id)).unwrap_or_default();
        let copy = self.data.plugins_dir().join(&id);
        match staged.dir {
            Some(dir) => {
                remove_dir(&copy);
                if let Err(e) = std::fs::rename(&dir, &copy) {
                    remove_dir(&dir);
                    return Err(Refusal::other(tr_in!(
                        lang,
                        "Le plugin n'a pas pu être copié dans {} : {e}",
                        "The plugin could not be copied to {}: {e}",
                        copy.display()
                    )));
                }
            }
            None => {
                // A copy of the same id goes: the folder takes its place (unless it is that copy).
                if let Origin::Dev { path } = &installed.origin {
                    if std::fs::canonicalize(&copy).ok() != std::fs::canonicalize(path).ok() {
                        remove_dir(&copy);
                    }
                }
            }
        }
        let _ = std::fs::remove_file(self.kept_file(&id));
        self.updates.write().remove(&id);
        let dev = match &installed.origin {
            Origin::Dev { path } => Some(PathBuf::from(path)),
            _ => None,
        };
        let view = self.put(installed, lang)?;
        if let Some(path) = dev {
            self.watch(&id, &path);
        }
        Ok(view)
    }

    /// Sets aside a copy of the folder `src`: its history (`.git`) and its links left out. Gives
    /// the copy and the SHA-256 of its files.
    fn set_aside(&self, src: &Path, lang: Lang) -> Result<(PathBuf, String), Refusal> {
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
        match copy_tree(src, &incoming).and_then(|_| digest(&incoming)) {
            Ok(d) => Ok((incoming, d)),
            Err(e) => {
                remove_dir(&incoming);
                Err(fail(e))
            }
        }
    }

    /// Where an uninstall that keeps a plugin's data keeps its choices.
    fn kept_file(&self, id: &str) -> PathBuf {
        self.data
            .plugins_dir()
            .join(".kept")
            .join(format!("{id}.json"))
    }

    fn kept(&self, id: &str) -> Option<Choices> {
        std::fs::read(self.kept_file(id))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    }

    /// Changes the user's choices of the plugin `id` with `f`, the list saved; false when it is not
    /// installed.
    pub fn choose(&self, id: &str, f: impl FnOnce(&mut Choices)) -> std::io::Result<bool> {
        let _w = self.writing.lock();
        {
            let mut loaded = self.loaded.write();
            let Some(l) = loaded.iter_mut().find(|l| l.installed.id == id) else {
                return Ok(false);
            };
            f(&mut l.installed.choices);
        }
        self.save()?;
        Ok(true)
    }

    /// The choices of the plugin `id`, when it is installed.
    pub fn choices(&self, id: &str) -> Option<Choices> {
        self.loaded
            .read()
            .iter()
            .find(|l| l.installed.id == id)
            .map(|l| l.installed.choices.clone())
    }

    /// Uninstalls a plugin: its copy removed (a folder in development is left where it is), and
    /// its settings with it; `keep`: its choices kept for a later install of the same id. Its
    /// secrets are the caller's (`secret_keys`).
    pub fn remove(&self, id: &str, keep: bool) -> std::io::Result<bool> {
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
        self.updates.write().remove(id);
        if !matches!(gone.installed.origin, Origin::Dev { .. }) {
            remove_dir_patiently(&gone.folder);
        }
        let kept = self.kept_file(id);
        if keep {
            if let Some(dir) = kept.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let bytes = serde_json::to_vec_pretty(&gone.installed.choices)
                .map_err(std::io::Error::other)?;
            crate::paths::write_atomic(&kept, &bytes)?;
        } else {
            let _ = std::fs::remove_file(&kept);
        }
        self.save()?;
        Ok(true)
    }

    /// Looks for a newer version of each installed plugin: a later tag of its repository, a later
    /// version in the folder it was copied from. Offered (`PluginView::update`), never installed.
    pub async fn check_updates(&self) {
        let installed: Vec<(Installed, String)> = self
            .loaded
            .read()
            .iter()
            .filter_map(|l| {
                let m = l.read.as_ref().ok()?.manifest.as_ref().ok()?;
                Some((l.installed.clone(), m.version.clone()))
            })
            .collect();
        for (i, version) in installed {
            let found = match &i.origin {
                Origin::Git { url, .. } => newer_tag(url, &version).await,
                Origin::Folder { path } => {
                    std::fs::read_to_string(Path::new(path).join(manifest::FILE))
                        .ok()
                        .and_then(|t| manifest::read(&t, false).manifest.ok())
                        .filter(|m| {
                            m.id == i.id
                                && manifest::version(&m.version) > manifest::version(&version)
                        })
                        .map(|m| Update {
                            version: m.version,
                            tag: None,
                        })
                }
                Origin::Dev { .. } => None,
            };
            let mut updates = self.updates.write();
            match found {
                Some(u) => updates.insert(i.id.clone(), u),
                None => updates.remove(&i.id),
            };
        }
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
    pub(crate) fn reload(&self, ids: &[String]) {
        let mut read = Vec::new();
        {
            let _w = self.writing.lock();
            let mut loaded = self.loaded.write();
            for l in loaded.iter_mut() {
                if !ids.contains(&l.installed.id)
                    || !matches!(l.installed.origin, Origin::Dev { .. })
                {
                    continue;
                }
                // Its files as they were last read: nothing to read again, its process goes on.
                if let Some(seen) = &mut l.seen {
                    let Ok(now) = stamp(&l.folder) else {
                        continue;
                    };
                    if now == seen.stamp {
                        continue;
                    }
                    if digest(&l.folder).is_ok_and(|d| d == seen.digest) {
                        seen.stamp = now;
                        continue;
                    }
                }
                *l = self.read(l.installed.clone());
                read.push(l.installed.id.clone());
            }
        }
        for id in &read {
            for hook in self.reloaded.lock().iter() {
                hook(id);
            }
        }
        self.revision.fetch_add(1, Ordering::AcqRel);
    }

    /// Calls `f` with the id of each folder in development read again.
    pub fn on_reload(&self, f: impl Fn(&str) + Send + Sync + 'static) {
        self.reloaded.lock().push(Box::new(f));
    }

    /// How many times the folders in development were read again.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
}

/// The latest tag of the repository `url` whose version is past `version`.
async fn newer_tag(url: &str, version: &str) -> Option<Update> {
    if url.starts_with('-') {
        return None;
    }
    let cwd = std::env::temp_dir().display().to_string();
    let out = crate::git::text(&cwd, &["ls-remote", "--tags", "--refs", "--", url])
        .await
        .ok()?;
    let mine = manifest::version(version);
    out.lines()
        .filter_map(|l| {
            l.split_once("refs/tags/")
                .map(|(_, t)| t.trim().to_string())
        })
        .filter_map(|t| {
            let v = crate::engine::registry::Version::find(&t)?;
            (v > mine).then_some((v, t))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(v, t)| Update {
            version: v.to_string(),
            tag: Some(t),
        })
}

/// The icon `rel` of the plugin's folder `folder` as a `data:image/png` address, when it holds:
/// read in the folder and never out of it, its header checked again.
fn icon_data(folder: &Path, rel: &str) -> Option<String> {
    use base64::Engine as _;
    let bytes = manifest::icon_file(folder, rel)?;
    if !manifest::icon_problems(&bytes).is_empty() {
        return None;
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{b64}"))
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

/// Removes a folder its plugin's process may still hold for a moment after it was stopped (Windows
/// keeps the files and the working folder of a process until it is gone): tried again for 3 s.
fn remove_dir_patiently(dir: &Path) {
    for _ in 0..30 {
        if !dir.exists() || std::fs::remove_dir_all(dir).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    remove_dir(dir);
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

/// A stamp of a folder's files, without reading them: their paths, sizes and times of change.
fn stamp(dir: &Path) -> std::io::Result<String> {
    let mut files = Vec::new();
    files_of(dir, "", &mut files)?;
    files.sort();
    let mut h = Sha256::new();
    for (rel, path) in files {
        let meta = std::fs::metadata(path)?;
        let changed = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        h.update(rel.as_bytes());
        h.update([0]);
        h.update(meta.len().to_le_bytes());
        h.update(changed.to_le_bytes());
    }
    Ok(format!("{:x}", h.finalize()))
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
