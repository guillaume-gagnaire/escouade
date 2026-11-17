//! The bodies of the server's fuzzing targets (the package `fuzz/` at the root of the repository, TS-18), and a test
//! that plays them in every run.
//!
//! - [`bodies`]: what a client sends before anything is checked: each request body as CBOR, the header of the
//!   versions' handshake, the address a proxy gives. The handshake accepts the server's version alone (and the one
//!   before), and the address is the peer's or one the header names, never anything else.
//! - [`requests`]: the real routes, in memory, on a server of small bounds over a temporary SQLite base: a request
//!   made of the bytes (a route and its ids, a session or none, the version, the type, the body) from no one, from a
//!   revoked device, from a member or from an administrator of a team. The server never fails (no 5xx); nothing but
//!   the open routes answers without a session, nor to the revoked device; a member never does what is the
//!   administrator's (enrol a device, change a role, revoke another device, change who has access to a project that
//!   exists, write the team's list of projects).

use std::net::SocketAddr;

use axum::body::Body;
use axum::http::{HeaderValue, Method, Request, StatusCode};
use ed25519_dalek::{Signer as _, SigningKey};
use tower::ServiceExt as _;

use crate::api::wire::*;
use crate::api::CATALOG_STREAM;
use crate::config::{Config, Rate, Rates};
use crate::transport::{cbor, handshake, listen};
use crate::util::Id;
use crate::Server;

pub fn bodies(data: &[u8]) {
    let _ = cbor::decode::<EnrollRequest>(data);
    let _ = cbor::decode::<ChallengeRequest>(data);
    let _ = cbor::decode::<SessionRequest>(data);
    let _ = cbor::decode::<EnrollmentRequest>(data);
    let _ = cbor::decode::<RoleRequest>(data);
    let _ = cbor::decode::<PairOpen>(data);
    let _ = cbor::decode::<PairTry>(data);
    let _ = cbor::decode::<PairAnswer>(data);
    let _ = cbor::decode::<AppendRequest>(data);
    let _ = cbor::decode::<AccessWire>(data);
    let _ = cbor::decode::<ClaimRequest>(data);
    let _ = cbor::decode::<TokenRequest>(data);
    let _ = cbor::decode::<SnapshotPut>(data);
    if let Ok(v) = HeaderValue::from_bytes(data) {
        let refused = handshake::refusal(crate::PROTOCOL, Some(&v));
        let said = v.to_str().ok().and_then(|s| s.trim().parse::<u32>().ok());
        assert_eq!(
            refused.is_none(),
            said == Some(crate::PROTOCOL),
            "the handshake on {v:?}"
        );
    }
    if let Ok(text) = std::str::from_utf8(data) {
        let peer: SocketAddr = "10.0.0.2:5000".parse().expect("an address");
        assert_eq!(
            listen::client_address(Some(peer), Some(text), false),
            Some(peer.ip())
        );
        if let Some(ip) = listen::client_address(Some(peer), Some(text), true) {
            assert!(
                // The peer, or the last hop the header names, read as an address (`D::` is `d::`).
                ip == peer.ip()
                    || text
                        .rsplit(',')
                        .next()
                        .and_then(|last| last.trim().parse::<std::net::IpAddr>().ok())
                        == Some(ip),
                "an address from nowhere: {ip} from {text:?}"
            );
        }
    }
}

/// Who sends a request.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Who {
    Nobody,
    Garbage,
    Revoked,
    Member,
    Admin,
}

