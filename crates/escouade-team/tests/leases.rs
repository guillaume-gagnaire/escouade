//! OE-2 against the real `escouade-team-server`, once on SQLite and once on PostgreSQL (`ESCOUADE_TEST_POSTGRES_URL`):
//! two machines of the team and the lease of the team's Overseer. One takes it and renews it in the background, the
//! other waits and says on which machine the Overseer runs; the first cut, its lease runs out and the second takes it
//! under a greater token; the first, back, writes nothing more under its old one; a machine that stops gives it back;
//! a member's device is refused.

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::*;
use escouade_team::crypto::{DeviceId, Role};
use escouade_team::lease::{self, waiting_text, OVERSEER};
use escouade_team::pairing::{self, PairCode};
use escouade_team::wire::LeaseFenceWire;
use escouade_team::{ClientError, LeaseError, LeaseKeeper, LeaseState, PairState, Team};

both!(
    one_machine_runs_the_overseer_and_the_other_waits_and_says_so,
    the_lease_is_renewed_then_taken_over_when_its_machine_is_cut,
    a_machine_that_stops_gives_the_lease_back,
);

/// How often the keepers of the tests renew (the app: every 3 minutes).
const EVERY: Duration = Duration::from_millis(300);

/// The administrator steps the pairing until it ends.
async fn step_until_done(
    team: &mut Team,
    pairing: &mut escouade_team::pairing::Pairing,
) -> PairState {
    for _ in 0..200 {
        match team.pair_step(pairing).await.unwrap() {
            PairState::Waiting | PairState::WaitingForToken => {
                tokio::time::sleep(Duration::from_millis(50)).await
            }
            done => return done,
        }
    }
    panic!("the pairing did not end");
}

/// A machine of the team enrolled by the administrator's short code (OE-1), the server's role in step.
async fn enroll(alice: &mut Team, place: &Place) -> Team {
    let mut pairing = alice.pair_start_service(NOW).await.unwrap();
    let code = PairCode::parse(&pairing.code.shown()).unwrap();
    let transport = place.liar.clone();
    let typed = tokio::spawn(async move {
        pairing::fetch_link(&*transport, &code, Duration::from_secs(20)).await
    });
    assert_eq!(step_until_done(alice, &mut pairing).await, PairState::Sent);
    let link = typed.await.unwrap().unwrap();
    let mut machine = Team::join_link(place.setup(), &link).await.unwrap();
    alice.sync_at(NOW).await.unwrap();
    let shown = alice.admissions(NOW);
    let words = machine.join_words().unwrap();
    alice
        .admit_invitation(shown[0].invitation, &words.join(" "), NOW)
        .await
        .unwrap();
    assert!(machine.sync_at(NOW).await.unwrap().joined);
    alice.serve(NOW).await.unwrap();
    machine.sync().await.unwrap();
    assert!(machine.is_service());
    machine
}

struct Two {
    _places: Vec<Place>,
    alice: Team,
    bob: Team,
    one: Team,
    two: Team,
    one_place: Arc<Liar>,
}

