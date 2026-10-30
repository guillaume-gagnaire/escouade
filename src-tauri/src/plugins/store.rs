//! The store in the app (STORE-9, docs/STORE.md §5): the catalogue of store.escouade.dev, kept on
//! the machine and searched there; the revocation list, signed, read at the start and every 6 hours,
//! its `serial` never going back; and a version's files, downloaded and checked before anything is
//! set aside: `version.json` against the catalogue's fingerprint, the archive and the program
//! against `version.json`, the signature with the keys the app embeds (`store_keys`). A gap stops
//! everything and says so. The store cannot make a seal: it has no key.
//!
//! Everything goes through Réglages › Réseau (proxy, TLS) as the app's other requests; nothing is
//! sent but plain GETs, and the count of installations when the user accepted the statistics.

use super::manifest::{self, Range};
use super::store_keys;
use crate::i18n::Lang;
use crate::model::Settings;
use crate::paths::DataDir;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

/// Where the store is.
pub const STORE_URL: &str = "https://store.escouade.dev";
/// Where the copy of the revocation list is, when the store does not answer (§4.6).
pub const MIRROR_URL: &str = "https://escouade.dev";
/// How often the revocation list (and the catalogue with it) is read again.
pub const EVERY: Duration = Duration::from_secs(6 * 3600);
/// The archive as the store keeps it: 50 MB at most (as `store/server/utils/tar.ts`).
pub const ARCHIVE_MAX: u64 = 50 * 1024 * 1024;
/// What an archive may unpack to: a gzip bomb stops here.
pub const UNPACKED_MAX: u64 = 200 * 1024 * 1024;
/// The entries an archive may hold.
pub const ENTRIES_MAX: usize = 20_000;
/// A program of a release: 100 MB at most.
pub const BINARY_MAX: u64 = 100 * 1024 * 1024;
/// The catalogue, `version.json`, a signature, a list: 20 MB at most.
const SMALL_MAX: u64 = 20 * 1024 * 1024;

/// The store's address: in a debug build, `ESCOUADE_STORE_URL` when set (the fake store of the
/// end-to-end tests).
pub fn store_url() -> String {
    debug_env("ESCOUADE_STORE_URL").unwrap_or_else(|| STORE_URL.into())
}

/// The copy's address: in a debug build, `ESCOUADE_STORE_MIRROR_URL` when set.
pub fn mirror_url() -> String {
    debug_env("ESCOUADE_STORE_MIRROR_URL").unwrap_or_else(|| MIRROR_URL.into())
}

fn debug_env(name: &str) -> Option<String> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var(name)
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
}

/// The system and the architecture of this machine, as the store names a release's program.
pub fn platform() -> String {
    let system = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        _ => "linux",
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        _ => "x64",
    };
    format!("{system}-{arch}")
}

/// The client of the store's requests: Réglages › Réseau's proxy, the certificates the system
/// trusts (a corporate proxy's own authority), TLS unchecked when the user switched it off.
pub fn client(settings: &Settings) -> reqwest::Result<reqwest::Client> {
    let mut b = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .user_agent(concat!("escouade/", env!("CARGO_PKG_VERSION")))
        .danger_accept_invalid_certs(settings.insecure_tls);
    let url = settings.proxy_url.trim();
    if !url.is_empty() {
        let proxy =
            reqwest::Proxy::all(url)?.no_proxy(reqwest::NoProxy::from_string(&settings.no_proxy));
        b = b.proxy(proxy);
    }
    b.build()
}

// ---- The catalogue (store/server/utils/catalog.ts) ----

