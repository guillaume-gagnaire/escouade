//! The bounds of the entry (§ 4.5, ESC-654): what a stranger who reaches the port may try.
//!
//! - **By source** (its address, or what a trusted proxy says of it in `X-Forwarded-For`): every attempt that fails
//!   (no handshake in time, not this protocol, a key no connector has, a wrong code) counts; past
//!   [`Bounds::failures_per_minute`] (20) in a minute, its connections are closed unread to the end of that minute.
//! - **The whole entry**: past 20 failed attempts in a minute from all sources, the pairing is closed unread to the end
//!   of the minute, for everyone; a connector already paired still gets in (its key is read, not guessed), so that a
//!   stranger never cuts the private network.
//! - **The wrong codes of a source**: past [`Bounds::wrong_codes_per_source`] (5) in a code's lifetime (10 minutes),
//!   its pairings are refused unread for the rest of it. A wrong code never ends the code that waits (ESC-654: that let
//!   anyone block the pairing at will).
//!
//! So a guess is out of reach: at most 20 wrong codes a minute from all the Internet, 200 in a code's 10 minutes,
//! against 32⁸ = 2⁴⁰ codes: about one chance in 5.5 billion.
//!
//! The times are Tokio's, so that the tests run them on a clock of their own.

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use tokio::time::Instant;

/// The sources whose failures are kept at most: past them, the one whose last failure is oldest is forgotten.
const MAX_SOURCES: usize = 4096;

const MINUTE: Duration = Duration::from_secs(60);

/// The numbers of the bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The failed attempts of a source in a minute past which it is closed unread (20).
    pub failures_per_minute: usize,
    /// The failed attempts of all sources in a minute past which the pairing is closed unread (20).
    pub failures_in_all: usize,
    /// The wrong codes of a source in [`Limits::codes_window`] past which its pairings are refused (5).
    pub wrong_codes_per_source: usize,
    /// A code's lifetime (10 minutes).
    pub codes_window: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            failures_per_minute: 20,
            failures_in_all: 20,
            wrong_codes_per_source: 5,
            codes_window: Duration::from_secs(10 * 60),
        }
    }
}

#[derive(Default)]
struct Tries {
    /// The failures of each source in the last minute.
    by_source: HashMap<String, VecDeque<Instant>>,
    /// The failures of all sources in the last minute.
    all: VecDeque<Instant>,
    /// The wrong codes of each source in the last window of a code.
    codes: HashMap<String, VecDeque<Instant>>,
}

/// The bounds, kept in memory: they start again with the server.
pub struct Bounds {
    pub limits: Limits,
    tries: Mutex<Tries>,
}

fn within(times: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while times
        .front()
        .is_some_and(|t| now.duration_since(*t) >= window)
    {
        times.pop_front();
    }
}

fn push(
    map: &mut HashMap<String, VecDeque<Instant>>,
    source: &str,
    now: Instant,
    window: Duration,
    keep: usize,
) -> usize {
    if !map.contains_key(source) && map.len() >= MAX_SOURCES {
        map.retain(|_, t| {
            within(t, now, window);
            !t.is_empty()
        });
        if map.len() >= MAX_SOURCES {
            if let Some(oldest) = map
                .iter()
                .min_by_key(|(_, t)| t.back().copied())
                .map(|(s, _)| s.clone())
            {
                map.remove(&oldest);
            }
        }
    }
    let times = map.entry(source.to_string()).or_default();
    within(times, now, window);
    times.push_back(now);
    while times.len() > keep.max(1) {
        times.pop_front();
    }
    times.len()
}

fn count(
    map: &mut HashMap<String, VecDeque<Instant>>,
    source: &str,
    now: Instant,
    window: Duration,
) -> usize {
    let Some(times) = map.get_mut(source) else {
        return 0;
    };
    within(times, now, window);
    if times.is_empty() {
        map.remove(source);
        return 0;
    }
    times.len()
}

