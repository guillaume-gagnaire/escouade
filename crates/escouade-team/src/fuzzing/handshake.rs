//! The handshake of the pairing by a short code (TS-3, `pairing.rs`): what the relay of the server carries between
//! the two devices, which the server can change.
//! - The bytes as a typed code: one that parses is shown and parses again the same, its slot of the alphabet.
//! - The new device's try, changed by the bytes (its SPAKE2 message and its confirmation): the device in the team
//!   takes it only as it was made with the right code, and its answer then opens with the new device's key alone.
//! - The bytes as a sealed answer: opened by no key that did not seal it.

use spake2::{Ed25519Group, Identity, Spake2};

use crate::pairing::{confirmation, open, PairCode, PairState, Pairing, ALPHABET, SIDE_A, SIDE_B};
use escouade_team_crypto::InviteLink;

fn link() -> InviteLink {
    InviteLink {
        server: "team.example.org".into(),
        id: [1; 16],
        secret: [2; 32],
        team: [3; 32],
        member: Some([4; 16]),
    }
}

pub fn handshake(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(code) = PairCode::parse(text) {
            assert_eq!(PairCode::parse(&code.shown()).expect("again"), code);
            assert!(code.slot.bytes().all(|b| ALPHABET.contains(&b)) && code.slot.len() == 3);
        }
    }
    let code = PairCode::parse("ABC4M2XZ").expect("a code");
    let (spake, first) = Pairing::start(&code);
    let mut pairing = Pairing {
        code: code.clone(),
        link: link(),
        expires_at: 0,
        spake: Some(spake),
        key: None,
        state: PairState::Waiting,
    };
    // The new device: the right code, or the one the first byte makes wrong.
    let wrong = data.first().is_some_and(|b| b % 4 == 0);
    let typed = if wrong {
        PairCode::parse("ABC4M2XY").expect("a code")
    } else {
        code.clone()
    };
    let (b, message) = Spake2::<Ed25519Group>::start_b(
        &typed.password(),
        &Identity::new(SIDE_A),
        &Identity::new(SIDE_B),
    );
    let key = b.finish(&first).expect("a key");
    let confirm = confirmation(&key);
    // What the relay hands on: the bytes change the message (first half) and the confirmation (second half).
    let half = data.len() / 2;
    let sent = super::alter(&message, &data[..half]);
    let said = super::alter(&confirm, &data[half..]);
    let taken = pairing.check_try(&sent, &said);
    if taken {
        assert!(!wrong, "a wrong code taken");
        assert_eq!(
            (&sent[..], &said[..]),
            (&message[..], &confirm[..]),
            "a try changed on the way and taken"
        );
        let sealed = pairing.sealed_answer().expect("an answer");
        let plain = open(&key, &code.slot, &sealed).expect("the new device opens it");
        assert_eq!(plain.as_slice(), link().to_url().as_bytes());
        assert!(
            open(&key, "ABD", &sealed).is_err(),
            "an answer opened under another slot"
        );
    } else {
        assert!(pairing.sealed_answer().is_none());
    }
    // The try is spent, whatever it was.
    assert!(!pairing.check_try(&message, &confirm), "a second try");
    // Raw bytes as an answer: no key opens them.
    let _ = open(&key, &code.slot, data);
}
