//! The team's private store of plugins (TS-14, `docs/TEAM-SERVER.md` §7): the team's own plugins, published on its
//! server, signed by the team's key, installable by its members alone. The server is blind (C10-A): it keeps encrypted
//! files and sealed entries, and checks nothing of them.
//!
//! - **Where it lives**: a stream of the server of its own ([`PLUGINS_STREAM`], a project id no client draws), as the
//!   team's list of projects (TS-8). Each entry is sealed under the team's key of its epoch and signed by the leaf of its
//!   author:
//!   - [`Entry::Key`]: the team's signing key (its key of every day and its backup key, minisign public keys,
//!     [`crate::signing`]) and **its namespace** (`fr.acme.*`), written by an administrator. It is the declaration to
//!     the team's journal (§3.7): every device refuses one sealed by a member who was not an administrator, or whose
//!     namespace is not one ([`namespace`]: never `dev.escouade.*`, nor a namespace that holds it).
//!   - [`Entry::Submit`]: a version proposed by a member, its checks passed on their machine (`escouade-plugin`'s,
//!     `escouade_plugin_cli::team`): its archive and its module are encrypted files of the server, its digest the one
//!     the administrator who signs must find again.
//!   - [`Entry::Publish`]: an administrator played the checks again on their machine, found the same digest and signed
//!     the version's statement ([`Statement`]) with the team's key. Every device checks that signature against the key
//!     declared, the id inside the namespace, never `dev.escouade.*`.
//!   - [`Entry::Revocations`]: the team's revocation list, in the format of the store's `revocations.json`, signed by
//!     the team's key, its `serial` never going back. The plugins' kill switch reads it (§9).
//!   - [`Entry::Dismiss`]: a proposal set aside, by an administrator or its author.
//! - **A lying server**: as the list of projects, the device reads again the last entry it read before reading on;
//!   gone, it is a rollback; changed, a rewritten entry: the reading stops and says it.
//! - **Installing stays the member's gesture**: nothing here installs; the core offers what is published, after the
//!   window of its permissions (§6.5).

use std::collections::{BTreeMap, BTreeSet};

use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::{MemberId, ObjectId, ProjectId, Role};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use crate::settings::Alarm;
use crate::signing;
use crate::team::Team;
use crate::{wire, CryptoError, Error};

/// The stream of the team's private store: a project id no client draws (16 bytes).
pub const PLUGINS_STREAM: ProjectId = ProjectId(*b"escouade/plugins");

/// Where this device keeps what it read of the stream, in the team's encrypted base.
const STATE: &str = "plugins";

/// What the trusted comment of a version's signature starts with: never the public store's (`escouade-verified-v1`).
pub const STATEMENT_COMMENT: &str = "escouade-team-v1";
/// What the trusted comment of the team's revocation list starts with, followed by its serial: the store's own, so
/// that the app reads it as the store's list.
pub const REVOCATIONS_COMMENT: &str = "escouade-revocations-v1";

/// The ids Escouade keeps for its own plugins: never a team's.
pub const FIRST_PARTY: &str = "dev.escouade";

// === The namespace ===

/// Why a namespace is refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum BadNamespace {
    /// Not `a.b.*` (lower case labels, two at least, then `.*`).
    Form,
    /// `dev.escouade.*`, or a namespace that holds it (`dev.*`).
    FirstParty,
}

impl std::fmt::Display for BadNamespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BadNamespace::Form => f.write_str(
                "a namespace is a reversed domain of two labels at least, then .* (fr.acme.*)",
            ),
            BadNamespace::FirstParty => {
                f.write_str("dev.escouade.* is Escouade's own: never a team's namespace")
            }
        }
    }
}

/// The namespace `raw` (`fr.acme.*`), as written to the team: its prefix (`fr.acme`), or why not.
pub fn namespace(raw: &str) -> Result<String, BadNamespace> {
    let raw = raw.trim().to_ascii_lowercase();
    let prefix = raw.strip_suffix(".*").ok_or(BadNamespace::Form)?;
    let labels: Vec<&str> = prefix.split('.').collect();
    let label = |l: &&str| {
        !l.is_empty()
            && l.len() <= 63
            && l.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !l.starts_with('-')
            && !l.ends_with('-')
    };
    if !labels.iter().all(label) {
        return Err(BadNamespace::Form);
    }
    if within(prefix, FIRST_PARTY) || within(FIRST_PARTY, prefix) {
        return Err(BadNamespace::FirstParty);
    }
    if labels.len() < 2 {
        return Err(BadNamespace::Form);
    }
    Ok(prefix.to_owned())
}

