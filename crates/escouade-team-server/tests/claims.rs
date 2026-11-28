//! TS-6: the claims of the tickets (§4.2), the projects' access (§2.6) and the tickets' counter (§4.1), against the
//! real server, on SQLite then on PostgreSQL.

#[macro_use]
mod common;

use std::time::Duration;

use common::*;
use escouade_team_server::api::wire::*;
use escouade_team_server::db::{Claim, Kind};
use escouade_team_server::util::random;
use reqwest::StatusCode;

both!(
    two_claims_at_once_one_passes,
    a_lease_runs_out_and_is_given_back,
    a_write_under_a_stale_token_is_refused,
    a_project_kept_to_some_devices_refuses_the_others,
    a_revoked_holder_holds_nothing,
    the_counter_gives_each_number_once,
    a_claim_goes_out_live_to_the_projects_devices_alone,
    a_ticket_handed_goes_to_its_new_holder,
);

fn path(p: &[u8; 16], rest: &str) -> String {
    format!("/v1/projects/{}{rest}", hex::encode(p))
}

async fn take(
    d: &Device,
    p: &[u8; 16],
    t: &[u8; 16],
    expected: Option<u64>,
) -> Answer<ClaimAnswer> {
    d.client
        .post(
            &path(p, &format!("/claims/{}", hex::encode(t))),
            &ClaimRequest { expected },
        )
        .await
}

async fn ask(
    d: &Device,
    p: &[u8; 16],
    t: &[u8; 16],
    what: &str,
    token: u64,
) -> Answer<ClaimAnswer> {
    d.client
        .post(
            &path(p, &format!("/claims/{}/{what}", hex::encode(t))),
            &TokenRequest { token },
        )
        .await
}

/// Sets a project's access: the status, and the refusal's code.
async fn put_access(d: &Device, p: &[u8; 16], access: &AccessWire) -> (StatusCode, String) {
    let r = d
        .client
        .request(reqwest::Method::PUT, &path(p, "/access"))
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(access))
        .send()
        .await
        .unwrap();
    let status = r.status();
    let bytes = r.bytes().await.unwrap();
    if status == StatusCode::NO_CONTENT {
        return (status, String::new());
    }
    let problem: Problem = ciborium::from_reader(&bytes[..]).unwrap();
    (status, problem.error)
}

fn detail(problem: &Problem) -> Claim {
    let v = problem.detail.clone().expect("a claim as detail");
    v.deserialized().expect("a claim")
}

fn id(d: &Device) -> [u8; 16] {
    d.id.clone().try_into().unwrap()
}

/// Ten devices ask for the same ticket at the same time: one passes, the nine others are told which device holds it.
async fn two_claims_at_once_one_passes(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let mut devices = vec![];
    for _ in 0..9 {
        devices.push(s.add_device(&alice).await);
    }
    devices.push(alice);
    let (p, t) = (random::<16>(), random::<16>());
    let asks = devices.iter().map(|d| take(d, &p, &t, None));
    let answers = futures_util::future::join_all(asks).await;
    let mut won = vec![];
    let mut refused = vec![];
    for (d, a) in devices.iter().zip(answers) {
        match a {
            Answer::Ok(c) => won.push((id(d), c)),
            Answer::No(status, problem) => {
                assert_eq!(
                    (status, problem.error.as_str()),
                    (StatusCode::CONFLICT, "taken")
                );
                refused.push(detail(&problem));
            }
        }
    }
    assert_eq!(won.len(), 1, "a single claim passes");
    let (holder, claim) = &won[0];
    assert_eq!((claim.claim.holder, claim.claim.token), (Some(*holder), 1));
    assert_eq!(claim.claim.until - claim.now, 600, "a lease of 10 minutes");
    assert_eq!(refused.len(), 9);
    assert!(refused.iter().all(|c| c.holder == Some(*holder)));
    // Another one asks again: still taken. The holder asks again: still its, under a new token.
    let other = devices.iter().find(|d| id(d) != *holder).unwrap();
    assert_eq!(take(other, &p, &t, None).await.refused().1.error, "taken");
    let holder_device = devices.iter().find(|d| id(d) == *holder).unwrap();
    let mine = take(holder_device, &p, &t, None).await.ok();
    assert_eq!(mine.claim.token, 2);
    s.finish().await;
}