/// The routes, by method and template: `{d}` a device, `{p}` a project, `{t}` a ticket, `{f}` a file, `{s}` a slot.
const ROUTES: &[(&str, &str, bool)] = &[
    // (method, template, open without a session)
    ("GET", "/healthz", true),
    ("GET", "/join/{f}", true),
    ("POST", "/v1/enroll", true),
    ("POST", "/v1/auth/challenge", true),
    ("POST", "/v1/auth/session", true),
    ("GET", "/v1/devices", false),
    ("POST", "/v1/devices/enrollments", false),
    ("GET", "/v1/devices/enrollments/alerts", false),
    ("POST", "/v1/devices/{d}/revoke", false),
    ("POST", "/v1/devices/{d}/role", false),
    ("POST", "/v1/pair", false),
    ("GET", "/v1/pair/{s}", true),
    ("DELETE", "/v1/pair/{s}", false),
    ("POST", "/v1/pair/{s}/try", true),
    ("GET", "/v1/pair/{s}/try", false),
    ("POST", "/v1/pair/{s}/answer", false),
    ("GET", "/v1/pair/{s}/answer", true),
    ("GET", "/v1/team/stream?since=0", false),
    ("POST", "/v1/team/stream", false),
    ("GET", "/v1/projects/{p}/stream?since=0&limit=3", false),
    ("POST", "/v1/projects/{p}/stream", false),
    ("GET", "/v1/projects/{p}/access", false),
    ("PUT", "/v1/projects/{p}/access", false),
    ("GET", "/v1/projects/{p}/claims", false),
    ("POST", "/v1/projects/{p}/claims/{t}", false),
    ("POST", "/v1/projects/{p}/claims/{t}/renew", false),
    ("POST", "/v1/projects/{p}/claims/{t}/release", false),
    ("POST", "/v1/projects/{p}/keys", false),
    ("GET", "/v1/projects/{p}/snapshot", false),
    ("PUT", "/v1/projects/{p}/snapshot", false),
    ("GET", "/v1/files/usage", false),
    ("GET", "/v1/files/{f}", false),
    ("PUT", "/v1/files/{f}", false),
    ("DELETE", "/v1/files/{f}", false),
    ("GET", "/v1/live", false),
];

struct Bench {
    rt: tokio::runtime::Runtime,
    router: axum::Router,
    admin: (Id, String),
    member: (Id, String),
    revoked: (Id, String),
    project: Id,
}

async fn call(
    router: &axum::Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Vec<u8>,
) -> (StatusCode, Vec<u8>) {
    let mut r = Request::builder()
        .method(method)
        .uri(path)
        .header("escouade-protocol", "1")
        .header("content-type", cbor::CONTENT_TYPE);
    if let Some(t) = token {
        r = r.header("authorization", format!("Bearer {t}"));
    }
    let answer = router
        .clone()
        .oneshot(r.body(Body::from(body)).expect("a request"))
        .await
        .expect("an answer");
    let status = answer.status();
    let bytes = axum::body::to_bytes(answer.into_body(), 64 << 20)
        .await
        .expect("a body");
    (status, bytes.to_vec())
}

