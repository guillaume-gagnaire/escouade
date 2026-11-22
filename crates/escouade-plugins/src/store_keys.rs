//! The seal « Vérifié par Escouade » of a plugin's version from the store (docs/STORE.md §4.5):
//! the review signs the version's `version.json` with minisign on the user's machine, its trusted
//! comment `escouade-verified-v1 <id>@<version> <date>`; the app checks it with the public keys it
//! embeds, the same as `store/keys/*.pub`. The store cannot make a seal: it has no secret key.
//!
//! Each key has its roles, here in the code (audit SEC-03, ESC-592): the key of the first hand
//! seals `dev.escouade.*` alone, the key of the third parties seals every other id and signs the
//! revocation list, the backup does all of it. A seal of a key whose role does not cover the id is
//! refused, its reason said: no key without a password can seal a plugin of another author, nor a
//! revocation list, whoever holds it.
//!
//! The key of the first hand and the backup were made by the user on their machine on 2026-10-30
//! (docs/STORE.md §4.5); the key of the third parties is made by the user too, with its password
//! (`npm run plugin-review -- create-key third-party`), then embedded here. Their secret halves
//! never leave the user's machine.

use minisign_verify::{PublicKey, Signature};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// What a key of the store may sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// The plugins of Escouade's own hand, `dev.escouade.*` only: the key the chief signs with
    /// after his review, kept without a password.
    FirstHand,
    /// The plugins of other authors, never `dev.escouade.*`: a key with a password.
    ThirdParty,
    /// The revocation list: a key with a password (the third parties' holds it).
    Revocation,
    /// The backup, kept offline: every role, to go on when another key is lost or revoked.
    Backup,
}

impl Role {
    /// Whether a seal of this role makes the plugin `id` verified.
    pub fn seals(self, id: &str) -> bool {
        match self {
            Role::FirstHand => crate::builtin::first_hand(id),
            Role::ThirdParty => !crate::builtin::first_hand(id),
            Role::Revocation => false,
            Role::Backup => true,
        }
    }

    /// Whether a revocation list signed by this role holds.
    pub fn signs_revocations(self) -> bool {
        matches!(self, Role::Revocation | Role::Backup)
    }
}

/// A public key the app embeds: its file in `store/keys/`, its base64 line, its roles.
#[derive(Debug)]
pub struct EmbeddedKey {
    pub file: &'static str,
    pub public: &'static str,
    pub roles: &'static [Role],
}

/// The roles of a key of `store/keys/` by its file's name, the same in the store and its tools
/// (`store/server/utils/minisign.ts`, `ROLES_BY_FILE`).
pub fn roles_of_file(file: &str) -> Option<&'static [Role]> {
    match file {
        "store.pub" => Some(&[Role::FirstHand]),
        "third-party.pub" => Some(&[Role::ThirdParty, Role::Revocation]),
        "backup.pub" => Some(&[Role::Backup]),
        _ => None,
    }
}

/// The public keys of the store's signatures, as the second line of their `.pub` files: the key of
/// the first hand (`store.pub`, id E0516468FFDCBDD5, the store's one key until ESC-592: it keeps
/// every seal it made, all of `dev.escouade.*`), then the backup (`backup.pub`, id
/// 177096D57E2EC0C8). The key of the third parties (`third-party.pub`) comes here once the user
/// made it; until then a plugin of another author is sealed by the backup alone.
pub const STORE_KEYS: &[EmbeddedKey] = &[
    EmbeddedKey {
        file: "store.pub",
        public: "RWTVvdz/aGRR4MBGtNPfgdhYoq0ihwxZLK1lkK+ymqmoITBDaYuddpCW",
        roles: &[Role::FirstHand],
    },
    EmbeddedKey {
        file: "backup.pub",
        public: "RWTIwC5+1ZZwFzhOKuS2/h00baOvCuqL7L15MIfpP2QIcaOl3JCDe75S",
        roles: &[Role::Backup],
    },
];

/// The revocation lists the key of the first hand signed before ESC-592, by the SHA-256 of their
/// bytes: they hold still (an app that kept one reads it again), and no other list of that key
/// does. The one: serial 1 of 2026-11-07, empty (`website/public/revocations.json`).
pub const LEGACY_REVOCATION_LISTS: &[&str] =
    &["0a9a3c2f9b497a487a1fd95d4ae943d5eee329267a3ab2733e9b1fcb82f300b9"];

/// What the trusted comment of a version's signature starts with.
pub const VERIFIED_COMMENT: &str = "escouade-verified-v1";

/// A key a seal or a list is checked with, and its roles. A bare key (a `&str`, a `String`: a
/// test's) holds every role, as the backup does.
pub trait TrustedKey {
    fn public(&self) -> &str;
    fn roles(&self) -> &[Role];
}

const EVERY_ROLE: &[Role] = &[Role::Backup];

impl TrustedKey for &str {
    fn public(&self) -> &str {
        self
    }
    fn roles(&self) -> &[Role] {
        EVERY_ROLE
    }
}

impl TrustedKey for String {
    fn public(&self) -> &str {
        self
    }
    fn roles(&self) -> &[Role] {
        EVERY_ROLE
    }
}

/// A key of the app, with its roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub public: String,
    pub roles: Vec<Role>,
}

impl TrustedKey for Key {
    fn public(&self) -> &str {
        &self.public
    }
    fn roles(&self) -> &[Role] {
        &self.roles
    }
}

/// The key that made a seal hold, as the window says it: its id, and the role it sealed in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SealKey {
    pub id: String,
    pub role: Role,
}

