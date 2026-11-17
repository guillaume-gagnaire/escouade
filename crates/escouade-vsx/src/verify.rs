//! A VSIX checked before anything of it is unpacked: its SHA-256 against the sum Open VSX gives, and Open VSX's signature.
//!
//! **What the signature is** (read on open-vsx.org, 2026-11-16): each version has a `.sigzip`, an archive of
//! `.signature.sig` (64 bytes: an Ed25519 signature of the VSIX's bytes, whole), `.signature.manifest` (the sums of its
//! entries) and an empty `.signature.p7s` (for VS Code's `vsce-sign`). Open VSX signs a version when it is published,
//! with a key of its own whose public half it serves (`/api/-/public-key/<id>`, SPKI PEM).
//!
//! **What it proves**: the bytes installed are those Open VSX received and published, unchanged on the way (its storage,
//! a mirror, a cache, a proxy that would serve another file). **What it does not prove**: that the extension is
//! harmless, nor that its author is who it says (that is the namespace « verified » of Open VSX, shown apart); nor
//! anything against Open VSX itself, whose key is read from it (over TLS).

use crate::{Code, Refusal, Result};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};
use std::io::Read;

/// The hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `bytes` checked against the `.sha256` file Open VSX serves (the hex sum, maybe followed by a name): the sum.
pub fn check_sum(bytes: &[u8], sum_file: &str) -> Result<String> {
    let expected = sum_file
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_lowercase();
    let got = sha256_hex(bytes);
    if expected.len() != 64 || expected != got {
        return Err(Refusal::new(
            Code::BadSum,
            tr!(
                "La somme SHA-256 de l’archive n’est pas celle qu’Open VSX donne : elle n’est pas installée.",
                "The archive’s SHA-256 is not the one Open VSX gives: it is not installed."
            ),
        ));
    }
    Ok(got)
}

/// The DER prefix of an Ed25519 public key in SPKI (RFC 8410): the 32 bytes of the key follow it.
const SPKI_ED25519: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

fn bad_signature() -> Refusal {
    Refusal::new(
        Code::BadSignature,
        tr!(
            "La signature d’Open VSX ne correspond pas à l’archive : elle n’est pas installée.",
            "Open VSX’s signature does not match the archive: it is not installed."
        ),
    )
}

/// An Ed25519 public key from its SPKI PEM (`-----BEGIN PUBLIC KEY-----`).
pub fn public_key(pem: &str) -> Result<VerifyingKey> {
    use base64::Engine;
    let body: String = pem
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("-----"))
        .collect();
    let der = base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .map_err(|_| bad_signature())?;
    let key: [u8; 32] = der
        .strip_prefix(&SPKI_ED25519[..])
        .and_then(|k| k.try_into().ok())
        .ok_or_else(bad_signature)?;
    VerifyingKey::from_bytes(&key).map_err(|_| bad_signature())
}

/// The signature in a `.sigzip` (its `.signature.sig`), the archive read within bounds.
pub fn signature_of(sigzip: &[u8]) -> Result<Signature> {
    let mut zip =
        zip::ZipArchive::new(std::io::Cursor::new(sigzip)).map_err(|_| bad_signature())?;
    let mut entry = zip.by_name(".signature.sig").map_err(|_| bad_signature())?;
    let mut sig = Vec::new();
    entry
        .by_ref()
        .take(65)
        .read_to_end(&mut sig)
        .map_err(|_| bad_signature())?;
    let sig: [u8; 64] = sig.try_into().map_err(|_| bad_signature())?;
    Ok(Signature::from_bytes(&sig))
}

/// `vsix` checked against the signature of `sigzip` with `key` (strictly: no malleable signature passes).
pub fn check_signature(vsix: &[u8], sigzip: &[u8], key: &VerifyingKey) -> Result<()> {
    let sig = signature_of(sigzip)?;
    key.verify_strict(vsix, &sig).map_err(|_| bad_signature())
}