/// The catalogue as the store serves it (`/api/v1/catalog.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub format: u32,
    pub updated_at: String,
    #[serde(default)]
    pub installs_until: String,
    #[serde(default)]
    pub revocations: Option<Value>,
    pub plugins: Vec<CatalogPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogPlugin {
    pub id: String,
    /// A text, or one per language (`{ fr, en }`).
    #[serde(default)]
    pub name: Value,
    #[serde(default)]
    pub description: Value,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub repository: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub installs: u64,
    #[serde(default)]
    pub latest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transferred_at: Option<String>,
    pub versions: Vec<CatalogVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogVersion {
    pub version: String,
    #[serde(default)]
    pub published_at: String,
    /// The SHA-256 of its `version.json`: the version's fingerprint.
    pub sha256: String,
    pub archive: ArchiveRef,
    #[serde(default)]
    pub platforms: Vec<String>,
    /// `{ api, min, max? }`, the api a number or a text.
    #[serde(default)]
    pub escouade: Value,
    #[serde(default)]
    pub verified: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<Value>,
    #[serde(default)]
    pub yanked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unverified: Option<Value>,
    #[serde(default)]
    pub permissions: Value,
    #[serde(default)]
    pub run: bool,
    #[serde(default)]
    pub contributes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<Value>,
    pub files: Files,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveRef {
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Files {
    pub version_json: String,
    pub archive: String,
    #[serde(default)]
    pub binaries: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl CatalogVersion {
    /// The Escouade it was written for, as a manifest's `escouade`.
    pub fn range(&self) -> Option<Range> {
        let api = match self.escouade.get("api")? {
            Value::Number(n) => n.as_u64()?,
            Value::String(s) => s.trim().parse().ok()?,
            _ => return None,
        };
        Some(Range {
            api,
            min: self.escouade.get("min")?.as_str()?.to_string(),
            max: self
                .escouade
                .get("max")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    /// Whether this Escouade (`app`) can run it.
    pub fn runs_on(&self, app: &str) -> bool {
        self.range()
            .is_some_and(|r| r.refusal(app, Lang::En).is_none())
    }

    fn pre_release(&self) -> bool {
        self.version.contains('-')
    }
}

impl CatalogPlugin {
    /// The version the app offers (docs/STORE.md §8.1): the last one this Escouade can run, not
    /// taken back by its author; a release before a pre-release.
    pub fn best(&self, app: &str) -> Option<&CatalogVersion> {
        let mut ok: Vec<&CatalogVersion> = self
            .versions
            .iter()
            .filter(|v| !v.yanked && v.runs_on(app))
            .collect();
        ok.sort_by_key(|v| std::cmp::Reverse(manifest::version(&v.version)));
        ok.iter()
            .find(|v| !v.pre_release())
            .or_else(|| ok.first())
            .copied()
    }

    pub fn version(&self, version: &str) -> Option<&CatalogVersion> {
        self.versions.iter().find(|v| v.version == version)
    }
}

// ---- The revocation list (§4.6, store/server/utils/revocation-list.ts) ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevocationList {
    pub serial: u64,
    pub issued_at: String,
    pub entries: Vec<RevocationEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys_revoked: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevocationEntry {
    pub id: String,
    /// `"*"` or a list of versions.
    pub versions: Value,
    /// `unverify` (the seal goes) or `disable` (the emergency withdrawal).
    pub action: String,
    pub reason: Reason,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reason {
    pub fr: String,
    pub en: String,
}

impl Reason {
    pub fn get(&self, lang: Lang) -> &str {
        match lang {
            Lang::Fr => &self.fr,
            Lang::En => &self.en,
        }
    }
}

impl RevocationList {
    /// The entry of `action` that covers version `version` of plugin `id`.
    pub fn find(&self, id: &str, version: &str, action: &str) -> Option<&RevocationEntry> {
        self.entries.iter().find(|e| {
            e.id == id
                && e.action == action
                && match &e.versions {
                    Value::String(s) => s == "*",
                    Value::Array(vs) => vs.iter().any(|v| v.as_str() == Some(version)),
                    _ => false,
                }
        })
    }

    /// The keys a seal is checked with under this list: `keys` less those it revokes.
    pub fn trusted(&self, keys: &[String]) -> Vec<String> {
        let revoked = self.keys_revoked.clone().unwrap_or_default();
        keys.iter()
            .filter(|k| !revoked.contains(&store_keys::key_id(k)))
            .cloned()
            .collect()
    }
}

/// Why a revocation list was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum ListRefused {
    /// It does not read as a list.
    Malformed,
    /// Not signed by a key of the app, or signed for another serial.
    Signature(store_keys::Refused),
    /// Signed by a key the list itself revokes.
    RevokedSigner,
    /// Its serial is below the last one seen: an old list played again.
    Older { serial: u64, seen: u64 },
}

/// Reads and checks a revocation list: signed by one of `keys` with its own serial, not by a key
/// it revokes, and not below `seen` (the serial of the last list taken).
pub fn check_list(
    keys: &[String],
    json: &[u8],
    minisig: &str,
    seen: u64,
) -> Result<RevocationList, ListRefused> {
    let list: RevocationList = serde_json::from_slice(json).map_err(|_| ListRefused::Malformed)?;
    if list.serial < 1 || list.entries.iter().any(|e| e.id.trim().is_empty()) {
        return Err(ListRefused::Malformed);
    }
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    let signer = store_keys::revocations_signed(&keys, json, minisig, list.serial)
        .map_err(ListRefused::Signature)?;
    if list
        .keys_revoked
        .as_ref()
        .is_some_and(|r| r.contains(&signer))
    {
        return Err(ListRefused::RevokedSigner);
    }
    if list.serial < seen {
        return Err(ListRefused::Older {
            serial: list.serial,
            seen,
        });
    }
    Ok(list)
}

// ---- The cache ----

/// What the store last gave, kept on the machine (`<data>/store/`).
#[derive(Debug, Clone, Default)]
struct Cache {
    catalog: Option<Catalog>,
    etag: String,
    /// When the catalogue was last read from the store (RFC 3339).
    read_at: Option<String>,
    /// Why the last reading failed (the catalogue kept is older).
    error: Option<String>,
    revocations: Option<RevocationList>,
    revocations_read_at: Option<String>,
    /// Where the list was read: `store` or `mirror`.
    revocations_from: Option<String>,
    revocations_error: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    #[serde(default)]
    etag: String,
    #[serde(default)]
    read_at: Option<String>,
    #[serde(default)]
    revocations_read_at: Option<String>,
    #[serde(default)]
    revocations_from: Option<String>,
}

/// The store as the window shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreView {
    pub url: String,
    pub catalog: Option<Catalog>,
    pub read_at: Option<String>,
    pub error: Option<String>,
    /// By plugin id, the version this Escouade would install (`CatalogPlugin::best`).
    pub best: BTreeMap<String, String>,
    pub revocations: Option<ListView>,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListView {
    pub serial: Option<u64>,
    pub read_at: Option<String>,
    pub from: Option<String>,
    pub error: Option<String>,
}

/// A version the store offers for an installed plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub version: String,
    pub verified: bool,
}

/// The store of the app: its catalogue and its revocation list, cached.
pub struct Store {
    dir: PathBuf,
    base: String,
    mirror: String,
    app_version: String,
    cache: RwLock<Cache>,
    /// One reading at a time.
    reading: tokio::sync::Mutex<()>,
}

impl Store {
    /// The store at `base` (its copy of the list at `mirror`), its cache in `data` read again.
    pub fn load(data: &DataDir, base: String, mirror: String, app_version: &str) -> Self {
        let dir = data.root().join("store");
        let meta: Meta = std::fs::read(dir.join("meta.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let catalog: Option<Catalog> = std::fs::read(dir.join("catalog.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        // The list kept is checked again: a file changed on the disk is not taken.
        let revocations = match (
            std::fs::read(dir.join("revocations.json")),
            std::fs::read_to_string(dir.join("revocations.json.minisig")),
        ) {
            (Ok(json), Ok(sig)) => check_list(&store_keys::keys(), &json, &sig, 0).ok(),
            _ => None,
        };
        Self {
            dir,
            base,
            mirror,
            app_version: app_version.to_string(),
            cache: RwLock::new(Cache {
                etag: if catalog.is_some() {
                    meta.etag
                } else {
                    String::new()
                },
                catalog,
                read_at: meta.read_at,
                revocations,
                revocations_read_at: meta.revocations_read_at,
                revocations_from: meta.revocations_from,
                ..Cache::default()
            }),
            reading: tokio::sync::Mutex::new(()),
        }
    }

    fn save_meta(&self) {
        let c = self.cache.read();
        let meta = Meta {
            etag: c.etag.clone(),
            read_at: c.read_at.clone(),
            revocations_read_at: c.revocations_read_at.clone(),
            revocations_from: c.revocations_from.clone(),
        };
        drop(c);
        if let Ok(bytes) = serde_json::to_vec_pretty(&meta) {
            let _ = std::fs::create_dir_all(&self.dir);
            let _ = crate::paths::write_atomic(&self.dir.join("meta.json"), &bytes);
        }
    }

    /// The store as the window shows it.
    pub fn view(&self) -> StoreView {
        let c = self.cache.read();
        let best = c
            .catalog
            .iter()
            .flat_map(|cat| &cat.plugins)
            .filter_map(|p| Some((p.id.clone(), p.best(&self.app_version)?.version.clone())))
            .collect();
        StoreView {
            url: self.base.clone(),
            catalog: c.catalog.clone(),
            read_at: c.read_at.clone(),
            error: c.error.clone(),
            best,
            revocations: Some(ListView {
                serial: c.revocations.as_ref().map(|l| l.serial),
                read_at: c.revocations_read_at.clone(),
                from: c.revocations_from.clone(),
                error: c.revocations_error.clone(),
            }),
            platform: platform(),
        }
    }

    /// The revocation list taken, if any.
    pub fn revocations(&self) -> Option<RevocationList> {
        self.cache.read().revocations.clone()
    }

    /// A plugin of the catalogue kept.
    pub fn plugin(&self, id: &str) -> Option<CatalogPlugin> {
        self.cache
            .read()
            .catalog
            .as_ref()?
            .plugins
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    /// By id, what the store offers: the last version this Escouade can run, and whether its seal
    /// holds as the catalogue says (an `unverify` entry of the list takes it off).
    pub fn offers(&self) -> HashMap<String, Offer> {
        let c = self.cache.read();
        let Some(cat) = c.catalog.as_ref() else {
            return HashMap::new();
        };
        cat.plugins
            .iter()
            .filter_map(|p| {
                let v = p.best(&self.app_version)?;
                let unverified = c
                    .revocations
                    .as_ref()
                    .is_some_and(|l| l.find(&p.id, &v.version, "unverify").is_some());
                Some((
                    p.id.clone(),
                    Offer {
                        version: v.version.clone(),
                        verified: v.verified && v.unverified.is_none() && !unverified,
                    },
                ))
            })
            .collect()
    }

    /// Reads the catalogue again (`If-None-Match`: a 304 keeps it), then the revocation list.
    /// A failure keeps what was cached and says it (`StoreView::error`).
    pub async fn refresh(&self, http: &reqwest::Client, lang: Lang) {
        let _r = self.reading.lock().await;
        let etag = self.cache.read().etag.clone();
        let mut req = http.get(format!("{}/api/v1/catalog.json", self.base));
        if !etag.is_empty() {
            req = req.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        let got = async {
            let res = req.send().await.map_err(|e| e.to_string())?;
            let status = res.status();
            if status == reqwest::StatusCode::NOT_MODIFIED {
                return Ok(None);
            }
            if !status.is_success() {
                return Err(format!("HTTP {status}"));
            }
            let etag = res
                .headers()
                .get(reqwest::header::ETAG)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let bytes = bounded(res, SMALL_MAX).await?;
            let cat: Catalog = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            Ok(Some((etag, bytes, cat)))
        }
        .await;
        match got {
            Ok(new) => {
                if let Some((etag, bytes, _)) = &new {
                    let _ = std::fs::create_dir_all(&self.dir);
                    let _ = crate::paths::write_atomic(&self.dir.join("catalog.json"), bytes);
                    self.cache.write().etag = etag.clone();
                }
                let mut c = self.cache.write();
                if let Some((_, _, cat)) = new {
                    c.catalog = Some(cat);
                }
                c.read_at = Some(now());
                c.error = None;
            }
            Err(e) => {
                self.cache.write().error = Some(tr_in!(
                    lang,
                    "Le store ne répond pas ({e}) : le catalogue gardé sur ce poste est montré.",
                    "The store does not answer ({e}): the catalog kept on this machine is shown."
                ));
            }
        }
        self.save_meta();
        self.refresh_revocations_locked(http, lang).await;
    }

    async fn refresh_revocations_locked(&self, http: &reqwest::Client, lang: Lang) {
        let seen = self
            .cache
            .read()
            .revocations
            .as_ref()
            .map_or(0, |l| l.serial);
        let keys = store_keys::keys();
        let from_store = fetch_list(
            http,
            &format!("{}/api/v1/revocations.json", self.base),
            &keys,
            seen,
        )
        .await;
        let (got, from) = match from_store {
            Ok(got) => (Ok(got), "store"),
            Err(first) => {
                match fetch_list(
                    http,
                    &format!("{}/revocations.json", self.mirror),
                    &keys,
                    seen,
                )
                .await
                {
                    Ok(got) => (Ok(got), "mirror"),
                    Err(_) => (Err(first), "store"),
                }
            }
        };
        match got {
            Ok(None) => {
                let mut c = self.cache.write();
                c.revocations_read_at = Some(now());
                c.revocations_from = Some(from.into());
                c.revocations_error = None;
            }
            Ok(Some((json, sig, list))) => {
                let _ = std::fs::create_dir_all(&self.dir);
                let kept = crate::paths::write_atomic(&self.dir.join("revocations.json"), &json)
                    .and_then(|_| {
                        crate::paths::write_atomic(
                            &self.dir.join("revocations.json.minisig"),
                            sig.as_bytes(),
                        )
                    });
                if let Err(e) = kept {
                    log::warn!("the revocation list is not kept: {e}");
                }
                let mut c = self.cache.write();
                c.revocations = Some(list);
                c.revocations_read_at = Some(now());
                c.revocations_from = Some(from.into());
                c.revocations_error = None;
            }
            Err(e) => {
                self.cache.write().revocations_error = Some(list_error(&e, lang));
            }
        }
        self.save_meta();
    }

    /// Downloads version `version` of plugin `id` (the one this Escouade would install when None)
    /// and checks it: `version.json` against the catalogue, the signature with the app's keys when
    /// the catalogue says it is verified, the archive and this system's program against
    /// `version.json`, the manifest against its fingerprint. Refused when `verified_only` and no
    /// seal holds. Nothing is written.
    pub async fn fetch(
        &self,
        http: &reqwest::Client,
        id: &str,
        version: Option<&str>,
        verified_only: bool,
        lang: Lang,
    ) -> Result<Fetched, String> {
        let plugin = self.plugin(id).ok_or_else(|| {
            tr_in!(
                lang,
                "{id} n'est pas dans le catalogue du store.",
                "{id} is not in the store's catalog."
            )
        })?;
        let entry = match version {
            Some(v) => plugin.version(v).filter(|e| !e.yanked),
            None => plugin.best(&self.app_version),
        }
        .ok_or_else(|| {
            tr_in!(
                lang,
                "Le store n'a pas de version de {id} que cette version d'Escouade puisse faire tourner.",
                "The store has no version of {id} this version of Escouade can run."
            )
        })?
        .clone();
        let list = self.revocations();
        if list
            .as_ref()
            .is_some_and(|l| l.find(id, &entry.version, "disable").is_some())
        {
            return Err(tr_in!(
                lang,
                "Cette version a été retirée par Escouade : elle ne s'installe pas.",
                "This version was withdrawn by Escouade: it does not install."
            ));
        }
        let base = &self.base;
        let get = |path: &str, max: u64| {
            let url = format!("{base}{path}");
            async move {
                let res = http.get(&url).send().await.map_err(|e| e.to_string())?;
                if !res.status().is_success() {
                    return Err(format!("HTTP {}", res.status()));
                }
                bounded(res, max).await
            }
        };
        let fail = |what: &str, e: String| {
            tr_in!(
                lang,
                "{what} n'a pas pu être téléchargé du store : {e}",
                "{what} could not be downloaded from the store: {e}"
            )
        };

        // 1. version.json, the bytes the catalogue (and the signature) name.
        let json = get(&entry.files.version_json, SMALL_MAX)
            .await
            .map_err(|e| fail("version.json", e))?;
        if sha256(&json) != entry.sha256 {
            return Err(tr_in!(
                lang,
                "Le version.json reçu ne correspond pas à celui que le catalogue annonce : rien n'est installé.",
                "The version.json received is not the one the catalog names: nothing is installed."
            ));
        }
        let file: VersionFile = serde_json::from_slice(&json).map_err(|e| e.to_string())?;
        if file.id != id || file.version != entry.version {
            return Err(tr_in!(
                lang,
                "Le version.json reçu est celui d'une autre version ({}@{}) : rien n'est installé.",
                "The version.json received is another version's ({}@{}): nothing is installed.",
                file.id,
                file.version
            ));
        }

        // 2. The seal: checked here, with the app's keys, never taken from the store's word.
        let unverified = list
            .as_ref()
            .is_some_and(|l| l.find(id, &entry.version, "unverify").is_some());
        let mut minisig = None;
        if entry.verified && !unverified {
            if let Some(path) = &entry.files.signature {
                let sig = get(path, SMALL_MAX)
                    .await
                    .map_err(|e| fail(".minisig", e))?;
                let sig = String::from_utf8(sig).map_err(|e| e.to_string())?;
                let keys = trusted_keys(list.as_ref());
                let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
                match store_keys::verified_with(&refs, &json, &sig, id, &entry.version) {
                    Ok(()) => minisig = Some(sig),
                    // An app that embeds no key yet shows no seal; it does not refuse.
                    Err(store_keys::Refused::NoKey) => {}
                    Err(_) => {
                        return Err(tr_in!(
                            lang,
                            "La signature de cette version ne tient pas avec les clés d'Escouade : rien n'est installé.",
                            "This version's signature does not hold with Escouade's keys: nothing is installed."
                        ))
                    }
                }
            }
        }
        if verified_only && minisig.is_none() {
            return Err(tr_in!(
                lang,
                "La v{} n'est pas vérifiée, et tu n'installes que des versions vérifiées (Plugins › Installés).",
                "v{} is not verified, and you install verified versions only (Plugins › Installed).",
                entry.version
            ));
        }

        // 3. The archive, the bytes version.json names.
        let archive = get(&entry.files.archive, ARCHIVE_MAX)
            .await
            .map_err(|e| fail("archive.tar.gz", e))?;
        if sha256(&archive) != file.archive.sha256 || archive.len() as u64 != file.archive.size {
            return Err(tr_in!(
                lang,
                "L'archive reçue ne correspond pas à celle que le store a publiée : rien n'est installé.",
                "The archive received is not the one the store published: nothing is installed."
            ));
        }
        let files = read_archive(&archive).map_err(|e| {
            tr_in!(
                lang,
                "L'archive ne se lit pas sans danger ({e}) : rien n'est installé.",
                "The archive does not read safely ({e}): nothing is installed."
            )
        })?;
        let manifest_bytes = files
            .iter()
            .find(|(p, _)| p == manifest::FILE)
            .map(|(_, b)| b.as_slice())
            .unwrap_or_default();
        if file.manifest_sha256.is_empty() || sha256(manifest_bytes) != file.manifest_sha256 {
            return Err(tr_in!(
                lang,
                "Le manifeste de l'archive n'est pas celui que version.json décrit : rien n'est installé.",
                "The archive's manifest is not the one version.json describes: nothing is installed."
            ));
        }

        // 4. This system's program, when the release has one.
        let here = platform();
        let program = match (file.binaries.get(&here), entry.files.binaries.get(&here)) {
            (Some(b), Some(path)) => {
                let bytes = get(path, BINARY_MAX).await.map_err(|e| fail(&b.name, e))?;
                if sha256(&bytes) != b.sha256 {
                    return Err(tr_in!(
                        lang,
                        "Le programme reçu pour {here} ne correspond pas à celui que le store a publié : rien n'est installé.",
                        "The program received for {here} is not the one the store published: nothing is installed."
                    ));
                }
                Some(bytes)
            }
            _ => None,
        };
        Ok(Fetched {
            id: id.to_string(),
            version: entry.version.clone(),
            sha256: entry.sha256.clone(),
            repository: file.repository.clone(),
            tag: file.tag.clone(),
            commit: file.commit.clone(),
            version_json: json,
            minisig,
            unverified: list
                .as_ref()
                .and_then(|l| l.find(id, &entry.version, "unverify"))
                .map(|e| e.reason.get(lang).to_string()),
            files,
            program,
        })
    }

    /// Counts an install or an update on the store (`POST /api/v1/installs`): only when the user
    /// accepted the statistics (the caller checks), without any id of this machine.
    pub async fn count(&self, http: &reqwest::Client, id: &str, version: &str, update: bool) {
        let body = serde_json::json!({
            "id": id,
            "version": version,
            "event": if update { "update" } else { "install" },
        });
        if let Err(e) = http
            .post(format!("{}/api/v1/installs", self.base))
            .json(&body)
            .send()
            .await
        {
            log::info!("the store's count of installs is not reached: {e}");
        }
    }
}

/// The app's keys less those the list revokes.
pub fn trusted_keys(list: Option<&RevocationList>) -> Vec<String> {
    let keys = store_keys::keys();
    match list {
        Some(l) => l.trusted(&keys),
        None => keys,
    }
}

/// A list read at `url` and its `.minisig`, checked: None when it is the one already taken (its
/// serial), or when the store has none yet (404).
async fn fetch_list(
    http: &reqwest::Client,
    url: &str,
    keys: &[String],
    seen: u64,
) -> Result<Option<(Vec<u8>, String, RevocationList)>, FetchError> {
    let res = http
        .get(url)
        .send()
        .await
        .map_err(|e| FetchError::Net(e.to_string()))?;
    if res.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !res.status().is_success() {
        return Err(FetchError::Net(format!("HTTP {}", res.status())));
    }
    let json = bounded(res, SMALL_MAX).await.map_err(FetchError::Net)?;
    let sig = http
        .get(format!("{url}.minisig"))
        .send()
        .await
        .map_err(|e| FetchError::Net(e.to_string()))?;
    if !sig.status().is_success() {
        return Err(FetchError::Net(format!("HTTP {}", sig.status())));
    }
    let sig = String::from_utf8(bounded(sig, SMALL_MAX).await.map_err(FetchError::Net)?)
        .map_err(|e| FetchError::Net(e.to_string()))?;
    let list = check_list(keys, &json, &sig, seen).map_err(FetchError::Refused)?;
    if list.serial == seen {
        return Ok(None);
    }
    Ok(Some((json, sig, list)))
}

#[derive(Debug)]
enum FetchError {
    Net(String),
    Refused(ListRefused),
}

fn list_error(e: &FetchError, lang: Lang) -> String {
    match e {
        FetchError::Net(e) => tr_in!(
            lang,
            "La liste de révocation n'a pas pu être lue ({e}) : celle gardée sur ce poste s'applique.",
            "The revocation list could not be read ({e}): the one kept on this machine applies."
        ),
        FetchError::Refused(ListRefused::Older { serial, seen }) => tr_in!(
            lang,
            "Une liste de révocation plus ancienne (nº {serial}, après le nº {seen}) a été refusée.",
            "An older revocation list (no. {serial}, after no. {seen}) was refused."
        ),
        FetchError::Refused(ListRefused::Signature(store_keys::Refused::NoKey)) => tr_in!(
            lang,
            "Cette version d'Escouade n'embarque pas encore les clés du store : la liste de révocation n'est pas lue.",
            "This version of Escouade does not embed the store's keys yet: the revocation list is not read."
        ),
        FetchError::Refused(_) => tr_in!(
            lang,
            "La liste de révocation reçue n'est pas signée par Escouade : elle a été refusée.",
            "The revocation list received is not signed by Escouade: it was refused."
        ),
    }
}

/// `version.json` (store/server/utils/versions.ts), what the app reads of it.
#[derive(Debug, Clone, Deserialize)]
pub struct VersionFile {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub repository: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub commit: String,
    pub archive: ArchiveRef,
    #[serde(default)]
    pub binaries: BTreeMap<String, BinaryRef>,
    #[serde(default)]
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinaryRef {
    #[serde(default)]
    pub name: String,
    pub sha256: String,
}

/// A version downloaded and checked, nothing written yet.
pub struct Fetched {
    pub id: String,
    pub version: String,
    /// The SHA-256 of its `version.json`.
    pub sha256: String,
    /// Where it was read from, as `version.json` says: the repository, the tag, its commit.
    pub repository: String,
    pub tag: String,
    pub commit: String,
    pub version_json: Vec<u8>,
    /// Its signature, when it holds with the app's keys: the seal.
    pub minisig: Option<String>,
    /// Why the revocation list took its seal off, if it did.
    pub unverified: Option<String>,
    /// The archive's regular files, by their path in the plugin.
    pub files: Vec<(String, Vec<u8>)>,
    /// This system's program from the release, when it has one.
    pub program: Option<Vec<u8>>,
}

/// The bytes of an answer, `max` at most.
async fn bounded(mut res: reqwest::Response, max: u64) -> Result<Vec<u8>, String> {
    if res.content_length().is_some_and(|n| n > max) {
        return Err(format!("more than {max} bytes"));
    }
    let mut out = Vec::new();
    while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        out.extend_from_slice(&chunk);
        if out.len() as u64 > max {
            return Err(format!("more than {max} bytes"));
        }
    }
    Ok(out)
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ---- « Pour <projet> » (choice 215) ----

/** A plugin of the catalogue Escouade proposes for a project, with what it saw. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub id: String,
    pub reason: String,
}

/// What Escouade sees, on this machine and in a project, that a plugin of the catalogue would serve:
/// read here, never sent.
#[derive(Debug, Default, Clone)]
pub struct Seen {
    /// The programs of the harnesses' plugins found on the PATH (`vibe`, `gemini`…).
    pub programs: Vec<String>,
    /// The most frequent prefix of ticket keys in the project's commits (`DEM` for DEM-12), when
    /// there are a few.
    pub ticket_prefix: Option<String>,
}

/// Prefixes that look like ticket keys and are not (`UTF-8`, `SHA-256`…).
const NOT_TICKETS: &[&str] = &[
    "UTF", "SHA", "ISO", "RFC", "HTTP", "TLS", "AES", "CVE", "MD", "PR", "CI", "WIP", "UI", "API",
];

/// The most frequent prefix of ticket keys (`ABC-123`) in `text`, if it comes 3 times or more.
pub fn ticket_prefix(text: &str) -> Option<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')) {
        let Some((head, tail)) = word.split_once('-') else {
            continue;
        };
        let key = head.len() >= 2
            && head.len() <= 10
            && head.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && head
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            && !tail.is_empty()
            && tail.len() <= 6
            && tail.chars().all(|c| c.is_ascii_digit());
        if key && !NOT_TICKETS.contains(&head) {
            *counts.entry(head.to_string()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter(|(_, n)| *n >= 3)
        .max_by_key(|(_, n)| *n)
        .map(|(k, _)| k)
}

/// Whether a program named `name` is on the PATH (with the extensions of PATHEXT under Windows).
pub fn on_path(name: &str) -> bool {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return false;
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".into())
            .split(';')
            .map(|e| e.to_ascii_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };
    std::env::split_paths(&path).any(|dir| {
        exts.iter()
            .any(|e| dir.join(format!("{name}{e}")).is_file())
    })
}

/// The categories of the store a suggestion may come from, and the words that name a tracker of
/// keys like DEM-12.
const KEYED_TRACKERS: &[&str] = &["jira", "linear", "youtrack"];

/// Up to `max` plugins of `catalog` that what was `seen` calls for, none of `installed`, the
/// verified first; each with its reason in `lang`.
pub fn suggest(
    catalog: &Catalog,
    app: &str,
    seen: &Seen,
    installed: &[String],
    lang: Lang,
    max: usize,
) -> Vec<Suggestion> {
    let mut out: Vec<(bool, u64, Suggestion)> = Vec::new();
    for p in &catalog.plugins {
        if installed.contains(&p.id) {
            continue;
        }
        let Some(best) = p.best(app) else {
            continue;
        };
        let keywords: Vec<String> = p.keywords.iter().map(|k| k.to_lowercase()).collect();
        let reason = if p.categories.iter().any(|c| c == "harness") {
            seen.programs
                .iter()
                .find(|prog| keywords.contains(&prog.to_lowercase()))
                .map(|prog| {
                    tr_in!(
                        lang,
                        "{prog} est installé sur ce poste",
                        "{prog} is installed on this machine"
                    )
                })
        } else if p.categories.iter().any(|c| c == "ticket-sources") {
            seen.ticket_prefix
                .as_ref()
                .filter(|_| {
                    keywords
                        .iter()
                        .any(|k| KEYED_TRACKERS.contains(&k.as_str()))
                })
                .map(|prefix| {
                    tr_in!(
                        lang,
                        "des clés {prefix}-… dans les commits de ce dépôt",
                        "{prefix}-… keys in this repository's commits"
                    )
                })
        } else {
            None
        };
        if let Some(reason) = reason {
            out.push((
                best.verified,
                p.installs,
                Suggestion {
                    id: p.id.clone(),
                    reason,
                },
            ));
        }
    }
    out.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    out.into_iter().take(max).map(|(_, _, s)| s).collect()
}

// ---- The archive ----

/// Reads a version's archive (a gzip of a tar as GitHub makes it, everything under one folder) in
/// memory: its regular files by their path in the plugin, GitHub's top folder taken off. A link
/// (symbolic or hard), a device, a path that is absolute or climbs with `..`, an archive past its
/// bounds: refused whole, with the reason.
pub fn read_archive(gz: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    if gz.len() as u64 > ARCHIVE_MAX {
        return Err("archive too big".into());
    }
    let reader = Bounded {
        inner: flate2::read::GzDecoder::new(gz),
        left: UNPACKED_MAX,
    };
    let mut ar = tar::Archive::new(reader);
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let entries = ar.entries().map_err(|e| e.to_string())?;
    for (n, entry) in entries.enumerate() {
        if n >= ENTRIES_MAX {
            return Err("too many entries".into());
        }
        let mut entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.header().entry_type();
        let raw = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        use tar::EntryType as T;
        match kind {
            T::XGlobalHeader | T::XHeader | T::GNULongName | T::GNULongLink => continue,
            T::Directory => {
                inner_path(&raw)?;
                continue;
            }
            T::Regular | T::Continuous => {}
            T::Symlink => return Err(format!("symbolic link {raw}")),
            T::Link => return Err(format!("hard link {raw}")),
            _ => return Err(format!("special file {raw}")),
        }
        let path = inner_path(&raw)?;
        if path.is_empty() {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if out.iter().any(|(p, _)| p == &path) {
            return Err(format!("{path} twice"));
        }
        out.push((path, bytes));
    }
    Ok(out)
}

/// A path of the archive inside the plugin: GitHub's top folder taken off; refused when it is
/// absolute or climbs out.
fn inner_path(raw: &str) -> Result<String, String> {
    let p = raw.replace('\\', "/");
    let b = p.as_bytes();
    if p.starts_with('/') || (b.len() >= 2 && b[1] == b':') || p.contains('\0') {
        return Err(format!("absolute path {raw}"));
    }
    let parts: Vec<&str> = p
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    if parts.contains(&"..") {
        return Err(format!("path that climbs out {raw}"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A tar.gz of `entries`: (path, kind, bytes), kind `0` a file, `2` a symbolic link, `1` a
    /// hard link, `5` a folder.
    fn tarball(entries: &[(&str, u8, &[u8])]) -> Vec<u8> {
        let mut tar = Vec::new();
        for (path, kind, bytes) in entries {
            let mut h = [0u8; 512];
            h[..path.len()].copy_from_slice(path.as_bytes());
            h[100..107].copy_from_slice(b"0000644");
            h[108..115].copy_from_slice(b"0000000");
            h[116..123].copy_from_slice(b"0000000");
            let size = format!("{:011o}", bytes.len());
            h[124..135].copy_from_slice(size.as_bytes());
            h[136..147].copy_from_slice(b"00000000000");
            h[156] = *kind;
            if *kind == b'2' || *kind == b'1' {
                h[157..170].copy_from_slice(b"/etc/passwdxx");
            }
            h[257..263].copy_from_slice(b"ustar\0");
            h[263..265].copy_from_slice(b"00");
            h[148..156].copy_from_slice(b"        ");
            let sum: u32 = h.iter().map(|b| *b as u32).sum();
            h[148..155].copy_from_slice(format!("{sum:06o}\0").as_bytes());
            tar.extend_from_slice(&h);
            tar.extend_from_slice(bytes);
            tar.resize(tar.len().div_ceil(512) * 512, 0);
        }
        tar.extend_from_slice(&[0u8; 1024]);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&tar).unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn an_archive_is_read_without_github_s_top_folder() {
        let gz = tarball(&[
            ("owner-repo-abc/", b'5', b""),
            ("owner-repo-abc/escouade-plugin.json", b'0', b"{}"),
            ("owner-repo-abc/bin/run", b'0', b"#!/bin/sh"),
        ]);
        let files = read_archive(&gz).unwrap();
        let names: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(names, ["escouade-plugin.json", "bin/run"]);
        assert_eq!(files[1].1, b"#!/bin/sh");
    }

    #[test]
    fn links_and_paths_out_of_the_folder_refuse_the_whole_archive() {
        for (bad, why) in [
            (("owner-repo-abc/ssh", b'2', &b""[..]), "symbolic link"),
            (("owner-repo-abc/hard", b'1', &b""[..]), "hard link"),
            (
                ("owner-repo-abc/../../escape", b'0', &b"x"[..]),
                "climbs out",
            ),
            (("/etc/evil", b'0', &b"x"[..]), "absolute"),
            (("owner-repo-abc/fifo", b'6', &b""[..]), "special file"),
        ] {
            let gz = tarball(&[("owner-repo-abc/escouade-plugin.json", b'0', b"{}"), bad]);
            let e = read_archive(&gz).unwrap_err();
            assert!(e.contains(why), "{why}: {e}");
        }
        assert!(read_archive(b"not a gzip").is_err());
    }

    #[test]
    fn a_gzip_bomb_stops_at_its_bound() {
        let big = vec![0u8; (UNPACKED_MAX + 1024) as usize];
        let gz = tarball(&[("owner-repo-abc/zeros", b'0', &big)]);
        assert!(gz.len() < 1024 * 1024, "zeros compress");
        let e = read_archive(&gz).unwrap_err();
        assert!(e.contains("200 MB"), "{e}");
    }

    #[test]
    fn the_last_compatible_version_is_a_release_before_a_pre_release() {
        let v = |version: &str, min: &str, yanked: bool| CatalogVersion {
            version: version.into(),
            published_at: String::new(),
            sha256: String::new(),
            archive: ArchiveRef {
                sha256: String::new(),
                size: 0,
            },
            platforms: vec![],
            escouade: serde_json::json!({ "api": "1", "min": min }),
            verified: false,
            review: None,
            yanked,
            unverified: None,
            permissions: Value::Null,
            run: false,
            contributes: vec![],
            icon: None,
            files: Files {
                version_json: String::new(),
                archive: String::new(),
                binaries: BTreeMap::new(),
                icon: None,
                signature: None,
            },
        };
        let p = CatalogPlugin {
            id: "a.b".into(),
            name: Value::Null,
            description: Value::Null,
            author: None,
            repository: String::new(),
            license: None,
            categories: vec![],
            keywords: vec![],
            installs: 0,
            latest: None,
            transferred_at: None,
            versions: vec![
                v("3.0.0", "99.0.0", false),
                v("2.1.0-beta.1", "2.1.0", false),
                v("2.0.0", "2.1.0", true),
                v("1.5.0", "2.1.0", false),
            ],
        };
        assert_eq!(p.best("2.1.0").unwrap().version, "1.5.0");
        assert_eq!(p.best("99.0.0").unwrap().version, "3.0.0");
        let only_pre = CatalogPlugin {
            versions: vec![v("2.1.0-beta.1", "2.1.0", false)],
            ..p
        };
        assert_eq!(only_pre.best("2.1.0").unwrap().version, "2.1.0-beta.1");
    }

    #[test]
    fn ticket_keys_are_found_in_commits_and_versions_are_not() {
        let log = "DEM-12 export
fix DEM-40 et DEM-41
UTF-8 SHA-256 UTF-8 UTF-8 ISO-8601
ABC-1
";
        assert_eq!(ticket_prefix(log).as_deref(), Some("DEM"));
        assert_eq!(ticket_prefix("DEM-1 DEM-2 UTF-8 UTF-8 UTF-8"), None);
        assert_eq!(ticket_prefix(""), None);
    }

    #[test]
    fn a_suggestion_says_what_was_seen_and_never_an_installed_plugin() {
        let cat: Catalog = serde_json::from_value(serde_json::json!({
            "format": 1, "updated_at": "", "plugins": [
                { "id": "x.vibe", "categories": ["harness"], "keywords": ["vibe"], "installs": 5,
                  "versions": [{ "version": "1.0.0", "sha256": "", "archive": { "sha256": "", "size": 0 },
                    "escouade": { "api": "1", "min": "2.1.0" }, "verified": true,
                    "files": { "version_json": "", "archive": "" } }] },
                { "id": "x.jira", "categories": ["ticket-sources"], "keywords": ["jira"], "installs": 9,
                  "versions": [{ "version": "2.0.0", "sha256": "", "archive": { "sha256": "", "size": 0 },
                    "escouade": { "api": 1, "min": "2.1.0" }, "files": { "version_json": "", "archive": "" } }] },
                { "id": "x.nord", "categories": ["themes"], "keywords": ["vibe"], "installs": 99,
                  "versions": [{ "version": "1.0.0", "sha256": "", "archive": { "sha256": "", "size": 0 },
                    "escouade": { "api": "1", "min": "2.1.0" }, "files": { "version_json": "", "archive": "" } }] }
            ]
        }))
        .unwrap();
        let seen = Seen {
            programs: vec!["vibe".into()],
            ticket_prefix: Some("DEM".into()),
        };
        let got = suggest(&cat, "2.1.0", &seen, &[], Lang::Fr, 6);
        assert_eq!(
            got,
            vec![
                Suggestion {
                    id: "x.vibe".into(),
                    reason: "vibe est installé sur ce poste".into()
                },
                Suggestion {
                    id: "x.jira".into(),
                    reason: "des clés DEM-… dans les commits de ce dépôt".into()
                },
            ]
        );
        let got = suggest(&cat, "2.1.0", &seen, &["x.vibe".into()], Lang::En, 6);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].reason, "DEM-… keys in this repository's commits");
        assert!(suggest(&cat, "2.1.0", &Seen::default(), &[], Lang::Fr, 6).is_empty());
    }
}
