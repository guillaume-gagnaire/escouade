//! The pairing relay of « Ajouter un appareil » by a short code (TS-3, `docs/TEAM-SERVER.md` §3.4).
//!
//! Two desktops of one member, without a camera: the device already in the team shows a code of 8 characters, typed
//! on the new one. The code is the password of a PAKE (SPAKE2) between the two, and this relay carries its four
//! messages, blind, in memory:
//!
//! 1. the device in the team opens a **slot** with its first message (`POST /v1/pair`, signed in): the server gives the
//!    slot's three characters, the head of the code; the other five never reach the server;
//! 2. the new device, not enrolled yet, reads that message (`GET /v1/pair/<slot>`) and leaves its own and its key
//!    confirmation (`POST /v1/pair/<slot>/try`). **One try per slot**: a second is refused, so a wrong code (or a
//!    guess) costs the slot, and a new code is needed;
//! 3. the device in the team reads the try, checks the confirmation, and leaves its answer, sealed under the key the
//!    PAKE gave (`POST /v1/pair/<slot>/answer`), or closes the slot on a wrong code (`DELETE`);
//! 4. the new device takes the answer (`GET /v1/pair/<slot>/answer`), and the slot is gone.
//!
//! The server sees the PAKE's messages and a sealed answer: neither gives the code, even to a dictionary offline
//! (that is what a PAKE is for), and the answer opens under the key of the right code only. A slot lives 10 minutes.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::util::{random, Id};

/// The characters of a code: Crockford's base 32 (no I, L, O, U), read aloud and typed without doubt.
pub const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A slot's characters: the head of the code, which the server chooses.
pub const SLOT_LEN: usize = 3;

pub const TTL: Duration = Duration::from_secs(600);

/// At most so many slots open at once on the server, and per team.
const MAX_SLOTS: usize = 4096;
const MAX_PER_TEAM: usize = 4;

/// The largest message of the PAKE (SPAKE2's are 33 bytes), confirmation and answer.
pub const MAX_MESSAGE: usize = 256;
pub const MAX_ANSWER: usize = 4096;

/// The new device's try: its PAKE message and its key confirmation.
pub type Try = (Vec<u8>, Vec<u8>);

struct Slot {
    team: Id,
    device: Id,
    message: Vec<u8>,
    tried: Option<Try>,
    answer: Option<Vec<u8>>,
    until: Instant,
}

/// What a step of the relay can refuse.
#[derive(Debug, PartialEq, Eq)]
pub enum PairError {
    /// No such slot, or its time is over, or it was closed.
    Gone,
    /// The slot had its try already.
    Tried,
    /// The slot is another device's.
    NotYours,
    /// Too many slots open.
    Full,
}

#[derive(Default)]
pub struct Pairs {
    slots: Mutex<HashMap<String, Slot>>,
}

fn slot_name() -> String {
    random::<SLOT_LEN>()
        .iter()
        .map(|b| ALPHABET[(*b as usize) % 32] as char)
        .collect()
}

/// A slot's characters as typed: upper case, `O` read as `0`, `I` and `L` as `1`.
pub fn normalize(slot: &str) -> String {
    slot.trim()
        .chars()
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        })
        .collect()
}

