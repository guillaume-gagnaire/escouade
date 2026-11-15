//! « Ajouter un appareil » by a short code (TS-3, `docs/TEAM-SERVER.md` §3.4): two desktops of one member, without a
//! camera. The device in the team shows 8 characters; they are typed on the new device.
//!
//! The code is not a MAC key: 40 bits would fall to a server that tries them all offline. It is the password of a
//! PAKE (SPAKE2 on Ed25519, RustCrypto's `spake2`), whose messages the server's relay carries (`escouade-team-server`,
//! `pair.rs`):
//!
//! - the code: the **slot** (3 characters, chosen by the server, which routes by it) and the **secret** (5 characters,
//!   25 bits, drawn by the device in the team, which never reach the server);
//! - the device in the team starts SPAKE2 (side A) on the secret and opens the slot with its message;
//! - the new device reads it, plays side B, and leaves its message with a key confirmation: the slot takes **one try**;
//! - the device in the team derives the key, checks the confirmation: a wrong code closes the slot (a new code is
//!   needed), and nothing has left the device; the right one gets the answer, sealed under the key
//!   (XChaCha20-Poly1305): the invitation link of the member's new device (`Purpose::OwnDevice`);
//! - the new device opens it and joins by the link, as by the QR code.
//!
//! What the server learns: two SPAKE2 messages and a sealed answer, neither of which tests a code offline. A guess
//! (the server's, or anyone's who knows the slot) costs the slot, and has one chance in 2^25.

use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use escouade_team_crypto::invite::InviteLink;
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, Spake2};
use zeroize::Zeroizing;

use crate::client::{self, ClientError, Transport};
use crate::Error;

/// Crockford's base 32: no I, L, O, U (the server's alphabet of slots too).
pub const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
pub const SLOT_LEN: usize = 3;
pub const SECRET_LEN: usize = 5;

const SIDE_A: &[u8] = b"escouade/pair/v1/device-in-the-team";
const SIDE_B: &[u8] = b"escouade/pair/v1/new-device";

/// A short code: its slot and its secret.
#[derive(Clone, PartialEq, Eq)]
pub struct PairCode {
    pub slot: String,
    secret: Zeroizing<String>,
}

impl std::fmt::Debug for PairCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PairCode({}-…)", self.slot)
    }
}

/// A character as typed: upper case, `O` for `0`, `I` and `L` for `1`; spaces and dashes are left out.
fn normal(c: char) -> Option<char> {
    match c.to_ascii_uppercase() {
        ' ' | '-' | '\u{a0}' => None,
        'O' => Some('0'),
        'I' | 'L' => Some('1'),
        c => Some(c),
    }
}

impl PairCode {
    /// A new secret for the slot the server gave.
    pub fn new(slot: &str) -> PairCode {
        let mut bytes = [0u8; SECRET_LEN];
        getrandom::fill(&mut bytes).expect("the system gives randomness");
        let secret = bytes
            .iter()
            .map(|b| ALPHABET[(*b as usize) % 32] as char)
            .collect::<String>();
        PairCode {
            slot: slot.to_string(),
            secret: Zeroizing::new(secret),
        }
    }

    /// A code as typed: 8 characters of the alphabet, spaces and dashes aside, any case.
    pub fn parse(text: &str) -> Result<PairCode, Error> {
        let chars: String = text.chars().filter_map(normal).collect();
        if chars.len() != SLOT_LEN + SECRET_LEN || !chars.bytes().all(|b| ALPHABET.contains(&b)) {
            return Err(Error::Refused(
                "a code is 8 characters, letters and digits".into(),
            ));
        }
        Ok(PairCode {
            slot: chars[..SLOT_LEN].to_string(),
            secret: Zeroizing::new(chars[SLOT_LEN..].to_string()),
        })
    }

    /// As shown: `K7Q4-M2XZ`.
    pub fn shown(&self) -> String {
        let all = format!("{}{}", self.slot, self.secret.as_str());
        format!("{}-{}", &all[..4], &all[4..])
    }

