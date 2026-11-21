//! The 32-byte keys the phone is given at pairing (MO-4's QR code): its WireGuard key, the core's public key, the
//! pairing key of the notifications. WireGuard writes its keys in standard base64 (`wg genkey`), the rest of the QR
//! code in URL-safe base64 without padding; both are read, so that neither side has to care.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use zeroize::Zeroizing;

/// A key that is not 32 bytes of base64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a 32-byte key in base64")]
pub struct BadKey;

/// Reads a 32-byte key, in either base64 (with or without padding, standard or URL-safe). The bytes are wiped when
/// dropped.
pub fn decode_key(text: &str) -> Result<Zeroizing<[u8; 32]>, BadKey> {
    let text = text.trim();
    let raw = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(text.trim_end_matches('='))
            .or_else(|_| STANDARD.decode(text))
            .map_err(|_| BadKey)?,
    );
    let mut key = Zeroizing::new([0u8; 32]);
    if raw.len() != key.len() {
        return Err(BadKey);
    }
    key.copy_from_slice(&raw);
    Ok(key)
}

/// A key in URL-safe base64 without padding, as the QR code writes it.
pub fn encode_key(key: &[u8; 32]) -> String {
    URL_SAFE_NO_PAD.encode(key)
}

/// A fresh random key (for the tests, and for whoever makes a pairing).
pub fn random_key() -> Zeroizing<[u8; 32]> {
    let mut key = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(key.as_mut()).expect("the system's randomness");
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_alphabets_and_padding() {
        let key: [u8; 32] = std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(250));
        let standard = STANDARD.encode(key);
        let url = URL_SAFE_NO_PAD.encode(key);
        assert_ne!(
            standard.trim_end_matches('='),
            url,
            "the key must hold + or / to tell the alphabets apart"
        );
        assert_eq!(*decode_key(&standard).unwrap(), key);
        assert_eq!(*decode_key(&url).unwrap(), key);
        assert_eq!(*decode_key(&format!("  {url}\n")).unwrap(), key);
        assert_eq!(encode_key(&key), url);
    }

    #[test]
    fn refuses_what_is_not_32_bytes() {
        assert_eq!(decode_key("").unwrap_err(), BadKey);
        assert_eq!(decode_key("court").unwrap_err(), BadKey);
        assert_eq!(
            decode_key(&URL_SAFE_NO_PAD.encode([1u8; 31])).unwrap_err(),
            BadKey
        );
        assert_eq!(
            decode_key(&URL_SAFE_NO_PAD.encode([1u8; 33])).unwrap_err(),
            BadKey
        );
        assert_eq!(decode_key("not base64 at all!").unwrap_err(), BadKey);
    }

    #[test]
    fn random_keys_differ() {
        assert_ne!(*random_key(), *random_key());
    }
}