/// Whether the id `id` is under the prefix `prefix` (`fr.acme.rules` under `fr.acme`; `fr.acme` itself too).
pub fn within(id: &str, prefix: &str) -> bool {
    let id = id.trim().to_ascii_lowercase();
    id == prefix
        || id
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('.'))
}

/// Whether the team's signature may stand for the plugin `id` under the namespace `prefix`: inside it, never
/// `dev.escouade.*`. The hard refusal of the app (§7).
pub fn may_sign(prefix: &str, id: &str) -> bool {
    within(id, prefix) && !within(id, FIRST_PARTY) && namespace(&format!("{prefix}.*")).is_ok()
}

// === The entries ===

/// One check of a version, as the machine that played it said it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checked {
    pub key: String,
    pub ok: bool,
    pub detail: String,
}

/// A version proposed for the team's signature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Submission {
    pub plugin: String,
    pub version: String,
    pub tag: String,
    /// Its name and its description, as its manifest writes them (a text, or one by language).
    #[serde(default)]
    pub name: serde_json::Value,
    #[serde(default)]
    pub description: serde_json::Value,
    /// The encrypted files of the server: the archive, the module (none for a declarative plugin).
    pub archive: [u8; 16],
    pub module: Option<[u8; 16]>,
    pub archive_sha256: String,
    pub module_sha256: Option<String>,
    pub digest: String,
    /// The checks as its author's machine played them.
    pub checks: Vec<Checked>,
}

/// What the team's key signs of a version: these bytes, as JSON, with the trusted comment
/// `escouade-team-v1 <id>@<version> <namespace>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statement {
    pub format: u32,
    pub id: String,
    pub version: String,
    pub tag: String,
    /// Its name and its description, as its manifest writes them.
    #[serde(default)]
    pub name: serde_json::Value,
    #[serde(default)]
    pub description: serde_json::Value,
    /// The namespace's prefix the key was declared with (`fr.acme`).
    pub namespace: String,
    /// The team's fingerprint: the statement of another team never stands here.
    pub team: String,
    pub archive_sha256: String,
    pub module_sha256: Option<String>,
    pub digest: String,
    /// When it was signed (seconds), and by which administrator (member id, hex).
    pub signed_at: u64,
    pub by: String,
}

impl Statement {
    /// The trusted comment of its signature.
    pub fn comment(&self) -> String {
        format!(
            "{STATEMENT_COMMENT} {}@{} {}",
            self.id, self.version, self.namespace
        )
    }

    pub fn bytes(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(self).map_err(|e| Error::Encoding(e.to_string()))
    }
}

/// An entry of the stream, sealed and signed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Entry {
    /// The team's signing key and its namespace (an administrator).
    Key {
        /// The key of every day, then the backup key (minisign public keys).
        keys: Vec<String>,
        /// `fr.acme.*`.
        namespace: String,
        at: u64,
    },
    /// A version proposed (a member).
    Submit {
        id: [u8; 16],
        submission: Submission,
        at: u64,
    },
    /// A version signed by the team's key (an administrator).
    Publish {
        /// The proposal it signs.
        submission: [u8; 16],
        #[serde(with = "serde_bytes")]
        statement: Vec<u8>,
        minisig: String,
        archive: [u8; 16],
        module: Option<[u8; 16]>,
        at: u64,
    },
    /// The team's revocation list (an administrator), signed by the team's key.
    Revocations {
        #[serde(with = "serde_bytes")]
        json: Vec<u8>,
        minisig: String,
        at: u64,
    },
    /// A proposal set aside (an administrator, or its author).
    Dismiss {
        submission: [u8; 16],
        why: String,
        at: u64,
    },
}

/// Why an entry is refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Refused {
    /// Signed by no leaf of the team (unsigned, or forged by the server).
    Unsigned,
    /// What an administrator alone writes, sealed by a member who was not one.
    NotAdmin,
    /// A namespace that is not one, or Escouade's own.
    Namespace { why: BadNamespace },
    /// A version signed before the team declared its key.
    NoKey,
    /// A signature the team's key did not make, or for another version, team or namespace.
    Signature,
    /// A plugin outside the team's namespace, or `dev.escouade.*`.
    OutsideNamespace { id: String },
    /// A revocation list whose serial goes back (or that changes under the same serial).
    Older { serial: u64, seen: u64 },
    /// An entry that does not decode, or a proposal that does not hold.
    Unreadable { why: String },
}

