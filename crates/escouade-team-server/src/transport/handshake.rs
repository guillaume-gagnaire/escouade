//! The handshake of the versions (`docs/TEAM-SERVER.md` §14). Every request of the API says the protocol of the
//! client in `Escouade-Protocol`; the server accepts its own and the one before, refuses an older one with « Mets
//! Escouade à jour », and a newer one with the server late. Every answer says the server's.

use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use super::Problem;

pub const HEADER: &str = "escouade-protocol";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Accepted,
    /// Older than the one before the server's.
    ClientTooOld,
    /// Newer than the server's.
    ServerTooOld,
}

/// The server's protocol `server` and the client's `client`: the same, or the one before (never below 1).
pub fn check(server: u32, client: u32) -> Verdict {
    let oldest = server.saturating_sub(1).max(1);
    if client > server {
        Verdict::ServerTooOld
    } else if client < oldest {
        Verdict::ClientTooOld
    } else {
        Verdict::Accepted
    }
}

#[derive(Serialize)]
struct Versions {
    server: u32,
    oldest: u32,
}

/// The refusal of a client, or `None`.
pub fn refusal(server: u32, header: Option<&HeaderValue>) -> Option<Problem> {
    let versions = Versions {
        server,
        oldest: server.saturating_sub(1).max(1),
    };
    let client = header
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u32>().ok());
    let verdict = match client {
        Some(client) => check(server, client),
        // A client without the header predates the protocol.
        None => Verdict::ClientTooOld,
    };
    match verdict {
        Verdict::Accepted => None,
        Verdict::ClientTooOld => Some(
            Problem::new(StatusCode::UPGRADE_REQUIRED, "client_too_old", "Mets Escouade à jour")
                .with_detail(&versions),
        ),
        Verdict::ServerTooOld => Some(
            Problem::new(
                StatusCode::BAD_REQUEST,
                "server_too_old",
                "Le serveur d'équipe est plus ancien que cet Escouade : son administrateur doit le mettre à jour",
            )
            .with_detail(&versions),
        ),
    }
}

/// The middleware: `axum::middleware::from_fn_with_state(PROTOCOL, handshake::layer)`.
pub async fn layer(State(server): State<u32>, req: Request, next: Next) -> Response {
    let mut response = match refusal(server, req.headers().get(HEADER)) {
        Some(problem) => problem.into_response(),
        None => next.run(req).await,
    };
    response
        .headers_mut()
        .insert(HEADER, HeaderValue::from(server));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_server_takes_its_version_and_the_one_before() {
        assert_eq!(check(5, 5), Verdict::Accepted);
        assert_eq!(check(5, 4), Verdict::Accepted);
        assert_eq!(check(5, 3), Verdict::ClientTooOld);
        assert_eq!(check(5, 0), Verdict::ClientTooOld);
        assert_eq!(check(5, 6), Verdict::ServerTooOld);
        // The first version has nothing before it.
        assert_eq!(check(1, 1), Verdict::Accepted);
        assert_eq!(check(1, 0), Verdict::ClientTooOld);
    }

    #[test]
    fn no_header_is_too_old() {
        let p = refusal(1, None).unwrap();
        assert_eq!(p.status, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(p.message, "Mets Escouade à jour");
        assert!(refusal(1, Some(&HeaderValue::from_static("1"))).is_none());
        assert_eq!(
            refusal(1, Some(&HeaderValue::from_static("x")))
                .unwrap()
                .error,
            "client_too_old"
        );
    }
}
