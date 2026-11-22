use super::*;

// Made by store/tests/fake/minisign.mjs (a key of a test, never the store's), as the store's
// review signs: the app and the store read the same signatures.
const KEY: &str = "RWSdD12p0w147Es0or2M3l5uzT4vcBno4LQIrfq7geBz2SWuUFka1+MD";
const JSON: &str = r#"{"format":1,"id":"dev.exemple.slack","version":"1.2.0"}"#;
const GOOD: &str = "untrusted comment: signature from minisign secret key\nRUSdD12p0w147Bgh0ETL38EK152D4z9+O8rxLdMPtAIFnqKKT9dil/krYuno8cC1zRL7VNJ+ACvVdZBfX1ItjS0KKPBCimtTywQ=\ntrusted comment: escouade-verified-v1 dev.exemple.slack@1.2.0 2026-10-29\nI5M6mYLq/SLrR885/61tWzWk1oQc5hO5wZxNVwiXsTonCGw5DbV8Tg3m/jtBT5DitE82TMqgHeR6Q/tnNp0zBQ==\n";
const OTHER: &str = "untrusted comment: signature from minisign secret key\nRUSdD12p0w147Bgh0ETL38EK152D4z9+O8rxLdMPtAIFnqKKT9dil/krYuno8cC1zRL7VNJ+ACvVdZBfX1ItjS0KKPBCimtTywQ=\ntrusted comment: escouade-verified-v1 dev.exemple.slack@1.3.0 2026-10-29\nLz9OPaP/5HWqMXFV6AKBFLDA4VFSQ6dCNDZVj3rfb7k1cc3PG/e37GOiO/6nNCajLW30/A2X8nsZnp7iA9QXAg==\n";

