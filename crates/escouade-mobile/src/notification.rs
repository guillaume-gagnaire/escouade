//! The sealed notifications (docs/MOBILE.md, § 3.11 and « Le format des notifications scellées »).
//!
//! The core seals the text of a notification with the pairing key; the relay, then FCM or APNs, see a wake-up and
//! opaque bytes only; the phone opens them here, in its notification service (Android) or its Notification Service
//! Extension (iOS), and in the app.
//!
//! **The format, version 1.** The sealed text is URL-safe base64 without padding (at most [`MAX_SEALED_LEN`]
//! characters, under the 4 KiB of FCM's data and of APNs) of:
//!
//! ```text
//! version (1 byte, 1) ‖ nonce (24 bytes, random) ‖ XChaCha20-Poly1305(key, nonce, aad, JSON) (the text, then 16 bytes of tag)
//! ```
//!
//! - `key` = HKDF-SHA256 (no salt, info `escouade-mobile/notification/v1`) of the 32-byte pairing key: the key of the
//!   notifications is never the pairing key itself, which may serve elsewhere.
//! - `aad` = `escouade-notification` followed by the version byte: a version cannot be changed without the tag failing.
//! - The JSON is a [`Notification`]: `seq` (from 1, increasing for each pairing), `at` (Unix seconds when sealed),
//!   `kind`, `title`, `body`, and `target` (what touching it reopens) when there is one. Unknown fields are ignored.
//!
//! **What is refused**: a text altered or sealed under another key (the tag, [`OpenError::Forged`]), a version not
//! known, a text too long or not of the format, and a replay: [`ReplayGuard`] keeps the highest `seq` seen and a
//! window of the 64 below it, as WireGuard and IPsec do, since FCM and APNs may deliver out of order; a `seq` seen
//! again is [`OpenError::Replayed`], one below the window [`OpenError::TooOld`]. The guard moves only after the tag
//! has checked out: a forged text never moves it.

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine as _;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroizing;

/// The version of the format this code seals and opens.
pub const VERSION: u8 = 1;
/// The longest sealed text (its base64), under the 4096 bytes of a data message of FCM and of the payload of APNs.
pub const MAX_SEALED_LEN: usize = 3072;
/// The info of the HKDF that gives the key of the notifications from the pairing key.
pub const KEY_INFO: &[u8] = b"escouade-mobile/notification/v1";
const AAD_PREFIX: &[u8] = b"escouade-notification";
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
/// How many `seq` below the highest one the guard remembers.
pub const WINDOW: u64 = 64;

/// What a notification says, once opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    /// From 1, increasing for each pairing: the replay guard reads it.
    pub seq: u64,
    /// When the core sealed it, in Unix seconds: for « il y a 4 min », never to refuse (the phone's clock may be off).
    pub at: u64,
    /// What it is about (`permission`, `question`, `inbox`, …), as MO-6 writes it.
    pub kind: String,
    pub title: String,
    pub body: String,
    /// What touching it reopens (a request, an agent), when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Why a notification was not sealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SealError {
    #[error("the pairing key is not 32 bytes")]
    BadKey,
    #[error("seq starts at 1")]
    ZeroSeq,
    #[error("the sealed notification would be longer than {MAX_SEALED_LEN} characters")]
    TooLarge,
}

/// Why a notification was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    #[error("the pairing key is not 32 bytes of base64")]
    BadKey,
    #[error("the sealed notification is longer than {MAX_SEALED_LEN} characters")]
    TooLarge,
    #[error("the sealed notification is not of the format (base64, its length, its content)")]
    Malformed,
    #[error("the sealed notification is of version {0}, which this app does not know")]
    UnknownVersion(u8),
    #[error("the notification does not open with this key: altered, or sealed under another key")]
    Forged,
    #[error("the notification {0} was already opened: a replay")]
    Replayed(u64),
    #[error("the notification {0} is older than the window of the replay guard")]
    TooOld(u64),
}

/// The guard's state cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the replay guard's state is not of the format")]
pub struct BadGuardState;

/// The `seq` already opened, for one pairing: the highest one and a window of [`WINDOW`] below it. Kept by the app
/// between two notifications (not a secret: a few numbers) by [`ReplayGuard::to_state`] and
/// [`ReplayGuard::from_state`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplayGuard {
    highest: u64,
    /// Bit `i`: `highest - i` was seen.
    seen: u64,
}

