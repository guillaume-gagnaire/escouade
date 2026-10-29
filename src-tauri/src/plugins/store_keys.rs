//! The seal « Vérifié par Escouade » of a plugin's version from the store (docs/STORE.md §4.5):
//! the review signs the version's `version.json` with minisign on the user's machine, its trusted
//! comment `escouade-verified-v1 <id>@<version> <date>`; the app checks it with the public keys it
//! embeds, the store's key and its backup, the same as `store/keys/*.pub`. The store cannot make a
//! seal: it has neither key.
//!
//! Until the user gives the two public keys (the procedure of docs/STORE.md §4.5), the list is
//! empty and every seal is refused. Installing from the store and showing the seal come with
//! STORE-9.

use minisign_verify::{PublicKey, Signature};

/// The public keys of the store's signatures, as the second line of their `.pub` files: the key
/// of every day, then the backup key. Empty until the user makes them: no version is verified.
pub const STORE_PUBLIC_KEYS: &[&str] = &[];

/// What the trusted comment of a version's signature starts with.
pub const VERIFIED_COMMENT: &str = "escouade-verified-v1";

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
}

/// Whether `minisig` makes version `version` of plugin `id` verified: the bytes of its
/// `version.json` signed by one of the embedded keys, the comment naming this id and this number.
#[cfg_attr(not(test), allow(dead_code))]
pub fn verified(
    version_json: &[u8],
    minisig: &str,
    id: &str,
    version: &str,
) -> Result<(), Refused> {
    verified_with(STORE_PUBLIC_KEYS, version_json, minisig, id, version)
}

/// `verified` with the keys given (the tests' own).
pub fn verified_with(
    keys: &[&str],
    version_json: &[u8],
    minisig: &str,
    id: &str,
    version: &str,
) -> Result<(), Refused> {
    if keys.is_empty() {
        return Err(Refused::NoKey);
    }
    let sig = Signature::decode(minisig).map_err(|_| Refused::Unreadable)?;
    let signed = keys.iter().any(|k| {
        PublicKey::from_base64(k).is_ok_and(|pk| pk.verify(version_json, &sig, false).is_ok())
    });
    if !signed {
        return Err(Refused::NotSigned);
    }
    let mut words = sig.trusted_comment().split(' ');
    let target = format!("{id}@{version}");
    match (words.next(), words.next(), words.next(), words.next()) {
        (Some(VERIFIED_COMMENT), Some(t), Some(_date), None) if t == target => Ok(()),
        _ => Err(Refused::OtherVersion),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Made by store/tests/fake/minisign.mjs (a key of a test, never the store's), as the store's
    // review signs: the app and the store read the same signatures.
    const KEY: &str = "RWSdD12p0w147Es0or2M3l5uzT4vcBno4LQIrfq7geBz2SWuUFka1+MD";
    const JSON: &str = r#"{"format":1,"id":"dev.exemple.slack","version":"1.2.0"}"#;
    const GOOD: &str = "untrusted comment: signature from minisign secret key\nRUSdD12p0w147Bgh0ETL38EK152D4z9+O8rxLdMPtAIFnqKKT9dil/krYuno8cC1zRL7VNJ+ACvVdZBfX1ItjS0KKPBCimtTywQ=\ntrusted comment: escouade-verified-v1 dev.exemple.slack@1.2.0 2026-10-29\nI5M6mYLq/SLrR885/61tWzWk1oQc5hO5wZxNVwiXsTonCGw5DbV8Tg3m/jtBT5DitE82TMqgHeR6Q/tnNp0zBQ==\n";
    const OTHER: &str = "untrusted comment: signature from minisign secret key\nRUSdD12p0w147Bgh0ETL38EK152D4z9+O8rxLdMPtAIFnqKKT9dil/krYuno8cC1zRL7VNJ+ACvVdZBfX1ItjS0KKPBCimtTywQ=\ntrusted comment: escouade-verified-v1 dev.exemple.slack@1.3.0 2026-10-29\nLz9OPaP/5HWqMXFV6AKBFLDA4VFSQ6dCNDZVj3rfb7k1cc3PG/e37GOiO/6nNCajLW30/A2X8nsZnp7iA9QXAg==\n";

    #[test]
    fn a_seal_is_refused_while_the_app_embeds_no_key() {
        assert!(STORE_PUBLIC_KEYS.is_empty() || STORE_PUBLIC_KEYS.len() == 2);
        if STORE_PUBLIC_KEYS.is_empty() {
            assert_eq!(
                verified(JSON.as_bytes(), GOOD, "dev.exemple.slack", "1.2.0"),
                Err(Refused::NoKey)
            );
        }
    }

    #[test]
    fn the_store_signature_of_this_version_passes_and_nothing_else() {
        let keys = [KEY];
        assert_eq!(
            verified_with(&keys, JSON.as_bytes(), GOOD, "dev.exemple.slack", "1.2.0"),
            Ok(())
        );
        let changed = JSON.replace("1.2.0", "1.2.1");
        assert_eq!(
            verified_with(
                &keys,
                changed.as_bytes(),
                GOOD,
                "dev.exemple.slack",
                "1.2.0"
            ),
            Err(Refused::NotSigned)
        );
        assert_eq!(
            verified_with(&keys, JSON.as_bytes(), GOOD, "dev.exemple.slack", "1.3.0"),
            Err(Refused::OtherVersion)
        );
        assert_eq!(
            verified_with(&keys, JSON.as_bytes(), GOOD, "dev.exemple.slak", "1.2.0"),
            Err(Refused::OtherVersion)
        );
        assert_eq!(
            verified_with(&keys, JSON.as_bytes(), OTHER, "dev.exemple.slack", "1.2.0"),
            Err(Refused::OtherVersion)
        );
        assert_eq!(
            verified_with(&keys, JSON.as_bytes(), "nope", "dev.exemple.slack", "1.2.0"),
            Err(Refused::Unreadable)
        );
        let stranger = ["RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3"];
        assert_eq!(
            verified_with(
                &stranger,
                JSON.as_bytes(),
                GOOD,
                "dev.exemple.slack",
                "1.2.0"
            ),
            Err(Refused::NotSigned)
        );
    }

    #[test]
    fn the_embedded_keys_are_those_of_the_store() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../store/keys");
        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
            .filter(|n| n.ends_with(".pub"))
            .collect();
        names.sort();
        let mut found: Vec<String> = names
            .iter()
            .map(|n| {
                let text = std::fs::read_to_string(dir.join(n)).unwrap();
                text.lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty() && !l.starts_with("untrusted comment:"))
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        found.sort();
        let mut embedded: Vec<String> = STORE_PUBLIC_KEYS.iter().map(|k| k.to_string()).collect();
        embedded.sort();
        assert_eq!(
            embedded, found,
            "STORE_PUBLIC_KEYS must hold the keys of store/keys/*.pub"
        );
    }
}
