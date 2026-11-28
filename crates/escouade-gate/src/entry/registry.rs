//! The connectors of a server (§ 4.5, § 4.7, § 4.8, R12): those paired, the codes that wait for one, the channels open,
//! and what the gate asks of them ([`Connectors`]).
//!
//! - **A code** ([`Registry::create_code`]): 8 letters of an alphabet of 32 (40 bits, no 0, O, 1 nor I), shown
//!   `ABCD-EFGH`, good once for [`CODE_LIFETIME_MS`] (10 minutes), for the connector's name to be. Only its pre-shared
//!   key is kept (`pairing::CodeKey`), in the secrets, never the code; a new code for the same name takes the place of
//!   the one before. A wrong code never ends it (the entry's bounds hold the guesses, `bounds`).
//! - **Pinned** ([`Registry::pin`]): the connector's public key in the secrets, its name and dates in the list; the code
//!   used, gone.
//! - **Removed** ([`Registry::remove`]): its key forgotten (it no longer gets in), its channel closed (by the server
//!   that holds it, at once, or within two seconds when another process removed it: [`Registry::sync`]), a trace kept
//!   (its name, when, by whom, its key's SHA-256).
//! - **Its state**, kept as the channels come and go: connected, cut since, its last connection, its version (« to
//!   update » two minors behind, « too old » beyond, refused), its ceiling as it last sent it.
//!
//! Neither a code nor a private key goes to the log.

use super::store::{
    key_secret, Paired, Pending, Removed, Saved, Store, MAX_REMOVED, PENDING, SERVER_KEY,
};
use crate::connectors::{Ask, Connectors, Failed, Known, Opening, Reach};
use crate::lists::Pattern;
use crate::reason::Lang;
use escouade_connector_proto::pairing::CodeKey;
use escouade_connector_proto::version::{judge, Hello, Welcome};
use escouade_connector_proto::{Keypair, OpenError, PublicKey, ServerChannel, Verdict, Version};
use parking_lot::Mutex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// How long a code waits for its connector.
pub const CODE_LIFETIME_MS: u64 = 10 * 60_000;

/// The letters of a code: no 0, O, 1 nor I, which read alike. 32 of them: a random byte gives one, evenly.
const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// The letters of a code (40 bits), shown in two groups of four.
pub const CODE_LENGTH: usize = 8;

/// The longest name of a connector.
pub const MAX_NAME: usize = 64;

/// Where a removal comes from when it is the command on the server.
pub const BY_COMMAND: &str = "escouade-server";

fn say(lang: Lang, fr: &str, en: &str) -> String {
    match lang {
        Lang::Fr => fr.to_string(),
        Lang::En => en.to_string(),
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// A new code, as `connector pair` shows it.
#[derive(Debug, Clone)]
pub struct NewCode {
    /// `ABCD-EFGH`: for the user alone, never in the log.
    pub text: String,
    /// When it ends (ms since 1970).
    pub expires_at: u64,
}

/// A connector's channel, as the space shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Connected,
    /// Cut (or the server started since and it did not come back yet).
    Down,
    /// Paired, never connected yet.
    Never,
}

/// What the server says of a connector's version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VersionState {
    Current,
    /// Two minors behind: accepted, « to update ».
    Outdated,
    /// Further behind: refused, its channel too.
    TooOld,
    /// Newer than the server: refused, the server is the one to update.
    TooNew,
}

impl From<Verdict> for VersionState {
    fn from(v: Verdict) -> Self {
        match v {
            Verdict::Current => Self::Current,
            Verdict::Outdated => Self::Outdated,
            Verdict::TooOld => Self::TooOld,
            Verdict::TooNew => Self::TooNew,
        }
    }
}

/// A connector as the space shows it (Réglages de l'espace › Réseau privé, RP-5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub id: String,
    pub name: String,
    pub paired_at: u64,
    pub state: State,
    /// Connected since, or cut since (ms since 1970).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<u64>,
    /// Its version, as it last said it (a refused one included).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_state: Option<VersionState>,
    /// What to do about its version (in English, as the connector's journal says it), when it is not current.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advice: Option<String>,
    /// Its ceiling as it last sent it: its rules as it writes them, its ranges of addresses included.
    pub ceiling: Vec<String>,
}

