//! OE-2: the lease of the team (`/v1/team/leases/{name}`, `docs/TEAM-OVERSEER.md` §7.1) between two machines of the
//! team, and the writes that carry its token, against the real server, on SQLite then on PostgreSQL.

#[macro_use]
mod common;

use std::time::Duration;

use common::*;
use escouade_team_server::api::wire::*;
use escouade_team_server::db::{Kind, Lease, Role};
use escouade_team_server::transport::cbor;
use escouade_team_server::util::random;
use reqwest::StatusCode;

both!(
    one_machine_holds_the_lease_and_the_other_waits,
    the_lease_runs_out_and_the_other_takes_it_under_a_greater_token,
    a_write_under_a_stale_lease_token_is_refused,
    a_lease_given_back_is_taken_at_once,
    a_members_device_is_refused,
    a_revoked_holder_holds_nothing,
    the_server_keeps_of_a_lease_a_name_a_device_and_hours,
);

const OVERSEER: &str = "/v1/team/leases/overseer";

/// A team: its administrator, and two machines of the team (role of service, OE-1).
async fn team(s: &TestServer) -> (Device, Device, Device) {
    let admin = s.new_team().await;
    let one = machine(s, &admin).await;
    let two = machine(s, &admin).await;
    (admin, one, two)
}

async fn machine(s: &TestServer, admin: &Device) -> Device {
    let mut d = s.add_device(admin).await;
    let r = admin
        .client
        .request(
            reqwest::Method::POST,
            &format!("/v1/devices/{}/role", d.id_hex()),
        )
        .header("content-type", "application/cbor")
        .body(cbor::encode(&RoleRequest {
            role: Role::Service,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    d.sign_in().await.unwrap();
    d
}

async fn take(d: &Device) -> Answer<LeaseAnswer> {
    d.client
        .post(OVERSEER, &ciborium::Value::Map(Vec::new()))
        .await
}

async fn ask(d: &Device, what: &str, token: u64) -> Answer<LeaseAnswer> {
    d.client
        .post(&format!("{OVERSEER}/{what}"), &TokenRequest { token })
        .await
}

async fn read(d: &Device) -> Answer<LeaseAnswer> {
    d.client.get(OVERSEER).await
}

fn lease_of(p: &Problem) -> Lease {
    p.detail
        .as_ref()
        .expect("the lease as detail")
        .deserialized()
        .expect("a lease")
}

/// A message of the team's stream in the name of the Overseer (an entry of the thread, OE-4), under `token`.
async fn leased_message(d: &Device, token: u64) -> Answer<Appended> {
    leased(d, "/v1/team/stream", token).await
}

async fn leased(d: &Device, path: &str, token: u64) -> Answer<Appended> {
    d.client
        .post(
            path,
            &AppendRequest {
                kind: Kind::Message,
                epoch: None,
                payload: vec![9; 32],
                fence: None,
                lease: Some(LeaseFenceWire {
                    name: "overseer".into(),
                    token,
                }),
                own: false,
            },
        )
        .await
}

/// One machine takes the lease; the other is told which holds it, since when and until when, and waits.
async fn one_machine_holds_the_lease_and_the_other_waits(b: Backend) {
    let s = TestServer::start(&b).await;
    let (_admin, one, two) = team(&s).await;
    let a = take(&one).await.ok();
    assert_eq!(a.lease.holder.map(|h| h.to_vec()), Some(one.id.clone()));
    assert_eq!(a.lease.token, 1);
    // Ten minutes, as the claims of the tickets.
    assert_eq!(a.length, 600);
    assert_eq!(a.lease.until, a.now + 600);
    let (status, p) = take(&two).await.refused();
    assert_eq!((status, p.error.as_str()), (StatusCode::CONFLICT, "taken"));
    let held = lease_of(&p);
    assert_eq!(held.holder.map(|h| h.to_vec()), Some(one.id.clone()));
    assert_eq!(held.claimed_at, a.lease.claimed_at);
    // Its holder renews it under its token; the other cannot.
    let r = ask(&one, "renew", 1).await.ok();
    assert_eq!((r.lease.token, r.lease.claimed_at), (1, a.lease.claimed_at));
    let (status, p) = ask(&two, "renew", 1).await.refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::CONFLICT, "lease_lost")
    );
    // Both read who holds it.
    let seen = read(&two).await.ok();
    assert_eq!(seen.lease.holder.map(|h| h.to_vec()), Some(one.id.clone()));
    // Two takes at once: one passes.
    let s2 = TestServer::start(&b).await;
    let (_admin, x, y) = team(&s2).await;
    let (ax, ay) = tokio::join!(take(&x), take(&y));
    let passed = [&ax, &ay]
        .iter()
        .filter(|a| matches!(a, Answer::Ok(_)))
        .count();
    assert_eq!(passed, 1);
    s2.finish().await;
    s.finish().await;
}

/// The first machine is cut: its lease runs out, the second takes it under a greater token, and the first, back, is
/// refused under its old one (renewal and writes alike); it waits in its turn.
async fn the_lease_runs_out_and_the_other_takes_it_under_a_greater_token(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.team_lease = Duration::from_secs(2)).await;
    let (_admin, one, two) = team(&s).await;
    let a = take(&one).await.ok();
    assert!(matches!(
        take(&two).await,
        Answer::No(StatusCode::CONFLICT, _)
    ));
    tokio::time::sleep(Duration::from_millis(3100)).await;
    let b2 = take(&two).await.ok();
    assert_eq!(b2.lease.holder.map(|h| h.to_vec()), Some(two.id.clone()));
    assert!(b2.lease.token > a.lease.token);
    let (_, p) = ask(&one, "renew", a.lease.token).await.refused();
    assert_eq!(p.error, "lease_lost");
    let (status, p) = leased_message(&one, a.lease.token).await.refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::CONFLICT, "stale_token")
    );
    assert_eq!(lease_of(&p).token, b2.lease.token);
    assert!(matches!(
        leased_message(&two, b2.lease.token).await,
        Answer::Ok(_)
    ));
    let (_, p) = take(&one).await.refused();
    assert_eq!(p.error, "taken");
    s.finish().await;
}