/// Alice (an administrator), Bob (a member), and two machines of the team, `build-1` and `build-2`, that wrote their
/// names in a project of the team.
async fn two_machines(w: &World) -> Two {
    let (pa, pb, p1, p2) = (w.place(), w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut one = enroll(&mut alice, &p1).await;
    let mut two = enroll(&mut alice, &p2).await;
    one.sync().await.unwrap();
    for t in [&mut alice, &mut bob, &mut one, &mut two] {
        t.serve(NOW).await.unwrap();
    }
    let p = alice.create_project(None).await.unwrap();
    alice.sync_project(p).await.unwrap();
    for (t, name) in [(&mut one, "build-1"), (&mut two, "build-2")] {
        t.follow(p).unwrap();
        t.sync_project(p).await.unwrap();
        t.set_name(p, name).unwrap();
        t.sync_project(p).await.unwrap();
    }
    one.sync_project(p).await.unwrap();
    two.sync_project(p).await.unwrap();
    let one_place = p1.liar.clone();
    Two {
        _places: vec![pa, pb, p1, p2],
        alice,
        bob,
        one,
        two,
        one_place,
    }
}

async fn until(what: &str, mut ok: impl FnMut() -> bool) {
    for _ in 0..200 {
        if ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("never: {what}");
}

fn token(k: &LeaseKeeper) -> u64 {
    match k.state() {
        LeaseState::Held { token, .. } => token,
        other => panic!("not held: {other:?}"),
    }
}

fn device(t: &Team) -> [u8; 16] {
    t.client().device()
}

/// The first machine takes the lease; the second waits and says which machine runs the Overseer, by the name that
/// machine wrote in the team's projects (the server never knows it). Only the holder writes in the Overseer's name.
/// A member's device takes no lease of the team.
async fn one_machine_runs_the_overseer_and_the_other_waits_and_says_so(b: Backend) {
    let w = World::on(&b).await;
    let mut t = two_machines(&w).await;
    let k1 = LeaseKeeper::start(t.one.client().clone(), OVERSEER, EVERY);
    until("the first machine holds the lease", || k1.held()).await;
    let k2 = LeaseKeeper::start(t.two.client().clone(), OVERSEER, EVERY);
    until("the second machine waits", || k2.holder().is_some()).await;
    let holder = k2.holder().unwrap();
    assert_eq!(holder, device(&t.one));
    let name = t.two.machine_name(&DeviceId(holder));
    assert_eq!(
        waiting_text(name.as_deref()),
        "En attente : l'Overseer d'équipe tourne sur la machine build-1"
    );
    assert!(k2.fence().is_none());

    // The holder writes in the name of the Overseer (an entry of the thread, OE-4); the one waiting does not.
    let seq = k1.write(None, b"thread entry").await.unwrap();
    assert!(seq > 0);
    assert!(matches!(
        k2.write(None, b"thread entry").await,
        Err(LeaseError::NotHeld)
    ));
    // It stays waiting while the first renews (several renewals go by).
    let first = token(&k1);
    tokio::time::sleep(EVERY * 4).await;
    assert_eq!(token(&k1), first, "renewed under the same token");
    assert_eq!(k2.holder(), Some(device(&t.one)));

    // A member's device, an administrator's too: refused (403).
    for d in [&t.bob, &t.alice] {
        match d.client().take_lease(OVERSEER).await {
            Err(ClientError::Refused { status: 403, .. }) => {}
            other => panic!("a person's device took the lease: {other:?}"),
        }
    }
    k1.stop().await;
    k2.stop().await;
    w.finish().await;
}

/// The first machine renews the lease while it is there; cut from the server, it holds nothing once its lease runs
/// out on its own clock, and the second takes it at its end under a greater token. The first, back, is refused under
/// its old token (`stale_token`), writes nothing in the Overseer's name, and waits in its turn.
async fn the_lease_is_renewed_then_taken_over_when_its_machine_is_cut(b: Backend) {
    let w = World::on_with(&b, |c| c.limits.team_lease = Duration::from_secs(2)).await;
    let mut t = two_machines(&w).await;
    let k1 = LeaseKeeper::start(t.one.client().clone(), OVERSEER, EVERY);
    until("the first machine holds the lease", || k1.held()).await;
    let k2 = LeaseKeeper::start(t.two.client().clone(), OVERSEER, EVERY);
    until("the second machine waits", || k2.holder().is_some()).await;
    let old = token(&k1);
    // Beyond its lease of 2 s, still held: renewed.
    tokio::time::sleep(Duration::from_millis(3000)).await;
    assert_eq!(token(&k1), old);
    assert!(!k2.held());

    // Cut.
    t.one_place.lies.lock().unwrap().offline = true;
    until(
        "the first machine holds nothing once its lease ran out",
        || !k1.held(),
    )
    .await;
    assert!(k1.fence().is_none());
    until("the second machine takes the lease at its end", || {
        k2.held()
    })
    .await;
    let new = token(&k2);
    assert!(new > old, "a greater token: {new} > {old}");
    assert!(k2.write(None, b"the new holder").await.is_ok());

    // Back: under its old token, refused; the keeper writes nothing, and waits for the second.
    t.one_place.lies.lock().unwrap().offline = false;
    match t
        .one
        .client()
        .append_leased(
            None,
            b"from before the cut",
            LeaseFenceWire {
                name: OVERSEER.into(),
                token: old,
            },
        )
        .await
    {
        Err(ClientError::Lease { code, lease }) => {
            assert_eq!(code, "stale_token");
            assert_eq!(lease.token, new);
        }
        other => panic!("a write under the old token passed: {other:?}"),
    }
    assert!(matches!(
        k1.write(None, b"from before the cut").await,
        Err(LeaseError::NotHeld)
    ));
    k1.poke();
    until("the first machine waits for the second", || {
        k1.holder() == Some(device(&t.two))
    })
    .await;
    let name = t.one.machine_name(&DeviceId(device(&t.two)));
    assert_eq!(
        waiting_text(name.as_deref()),
        "En attente : l'Overseer d'équipe tourne sur la machine build-2"
    );
    k2.stop().await;
    k1.stop().await;
    w.finish().await;
}

/// A machine that stops gives the lease back: the other takes it at once, under a greater token, and the first
/// renews nothing more.
async fn a_machine_that_stops_gives_the_lease_back(b: Backend) {
    let w = World::on(&b).await;
    let t = two_machines(&w).await;
    let k1 = LeaseKeeper::start(t.one.client().clone(), lease::OVERSEER, EVERY);
    until("the first machine holds the lease", || k1.held()).await;
    let k2 = LeaseKeeper::start(t.two.client().clone(), lease::OVERSEER, EVERY);
    until("the second machine waits", || k2.holder().is_some()).await;
    let old = token(&k1);
    let changes = k2.watch();
    k1.stop().await;
    assert_eq!(k1.state(), LeaseState::Stopped);
    assert!(k1.fence().is_none());
    k2.poke();
    until("the second machine takes it at once", || k2.held()).await;
    assert!(token(&k2) > old);
    assert!(changes.has_changed().unwrap());
    let seen = t.two.client().lease(OVERSEER).await.unwrap();
    assert_eq!(
        seen.lease.holder.as_deref(),
        Some(device(&t.two).as_slice())
    );
    k2.stop().await;
    let seen = t.one.client().lease(OVERSEER).await.unwrap();
    assert_eq!(seen.lease.holder, None);
    w.finish().await;
}