/// An entry refused at the reading.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Refusal {
    pub seq: u64,
    pub reason: Refused,
}

/// The team's key as declared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyView {
    pub keys: Vec<String>,
    /// Their ids, as minisign prints them.
    pub ids: Vec<String>,
    /// `fr.acme.*`, and its prefix.
    pub namespace: String,
    pub prefix: String,
    /// The administrator who declared it (member id, hex), when, and the number of its entry.
    pub by: String,
    pub at: u64,
    pub seq: u64,
}

/// A proposal waiting for an administrator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposed {
    /// Its id (hex).
    pub id: String,
    pub submission: Submission,
    pub by: String,
    pub at: u64,
    pub seq: u64,
}

/// A version the team published.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Published {
    pub statement: Statement,
    #[serde(with = "serde_bytes")]
    pub statement_bytes: Vec<u8>,
    pub minisig: String,
    pub archive: [u8; 16],
    pub module: Option<[u8; 16]>,
    /// The id of the key that signed.
    pub signer: String,
    pub seq: u64,
}

/// The team's revocation list as this device took it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Revocations {
    pub serial: u64,
    #[serde(with = "serde_bytes")]
    pub json: Vec<u8>,
    pub minisig: String,
    pub seq: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    cursor: u64,
    last: Option<(u64, [u8; 32])>,
    key: Option<KeyView>,
    proposed: BTreeMap<String, Proposed>,
    published: Vec<Published>,
    revocations: Option<Revocations>,
    refusals: Vec<Refusal>,
    alarm: Option<Alarm>,
}

/// What this device knows of the team's store.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Store {
    pub key: Option<KeyView>,
    /// The proposals waiting, oldest first.
    pub proposed: Vec<Proposed>,
    /// The versions published, in the order of the stream.
    pub published: Vec<Published>,
    pub revocations: Option<Revocations>,
    pub refusals: Vec<Refusal>,
    pub alarm: Option<Alarm>,
}

impl Store {
    /// The last version published of each plugin, by id.
    pub fn latest(&self) -> BTreeMap<String, &Published> {
        let mut out: BTreeMap<String, &Published> = BTreeMap::new();
        for p in &self.published {
            let newer = out.get(&p.statement.id).is_none_or(|q| {
                version_key(&p.statement.version) > version_key(&q.statement.version)
            });
            if newer {
                out.insert(p.statement.id.clone(), p);
            }
        }
        out
    }

    /// A version published.
    pub fn version(&self, id: &str, version: &str) -> Option<&Published> {
        self.published
            .iter()
            .rev()
            .find(|p| p.statement.id == id && p.statement.version == version)
    }

    /// The trusted keys: the keys declared, less those the revocation list revokes.
    pub fn trusted_keys(&self) -> Vec<String> {
        let Some(k) = &self.key else {
            return Vec::new();
        };
        let revoked = self
            .revocations
            .as_ref()
            .map(|r| keys_revoked(&r.json))
            .unwrap_or_default();
        k.keys
            .iter()
            .filter(|key| signing::key_id(key).is_none_or(|id| !revoked.contains(&id)))
            .cloned()
            .collect()
    }
}

