//! The hybrid logical clock (HLC, Kulkarni et al. 2014) of the operations (`docs/TEAM-SERVER.md` §2.4, C2-A).
//!
//! A stamp is the wall clock in milliseconds, a counter, and the author's device: two stamps of one project are never
//! equal, and they order every write of a field the same way on every device (the last writer wins). A device's
//! clock never goes back, and it is always ahead of every stamp it saw: a write made after seeing another one wins
//! over it, whatever the two devices' clocks say.
//!
//! - A stamp seen from a device whose clock is far ahead moves this clock at most [`MAX_DRIFT_MS`] ahead of its own
//!   wall clock: the stamp is kept as it is (every copy must apply the same writes the same way, whatever its own
//!   clock), but this device does not jump a day ahead because of one wrong clock.
//! - The bytes of a stamp ([`Hlc::to_bytes`]) sort as the stamps do: the local copy compares them in SQL.

use serde::{Deserialize, Serialize};

use crate::identity::DeviceId;

/// How far ahead of its own wall clock a device lets a stamp it saw move its clock: an hour.
pub const MAX_DRIFT_MS: u64 = 3_600_000;

/// A stamp: ordered by its wall clock, then its counter, then its device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Hlc {
    /// Milliseconds since the Unix epoch, as the author's clock (moved by what it saw) said.
    pub wall: u64,
    pub count: u32,
    pub device: DeviceId,
}

/// The length of [`Hlc::to_bytes`].
pub const HLC_BYTES: usize = 28;

impl Hlc {
    /// Big-endian wall, counter, then the device: the bytes sort as the stamps.
    pub fn to_bytes(&self) -> [u8; HLC_BYTES] {
        let mut out = [0u8; HLC_BYTES];
        out[..8].copy_from_slice(&self.wall.to_be_bytes());
        out[8..12].copy_from_slice(&self.count.to_be_bytes());
        out[12..].copy_from_slice(&self.device.0);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != HLC_BYTES {
            return None;
        }
        let mut device = [0u8; 16];
        device.copy_from_slice(&bytes[12..]);
        Some(Self {
            wall: u64::from_be_bytes(bytes[..8].try_into().ok()?),
            count: u32::from_be_bytes(bytes[8..12].try_into().ok()?),
            device: DeviceId(device),
        })
    }
}

/// A device's clock for a project: the last stamp it made or saw (without the device).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clock {
    pub wall: u64,
    pub count: u32,
}

impl Clock {
    /// A new stamp for a write of `device` at the wall clock `now_ms`: after everything this clock made or saw.
    pub fn tick(&mut self, now_ms: u64, device: DeviceId) -> Hlc {
        if now_ms > self.wall {
            self.wall = now_ms;
            self.count = 0;
        } else if self.count == u32::MAX {
            self.wall += 1;
            self.count = 0;
        } else {
            self.count += 1;
        }
        Hlc {
            wall: self.wall,
            count: self.count,
            device,
        }
    }

    /// A stamp seen in a write of another device: this clock's next stamp comes after it (up to the drift bound).
    pub fn observe(&mut self, seen: &Hlc, now_ms: u64) {
        let bound = now_ms.saturating_add(MAX_DRIFT_MS);
        let (wall, count) = if seen.wall > bound {
            (bound, 0)
        } else {
            (seen.wall, seen.count)
        };
        if wall > self.wall {
            self.wall = wall;
            self.count = count;
        } else if wall == self.wall && count > self.count {
            self.count = count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: DeviceId = DeviceId([1; 16]);
    const B: DeviceId = DeviceId([2; 16]);

    #[test]
    fn a_clock_never_goes_back_and_passes_what_it_saw() {
        let mut c = Clock::default();
        let a = c.tick(1_000, A);
        let b = c.tick(900, A);
        assert!(b > a, "a clock set back still moves on");
        c.observe(
            &Hlc {
                wall: 5_000,
                count: 7,
                device: B,
            },
            1_000,
        );
        let d = c.tick(1_000, A);
        assert_eq!((d.wall, d.count), (5_000, 8));
    }

    #[test]
    fn a_stamp_far_ahead_moves_the_clock_by_the_drift_at_most() {
        let mut c = Clock::default();
        c.observe(
            &Hlc {
                wall: u64::MAX - 5,
                count: 0,
                device: B,
            },
            10,
        );
        assert_eq!(c.wall, 10 + MAX_DRIFT_MS);
    }

    #[test]
    fn the_bytes_sort_as_the_stamps() {
        let stamps = [
            Hlc {
                wall: 1,
                count: 9,
                device: B,
            },
            Hlc {
                wall: 2,
                count: 0,
                device: A,
            },
            Hlc {
                wall: 2,
                count: 0,
                device: B,
            },
            Hlc {
                wall: 2,
                count: 1,
                device: A,
            },
        ];
        for w in stamps.windows(2) {
            assert!(w[0] < w[1]);
            assert!(w[0].to_bytes() < w[1].to_bytes());
            assert_eq!(Hlc::from_bytes(&w[0].to_bytes()), Some(w[0]));
        }
        assert_eq!(Hlc::from_bytes(&[0; 3]), None);
    }
}