impl ReplayGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// The highest `seq` opened so far (0: none).
    pub fn highest(&self) -> u64 {
        self.highest
    }

    /// Takes `seq` if it was never seen and is not below the window.
    fn accept(&mut self, seq: u64) -> Result<(), OpenError> {
        if seq > self.highest {
            let shift = seq - self.highest;
            self.seen = if shift >= WINDOW {
                0
            } else {
                self.seen << shift
            };
            self.seen |= 1;
            self.highest = seq;
            return Ok(());
        }
        let back = self.highest - seq;
        if back >= WINDOW {
            return Err(OpenError::TooOld(seq));
        }
        let bit = 1u64 << back;
        if self.seen & bit != 0 {
            return Err(OpenError::Replayed(seq));
        }
        self.seen |= bit;
        Ok(())
    }

    /// The state, to keep: `1.<highest>.<window in hex>`.
    pub fn to_state(&self) -> String {
        format!("1.{}.{:016x}", self.highest, self.seen)
    }

    /// Reads a state kept by [`to_state`](Self::to_state). An empty one is a new guard.
    pub fn from_state(state: &str) -> Result<Self, BadGuardState> {
        let state = state.trim();
        if state.is_empty() {
            return Ok(Self::new());
        }
        let mut parts = state.split('.');
        let (Some("1"), Some(highest), Some(seen), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(BadGuardState);
        };
        let highest = highest.parse().map_err(|_| BadGuardState)?;
        if seen.len() != 16 {
            return Err(BadGuardState);
        }
        let seen = u64::from_str_radix(seen, 16).map_err(|_| BadGuardState)?;
        // A window without its highest bit, or bits above a small highest, was not written by to_state.
        if (highest == 0) != (seen == 0)
            || (highest > 0 && seen & 1 == 0)
            || (highest < WINDOW && seen >> highest != 0)
        {
            return Err(BadGuardState);
        }
        Ok(Self { highest, seen })
    }
}

fn cipher(pairing_key: &[u8; 32]) -> XChaCha20Poly1305 {
    let mut key = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(None, pairing_key)
        .expand(KEY_INFO, key.as_mut())
        .expect("32 bytes is a valid length for HKDF-SHA256");
    XChaCha20Poly1305::new(key.as_ref().into())
}

fn aad(version: u8) -> Vec<u8> {
    let mut aad = AAD_PREFIX.to_vec();
    aad.push(version);
    aad
}

/// Seals `notification` under the pairing key (what the core does, MO-6; here for the tests and so that both sides
/// share one code).
pub fn seal(pairing_key: &[u8; 32], notification: &Notification) -> Result<String, SealError> {
    if notification.seq == 0 {
        return Err(SealError::ZeroSeq);
    }
    let plain =
        Zeroizing::new(serde_json::to_vec(notification).expect("a notification serializes"));
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce).expect("the system's randomness");
    let sealed = cipher(pairing_key)
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &plain,
                aad: &aad(VERSION),
            },
        )
        .expect("XChaCha20-Poly1305 seals any length that fits in memory");
    let mut raw = Vec::with_capacity(1 + NONCE_LEN + sealed.len());
    raw.push(VERSION);
    raw.extend_from_slice(&nonce);
    raw.extend_from_slice(&sealed);
    let text = B64.encode(raw);
    if text.len() > MAX_SEALED_LEN {
        return Err(SealError::TooLarge);
    }
    Ok(text)
}