fn version_key(v: &str) -> Vec<u64> {
    v.split(['.', '-', '+'])
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

fn keys_revoked(json: &[u8]) -> BTreeSet<String> {
    #[derive(Deserialize)]
    struct Head {
        #[serde(default)]
        keys_revoked: Option<Vec<String>>,
    }
    serde_json::from_slice::<Head>(json)
        .ok()
        .and_then(|h| h.keys_revoked)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn serial_of(json: &[u8]) -> Option<u64> {
    #[derive(Deserialize)]
    struct Head {
        serial: u64,
        entries: Vec<serde_json::Value>,
    }
    serde_json::from_slice::<Head>(json)
        .ok()
        .filter(|h| h.serial >= 1 && h.entries.iter().all(|e| e.get("id").is_some()))
        .map(|h| h.serial)
}

/// The serial of the revocation list `json` when one of the trusted `keys` signed it with the store's comment for that
/// serial, and not a key the list itself revokes.
pub fn revocations_signed(keys: &[String], json: &[u8], minisig: &str) -> Option<u64> {
    let serial = serial_of(json)?;
    let (signer, comment) = signing::signed_by(keys, json, minisig)?;
    if keys_revoked(json).contains(&signer) {
        return None;
    }
    let mut words = comment.split_whitespace();
    match (words.next(), words.next()) {
        (Some(REVOCATIONS_COMMENT), Some(s)) if s == serial.to_string() => Some(serial),
        _ => None,
    }
}

/// Whether `minisig` is the team's signature of the statement `bytes`, by one of `keys`, for this team and namespace:
/// the statement read, and the id of the key that signed.
pub fn statement_signed(
    keys: &[String],
    bytes: &[u8],
    minisig: &str,
    prefix: &str,
    team: &str,
) -> Result<(Statement, String), Refused> {
    let statement: Statement =
        serde_json::from_slice(bytes).map_err(|e| Refused::Unreadable { why: e.to_string() })?;
    if !may_sign(prefix, &statement.id) {
        return Err(Refused::OutsideNamespace {
            id: statement.id.clone(),
        });
    }
    let (signer, comment) = signing::signed_by(keys, bytes, minisig).ok_or(Refused::Signature)?;
    if comment != statement.comment()
        || statement.namespace != prefix
        || statement.team != team
        || statement.format != 1
    {
        return Err(Refused::Signature);
    }
    Ok((statement, signer))
}

fn store_of(state: &State) -> Store {
    let mut proposed: Vec<Proposed> = state.proposed.values().cloned().collect();
    proposed.sort_by_key(|p| p.seq);
    Store {
        key: state.key.clone(),
        proposed,
        published: state.published.clone(),
        revocations: state.revocations.clone(),
        refusals: state.refusals.clone(),
        alarm: state.alarm,
    }
}

enum Opened {
    Refused(Refused),
    Fork,
    Later,
}

fn entry_digest(e: &wire::Entry) -> [u8; 32] {
    Sha256::digest(wire::encode(e)).into()
}

fn admins_now(team: &Team) -> BTreeSet<MemberId> {
    team.roster()
        .map(|r| {
            r.members
                .iter()
                .filter(|(_, role)| **role == Role::Admin)
                .map(|(m, _)| *m)
                .collect()
        })
        .unwrap_or_default()
}

/// The team's fingerprint, as a statement names it.
fn team_mark(team: &Team) -> String {
    team.fingerprint()
        .map(|f| hex::encode(f.0))
        .unwrap_or_default()
}

/// Opens an entry: its seal, its signer, whether it was an administrator at its epoch, and what it says.
fn open(team: &Team, e: &wire::Entry) -> Result<(MemberId, bool, Entry), Opened> {
    let unreadable = |why: String| Opened::Refused(Refused::Unreadable { why });
    let sealed: SealedObject = wire::decode(&e.payload).map_err(unreadable)?;
    if sealed.header.project != PLUGINS_STREAM {
        return Err(unreadable("an object of another stream".into()));
    }
    let plain = match team.device().open(&sealed) {
        Ok(p) => p,
        Err(CryptoError::Fork { .. }) => return Err(Opened::Fork),
        Err(CryptoError::Behind(_)) => return Err(Opened::Later),
        Err(_) => return Err(Opened::Refused(Refused::Unsigned)),
    };
    let who = team
        .signer_at(sealed.header.epoch, &sealed.header.author)
        .ok_or(Opened::Refused(Refused::Unsigned))?;
    let admin = match team.admins_at(sealed.header.epoch) {
        Some(then) => then.contains(&who.member),
        None => admins_now(team).contains(&who.member),
    };
    let entry: Entry = wire::decode(&plain).map_err(unreadable)?;
    Ok((who.member, admin, entry))
}

/// What an entry does to the state, or why it is refused.
fn apply(
    team: &Team,
    state: &mut State,
    seq: u64,
    by: MemberId,
    admin: bool,
    entry: Entry,
) -> Result<(), Refused> {
    let admin_only = || {
        if admin {
            Ok(())
        } else {
            Err(Refused::NotAdmin)
        }
    };
    match entry {
        Entry::Key {
            keys,
            namespace: ns,
            at,
        } => {
            admin_only()?;
            let prefix = namespace(&ns).map_err(|why| Refused::Namespace { why })?;
            if keys.is_empty() || keys.len() > 2 || !keys.iter().all(|k| signing::is_public_key(k))
            {
                return Err(Refused::Unreadable {
                    why: "the team's key is one or two minisign public keys".into(),
                });
            }
            let ids = keys.iter().filter_map(|k| signing::key_id(k)).collect();
            state.key = Some(KeyView {
                keys,
                ids,
                namespace: format!("{prefix}.*"),
                prefix,
                by: by.to_string(),
                at,
                seq,
            });
        }
        Entry::Submit { id, submission, at } => {
            let prefix = state.key.as_ref().map(|k| k.prefix.clone());
            if prefix.is_some_and(|p| !may_sign(&p, &submission.plugin)) {
                return Err(Refused::OutsideNamespace {
                    id: submission.plugin,
                });
            }
            if submission.digest.len() != 64 || submission.archive_sha256.len() != 64 {
                return Err(Refused::Unreadable {
                    why: "a proposal names its archive by its SHA-256".into(),
                });
            }
            let id = hex::encode(id);
            state.proposed.insert(
                id.clone(),
                Proposed {
                    id,
                    submission,
                    by: by.to_string(),
                    at,
                    seq,
                },
            );
        }
        Entry::Publish {
            submission,
            statement,
            minisig,
            archive,
            module,
            ..
        } => {
            admin_only()?;
            let key = state.key.clone().ok_or(Refused::NoKey)?;
            let trusted = store_of(state).trusted_keys();
            let (read, signer) = statement_signed(
                &trusted,
                &statement,
                &minisig,
                &key.prefix,
                &team_mark(team),
            )?;
            state.proposed.remove(&hex::encode(submission));
            state
                .published
                .retain(|p| !(p.statement.id == read.id && p.statement.version == read.version));
            state.published.push(Published {
                statement: read,
                statement_bytes: statement,
                minisig,
                archive,
                module,
                signer,
                seq,
            });
        }
        Entry::Revocations { json, minisig, .. } => {
            admin_only()?;
            let trusted = store_of(state).trusted_keys();
            let serial = revocations_signed(&trusted, &json, &minisig).ok_or(Refused::Signature)?;
            if let Some(seen) = &state.revocations {
                if serial < seen.serial || serial == seen.serial && json != seen.json {
                    return Err(Refused::Older {
                        serial,
                        seen: seen.serial,
                    });
                }
            }
            state.revocations = Some(Revocations {
                serial,
                json,
                minisig,
                seq,
            });
        }
        Entry::Dismiss { submission, .. } => {
            let id = hex::encode(submission);
            let author = state
                .proposed
                .get(&id)
                .is_some_and(|p| p.by == by.to_string());
            if !admin && !author {
                return Err(Refused::NotAdmin);
            }
            state.proposed.remove(&id);
        }
    }
    Ok(())
}

async fn read(team: &Team, state: &mut State) -> Result<(), Error> {
    let client = team.client().clone();
    if let Some((seq, d)) = state.last {
        let page = client
            .read_project(&PLUGINS_STREAM.0, seq - 1, Some(1))
            .await?;
        match page.entries.first() {
            Some(e) if e.seq == seq => {
                if entry_digest(e) != d {
                    state.alarm = Some(Alarm::Rewritten);
                    return Ok(());
                }
            }
            // The very entry removed by the server's operator (SR2-02): a hole, the reading goes on after it.
            _ if client.was_dropped(Some(&PLUGINS_STREAM.0), seq, &d).await? => state.last = None,
            _ => {
                state.alarm = Some(Alarm::Rollback);
                return Ok(());
            }
        }
    }
    loop {
        let page = client
            .read_project(&PLUGINS_STREAM.0, state.cursor, None)
            .await?;
        for e in &page.entries {
            if e.seq <= state.cursor {
                state.alarm = Some(Alarm::Server);
                return Ok(());
            }
            match open(team, e) {
                Ok((by, admin, entry)) => {
                    if let Err(reason) = apply(team, state, e.seq, by, admin, entry) {
                        state.refusals.push(Refusal { seq: e.seq, reason });
                    }
                }
                Err(Opened::Refused(reason)) => state.refusals.push(Refusal { seq: e.seq, reason }),
                Err(Opened::Fork) => {
                    state.alarm = Some(Alarm::Fork);
                    return Ok(());
                }
                Err(Opened::Later) => return Ok(()),
            }
            state.cursor = e.seq;
            state.last = Some((e.seq, entry_digest(e)));
        }
        if !page.more || page.entries.is_empty() {
            return Ok(());
        }
    }
}

fn load(team: &Team) -> Result<State, Error> {
    Ok(team.load_extra(STATE)?.unwrap_or_default())
}

fn save(team: &Team, state: &State) -> Result<(), Error> {
    team.save_extra(STATE, state)
}

/// Reads what is new in the team's store. Under an alarm, nothing is read until [`retry`].
pub async fn sync(team: &mut Team) -> Result<Store, Error> {
    let mut state = load(team)?;
    if state.alarm.is_none() {
        read(team, &mut state).await?;
    }
    save(team, &state)?;
    Ok(store_of(&state))
}

/// The store as this device read it last, without reading the server.
pub fn view(team: &Team) -> Result<Store, Error> {
    Ok(store_of(&load(team)?))
}

/// « Réessayer »: the alarm lifted, the stream read again from where it was.
pub async fn retry(team: &mut Team) -> Result<Store, Error> {
    let mut state = load(team)?;
    state.alarm = None;
    save(team, &state)?;
    sync(team).await
}

fn admin(team: &Team) -> Result<(), Error> {
    if team.is_admin()? {
        Ok(())
    } else {
        Err(CryptoError::NotAdmin.into())
    }
}

fn random16() -> Result<[u8; 16], Error> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|e| Error::Encoding(e.to_string()))?;
    Ok(b)
}