// Three keys of a test (store/tests/fake/minisign.mjs), one for each role, and what each signed
// (ESC-592): `FH` the version.json of a plugin of the first hand, `TP` of a third party's, `LIST` a
// revocation list of serial 2.
const FH: &str = r#"{"format":1,"id":"dev.escouade.slack","version":"1.2.0"}"#;
const TP: &str = r#"{"format":1,"id":"dev.exemple.slack","version":"1.2.0"}"#;
const LIST: &str = r#"{"serial":2,"issued_at":"2026-11-22T00:00:00Z","entries":[]}"#;
const FIRST_HAND: &str = "RWRPVchoY6S6M1kqp2+HW+MB4jQ1arV1vJeNcaPL/v1xbLBnoazkD8OK";
const THIRD_PARTY: &str = "RWTOrJfmsILrzcZRvOK3MMHkiNwONNv9Mn1I1fDH+iWym/L8m7dYdM5U";
const BACKUP: &str = "RWSgntUOO9ibevnxWRsPJ9EzpH0+NCfg/o+9v/YwYfGK0j+ylEsYSAIM";
const FH_BY_FIRST: &str = "untrusted comment: signature from minisign secret key\nRURPVchoY6S6M0cuOBUKm2g4vFpWzveXAfMT4IQlBDbfCDXjS8e4IOvI2g/Q58m546WaDBkXxbnrtgXo/sU515/kC0LaX9JTSAo=\ntrusted comment: escouade-verified-v1 dev.escouade.slack@1.2.0 2026-11-22\ntD3ey7svB+sZhOZCrY4QzcGDnaCtcvz8SDOEFMtYwmspLgdwqqi6fsEryYg4GBHh/x7/AtUWmg5ioN3Ozdi7Cw==\n";
const FH_BY_THIRD: &str = "untrusted comment: signature from minisign secret key\nRUTOrJfmsILrzSTMmTQplA346fLHjx543I06SvlAbXLtAGcGkFE2fxGrCvRQQAg66no0DuXW4yc4jJWhl2s0p0fdOajqD/xlGQ8=\ntrusted comment: escouade-verified-v1 dev.escouade.slack@1.2.0 2026-11-22\nY6VIl8kMLCwakd8Ip84ahM94bGc5TSgOFaekPImVPkWyQdU8P9eCAisQOdP/mq98J1sKO5LOkOl7oG3xDN5tDA==\n";
const FH_BY_BACKUP: &str = "untrusted comment: signature from minisign secret key\nRUSgntUOO9ibejC2D+nnViBSm2pgRpe5f9jQeQ+0XRoBTT8hoTHHr9ujLgnbiBEiQFfa1XPLK3TErksvfaZt/S4Rf0v3zM+yJQ8=\ntrusted comment: escouade-verified-v1 dev.escouade.slack@1.2.0 2026-11-22\nrEodFvdRhQ90G0v+oVIqNNFrCwu8q4s9LPhGqsxVwowJeY+wS3H6hKXlJEgfOiUVGlSbG1iUWFKkKZEaVKiUAQ==\n";
const TP_BY_FIRST: &str = "untrusted comment: signature from minisign secret key\nRURPVchoY6S6MyJ0GFw9et5ljTPC0LeGIkNYYVYz9vzlIxsvKxg5EOHzdzbW6qTZWZlPlxDRMiF4tWwASBtUHdbtSimznalyjQE=\ntrusted comment: escouade-verified-v1 dev.exemple.slack@1.2.0 2026-11-22\nRABbvKyxGJTW2U+ckWIC9IqOR1TqZLmitApPAlSYV8H5iciSVLzkqm3r9uittUx1lQ39K+WP0xspNGORQ9bjAw==\n";
const TP_BY_THIRD: &str = "untrusted comment: signature from minisign secret key\nRUTOrJfmsILrzYr7ObxaiPtE2nqLgkQi3GGS3cyqU03G4Ynd4xVknjKLZSiFcX5B9hzefopbCq7hgH/A3G+nKyRn+CWQEf2W5wE=\ntrusted comment: escouade-verified-v1 dev.exemple.slack@1.2.0 2026-11-22\nsbqZAVaRl8v0WSLtGnqyzaHPo/2PzfE8+rSwCw1Bbt3JNmBMIdjJJxkN78ynwj/f9xgpAZZQiGIGgRTvPzQ/Bw==\n";
const TP_BY_BACKUP: &str = "untrusted comment: signature from minisign secret key\nRUSgntUOO9ibetP8upoC5GiFAMaxMcL+69UqMVRSr3ZmkBAuodiwA5rYouOFu+ks1bXDXpBCRvwGEx3mc/rGrJhjHSB1G7jV8AM=\ntrusted comment: escouade-verified-v1 dev.exemple.slack@1.2.0 2026-11-22\n8REVPPO+1ArvvTsXaAl0uXBdQ+HQoNiH+F2aYyxrjZsHn2405Ab6BfVhV/k4dv2tjwffnXMvV6XsdBI9xWb1Dg==\n";
const LIST_BY_FIRST: &str = "untrusted comment: signature from minisign secret key\nRURPVchoY6S6MxXqMJoLzp56Td/KwlKW6yMsswKVeLDFvkgIi6/R3RCJDsR2uk7u/pO5PML6vUe14QALysBH0CfAvW1sL8tdUg8=\ntrusted comment: escouade-revocations-v1 2 2026-11-22T00:00:00Z\nJ6//yP5NeiwvDztK0ujZK3XIyF/R3FCUfjZjXG7Qmn1lTfovh0XEyUgTiupKqf5RtgAT6TIbEBIzso34kosyAg==\n";
const LIST_BY_THIRD: &str = "untrusted comment: signature from minisign secret key\nRUTOrJfmsILrzY0azKOixP7nGZGi6o5MsGFuv/7v5z8X47rBMtdxvYSltUyF1/wubbTDNn5MS0dx0Me0FYco6AnfyswGHNrgQgM=\ntrusted comment: escouade-revocations-v1 2 2026-11-22T00:00:00Z\nQQCewcMiQIO+erNk1o5Sq6eMQBo4G2rW7xEC1PTUb2W2xP0sP26OXHjuTYkfAIG7G9TngQw/i1n6yIblDK7JDA==\n";
const LIST_BY_BACKUP: &str = "untrusted comment: signature from minisign secret key\nRUSgntUOO9ibetDjvVGPbGJpNMinc50lyTenRaMUT7BbMUgLTRgDVyaApI3cJjnvUvlBXGj8xG+dTZ1eJwRBh6OUdRZlyO0BsQU=\ntrusted comment: escouade-revocations-v1 2 2026-11-22T00:00:00Z\nxL+ig3qfiFo4gt67uMU7LmBLsCVwLcGDd7tQx+XaShsBpIqSSY3qAyq6ZHgWKKcwXAuZZsuZZgSzThY6QPWhCQ==\n";