impl Pairs {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Slot>> {
        self.slots.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn live<'a>(map: &'a mut HashMap<String, Slot>, slot: &str) -> Result<&'a mut Slot, PairError> {
        let now = Instant::now();
        match map.get_mut(&normalize(slot)) {
            Some(s) if s.until > now => Ok(s),
            _ => Err(PairError::Gone),
        }
    }

    /// Opens a slot for `device` of `team` with the PAKE's first message.
    pub fn open(&self, team: Id, device: Id, message: Vec<u8>) -> Result<String, PairError> {
        let mut map = self.lock();
        let now = Instant::now();
        map.retain(|_, s| s.until > now);
        if map.len() >= MAX_SLOTS || map.values().filter(|s| s.team == team).count() >= MAX_PER_TEAM
        {
            return Err(PairError::Full);
        }
        // A free name: 32 768 of them, a few thousand open at most.
        let name = loop {
            let n = slot_name();
            if !map.contains_key(&n) {
                break n;
            }
        };
        map.insert(
            name.clone(),
            Slot {
                team,
                device,
                message,
                tried: None,
                answer: None,
                until: now + TTL,
            },
        );
        Ok(name)
    }

    /// The first message, for the new device.
    pub fn message(&self, slot: &str) -> Result<Vec<u8>, PairError> {
        let mut map = self.lock();
        Ok(Self::live(&mut map, slot)?.message.clone())
    }

    /// The new device's try: once.
    pub fn try_once(
        &self,
        slot: &str,
        message: Vec<u8>,
        confirm: Vec<u8>,
    ) -> Result<(), PairError> {
        let mut map = self.lock();
        let s = Self::live(&mut map, slot)?;
        if s.tried.is_some() {
            return Err(PairError::Tried);
        }
        s.tried = Some((message, confirm));
        Ok(())
    }

    /// The try, for the device that opened the slot; None while there is none.
    pub fn tried(&self, slot: &str, device: &Id) -> Result<Option<Try>, PairError> {
        let mut map = self.lock();
        let s = Self::live(&mut map, slot)?;
        if s.device != *device {
            return Err(PairError::NotYours);
        }
        Ok(s.tried.clone())
    }

    /// The sealed answer of the device that opened the slot, after a try.
    pub fn answer(&self, slot: &str, device: &Id, sealed: Vec<u8>) -> Result<(), PairError> {
        let mut map = self.lock();
        let s = Self::live(&mut map, slot)?;
        if s.device != *device {
            return Err(PairError::NotYours);
        }
        if s.tried.is_none() {
            return Err(PairError::Gone);
        }
        s.answer = Some(sealed);
        Ok(())
    }

    /// The answer, taken once by the new device: the slot goes with it. None while there is none.
    pub fn take_answer(&self, slot: &str) -> Result<Option<Vec<u8>>, PairError> {
        let mut map = self.lock();
        let key = normalize(slot);
        let s = Self::live(&mut map, &key)?;
        let Some(answer) = s.answer.take() else {
            return Ok(None);
        };
        map.remove(&key);
        Ok(Some(answer))
    }

    /// Closes a slot (a wrong code, or the user gave up): by the device that opened it.
    pub fn close(&self, slot: &str, device: &Id) -> Result<(), PairError> {
        let mut map = self.lock();
        let key = normalize(slot);
        let s = Self::live(&mut map, &key)?;
        if s.device != *device {
            return Err(PairError::NotYours);
        }
        map.remove(&key);
        Ok(())
    }

    /// Forgets the slots whose time is over.
    pub fn sweep(&self) {
        let now = Instant::now();
        self.lock().retain(|_, s| s.until > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_takes_one_try_and_gives_its_answer_once() {
        let pairs = Pairs::default();
        let (team, a) = ([1; 16], [2; 16]);
        let slot = pairs.open(team, a, b"m1".to_vec()).unwrap();
        assert_eq!(slot.len(), SLOT_LEN);
        assert_eq!(pairs.message(&slot.to_lowercase()).unwrap(), b"m1");
        assert_eq!(pairs.tried(&slot, &a).unwrap(), None);
        assert_eq!(pairs.answer(&slot, &a, b"x".to_vec()), Err(PairError::Gone));
        pairs
            .try_once(&slot, b"m2".to_vec(), b"c".to_vec())
            .unwrap();
        assert_eq!(
            pairs.try_once(&slot, b"m3".to_vec(), b"c".to_vec()),
            Err(PairError::Tried)
        );
        assert_eq!(pairs.tried(&slot, &[9; 16]), Err(PairError::NotYours));
        assert_eq!(pairs.take_answer(&slot).unwrap(), None);
        pairs.answer(&slot, &a, b"sealed".to_vec()).unwrap();
        assert_eq!(pairs.take_answer(&slot).unwrap().unwrap(), b"sealed");
        assert_eq!(pairs.take_answer(&slot), Err(PairError::Gone));
    }

    #[test]
    fn a_team_opens_few_slots_and_a_closed_one_is_gone() {
        let pairs = Pairs::default();
        let slots: Vec<_> = (0..MAX_PER_TEAM)
            .map(|_| pairs.open([1; 16], [2; 16], vec![1]).unwrap())
            .collect();
        assert_eq!(pairs.open([1; 16], [2; 16], vec![1]), Err(PairError::Full));
        assert!(pairs.open([3; 16], [4; 16], vec![1]).is_ok());
        pairs.close(&slots[0], &[2; 16]).unwrap();
        assert_eq!(pairs.message(&slots[0]), Err(PairError::Gone));
    }

    #[test]
    fn a_typed_slot_reads_o_as_zero_and_i_l_as_one() {
        assert_eq!(normalize(" o1l "), "011");
        assert_eq!(normalize("ab9"), "AB9");
    }
}
