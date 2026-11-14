//! Rate limits: a token bucket by key (an address, a device). In memory, for one server process.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::Instant;

use axum::http::StatusCode;

use super::Problem;

#[derive(Clone, Copy, Debug)]
pub struct Rate {
    pub per_second: f64,
    pub burst: f64,
}

struct Bucket {
    tokens: f64,
    at: Instant,
}

/// The buckets of one kind of key. A full bucket is forgotten at the next sweep: the map holds only keys seen lately.
pub struct Limiter<K> {
    rate: Rate,
    buckets: Mutex<HashMap<K, Bucket>>,
}

impl<K: Eq + Hash + Clone> Limiter<K> {
    pub fn new(rate: Rate) -> Self {
        Limiter {
            rate,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Takes one token for `key`; `Err(seconds)` until the next one when there is none.
    pub fn take(&self, key: &K) -> Result<(), u64> {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        let bucket = buckets.entry(key.clone()).or_insert(Bucket {
            tokens: self.rate.burst,
            at: now,
        });
        let elapsed = now.duration_since(bucket.at).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.rate.per_second).min(self.rate.burst);
        bucket.at = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            let wait = (1.0 - bucket.tokens) / self.rate.per_second.max(f64::MIN_POSITIVE);
            Err(wait.ceil().max(1.0) as u64)
        }
    }

    /// Forgets the buckets that have filled up again.
    pub fn sweep(&self) {
        let now = Instant::now();
        let rate = self.rate;
        self.buckets
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, b| {
                b.tokens + now.duration_since(b.at).as_secs_f64() * rate.per_second < rate.burst
            });
    }

    pub fn len(&self) -> usize {
        self.buckets.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub fn refused(retry_after: u64) -> Problem {
    let mut p = Problem::new(
        StatusCode::TOO_MANY_REQUESTS,
        "rate_limited",
        "too many requests: wait a little",
    );
    p.retry_after = Some(retry_after);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bucket_empties_then_fills_again() {
        let limiter = Limiter::new(Rate {
            per_second: 1000.0,
            burst: 3.0,
        });
        for _ in 0..3 {
            limiter.take(&1).unwrap();
        }
        assert!(limiter.take(&1).is_err());
        // Another key has its own bucket.
        limiter.take(&2).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        limiter.take(&1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        limiter.sweep();
        assert!(limiter.is_empty());
    }
}
