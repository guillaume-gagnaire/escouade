//! What the server accepts, bound by bound (TS-18): each bound of `config::Limits` and of the routes has its test here
//! or where it is named below, so that none goes without one. Each on SQLite, then on PostgreSQL when
//! `ESCOUADE_TEST_POSTGRES_URL` is set. Elsewhere:
//! - `max_entry`, a length announced too large, a CBOR string or array announcing more than it holds, the bodies of the
//!   open routes: `server.rs`, `the_sizes_are_bounded`;
//! - `max_file`, `team_quota`: `server.rs`, `files_and_quotas`; `page_entries`: `the_streams_are_numbered…`;
//! - `session_ttl`, a challenge used twice or invented: `server.rs`, `the_signed_challenge_opens_a_short_session`;
//! - `max_live_per_device`: `server.rs`, `the_live_connection_closes_with_the_session`;
//! - the rates by address and by device: `server.rs`, `the_rates_are_limited`;
//! - `claim_lease`: `claims.rs`; `MAX_CLAIMS`: `db/sqlite.rs`; the challenges and sessions waiting (`MAX_PENDING`):
//!   `auth.rs`; the pairing's slots (`MAX_PER_TEAM`, `MAX_SLOTS`): `pair.rs`.

#[macro_use]
mod common;

use std::time::Duration;

use common::*;
use escouade_team_server::api::wire::*;
use escouade_team_server::pair::{MAX_ANSWER, MAX_MESSAGE};
use escouade_team_server::util::random;
use reqwest::{Method, StatusCode};

both!(
    the_devices_and_the_projects_of_a_team_are_bounded,
    a_challenge_lasts_its_time_and_says_nothing_of_who_is_known,
    a_page_is_bounded_in_bytes_and_in_entries,
    an_enrolment_token_lasts_at_most_its_bound_and_is_short,
    the_live_connection_takes_small_frames_only,
    the_pairing_relay_bounds_what_it_carries,
    a_revoked_device_does_nothing_more,
);

async fn the_devices_and_the_projects_of_a_team_are_bounded(b: Backend) {
    let s = TestServer::start_with(&b, |c| {
        c.limits.max_devices = 3;
        c.limits.max_projects = 2;
    })
    .await;
    let alice = s.new_team().await;
    let _bob = s.add_device(&alice).await;
    let _carol = s.add_device(&alice).await;
    // A fourth device: its token is made, its enrolment refused.
    let token = alice.enrollment_token().await;
    let (status, p) = Device::enroll(&s, &token).await.err().expect("refused");
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::FORBIDDEN, "too_many")
    );
    let devices: Devices = alice.client.get("/v1/devices").await.ok();
    assert_eq!(devices.devices.len(), 3);
    // Two projects, then a third refused.
    alice.append_project(&random::<16>(), b"op").await.ok();
    alice.append_project(&random::<16>(), b"op").await.ok();
    let (status, p) = alice.append_project(&random::<16>(), b"op").await.refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::FORBIDDEN, "too_many")
    );
    // An access listing more devices than a team may hold.
    let project = random::<16>();
    let access = AccessWire {
        everyone: false,
        devices: (0..4)
            .map(|_| serde_bytes::ByteBuf::from(random::<16>().to_vec()))
            .collect(),
    };
    let (status, p) = alice
        .client
        .put::<_, AccessWire>(
            &format!("/v1/projects/{}/access", hex::encode(project)),
            &access,
        )
        .await
        .refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::BAD_REQUEST, "too_many")
    );
    s.finish().await;
}

async fn a_challenge_lasts_its_time_and_says_nothing_of_who_is_known(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.challenge_ttl = Duration::from_secs(1)).await;
    let alice = s.new_team().await;
    let challenge = alice.challenge().await.ok();
    assert_eq!(challenge.expires_in, 1);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let signature = alice.sign(&challenge.challenge);
    let (status, _) = alice
        .session(&challenge.challenge, signature)
        .await
        .refused();
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "a challenge past its time"
    );
    // A device unknown and a device revoked: the same answer, word for word.
    let bob = s.add_device(&alice).await;
    assert_eq!(alice.revoke(&bob).await, StatusCode::NO_CONTENT);
    let unknown: Answer<ChallengeResponse> = s
        .client()
        .post(
            "/v1/auth/challenge",
            &ChallengeRequest {
                device: random::<16>().to_vec(),
            },
        )
        .await;
    let (u_status, u) = unknown.refused();
    let (r_status, r) = bob.challenge().await.refused();
    assert_eq!(
        (u_status, &u.error, &u.message),
        (r_status, &r.error, &r.message)
    );
    // An id of another length.
    let short: Answer<ChallengeResponse> = s
        .client()
        .post(
            "/v1/auth/challenge",
            &ChallengeRequest { device: vec![1; 3] },
        )
        .await;
    assert!(short.refused().0.is_client_error());
    s.finish().await;
}

