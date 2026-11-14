//! Small things every part uses: opaque ids, randomness, the clock, token hashes.

use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// An opaque id of 16 bytes: a team, a device, a project or a file. The server makes the team's and the device's;
/// a client makes a project's and a file's (they are what it keys its encrypted objects by).
pub type Id = [u8; 16];

pub fn random<const N: usize>() -> [u8; N] {
    let mut out = [0u8; N];
    getrandom::fill(&mut out).expect("the system gives randomness");
    out
}

/// Seconds since the Unix epoch.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// What the base keeps of a token: its SHA-256, never the token.
pub fn token_hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// A token as it travels: 32 random bytes, base64url without padding.
pub fn new_token() -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random::<32>())
}

pub fn id_from(bytes: &[u8]) -> Option<Id> {
    bytes.try_into().ok()
}

pub fn id_from_hex(text: &str) -> Option<Id> {
    let mut out = [0u8; 16];
    hex::decode_to_slice(text, &mut out).ok()?;
    Some(out)
}
