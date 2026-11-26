//! Authentication by a signed challenge (`docs/TEAM-SERVER.md` §2.6): the server gives a challenge, the device signs
//! it with its Ed25519 key, the server gives a short session token. No password anywhere.
//!
//! What the device signs: `AUTH_LABEL ‖ device id (16 bytes) ‖ challenge (32 bytes)`. A challenge serves once, for
//! `challenge_ttl`; a session lasts `session_ttl`, then the device signs a new challenge. Both live in memory: a
//! restart only asks the devices to sign again.
//!
//! **Bounded by device and by team** (SR2-06): a device has at most `PER_DEVICE` challenges waiting and
//! `SESSIONS_PER_DEVICE` sessions open (a new one takes the place of its oldest), a team at most `PER_TEAM` challenges
//! waiting, and the server `MAX_PENDING` of each. A device, or a team, that asks without end fills only its own
//! places: the other teams still sign in.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
use ed25519_dalek::{Signature, VerifyingKey};

use crate::db::{Device, Role};
use crate::metrics::Metrics;
use crate::transport::{rate, Problem};
use crate::util::{random, token_hash, Id};
use crate::Shared;

pub const AUTH_LABEL: &[u8] = b"escouade-team-server/auth/v1\0";

/// At most so many challenges and sessions waiting in memory; past it, the expired ones go, then new ones wait.
const MAX_PENDING: usize = 100_000;
/// The challenges of one device waiting at once: a new one takes the place of its oldest.
pub const PER_DEVICE: usize = 2;
/// The challenges of one team waiting at once (its 500 devices with their two each fit in it).
pub const PER_TEAM: usize = 1_024;
/// The sessions of one device open at once: a new one takes the place of its oldest.
pub const SESSIONS_PER_DEVICE: usize = 8;

/// What a device signs to open a session.
pub fn signed_message(device: &Id, challenge: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(AUTH_LABEL.len() + 48);
    out.extend_from_slice(AUTH_LABEL);
    out.extend_from_slice(device);
    out.extend_from_slice(challenge);
    out
}

#[derive(Clone, Copy, Debug)]
struct Session {
    device: Id,
    expires: Instant,
}

#[derive(Clone, Copy, Debug)]
struct Pending {
    device: Id,
    team: Id,
    expires: Instant,
}

/// Keys kept by an owner (a device, a team) in the order they came, so that the oldest goes first.
fn forget<K: PartialEq>(owners: &mut HashMap<Id, VecDeque<K>>, owner: &Id, key: &K) {
    if let Some(q) = owners.get_mut(owner) {
        q.retain(|k| k != key);
        if q.is_empty() {
            owners.remove(owner);
        }
    }
}

/// The challenges waiting, by challenge, with what each device and each team holds of them.
#[derive(Default)]
struct Challenges {
    by_challenge: HashMap<[u8; 32], Pending>,
    by_device: HashMap<Id, VecDeque<[u8; 32]>>,
    by_team: HashMap<Id, usize>,
}

impl Challenges {
    fn remove(&mut self, challenge: &[u8; 32]) -> Option<Pending> {
        let p = self.by_challenge.remove(challenge)?;
        forget(&mut self.by_device, &p.device, challenge);
        if let Some(n) = self.by_team.get_mut(&p.team) {
            *n -= 1;
            if *n == 0 {
                self.by_team.remove(&p.team);
            }
        }
        Some(p)
    }

    fn sweep(&mut self, now: Instant) {
        let gone: Vec<[u8; 32]> = self
            .by_challenge
            .iter()
            .filter(|(_, p)| p.expires <= now)
            .map(|(c, _)| *c)
            .collect();
        for c in gone {
            self.remove(&c);
        }
    }

    fn team_full(&self, team: &Id) -> bool {
        self.by_team.get(team).is_some_and(|n| *n >= PER_TEAM)
    }
}

/// The sessions open, by the hash of their token, with those of each device in order.
#[derive(Default)]
struct Open {
    by_token: HashMap<[u8; 32], Session>,
    by_device: HashMap<Id, VecDeque<[u8; 32]>>,
}

impl Open {
    fn remove(&mut self, hash: &[u8; 32]) -> Option<Session> {
        let s = self.by_token.remove(hash)?;
        forget(&mut self.by_device, &s.device, hash);
        Some(s)
    }

    fn sweep(&mut self, now: Instant) {
        let gone: Vec<[u8; 32]> = self
            .by_token
            .iter()
            .filter(|(_, s)| s.expires <= now)
            .map(|(h, _)| *h)
            .collect();
        for h in gone {
            self.remove(&h);
        }
    }
}

