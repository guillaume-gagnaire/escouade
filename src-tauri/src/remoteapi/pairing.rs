//! The pairing of a new device (ESC-514, `docs/REMOTE.md`, « Le mode serveur »): `escouade-server pair` shows a code,
//! the user types it in the client, which gives it to the API in place of the handshake (`proto::ToServer::Pair`); the
//! server answers a device token, kept by the client from then on (`devices`).
//!
//! - **Once and briefly**: one code waits at a time (a new `pair` takes the place of the one before), for
//!   [`LIFETIME`]; the right code ends it, and so do [`MAX_TRIES`] wrong ones.
//! - **Written beside the data**: the code waits in `remote-pairing.json`, readable by its owner alone, so that `pair`
//!   works whether the server runs or not (the server holds the data folder: `pair` writes beside it, under a lock of its
//!   own, `headless::with_file_lock`). The file holds the code's hash, never the code.
//! - **Compared in constant time** (`escouade_mcp::same_token`, on the hashes): the time an answer takes says nothing
//!   of how close a code came.
//! - **Never written in the log**: neither the code nor the token it is exchanged for; `pair` prints the code on its
//!   standard output, for the user alone.

use crate::headless::with_file_lock;
use crate::paths::DataDir;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// The file of the code that waits, in the data folder.
pub const FILE: &str = "remote-pairing.json";

/// How long a code waits for its device.
pub const LIFETIME_MS: i64 = 10 * 60_000;

/// The wrong codes a code takes before it ends.
pub const MAX_TRIES: u32 = 5;

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

/// The code that waits, as its file keeps it: its hash, its end, the wrong codes it may still take.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Waiting {
    hash: String,
    expires_at: i64,
    tries_left: u32,
}

/// Why a code was refused: what the client tells the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// No code waits: none was asked for, or it was used (a code given again is this), or it ended.
    NoCode,
    /// The code waited too long.
    Expired,
    /// Not the code that waits: `left` tries remain.
    Wrong { left: u32 },
    /// The last try was wrong: the code ended.
    TooManyTries,
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
            Refusal::Wrong { left } => tr!(
                "Code d’appairage faux ({left} essais restants).",
                "Wrong pairing code ({left} tries left)."
            ),
            Refusal::TooManyTries => tr!(
                "Trop d’essais faux : ce code d’appairage ne vaut plus. Lance « escouade-server pair » de nouveau.",
                "Too many wrong tries: this pairing code is void. Run “escouade-server pair” again."
            ),
        }
    }

    /// Why, as the client reads it (`proto::ToClient::Refused::reason`, ESC-537): `none`, `expired`, `wrong`, `tries`.
    pub fn code(&self) -> &'static str {
        match self {
            Refusal::NoCode => "none",
            Refusal::Expired => "expired",
            Refusal::Wrong { .. } => "wrong",
            Refusal::TooManyTries => "tries",
        }
    }

    /// As the log says it: never the code.
    pub fn reason(&self) -> &'static str {
        match self {
            Refusal::NoCode => "no code waiting",
            Refusal::Expired => "code expired",
            Refusal::Wrong { .. } => "wrong code",
            Refusal::TooManyTries => "too many wrong codes: the code ended",
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
    let mut bytes = [0u8; LENGTH];
    getrandom::fill(&mut bytes).map_err(|e| anyhow::anyhow!("random: {e}"))?;
    let letters: String = bytes
        .iter()
        .map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char)
        .collect();
    let waiting = Waiting {
        hash: hash(&letters),
        expires_at: now + LIFETIME_MS,
        tries_left: MAX_TRIES,
    };
    let path = file(data);
    with_file_lock(&path, || write(&path, &waiting))?;
    Ok(Code {
        text: format!("{}-{}", &letters[..4], &letters[4..]),
        expires_at: waiting.expires_at,
    })
}

/// The code `presented` given for `data` at `now`: Ok once, when it is the one that waits and has not ended; it ends
/// then. A wrong one costs a try.
pub fn redeem(data: &DataDir, presented: &str, now: i64) -> std::result::Result<(), Refusal> {
    let path = file(data);
    let outcome = with_file_lock(&path, || {
        let Some(mut waiting) = read(&path) else {
            return Ok(Err(Refusal::NoCode));
        };
        if now >= waiting.expires_at {
            remove(&path);
            return Ok(Err(Refusal::Expired));
        }
        if waiting.tries_left == 0 {
            remove(&path);
            return Ok(Err(Refusal::TooManyTries));
        }
        let code = normalized(presented);
        let right = code.len() == LENGTH && escouade_mcp::same_token(&hash(&code), &waiting.hash);
        if right {
            remove(&path);
            return Ok(Ok(()));
        }
        waiting.tries_left -= 1;
        if waiting.tries_left == 0 {
            remove(&path);
            return Ok(Err(Refusal::TooManyTries));
        }
        write(&path, &waiting)?;
        Ok(Err(Refusal::Wrong {
            left: waiting.tries_left,
        }))
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
        .filter(|w| now < w.expires_at && w.tries_left > 0)
        .map(|w| w.expires_at)
}

/// The code that waits ended by hand (`escouade-server pair --cancel`).
pub fn cancel(data: &DataDir) -> Result<()> {
    let path = file(data);
    with_file_lock(&path, || {
        remove(&path);
        Ok(())
    })
}