    fn password(&self) -> Password {
        Password::new(self.secret.as_bytes())
    }
}

fn derive(key: &[u8], label: &[u8]) -> Zeroizing<[u8; 32]> {
    let mut h = Sha256::new();
    h.update(label);
    h.update([0]);
    h.update(key);
    Zeroizing::new(h.finalize().into())
}

fn confirmation(key: &[u8]) -> [u8; 32] {
    *derive(key, b"escouade/pair/v1/confirm")
}

fn seal(key: &[u8], slot: &str, plain: &[u8]) -> Vec<u8> {
    let k = derive(key, b"escouade/pair/v1/answer");
    let mut nonce = [0u8; 24];
    getrandom::fill(&mut nonce).expect("the system gives randomness");
    let mut out = nonce.to_vec();
    out.extend(
        XChaCha20Poly1305::new((&*k).into())
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: plain,
                    aad: slot.as_bytes(),
                },
            )
            .expect("XChaCha20-Poly1305 seals any answer"),
    );
    out
}

fn open(key: &[u8], slot: &str, sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    if sealed.len() < 24 {
        return Err(Error::WrongCode);
    }
    let k = derive(key, b"escouade/pair/v1/answer");
    XChaCha20Poly1305::new((&*k).into())
        .decrypt(
            XNonce::from_slice(&sealed[..24]),
            Payload {
                msg: &sealed[24..],
                aad: slot.as_bytes(),
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::WrongCode)
}

/// Where a pairing is, on the device in the team.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairState {
    /// The code is shown; no try yet.
    Waiting,
    /// The new device tried the right code, but the invitation's token is not registered yet: an administrator has
    /// to do it (this device is not one on the server). The answer goes as soon as it is.
    WaitingForToken,
    /// The answer went: the new device joins by it.
    Sent,
    /// A wrong code was typed: the slot is closed, a new code is needed.
    WrongCode,
}

/// A pairing under way on the device in the team ([`crate::Team::pair_start`]).
pub struct Pairing {
    pub code: PairCode,
    pub link: InviteLink,
    /// Seconds since the epoch.
    pub expires_at: u64,
    pub(crate) spake: Option<Spake2<Ed25519Group>>,
    pub(crate) key: Option<Zeroizing<Vec<u8>>>,
    pub state: PairState,
}

impl Pairing {
    /// Side A's start: its message for the slot.
    pub(crate) fn start(secret_holder: &PairCode) -> (Spake2<Ed25519Group>, Vec<u8>) {
        Spake2::<Ed25519Group>::start_a(
            &secret_holder.password(),
            &Identity::new(SIDE_A),
            &Identity::new(SIDE_B),
        )
    }

    /// The new device's try, read on the relay: the key, if its confirmation holds.
    pub(crate) fn check_try(&mut self, message: &[u8], confirm: &[u8]) -> bool {
        let Some(spake) = self.spake.take() else {
            return false;
        };
        let Ok(key) = spake.finish(message) else {
            return false;
        };
        if confirmation(&key).as_slice() != confirm {
            return false;
        }
        self.key = Some(Zeroizing::new(key));
        true
    }

    /// The answer, sealed under the key of the right code.
    pub(crate) fn sealed_answer(&self) -> Option<Vec<u8>> {
        let key = self.key.as_ref()?;
        Some(seal(key, &self.code.slot, self.link.to_url().as_bytes()))
    }
}