// A seal the key of the first hand made on 2026-11-17, as store.escouade.dev serves it
// (`/api/v1/plugins/dev.escouade.acp/2.2.5/version.json` and its `.minisig`): one of the 119 seals
// of that key at ESC-592, all of `dev.escouade.*`.
const ACP: &str = r#"{"archive":{"sha256":"05c329e44739413d4fab5fc9d7e0831a19261c37a024015c711d6e6238b8c6e3","size":30799},"binaries":{},"commit":"97aa4cc3af1bfec458bc18ae828fa0344839350a","escouade":{"api":"1","min":"2.2.0"},"format":1,"id":"dev.escouade.acp","manifest_sha256":"a65e524c1bd8d5430fcbab668e11cddb2c450bce7ebd61abb9acbd054856bb3b","path":"plugins/acp","repository":"https://github.com/guillaume-gagnaire/escouade","tag":"plugins/acp/v2.2.5","version":"2.2.5","wasm":{"compiler":{"canon":"4d3e106abad9ae5683b33129dca6a943b3785c2c31e3fee4b92f4f98f692db11","lock":"8b6c9ad5f314a1eaae259f9b63d51b4a7dfb4a944fe71d3863147d4a8c83fe0d","rust":"1.98.0","target":"wasm32-wasip1"},"sha256":"19301680873f11f716bdb5a77cbff16bcfda82ab0600d851fbfe76037db002b3","size":942937}}"#;
const ACP_SEAL: &str = "untrusted comment: signature from minisign secret key\r\nRUTVvdz/aGRR4G5mLfG47JyrLPEuGqvUTIgVjlkzMY/SIdT+IQhXdH7tp7OVC37DtqrzHWdEH2vmH5PGl4B1L01zpOXFA+ei1Ao=\r\ntrusted comment: escouade-verified-v1 dev.escouade.acp@2.2.5 2026-11-17\r\nVV516KG4qLHOpwezqqCyEJ4Zm7Z7oLvDCGyKM6zVUGTOUoO6ZwsjwZJtbyQ9fXqLhn4WTG2eub6/SU0CoY+yDA==\r\n";

// The revocation list the key of the first hand signed on 2026-11-07 (serial 1, empty), as the
// store and escouade.dev served it at ESC-592 (`website/public/revocations.json` then).
const LEGACY_LIST: &str =
    "{\n  \"serial\": 1,\n  \"issued_at\": \"2026-11-07T23:37:24.390Z\",\n  \"entries\": []\n}\n";
const LEGACY_LIST_SEAL: &str = "untrusted comment: signature from minisign secret key\nRUTVvdz/aGRR4P3bymNfEWE2XZtHi7SCcC0/hRi5LDz3FexmYHob1XwGDXoRv6ENlo/ksNMVF/78rrrl+5k3rINTjyAalhgoqgw=\ntrusted comment: escouade-revocations-v1 1 2026-11-07T23:37:24.390Z\ncUJtL92xY/fYGRL1kCm3kLHKfwbVc0iSVEVRoEcvE457kilr3NOfWHJRO3eTmIB8fnlXwPmNj9NpDUn2GDJnBw==\n";

fn key(public: &str, roles: &[Role]) -> Key {
    Key {
        public: public.to_string(),
        roles: roles.to_vec(),
    }
}

/// The keys of the app, as `STORE_KEYS` gives them, with a test's key in each role.
fn roles() -> Vec<Key> {
    vec![
        key(FIRST_HAND, &[Role::FirstHand]),
        key(THIRD_PARTY, &[Role::ThirdParty, Role::Revocation]),
        key(BACKUP, &[Role::Backup]),
    ]
}

