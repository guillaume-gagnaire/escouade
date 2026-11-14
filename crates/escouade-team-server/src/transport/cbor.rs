//! CBOR bodies (RFC 8949): the requests and the answers of the API. Encrypted objects travel inside as byte strings
//! (`serde_bytes`), never as base64.

use axum::extract::{FromRequest, Request};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::body;

pub const CONTENT_TYPE: &str = "application/cbor";

/// The largest CBOR request when a route says nothing else.
pub const DEFAULT_MAX: usize = 64 << 10;

pub fn encode(value: &impl Serialize) -> Vec<u8> {
    let mut out = Vec::new();
    ciborium::into_writer(value, &mut out).expect("a value of ours encodes");
    out
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Problem> {
    ciborium::from_reader(bytes)
        .map_err(|_| Problem::bad_request("bad_cbor", "the body is not the CBOR expected"))
}

/// A CBOR body in, or a CBOR answer out. As a request, it is bounded by `body::Max` when the route sets one, else by
/// `DEFAULT_MAX`.
#[derive(Debug, Clone)]
pub struct Cbor<T>(pub T);

impl<T, S> FromRequest<S> for Cbor<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Problem;

    async fn from_request(req: Request, _: &S) -> Result<Self, Self::Rejection> {
        let is_cbor = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.split(';').next().map(str::trim) == Some(CONTENT_TYPE));
        if !is_cbor {
            return Err(Problem::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "not_cbor",
                "the body must be application/cbor",
            ));
        }
        let max = req
            .extensions()
            .get::<body::Max>()
            .map_or(DEFAULT_MAX, |m| m.0);
        let bytes = body::read(req, max).await?;
        decode(&bytes).map(Cbor)
    }
}

impl<T: Serialize> IntoResponse for Cbor<T> {
    fn into_response(self) -> Response {
        (
            [(header::CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE))],
            encode(&self.0),
        )
            .into_response()
    }
}

/// What every refusal answers: a status, a stable code for the client, a sentence, and sometimes more.
#[derive(Debug, Clone, Serialize)]
pub struct Problem {
    #[serde(skip)]
    pub status: StatusCode,
    pub error: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ciborium::Value>,
    #[serde(skip)]
    pub retry_after: Option<u64>,
}

impl Problem {
    pub fn new(status: StatusCode, error: &'static str, message: impl Into<String>) -> Self {
        Problem {
            status,
            error,
            message: message.into(),
            detail: None,
            retry_after: None,
        }
    }

    pub fn bad_request(error: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error, message)
    }

    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", "sign in again")
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "nothing here")
    }

    pub fn too_large(max: usize) -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "too_large",
            format!("the body is larger than {max} bytes"),
        )
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "the server failed",
        )
    }

    pub fn with_detail(mut self, detail: &impl Serialize) -> Self {
        self.detail = ciborium::Value::serialized(detail).ok();
        self
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let mut response = (self.status, Cbor(&self)).into_response();
        if let Some(seconds) = self.retry_after {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }
        response
    }
}