/// Why a seal is refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// The app embeds no key yet.
    NoKey,
    /// The `.minisig` does not read.
    Unreadable,
    /// No embedded key signed these bytes.
    NotSigned,
    /// Signed, but for another plugin or another version.
    OtherVersion,
    /// Signed by a key of the app whose roles do not cover it: the key of the first hand on an id
    /// out of `dev.escouade.*`, the third parties' on `dev.escouade.*`, a list signed by a key
    /// that signs no list. `key`: its id.
    OutOfRole { key: String },
}

/// The keys a seal or a revocation list is checked with: the embedded ones; in a debug build, also
/// the key of `ESCOUADE_STORE_TEST_KEY` (the fake store of the end-to-end tests), with every role.
/// A release build never reads it: no variable of the environment can make a seal there.
pub fn keys() -> Vec<Key> {
    let mut keys: Vec<Key> = STORE_KEYS
        .iter()
        .map(|k| Key {
            public: k.public.to_string(),
            roles: k.roles.to_vec(),
        })
        .collect();
    if cfg!(debug_assertions) {
        if let Some(k) = std::env::var("ESCOUADE_STORE_TEST_KEY")
            .ok()
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
        {
            keys.push(Key {
                public: k,
                roles: vec![Role::Backup],
            });
        }
    }
    keys
}

/// Whether `minisig` makes version `version` of plugin `id` verified: the bytes of its
/// `version.json` signed by one of the app's keys (`keys`) whose role covers this id, the comment
/// naming this id and this number.
#[cfg_attr(not(any(test, feature = "testing")), allow(dead_code))]
pub fn verified(
    version_json: &[u8],
    minisig: &str,
    id: &str,
    version: &str,
) -> Result<(), Refused> {
    verified_with(&keys(), version_json, minisig, id, version)
}

/// What the trusted comment of a revocation list starts with, followed by its serial.
pub const REVOCATIONS_COMMENT: &str = "escouade-revocations-v1";

/// The key of `keys` that signed `bytes` with `sig`.
fn signer<'k, K: TrustedKey>(keys: &'k [K], bytes: &[u8], sig: &Signature) -> Option<&'k K> {
    keys.iter().find(|k| {
        PublicKey::from_base64(k.public()).is_ok_and(|pk| pk.verify(bytes, sig, false).is_ok())
    })
}

/// Whether `minisig` signs the revocation list `json` of serial `serial`, with `keys`: the bytes
/// signed by one of them, the comment `escouade-revocations-v1 <serial> …`, by a key that signs
/// lists (the third parties', the backup), or by the key of the first hand for a list it signed
/// before ESC-592 (`LEGACY_REVOCATION_LISTS`). Gives the id of the key that signed (a list that
/// revokes its own signer is refused by the caller).
pub fn revocations_signed<K: TrustedKey>(
    keys: &[K],
    json: &[u8],
    minisig: &str,
    serial: u64,
) -> Result<String, Refused> {
    if keys.is_empty() {
        return Err(Refused::NoKey);
    }
    let sig = Signature::decode(minisig).map_err(|_| Refused::Unreadable)?;
    let Some(key) = signer(keys, json, &sig) else {
        return Err(Refused::NotSigned);
    };
    let mut words = sig.trusted_comment().split_whitespace();
    match (words.next(), words.next()) {
        (Some(REVOCATIONS_COMMENT), Some(s)) if s == serial.to_string() => {}
        _ => return Err(Refused::OtherVersion),
    }
    let id = key_id(key.public());
    let legacy = key.roles().contains(&Role::FirstHand)
        && LEGACY_REVOCATION_LISTS.contains(&hex_sha256(json).as_str());
    if key.roles().iter().any(|r| r.signs_revocations()) || legacy {
        Ok(id)
    } else {
        Err(Refused::OutOfRole { key: id })
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The id of a minisign public key (its bytes 2 to 10, as minisign writes it: hex, upper case,
/// the bytes reversed), the one `keys_revoked` names.
pub fn key_id(key: &str) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(key.trim())
        .ok()
        .filter(|b| b.len() >= 10)
        .map(|b| b[2..10].iter().rev().map(|x| format!("{x:02X}")).collect())
        .unwrap_or_default()
}

/// `verified` with the keys given (the tests' own).
pub fn verified_with<K: TrustedKey>(
    keys: &[K],
    version_json: &[u8],
    minisig: &str,
    id: &str,
    version: &str,
) -> Result<(), Refused> {
    seal_of(keys, version_json, minisig, id, version).map(|_| ())
}

/// The seal of version `version` of plugin `id` with `keys`, as `verified` checks it: the key that
/// signed and the role it sealed in.
pub fn seal_of<K: TrustedKey>(
    keys: &[K],
    version_json: &[u8],
    minisig: &str,
    id: &str,
    version: &str,
) -> Result<SealKey, Refused> {
    if keys.is_empty() {
        return Err(Refused::NoKey);
    }
    let sig = Signature::decode(minisig).map_err(|_| Refused::Unreadable)?;
    let Some(key) = signer(keys, version_json, &sig) else {
        return Err(Refused::NotSigned);
    };
    let mut words = sig.trusted_comment().split(' ');
    let target = format!("{id}@{version}");
    match (words.next(), words.next(), words.next(), words.next()) {
        (Some(VERIFIED_COMMENT), Some(t), Some(_date), None) if t == target => {}
        _ => return Err(Refused::OtherVersion),
    }
    let kid = key_id(key.public());
    match key.roles().iter().find(|r| r.seals(id)) {
        Some(role) => Ok(SealKey {
            id: kid,
            role: *role,
        }),
        None => Err(Refused::OutOfRole { key: kid }),
    }
}

#[cfg(test)]
#[path = "store_keys_tests.rs"]
mod tests;