/// The entry sealed by this device, as it goes to the server: a fresh object id each time.
pub fn seal(team: &Team, entry: &Entry) -> Result<Vec<u8>, Error> {
    let sealed = team.device().seal(
        PLUGINS_STREAM,
        ObjectId(random16()?),
        0,
        &wire::encode(entry),
    )?;
    Ok(wire::encode(&sealed))
}

async fn append(team: &mut Team, entry: &Entry) -> Result<Store, Error> {
    let view = sync(team).await?;
    if view.alarm.is_some() {
        return Err(Error::Refused(
            "the server shows what cannot be in the team's store: nothing is sent".into(),
        ));
    }
    let payload = seal(team, entry)?;
    team.client()
        .append_project(&PLUGINS_STREAM.0, &payload)
        .await?;
    sync(team).await
}

/// An administrator declares the team's key (its key of every day and its backup key) and its namespace. The
/// namespace is checked first: `dev.escouade.*` never leaves. Whether a plugin of another origin installed on this
/// machine is in the namespace is the core's to check, before.
pub async fn declare(
    team: &mut Team,
    keys: Vec<String>,
    raw_namespace: &str,
    at: u64,
) -> Result<Store, Error> {
    admin(team)?;
    let prefix = namespace(raw_namespace).map_err(|e| Error::Refused(e.to_string()))?;
    if keys.is_empty() || keys.len() > 2 || !keys.iter().all(|k| signing::is_public_key(k)) {
        return Err(Error::Refused(
            "the team's key is one or two minisign public keys".into(),
        ));
    }
    append(
        team,
        &Entry::Key {
            keys,
            namespace: format!("{prefix}.*"),
            at,
        },
    )
    .await
}