/// A write that says it is the Overseer's passes under the current token alone, in the team's stream and in a
/// project's; nothing is written of a refused one. A lease run out refuses even its holder's current token until it
/// renews.
async fn a_write_under_a_stale_lease_token_is_refused(b: Backend) {
    let s = TestServer::start_with(&b, |c| c.limits.team_lease = Duration::from_secs(2)).await;
    let (admin, one, two) = team(&s).await;
    // Never taken: refused.
    let (_, p) = leased_message(&one, 0).await.refused();
    assert_eq!(p.error, "stale_token");
    let a = take(&one).await.ok();
    let seq = leased_message(&one, a.lease.token).await.ok().seq;
    let project = random::<16>();
    let in_project = format!("/v1/projects/{}/stream", hex::encode(project));
    assert!(matches!(
        leased(&one, &in_project, a.lease.token).await,
        Answer::Ok(_)
    ));
    // Another machine, under the holder's token: refused (the token is the holder's).
    let (_, p) = leased_message(&two, a.lease.token).await.refused();
    assert_eq!(p.error, "stale_token");
    // A token from elsewhere: refused.
    let (_, p) = leased_message(&one, a.lease.token + 1).await.refused();
    assert_eq!(p.error, "stale_token");
    // The holder takes it again (a restart): a new token, the old one retired.
    let again = take(&one).await.ok();
    assert_eq!(again.lease.token, a.lease.token + 1);
    let (_, p) = leased_message(&one, a.lease.token).await.refused();
    assert_eq!(p.error, "stale_token");
    let (_, p) = leased(&one, &in_project, a.lease.token).await.refused();
    assert_eq!(p.error, "stale_token");
    // Its lease over, its own current token writes nothing until it renews.
    tokio::time::sleep(Duration::from_millis(3100)).await;
    let (_, p) = leased_message(&one, again.lease.token).await.refused();
    assert_eq!(p.error, "stale_token");
    ask(&one, "renew", again.lease.token).await.ok();
    assert!(matches!(
        leased_message(&one, again.lease.token).await,
        Answer::Ok(_)
    ));
    // The team's stream holds the two written in the name of the Overseer, nothing of the refused.
    let entries = admin.read_team(seq - 1).await;
    assert_eq!(entries.entries.len(), 2);
    // A bad name is refused before the base.
    let r = one
        .client
        .post::<_, Appended>(
            "/v1/team/stream",
            &AppendRequest {
                kind: Kind::Message,
                epoch: None,
                payload: vec![1],
                fence: None,
                lease: Some(LeaseFenceWire {
                    name: "Not A Name".into(),
                    token: 1,
                }),
                own: false,
            },
        )
        .await;
    assert!(matches!(r, Answer::No(StatusCode::BAD_REQUEST, _)));
    s.finish().await;
}

