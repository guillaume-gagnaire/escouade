//! The team's signing key (TS-14, `docs/TEAM-SERVER.md` §7, choice C13-A): a minisign pair (Ed25519) of its own,
//! made on an administrator's machine, never on the server, kept off line under a password, with a backup key made
//! with it. What it signs is read by minisign's own tools: its public key is a `.pub` line (`Ed` + id + key), its
//! signatures `.minisig` files in the prehashed form (`ED`, BLAKE2b-512 of the bytes), a trusted comment signed with
//! them, as the public store's (`store_keys.rs`).
//!
//! - **Its secret half** never leaves the machine in the clear: [`SigningKey::seal`] writes it under a password
//!   (PBKDF2-HMAC-SHA256, [`ROUNDS`] rounds, a salt of its own; XChaCha20-Poly1305, the key's id as associated data),
//!   [`SigningKey::open`] reads it back. A wrong password, or a file changed, opens nothing.
//! - **Two pairs**: the key of every day and the backup key, both declared to the team ([`crate::plugins`]). The backup
//!   key is given once, as a file to keep elsewhere (a USB stick in a drawer); the app keeps only the sealed key of
//!   every day. A revocation list signed by the backup key can revoke the other (`keys_revoked`).
//! - **No key in the repository, nor in the tests**: a test makes its own ([`SigningKey::generate`]).

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use blake2::{Blake2b512, Digest};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::Error;

/// The rounds of PBKDF2 a password goes through (OWASP's advice for HMAC-SHA256, 2023).
pub const ROUNDS: u32 = 600_000;
/// The fewest a sealed key may say: below, the file was not made by the app.
pub const MIN_ROUNDS: u32 = 1_000;
/// The shortest password of a team's key.
pub const MIN_PASSWORD: usize = 12;

/// What the first line of a sealed key says.
const SEALED_COMMENT: &str = "untrusted comment: escouade team signing key";

/// A pair of the team's signing key, opened.
pub struct SigningKey {
    /// minisign's key number: 8 random bytes.
    number: [u8; 8],
    secret: ed25519_dalek::SigningKey,
}

/// A sealed key, as written in its file.
#[derive(Serialize, Deserialize)]
struct Sealed {
    v: u32,
    /// The key's number (hex), its associated data.
    id: String,
    kdf: String,
    rounds: u32,
    salt: String,
    nonce: String,
    sealed: String,
}

fn random<const N: usize>() -> Result<[u8; N], Error> {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).map_err(|e| Error::Encoding(e.to_string()))?;
    Ok(b)
}

/// minisign's way of writing a key's id: its 8 bytes from the last, in capitals.
fn id_of(number: &[u8; 8]) -> String {
    number.iter().rev().map(|b| format!("{b:02X}")).collect()
}

fn derive(password: &str, salt: &[u8], rounds: u32) -> Zeroizing<[u8; 32]> {
    let mut key = Zeroizing::new([0u8; 32]);
    pbkdf2::pbkdf2_hmac::<sha2_10::Sha256>(password.as_bytes(), salt, rounds, key.as_mut());
    key
}

impl SigningKey {
    /// A new pair, drawn on this machine.
    pub fn generate() -> Result<SigningKey, Error> {
        let seed = Zeroizing::new(random::<32>()?);
        Ok(SigningKey {
            number: random::<8>()?,
            secret: ed25519_dalek::SigningKey::from_bytes(&seed),
        })
    }

    /// Its public key, as the second line of a minisign `.pub` file.
    pub fn public(&self) -> String {
        let mut raw = Vec::with_capacity(42);
        raw.extend_from_slice(b"Ed");
        raw.extend_from_slice(&self.number);
        raw.extend_from_slice(self.secret.verifying_key().as_bytes());
        B64.encode(raw)
    }

    /// Its id, as minisign prints it (16 hexadecimal capitals).
    pub fn id(&self) -> String {
        id_of(&self.number)
    }

    /// The `.pub` file of its public key.
    pub fn public_file(&self) -> String {
        format!(
            "untrusted comment: minisign public key {}\n{}\n",
            self.id(),
            self.public()
        )
    }