/// A member proposes a version: its archive and its module go to the server as encrypted files (the team's key of
/// the epoch), then its proposal. Its id (hex).
pub async fn submit(
    team: &mut Team,
    mut submission: Submission,
    archive: &[u8],
    module: Option<&[u8]>,
    at: u64,
) -> Result<(String, Store), Error> {
    let view = sync(team).await?;
    let key = view.key.as_ref().ok_or_else(|| {
        Error::Refused("the team has no key for its store yet: an administrator makes it".into())
    })?;
    if !may_sign(&key.prefix, &submission.plugin) {
        return Err(Error::Refused(format!(
            "{} is not in the team's namespace {}",
            submission.plugin, key.namespace
        )));
    }
    if hex::encode(Sha256::digest(archive)) != submission.archive_sha256 {
        return Err(Error::Refused("the archive is not the one checked".into()));
    }
    let archive_id = ObjectId(random16()?);
    team.put_file(PLUGINS_STREAM, archive_id, archive).await?;
    submission.archive = archive_id.0;
    submission.module = match module {
        Some(bytes) => {
            let id = ObjectId(random16()?);
            team.put_file(PLUGINS_STREAM, id, bytes).await?;
            Some(id.0)
        }
        None => None,
    };
    let id = random16()?;
    let store = append(team, &Entry::Submit { id, submission, at }).await?;
    Ok((hex::encode(id), store))
}

/// The archive and the module of a proposal or a version, opened (the team's key), checked against their SHA-256.
pub async fn files(
    team: &mut Team,
    archive: [u8; 16],
    archive_sha256: &str,
    module: Option<[u8; 16]>,
    module_sha256: Option<&str>,
) -> Result<(Vec<u8>, Option<Vec<u8>>), Error> {
    let gone = || Error::Refused("the server no longer has this version's files".into());
    let a = team
        .get_file(PLUGINS_STREAM, ObjectId(archive))
        .await?
        .ok_or_else(gone)?;
    if hex::encode(Sha256::digest(&a)) != archive_sha256 {
        return Err(Error::Refused(
            "the archive the server gave is not the one signed".into(),
        ));
    }
    let m = match module {
        Some(id) => {
            let bytes = team
                .get_file(PLUGINS_STREAM, ObjectId(id))
                .await?
                .ok_or_else(gone)?;
            if Some(hex::encode(Sha256::digest(&bytes)).as_str()) != module_sha256 {
                return Err(Error::Refused(
                    "the module the server gave is not the one signed".into(),
                ));
            }
            Some(bytes)
        }
        None => None,
    };
    Ok((a, m))
}

