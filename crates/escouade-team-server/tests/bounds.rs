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
    a_device_asking_challenges_without_end_stops_no_other,
    the_pairing_relay_is_bounded_by_address,
    every_row_counts_in_the_projects_or_the_quota,
    the_team_token_serves_once_and_expires,
    the_revoked_devices_leave_the_bound,
    the_last_administrator_is_not_revoked,
);

/// SR2-06: a device that asks for challenges without end holds two of them, its newest: its own team's other devices
/// and the other teams sign in as before. (The bounds by team and on the whole server: `auth.rs`.)
async fn a_device_asking_challenges_without_end_stops_no_other(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let mut bob = s.add_device(&alice).await;
    let mut carol = s.new_team().await;
    let first = alice.challenge().await.ok();
    let mut last = None;
    for _ in 0..3000 {
        last = Some(alice.challenge().await.ok());
    }
    // Its first is gone, its last serves.
    let (status, _) = alice
        .session(&first.challenge, alice.sign(&first.challenge))
        .await
        .refused();
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let last = last.unwrap();
    let _: TokenResponse = alice
        .session(&last.challenge, alice.sign(&last.challenge))
        .await
        .ok();
    bob.sign_in().await.expect("its teammate signs in");
    carol.sign_in().await.expect("another team signs in");
    s.finish().await;
}

