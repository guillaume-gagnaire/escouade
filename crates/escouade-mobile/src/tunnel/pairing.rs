//! The phone's half of a pairing (ESC-570, `docs/MOBILE.md` § 3.1, SR1-15): a key pair (X25519) made by the native side;
//! its public half goes to the core with the pairing code, the core answers the public half of one of its own, and each
//! side derives the device's token and the pairing key from the secret they share: HKDF-SHA256, no salt, `info`
//! [`TOKEN_INFO`] and [`KEY_INFO`] (`src-tauri/src/remoteapi/pairing.rs` derives the same). Neither is ever sent: the
//! WebView's JavaScript sees two public keys.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use boringtun::x25519::{PublicKey, StaticSecret};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::keys::{decode_key, random_key, BadKey};

/// HKDF's `info` of the device's token.
pub const TOKEN_INFO: &[u8] = b"escouade-mobile/device-token/v1";
/// HKDF's `info` of the pairing key (which seals the notifications, MO-6).
pub const KEY_INFO: &[u8] = b"escouade-mobile/pairing-key/v1";

/// A new key pair: its private half (base64, wiped when dropped) and its public half (standard base64, as the core
/// reads it).
pub fn new_key() -> (Zeroizing<String>, String) {
    let private = random_key();
    let public = PublicKey::from(&StaticSecret::from(*private));
    (
        Zeroizing::new(STANDARD.encode(*private)),
        STANDARD.encode(public.as_bytes()),
    )
}

/// The device's token (base64url, no padding) and the pairing key (standard base64), from the phone's private half and
/// the core's public half. Refused when a key is not 32 bytes, or the secret is none (a point of small order).
pub fn finish(
    private: &str,
    core_public: &str,
) -> Result<(Zeroizing<String>, Zeroizing<String>), BadKey> {
    let private = decode_key(private)?;
    let theirs = decode_key(core_public)?;
    let shared = StaticSecret::from(*private).diffie_hellman(&PublicKey::from(*theirs));
    if !shared.was_contributory() {
        return Err(BadKey);
    }
    let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut token = Zeroizing::new([0u8; 32]);
    let mut key = Zeroizing::new([0u8; 32]);
    hk.expand(TOKEN_INFO, token.as_mut()).map_err(|_| BadKey)?;
    hk.expand(KEY_INFO, key.as_mut()).map_err(|_| BadKey)?;
    Ok((
        Zeroizing::new(URL_SAFE_NO_PAD.encode(*token)),
        Zeroizing::new(STANDARD.encode(*key)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sides_derive_the_same_and_a_vector_holds_with_the_core() {
        let (a_private, a_public) = new_key();
        let (b_private, b_public) = new_key();
        let (t1, k1) = finish(&a_private, &b_public).unwrap();
        let (t2, k2) = finish(&b_private, &a_public).unwrap();
        assert_eq!((t1.as_str(), k1.as_str()), (t2.as_str(), k2.as_str()));
        // The vector `src-tauri/src/remoteapi_tests.rs` checks with the core's code: a change of one side fails.
        let phone = STANDARD.encode([1u8; 32]);
        let core = STANDARD.encode(PublicKey::from(&StaticSecret::from([2u8; 32])).as_bytes());
        let (token, key) = finish(&phone, &core).unwrap();
        assert_eq!(token.as_str(), VECTOR_TOKEN);
        assert_eq!(key.as_str(), VECTOR_KEY);
    }

    #[test]
    fn a_key_of_none_or_not_32_bytes_is_refused() {
        let (private, _) = new_key();
        assert!(finish(&private, &STANDARD.encode([0u8; 32])).is_err());
        assert!(finish(&private, "bm9wZQ==").is_err());
        assert!(finish("x", &STANDARD.encode([9u8; 32])).is_err());
    }

    /// What the phone [1; 32] and the core [2; 32] derive.
    const VECTOR_TOKEN: &str = "mX1HZhfIWYIZzkH1vuI6Ig8Ga_W-jTfxGzbHZeqlB5o";
    const VECTOR_KEY: &str = "qZxFr/xX1SpwFdIjtQhNHT2R4TiuZur629pvqH4Aoq8=";
}