/// The machine that stops gives the lease back: the other takes it at once, under a greater token.
async fn a_lease_given_back_is_taken_at_once(b: Backend) {
    let s = TestServer::start(&b).await;
    let (_admin, one, two) = team(&s).await;
    let a = take(&one).await.ok();
    let (_, p) = ask(&two, "release", a.lease.token).await.refused();
    assert_eq!(p.error, "lease_lost");
    let given = ask(&one, "release", a.lease.token).await.ok();
    assert_eq!((given.lease.holder, given.lease.until), (None, 0));
    let b2 = take(&two).await.ok();
    assert_eq!(b2.lease.token, a.lease.token + 1);
    let (_, p) = leased_message(&one, a.lease.token).await.refused();
    assert_eq!(p.error, "stale_token");
    // Giving back twice: lost.
    let (_, p) = ask(&one, "release", a.lease.token).await.refused();
    assert_eq!(p.error, "lease_lost");
    s.finish().await;
}

/// The leases are the machines' of the team: a member's device, an administrator's too, is refused on each route.
async fn a_members_device_is_refused(b: Backend) {
    let s = TestServer::start(&b).await;
    let (admin, one, _two) = team(&s).await;
    let member = s.add_device(&admin).await;
    for d in [&admin, &member] {
        let (status, _) = take(d).await.refused();
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = ask(d, "renew", 1).await.refused();
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = ask(d, "release", 1).await.refused();
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = read(d).await.refused();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
    // Nor does a member write in the name of the Overseer under the machine's token.
    let a = take(&one).await.ok();
    let (_, p) = leased_message(&member, a.lease.token).await.refused();
    assert_eq!(p.error, "stale_token");
    // A bad name, for a machine.
    let (status, p) = one
        .client
        .post::<_, LeaseAnswer>(
            "/v1/team/leases/Bad%20Name",
            &ciborium::Value::Map(Vec::new()),
        )
        .await
        .refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::BAD_REQUEST, "bad_name")
    );
    s.finish().await;
}

/// A machine removed (its device revoked) holds nothing: the other takes the lease at once.
async fn a_revoked_holder_holds_nothing(b: Backend) {
    let s = TestServer::start(&b).await;
    let (admin, one, two) = team(&s).await;
    let a = take(&one).await.ok();
    assert_eq!(admin.revoke(&one).await, StatusCode::NO_CONTENT);
    let b2 = take(&two).await.ok();
    assert_eq!(b2.lease.holder.map(|h| h.to_vec()), Some(two.id.clone()));
    assert!(b2.lease.token > a.lease.token);
    s.finish().await;
}

/// What the server keeps of a lease: its name, the device, hours, the token; nothing else (no machine's name, no
/// project, no content).
async fn the_server_keeps_of_a_lease_a_name_a_device_and_hours(b: Backend) {
    let s = TestServer::start(&b).await;
    let (_admin, one, _two) = team(&s).await;
    let a = take(&one).await.ok();
    let mut fields: Vec<String> = match ciborium::Value::serialized(&a.lease).unwrap() {
        ciborium::Value::Map(m) => m.into_iter().map(|(k, _)| k.into_text().unwrap()).collect(),
        _ => panic!("a map"),
    };
    fields.sort();
    assert_eq!(
        fields,
        [
            "claimed_at",
            "holder",
            "name",
            "renewed_at",
            "token",
            "until"
        ]
    );
    s.finish().await;
}