/// SR2-05: the steps of the pairing relay without a session (a slot's message, its try) have their own rate by address,
/// an IPv6 address counted by its /64; the answer goes only with the token of the try.
async fn the_pairing_relay_is_bounded_by_address(b: Backend) {
    let s = TestServer::start_with(&b, |c| {
        c.trust_proxy = true;
        c.rates.pairing.burst = 3.0;
        c.rates.pairing.per_second = 0.001;
    })
    .await;
    let alice = s.new_team().await;
    let opened: PairOpened = alice
        .client
        .post(
            "/v1/pair",
            &PairOpen {
                message: vec![1; 33],
            },
        )
        .await
        .ok();
    assert_eq!(opened.slot.len(), 5);
    let stranger = |from: &'static str, path: String| {
        let client = s.client();
        async move {
            client
                .request(Method::GET, &path)
                .header("x-forwarded-for", from)
                .send()
                .await
                .unwrap()
                .status()
        }
    };
    // Three looks from one /64, then it waits; changing address within it changes nothing.
    for n in 0..3 {
        let status = stranger("2001:db8:1:2::1", format!("/v1/pair/{:05}", n)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let status = stranger("2001:db8:1:2:ffff::9", format!("/v1/pair/{}", opened.slot)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    // Its try waits too.
    let tried = s
        .client()
        .request(Method::POST, &format!("/v1/pair/{}/try", opened.slot))
        .header("x-forwarded-for", "2001:db8:1:2::7")
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(&PairTry {
            message: vec![2; 33],
            confirm: vec![3; 32],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(tried.status(), StatusCode::TOO_MANY_REQUESTS);
    // Another address finds the slot (it has the code), tries, and takes the answer with its token only.
    let status = stranger("203.0.113.7", format!("/v1/pair/{}", opened.slot)).await;
    assert_eq!(status, StatusCode::OK);
    let tried: PairTried = s
        .client()
        .post(
            &format!("/v1/pair/{}/try", opened.slot),
            &PairTry {
                message: vec![2; 33],
                confirm: vec![3; 32],
            },
        )
        .await
        .ok();
    assert_eq!(tried.token.len(), 32, "128 bits");
    let (status, _) = s
        .client()
        .post::<_, PairAnswer>(
            &format!("/v1/pair/{}/take", opened.slot),
            &PairTake {
                token: hex::encode(random::<16>()),
            },
        )
        .await
        .refused();
    assert_eq!(status, StatusCode::FORBIDDEN);
    s.finish().await;
}

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

/// Reads until the server answers or closes: the status line of its answer, or None when it closed without one. Waits
/// `within` at most.
async fn answer_or_close(tcp: &mut tokio::net::TcpStream, within: Duration) -> Option<String> {
    use tokio::io::AsyncReadExt;
    let mut buf = vec![0u8; 512];
    let n = tokio::time::timeout(within, tcp.read(&mut buf))
        .await
        .expect("an answer or a close in time")
        .unwrap_or(0);
    (n > 0).then(|| {
        String::from_utf8_lossy(&buf[..n])
            .lines()
            .next()
            .unwrap_or_default()
            .to_string()
    })
}

/// SR2-07: a body that trickles in is cut at its time (`body_timeout`, plus its length at `body_min_rate`), on a route
/// without a session; the connection closes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_trickled_body_is_cut_at_its_time() {
    use tokio::io::AsyncWriteExt;
    let s = TestServer::start_with(&Backend::Sqlite, |c| {
        c.limits.body_timeout = Duration::from_secs(2);
        c.limits.body_min_rate = 1 << 30;
    })
    .await;
    let mut tcp = tokio::net::TcpStream::connect(s.addr).await.unwrap();
    tcp.write_all(
        b"POST /v1/enroll HTTP/1.1\r\nHost: x\r\nX-Escouade-Protocol: 1\r\nContent-Type: application/cbor\r\nContent-Length: 60000\r\n\r\n",
    )
    .await
    .unwrap();
    let started = std::time::Instant::now();
    let trickle = tokio::spawn(async move {
        for _ in 0..40 {
            if tcp.write_all(b"\0").await.is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        tcp
    });
    let mut tcp = trickle.await.unwrap();
    let line = answer_or_close(&mut tcp, Duration::from_secs(5)).await;
    let elapsed = started.elapsed();
    assert!(
        line.as_deref().is_none_or(|l| l.contains("408")),
        "{line:?}"
    );
    assert!(elapsed < Duration::from_secs(12), "cut after {elapsed:?}");
    s.finish().await;
}

/// SR2-07: a connection where nothing passes is closed at its idle time, after an answer as before a first request.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_silent_connection_is_closed_at_its_idle_time() {
    use tokio::io::AsyncWriteExt;
    let s = TestServer::start_with(&Backend::Sqlite, |c| {
        c.limits.idle_timeout = Duration::from_secs(2)
    })
    .await;
    let mut tcp = tokio::net::TcpStream::connect(s.addr).await.unwrap();
    tcp.write_all(b"GET /healthz HTTP/1.1\r\nHost: x\r\n\r\n")
        .await
        .unwrap();
    let line = answer_or_close(&mut tcp, Duration::from_secs(5)).await;
    assert!(
        line.as_deref().is_some_and(|l| l.contains("200")),
        "{line:?}"
    );
    // Kept alive, then silent: closed within its idle time, well before the 15 s of a head.
    let started = std::time::Instant::now();
    assert_eq!(
        answer_or_close(&mut tcp, Duration::from_secs(10)).await,
        None
    );
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(1500) && elapsed < Duration::from_secs(6),
        "closed after {elapsed:?}"
    );
    s.finish().await;
}

/// SR2-07: so many connections open from one address, and in all; past it, a new one is closed at once, and counted.
/// One that ends gives its place back. Behind a trusted proxy, only the bound in all holds.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_connections_are_bounded_by_address_and_in_all() {
    use std::sync::atomic::Ordering;
    use tokio::io::AsyncWriteExt;
    let open = |addr| async move { tokio::net::TcpStream::connect(addr).await.unwrap() };
    let healthy = |mut tcp: tokio::net::TcpStream| async move {
        tcp.write_all(b"GET /healthz HTTP/1.1\r\nHost: x\r\n\r\n")
            .await
            .unwrap();
        answer_or_close(&mut tcp, Duration::from_secs(5)).await
    };
    // By address.
    let s = TestServer::start_with(&Backend::Sqlite, |c| {
        c.limits.max_connections_per_address = 3
    })
    .await;
    let mut held = Vec::new();
    for _ in 0..3 {
        held.push(open(s.addr).await);
    }
    let mut fourth = open(s.addr).await;
    assert_eq!(
        answer_or_close(&mut fourth, Duration::from_secs(5)).await,
        None
    );
    assert_eq!(
        s.server
            .app()
            .metrics
            .connections_refused
            .load(Ordering::Relaxed),
        1
    );
    // One ends: its place serves again.
    drop(held.pop());
    tokio::time::sleep(Duration::from_millis(300)).await;
    let line = healthy(open(s.addr).await).await;
    assert!(
        line.as_deref().is_some_and(|l| l.contains("200")),
        "{line:?}"
    );
    s.finish().await;

    // In all, behind a proxy: the bound by address does not hold, the one in all does.
    let s = TestServer::start_with(&Backend::Sqlite, |c| {
        c.trust_proxy = true;
        c.limits.max_connections_per_address = 1;
        c.limits.max_connections = 3;
    })
    .await;
    let mut held = Vec::new();
    for _ in 0..2 {
        held.push(open(s.addr).await);
    }
    let line = healthy(open(s.addr).await).await;
    assert!(
        line.as_deref().is_some_and(|l| l.contains("200")),
        "{line:?}"
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    held.push(open(s.addr).await);
    let mut over = open(s.addr).await;
    assert_eq!(
        answer_or_close(&mut over, Duration::from_secs(5)).await,
        None
    );
    s.finish().await;
}

fn project_path(p: &[u8; 16], rest: &str) -> String {
    format!("/v1/projects/{}{rest}", hex::encode(p))
}

async fn take_claim(d: &Device, p: &[u8; 16]) -> Answer<ClaimAnswer> {
    d.client
        .post(
            &project_path(p, &format!("/claims/{}", hex::encode(random::<16>()))),
            &ClaimRequest { expected: None },
        )
        .await
}

async fn next_key(d: &Device, p: &[u8; 16]) -> Answer<KeyNumber> {
    d.client.post(&project_path(p, "/keys"), &()).await
}

/// Puts a project's access: the status, and the refusal's code.
async fn put_access(d: &Device, p: &[u8; 16], access: &AccessWire) -> (StatusCode, String) {
    let r = d
        .client
        .request(Method::PUT, &project_path(p, "/access"))
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(access))
        .send()
        .await
        .unwrap();
    let status = r.status();
    let body = r.bytes().await.unwrap();
    let code = ciborium::from_reader::<Problem, _>(body.as_ref())
        .map(|p| p.error)
        .unwrap_or_default();
    (status, code)
}

async fn used(d: &Device) -> u64 {
    let usage: Usage = d.client.get("/v1/files/usage").await.ok();
    usage.used
}

/// SR2-04: every write that makes a row makes its project known, counted in the team's projects (an unknown project
/// is refused beyond them, whatever the write), and the rows with no payload count in the team's quota; an access list
/// names each device once.
async fn every_row_counts_in_the_projects_or_the_quota(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.max_projects = 3).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    // A device listed twice.
    let twice = AccessWire {
        everyone: false,
        devices: vec![
            serde_bytes::ByteBuf::from(bob.id.clone()),
            serde_bytes::ByteBuf::from(bob.id.clone()),
        ],
    };
    assert_eq!(
        put_access(&alice, &random::<16>(), &twice).await,
        (StatusCode::BAD_REQUEST, "duplicate_device".into())
    );
    // A number, a claim, an access: three projects the server did not know, each counted.
    let before = used(&alice).await;
    next_key(&alice, &random::<16>()).await.ok();
    take_claim(&alice, &random::<16>()).await.ok();
    let once = AccessWire {
        everyone: false,
        devices: vec![serde_bytes::ByteBuf::from(bob.id.clone())],
    };
    assert_eq!(
        put_access(&alice, &random::<16>(), &once).await.0,
        StatusCode::NO_CONTENT
    );
    // The claim's row and the access list's bytes are in the team's usage.
    assert!(used(&alice).await >= before + 128 + 16);
    // A fourth project is refused, whatever its first write.
    let fourth = random::<16>();
    let too_many = |(status, p): (StatusCode, Problem)| {
        assert_eq!(
            (status, p.error.as_str()),
            (StatusCode::FORBIDDEN, "too_many")
        )
    };
    too_many(next_key(&alice, &fourth).await.refused());
    too_many(take_claim(&alice, &fourth).await.refused());
    too_many(alice.append_project(&fourth, b"op").await.refused());
    assert_eq!(
        put_access(&alice, &fourth, &once).await,
        (StatusCode::FORBIDDEN, "too_many".into())
    );
    s.finish().await;

    // The claims fill the quota like any byte: beyond it, refused.
    let s = TestServer::start_with(&b, |c| c.limits.team_quota = 300).await;
    let alice = s.new_team().await;
    let project = random::<16>();
    take_claim(&alice, &project).await.ok();
    take_claim(&alice, &project).await.ok();
    let (status, p) = take_claim(&alice, &project).await.refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::INSUFFICIENT_STORAGE, "quota")
    );
    s.finish().await;
}