/// The statement an administrator signs for the proposal `proposed`, whose digest their own machine found.
pub fn statement_for(
    team: &Team,
    store: &Store,
    proposed: &Proposed,
    at: u64,
) -> Result<Statement, Error> {
    let key = store
        .key
        .as_ref()
        .ok_or_else(|| Error::Refused("the team has no key for its store yet".into()))?;
    let s = &proposed.submission;
    if !may_sign(&key.prefix, &s.plugin) {
        return Err(Error::Refused(format!(
            "{} is not in the team's namespace {}",
            s.plugin, key.namespace
        )));
    }
    Ok(Statement {
        format: 1,
        id: s.plugin.clone(),
        version: s.version.clone(),
        tag: s.tag.clone(),
        name: s.name.clone(),
        description: s.description.clone(),
        namespace: key.prefix.clone(),
        team: team_mark(team),
        archive_sha256: s.archive_sha256.clone(),
        module_sha256: s.module_sha256.clone(),
        digest: s.digest.clone(),
        signed_at: at,
        by: team.identity().member.to_string(),
    })
}

/// What signs a statement's bytes with its trusted comment: the team's key, opened by its password.
pub type Sign<'a> = dyn Fn(&[u8], &str) -> Result<String, Error> + Sync + 'a;

/// An administrator publishes the proposal `id`, once their own checks found `digest`: the same as its author's, or
/// nothing is signed. `sign` signs the statement's bytes with its comment (the team's key, opened by its password).
pub async fn publish(
    team: &mut Team,
    id: &str,
    digest: &str,
    sign: &Sign<'_>,
    at: u64,
) -> Result<Store, Error> {
    admin(team)?;
    let store = sync(team).await?;
    let proposed = store
        .proposed
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| Error::Refused("this proposal is no longer waiting".into()))?;
    if proposed.submission.digest != digest {
        return Err(Error::Refused(format!(
            "the digests differ: {} on the machine of who proposed, {digest} here; nothing is signed",
            proposed.submission.digest
        )));
    }
    let statement = statement_for(team, &store, &proposed, at)?;
    let bytes = statement.bytes()?;
    let minisig = sign(&bytes, &statement.comment())?;
    let key = store.key.as_ref().expect("statement_for checked it");
    statement_signed(
        &store.trusted_keys(),
        &bytes,
        &minisig,
        &key.prefix,
        &team_mark(team),
    )
    .map_err(|_| {
        Error::Refused("this key is not the team's declared key: nothing is published".into())
    })?;
    let mut sub = [0u8; 16];
    hex::decode_to_slice(id, &mut sub).map_err(|e| Error::Encoding(e.to_string()))?;
    append(
        team,
        &Entry::Publish {
            submission: sub,
            statement: bytes,
            minisig,
            archive: proposed.submission.archive,
            module: proposed.submission.module,
            at,
        },
    )
    .await
}

/// An administrator writes the team's revocation list, signed by the team's key: its serial must be above the last.
pub async fn revoke(
    team: &mut Team,
    json: Vec<u8>,
    minisig: String,
    at: u64,
) -> Result<Store, Error> {
    admin(team)?;
    let store = sync(team).await?;
    let serial = revocations_signed(&store.trusted_keys(), &json, &minisig).ok_or_else(|| {
        Error::Refused("the list is not signed by the team's key, or not for its serial".into())
    })?;
    if let Some(seen) = &store.revocations {
        if serial <= seen.serial {
            return Err(Error::Refused(format!(
                "the serial {serial} does not go past the last list's ({})",
                seen.serial
            )));
        }
    }
    append(team, &Entry::Revocations { json, minisig, at }).await
}

