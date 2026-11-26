//! The bounds of a channel (§ 4.4), set by the caller (`connector.toml` for the connector, the server's own), and the
//! constants of the protocol that both sides share.

use std::collections::VecDeque;
use std::time::Duration;

use tokio::time::Instant;

/// A `ping` every 15 s, from both sides.
pub const PING_EVERY: Duration = Duration::from_secs(15);
/// Nothing heard for 45 s: the channel is cut.
pub const DEAD_AFTER: Duration = Duration::from_secs(45);
/// The window each side grants on a stream when it opens, in each direction.
pub const INITIAL_WINDOW: u32 = 256 << 10;
/// The most bytes of one `data` frame.
pub const DATA_CHUNK: usize = 16 << 10;
/// The most bytes of a frame, the most a Noise message holds (65 535, less its tag of 16).
pub const MAX_FRAME: usize = 65_535 - 16;
/// The least a caller may set as the bound of a frame: a full `data` frame, and the largest ceiling a peer sends.
pub const MIN_FRAME: usize = 32 << 10;
/// The most bytes of the ceiling's frame a connector sends, whatever the server's bound.
pub const MAX_CEILING_FRAME: usize = MIN_FRAME;
/// The most a window may hold (as HTTP/2: 2³¹ − 1).
pub const MAX_WINDOW: u64 = (1 << 31) - 1;

/// The bounds of one channel, enforced on what the peer sends (and on what this side asks to open).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Limits {
    /// The most bytes of a frame received (kept within [`MIN_FRAME`]..=[`MAX_FRAME`]); beyond, the channel is closed.
    pub max_frame: usize,
    /// The most streams open at once (256); one more is refused (`limits`).
    pub max_streams: usize,
    /// The most streams asked in a second (32), refused ones included; one more is refused (`limits`).
    pub opens_per_second: u32,
    /// A stream on which nothing goes for this long is closed (10 min).
    pub stream_idle: Duration,
    /// The handshake's two messages within this (10 s).
    pub handshake_timeout: Duration,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_frame: MAX_FRAME,
            max_streams: 256,
            opens_per_second: 32,
            stream_idle: Duration::from_secs(10 * 60),
            handshake_timeout: Duration::from_secs(10),
        }
    }
}

impl Limits {
    /// The bound of a frame within what the protocol needs and allows.
    pub fn max_frame(&self) -> usize {
        self.max_frame.clamp(MIN_FRAME, MAX_FRAME)
    }
}

/// The opens of the last second.
#[derive(Debug, Default)]
pub(crate) struct Rate {
    opens: VecDeque<Instant>,
}

impl Rate {
    /// Whether one more open is within `per_second`, counting it if so.
    pub(crate) fn admit(&mut self, now: Instant, per_second: u32) -> bool {
        while let Some(&first) = self.opens.front() {
            if now.duration_since(first) >= Duration::from_secs(1) {
                self.opens.pop_front();
            } else {
                break;
            }
        }
        if self.opens.len() >= per_second as usize {
            return false;
        }
        self.opens.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rate_admits_so_many_in_a_second() {
        let start = Instant::now();
        let mut rate = Rate::default();
        for _ in 0..32 {
            assert!(rate.admit(start, 32));
        }
        assert!(!rate.admit(start, 32));
        assert!(!rate.admit(start + Duration::from_millis(999), 32));
        assert!(rate.admit(start + Duration::from_secs(1), 32));
    }

    #[test]
    fn the_frame_bound_stays_within_the_protocol() {
        let l = |max_frame| Limits {
            max_frame,
            ..Limits::default()
        };
        assert_eq!(l(0).max_frame(), MIN_FRAME);
        assert_eq!(l(1 << 20).max_frame(), MAX_FRAME);
        assert_eq!(l(40_000).max_frame(), 40_000);
        let d = Limits::default();
        assert_eq!(
            (d.max_streams, d.opens_per_second, d.stream_idle),
            (256, 32, Duration::from_secs(600))
        );
    }
}