/// SR2-11: the token of `init` serves once, and lasts 24 hours: past them, it creates no team.
async fn the_team_token_serves_once_and_expires(b: Backend) {
    use escouade_team_server::db::{Quotas, StoreError, TEAM_TOKEN_TTL};
    use escouade_team_server::util::{now, token_hash};
    let s = TestServer::start(&b).await;
    assert_eq!(TEAM_TOKEN_TTL, 24 * 3600);
    // Once.
    let token = s.team_token().await;
    Device::enroll(&s, &token).await.expect("it serves");
    let (status, p) = Device::enroll(&s, &token).await.err().expect("refused");
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::CONFLICT, "token_used")
    );
    // Unused, past its 24 hours: refused, as unknown.
    let token = s.team_token().await;
    let quotas = Quotas {
        team_bytes: 1 << 20,
        devices: 10,
        projects: 10,
    };
    let late = s
        .server
        .app()
        .store
        .enroll(
            token_hash(&token),
            random(),
            now() + TEAM_TOKEN_TTL + 1,
            quotas,
        )
        .await;
    assert!(matches!(late, Err(StoreError::BadToken)), "{late:?}");
    let (status, p) = Device::enroll(&s, &token).await.err().expect("refused");
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::UNAUTHORIZED, "bad_token")
    );
    s.finish().await;
}

