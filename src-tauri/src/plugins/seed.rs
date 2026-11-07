//! The app's seed (ESC-338, docs/PLUGINS.md §8.7): what the store published for the last version of each plugin of the
//! first hand, shipped in the installer, the same bytes and the same signature: `<seed>/<id>/version.json`, its
//! `.minisig`, `archive.tar.gz` and `plugin.wasm`, with the revocation list of the day (`<seed>/revocations.json`).
//!
//! The list is `plugins/seed.json`, committed and embedded: it names each version by the SHA-256 of its `version.json`,
//! so the files beside the app are worth what that fingerprint and their signature say, nothing more. What is taken from
//! here goes through the install of the store (`Plugins::inspect_store`, `install`), with no network: the checks are
//! `store`'s own, an origin `store` marked `seed`. Then these are plugins like any other: switched off, uninstalled, updated
//! on a click only. Nothing here updates anything, nothing here goes back a version.

use super::store::{self, Fetched, Reason, RevocationList};
use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `plugins/seed.json`, as the repository holds it.
const LIST: &str = include_str!("../../../plugins/seed.json");

/// The version the list reads.
const FORMAT: u32 = 1;

/// A version of the seed: `version.json`'s SHA-256 is what binds the app to its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub version: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
struct List {
    format: u32,
    plugins: Vec<Entry>,
}

/// Reads a `seed.json`.
pub fn parse(text: &str) -> Result<Vec<Entry>, String> {
    let list: List = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if list.format != FORMAT {
        return Err(format!("format {} is not {FORMAT}", list.format));
    }
    Ok(list.plugins)
}

/// The app's seed.
#[derive(Debug, Clone, Default)]
pub struct Seed {
    /// Where its files are (Tauri's resources), when the app has them.
    pub dir: Option<PathBuf>,
    pub entries: Vec<Entry>,
    /// Whether a version without a signature is taken: a build of development alone, whose modules are those of
    /// `scripts/seed-dev.sh`, as it runs a program that is not signed.
    pub unsigned: bool,
}

impl Seed {
    /// The seed of this app: the list it embeds, its files in `dir`. In a build of development, `ESCOUADE_SEED_DIR` names
    /// the folder (the end-to-end tests', `scripts/seed-dev.sh`'s) and its own `seed.json`, when there is one, takes
    /// the place of the list: a release reads neither.
    pub fn of_app(dir: Option<PathBuf>) -> Self {
        let dir = store::debug_env("ESCOUADE_SEED_DIR")
            .map(PathBuf::from)
            .or(dir);
        let own = dir
            .as_ref()
            .filter(|_| cfg!(debug_assertions))
            .and_then(|d| std::fs::read_to_string(d.join("seed.json")).ok());
        let mut entries = match parse(own.as_deref().unwrap_or(LIST)) {
            Ok(entries) => entries,
            Err(e) => {
                log::error!("the seed's list does not read: {e}");
                Vec::new()
            }
        };
        if own.is_none() {
            entries.retain(|e| {
                let ours = super::builtin::seeds(&e.id);
                if !ours {
                    log::error!("{} is not an id of the seed (plugins/builtin.rs)", e.id);
                }
                ours
            });
        }
        Self {
            dir,
            entries,
            unsigned: cfg!(debug_assertions),
        }
    }

