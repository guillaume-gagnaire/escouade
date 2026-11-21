//! The sealed notifications, against the format as `docs/MOBILE.md` writes it (« Le format des notifications
//! scellées »): built here by hand from that text (HKDF, the header, the nonce, the AAD), not by the crate's `seal`,
//! so that a change of the format on one side alone fails. Then: opened with the right key; refused altered,
//! replayed, too old or under another key. The keys are drawn by the test.

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine as _;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use escouade_mobile::keys::random_key;
use escouade_mobile::notification::{
    open, seal, Notification, OpenError, ReplayGuard, MAX_SEALED_LEN,
};
use hkdf::Hkdf;
use sha2::Sha256;

/// The format, from the document: base64url(1 ‖ nonce ‖ XChaCha20-Poly1305(HKDF(key), nonce, "escouade-notification" ‖ 1, JSON)).
fn seal_by_the_book(pairing_key: &[u8; 32], json: &str, nonce: [u8; 24]) -> String {
    let mut key = [0u8; 32];
    Hkdf::<Sha256>::new(None, pairing_key)
        .expand(b"escouade-mobile/notification/v1", &mut key)
        .unwrap();
    let ct = XChaCha20Poly1305::new(&key.into())
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: json.as_bytes(),
                aad: b"escouade-notification\x01",
            },
        )
        .unwrap();
    B64.encode([&[1u8][..], &nonce, &ct].concat())
}

fn open_by_the_book(pairing_key: &[u8; 32], sealed: &str) -> serde_json::Value {
    let raw = B64.decode(sealed).unwrap();
    assert_eq!(raw[0], 1, "the version byte");
    let mut key = [0u8; 32];
    Hkdf::<Sha256>::new(None, pairing_key)
        .expand(b"escouade-mobile/notification/v1", &mut key)
        .unwrap();
    let plain = XChaCha20Poly1305::new(&key.into())
        .decrypt(
            XNonce::from_slice(&raw[1..25]),
            Payload {
                msg: &raw[25..],
                aad: b"escouade-notification\x01",
            },
        )
        .unwrap();
    serde_json::from_slice(&plain).unwrap()
}

fn note(seq: u64) -> Notification {
    Notification {
        seq,
        at: 1_795_300_000,
        kind: "permission".into(),
        title: "tests-e2e attend une autorisation".into(),
        body: "cargo nextest run --workspace".into(),
        target: Some("permission:7f3a".into()),
    }
}

#[test]
fn a_notification_sealed_by_the_format_opens_with_the_right_key() {
    let key = random_key();
    let json = r#"{"seq":1,"at":1795300000,"kind":"question","title":"api pose une question","body":"Quelle base ?","target":"question:9"}"#;
    let mut guard = ReplayGuard::new();
    let n = open(&key, &seal_by_the_book(&key, json, [9u8; 24]), &mut guard).unwrap();
    assert_eq!(
        n,
        Notification {
            seq: 1,
            at: 1_795_300_000,
            kind: "question".into(),
            title: "api pose une question".into(),
            body: "Quelle base ?".into(),
            target: Some("question:9".into()),
        }
    );
    assert_eq!(guard.highest(), 1);
}

#[test]
fn what_the_crate_seals_is_the_format() {
    let key = random_key();
    let sealed = seal(&key, &note(5)).unwrap();
    let v = open_by_the_book(&key, &sealed);
    assert_eq!(v["seq"], 5);
    assert_eq!(v["title"], "tests-e2e attend une autorisation");
    assert_eq!(v["target"], "permission:7f3a");
    // Without a target, the field is left out.
    let v = open_by_the_book(
        &key,
        &seal(
            &key,
            &Notification {
                target: None,
                ..note(6)
            },
        )
        .unwrap(),
    );
    assert!(v.get("target").is_none());
    assert_eq!(
        open(&key, &sealed, &mut ReplayGuard::new()).unwrap(),
        note(5)
    );
}