async fn a_page_is_bounded_in_bytes_and_in_entries(b: Backend) {
    let s = TestServer::start_with(&b, |c| {
        c.limits.page_bytes = 2500;
        c.limits.page_entries = 4;
    })
    .await;
    let alice = s.new_team().await;
    let project = random::<16>();
    for _ in 0..6 {
        alice.append_project(&project, &[7; 1000]).await.ok();
    }
    // 2500 bytes: two entries of 1000, and more to read.
    let page = alice.read_project(&project, 0).await;
    assert_eq!(page.entries.len(), 2);
    assert!(page.more);
    // One entry always goes out, even past the bytes.
    let big = random::<16>();
    let s2 = TestServer::start_with(&b, |c| c.limits.page_bytes = 10).await;
    let carol = s2.new_team().await;
    carol.append_project(&big, &[1; 500]).await.ok();
    carol.append_project(&big, &[1; 500]).await.ok();
    let page = carol.read_project(&big, 0).await;
    assert_eq!((page.entries.len(), page.more), (1, true));
    s2.finish().await;
    // A limit asked above the bound is the bound.
    let s3 = TestServer::start_with(&b, |c| c.limits.page_entries = 3).await;
    let dave = s3.new_team().await;
    for _ in 0..5 {
        dave.message(b"m").await;
    }
    let page: Entries = dave
        .client
        .get("/v1/team/stream?since=0&limit=1000")
        .await
        .ok();
    assert_eq!(page.entries.len(), 3);
    s3.finish().await;
    s.finish().await;
}

async fn an_enrolment_token_lasts_at_most_its_bound_and_is_short(b: Backend) {
    let s = TestServer::start_with(&b, |c| {
        c.limits.enrollment_ttl_max = Duration::from_secs(100)
    })
    .await;
    let alice = s.new_team().await;
    let r: TokenResponse = alice
        .client
        .post(
            "/v1/devices/enrollments",
            &EnrollmentRequest {
                expires_in: Some(1_000_000_000),
                token_hash: None,
            },
        )
        .await
        .ok();
    assert_eq!(r.expires_in, 100);
    // A hash that is not a SHA-256.
    let (status, p) = alice
        .client
        .post::<_, Registered>(
            "/v1/devices/enrollments",
            &EnrollmentRequest {
                expires_in: None,
                token_hash: Some(vec![1; 31]),
            },
        )
        .await
        .refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::BAD_REQUEST, "bad_hash")
    );
    // A token of more than 128 characters is not looked for.
    let long = "x".repeat(129);
    let (status, _) = Device::enroll(&s, &long).await.err().expect("refused");
    assert!(status.is_client_error());
    // Nor is a key that is not Ed25519.
    let (status, p) = s
        .client()
        .post::<_, EnrollResponse>(
            "/v1/enroll",
            &EnrollRequest {
                token: r.token.clone(),
                public_key: vec![1; 31],
            },
        )
        .await
        .refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::BAD_REQUEST, "bad_key")
    );
    // The token still serves, once.
    Device::enroll(&s, &r.token).await.expect("enrolled");
    s.finish().await;
}

async fn the_live_connection_takes_small_frames_only(b: Backend) {
    use futures_util::SinkExt as _;
    use tokio_tungstenite::tungstenite::Message;
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let mut live = alice.live(&s).await;
    // A small message is ignored; one of 20 KiB closes the connection.
    live.socket
        .send(Message::Binary(vec![1; 100].into()))
        .await
        .unwrap();
    let _ = live
        .socket
        .send(Message::Binary(vec![1; 20 << 10].into()))
        .await;
    match live.next(Duration::from_secs(5)).await {
        Some(LiveEvent::Closed(_)) => {}
        Some(LiveEvent::Frame(_)) => panic!("a frame after a message too large"),
        None => panic!("the connection stays open after a message too large"),
    }
    // The server stands.
    assert_eq!(
        s.client().status(Method::GET, "/healthz", None).await.0,
        StatusCode::OK
    );
    s.finish().await;
}