/// Opens a sealed notification, and takes its `seq` in the guard: refused if altered, under another key, replayed or
/// too old (the module's documentation). The guard does not move on a refusal.
pub fn open(
    pairing_key: &[u8; 32],
    sealed: &str,
    guard: &mut ReplayGuard,
) -> Result<Notification, OpenError> {
    let sealed = sealed.trim();
    if sealed.len() > MAX_SEALED_LEN {
        return Err(OpenError::TooLarge);
    }
    let raw = B64.decode(sealed).map_err(|_| OpenError::Malformed)?;
    let Some((&version, rest)) = raw.split_first() else {
        return Err(OpenError::Malformed);
    };
    if version != VERSION {
        return Err(OpenError::UnknownVersion(version));
    }
    if rest.len() < NONCE_LEN + TAG_LEN {
        return Err(OpenError::Malformed);
    }
    let (nonce, sealed) = rest.split_at(NONCE_LEN);
    let plain = Zeroizing::new(
        cipher(pairing_key)
            .decrypt(
                XNonce::from_slice(nonce),
                Payload {
                    msg: sealed,
                    aad: &aad(version),
                },
            )
            .map_err(|_| OpenError::Forged)?,
    );
    let notification: Notification =
        serde_json::from_slice(&plain).map_err(|_| OpenError::Malformed)?;
    if notification.seq == 0 {
        return Err(OpenError::Malformed);
    }
    guard.accept(notification.seq)?;
    Ok(notification)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::random_key;

    fn note(seq: u64) -> Notification {
        Notification {
            seq,
            at: 1_795_000_000,
            kind: "permission".into(),
            title: "tests-e2e attend une autorisation".into(),
            body: "cargo nextest run --workspace · Escouade · Travail".into(),
            target: Some("permission:a1b2c3".into()),
        }
    }

    #[test]
    fn the_guard_takes_each_seq_once_in_any_order() {
        let mut g = ReplayGuard::new();
        for seq in [3, 1, 2, 10, 5] {
            g.accept(seq).unwrap();
        }
        for seq in [1, 2, 3, 5, 10] {
            assert_eq!(g.accept(seq), Err(OpenError::Replayed(seq)));
        }
        g.accept(4).unwrap();
        g.accept(10 + WINDOW - 1).unwrap();
        // 10 is now 63 below the highest: still in the window, still remembered.
        assert_eq!(g.accept(10), Err(OpenError::Replayed(10)));
        assert_eq!(g.accept(9), Err(OpenError::TooOld(9)));
        g.accept(1000).unwrap();
        assert_eq!(
            g.accept(10 + WINDOW - 1),
            Err(OpenError::TooOld(10 + WINDOW - 1))
        );
        assert_eq!(g.highest(), 1000);
    }

    #[test]
    fn the_guard_keeps_its_state_and_refuses_a_bad_one() {
        let mut g = ReplayGuard::new();
        for seq in [7, 2, 9] {
            g.accept(seq).unwrap();
        }
        let state = g.to_state();
        let mut back = ReplayGuard::from_state(&state).unwrap();
        assert_eq!(back, g);
        assert_eq!(back.accept(7), Err(OpenError::Replayed(7)));
        back.accept(8).unwrap();
        assert_eq!(ReplayGuard::from_state("").unwrap(), ReplayGuard::new());
        assert_eq!(
            ReplayGuard::from_state(&ReplayGuard::new().to_state()).unwrap(),
            ReplayGuard::new()
        );
        for bad in [
            "2.9.0000000000000001",
            "1.9",
            "1.x.0000000000000001",
            "1.9.1",
            "1.9.000000000000000g",
            "1.9.0000000000000002",
            "1.0.0000000000000001",
            "1.3.00000000000000ff",
            "1.9.0000000000000001.0",
        ] {
            assert_eq!(ReplayGuard::from_state(bad), Err(BadGuardState), "{bad}");
        }
    }

    #[test]
    fn seal_refuses_seq_zero_and_too_long() {
        let key = random_key();
        assert_eq!(seal(&key, &note(0)), Err(SealError::ZeroSeq));
        let mut long = note(1);
        long.body = "x".repeat(MAX_SEALED_LEN);
        assert_eq!(seal(&key, &long), Err(SealError::TooLarge));
        // The longest body that fits does fit.
        let fixed = seal(
            &key,
            &Notification {
                body: String::new(),
                ..note(1)
            },
        )
        .unwrap()
        .len();
        let room = (MAX_SEALED_LEN - fixed) * 3 / 4 - 2;
        let fits = Notification {
            body: "x".repeat(room),
            ..note(1)
        };
        assert!(seal(&key, &fits).unwrap().len() <= MAX_SEALED_LEN);
    }

    #[test]
    fn the_nonce_is_fresh_each_time() {
        let key = random_key();
        assert_ne!(seal(&key, &note(1)).unwrap(), seal(&key, &note(1)).unwrap());
    }

    #[test]
    fn an_authentic_text_that_is_not_a_notification_is_malformed() {
        let key = random_key();
        let seal_raw = |plain: &[u8]| {
            let nonce = [7u8; NONCE_LEN];
            let ct = cipher(&key)
                .encrypt(
                    XNonce::from_slice(&nonce),
                    Payload {
                        msg: plain,
                        aad: &aad(VERSION),
                    },
                )
                .unwrap();
            B64.encode([&[VERSION][..], &nonce, &ct].concat())
        };
        let mut g = ReplayGuard::new();
        assert_eq!(
            open(&key, &seal_raw(b"not json"), &mut g),
            Err(OpenError::Malformed)
        );
        let zero = serde_json::to_vec(&note(0)).unwrap();
        assert_eq!(
            open(&key, &seal_raw(&zero), &mut g),
            Err(OpenError::Malformed)
        );
        // Unknown fields are ignored, target may be missing.
        let extra = br#"{"seq":4,"at":1,"kind":"inbox","title":"t","body":"b","later":true}"#;
        let n = open(&key, &seal_raw(extra), &mut g).unwrap();
        assert_eq!((n.seq, n.target), (4, None));
    }
}