#[derive(Default)]
pub struct Sessions {
    challenges: Mutex<Challenges>,
    sessions: Mutex<Open>,
}

fn full() -> Problem {
    Problem::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "busy",
        "too many sign-ins at once: try again",
    )
}

impl Sessions {
    /// A new challenge for `device` of `team`. The device's oldest goes when it has `PER_DEVICE` waiting; a team with
    /// `PER_TEAM` waiting, or the server with `MAX_PENDING`, waits (503 `busy`) once those past their time are gone.
    pub fn challenge(&self, device: Id, team: Id, expires: Instant) -> Result<[u8; 32], Problem> {
        let mut map = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        while let Some(oldest) = map
            .by_device
            .get(&device)
            .filter(|q| q.len() >= PER_DEVICE)
            .and_then(|q| q.front().copied())
        {
            map.remove(&oldest);
        }
        // A full team: a device that has one waiting takes its place.
        if map.team_full(&team) {
            if let Some(oldest) = map.by_device.get(&device).and_then(|q| q.front().copied()) {
                map.remove(&oldest);
            }
        }
        if map.by_challenge.len() >= MAX_PENDING || map.team_full(&team) {
            map.sweep(Instant::now());
            if map.by_challenge.len() >= MAX_PENDING || map.team_full(&team) {
                return Err(full());
            }
        }
        let challenge = random::<32>();
        map.by_challenge.insert(
            challenge,
            Pending {
                device,
                team,
                expires,
            },
        );
        map.by_device
            .entry(device)
            .or_default()
            .push_back(challenge);
        *map.by_team.entry(team).or_default() += 1;
        Ok(challenge)
    }

    /// Takes a challenge once: true if it was `device`'s and is still valid.
    pub fn take_challenge(&self, device: &Id, challenge: &[u8; 32]) -> bool {
        let taken = self
            .challenges
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(challenge);
        matches!(taken, Some(p) if p.device == *device && p.expires > Instant::now())
    }

    /// A new session for `device`: its oldest closes when it has `SESSIONS_PER_DEVICE` open.
    pub fn open(&self, device: Id, expires: Instant) -> Result<String, Problem> {
        let token = crate::util::new_token();
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        while let Some(oldest) = map
            .by_device
            .get(&device)
            .filter(|q| q.len() >= SESSIONS_PER_DEVICE)
            .and_then(|q| q.front().copied())
        {
            map.remove(&oldest);
        }
        if map.by_token.len() >= MAX_PENDING {
            map.sweep(Instant::now());
            if map.by_token.len() >= MAX_PENDING {
                return Err(full());
            }
        }
        let hash = token_hash(&token);
        map.by_token.insert(hash, Session { device, expires });
        map.by_device.entry(device).or_default().push_back(hash);
        Ok(token)
    }

    fn session(&self, token: &str) -> Option<Session> {
        let hash = token_hash(token);
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        match map.by_token.get(&hash) {
            Some(s) if s.expires > Instant::now() => Some(*s),
            Some(_) => {
                map.remove(&hash);
                None
            }
            None => None,
        }
    }

    /// Closes every session of a device (it is revoked).
    pub fn close_device(&self, device: &Id) {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        for hash in map.by_device.remove(device).unwrap_or_default() {
            map.by_token.remove(&hash);
        }
    }

    pub fn sweep(&self) {
        let now = Instant::now();
        self.challenges
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .sweep(now);
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .sweep(now);
    }
}

