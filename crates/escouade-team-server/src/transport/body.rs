//! Bounded bodies. A `Content-Length` above the bound is refused before a byte is read; a body without one is read
//! up to the bound, and refused past it.

use axum::body::Bytes;
use axum::extract::Request;
use axum::http::header;
use http_body_util::{BodyExt, Limited};

use super::Problem;

/// The bound of a route's body, put in the request's extensions by the route (`axum::Extension`).
#[derive(Clone, Copy, Debug)]
pub struct Max(pub usize);

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

pub async fn read(req: Request, max: usize) -> Result<Bytes, Problem> {
    if announced(&req)?.is_some_and(|n| n > max as u64) {
        return Err(Problem::too_large(max));
    }
    match Limited::new(req.into_body(), max).collect().await {
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
