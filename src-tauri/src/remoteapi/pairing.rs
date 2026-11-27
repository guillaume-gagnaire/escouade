//! The pairing of a new device (ESC-514, `docs/REMOTE.md`, « Le mode serveur »): `escouade-server pair` shows a code,
//! the user types it in the client, which gives it to the API in place of the handshake (`proto::ToServer::Pair`); the
//! server answers a device token, kept by the client from then on (`devices`).
//!
//! - **Once and briefly**: one code waits at a time (a new `pair` takes the place of the one before), for
//!   [`LIFETIME_MS`]; the right code ends it. A wrong one never does (ESC-654: five wrong codes from anyone who reached
//!   the API once ended it, and so blocked the pairing at will): the server bounds the tries, by source and in all
//!   (`server::Limits`), which keeps a guess out of reach: 120 wrong codes a minute from this machine and 120 from
//!   elsewhere (apart, so that strangers never refuse the codes of this machine: SR1-04, ESC-631), 2 400 in a code's 10
//!   minutes, against 32⁸ = 2⁴⁰ codes, one chance in some 450 million.
//! - **Written beside the data**: the code waits in `remote-pairing.json`, readable by its owner alone, so that `pair`
//!   works whether the server runs or not (the server holds the data folder: `pair` writes beside it, under a lock of its
//!   own, `headless::with_file_lock`). The file holds the code's hash, never the code.
//! - **Compared in constant time** (`escouade_mcp::same_token`, on the hashes): the time an answer takes says nothing
//!   of how close a code came.
//! - **Never written in the log**: neither the code nor the token it is exchanged for; `pair` prints the code on its
//!   standard output, for the user alone.
//! - **Its scope** (ESC-570, SR1-01): a code says the scope of the device born of it (`computer` for « Créer un code »
//!   and `escouade-server pair`, `phone` for a phone's QR code), and the phone's peer it was made with, when it is a
//!   QR code (the peer goes when the code ends unused, or another code takes its place).
//! - **A phone's token, never sent** (ESC-570, SR1-15): a phone gives, with the code, the public half of a key pair its
//!   native side made (X25519); the core answers the public half of one of its own, and each side derives the device's
//!   token and its pairing key from the secret they share ([`derive`]): the token never crosses the WebView's
//!   JavaScript, which sees two public keys.

use crate::headless::with_file_lock;
use crate::paths::DataDir;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub use super::Scope;

/// HKDF-SHA256's `info` of a phone's device token, derived from the secret of the pairing (`docs/MOBILE.md` § 3.1;
/// `crates/escouade-mobile/src/pairing.rs` derives the same).
pub const TOKEN_INFO: &[u8] = b"escouade-mobile/device-token/v1";
/// HKDF-SHA256's `info` of a phone's pairing key (which seals its notifications, MO-6).
pub const KEY_INFO: &[u8] = b"escouade-mobile/pairing-key/v1";

/// The file of the code that waits, in the data folder.
pub const FILE: &str = "remote-pairing.json";

/// How long a code waits for its device.
pub const LIFETIME_MS: i64 = 10 * 60_000;

/// The letters of a code: no 0, O, 1 nor I, which read alike. 32 of them: a random byte gives one, evenly.
const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// The letters of a code (40 bits), shown in two groups of four.
const LENGTH: usize = 8;

/// The longest text read as a code: anything longer is wrong, and never hashed whole.
const MAX_PRESENTED: usize = 64;

/// A code, as `pair` shows it.
#[derive(Debug, Clone)]
pub struct Code {
    /// `ABCD-EFGH`.
    pub text: String,
    /// When it ends (ms since the epoch).
    pub expires_at: i64,
}

/// The code that waits, as its file keeps it: its hash, its end. (A file of before ESC-654 also says `triesLeft`,
/// read and left aside.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Waiting {
    hash: String,
    expires_at: i64,
    /// The device that asked for it (SR1-02): the device born of it is paired from it, and goes when it is revoked.
    /// None: the window or `escouade-server pair`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    by: Option<String>,
    /// The scope of the device born of it (ESC-570): `computer` in a file of before.
    #[serde(default)]
    scope: Scope,
    /// The phone's peer made with it (a QR code): it goes when the code ends unused, or another takes its place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    peer: Option<String>,
}

/// What asks for a code: who, for which scope, with which phone's peer.
#[derive(Debug, Clone, Default)]
pub struct Ask {
    /// The device that asked for it (none: the window, `escouade-server pair`).
    pub by: Option<String>,
    pub scope: Scope,
    /// The phone's peer made for it (a QR code).
    pub peer: Option<String>,
}

/// What a code gave, once redeemed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redeemed {
    pub by: Option<String>,
    pub scope: Scope,
    pub peer: Option<String>,
}