impl Bounds {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            tries: Mutex::default(),
        }
    }

    /// Whether `source` is closed unread: too many failures of it in the minute.
    pub fn closed(&self, source: &str) -> bool {
        let now = Instant::now();
        count(&mut self.tries.lock().by_source, source, now, MINUTE)
            >= self.limits.failures_per_minute
    }

    /// Whether a pairing from `source` is read: neither it nor the whole entry past their bounds.
    pub fn may_pair(&self, source: &str) -> bool {
        let now = Instant::now();
        let mut tries = self.tries.lock();
        within(&mut tries.all, now, MINUTE);
        if tries.all.len() >= self.limits.failures_in_all {
            return false;
        }
        if count(&mut tries.by_source, source, now, MINUTE) >= self.limits.failures_per_minute {
            return false;
        }
        count(&mut tries.codes, source, now, self.limits.codes_window)
            < self.limits.wrong_codes_per_source
    }

    /// A failed attempt of `source`.
    pub fn failed(&self, source: &str) {
        let now = Instant::now();
        let limits = self.limits;
        let mut tries = self.tries.lock();
        let n = push(
            &mut tries.by_source,
            source,
            now,
            MINUTE,
            limits.failures_per_minute,
        );
        if n == limits.failures_per_minute {
            log::warn!(
                "connectors' entry: {n} failed attempts from {source} in a minute: closed to it until the minute ends"
            );
        }
        within(&mut tries.all, now, MINUTE);
        tries.all.push_back(now);
        if tries.all.len() == limits.failures_in_all {
            log::warn!(
                "connectors' entry: {} failed attempts in a minute from all sources: the pairing closed until the minute ends (paired connectors still get in)",
                tries.all.len()
            );
        }
        while tries.all.len() > limits.failures_in_all.max(1) {
            tries.all.pop_front();
        }
    }

    /// A wrong code from `source` (a failed attempt too: [`Bounds::failed`] is called with it).
    pub fn wrong_code(&self, source: &str) {
        let now = Instant::now();
        let limits = self.limits;
        {
            let mut tries = self.tries.lock();
            let n = push(
                &mut tries.codes,
                source,
                now,
                limits.codes_window,
                limits.wrong_codes_per_source,
            );
            if n == limits.wrong_codes_per_source {
                log::warn!(
                    "connectors' entry: {n} wrong pairing codes from {source}: its pairings refused for {} minutes (the code that waits still holds)",
                    limits.codes_window.as_secs() / 60
                );
            }
        }
        self.failed(source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a_source_is_closed_after_twenty_failures_until_the_minute_ends() {
        let b = Bounds::new(Limits::default());
        for _ in 0..19 {
            b.failed("1.2.3.4");
        }
        assert!(!b.closed("1.2.3.4"));
        b.failed("1.2.3.4");
        assert!(b.closed("1.2.3.4"));
        assert!(!b.closed("5.6.7.8"), "another source is not");
        tokio::time::advance(MINUTE).await;
        assert!(!b.closed("1.2.3.4"), "open again once the minute ended");
    }

    #[tokio::test(start_paused = true)]
    async fn the_pairing_closes_for_all_after_twenty_failures_of_all_sources() {
        let b = Bounds::new(Limits::default());
        for i in 0..20 {
            assert!(b.may_pair("9.9.9.9"));
            b.failed(&format!("10.0.0.{i}"));
        }
        assert!(!b.may_pair("9.9.9.9"), "closed for everyone");
        assert!(!b.closed("9.9.9.9"), "but a paired connector still gets in");
        tokio::time::advance(MINUTE).await;
        assert!(b.may_pair("9.9.9.9"));
    }

    #[tokio::test(start_paused = true)]
    async fn five_wrong_codes_close_the_pairing_to_a_source_for_a_code_s_lifetime() {
        let b = Bounds::new(Limits::default());
        for _ in 0..5 {
            assert!(b.may_pair("1.2.3.4"));
            b.wrong_code("1.2.3.4");
            tokio::time::advance(Duration::from_secs(61)).await;
        }
        assert!(!b.may_pair("1.2.3.4"));
        assert!(b.may_pair("5.6.7.8"), "another source may still pair");
        tokio::time::advance(Duration::from_secs(10 * 60)).await;
        assert!(b.may_pair("1.2.3.4"));
    }
}