    /// Whether the seed names the plugin `id`.
    pub fn has(&self, id: &str) -> bool {
        self.entry(id).is_some()
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// The revocation list the seed carries, when it holds with the app's keys.
    pub fn revocations(&self) -> Option<RevocationList> {
        let dir = self.dir.as_ref()?;
        let json = std::fs::read(dir.join("revocations.json")).ok()?;
        let sig = std::fs::read_to_string(dir.join("revocations.json.minisig")).ok()?;
        match store::check_list(&super::store_keys::keys(), &json, &sig, 0) {
            Ok(list) => Some(list),
            Err(e) => {
                log::warn!("the seed's revocation list is refused: {e:?}");
                None
            }
        }
    }

    /// The version `e` read from the seed and checked as `Store::fetch` checks a download: `version.json` against the
    /// fingerprint the app embeds, its signature with the app's keys less those `list` revokes, the archive and the module
    /// against `version.json`. Nothing is written. A version the list takes the seal off does not install, and says why;
    /// one the list withdraws (`disable`) does, stopped with its reason, as it would be had it come earlier.
    pub fn take(
        &self,
        e: &Entry,
        list: Option<&RevocationList>,
        lang: Lang,
    ) -> Result<Fetched, String> {
        let folder = self
            .dir
            .as_ref()
            .map(|d| d.join(&e.id))
            .filter(|d| d.is_dir())
            .ok_or_else(|| {
                tr_in!(
                    lang,
                    "L'amorce de cette version d'Escouade n'a pas {} : rien n'est installé.",
                    "This version of Escouade's seed does not hold {}: nothing is installed.",
                    e.id
                )
            })?;
        let json = bytes(&folder, "version.json", store::MODULE_MAX, lang)?;
        let file = store::check_version_json(&json, &e.sha256, &e.id, &e.version, lang)?;
        if let Some(l) = list.and_then(|l| l.find(&e.id, &e.version, "unverify")) {
            return Err(tr_in!(
                lang,
                "Escouade a retiré la signature de {} v{} : {}",
                "Escouade took the signature of {} v{} back: {}",
                e.id,
                e.version,
                l.reason.get(lang)
            ));
        }
        let minisig = match std::fs::read_to_string(folder.join("version.json.minisig")) {
            Ok(sig) => store::check_seal(&json, sig, &e.id, &e.version, list, lang)?,
            Err(_) => None,
        };
        let unsigned = minisig.is_none();
        if unsigned && !self.unsigned {
            return Err(tr_in!(
                lang,
                "{} v{} n'est pas signé : un plugin de l'amorce ne s'installe que signé.",
                "{} v{} is not signed: a plugin of the seed installs signed only.",
                e.id,
                e.version
            ));
        }
        let archive = bytes(&folder, "archive.tar.gz", store::ARCHIVE_MAX, lang)?;
        let files = store::check_archive(&archive, &file, lang)?;
        let module = match &file.wasm {
            Some(w) => {
                let module = bytes(&folder, "plugin.wasm", store::MODULE_MAX, lang)?;
                store::check_module(&module, w, lang)?;
                Some(module)
            }
            None => None,
        };
        Ok(Fetched {
            id: e.id.clone(),
            version: e.version.clone(),
            sha256: e.sha256.clone(),
            repository: file.repository,
            path: file.path,
            tag: file.tag,
            commit: file.commit,
            version_json: json,
            minisig,
            unverified: None,
            files,
            program: None,
            module,
            seed: true,
            unsigned,
        })
    }
}

/// A file of the seed, `max` bytes at most.
fn bytes(folder: &Path, name: &str, max: u64, lang: Lang) -> Result<Vec<u8>, String> {
    let path = folder.join(name);
    let bytes = std::fs::metadata(&path)
        .ok()
        .filter(|m| m.len() <= max)
        .and_then(|_| std::fs::read(&path).ok());
    bytes.ok_or_else(|| {
        tr_in!(
            lang,
            "{} de l'amorce ne se lit pas : rien n'est installé.",
            "The seed's {} does not read: nothing is installed.",
            path.display()
        )
    })
}

/// What the app keeps of the seed beside the plugins (`<plugins>/.seed.json`): what the user uninstalled, which the seed
/// does not put back.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct Notes {
    #[serde(default)]
    pub removed: BTreeMap<String, Removed>,
}

/// A plugin of the seed the user uninstalled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Removed {
    /// The version they had.
    pub version: String,
    /// Its name then, for the line of the view.
    pub name: Reason,
}

/// A plugin of the seed that is not installed, as the view proposes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub id: String,
    pub name: String,
    pub version: String,
    /// The version the user had when they uninstalled it; none if they never did.
    pub removed: Option<String>,
    /// Why the seed did not install it, when it tried.
    pub refused: Option<String>,
}