#[test]
fn a_seal_is_refused_while_the_app_embeds_no_key() {
    let none: Vec<Key> = Vec::new();
    assert_eq!(
        verified_with(&none, JSON.as_bytes(), GOOD, "dev.exemple.slack", "1.2.0"),
        Err(Refused::NoKey)
    );
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

/// The key of the first hand seals `dev.escouade.*` and nothing else, whoever holds it (SEC-03).
#[test]
fn the_first_hand_key_seals_nothing_out_of_dev_escouade() {
    let keys = roles();
    assert_eq!(
        seal_of(
            &keys,
            FH.as_bytes(),
            FH_BY_FIRST,
            "dev.escouade.slack",
            "1.2.0"
        ),
        Ok(SealKey {
            id: key_id(FIRST_HAND),
            role: Role::FirstHand
        })
    );
    assert_eq!(
        seal_of(
            &keys,
            TP.as_bytes(),
            TP_BY_FIRST,
            "dev.exemple.slack",
            "1.2.0"
        ),
        Err(Refused::OutOfRole {
            key: key_id(FIRST_HAND)
        })
    );
    // Alone, as the key of every day was before: refused all the same.
    let first = [key(FIRST_HAND, &[Role::FirstHand])];
    assert!(verified_with(
        &first,
        TP.as_bytes(),
        TP_BY_FIRST,
        "dev.exemple.slack",
        "1.2.0"
    )
    .is_err());
}

/// What installs a version (`store::check_seal`, the store's and the seed's) refuses it and says
/// why: nothing is installed.
#[test]
fn an_install_refuses_a_first_hand_seal_out_of_dev_escouade_with_its_reason() {
    use crate::i18n::Lang;
    let keys = roles();
    let said = crate::store::check_seal_with(
        &keys,
        TP.as_bytes(),
        TP_BY_FIRST.into(),
        "dev.exemple.slack",
        "1.2.0",
        Lang::Fr,
    )
    .unwrap_err();
    assert!(
        said.contains("première main") && said.contains("ne scelle que les plugins dev.escouade.*"),
        "{said}"
    );
    assert!(said.contains(&key_id(FIRST_HAND)), "{said}");
    let said = crate::store::check_seal_with(
        &keys,
        FH.as_bytes(),
        FH_BY_THIRD.into(),
        "dev.escouade.slack",
        "1.2.0",
        Lang::En,
    )
    .unwrap_err();
    assert!(said.contains("does not seal Escouade's plugins"), "{said}");
    assert_eq!(
        crate::store::check_seal_with(
            &keys,
            FH.as_bytes(),
            FH_BY_FIRST.into(),
            "dev.escouade.slack",
            "1.2.0",
            Lang::Fr,
        ),
        Ok(Some(FH_BY_FIRST.to_string()))
    );
}

/// The key of the third parties seals a third party's plugin, never one of the first hand.
#[test]
fn the_third_party_key_seals_a_third_party_and_not_the_first_hand() {
    let keys = roles();
    assert_eq!(
        seal_of(
            &keys,
            TP.as_bytes(),
            TP_BY_THIRD,
            "dev.exemple.slack",
            "1.2.0"
        ),
        Ok(SealKey {
            id: key_id(THIRD_PARTY),
            role: Role::ThirdParty
        })
    );
    assert_eq!(
        seal_of(
            &keys,
            FH.as_bytes(),
            FH_BY_THIRD,
            "dev.escouade.slack",
            "1.2.0"
        ),
        Err(Refused::OutOfRole {
            key: key_id(THIRD_PARTY)
        })
    );
}

/// The backup seals both, as a key of the tests does (`ESCOUADE_STORE_TEST_KEY`, a bare key).
#[test]
fn the_backup_seals_every_plugin() {
    let keys = roles();
    for (json, sig, id) in [
        (FH, FH_BY_BACKUP, "dev.escouade.slack"),
        (TP, TP_BY_BACKUP, "dev.exemple.slack"),
    ] {
        assert_eq!(
            seal_of(&keys, json.as_bytes(), sig, id, "1.2.0"),
            Ok(SealKey {
                id: key_id(BACKUP),
                role: Role::Backup
            })
        );
    }
    assert_eq!(
        verified_with(
            &[BACKUP],
            TP.as_bytes(),
            TP_BY_BACKUP,
            "dev.exemple.slack",
            "1.2.0"
        ),
        Ok(())
    );
}

/// The revocation list holds signed by the key that signs lists or by the backup; the key of the
/// first hand signs none since ESC-592.
#[test]
fn the_revocation_list_is_signed_by_its_own_key() {
    let keys = roles();
    assert_eq!(
        revocations_signed(&keys, LIST.as_bytes(), LIST_BY_THIRD, 2),
        Ok(key_id(THIRD_PARTY))
    );
    assert_eq!(
        revocations_signed(&keys, LIST.as_bytes(), LIST_BY_BACKUP, 2),
        Ok(key_id(BACKUP))
    );
    assert_eq!(
        revocations_signed(&keys, LIST.as_bytes(), LIST_BY_FIRST, 2),
        Err(Refused::OutOfRole {
            key: key_id(FIRST_HAND)
        })
    );
    assert_eq!(
        revocations_signed(&keys, LIST.as_bytes(), LIST_BY_THIRD, 3),
        Err(Refused::OtherVersion)
    );
}

/// What the key of the first hand signed before ESC-592 holds with the keys the app embeds: its
/// seals, all of `dev.escouade.*` (one of them here), and the revocation list it signed.
#[test]
fn the_seals_and_the_list_signed_before_still_hold() {
    let keys = keys();
    assert_eq!(
        seal_of(&keys, ACP.as_bytes(), ACP_SEAL, "dev.escouade.acp", "2.2.5"),
        Ok(SealKey {
            id: "E0516468FFDCBDD5".into(),
            role: Role::FirstHand
        })
    );
    let json = LEGACY_LIST.as_bytes().to_vec();
    let sig = LEGACY_LIST_SEAL;
    assert_eq!(
        revocations_signed(&keys, &json, sig, 1),
        Ok("E0516468FFDCBDD5".into())
    );
    // Another list of that key is refused: one byte more is no longer the list it signed.
    let mut other = json.clone();
    other.push(b'\n');
    assert_eq!(
        revocations_signed(&keys, &other, sig, 1),
        Err(Refused::NotSigned)
    );
}

#[test]
fn the_roles_say_which_ids_they_seal() {
    assert!(Role::FirstHand.seals("dev.escouade.claude"));
    assert!(!Role::FirstHand.seals("dev.exemple.slack"));
    assert!(!Role::FirstHand.seals("dev.escouadex.slack"));
    assert!(!Role::FirstHand.seals("com.dev.escouade.slack"));
    assert!(Role::ThirdParty.seals("dev.exemple.slack"));
    assert!(!Role::ThirdParty.seals("dev.escouade.claude"));
    assert!(!Role::Revocation.seals("dev.exemple.slack"));
    assert!(Role::Backup.seals("dev.escouade.claude") && Role::Backup.seals("a.b"));
    assert!(!Role::FirstHand.signs_revocations() && !Role::ThirdParty.signs_revocations());
    assert!(Role::Revocation.signs_revocations() && Role::Backup.signs_revocations());
    assert_eq!(
        serde_json::to_string(&Role::FirstHand).unwrap(),
        "\"first-hand\""
    );
}

#[test]
fn the_embedded_keys_are_those_of_the_store_with_their_roles() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join("store/keys");
    let mut found: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|n| n.ends_with(".pub"))
        .map(|n| {
            let text = std::fs::read_to_string(dir.join(&n)).unwrap();
            let line = text
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with("untrusted comment:"))
                .unwrap_or_default()
                .to_string();
            assert!(
                roles_of_file(&n).is_some(),
                "store/keys/{n}: a file of no role"
            );
            (n, line)
        })
        .collect();
    found.sort();
    let mut embedded: Vec<(String, String)> = STORE_KEYS
        .iter()
        .map(|k| {
            assert_eq!(Some(k.roles), roles_of_file(k.file), "{}", k.file);
            (k.file.to_string(), k.public.to_string())
        })
        .collect();
    embedded.sort();
    assert_eq!(
        embedded, found,
        "STORE_KEYS must hold the keys of store/keys/*.pub, each with the roles of its file"
    );
    assert!(LEGACY_REVOCATION_LISTS.contains(&hex_sha256(LEGACY_LIST.as_bytes()).as_str()));
}