#[test]
fn an_altered_notification_is_refused_and_moves_nothing() {
    let key = random_key();
    let sealed = seal(&key, &note(3)).unwrap();
    let raw = B64.decode(&sealed).unwrap();
    let mut guard = ReplayGuard::new();
    // A byte of the nonce, of the text, of the tag.
    for at in [1, 24, 25, raw.len() / 2, raw.len() - 1] {
        let mut bad = raw.clone();
        bad[at] ^= 0x20;
        assert_eq!(
            open(&key, &B64.encode(&bad), &mut guard),
            Err(OpenError::Forged),
            "byte {at}"
        );
    }
    // The version: unknown, or the AAD fails if one day a version 2 is decoded as 1.
    let mut v2 = raw.clone();
    v2[0] = 2;
    assert_eq!(
        open(&key, &B64.encode(&v2), &mut guard),
        Err(OpenError::UnknownVersion(2))
    );
    let mut ct = B64
        .decode(seal_by_the_book(
            &key,
            r#"{"seq":3,"at":0,"kind":"k","title":"t","body":"b"}"#,
            [1; 24],
        ))
        .unwrap();
    ct.truncate(30);
    assert_eq!(
        open(&key, &B64.encode(&ct), &mut guard),
        Err(OpenError::Malformed)
    );
    assert_eq!(
        open(&key, "pas du base64 !", &mut guard),
        Err(OpenError::Malformed)
    );
    assert_eq!(open(&key, "", &mut guard), Err(OpenError::Malformed));
    assert_eq!(
        open(&key, &"A".repeat(MAX_SEALED_LEN + 1), &mut guard),
        Err(OpenError::TooLarge)
    );
    // Text added after a good one.
    assert_eq!(
        open(&key, &format!("{sealed}AAAA"), &mut guard),
        Err(OpenError::Forged)
    );
    assert_eq!(guard, ReplayGuard::new(), "a refusal never moves the guard");
    assert_eq!(open(&key, &sealed, &mut guard).unwrap().seq, 3);
}

#[test]
fn a_notification_under_another_key_is_refused() {
    let key = random_key();
    let other = random_key();
    let mut guard = ReplayGuard::new();
    assert_eq!(
        open(&other, &seal(&key, &note(1)).unwrap(), &mut guard),
        Err(OpenError::Forged)
    );
    // The pairing key itself is never the cipher's key: HKDF stands between.
    let json = serde_json::to_string(&note(1)).unwrap();
    let raw_key_sealed = {
        let ct = XChaCha20Poly1305::new(key.as_ref().into())
            .encrypt(
                XNonce::from_slice(&[3u8; 24]),
                Payload {
                    msg: json.as_bytes(),
                    aad: b"escouade-notification\x01",
                },
            )
            .unwrap();
        B64.encode([&[1u8][..], &[3u8; 24], &ct].concat())
    };
    assert_eq!(
        open(&key, &raw_key_sealed, &mut guard),
        Err(OpenError::Forged)
    );
    assert_eq!(guard, ReplayGuard::new());
}

#[test]
fn a_replayed_or_too_old_notification_is_refused() {
    let key = random_key();
    let mut guard = ReplayGuard::new();
    let first = seal(&key, &note(10)).unwrap();
    open(&key, &first, &mut guard).unwrap();
    // The same text again, or the same seq sealed anew (another nonce): a replay.
    assert_eq!(open(&key, &first, &mut guard), Err(OpenError::Replayed(10)));
    assert_eq!(
        open(&key, &seal(&key, &note(10)).unwrap(), &mut guard),
        Err(OpenError::Replayed(10))
    );
    // Out of order is not a replay: FCM and APNs may deliver so.
    open(&key, &seal(&key, &note(12)).unwrap(), &mut guard).unwrap();
    open(&key, &seal(&key, &note(11)).unwrap(), &mut guard).unwrap();
    // Kept between two launches of the app.
    let mut later = ReplayGuard::from_state(&guard.to_state()).unwrap();
    assert_eq!(
        open(&key, &seal(&key, &note(11)).unwrap(), &mut later),
        Err(OpenError::Replayed(11))
    );
    // Far ahead, then one from before the window.
    open(&key, &seal(&key, &note(500)).unwrap(), &mut later).unwrap();
    assert_eq!(
        open(&key, &seal(&key, &note(13)).unwrap(), &mut later),
        Err(OpenError::TooOld(13))
    );
    assert_eq!(later.highest(), 500);
}