struct Live {
    generation: u64,
    channel: Arc<ServerChannel>,
    since: u64,
    verdict: Verdict,
}

/// The connectors of the server.
pub struct Registry {
    store: Arc<dyn Store>,
    lang: Lang,
    version: Version,
    live: Mutex<HashMap<String, Live>>,
    /// When each channel was cut, in this process.
    down: Mutex<HashMap<String, u64>>,
    generation: AtomicU64,
}

fn key_hash(key_hex: &str) -> String {
    hex::encode(Sha256::digest(
        format!("escouade-connector:{key_hex}").as_bytes(),
    ))
}

fn new_id() -> String {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("the system's random generator");
    format!("c{}", hex::encode(bytes))
}

/// A new code's letters.
fn new_code() -> String {
    let mut bytes = [0u8; CODE_LENGTH];
    getrandom::fill(&mut bytes).expect("the system's random generator");
    let letters: String = bytes
        .iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect();
    format!("{}-{}", &letters[..4], &letters[4..])
}

fn read_pending(text: Option<String>) -> Vec<Pending> {
    text.and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write_pending(list: &[Pending]) -> Option<String> {
    if list.is_empty() {
        None
    } else {
        serde_json::to_string(list).ok()
    }
}

impl Registry {
    /// The connectors of `store`, for a server of `version`, its sentences in `lang`.
    pub fn new(store: Arc<dyn Store>, lang: Lang, version: Version) -> Arc<Self> {
        Arc::new(Self {
            store,
            lang,
            version,
            live: Mutex::default(),
            down: Mutex::default(),
            generation: AtomicU64::new(1),
        })
    }

    pub fn version(&self) -> Version {
        self.version
    }

    fn say(&self, fr: &str, en: &str) -> String {
        say(self.lang, fr, en)
    }

    /// The server's static key: made the first time, kept in the secrets.
    pub fn server_keys(&self) -> Result<Keypair, String> {
        let mut made = false;
        self.store.change_secret(SERVER_KEY, &mut |now| match now {
            Some(k) => Some(k),
            None => {
                let k = Keypair::generate();
                let text = hex::encode(k.private_bytes());
                made = true;
                Some(text)
            }
        })?;
        if made {
            log::info!("connectors: the server's key made (kept in the secrets)");
        }
        let text = self
            .store
            .secret(SERVER_KEY)?
            .ok_or_else(|| "the server's key is not kept".to_string())?;
        let bytes: [u8; 32] = hex::decode(text.trim())
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| format!("{SERVER_KEY}: not a key"))?;
        Ok(Keypair::from_private(bytes))
    }

    /// The list as it is kept.
    pub fn saved(&self) -> Result<Saved, String> {
        self.store.read()
    }

    /// The entry's address, as the server that runs serves it.
    pub fn entry_url(&self) -> Option<String> {
        self.store.read().ok().and_then(|s| s.entry)
    }

    /// The entry's address written, for `connector pair` beside the server (`None`: no entry).
    pub fn set_entry_url(&self, url: Option<String>) -> Result<(), String> {
        self.store.change(&mut |s| {
            if s.entry == url {
                return false;
            }
            s.entry = url.clone();
            true
        })
    }

    fn check_name(&self, name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME || name.chars().any(char::is_control)
        {
            return Err(self.say(
                &format!("le nom d’un connecteur : 1 à {MAX_NAME} caractères, sans caractère de contrôle"),
                &format!("a connector's name: 1 to {MAX_NAME} characters, no control character"),
            ));
        }
        Ok(name.to_string())
    }

    fn named<'a>(saved: &'a Saved, name: &str) -> Option<&'a Paired> {
        saved
            .connectors
            .iter()
            .find(|c| c.name.to_lowercase() == name.to_lowercase())
    }

    /// A new code for a connector to be named `name`, good once for 10 minutes: the code of before for that name goes.
    /// Refused when a connector of that name is paired already.
    pub fn create_code(&self, name: &str, now: u64) -> Result<NewCode, String> {
        let name = self.check_name(name)?;
        let saved = self.store.read()?;
        if let Some(c) = Self::named(&saved, &name) {
            return Err(self.say(
                &format!("un connecteur « {} » est déjà appairé : retire-le d’abord (escouade-server connectors remove {})", c.name, c.name),
                &format!("a connector “{}” is paired already: remove it first (escouade-server connectors remove {})", c.name, c.name),
            ));
        }
        let text = new_code();
        let key = CodeKey::from_code(&text);
        let expires_at = now + CODE_LIFETIME_MS;
        let entry = Pending {
            name: name.clone(),
            key: hex::encode(key.as_bytes()),
            expires_at,
        };
        self.store.change_secret(PENDING, &mut |was| {
            let mut list = read_pending(was);
            list.retain(|p| p.expires_at > now && p.name.to_lowercase() != name.to_lowercase());
            list.push(entry.clone());
            write_pending(&list)
        })?;
        log::info!("connectors: a pairing code made for “{name}” (10 minutes, once)");
        Ok(NewCode { text, expires_at })
    }

    /// The codes that wait, their end not come.
    pub fn pending(&self, now: u64) -> Vec<Pending> {
        let list = read_pending(self.store.secret(PENDING).ok().flatten());
        list.into_iter().filter(|p| p.expires_at > now).collect()
    }

    /// The connector of the code `pending` pinned with `key`: its id. Refused (said to the connector) when the code
    /// ended or was used since, when its name was taken since, or when that key is a connector's already.
    pub fn pin(
        &self,
        pending: &Pending,
        key: &PublicKey,
        version: Version,
        now: u64,
    ) -> Result<String, String> {
        let still = self
            .pending(now)
            .into_iter()
            .any(|p| p.key == pending.key && p.name == pending.name);
        if !still {
            return Err("the pairing code ended or was used".into());
        }
        let saved = self.store.read()?;
        if Self::named(&saved, &pending.name).is_some() {
            return Err(format!("a connector “{}” is paired already", pending.name));
        }
        if self.find(key).is_some() {
            return Err("this connector's key is paired already".into());
        }
        let id = new_id();
        let key_hex = key.to_hex();
        self.store
            .change_secret(&key_secret(&id), &mut |_| Some(key_hex.clone()))?;
        let paired = Paired {
            id: id.clone(),
            name: pending.name.clone(),
            paired_at: now,
            last_seen: None,
            connected_since: None,
            version: Some(version.to_string()),
            refused_version: None,
            ceiling: Vec::new(),
        };
        self.store.change(&mut |s| {
            s.connectors.push(paired.clone());
            true
        })?;
        self.store.change_secret(PENDING, &mut |was| {
            let mut list = read_pending(was);
            list.retain(|p| p.key != pending.key && p.expires_at > now);
            write_pending(&list)
        })?;
        log::info!(
            "connectors: “{}” paired ({id}, escouade-connector {version})",
            pending.name
        );
        Ok(id)
    }

    /// The connector whose key is `key`.
    pub fn find(&self, key: &PublicKey) -> Option<Paired> {
        let saved = self.store.read().ok()?;
        let hex = key.to_hex();
        saved.connectors.into_iter().find(|c| {
            self.store
                .secret(&key_secret(&c.id))
                .ok()
                .flatten()
                .is_some_and(|k| k.trim() == hex)
        })
    }

    /// The connector named or of id `which` removed by `by`: its key forgotten, its trace kept, its channel closed.
    /// `None` when there is none.
    pub fn remove(&self, which: &str, by: &str, now: u64) -> Result<Option<Removed>, String> {
        let which = which.trim();
        let saved = self.store.read()?;
        let Some(c) = saved
            .connectors
            .iter()
            .find(|c| c.id == which)
            .or_else(|| Self::named(&saved, which))
            .cloned()
        else {
            return Ok(None);
        };
        let key = self.store.secret(&key_secret(&c.id))?.unwrap_or_default();
        self.store
            .change_secret(&key_secret(&c.id), &mut |_| None)?;
        let trace = Removed {
            id: c.id.clone(),
            name: c.name.clone(),
            at: now,
            by: by.to_string(),
            key_hash: key_hash(key.trim()),
        };
        self.store.change(&mut |s| {
            s.connectors.retain(|x| x.id != c.id);
            s.removed.push(trace.clone());
            let extra = s.removed.len().saturating_sub(MAX_REMOVED);
            s.removed.drain(..extra);
            true
        })?;
        self.close(&c.id);
        self.down.lock().remove(&c.id);
        log::info!("connectors: “{}” removed ({}) by {by}", c.name, c.id);
        Ok(Some(trace))
    }

    /// The traces of the connectors removed, the most recent last.
    pub fn removed(&self) -> Vec<Removed> {
        self.store.read().map(|s| s.removed).unwrap_or_default()
    }

    /// The channel of `id` closed, when this process holds it.
    fn close(&self, id: &str) {
        if let Some(l) = self.live.lock().remove(id) {
            l.channel.close();
        }
    }

    /// Every channel closed (the entry stopped).
    pub fn close_all(&self) {
        let now = now_ms();
        let live: Vec<(String, Live)> = self.live.lock().drain().collect();
        for (id, l) in live {
            l.channel.close();
            self.down.lock().insert(id.clone(), now);
            self.save(&id, |c| {
                c.connected_since = None;
                c.last_seen = Some(now);
            });
        }
    }

    /// No channel held: what a server that stopped without saying so left behind forgotten (its start).
    pub fn reset_connected(&self) {
        let done = self.store.change(&mut |s| {
            let mut changed = false;
            for c in s
                .connectors
                .iter_mut()
                .filter(|c| c.connected_since.is_some())
            {
                c.connected_since = None;
                changed = true;
            }
            changed
        });
        if let Err(e) = done {
            log::warn!("connectors: {} not written: {e}", super::store::FILE);
        }
    }

    /// The channels of the connectors removed by another process (`escouade-server connectors remove`) closed.
    pub fn sync(&self) {
        let Ok(saved) = self.store.read() else { return };
        let gone: Vec<String> = self
            .live
            .lock()
            .keys()
            .filter(|id| !saved.connectors.iter().any(|c| &c.id == *id))
            .cloned()
            .collect();
        for id in gone {
            log::info!("connectors: {id} removed: its channel closed");
            self.close(&id);
            self.down.lock().remove(&id);
        }
    }

    fn save(&self, id: &str, change: impl Fn(&mut Paired)) {
        let done = self.store.change(&mut |s| {
            let Some(c) = s.connectors.iter_mut().find(|c| c.id == id) else {
                return false;
            };
            let before = c.clone();
            change(c);
            *c != before
        });
        if let Err(e) = done {
            log::warn!("connectors: {} not written: {e}", super::store::FILE);
        }
    }

    /// The channel of `id` open: it takes the place of one of before. Its generation, and the channel shared.
    pub fn attach(&self, id: &str, channel: ServerChannel, now: u64) -> (u64, Arc<ServerChannel>) {
        let generation = self.generation.fetch_add(1, Ordering::Relaxed);
        let verdict = channel.verdict();
        let version = channel.peer_version().to_string();
        let ceiling = channel.ceiling().map(|c| c.rules);
        let channel = Arc::new(channel);
        let before = self.live.lock().insert(
            id.to_string(),
            Live {
                generation,
                channel: channel.clone(),
                since: now,
                verdict,
            },
        );
        if let Some(b) = before {
            b.channel.close();
        }
        self.down.lock().remove(id);
        self.save(id, |c| {
            c.last_seen = Some(now);
            c.connected_since = Some(now);
            c.version = Some(version.clone());
            c.refused_version = None;
            if let Some(rules) = &ceiling {
                c.ceiling = rules.clone();
            }
        });
        (generation, channel)
    }

    /// The ceiling `rules` the connector `id` sent on its channel of `generation`.
    pub fn ceiling_received(&self, id: &str, generation: u64, rules: Vec<String>) {
        if self.live.lock().get(id).map(|l| l.generation) != Some(generation) {
            return;
        }
        log::info!("connectors: {id} sent its ceiling ({} rules)", rules.len());
        self.save(id, |c| c.ceiling = rules.clone());
    }

    /// The channel of `generation` of `id` ended.
    pub fn detach(&self, id: &str, generation: u64, now: u64) {
        let mut live = self.live.lock();
        if live.get(id).map(|l| l.generation) != Some(generation) {
            return;
        }
        live.remove(id);
        drop(live);
        self.down.lock().insert(id.to_string(), now);
        self.save(id, |c| {
            c.last_seen = Some(now);
            c.connected_since = None;
        });
    }

    /// A version of `id` refused at its connection.
    pub fn refused_version(&self, id: &str, version: Version, now: u64) {
        self.save(id, |c| {
            c.refused_version = Some(version.to_string());
            c.last_seen = Some(now);
        });
    }

    /// Whether `id` has a channel open in this process.
    pub fn is_connected(&self, id: &str) -> bool {
        self.live
            .lock()
            .get(id)
            .is_some_and(|l| !l.channel.is_closed())
    }

    fn version_of(&self, text: &str) -> Option<(VersionState, Option<String>)> {
        let v = Version::parse(text)?;
        let welcome = Welcome::for_hello(self.version, &Hello { version: v });
        Some((judge(self.version, v).into(), welcome.advice(v)))
    }

    /// Every connector paired, as the space shows it.
    pub fn views(&self) -> Vec<View> {
        let saved = self.store.read().unwrap_or_default();
        let live = self.live.lock();
        let down = self.down.lock();
        saved
            .connectors
            .into_iter()
            .map(|c| {
                let open = live.get(&c.id).filter(|l| !l.channel.is_closed());
                let (state, since) = match open {
                    Some(l) => (State::Connected, Some(l.since)),
                    // Held by the server beside this process (the command `connectors`).
                    None if c.connected_since.is_some()
                        && !live.contains_key(&c.id)
                        && down.get(&c.id).is_none() =>
                    {
                        (State::Connected, c.connected_since)
                    }
                    None if c.last_seen.is_none() => (State::Never, None),
                    None => (State::Down, down.get(&c.id).copied().or(c.last_seen)),
                };
                let shown = c.refused_version.clone().or(c.version.clone());
                let (version_state, advice) = match (open, &c.refused_version) {
                    (Some(l), _) => {
                        let v = l.channel.peer_version();
                        let welcome = Welcome::for_hello(self.version, &Hello { version: v });
                        (Some(l.verdict.into()), welcome.advice(v))
                    }
                    _ => match shown.as_deref().and_then(|t| self.version_of(t)) {
                        Some((s, a)) => (Some(s), a),
                        None => (None, None),
                    },
                };
                View {
                    id: c.id,
                    name: c.name,
                    paired_at: c.paired_at,
                    state,
                    since,
                    last_seen: c.last_seen,
                    version: shown,
                    version_state,
                    advice,
                    ceiling: c.ceiling,
                }
            })
            .collect()
    }
}

