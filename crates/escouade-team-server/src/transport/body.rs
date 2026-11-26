//! Bounded bodies. A `Content-Length` above the bound is refused before a byte is read; a body without one is read
//! up to the bound, and refused past it.
//!
//! **Read in a bounded time** (SR2-07): a body comes whole within `Deadline::base`, plus what its length takes at
//! `Deadline::min_rate` (the announced length, else the route's bound). Past it, the request is refused (408
//! `slow_body`) and the connection closes: a client that trickles a body does not hold its connection.

use std::time::Duration;

use axum::body::Bytes;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use http_body_util::{BodyExt, Limited};

use super::Problem;

/// The bound of a route's body, put in the request's extensions by the route (`axum::Extension`).
#[derive(Clone, Copy, Debug)]
pub struct Max(pub usize);

/// How long a body may take, put in the request's extensions by the router (`axum::Extension`).
#[derive(Clone, Copy, Debug)]
pub struct Deadline {
    /// The time any body has.
    pub base: Duration,
    /// The slowest rate a long body is read at, in bytes a second: its time grows with its length.
    pub min_rate: u64,
}

impl Default for Deadline {
    fn default() -> Self {
        Deadline {
            base: Duration::from_secs(30),
            min_rate: 64 << 10,
        }
    }
}

impl Deadline {
    /// The time a body of `length` bytes has.
    pub fn for_length(&self, length: u64) -> Duration {
        self.base + Duration::from_secs(length / self.min_rate.max(1))
    }
}

/// The announced length, if any. A length that does not parse is refused.
pub fn announced(req: &Request) -> Result<Option<u64>, Problem> {
    match req.headers().get(header::CONTENT_LENGTH) {
        None => Ok(None),
        Some(v) => v
            .to_str()
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Some)
            .ok_or_else(|| Problem::bad_request("bad_length", "Content-Length is not a number")),
    }
}

fn slow() -> Problem {
    Problem::new(
        StatusCode::REQUEST_TIMEOUT,
        "slow_body",
        "the body did not come in time",
    )
}

pub async fn read(req: Request, max: usize) -> Result<Bytes, Problem> {
    let length = announced(&req)?;
    if length.is_some_and(|n| n > max as u64) {
        return Err(Problem::too_large(max));
    }
    let deadline = req
        .extensions()
        .get::<Deadline>()
        .copied()
        .unwrap_or_default();
    let time = deadline.for_length(length.unwrap_or(max as u64));
    let collected = tokio::time::timeout(time, Limited::new(req.into_body(), max).collect())
        .await
        .map_err(|_| slow())?;
    match collected {
        Ok(collected) => Ok(collected.to_bytes()),
        Err(e)
            if e.downcast_ref::<http_body_util::LengthLimitError>()
                .is_some() =>
        {
            Err(Problem::too_large(max))
        }
        Err(_) => Err(Problem::bad_request(
            "bad_body",
            "the body could not be read",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_longer_body_has_more_time() {
        let d = Deadline::default();
        assert_eq!(d.for_length(0), Duration::from_secs(30));
        assert_eq!(d.for_length(64 << 10), Duration::from_secs(31));
        // A file of 25 MiB, read at 64 KiB/s at least: 30 s and 400 s.
        assert_eq!(d.for_length(25 << 20), Duration::from_secs(430));
    }
}