    /// A `.minisig` of `bytes` with the trusted comment `comment` (one line), in the prehashed form.
    pub fn sign(&self, bytes: &[u8], comment: &str) -> Result<String, Error> {
        if comment.contains('\n') || comment.contains('\r') {
            return Err(Error::Refused("a trusted comment is one line".into()));
        }
        let hash = Blake2b512::digest(bytes);
        let signature = self.secret.sign(&hash).to_bytes();
        let mut line = Vec::with_capacity(74);
        line.extend_from_slice(b"ED");
        line.extend_from_slice(&self.number);
        line.extend_from_slice(&signature);
        let mut global = Vec::with_capacity(64 + comment.len());
        global.extend_from_slice(&signature);
        global.extend_from_slice(comment.as_bytes());
        let global = self.secret.sign(&global).to_bytes();
        Ok(format!(
            "untrusted comment: signature from escouade team key {}\n{}\ntrusted comment: {comment}\n{}\n",
            self.id(),
            B64.encode(line),
            B64.encode(global)
        ))
    }

    /// Its secret half under `password`, as the file the app keeps (or the backup key's, to keep elsewhere).
    pub fn seal(&self, password: &str) -> Result<String, Error> {
        self.seal_with(password, ROUNDS)
    }

    /// [`SigningKey::seal`] with fewer rounds: what a test does, to stay quick.
    #[doc(hidden)]
    pub fn seal_with(&self, password: &str, rounds: u32) -> Result<String, Error> {
        if password.chars().count() < MIN_PASSWORD {
            return Err(Error::Refused(format!(
                "a password of {MIN_PASSWORD} characters at least"
            )));
        }
        let salt = random::<16>()?;
        let nonce = random::<24>()?;
        let key = derive(password, &salt, rounds);
        let cipher = XChaCha20Poly1305::new(key.as_ref().into());
        let seed = Zeroizing::new(self.secret.to_bytes());
        let sealed = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: seed.as_ref(),
                    aad: &self.number,
                },
            )
            .map_err(|_| Error::Encoding("the key could not be sealed".into()))?;
        let file = Sealed {
            v: 1,
            id: hex::encode(self.number),
            kdf: "pbkdf2-hmac-sha256".into(),
            rounds,
            salt: B64.encode(salt),
            nonce: B64.encode(nonce),
            sealed: B64.encode(sealed),
        };
        let json = serde_json::to_vec(&file).map_err(|e| Error::Encoding(e.to_string()))?;
        Ok(format!(
            "{SEALED_COMMENT} {}\n{}\n",
            self.id(),
            B64.encode(json)
        ))
    }

    /// Opens a sealed key with `password`. A wrong password, or a file changed, opens nothing.
    pub fn open(text: &str, password: &str) -> Result<SigningKey, Error> {
        let bad = || Error::Refused("this is not a sealed key of a team".into());
        let line = text
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with("untrusted comment:"))
            .ok_or_else(bad)?;
        let json = B64.decode(line).map_err(|_| bad())?;
        let file: Sealed = serde_json::from_slice(&json).map_err(|_| bad())?;
        if file.v != 1 || file.kdf != "pbkdf2-hmac-sha256" || file.rounds < MIN_ROUNDS {
            return Err(bad());
        }
        let mut number = [0u8; 8];
        hex::decode_to_slice(&file.id, &mut number).map_err(|_| bad())?;
        let salt = B64.decode(&file.salt).map_err(|_| bad())?;
        let nonce = B64.decode(&file.nonce).map_err(|_| bad())?;
        if nonce.len() != 24 {
            return Err(bad());
        }
        let sealed = B64.decode(&file.sealed).map_err(|_| bad())?;
        let key = derive(password, &salt, file.rounds);
        let cipher = XChaCha20Poly1305::new(key.as_ref().into());
        let seed = Zeroizing::new(
            cipher
                .decrypt(
                    XNonce::from_slice(&nonce),
                    Payload {
                        msg: &sealed,
                        aad: &number,
                    },
                )
                .map_err(|_| {
                    Error::Refused("wrong password: the team's key does not open".into())
                })?,
        );
        let seed: [u8; 32] = seed.as_slice().try_into().map_err(|_| bad())?;
        let seed = Zeroizing::new(seed);
        Ok(SigningKey {
            number,
            secret: ed25519_dalek::SigningKey::from_bytes(&seed),
        })
    }
}

