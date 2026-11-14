//! ESC-445 : le pont vers le Rust, le même code pour les quatre maquettes.
//!
//! Le téléphone reçoit d'un relais un push chiffré de bout en bout, et le déchiffre lui-même avec la clé de l'appairage
//! (`docs/REMOTE.md`, « Les notifications push »). Ici, XChaCha20-Poly1305 : une clé de 32 octets, un nonce de 24 octets
//! tiré au hasard et mis en tête du message scellé. Les clés et les messages vont en base64 URL sans remplissage, comme
//! dans le QR code de l'appairage.

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine as _;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub const NONCE_LEN: usize = 24;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error))]
pub enum CoreError {
    #[error("la clé n'est pas 32 octets en base64")]
    BadKey,
    #[error("le message scellé n'est pas du base64, ou il est trop court")]
    BadInput,
    #[error("le message ne s'ouvre pas avec cette clé (altéré, ou une autre clé)")]
    Decrypt,
}

fn cipher(key_b64: &str) -> Result<XChaCha20Poly1305, CoreError> {
    let key = B64.decode(key_b64.trim()).map_err(|_| CoreError::BadKey)?;
    if key.len() != 32 {
        return Err(CoreError::BadKey);
    }
    XChaCha20Poly1305::new_from_slice(&key).map_err(|_| CoreError::BadKey)
}

/// La version de ce code, pour que chaque maquette montre qu'elle l'appelle bien.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn core_version() -> String {
    format!("escouade-mobile-core {}", env!("CARGO_PKG_VERSION"))
}

/// Une clé neuve, tirée au hasard (en base64).
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn new_key() -> String {
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).expect("aléa du système");
    B64.encode(key)
}

/// Scelle `plaintext` : nonce (24 octets) puis texte chiffré et étiquette, en base64.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn seal(key_b64: String, plaintext: String) -> Result<String, CoreError> {
    let c = cipher(&key_b64)?;
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce).expect("aléa du système");
    let ct = c.encrypt(XNonce::from_slice(&nonce), plaintext.as_bytes()).map_err(|_| CoreError::Decrypt)?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&ct);
    Ok(B64.encode(out))
}

/// Ouvre un message scellé par `seal` (ou par le faux serveur, en JavaScript).
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn open(key_b64: String, sealed_b64: String) -> Result<String, CoreError> {
    let c = cipher(&key_b64)?;
    let raw = B64.decode(sealed_b64.trim()).map_err(|_| CoreError::BadInput)?;
    if raw.len() < NONCE_LEN + 16 {
        return Err(CoreError::BadInput);
    }
    let (nonce, ct) = raw.split_at(NONCE_LEN);
    let pt = c.decrypt(XNonce::from_slice(nonce), ct).map_err(|_| CoreError::Decrypt)?;
    String::from_utf8(pt).map_err(|_| CoreError::Decrypt)
}

/// `n` allers-retours `seal` puis `open` d'un texte de `size` octets, faits dans le Rust : le temps (µs) sert de repère
/// pour lire le coût du pont, que chaque maquette mesure en appelant `open` `n` fois de son côté.
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn bench_roundtrips(n: u32, size: u32) -> u64 {
    let key = new_key();
    let text = "x".repeat(size as usize);
    let start = std::time::Instant::now();
    for _ in 0..n {
        let s = seal(key.clone(), text.clone()).unwrap();
        let _ = open(key.clone(), s).unwrap();
    }
    start.elapsed().as_micros() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La clé et le message scellé de `shared/fixture/vectors.json`, faits par le faux serveur (JavaScript) : le Rust
    /// les ouvre, donc les deux implémentations s'accordent.
    const KEY: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";

    #[test]
    fn seal_then_open() {
        let s = seal(KEY.into(), "Agent « api » attend : cargo test".into()).unwrap();
        assert_eq!(open(KEY.into(), s).unwrap(), "Agent « api » attend : cargo test");
    }

    #[test]
    fn opens_what_javascript_sealed() {
        let v: &str = include_str!("../../fixture/vectors.json");
        // Lecture sans dépendance : "sealed": "…", "plain": "…".
        let field = |name: &str| {
            let at = v.find(&format!("\"{name}\": \"")).unwrap() + name.len() + 5;
            v[at..at + v[at..].find('"').unwrap()].to_string()
        };
        assert_eq!(field("key"), KEY);
        assert_eq!(open(KEY.into(), field("sealed")).unwrap(), field("plain"));
    }

    #[test]
    fn wrong_key_or_tampered() {
        let s = seal(KEY.into(), "secret".into()).unwrap();
        assert_eq!(open(new_key(), s.clone()), Err(CoreError::Decrypt));
        let mut raw = B64.decode(&s).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 1;
        assert_eq!(open(KEY.into(), B64.encode(raw)), Err(CoreError::Decrypt));
        assert_eq!(open("court".into(), s), Err(CoreError::BadKey));
        assert_eq!(open(KEY.into(), "AAAA".into()), Err(CoreError::BadInput));
    }

    #[test]
    fn bench_runs() {
        assert!(bench_roundtrips(10, 4096) > 0);
    }
}