/// SR2-12: the revoked devices are kept (the history of the team) and leave the bound of its devices: a team that
/// renews its devices enrols new ones.
async fn the_revoked_devices_leave_the_bound(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.max_devices = 2).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let token = alice.enrollment_token().await;
    assert_eq!(
        Device::enroll(&s, &token).await.err().expect("refused").0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(alice.revoke(&bob).await, StatusCode::NO_CONTENT);
    let carol = s.add_device(&alice).await;
    let devices: Devices = alice.client.get("/v1/devices").await.ok();
    assert_eq!(devices.devices.len(), 3);
    assert!(devices.devices.iter().any(|d| d.id == bob.id && d.revoked));
    assert!(devices
        .devices
        .iter()
        .any(|d| d.id == carol.id && !d.revoked));
    s.finish().await;
}

async fn set_role(
    admin: &Device,
    device: &Device,
    role: escouade_team_server::db::Role,
) -> StatusCode {
    admin
        .client
        .request(
            Method::POST,
            &format!("/v1/devices/{}/role", device.id_hex()),
        )
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(
            &RoleRequest { role },
        ))
        .send()
        .await
        .unwrap()
        .status()
}

/// SR2-13: the team's last administrator on the server is not revoked, by itself or another administrator, and is
/// told why; once another is named, it is.
async fn the_last_administrator_is_not_revoked(b: Backend) {
    use escouade_team_server::db::Role;
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let (status, body) = alice
        .client
        .status(
            Method::POST,
            &format!("/v1/devices/{}/revoke", alice.id_hex()),
            None,
        )
        .await;
    let p: Problem = ciborium::from_reader(body.as_slice()).unwrap();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::CONFLICT, "last_admin")
    );
    assert!(p.message.contains("one administrator"), "{}", p.message);
    // Bob named administrator: Alice goes, then Bob is the last and stays.
    assert_eq!(
        set_role(&alice, &bob, Role::Admin).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(bob.revoke(&alice).await, StatusCode::NO_CONTENT);
    assert_eq!(bob.revoke(&bob).await, StatusCode::CONFLICT);
    let devices: Devices = bob.client.get("/v1/devices").await.ok();
    assert!(devices
        .devices
        .iter()
        .any(|d| d.id == bob.id && !d.revoked && d.role == Role::Admin));
    s.finish().await;
}
