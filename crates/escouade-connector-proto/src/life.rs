//! The connector's schedule of reconnection (§ 4.2, § 4.8): 250 ms, 500 ms, 1 s, 2 s, 4 s, then every 5 s, as the
//! client of the remote API. The connector (RP-3) waits [`Backoff::next_delay`] before each try and
//! [`Backoff::reset`]s once a channel is up.

use std::time::Duration;

/// The first delays, then [`THEN_EVERY`].
pub const RECONNECT: [Duration; 5] = [
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];
pub const THEN_EVERY: Duration = Duration::from_secs(5);

/// The delay before try `attempt` (0 is the first after a cut).
pub fn reconnect_delay(attempt: usize) -> Duration {
    RECONNECT.get(attempt).copied().unwrap_or(THEN_EVERY)
}

#[derive(Clone, Debug, Default)]
pub struct Backoff {
    attempt: usize,
}

impl Backoff {
    pub fn new() -> Backoff {
        Backoff::default()
    }

    /// The delay before the next try, and one more try counted.
    pub fn next_delay(&mut self) -> Duration {
        let d = reconnect_delay(self.attempt);
        self.attempt = self.attempt.saturating_add(1);
        d
    }

    /// A channel is up: the next cut starts again at 250 ms.
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_schedule_is_the_clients() {
        let mut b = Backoff::new();
        let ms: Vec<u128> = (0..8).map(|_| b.next_delay().as_millis()).collect();
        assert_eq!(ms, [250, 500, 1000, 2000, 4000, 5000, 5000, 5000]);
        b.reset();
        assert_eq!(b.next_delay(), Duration::from_millis(250));
        assert_eq!(reconnect_delay(usize::MAX), THEN_EVERY);
    }
}
