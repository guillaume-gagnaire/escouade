//! The server on localhost, through its HTTP API and its WebSocket: each test once on a temporary SQLite base, and
//! once on PostgreSQL when `ESCOUADE_TEST_POSTGRES_URL` is set.

#[macro_use]
mod common;

use std::time::Duration;

use common::*;
use escouade_team_server::api::wire::*;
use escouade_team_server::db::{Kind, Role};
use escouade_team_server::live::{CLOSE_REVOKED, CLOSE_SESSION_ENDED};
use escouade_team_server::transport::cbor;
use escouade_team_server::util::random;
use reqwest::{Method, StatusCode};

both!(
    the_init_token_enrols_once,
    an_administrator_adds_a_device_and_revokes_it,
    the_signed_challenge_opens_a_short_session,
    the_streams_are_numbered_and_read_again_from_n,
    the_first_commit_of_an_epoch_wins,
    files_and_quotas,
    the_live_connection_receives_the_new_entries,
    the_live_connection_closes_with_the_session,
    the_versions_are_checked,
    the_rates_are_limited,
    the_sizes_are_bounded,
    the_teams_do_not_see_each_other,
);

async fn the_init_token_enrols_once(b: Backend) {
    let s = TestServer::start(&b).await;
    let token = s.team_token().await;
    let admin = Device::enroll(&s, &token).await.expect("first use");
    assert_eq!(admin.id.len(), 16);
    assert_eq!(admin.team.len(), 16);
    // A second time: refused, and said used (TS-3), not unknown.
    let (status, p) = Device::enroll(&s, &token).await.err().unwrap();
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(p.error, "token_used");
    // A made-up token, a key that is not one: refused.
    assert_eq!(
        Device::enroll(&s, "nope").await.err().unwrap().0,
        StatusCode::UNAUTHORIZED
    );
    let r: Answer<EnrollResponse> = s
        .client()
        .post(
            "/v1/enroll",
            &EnrollRequest {
                token: s.team_token().await,
                public_key: vec![1; 31],
            },
        )
        .await;
    assert_eq!(r.refused().1.error, "bad_key");
    // The admin is one.
    let mut admin = admin;
    admin.sign_in().await.unwrap();
    let devices: Devices = admin.client.get("/v1/devices").await.ok();
    assert_eq!(devices.devices.len(), 1);
    assert_eq!(devices.devices[0].role, Role::Admin);
    assert_eq!(
        devices.devices[0].public_key,
        admin.key.verifying_key().to_bytes()
    );
    s.finish().await;
}

async fn an_administrator_adds_a_device_and_revokes_it(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let token = admin.enrollment_token().await;
    let mut bob = Device::enroll(&s, &token).await.unwrap();
    assert_eq!(bob.team, admin.team);
    // The enrolment token is used once.
    assert_eq!(
        Device::enroll(&s, &token).await.err().unwrap().0,
        StatusCode::CONFLICT
    );
    bob.sign_in().await.unwrap();
    // A member neither adds a device nor revokes another one.
    let r: Answer<TokenResponse> = bob
        .client
        .post("/v1/devices/enrollments", &EnrollmentRequest::default())
        .await;
    assert_eq!(r.refused().0, StatusCode::FORBIDDEN);
    assert_eq!(bob.revoke(&admin).await, StatusCode::FORBIDDEN);
    let devices: Devices = bob.client.get("/v1/devices").await.ok();
    assert_eq!(devices.devices.len(), 2);
    let bob_id = bob.id.clone();
    let of_bob = move |d: &Devices| {
        d.devices
            .iter()
            .find(|x| x.id == bob_id)
            .map(|x| (x.role, x.revoked))
            .unwrap()
    };
    assert_eq!(of_bob(&devices), (Role::Member, false));
    // The admin revokes Bob: his session ends, he cannot sign in again.
    assert_eq!(admin.revoke(&bob).await, StatusCode::NO_CONTENT);
    let (status, _) = bob.client.get::<Devices>("/v1/devices").await.refused();
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        bob.sign_in().await.err().unwrap().0,
        StatusCode::UNAUTHORIZED
    );
    let devices: Devices = admin.client.get("/v1/devices").await.ok();
    assert_eq!(of_bob(&devices), (Role::Member, true));
    // An enrolment token past its life is refused.
    let r: TokenResponse = admin
        .client
        .post(
            "/v1/devices/enrollments",
            &EnrollmentRequest {
                expires_in: Some(1),
                token_hash: None,
            },
        )
        .await
        .ok();
    tokio::time::sleep(Duration::from_millis(2100)).await;
    assert_eq!(
        Device::enroll(&s, &r.token).await.err().unwrap().0,
        StatusCode::UNAUTHORIZED
    );
    s.finish().await;
}