async fn enrol(router: &axum::Router, token: &str, seed: u8) -> (Id, String) {
    let key = SigningKey::from_bytes(&[seed; 32]);
    let (status, body) = call(
        router,
        Method::POST,
        "/v1/enroll",
        None,
        cbor::encode(&EnrollRequest {
            token: token.into(),
            public_key: key.verifying_key().to_bytes().to_vec(),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "enrolled");
    let e: EnrollResponse = cbor::decode(&body).expect("an enrolment");
    let id: Id = e.device.as_slice().try_into().expect("an id");
    let (_, body) = call(
        router,
        Method::POST,
        "/v1/auth/challenge",
        None,
        cbor::encode(&ChallengeRequest {
            device: id.to_vec(),
        }),
    )
    .await;
    let c: ChallengeResponse = cbor::decode(&body).expect("a challenge");
    let challenge: [u8; 32] = c.challenge.as_slice().try_into().expect("32 bytes");
    let signature = key.sign(&crate::auth::signed_message(&id, &challenge));
    let (_, body) = call(
        router,
        Method::POST,
        "/v1/auth/session",
        None,
        cbor::encode(&SessionRequest {
            device: id.to_vec(),
            challenge: challenge.to_vec(),
            signature: signature.to_bytes().to_vec(),
        }),
    )
    .await;
    let t: TokenResponse = cbor::decode(&body).expect("a session");
    (id, t.token)
}

impl Bench {
    fn new() -> Bench {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        let dir = std::env::temp_dir().join(format!(
            "escouade-team-server-fuzz-{}-{}",
            std::process::id(),
            hex::encode(crate::util::random::<4>())
        ));
        let mut config = Config::in_folder(&dir);
        config.limits.max_entry = 4096;
        config.limits.max_file = 16 << 10;
        config.limits.team_quota = 1 << 20;
        config.limits.page_bytes = 64 << 10;
        config.limits.session_ttl = std::time::Duration::from_secs(30 * 24 * 3600);
        let fast = Rate {
            per_second: 1e9,
            burst: 1e9,
        };
        config.rates = Rates {
            per_address: fast,
            per_device: fast,
        };
        let (router, admin, member, revoked, project) = rt.block_on(async {
            let server = Server::open(config).await.expect("the server opens");
            let router = server.router();
            let first = server.new_team_token().await.expect("a token");
            let admin = enrol(&router, &first, 1).await;
            let more = |router: axum::Router, token: String| async move {
                let (_, body) = call(
                    &router,
                    Method::POST,
                    "/v1/devices/enrollments",
                    Some(&token),
                    cbor::encode(&EnrollmentRequest::default()),
                )
                .await;
                cbor::decode::<TokenResponse>(&body).expect("a token").token
            };
            let t = more(router.clone(), admin.1.clone()).await;
            let member = enrol(&router, &t, 2).await;
            let t = more(router.clone(), admin.1.clone()).await;
            let revoked = enrol(&router, &t, 3).await;
            let (status, _) = call(
                &router,
                Method::POST,
                &format!("/v1/devices/{}/revoke", hex::encode(revoked.0)),
                Some(&admin.1),
                Vec::new(),
            )
            .await;
            assert!(status.is_success(), "revoked: {status}");
            let project = [7u8; 16];
            let append = cbor::encode(&ciborium::Value::Map(vec![
                ("kind".into(), "message".into()),
                ("payload".into(), ciborium::Value::Bytes(vec![1, 2, 3])),
            ]));
            let (status, _) = call(
                &router,
                Method::POST,
                &format!("/v1/projects/{}/stream", hex::encode(project)),
                Some(&admin.1),
                append,
            )
            .await;
            assert!(status.is_success(), "a project made: {status}");
            (router, admin, member, revoked, project)
        });
        Bench {
            rt,
            router,
            admin,
            member,
            revoked,
            project,
        }
    }
}

pub fn requests(data: &[u8]) {
    thread_local! {
        static BENCH: Bench = Bench::new();
    }
    BENCH.with(|b| one(b, data));
}

fn one(b: &Bench, data: &[u8]) {
    let mut bytes = data.iter().copied();
    let mut next = || bytes.next().unwrap_or(0);
    let who = match next() % 5 {
        0 => Who::Nobody,
        1 => Who::Garbage,
        2 => Who::Revoked,
        3 => Who::Member,
        _ => Who::Admin,
    };
    let (method, template, open) = ROUTES[usize::from(next()) % ROUTES.len()];
    let pick = next();
    let id = |known: Id| -> String {
        match pick % 4 {
            0 => hex::encode(known),
            1 => hex::encode(b.admin.0),
            2 => hex::encode(CATALOG_STREAM),
            _ => "zz".repeat(16),
        }
    };
    let device = match pick % 3 {
        0 => b.member.0,
        1 => b.admin.0,
        _ => b.revoked.0,
    };
    let path = template
        .replace("{d}", &id(device))
        .replace("{p}", &id(b.project))
        .replace("{t}", &id([9; 16]))
        .replace("{f}", &id([8; 16]))
        .replace("{s}", ["AB1", "000", "ZZZ", "a/b"][usize::from(pick) % 4]);
    let protocol = next();
    let kind = next();
    let body: Vec<u8> = data.get(6..).unwrap_or_default().to_vec();
    let mut r = Request::builder().method(method).uri(&path);
    match protocol % 4 {
        0 => r = r.header("escouade-protocol", "1"),
        1 => {}
        2 => r = r.header("escouade-protocol", "0"),
        _ => r = r.header("escouade-protocol", "2"),
    }
    match kind % 3 {
        0 => r = r.header("content-type", cbor::CONTENT_TYPE),
        1 => r = r.header("content-type", "application/octet-stream"),
        _ => {}
    }
    let token = match who {
        Who::Nobody => None,
        Who::Garbage => Some("not-a-session".to_string()),
        Who::Revoked => Some(b.revoked.1.clone()),
        Who::Member => Some(b.member.1.clone()),
        Who::Admin => Some(b.admin.1.clone()),
    };
    if let Some(t) = &token {
        r = r.header("authorization", format!("Bearer {t}"));
    }
    let Ok(request) = r.body(Body::from(body)) else {
        return;
    };
    let status = b.rt.block_on(async {
        let answer = b.router.clone().oneshot(request).await.expect("an answer");
        let status = answer.status();
        let _ = axum::body::to_bytes(answer.into_body(), 64 << 20).await;
        status
    });
    assert!(
        !status.is_server_error(),
        "{status} for {who:?} {method} {path}"
    );
    if status.is_success() {
        let admin_only = matches!(
            template,
            "/v1/devices/enrollments" | "/v1/devices/enrollments/alerts" | "/v1/devices/{d}/role"
        ) || template == "/v1/devices/{d}/revoke" && device != b.member.0
            || template == "/v1/projects/{p}/access"
                && method == "PUT"
                && path.contains(&hex::encode(b.project))
            || template == "/v1/projects/{p}/stream"
                && method == "POST"
                && path.contains(&hex::encode(CATALOG_STREAM));
        match who {
            Who::Nobody | Who::Garbage | Who::Revoked => {
                assert!(
                    open,
                    "{status} without a valid session for {who:?} {method} {path}"
                )
            }
            Who::Member => assert!(
                !admin_only,
                "a member did what is the administrator's: {method} {path}"
            ),
            Who::Admin => {}
        }
    }
}

#[cfg(test)]
mod tests {
    /// Both bodies on a few inputs and on the seeds committed in `fuzz/seeds/` (`server-bodies`, `server-requests`).
    #[test]
    fn the_targets_hold() {
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..300 {
            let len = (next() % 80) as usize;
            let data: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            super::bodies(&data);
            super::requests(&data);
        }
        // Every route, by everyone.
        for who in 0..5u8 {
            for route in 0..super::ROUTES.len() as u8 {
                for pick in 0..4u8 {
                    super::requests(&[who, route, pick, 0, 0, 0, 0xa0]);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
        // `ESCOUADE_WRITE_SEEDS=1` writes them again: a body of each route, the handshake's header, a forwarded
        // address; a request of each route by each sender.
        if std::env::var("ESCOUADE_WRITE_SEEDS").as_deref() == Ok("1") {
            use crate::api::wire::*;
            use crate::transport::cbor::encode;
            let write = |target: &str, name: &str, bytes: &[u8]| {
                let dir = root.join(target);
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join(name), bytes).unwrap();
            };
            write(
                "server-bodies",
                "enroll",
                &encode(&EnrollRequest {
                    token: "t".into(),
                    public_key: vec![1; 32],
                }),
            );
            write(
                "server-bodies",
                "session",
                &encode(&SessionRequest {
                    device: vec![2; 16],
                    challenge: vec![3; 32],
                    signature: vec![4; 64],
                }),
            );
            write(
                "server-bodies",
                "pair-try",
                &encode(&PairTry {
                    message: vec![5; 33],
                    confirm: vec![6; 32],
                }),
            );
            write(
                "server-bodies",
                "enrollment",
                &encode(&EnrollmentRequest {
                    expires_in: Some(60),
                    token_hash: Some(vec![7; 32]),
                }),
            );
            write(
                "server-bodies",
                "bomb",
                &[
                    0xa1, 0x67, b'p', b'a', b'y', b'l', b'o', b'a', b'd', 0x5b, 0x40, 0, 0, 0, 0,
                    0, 0, 0,
                ],
            );
            write("server-bodies", "header", b" 1 ");
            write("server-bodies", "forwarded", b"1.1.1.1, 203.0.113.7");
            for who in 0..5u8 {
                for route in [2u8, 6, 8, 18, 20, 22, 24, 32] {
                    write(
                        "server-requests",
                        &format!("who-{who}-route-{route}"),
                        &[who, route, 0, 0, 0, 0, 0xa0],
                    );
                }
            }
        }
        for (name, target) in [
            ("server-bodies", super::bodies as fn(&[u8])),
            ("server-requests", super::requests),
        ] {
            for e in std::fs::read_dir(root.join(name)).expect("the seeds") {
                target(&std::fs::read(e.unwrap().path()).unwrap());
            }
        }
    }
}
