//! Authentication by a signed challenge (`docs/TEAM-SERVER.md` §2.6): the server gives a challenge, the device signs
//! it with its Ed25519 key, the server gives a short session token. No password anywhere.
//!
//! What the device signs: `AUTH_LABEL ‖ device id (16 bytes) ‖ challenge (32 bytes)`. A challenge serves once, for
//! `challenge_ttl`; a session lasts `session_ttl`, then the device signs a new challenge. Both live in memory: a
//! restart only asks the devices to sign again.

use std::collections::HashMap;
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

#[derive(Default)]
pub struct Sessions {
    challenges: Mutex<HashMap<[u8; 32], (Id, Instant)>>,
    sessions: Mutex<HashMap<[u8; 32], Session>>,
}

fn full() -> Problem {
    Problem::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "busy",
        "too many sign-ins at once: try again",
    )
}

impl Sessions {
    pub fn challenge(&self, device: Id, expires: Instant) -> Result<[u8; 32], Problem> {
        let mut map = self.challenges.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() >= MAX_PENDING {
            let now = Instant::now();
            map.retain(|_, (_, e)| *e > now);
            if map.len() >= MAX_PENDING {
                return Err(full());
            }
        }
        let challenge = random::<32>();
        map.insert(challenge, (device, expires));
        Ok(challenge)
    }

    /// Takes a challenge once: true if it was `device`'s and is still valid.
    pub fn take_challenge(&self, device: &Id, challenge: &[u8; 32]) -> bool {
        let taken = self
            .challenges
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(challenge);
        matches!(taken, Some((d, expires)) if d == *device && expires > Instant::now())
    }

    pub fn open(&self, device: Id, expires: Instant) -> Result<String, Problem> {
        let token = crate::util::new_token();
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() >= MAX_PENDING {
            let now = Instant::now();
            map.retain(|_, s| s.expires > now);
            if map.len() >= MAX_PENDING {
                return Err(full());
            }
        }
        map.insert(token_hash(&token), Session { device, expires });
        Ok(token)
    }

    fn session(&self, token: &str) -> Option<Session> {
        let hash = token_hash(token);
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(&hash) {
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
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, s| s.device != *device);
    }

    pub fn sweep(&self) {
        let now = Instant::now();
        self.challenges
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, (_, e)| *e > now);
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, s| s.expires > now);
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

    /// At most `MAX_PENDING` challenges, and as many sessions, wait in memory (TS-18): past it, those whose time is
    /// over make room, and when none is, a new one is refused (503 `busy`) rather than kept.
    #[test]
    fn the_challenges_and_the_sessions_waiting_are_bounded() {
        let later = Instant::now() + Duration::from_secs(60);
        let s = Sessions::default();
        for _ in 0..MAX_PENDING {
            s.challenge([1; 16], later).unwrap();
            s.open([1; 16], later).unwrap();
        }
        assert_eq!(s.challenge([1; 16], later).unwrap_err().error, "busy");
        assert_eq!(s.open([1; 16], later).unwrap_err().error, "busy");
        // Their time over, they make room.
        let s = Sessions::default();
        let now = Instant::now();
        for _ in 0..MAX_PENDING {
            s.challenge([1; 16], now).unwrap();
            s.open([1; 16], now).unwrap();
        }
        assert!(s.challenge([1; 16], later).is_ok());
        assert!(s.open([1; 16], later).is_ok());
        assert_eq!(s.challenges.lock().unwrap().len(), 1);
    }
}