/// The new device: plays its one try with `code` on the server of `transport`, then waits up to `wait` for the
/// answer, and gives the invitation link it holds. A wrong code: [`Error::WrongCode`] (the device in the team closed
/// the slot); a code that had its try already: [`Error::CodeTried`].
pub async fn fetch_link(
    transport: &dyn Transport,
    code: &PairCode,
    wait: std::time::Duration,
) -> Result<InviteLink, Error> {
    let gone = |e: ClientError| match e.code() {
        Some("pair_gone") => Error::CodeGone,
        Some("pair_tried") => Error::CodeTried,
        _ => Error::Client(e),
    };
    let first = client::pair_message(transport, &code.slot)
        .await
        .map_err(gone)?;
    let (spake, message) = Spake2::<Ed25519Group>::start_b(
        &code.password(),
        &Identity::new(SIDE_A),
        &Identity::new(SIDE_B),
    );
    let key = Zeroizing::new(spake.finish(&first).map_err(|_| Error::WrongCode)?);
    client::pair_try(transport, &code.slot, message, confirmation(&key).to_vec())
        .await
        .map_err(gone)?;
    let until = std::time::Instant::now() + wait;
    loop {
        match client::pair_take(transport, &code.slot).await {
            Ok(Some(sealed)) => {
                let plain = open(&key, &code.slot, &sealed)?;
                let url = std::str::from_utf8(&plain).map_err(|_| Error::WrongCode)?;
                return Ok(InviteLink::parse(url)?);
            }
            Ok(None) => {}
            // Closed by the device in the team: the code was wrong.
            Err(e) if e.code() == Some("pair_gone") => return Err(Error::WrongCode),
            Err(e) => return Err(Error::Client(e)),
        }
        if std::time::Instant::now() >= until {
            return Err(Error::Refused(
                "the device in the team did not answer in time: is Escouade open on it?".into(),
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_typed_loosely_and_shown_in_two_halves() {
        let code = PairCode::new("K7Q");
        let shown = code.shown();
        assert_eq!(shown.len(), 9);
        assert_eq!(&shown[4..5], "-");
        assert_eq!(PairCode::parse(&shown.to_lowercase()).unwrap(), code);
        let typed = PairCode::parse(" k7q o-il zz ").unwrap();
        assert_eq!(
            (typed.slot.as_str(), typed.secret.as_str()),
            ("K7Q", "011ZZ")
        );
        assert!(PairCode::parse("K7Q4M2X").is_err());
        assert!(PairCode::parse("K7Q4M2XU").is_err());
        assert!(!format!("{code:?}").contains(code.secret.as_str()));
    }

    #[test]
    fn the_right_code_opens_the_answer_and_a_wrong_one_fails_the_confirmation() {
        let code = PairCode::new("ABC");
        let (spake, first) = Pairing::start(&code);
        let link = InviteLink {
            server: "team.example.org".into(),
            id: [1; 16],
            secret: [2; 32],
            team: [3; 32],
            member: Some([4; 16]),
        };
        let mut pairing = Pairing {
            code: code.clone(),
            link: link.clone(),
            expires_at: 0,
            spake: Some(spake),
            key: None,
            state: PairState::Waiting,
        };
        // The new device, with the right code.
        let (b, message) = Spake2::<Ed25519Group>::start_b(
            &code.password(),
            &Identity::new(SIDE_A),
            &Identity::new(SIDE_B),
        );
        let key = b.finish(&first).unwrap();
        assert!(pairing.check_try(&message, &confirmation(&key)));
        let sealed = pairing.sealed_answer().unwrap();
        let plain = open(&key, "ABC", &sealed).unwrap();
        assert_eq!(
            InviteLink::parse(std::str::from_utf8(&plain).unwrap()).unwrap(),
            link
        );
        // Under another slot, or another key: nothing.
        assert!(open(&key, "ABD", &sealed).is_err());

        // A wrong code: the confirmation does not hold, and the try is spent.
        let (spake, first) = Pairing::start(&code);
        let mut pairing = Pairing {
            spake: Some(spake),
            key: None,
            ..pairing
        };
        let wrong = PairCode::parse("ABC00000").unwrap();
        let (b, message) = Spake2::<Ed25519Group>::start_b(
            &wrong.password(),
            &Identity::new(SIDE_A),
            &Identity::new(SIDE_B),
        );
        let key = b.finish(&first).unwrap();
        assert!(!pairing.check_try(&message, &confirmation(&key)));
        assert!(pairing.sealed_answer().is_none());
        assert!(!pairing.check_try(&message, &confirmation(&key)));
    }
}