/// What a phone's pairing derives on the core's side ([`derive`]).
pub struct Derived {
    /// The device's token (base64url, 32 bytes): the phone derives the same.
    pub token: String,
    /// The pairing key (base64, 32 bytes): it seals the phone's notifications (MO-6).
    pub key: String,
    /// The core's public half (base64, 32 bytes), answered to the phone.
    pub public: String,
}

/// The token and the pairing key of a phone that gave `phone_public` (its X25519 public key, base64): a key pair of the
/// core's made for it, the secret they share, HKDF-SHA256 (no salt) with [`TOKEN_INFO`] and [`KEY_INFO`]. None when
/// the key is not 32 bytes of base64, or gives a secret of none (a point of small order).
pub fn derive(phone_public: &str) -> Option<Derived> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let bytes: [u8; 32] = b64.decode(phone_public.trim()).ok()?.try_into().ok()?;
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).ok()?;
    let ours = boringtun::x25519::StaticSecret::from(seed);
    let public = boringtun::x25519::PublicKey::from(&ours);
    let (token, key) = derive_from(&ours, &bytes)?;
    Some(Derived {
        token,
        key,
        public: b64.encode(public.as_bytes()),
    })
}

/// The token and the key of the secret `ours` shares with `theirs`: what both sides derive (`derive`, the tests).
pub fn derive_from(
    ours: &boringtun::x25519::StaticSecret,
    theirs: &[u8; 32],
) -> Option<(String, String)> {
    use base64::Engine;
    let shared = ours.diffie_hellman(&boringtun::x25519::PublicKey::from(*theirs));
    if !shared.was_contributory() {
        return None;
    }
    let hk = hkdf::Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut token = [0u8; 32];
    let mut key = [0u8; 32];
    hk.expand(TOKEN_INFO, &mut token).ok()?;
    hk.expand(KEY_INFO, &mut key).ok()?;
    Some((
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token),
        base64::engine::general_purpose::STANDARD.encode(key),
    ))
}

/// Why a code was refused: what the client tells the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// No code waits: none was asked for, or it was used (a code given again is this), or it ended.
    NoCode,
    /// The code waited too long.
    Expired,
    /// Not the code that waits; it waits on.
    Wrong,
    /// Not read: too many wrong tries in the minute, from where it comes or in all (the server's bounds); the code
    /// waits on.
    Wait,
}

impl Refusal {
    pub fn text(&self) -> String {
        match self {
            Refusal::NoCode => tr!(
                "Aucun code d’appairage n’attend (déjà utilisé, ou jamais demandé) : lance « escouade-server pair » sur le serveur.",
                "No pairing code is waiting (used already, or never asked for): run “escouade-server pair” on the server."
            ),
            Refusal::Expired => tr!(
                "Ce code d’appairage a expiré : lance « escouade-server pair » de nouveau.",
                "This pairing code has expired: run “escouade-server pair” again."
            ),
            Refusal::Wrong => tr!(
                "Code d’appairage faux.",
                "Wrong pairing code."
            ),
            Refusal::Wait => tr!(
                "Trop d’essais faux en une minute : le code d’appairage vaut toujours, réessaie dans une minute.",
                "Too many wrong tries in a minute: the pairing code still holds, try again in a minute."
            ),
        }
    }

    /// Why, as the client reads it (`proto::ToClient::Refused::reason`, ESC-537): `none`, `expired`, `wrong`, `wait`
    /// (a server of before ESC-654 also said `tries`, its code ended).
    pub fn code(&self) -> &'static str {
        match self {
            Refusal::NoCode => "none",
            Refusal::Expired => "expired",
            Refusal::Wrong => "wrong",
            Refusal::Wait => "wait",
        }
    }

    /// As the log says it: never the code.
    pub fn reason(&self) -> &'static str {
        match self {
            Refusal::NoCode => "no code waiting",
            Refusal::Expired => "code expired",
            Refusal::Wrong => "wrong code",
            Refusal::Wait => "too many wrong codes in the minute: not read",
        }
    }
}

fn file(data: &DataDir) -> PathBuf {
    data.root().join(FILE)
}

/// A code as it is compared: its letters and digits, in capitals (the dash and spaces the user typed left out).
fn normalized(presented: &str) -> String {
    presented
        .chars()
        .take(MAX_PRESENTED)
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn hash(code: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"escouade-pairing:");
    h.update(code.as_bytes());
    hex::encode(h.finalize())
}

fn read(path: &std::path::Path) -> Option<Waiting> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write(path: &std::path::Path, waiting: &Waiting) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(waiting)?;
    crate::paths::write_private(path, &bytes).with_context(|| format!("writing {FILE}"))
}

fn remove(path: &std::path::Path) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            log::warn!("remote API: {FILE} not removed: {e}");
        }
    }
}

