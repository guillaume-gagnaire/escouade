//! The sessions of the remote API (ESC-495): what a client finds again when it comes back after a cut.
//!
//! - A session is a device's (`devices`): the server gives its id at the handshake (`Welcome`), the client gives it
//!   back at the next one (`Hello`). A session of another device, or one forgotten, is not found: the client gets a
//!   new one, and resumes nothing. A client never reads the output of a terminal of another device's session.
//! - It holds the terminals the client opened, by the id of their channel (unique in the session: the client chooses
//!   it), each its output's ring (`termlog::Replay`): resumed from where the client stopped.
//! - Bounded: [`MAX_CHANNELS`] terminals per session (the ended ones let go first), [`MAX_PER_DEVICE`] sessions per
//!   device and [`MAX_SESSIONS`] in all (the oldest without a connection let go first), each forgotten [`KEPT`] after
//!   its last connection ended.

use crate::termlog::Replay;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The most terminals a session keeps.
pub const MAX_CHANNELS: usize = 256;
/// The most sessions of one device.
pub const MAX_PER_DEVICE: usize = 8;
/// The most sessions in all.
pub const MAX_SESSIONS: usize = 64;
/// How long a session is kept once its connection ended.
pub const KEPT: Duration = Duration::from_secs(15 * 60);

struct Session {
    device: String,
    channels: HashMap<u32, Arc<dyn Replay>>,
    /// The channels, oldest first.
    order: VecDeque<u32>,
    /// Its connections open now.
    live: usize,
    /// When its last connection ended.
    left: Instant,
    /// The events of the hub emitted when it ended (`Hub::emitted`): those since, its client missed (ESC-538).
    seen: u64,
}

#[derive(Default)]
pub struct Sessions {
    all: Mutex<HashMap<String, Session>>,
}

/// A new session id: 128 random bits, in hex.
fn new_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the system's random numbers");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl Sessions {
    /// The session `asked` of `device` taken again, when it is that device's and still kept; else a new one. Its id,
    /// and whether it is the one asked. Its connections count one more.
    pub fn open(&self, device: &str, asked: Option<&str>, now: Instant) -> (String, bool) {
        let mut all = self.all.lock();
        all.retain(|_, s| s.live > 0 || now.duration_since(s.left) < KEPT);
        if let Some(id) = asked.filter(|a| a.len() <= super::proto::MAX_SESSION) {
            if let Some(s) = all.get_mut(id).filter(|s| s.device == device) {
                s.live += 1;
                return (id.to_string(), true);
            }
        }
        // Room for one more: the oldest left of the device first, then of all.
        for (limit, of_device) in [(MAX_PER_DEVICE, true), (MAX_SESSIONS, false)] {
            loop {
                let count = all
                    .values()
                    .filter(|s| !of_device || s.device == device)
                    .count();
                if count < limit {
                    break;
                }
                let oldest = all
                    .iter()
                    .filter(|(_, s)| s.live == 0 && (!of_device || s.device == device))
                    .min_by_key(|(_, s)| s.left)
                    .map(|(id, _)| id.clone());
                match oldest {
                    Some(id) => {
                        all.remove(&id);
                    }
                    None => break,
                }
            }
        }
        let id = new_id();
        all.insert(
            id.clone(),
            Session {
                device: device.to_string(),
                channels: HashMap::new(),
                order: VecDeque::new(),
                live: 1,
                left: now,
                seen: 0,
            },
        );
        (id, false)
    }

    /// A connection of the session ended.
    pub fn leave(&self, id: &str, now: Instant) {
        if let Some(s) = self.all.lock().get_mut(id) {
            s.live = s.live.saturating_sub(1);
            s.left = now;
        }
    }

    /// The events of the hub emitted as a connection of the session ends (`Hub::emitted`).
    pub fn note_seen(&self, id: &str, emitted: u64) {
        if let Some(s) = self.all.lock().get_mut(id) {
            s.seen = emitted;
        }
    }

    /// The events emitted since the session's last connection ended, for a client that takes it again: none for a
    /// session never left.
    pub fn missed(&self, id: &str, emitted: u64) -> Option<u64> {
        self.all
            .lock()
            .get(id)
            .filter(|s| s.seen > 0)
            .map(|s| emitted.saturating_sub(s.seen))
    }

    /// The terminal of `channel` in the session `id` (its output, `replay`), from now on.
    pub fn bind(&self, id: &str, channel: u32, replay: Arc<dyn Replay>) {
        let mut all = self.all.lock();
        let Some(s) = all.get_mut(id) else { return };
        if s.channels.insert(channel, replay).is_none() {
            s.order.push_back(channel);
        }
        while s.channels.len() > MAX_CHANNELS {
            // The oldest that ended first, else the oldest.
            let at = s
                .order
                .iter()
                .position(|c| s.channels.get(c).is_some_and(|r| r.over()))
                .unwrap_or(0);
            if let Some(c) = s.order.remove(at) {
                s.channels.remove(&c);
            }
        }
    }

    /// The terminal of `channel` in the session `id` of `device`.
    pub fn find(&self, id: &str, device: &str, channel: u32) -> Option<Arc<dyn Replay>> {
        let all = self.all.lock();
        let s = all.get(id).filter(|s| s.device == device)?;
        s.channels.get(&channel).cloned()
    }

    /// The device's sessions forgotten (it was revoked).
    pub fn forget_device(&self, device: &str) {
        self.all.lock().retain(|_, s| s.device != device);
    }

    /// How many sessions are kept.
    pub fn len(&self) -> usize {
        self.all.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.all.lock().is_empty()
    }

    /// How many terminals the session `id` keeps.
    pub fn channels(&self, id: &str) -> usize {
        self.all.lock().get(id).map_or(0, |s| s.channels.len())
    }
}