/// A lease renewed while the agent runs, given back after; another one's lease run out frees the ticket, which shows
/// whose it was (« interrompu chez Alice »).
async fn a_lease_runs_out_and_is_given_back(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.claim_lease = Duration::from_secs(2)).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let (p, t) = (random::<16>(), random::<16>());
    let a = take(&alice, &p, &t, None).await.ok();
    let renewed = ask(&alice, &p, &t, "renew", a.claim.token).await.ok();
    assert!(renewed.claim.until >= a.claim.until);
    assert_eq!(take(&bob, &p, &t, None).await.refused().1.error, "taken");
    tokio::time::sleep(Duration::from_millis(3100)).await;
    // Its lease over: listed with its holder still, then free to take.
    let list: Claims = bob.client.get(&path(&p, "/claims")).await.ok();
    let c = list.claims.iter().find(|c| c.ticket == t).unwrap();
    assert_eq!(c.holder, Some(id(&alice)));
    assert!(c.until <= list.now, "the lease is over");
    let b2 = take(&bob, &p, &t, Some(c.version)).await.ok();
    assert_eq!(
        (b2.claim.holder, b2.claim.token, b2.claim.last_holder),
        (Some(id(&bob)), 2, Some(id(&alice)))
    );
    // Alice's renewal comes too late: the ticket is Bob's.
    assert_eq!(
        ask(&alice, &p, &t, "renew", 1).await.refused().1.error,
        "claim_lost"
    );
    let back = ask(&bob, &p, &t, "release", 2).await.ok();
    assert_eq!((back.claim.holder, back.claim.until), (None, 0));
    // A claim that expects the version before the release is refused: the ticket moved on.
    let stale = take(&alice, &p, &t, Some(b2.claim.version)).await.refused();
    assert_eq!(stale.1.error, "stale_version");
    assert!(
        take(&alice, &p, &t, Some(back.claim.version))
            .await
            .ok()
            .claim
            .token
            == 3
    );
    s.finish().await;
}

async fn fenced_append(d: &Device, p: &[u8; 16], t: &[u8; 16], token: u64) -> Answer<Appended> {
    d.client
        .post(
            &path(p, "/stream"),
            &AppendRequest {
                own: false,
                lease: None,
                kind: Kind::Message,
                epoch: None,
                payload: vec![7; 40],
                fence: Some(FenceWire {
                    ticket: t.to_vec(),
                    token,
                }),
            },
        )
        .await
}

/// A device back from a cut, while another took the ticket over, no longer moves it: its write is refused.
async fn a_write_under_a_stale_token_is_refused(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.claim_lease = Duration::from_secs(2)).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let (p, t) = (random::<16>(), random::<16>());
    let a = take(&alice, &p, &t, None).await.ok();
    assert!(matches!(
        fenced_append(&alice, &p, &t, a.claim.token).await,
        Answer::Ok(_)
    ));
    tokio::time::sleep(Duration::from_millis(3100)).await;
    // Its lease over but nobody took it: the holder's write still passes.
    assert!(matches!(
        fenced_append(&alice, &p, &t, a.claim.token).await,
        Answer::Ok(_)
    ));
    let b2 = take(&bob, &p, &t, None).await.ok();
    let (status, problem) = fenced_append(&alice, &p, &t, a.claim.token).await.refused();
    assert_eq!(
        (status, problem.error.as_str()),
        (StatusCode::CONFLICT, "stale_token")
    );
    assert_eq!(detail(&problem).holder, Some(id(&bob)));
    // Nothing of the refused write was kept.
    let entries = bob.read_project(&p, 0).await;
    assert_eq!(entries.entries.len(), 2);
    assert!(matches!(
        fenced_append(&bob, &p, &t, b2.claim.token).await,
        Answer::Ok(_)
    ));
    // A token made up is refused too, and the team's stream takes none.
    assert_eq!(
        fenced_append(&bob, &p, &t, 99).await.refused().1.error,
        "stale_token"
    );
    let team: Answer<Appended> = bob
        .client
        .post(
            "/v1/team/stream",
            &AppendRequest {
                own: false,
                lease: None,
                kind: Kind::Message,
                epoch: None,
                payload: vec![1],
                fence: Some(FenceWire {
                    ticket: t.to_vec(),
                    token: 1,
                }),
            },
        )
        .await;
    assert_eq!(team.refused().1.error, "fence_on_team");
    s.finish().await;
}