/// Checks a device's signature of a challenge.
pub fn verify(public_key: &[u8; 32], device: &Id, challenge: &[u8; 32], signature: &[u8]) -> bool {
    let Ok(key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let Ok(signature) = Signature::from_slice(signature) else {
        return false;
    };
    key.verify_strict(&signed_message(device, challenge), &signature)
        .is_ok()
}

/// A request of a signed-in device: its session is valid, the device is not revoked, its rate is not exceeded.
#[derive(Clone, Debug)]
pub struct Authed {
    pub device: Device,
    pub expires: Instant,
}

impl Authed {
    pub fn is_admin(&self) -> bool {
        self.device.role == Role::Admin
    }
}

impl FromRequestParts<Shared> for Authed {
    type Rejection = Problem;

    async fn from_request_parts(parts: &mut Parts, app: &Shared) -> Result<Self, Self::Rejection> {
        let refuse = || {
            Metrics::inc(&app.metrics.auth_failures);
            Problem::unauthorized()
        };
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::trim)
            .ok_or_else(refuse)?;
        let session = app.sessions.session(token).ok_or_else(refuse)?;
        let device = app
            .store
            .device(session.device)
            .await
            .map_err(|_| Problem::internal())?
            .filter(|d| d.revoked_at.is_none())
            .ok_or_else(refuse)?;
        if let Err(wait) = app.device_rate.take(&device.id) {
            Metrics::inc(&app.metrics.rate_limited);
            return Err(rate::refused(wait));
        }
        Ok(Authed {
            device,
            expires: session.expires,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn id(n: usize) -> Id {
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&(n as u64).to_be_bytes());
        id
    }

    /// At most `MAX_PENDING` challenges, and as many sessions, wait in memory (TS-18): past it, those whose time is
    /// over make room, and when none is, a new one is refused (503 `busy`) rather than kept.
    #[test]
    fn the_challenges_and_the_sessions_waiting_are_bounded() {
        let later = Instant::now() + Duration::from_secs(60);
        let s = Sessions::default();
        // One device each, `PER_TEAM` devices a team: only the bound of the server stops them.
        for n in 0..MAX_PENDING {
            s.challenge(id(n), id(n / PER_TEAM), later).unwrap();
            s.open(id(n), later).unwrap();
        }
        let (d, t) = (id(MAX_PENDING), id(usize::MAX));
        assert_eq!(s.challenge(d, t, later).unwrap_err().error, "busy");
        assert_eq!(s.open(d, later).unwrap_err().error, "busy");
        // Their time over, they make room.
        let s = Sessions::default();
        let now = Instant::now();
        for n in 0..MAX_PENDING {
            s.challenge(id(n), id(n / PER_TEAM), now).unwrap();
            s.open(id(n), now).unwrap();
        }
        assert!(s.challenge(d, t, later).is_ok());
        assert!(s.open(d, later).is_ok());
        let map = s.challenges.lock().unwrap();
        assert_eq!(
            (
                map.by_challenge.len(),
                map.by_device.len(),
                map.by_team.len()
            ),
            (1, 1, 1)
        );
    }

    /// SR2-06: a device holds `PER_DEVICE` challenges, its newest; its oldest is no longer taken.
    #[test]
    fn a_device_holds_its_newest_challenges_only() {
        let later = Instant::now() + Duration::from_secs(60);
        let s = Sessions::default();
        let (d, t) = (id(1), id(2));
        let all: Vec<_> = (0..10).map(|_| s.challenge(d, t, later).unwrap()).collect();
        {
            let map = s.challenges.lock().unwrap();
            assert_eq!(map.by_challenge.len(), PER_DEVICE);
            assert_eq!(map.by_team[&t], PER_DEVICE);
        }
        assert!(!s.take_challenge(&d, &all[0]));
        assert!(s.take_challenge(&d, &all[9]));
        assert!(s.take_challenge(&d, &all[8]));
        let map = s.challenges.lock().unwrap();
        assert!(map.by_challenge.is_empty() && map.by_device.is_empty() && map.by_team.is_empty());
    }

    /// SR2-06: a team holds `PER_TEAM` challenges; past it, a new device of it waits, and another team signs in.
    #[test]
    fn a_team_full_of_challenges_does_not_stop_another() {
        let later = Instant::now() + Duration::from_secs(60);
        let s = Sessions::default();
        let team = id(7);
        for n in 0..PER_TEAM {
            s.challenge(id(n), team, later).unwrap();
        }
        assert_eq!(
            s.challenge(id(PER_TEAM), team, later).unwrap_err().error,
            "busy"
        );
        // A device of the team that has one already still gets a new one.
        assert!(s.challenge(id(0), team, later).is_ok());
        assert!(s.challenge(id(PER_TEAM + 1), id(8), later).is_ok());
    }

    /// A device holds `SESSIONS_PER_DEVICE` sessions, its newest; revoked, it holds none.
    #[test]
    fn a_device_holds_a_few_sessions() {
        let later = Instant::now() + Duration::from_secs(60);
        let s = Sessions::default();
        let tokens: Vec<_> = (0..SESSIONS_PER_DEVICE + 3)
            .map(|_| s.open(id(1), later).unwrap())
            .collect();
        assert!(s.session(&tokens[0]).is_none());
        assert!(s.session(tokens.last().unwrap()).is_some());
        assert_eq!(
            s.sessions.lock().unwrap().by_token.len(),
            SESSIONS_PER_DEVICE
        );
        s.open(id(2), later).unwrap();
        s.close_device(&id(1));
        let map = s.sessions.lock().unwrap();
        assert_eq!((map.by_token.len(), map.by_device.len()), (1, 1));
    }
}