async fn the_signed_challenge_opens_a_short_session(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.session_ttl = Duration::from_secs(1)).await;
    let token = s.team_token().await;
    let mut device = Device::enroll(&s, &token).await.unwrap();

    // No session: every route but the open ones refuses.
    for path in ["/v1/devices", "/v1/team/stream", "/v1/files/usage"] {
        assert_eq!(
            device.client.status(Method::GET, path, None).await.0,
            StatusCode::UNAUTHORIZED,
            "{path}"
        );
    }
    device.client.token = Some("made-up".into());
    assert_eq!(
        device
            .client
            .status(Method::GET, "/v1/devices", None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    device.client.token = None;

    // A bad signature: refused, and the challenge is spent.
    let c = device.challenge().await.ok();
    let mut bad = device.sign(&c.challenge);
    bad[0] ^= 1;
    assert_eq!(
        device.session(&c.challenge, bad).await.refused().0,
        StatusCode::UNAUTHORIZED
    );
    let good = device.sign(&c.challenge);
    assert_eq!(
        device.session(&c.challenge, good).await.refused().0,
        StatusCode::UNAUTHORIZED
    );
    // Another device's signature of this device's challenge: refused.
    let other = ed25519_dalek::SigningKey::from_bytes(&random::<32>());
    let c = device.challenge().await.ok();
    let id: [u8; 16] = device.id.as_slice().try_into().unwrap();
    let forged = ed25519_dalek::Signer::sign(
        &other,
        &escouade_team_server::auth::signed_message(
            &id,
            &c.challenge.as_slice().try_into().unwrap(),
        ),
    );
    assert_eq!(
        device
            .session(&c.challenge, forged.to_bytes().to_vec())
            .await
            .refused()
            .0,
        StatusCode::UNAUTHORIZED
    );
    // A made-up challenge: refused. An unknown device gets no challenge.
    let made_up = random::<32>().to_vec();
    let sig = device.sign(&made_up);
    assert_eq!(
        device.session(&made_up, sig).await.refused().0,
        StatusCode::UNAUTHORIZED
    );
    let r: Answer<ChallengeResponse> = device
        .client
        .post(
            "/v1/auth/challenge",
            &ChallengeRequest {
                device: random::<16>().to_vec(),
            },
        )
        .await;
    assert_eq!(r.refused().0, StatusCode::UNAUTHORIZED);

    // The good signature opens a session, which ends after its life.
    device.sign_in().await.unwrap();
    let devices: Devices = device.client.get("/v1/devices").await.ok();
    assert_eq!(devices.devices.len(), 1);
    tokio::time::sleep(Duration::from_millis(1300)).await;
    let (status, p) = device.client.get::<Devices>("/v1/devices").await.refused();
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(p.error, "unauthorized");
    device.sign_in().await.unwrap();
    device.client.get::<Devices>("/v1/devices").await.ok();

    // A revoked device gets no challenge, and its open session is refused.
    let admin = device;
    assert_eq!(admin.revoke(&admin).await, StatusCode::NO_CONTENT);
    assert_eq!(
        admin
            .client
            .status(Method::GET, "/v1/devices", None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        admin.challenge().await.refused().0,
        StatusCode::UNAUTHORIZED
    );
    s.finish().await;
}

async fn the_streams_are_numbered_and_read_again_from_n(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.page_entries = 3).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    for i in 1..=5u8 {
        let who = if i % 2 == 0 { &bob } else { &alice };
        assert_eq!(who.message(&[i; 10]).await, i as u64);
    }
    // A page of 3, then from 3 on.
    let page = bob.read_team(0).await;
    assert_eq!(
        page.entries.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(page.more);
    assert_eq!(page.entries[1].device, bob.id);
    assert_eq!(page.entries[0].payload, [1; 10]);
    assert_eq!(page.entries[0].kind, Kind::Message);
    let rest = bob.read_team(3).await;
    assert_eq!(
        rest.entries.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [4, 5]
    );
    assert!(!rest.more);
    assert!(bob.read_team(5).await.entries.is_empty());
    // A smaller limit.
    let one: Entries = bob.client.get("/v1/team/stream?since=0&limit=1").await.ok();
    assert_eq!(one.entries.len(), 1);

    // Each project has its own numbering.
    let (p1, p2) = (random::<16>(), random::<16>());
    assert_eq!(alice.append_project(&p1, b"op 1").await.ok().seq, 1);
    assert_eq!(bob.append_project(&p1, b"op 2").await.ok().seq, 2);
    assert_eq!(bob.append_project(&p2, b"other").await.ok().seq, 1);
    let ops = alice.read_project(&p1, 1).await;
    assert_eq!(ops.entries.len(), 1);
    assert_eq!(ops.entries[0].payload, b"op 2");
    assert_eq!(alice.read_project(&p2, 0).await.entries.len(), 1);
    assert!(alice
        .read_project(&random::<16>(), 0)
        .await
        .entries
        .is_empty());
    // The team's stream did not move.
    assert_eq!(alice.read_team(5).await.entries.len(), 0);
    // No commit in a project's stream, no epoch on a message, no commit without one.
    let r: Answer<Appended> = alice
        .client
        .post(
            &format!("/v1/projects/{}/stream", hex::encode(p1)),
            &AppendRequest {
                kind: Kind::Commit,
                epoch: Some(0),
                payload: vec![1],
            },
        )
        .await;
    assert_eq!(r.refused().1.error, "no_commit_here");
    assert_eq!(
        alice
            .append_team(Kind::Message, Some(3), b"x")
            .await
            .refused()
            .1
            .error,
        "epoch_on_message"
    );
    assert_eq!(
        alice
            .append_team(Kind::Commit, None, b"x")
            .await
            .refused()
            .1
            .error,
        "no_epoch"
    );
    assert_eq!(
        alice
            .append_team(Kind::Message, None, b"")
            .await
            .refused()
            .1
            .error,
        "empty"
    );
    s.finish().await;
}

async fn the_first_commit_of_an_epoch_wins(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    assert_eq!(alice.commit(0, b"alice: epoch 0 -> 1").await.ok().seq, 1);
    alice.message(b"welcome for someone").await;
    // Bob's commit for the same epoch is refused, with what preceded it.
    let (status, p) = bob.commit(0, b"bob: epoch 0 -> 1").await.refused();
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(p.error, "epoch_taken");
    let detail: EpochTaken = p.detail.unwrap().deserialized().unwrap();
    assert_eq!(detail.expected_epoch, 1);
    assert_eq!(
        detail.preceding.iter().map(|e| e.seq).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(detail.preceding[0].payload, b"alice: epoch 0 -> 1");
    assert_eq!(detail.preceding[0].epoch, Some(0));
    assert_eq!(detail.preceding[0].kind, Kind::Commit);
    // Bob catches up and commits the next epoch.
    assert_eq!(bob.commit(1, b"bob: epoch 1 -> 2").await.ok().seq, 3);
    // An epoch ahead is refused too, with nothing before it.
    let (_, p) = alice.commit(7, b"ahead").await.refused();
    let detail: EpochTaken = p.detail.unwrap().deserialized().unwrap();
    assert_eq!(detail.expected_epoch, 2);
    assert!(detail.preceding.is_empty());

    // Under concurrency: eight commits for epoch 2, one wins.
    let mut tasks = Vec::new();
    for i in 0..8u8 {
        let client = if i % 2 == 0 {
            alice.client.clone()
        } else {
            bob.client.clone()
        };
        tasks.push(tokio::spawn(async move {
            let r: Answer<Appended> = client
                .post(
                    "/v1/team/stream",
                    &AppendRequest {
                        kind: Kind::Commit,
                        epoch: Some(2),
                        payload: vec![i; 8],
                    },
                )
                .await;
            matches!(r, Answer::Ok(_))
        }));
    }
    let mut won = 0;
    for t in tasks {
        won += t.await.unwrap() as u32;
    }
    assert_eq!(won, 1);
    let all = alice.read_team(0).await;
    let commits: Vec<_> = all
        .entries
        .iter()
        .filter(|e| e.kind == Kind::Commit)
        .map(|e| e.epoch.unwrap())
        .collect();
    assert_eq!(commits, [0, 1, 2]);
    s.finish().await;
}

async fn files_and_quotas(b: Backend) {
    let s = TestServer::start_with(&b, |c| {
        c.limits.max_file = 1000;
        c.limits.team_quota = 2500;
        if let Some(s3) = s3() {
            c.files = s3;
        }
    })
    .await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let (f1, f2, f3) = (random::<16>(), random::<16>(), random::<16>());
    let bytes: Vec<u8> = (0..1000).map(|i| (i * 7) as u8).collect();
    assert_eq!(
        alice.put_file(&f1, bytes.clone()).await,
        StatusCode::CREATED
    );
    let (status, got) = bob.get_file(&f1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got, bytes);
    // A file is written once.
    assert_eq!(bob.put_file(&f1, vec![1; 10]).await, StatusCode::CONFLICT);
    // Above the bound of a file.
    assert_eq!(
        alice.put_file(&f2, vec![0; 1001]).await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(alice.put_file(&f2, vec![]).await, StatusCode::BAD_REQUEST);
    // The quota: 1000 + 1000 fit, then a third does not (the stream's bytes count too).
    assert_eq!(
        alice.put_file(&f2, vec![2; 1000]).await,
        StatusCode::CREATED
    );
    assert_eq!(
        alice.put_file(&f3, vec![3; 600]).await,
        StatusCode::INSUFFICIENT_STORAGE
    );
    alice.message(&[9; 400]).await;
    let usage: Usage = alice.client.get("/v1/files/usage").await.ok();
    assert_eq!(
        (usage.used, usage.quota, usage.max_file),
        (2400, 2500, 1000)
    );
    assert_eq!(
        alice
            .append_team(Kind::Message, None, &[9; 200])
            .await
            .refused()
            .0,
        StatusCode::INSUFFICIENT_STORAGE
    );
    // A deleted file gives its place back.
    assert_eq!(bob.delete_file(&f2).await, StatusCode::NO_CONTENT);
    assert_eq!(bob.delete_file(&f2).await, StatusCode::NOT_FOUND);
    assert_eq!(alice.get_file(&f2).await.0, StatusCode::NOT_FOUND);
    assert_eq!(alice.put_file(&f3, vec![3; 600]).await, StatusCode::CREATED);
    assert_eq!(alice.get_file(&f3).await.1, vec![3; 600]);
    // A bad id.
    assert_eq!(
        alice
            .client
            .status(Method::GET, "/v1/files/zz", None)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    s.finish().await;
}

async fn the_live_connection_receives_the_new_entries(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let outsider = s.new_team().await;
    let mut bob_live = bob.live(&s).await;
    let mut outsider_live = outsider.live(&s).await;
    // Give the subscriptions a moment.
    tokio::time::sleep(Duration::from_millis(100)).await;

    alice.message(b"one").await;
    let project = random::<16>();
    alice.append_project(&project, b"op").await.ok();
    alice.commit(0, b"commit").await.ok();

    let f = bob_live.frame().await;
    assert_eq!(f.stream, "team");
    let e = f.entry.unwrap();
    assert_eq!(
        (e.seq, e.payload.as_slice(), e.device.clone()),
        (1, b"one".as_slice(), alice.id.clone())
    );
    let f = bob_live.frame().await;
    assert_eq!(f.stream, "project");
    assert_eq!(f.project.unwrap(), project);
    assert_eq!(f.entry.unwrap().payload, b"op");
    let f = bob_live.frame().await;
    let e = f.entry.unwrap();
    assert_eq!((e.seq, e.kind, e.epoch), (2, Kind::Commit, Some(0)));
    // Another team hears nothing.
    assert!(outsider_live
        .next(Duration::from_millis(300))
        .await
        .is_none());
    // A revoked device's connection closes.
    assert_eq!(alice.revoke(&bob).await, StatusCode::NO_CONTENT);
    match bob_live.next(Duration::from_secs(5)).await {
        Some(LiveEvent::Closed(code)) => assert_eq!(code, Some(CLOSE_REVOKED)),
        _ => panic!("the connection stays open"),
    }
    // Without a session, no live connection.
    let mut request =
        tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(format!(
            "ws://{}/v1/live",
            s.addr
        ))
        .unwrap();
    request
        .headers_mut()
        .insert("escouade-protocol", "1".parse().unwrap());
    assert!(tokio_tungstenite::connect_async(request).await.is_err());
    s.finish().await;
}

async fn the_live_connection_closes_with_the_session(b: Backend) {
    let s = TestServer::start_with(&b, |c| {
        c.limits.session_ttl = Duration::from_secs(1);
        c.limits.max_live_per_device = 2;
    })
    .await;
    let alice = s.new_team().await;
    let mut live = alice.live(&s).await;
    let _second = alice.live(&s).await;
    // A third connection of the same device is refused.
    let mut request =
        tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(format!(
            "ws://{}/v1/live",
            s.addr
        ))
        .unwrap();
    request
        .headers_mut()
        .insert("escouade-protocol", "1".parse().unwrap());
    request.headers_mut().insert(
        "authorization",
        format!("Bearer {}", alice.client.token.as_ref().unwrap())
            .parse()
            .unwrap(),
    );
    assert!(tokio_tungstenite::connect_async(request).await.is_err());
    match live.next(Duration::from_secs(5)).await {
        Some(LiveEvent::Closed(code)) => assert_eq!(code, Some(CLOSE_SESSION_ENDED)),
        _ => panic!("the connection outlives its session"),
    }
    s.finish().await;
}

async fn the_versions_are_checked(b: Backend) {
    let s = TestServer::start(&b).await;
    let mut client = s.client();
    // /healthz needs no version, no session.
    client.protocol = None;
    let (status, body) = client.status(Method::GET, "/healthz", None).await;
    assert_eq!(
        (status, body.as_slice()),
        (StatusCode::OK, b"ok\n".as_slice())
    );
    // No version, an older one: « Mets Escouade à jour ».
    for protocol in [None, Some(0)] {
        client.protocol = protocol;
        let (status, p) = client
            .post::<_, EnrollResponse>(
                "/v1/enroll",
                &EnrollRequest {
                    token: "x".into(),
                    public_key: vec![],
                },
            )
            .await
            .refused();
        assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(p.error, "client_too_old");
        assert_eq!(p.message, "Mets Escouade à jour");
    }
    // A newer one: the server is late.
    client.protocol = Some(escouade_team_server::PROTOCOL + 1);
    let r = client
        .request(Method::GET, "/v1/devices")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        r.headers()["escouade-protocol"],
        escouade_team_server::PROTOCOL.to_string().as_str()
    );
    let p: Problem = cbor::decode(&r.bytes().await.unwrap()).unwrap();
    assert_eq!(p.error, "server_too_old");
    // Its own: through (then refused for want of a session).
    client.protocol = Some(escouade_team_server::PROTOCOL);
    assert_eq!(
        client.status(Method::GET, "/v1/devices", None).await.0,
        StatusCode::UNAUTHORIZED
    );
    s.finish().await;
}

async fn the_rates_are_limited(b: Backend) {
    // By address.
    let s = TestServer::start_with(&b, |c| {
        c.rates.per_address.burst = 5.0;
        c.rates.per_address.per_second = 0.5;
    })
    .await;
    let client = s.client();
    for _ in 0..5 {
        assert_eq!(
            client.status(Method::GET, "/healthz", None).await.0,
            StatusCode::OK
        );
    }
    let r = client
        .request(Method::GET, "/healthz")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(r.headers().contains_key("retry-after"));
    s.finish().await;

    // By device: one device is held back, another of the same address is not.
    let s = TestServer::start_with(&b, |c| {
        c.rates.per_device.burst = 3.0;
        c.rates.per_device.per_second = 0.5;
    })
    .await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await; // alice's first authenticated request
    let mut statuses = Vec::new();
    for _ in 0..3 {
        statuses.push(
            alice
                .client
                .status(Method::GET, "/v1/devices", None)
                .await
                .0,
        );
    }
    assert_eq!(
        statuses,
        [
            StatusCode::OK,
            StatusCode::OK,
            StatusCode::TOO_MANY_REQUESTS
        ]
    );
    assert_eq!(
        bob.client.status(Method::GET, "/v1/devices", None).await.0,
        StatusCode::OK
    );
    s.finish().await;
}

/// Writes a raw request, and reads the status line of the answer.
async fn raw(addr: std::net::SocketAddr, head: &str, body: &[u8]) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream.write_all(head.as_bytes()).await.unwrap();
    stream.write_all(body).await.unwrap();
    let mut buf = vec![0u8; 512];
    let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf))
        .await
        .expect("an answer without waiting for the announced body")
        .unwrap();
    String::from_utf8_lossy(&buf[..n])
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

async fn the_sizes_are_bounded(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.max_entry = 1000).await;
    let alice = s.new_team().await;
    // An entry above its bound.
    let (status, p) = alice
        .append_team(Kind::Message, None, &[0; 1001])
        .await
        .refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::PAYLOAD_TOO_LARGE, "too_large")
    );
    alice
        .append_team(Kind::Message, None, &[0; 1000])
        .await
        .ok();
    let token = alice.client.token.clone().unwrap();
    // A length announced far too large is refused at once, without reading.
    let line = raw(
        s.addr,
        &format!(
            "POST /v1/team/stream HTTP/1.1\r\nHost: x\r\nescouade-protocol: 1\r\nAuthorization: Bearer {token}\r\n\
             Content-Type: application/cbor\r\nContent-Length: 10000000000\r\n\r\n"
        ),
        b"\xa0",
    )
    .await;
    assert!(line.contains("413"), "{line}");
    let line = raw(
        s.addr,
        &format!(
            "PUT /v1/files/{} HTTP/1.1\r\nHost: x\r\nescouade-protocol: 1\r\nAuthorization: Bearer {token}\r\n\
             Content-Length: 999999999999\r\n\r\n",
            hex::encode(random::<16>())
        ),
        b"abc",
    )
    .await;
    assert!(line.contains("413"), "{line}");
    // The open routes too.
    let line = raw(
        s.addr,
        "POST /v1/enroll HTTP/1.1\r\nHost: x\r\nescouade-protocol: 1\r\nContent-Type: application/cbor\r\n\
         Content-Length: 70000\r\n\r\n",
        b"",
    )
    .await;
    assert!(line.contains("413"), "{line}");
    // A CBOR byte string announcing 2^62 bytes in a body of a few: refused, and the server stands.
    let mut bomb = vec![0xa3u8]; // a map of three
    bomb.extend_from_slice(b"\x64kind\x67message");
    bomb.extend_from_slice(b"\x67payload");
    bomb.extend_from_slice(&[0x5b, 0x40, 0, 0, 0, 0, 0, 0, 0]); // bytes, length 2^62
    bomb.extend_from_slice(b"abc");
    let r = alice
        .client
        .request(Method::POST, "/v1/team/stream")
        .header("content-type", "application/cbor")
        .body(bomb)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    // An array announcing 2^62 items.
    let r = alice
        .client
        .request(Method::POST, "/v1/devices/enrollments")
        .header("content-type", "application/cbor")
        .body(vec![0x9b, 0x40, 0, 0, 0, 0, 0, 0, 0, 1])
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    // Not CBOR.
    let r = alice
        .client
        .request(Method::POST, "/v1/team/stream")
        .header("content-type", "application/json")
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(
        s.client().status(Method::GET, "/healthz", None).await.0,
        StatusCode::OK
    );
    s.finish().await;
}

async fn the_teams_do_not_see_each_other(b: Backend) {
    let s = TestServer::start(&b).await;
    let a = s.new_team().await;
    let z = s.new_team().await;
    assert_ne!(a.team, z.team);
    a.message(b"a's").await;
    // Each team has its own numbering, and reads only its own.
    assert_eq!(z.message(b"z's").await, 1);
    assert_eq!(z.read_team(0).await.entries.len(), 1);
    let project = random::<16>();
    a.append_project(&project, b"a's op").await.ok();
    assert!(z.read_project(&project, 0).await.entries.is_empty());
    let file = random::<16>();
    assert_eq!(a.put_file(&file, vec![1; 10]).await, StatusCode::CREATED);
    assert_eq!(z.get_file(&file).await.0, StatusCode::NOT_FOUND);
    assert_eq!(z.delete_file(&file).await, StatusCode::NOT_FOUND);
    // Z's admin cannot revoke A's device.
    assert_eq!(z.revoke(&a).await, StatusCode::NOT_FOUND);
    let devices: Devices = z.client.get("/v1/devices").await.ok();
    assert_eq!(devices.devices.len(), 1);
    s.finish().await;
}