/// The id of a public key (its `.pub` line), as minisign prints it; None when it is not one.
pub fn key_id(public: &str) -> Option<String> {
    let raw = B64.decode(public.trim()).ok()?;
    if raw.len() != 42 || &raw[..2] != b"Ed" {
        return None;
    }
    let number: [u8; 8] = raw[2..10].try_into().ok()?;
    Some(id_of(&number))
}

/// Whether `public` is a minisign public key the app can check with.
pub fn is_public_key(public: &str) -> bool {
    key_id(public).is_some() && minisign_verify::PublicKey::from_base64(public.trim()).is_ok()
}

/// Which of `keys` signed `bytes` with `minisig` (the prehashed form only), and its trusted comment.
pub fn signed_by(keys: &[String], bytes: &[u8], minisig: &str) -> Option<(String, String)> {
    let sig = minisign_verify::Signature::decode(minisig).ok()?;
    keys.iter().find_map(|k| {
        let pk = minisign_verify::PublicKey::from_base64(k.trim()).ok()?;
        pk.verify(bytes, &sig, false).ok()?;
        Some((key_id(k)?, sig.trusted_comment().to_owned()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "un mot de passe de test";

    #[test]
    fn its_signatures_are_minisign_signatures() {
        let key = SigningKey::generate().unwrap();
        let other = SigningKey::generate().unwrap();
        assert_ne!(key.public(), other.public());
        assert!(is_public_key(&key.public()));
        assert_eq!(key_id(&key.public()).as_deref(), Some(key.id().as_str()));
        let sig = key
            .sign(b"des octets", "escouade-team-v1 fr.acme.x@1.0.0")
            .unwrap();
        let pk = minisign_verify::PublicKey::from_base64(&key.public()).unwrap();
        let decoded = minisign_verify::Signature::decode(&sig).unwrap();
        pk.verify(b"des octets", &decoded, false)
            .expect("minisign reads it");
        assert!(pk.verify(b"d'autres octets", &decoded, false).is_err());
        assert_eq!(
            signed_by(&[other.public(), key.public()], b"des octets", &sig),
            Some((key.id(), "escouade-team-v1 fr.acme.x@1.0.0".into()))
        );
        assert_eq!(signed_by(&[other.public()], b"des octets", &sig), None);
        assert!(key.sign(b"x", "two\nlines").is_err());
    }

    #[test]
    fn its_secret_half_opens_with_its_password_alone() {
        let key = SigningKey::generate().unwrap();
        assert!(key.seal("court").is_err(), "a password too short");
        let sealed = key.seal_with(PASSWORD, MIN_ROUNDS).unwrap();
        assert!(!sealed.contains(&B64.encode(key.secret.to_bytes())));
        let opened = SigningKey::open(&sealed, PASSWORD).unwrap();
        assert_eq!(opened.public(), key.public());
        let e = SigningKey::open(&sealed, "un autre mot de passe")
            .err()
            .unwrap();
        assert!(e.to_string().contains("wrong password"), "{e}");
        // A file whose id was changed opens nothing: the id is its associated data.
        let line = sealed.lines().nth(1).unwrap();
        let mut file: serde_json::Value =
            serde_json::from_slice(&B64.decode(line).unwrap()).unwrap();
        file["id"] = serde_json::Value::String("00".repeat(8));
        let changed = format!("{SEALED_COMMENT}\n{}\n", B64.encode(file.to_string()));
        assert!(SigningKey::open(&changed, PASSWORD).is_err());
        file["rounds"] = serde_json::json!(1);
        let weak = format!("{SEALED_COMMENT}\n{}\n", B64.encode(file.to_string()));
        assert!(SigningKey::open(&weak, PASSWORD).is_err(), "too few rounds");
        assert!(SigningKey::open("n'importe quoi", PASSWORD).is_err());
    }
}