/// A proposal set aside: by an administrator, or its author.
pub async fn dismiss(team: &mut Team, id: &str, why: &str, at: u64) -> Result<Store, Error> {
    let mut sub = [0u8; 16];
    hex::decode_to_slice(id, &mut sub).map_err(|e| Error::Encoding(e.to_string()))?;
    append(
        team,
        &Entry::Dismiss {
            submission: sub,
            why: why.to_owned(),
            at,
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_namespace_is_a_reversed_domain_never_escouade_s() {
        assert_eq!(namespace("fr.acme.*").as_deref(), Ok("fr.acme"));
        assert_eq!(namespace(" FR.Acme.* ").as_deref(), Ok("fr.acme"));
        assert_eq!(
            namespace("com.acme.tools.*").as_deref(),
            Ok("com.acme.tools")
        );
        for bad in [
            "fr.acme",
            "acme.*",
            "*",
            "fr..acme.*",
            "fr.acme.x*",
            "fr.ac me.*",
            "fr.-acme.*",
        ] {
            assert_eq!(namespace(bad), Err(BadNamespace::Form), "{bad}");
        }
        for first in ["dev.escouade.*", "dev.escouade.team.*", "dev.*"] {
            assert_eq!(namespace(first), Err(BadNamespace::FirstParty), "{first}");
        }
        assert!(namespace("dev.exemple.*").is_ok());
    }

    #[test]
    fn the_team_signs_inside_its_namespace_alone() {
        assert!(may_sign("fr.acme", "fr.acme.rules"));
        assert!(may_sign("fr.acme", "fr.acme.tools.lint"));
        assert!(
            !may_sign("fr.acme", "fr.acmeco.rules"),
            "a prefix is not a namespace"
        );
        assert!(!may_sign("fr.acme", "com.other.rules"));
        assert!(!may_sign("fr.acme", "dev.escouade.github"));
        assert!(
            !may_sign("dev", "dev.escouade.github"),
            "never dev.escouade.*"
        );
    }

    fn statement(id: &str) -> Statement {
        Statement {
            format: 1,
            id: id.into(),
            version: "1.0.0".into(),
            tag: "v1.0.0".into(),
            name: serde_json::Value::Null,
            description: serde_json::Value::Null,
            namespace: "fr.acme".into(),
            team: "aa".into(),
            archive_sha256: "0".repeat(64),
            module_sha256: None,
            digest: "1".repeat(64),
            signed_at: 1,
            by: "bb".into(),
        }
    }

    #[test]
    fn a_statement_holds_with_the_team_s_key_alone() {
        let key = signing::SigningKey::generate().unwrap();
        let other = signing::SigningKey::generate().unwrap();
        let s = statement("fr.acme.rules");
        let bytes = s.bytes().unwrap();
        let sig = key.sign(&bytes, &s.comment()).unwrap();
        let keys = vec![key.public()];
        let (read, signer) = statement_signed(&keys, &bytes, &sig, "fr.acme", "aa").unwrap();
        assert_eq!((read, signer), (s.clone(), key.id()));
        assert_eq!(
            statement_signed(&[other.public()], &bytes, &sig, "fr.acme", "aa"),
            Err(Refused::Signature)
        );
        assert_eq!(
            statement_signed(&keys, &bytes, &sig, "fr.acme", "another team"),
            Err(Refused::Signature),
            "another team's statement"
        );
        // Signed with the comment of another version.
        let wrong = key
            .sign(&bytes, "escouade-team-v1 fr.acme.rules@9.9.9 fr.acme")
            .unwrap();
        assert_eq!(
            statement_signed(&keys, &bytes, &wrong, "fr.acme", "aa"),
            Err(Refused::Signature)
        );
        // Outside the namespace, or Escouade's own: refused before the signature is even read.
        for id in ["com.other.x", "dev.escouade.github"] {
            let s = statement(id);
            let bytes = s.bytes().unwrap();
            let sig = key.sign(&bytes, &s.comment()).unwrap();
            assert!(matches!(
                statement_signed(&keys, &bytes, &sig, "fr.acme", "aa"),
                Err(Refused::OutsideNamespace { .. })
            ));
        }
    }

    #[test]
    fn a_revocation_list_names_its_serial_in_its_signature() {
        let key = signing::SigningKey::generate().unwrap();
        let json = br#"{"serial":4,"issued_at":"2026-11-21","entries":[]}"#;
        let sig = key.sign(json, "escouade-revocations-v1 4 fr.acme").unwrap();
        assert_eq!(revocations_signed(&[key.public()], json, &sig), Some(4));
        let other = key.sign(json, "escouade-revocations-v1 3 fr.acme").unwrap();
        assert_eq!(revocations_signed(&[key.public()], json, &other), None);
        let bad = br#"{"serial":0,"issued_at":"x","entries":[]}"#;
        let sig = key.sign(bad, "escouade-revocations-v1 0").unwrap();
        assert_eq!(revocations_signed(&[key.public()], bad, &sig), None);
    }
}