async fn the_pairing_relay_bounds_what_it_carries(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let (status, _) = alice
        .client
        .post::<_, PairOpened>(
            "/v1/pair",
            &PairOpen {
                message: vec![1; MAX_MESSAGE + 1],
            },
        )
        .await
        .refused();
    assert!(status.is_client_error());
    let opened: PairOpened = alice
        .client
        .post(
            "/v1/pair",
            &PairOpen {
                message: vec![1; MAX_MESSAGE],
            },
        )
        .await
        .ok();
    let slot = opened.slot;
    // The new device's try: no session, and bounded.
    let big = s
        .client()
        .request(Method::POST, &format!("/v1/pair/{slot}/try"))
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(&PairTry {
            message: vec![2; MAX_MESSAGE + 1],
            confirm: vec![3; 32],
        }))
        .send()
        .await
        .unwrap();
    assert!(big.status().is_client_error());
    let big = s
        .client()
        .request(Method::POST, &format!("/v1/pair/{slot}/try"))
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(&PairTry {
            message: vec![2; 32],
            confirm: vec![3; MAX_MESSAGE + 1],
        }))
        .send()
        .await
        .unwrap();
    assert!(big.status().is_client_error());
    // A refused try does not spend the slot's one try.
    let tried = s
        .client()
        .request(Method::POST, &format!("/v1/pair/{slot}/try"))
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(&PairTry {
            message: vec![2; 32],
            confirm: vec![3; 32],
        }))
        .send()
        .await
        .unwrap();
    assert!(tried.status().is_success(), "{}", tried.status());
    let (status, _) = alice
        .client
        .post::<_, ()>(
            &format!("/v1/pair/{slot}/answer"),
            &PairAnswer {
                sealed: vec![4; MAX_ANSWER + 1],
            },
        )
        .await
        .refused();
    assert!(status.is_client_error());
    s.finish().await;
}

async fn a_revoked_device_does_nothing_more(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let project = random::<16>();
    alice.append_project(&project, b"op").await.ok();
    assert_eq!(alice.revoke(&bob).await, StatusCode::NO_CONTENT);
    // With the session it had: nothing, on any route.
    let routes: Vec<(Method, String)> = vec![
        (Method::GET, "/v1/devices".into()),
        (Method::GET, "/v1/team/stream?since=0".into()),
        (Method::POST, "/v1/team/stream".into()),
        (
            Method::GET,
            format!("/v1/projects/{}/stream?since=0", hex::encode(project)),
        ),
        (
            Method::POST,
            format!("/v1/projects/{}/stream", hex::encode(project)),
        ),
        (
            Method::POST,
            format!(
                "/v1/projects/{}/claims/{}",
                hex::encode(project),
                hex::encode(random::<16>())
            ),
        ),
        (
            Method::POST,
            format!("/v1/projects/{}/keys", hex::encode(project)),
        ),
        (Method::GET, "/v1/files/usage".into()),
        (Method::POST, "/v1/devices/enrollments".into()),
        (Method::POST, "/v1/pair".into()),
        (
            Method::POST,
            format!("/v1/devices/{}/revoke", alice.id_hex()),
        ),
    ];
    for (method, path) in routes {
        let (status, _) = bob
            .client
            .status(method.clone(), &path, Some(vec![0xa0]))
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
    }
    // Nor a new session.
    let mut again = Device {
        key: bob.key.clone(),
        id: bob.id.clone(),
        team: bob.team.clone(),
        client: s.client(),
    };
    assert_eq!(
        again.sign_in().await.expect_err("refused").0,
        StatusCode::UNAUTHORIZED
    );
    s.finish().await;
}

/// The HTTP head: 64 KiB at most, read within 15 s (`transport/listen.rs`). Once, the transport does not depend on the
/// base.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_head_of_a_request_is_bounded_in_size_and_in_time() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let s = TestServer::start(&Backend::Sqlite).await;
    // 70 KiB of headers: refused, or the connection closed.
    let mut tcp = tokio::net::TcpStream::connect(s.addr).await.unwrap();
    let mut head = String::from("GET /healthz HTTP/1.1\r\nHost: x\r\n");
    for i in 0..700 {
        head.push_str(&format!("x-filler-{i}: {}\r\n", "a".repeat(90)));
    }
    head.push_str("\r\n");
    let _ = tcp.write_all(head.as_bytes()).await;
    let mut buf = vec![0u8; 512];
    let n = tokio::time::timeout(Duration::from_secs(10), tcp.read(&mut buf))
        .await
        .expect("an answer or a close")
        .unwrap_or(0);
    let line = String::from_utf8_lossy(&buf[..n])
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    assert!(
        n == 0 || line.contains("431") || line.contains("400"),
        "{line}"
    );
    // A head that never ends: the connection is closed after 15 s.
    let mut tcp = tokio::net::TcpStream::connect(s.addr).await.unwrap();
    tcp.write_all(b"GET /healthz HTTP/1.1\r\nHost: x\r\n")
        .await
        .unwrap();
    let started = std::time::Instant::now();
    let n = tokio::time::timeout(Duration::from_secs(25), tcp.read(&mut buf))
        .await
        .expect("closed within 25 s")
        .unwrap_or(0);
    let line = String::from_utf8_lossy(&buf[..n])
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    assert!(n == 0 || line.contains("408"), "{line}");
    assert!(
        started.elapsed() >= Duration::from_secs(14),
        "closed after {:?}",
        started.elapsed()
    );
    s.finish().await;
}