impl Connectors for Registry {
    fn known(&self) -> Vec<Known> {
        let saved = self.store.read().unwrap_or_default();
        let live = self.live.lock();
        let down = self.down.lock();
        saved
            .connectors
            .into_iter()
            .map(|c| {
                let up = live.get(&c.id).is_some_and(|l| !l.channel.is_closed());
                let reach = if up {
                    Reach::Up
                } else {
                    Reach::Down {
                        since_ms: down
                            .get(&c.id)
                            .copied()
                            .or(c.last_seen)
                            .unwrap_or(c.paired_at),
                    }
                };
                Known {
                    // Its ranges of addresses are the connector's alone (after its resolution): no pattern here.
                    ceiling: c.ceiling.iter().filter_map(|r| Pattern::parse(r)).collect(),
                    id: c.id,
                    name: c.name,
                    reach,
                }
            })
            .collect()
    }

    fn open(&self, id: &str, ask: Ask) -> Opening {
        let channel = self
            .live
            .lock()
            .get(id)
            .filter(|l| !l.channel.is_closed())
            .map(|l| l.channel.clone());
        let since = self.down.lock().get(id).copied().unwrap_or_else(now_ms);
        Box::pin(async move {
            let Some(channel) = channel else {
                return Err(Failed::Down { since_ms: since });
            };
            match channel
                .open(&ask.host, ask.port, &ask.agent, &ask.project)
                .await
            {
                Ok(stream) => Ok(Box::new(stream) as crate::connectors::Flow),
                Err(OpenError::Refused(r)) => Err(Failed::Refused(r.reason)),
                Err(OpenError::Closed) => Err(Failed::Down { since_ms: now_ms() }),
                Err(e @ (OpenError::Limits | OpenError::Invalid)) => {
                    Err(Failed::Broken(e.to_string()))
                }
            }
        })
    }
}