/// A new code for `data`, from now (`now`, ms): it takes the place of the one that waited.
pub fn create(data: &DataDir, now: i64) -> Result<Code> {
    create_by(data, now, None)
}

/// A new code asked by the device `by` (none: the window, `escouade-server pair`), kept with it.
pub fn create_by(data: &DataDir, now: i64, by: Option<&str>) -> Result<Code> {
    create_with(
        data,
        now,
        Ask {
            by: by.map(str::to_string),
            ..Ask::default()
        },
    )
    .map(|(code, _)| code)
}

/// A new code as `ask` says, from `now`: it, and the phone's peer of the code it took the place of (to remove).
pub fn create_with(data: &DataDir, now: i64, ask: Ask) -> Result<(Code, Option<String>)> {
    let mut bytes = [0u8; LENGTH];
    getrandom::fill(&mut bytes).map_err(|e| anyhow::anyhow!("random: {e}"))?;
    let letters: String = bytes
        .iter()
        .map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char)
        .collect();
    let waiting = Waiting {
        hash: hash(&letters),
        expires_at: now + LIFETIME_MS,
        by: ask.by,
        scope: ask.scope,
        peer: ask.peer,
    };
    let path = file(data);
    let replaced = with_file_lock(&path, || {
        let before = read(&path).and_then(|w| w.peer);
        write(&path, &waiting)?;
        Ok(before)
    })?;
    Ok((
        Code {
            text: format!("{}-{}", &letters[..4], &letters[4..]),
            expires_at: waiting.expires_at,
        },
        replaced,
    ))
}

/// The code `presented` given for `data` at `now`: Ok once, when it is the one that waits and has not ended; it ends
/// then. A wrong one is refused, and the code waits on.
pub fn redeem(data: &DataDir, presented: &str, now: i64) -> std::result::Result<(), Refusal> {
    redeem_by(data, presented, now).map(|_| ())
}

/// As `redeem`, with the device that asked for the code (none: the window, `escouade-server pair`).
pub fn redeem_by(
    data: &DataDir,
    presented: &str,
    now: i64,
) -> std::result::Result<Option<String>, Refusal> {
    redeem_with(data, presented, now).map(|r| r.by)
}

/// As `redeem`: who asked for the code, its scope and its phone's peer.
pub fn redeem_with(
    data: &DataDir,
    presented: &str,
    now: i64,
) -> std::result::Result<Redeemed, Refusal> {
    let path = file(data);
    let outcome = with_file_lock(&path, || {
        let Some(waiting) = read(&path) else {
            return Ok(Err(Refusal::NoCode));
        };
        if now >= waiting.expires_at {
            remove(&path);
            return Ok(Err(Refusal::Expired));
        }
        let code = normalized(presented);
        let right = code.len() == LENGTH && escouade_mcp::same_token(&hash(&code), &waiting.hash);
        if right {
            remove(&path);
            return Ok(Ok(Redeemed {
                by: waiting.by,
                scope: waiting.scope,
                peer: waiting.peer,
            }));
        }
        // Nothing written: a wrong code leaves the one that waits as it was (ESC-654).
        Ok(Err(Refusal::Wrong))
    });
    match outcome {
        Ok(outcome) => outcome,
        Err(e) => {
            log::warn!("remote API: the pairing code could not be read: {e:#}");
            Err(Refusal::NoCode)
        }
    }
}

/// Whether a code waits for `data` at `now`, and when it ends (`escouade-server pair --status`, the tests).
pub fn waiting_until(data: &DataDir, now: i64) -> Option<i64> {
    read(&file(data))
        .filter(|w| now < w.expires_at)
        .map(|w| w.expires_at)
}

/// The phone's peer of a code that ended unused (expired) at `now`, the code forgotten: the peer to remove. None while
/// it waits, or when it was no QR code.
pub fn ended_peer(data: &DataDir, now: i64) -> Option<String> {
    let path = file(data);
    let w = read(&path)?;
    if now < w.expires_at || w.peer.is_none() {
        return None;
    }
    with_file_lock(&path, || {
        // Read again under the lock: another process may have made a new one meanwhile.
        match read(&path) {
            Some(again) if now >= again.expires_at => {
                remove(&path);
                Ok(again.peer)
            }
            _ => Ok(None),
        }
    })
    .ok()
    .flatten()
}

/// The phone's peer of the code that waits (a QR code), while it waits.
pub fn waiting_peer(data: &DataDir, now: i64) -> Option<String> {
    read(&file(data))
        .filter(|w| now < w.expires_at)
        .and_then(|w| w.peer)
}

/// The code that waits ended by hand (`escouade-server pair --cancel`).
pub fn cancel(data: &DataDir) -> Result<()> {
    let path = file(data);
    with_file_lock(&path, || {
        remove(&path);
        Ok(())
    })
}