/// An administrator keeps a project to some devices: the others neither read it, nor write in it, nor take its
/// tickets, nor hear of it live. A member does not change who has access once the project exists.
async fn a_project_kept_to_some_devices_refuses_the_others(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    let carol = s.add_device(&admin).await;
    let p = random::<16>();
    let kept = AccessWire {
        everyone: false,
        devices: vec![
            serde_bytes::ByteBuf::from(admin.id.clone()),
            serde_bytes::ByteBuf::from(bob.id.clone()),
        ],
    };
    let set = put_access(&admin, &p, &kept).await;
    assert_eq!(set, (StatusCode::NO_CONTENT, String::new()));
    let (status, _) = admin
        .client
        .status(reqwest::Method::GET, &path(&p, "/access"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    bob.append_project(&p, b"x").await.ok();
    assert_eq!(
        carol.append_project(&p, b"y").await.refused().1.error,
        "no_access"
    );
    let (status, _) = carol
        .client
        .status(reqwest::Method::GET, &path(&p, "/stream?since=0"), None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        take(&carol, &p, &random::<16>(), None)
            .await
            .refused()
            .1
            .error,
        "no_access"
    );
    let key: Answer<KeyNumber> = carol.client.post(&path(&p, "/keys"), &()).await;
    assert_eq!(key.refused().1.error, "no_access");
    // A member does not open it to itself.
    let open = AccessWire {
        everyone: true,
        devices: vec![],
    };
    let r = put_access(&bob, &p, &open).await;
    assert_eq!(r, (StatusCode::FORBIDDEN, "not_admin".to_string()));
    // A project open to the team by its first write: a member no longer restricts it.
    let q = random::<16>();
    bob.append_project(&q, b"z").await.ok();
    let r = put_access(&bob, &q, &kept).await;
    assert_eq!(r.0, StatusCode::FORBIDDEN);
    carol.append_project(&q, b"w").await.ok();
    // Out of the project: the claim Bob held no longer holds.
    let t = random::<16>();
    take(&bob, &p, &t, None).await.ok();
    let only_admin = AccessWire {
        everyone: false,
        devices: vec![serde_bytes::ByteBuf::from(admin.id.clone())],
    };
    let _ = put_access(&admin, &p, &only_admin).await;
    assert_eq!(
        take(&bob, &p, &t, None).await.refused().1.error,
        "no_access"
    );
    assert_eq!(
        take(&admin, &p, &t, None).await.ok().claim.holder,
        Some(id(&admin))
    );
    // An unknown device is refused in a list.
    let wrong = AccessWire {
        everyone: false,
        devices: vec![serde_bytes::ByteBuf::from(vec![9u8; 16])],
    };
    let r = put_access(&admin, &p, &wrong).await;
    assert_eq!(r, (StatusCode::BAD_REQUEST, "unknown_device".to_string()));
    s.finish().await;
}

/// A member removed from the team (its devices revoked) takes nothing more, and the ticket it held is free.
async fn a_revoked_holder_holds_nothing(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    let (p, t) = (random::<16>(), random::<16>());
    take(&bob, &p, &t, None).await.ok();
    assert_eq!(take(&admin, &p, &t, None).await.refused().1.error, "taken");
    assert_eq!(admin.revoke(&bob).await, StatusCode::NO_CONTENT);
    let (status, _) = bob
        .client
        .status(reqwest::Method::GET, &path(&p, "/claims"), None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let mine = take(&admin, &p, &t, None).await.ok();
    assert_eq!(
        (mine.claim.holder, mine.claim.last_holder),
        (Some(id(&admin)), Some(id(&bob)))
    );
    s.finish().await;
}

/// The numbers of a project's tickets: one each, never twice, even asked at once; each project its own.
async fn the_counter_gives_each_number_once(b: Backend) {
    let s = TestServer::start(&b).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let p = random::<16>();
    let keys = path(&p, "/keys");
    let asks = (0..20).map(|i| {
        let d = if i % 2 == 0 { &alice } else { &bob };
        d.client.post::<_, KeyNumber>(&keys, &())
    });
    let mut got: Vec<u64> = futures_util::future::join_all(asks)
        .await
        .into_iter()
        .map(|a| a.ok().n)
        .collect();
    got.sort();
    assert_eq!(got, (1..=20).collect::<Vec<_>>());
    let other: KeyNumber = alice
        .client
        .post(&path(&random::<16>(), "/keys"), &())
        .await
        .ok();
    assert_eq!(other.n, 1);
    s.finish().await;
}

/// A claim is pushed live, within a second, to the devices that have access to the project, and to them alone.
async fn a_claim_goes_out_live_to_the_projects_devices_alone(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    let carol = s.add_device(&admin).await;
    let p = random::<16>();
    let kept = AccessWire {
        everyone: false,
        devices: vec![
            serde_bytes::ByteBuf::from(admin.id.clone()),
            serde_bytes::ByteBuf::from(bob.id.clone()),
        ],
    };
    let _ = put_access(&admin, &p, &kept).await;
    let mut bob_live = bob.live(&s).await;
    let mut carol_live = carol.live(&s).await;
    let t = random::<16>();
    let started = std::time::Instant::now();
    take(&admin, &p, &t, None).await.ok();
    let f = bob_live.frame().await;
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(f.stream, "claim");
    assert_eq!(f.claim.unwrap().holder, Some(id(&admin)));
    assert!(
        carol_live.next(Duration::from_millis(500)).await.is_none(),
        "Carol hears nothing of it"
    );
    s.finish().await;
}

/// TS-17: a ticket handed goes to a device of the project under a new token, for the longer lease of a hand; the one
/// that handed writes nothing more under its old token, takes it back while it was not taken, and not after. A hand to
/// a device out of the project is refused, the claim unchanged.
async fn a_ticket_handed_goes_to_its_new_holder(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    let carol = s.add_device(&admin).await;
    let p = random::<16>();
    let kept = AccessWire {
        everyone: false,
        devices: vec![
            serde_bytes::ByteBuf::from(admin.id.clone()),
            serde_bytes::ByteBuf::from(bob.id.clone()),
        ],
    };
    let _ = put_access(&admin, &p, &kept).await;
    let t = random::<16>();
    let a = take(&admin, &p, &t, None).await.ok();
    let hand = |d: &Device, to: &Device, token: u64| {
        let body = HandRequest {
            token,
            to: to.id.clone(),
        };
        let url = path(&p, &format!("/claims/{}/hand", hex::encode(t)));
        let client = d.client.clone();
        async move { client.post::<_, ClaimAnswer>(&url, &body).await }
    };
    // Carol is not of the project: refused, nothing changed.
    let (status, problem) = hand(&admin, &carol, a.claim.token).await.refused();
    assert_eq!(
        (status, problem.error.as_str()),
        (StatusCode::CONFLICT, "not_in_project")
    );
    assert_eq!(detail(&problem).holder, Some(id(&admin)));
    let h = hand(&admin, &bob, a.claim.token).await.ok();
    assert_eq!(
        (h.claim.holder, h.claim.token, h.claim.handed_by),
        (Some(id(&bob)), 2, Some(id(&admin)))
    );
    assert_eq!(h.claim.until - h.now, 14 * 24 * 3600, "a lease of 14 days");
    assert_eq!(
        fenced_append(&admin, &p, &t, a.claim.token)
            .await
            .refused()
            .1
            .error,
        "stale_token"
    );
    // Abandoned: the admin takes it back, then hands it again; Bob takes it.
    let w = ask(&admin, &p, &t, "withdraw", 2).await.ok();
    assert_eq!((w.claim.holder, w.claim.token), (Some(id(&admin)), 3));
    let h = hand(&admin, &bob, 3).await.ok();
    let mine = take(&bob, &p, &t, Some(h.claim.version)).await.ok();
    assert_eq!((mine.claim.holder, mine.claim.token), (Some(id(&bob)), 5));
    assert_eq!(
        ask(&admin, &p, &t, "withdraw", 4).await.refused().1.error,
        "claim_lost"
    );
    assert!(matches!(
        fenced_append(&bob, &p, &t, 5).await,
        Answer::Ok(_)
    ));
    s.finish().await;
}
