//! AU-3 against the real `escouade-team-server`, once on SQLite and once on PostgreSQL (`ESCOUADE_TEST_POSTGRES_URL`):
//! the key of the audit, a pair by epoch wrapped for the emitters alone (the machine of the team, else the
//! administrators), turned when one leaves or a machine comes; the stream of the stations' facts, where every device
//! drops its facts sealed and that the server refuses to a member, who opens neither its own nor another's; the queue
//! offline and its return, a batch sent twice taken once, the station's hour (`at`) and the server's (`seen`).

mod common;

use std::time::Duration;

use common::*;
use escouade_audit::{Fact, Id};
use escouade_team::audit::{self, FACTS_STREAM, KEY_STREAM};
use escouade_team::crypto::Role;
use escouade_team::pairing::{self, PairCode};
use escouade_team::{PairState, Team};
use serde_json::json;

both!(
    a_member_drops_its_facts_sealed_and_opens_none,
    with_a_machine_the_key_is_the_machines_alone,
    the_key_turns_when_an_emitter_leaves,
    offline_the_facts_wait_and_go_once,
);

fn lock_refused(key: &str) -> Fact {
    Fact::from_parts(
        "lock.refused",
        json!({}),
        json!({ "keys": [key] }),
        json!({ "via": "mcp" }),
    )
    .unwrap()
}

/// As the core would read a team: its stream first (the epochs), then the audit.
async fn sync(t: &mut Team) -> audit::View {
    t.sync().await.expect("the team is read");
    audit::sync(t).await.expect("the audit is read")
}

fn hex_of(t: &Team) -> (String, String) {
    let i = t.identity();
    (hex::encode(i.member.0), hex::encode(i.device.0))
}

/// Every byte the server keeps on its disk.
fn server_bytes(w: &World) -> Vec<u8> {
    fn walk(dir: &std::path::Path, out: &mut Vec<u8>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if let Ok(b) = std::fs::read(&p) {
                out.extend(b);
            }
        }
    }
    let mut out = Vec::new();
    walk(&w.server.folder, &mut out);
    out
}

fn holds(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

async fn step_until_done(team: &mut Team, pairing: &mut pairing::Pairing) -> PairState {
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

/// A machine of the team (OE-1), enrolled by Alice's short code.
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
    machine
}

/// Without a machine, Alice's device (an administrator's) emits: it draws the pair and holds it. Bob, a member, drops
/// his facts sealed: he holds no key, the server refuses him the stream and the key's, and keeps nothing in the clear.
async fn a_member_drops_its_facts_sealed_and_opens_none(backend: Backend) {
    let w = World::on(&backend).await;
    let (pa, pb, pc) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut carol = w.join(&mut alice, &pc, member(3), Role::Member).await;
    let a = sync(&mut alice).await;
    assert_eq!(a.epoch, Some(1));
    assert!(a.holds && a.emitter);

    let id = audit::queue(&bob, &lock_refused("agents.zebrakey"), 1_700).unwrap();
    assert_eq!(audit::view(&bob).unwrap().pending, 1);
    let b = sync(&mut bob).await;
    assert_eq!(b.pending, 0, "sent");
    assert_eq!(b.sent.len(), 1);
    assert_eq!(b.sent[0].id, id);
    assert!(!b.holds && !b.emitter, "a member holds no key");

    // Alice's device takes it, its actor from the signature, with the station's hour and the server's.
    let got = audit::take(&mut alice).await.unwrap();
    assert_eq!(got.facts.len(), 1);
    let f = &got.facts[0];
    let (bm, bd) = hex_of(&bob);
    assert_eq!(
        (f.member.as_str(), f.device.as_str()),
        (bm.as_str(), bd.as_str())
    );
    assert_eq!(f.fact.actor().member, Some(Id::new(bm.clone()).unwrap()));
    assert_eq!(f.fact.actor().device, Some(Id::new(bd.clone()).unwrap()));
    assert_eq!(f.fact.name(), "lock.refused");
    assert_eq!(f.at, 1_700, "the station's hour");
    assert!(f.seen > 1_000_000_000_000, "the server's hour: {}", f.seen);
    assert_eq!(f.id, id);
    assert!(
        audit::take(&mut alice).await.unwrap().facts.is_empty(),
        "once"
    );

    // The server refuses the facts to a member (Bob's own included), and the key's stream to its writing.
    for t in [&mut bob, &mut carol] {
        assert!(audit::take(t).await.is_err(), "the server refuses a member");
        assert!(t
            .client()
            .read_project(&FACTS_STREAM.0, 0, None)
            .await
            .is_err());
        assert!(t
            .client()
            .append_project(&KEY_STREAM.0, b"not a key")
            .await
            .is_err());
        assert!(!audit::view(t).unwrap().holds);
    }
    // Carol reads the key's stream (to seal), never a fact.
    let c = sync(&mut carol).await;
    assert_eq!(c.epoch, Some(1));
    assert!(!c.holds);
    // The server reads nothing of it.
    if matches!(backend, Backend::Sqlite) {
        let disk = server_bytes(&w);
        for secret in ["agents.zebrakey", "lock.refused", "\"via\""] {
            assert!(!holds(&disk, secret), "the server keeps {secret}");
        }
    }
    // An event the station does not say is refused at the queue.
    let not_ours = Fact::from_parts("session.opened", json!({}), json!({}), json!({})).unwrap();
    assert!(audit::queue(&bob, &not_ours, 1).is_err());
    drop((pa, pb, pc));
    w.finish().await;
}

/// The administrators emit until a machine comes: then the key they held is stale, the station's facts wait, the
/// machine draws a pair wrapped for itself alone, and the administrators open none of what follows.
async fn with_a_machine_the_key_is_the_machines_alone(backend: Backend) {
    let w = World::on(&backend).await;
    let (pa, pb, pm) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    assert_eq!(sync(&mut alice).await.epoch, Some(1));
    let mut machine = enroll(&mut alice, &pm).await;
    bob.sync().await.unwrap();

    // Alice no longer emits; the key of epoch 1 is wrapped for her: stale, Bob's facts wait.
    let a = sync(&mut alice).await;
    assert!(!a.emitter && a.stale);
    audit::queue(&bob, &lock_refused("agents.engines"), 10).unwrap();
    let b = sync(&mut bob).await;
    assert!(b.stale);
    assert_eq!(b.pending, 1, "waits for the machine's key");

    let m = sync(&mut machine).await;
    assert!(m.emitter);
    assert_eq!(m.epoch, Some(2));
    assert!(m.holds);
    let b = sync(&mut bob).await;
    assert_eq!((b.epoch, b.pending, b.stale), (Some(2), 0, false));
    let a = sync(&mut alice).await;
    assert!(
        !a.holds,
        "the administrator does not hold the machine's key"
    );

    let got = audit::take(&mut machine).await.unwrap();
    assert_eq!(got.facts.len(), 1);
    assert_eq!(got.facts[0].member, hex_of(&bob).0);
    // Alice may read the stream (an administrator), and opens nothing of it.
    let a = audit::take(&mut alice).await.unwrap();
    assert!(a.facts.is_empty());
    assert_eq!(audit::view(&alice).unwrap().unreadable, 1);
    drop((pa, pb, pm));
    w.finish().await;
}

/// Alice and Dave, administrators, emit; Dave leaves: Bob's device seals nothing to the old key, his facts wait, Alice
/// draws a new pair, and they go to it.
async fn the_key_turns_when_an_emitter_leaves(backend: Backend) {
    let w = World::on(&backend).await;
    let (pa, pb, pd) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Admin).await;
    alice.serve(NOW).await.unwrap();
    sync(&mut alice).await;
    sync(&mut dave).await;
    sync(&mut alice).await;
    let d = sync(&mut dave).await;
    assert_eq!(d.epoch, Some(1));
    assert!(d.holds && d.emitter, "Dave, an emitter, holds the key");

    alice.remove_member(member(4)).await.unwrap();
    bob.sync().await.unwrap();
    audit::queue(&bob, &lock_refused("agents.engines"), 20).unwrap();
    let b = sync(&mut bob).await;
    assert!(b.stale);
    assert_eq!(b.pending, 1);
    let a = sync(&mut alice).await;
    assert_eq!(a.epoch, Some(2));
    let b = sync(&mut bob).await;
    assert_eq!((b.epoch, b.pending), (Some(2), 0));
    let got = audit::take(&mut alice).await.unwrap();
    assert_eq!(got.facts.len(), 1);
    assert_eq!(got.facts[0].at, 20);
    drop((pa, pb, pd));
    w.finish().await;
}

/// Offline, the facts wait in their order; back, they go; a reply lost sends a batch twice, the emitter takes each
/// fact once.
async fn offline_the_facts_wait_and_go_once(backend: Backend) {
    let w = World::on(&backend).await;
    let (pa, pb) = (w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    sync(&mut alice).await;
    sync(&mut bob).await;

    let dead = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    w.relay.point_to(dead.local_addr().unwrap());
    drop(dead);
    let first = audit::queue(&bob, &lock_refused("agents.engines"), 1_000).unwrap();
    assert!(audit::sync(&mut bob).await.is_err());
    let second = audit::queue(&bob, &lock_refused("terminal.shell"), 2_000).unwrap();
    assert!(audit::sync(&mut bob).await.is_err());
    assert_eq!(audit::view(&bob).unwrap().pending, 2, "both wait");
    w.relay.point_to(w.server.addr);

    // Back, but the answer of the batch is lost: still waiting, then sent again.
    audit::read(&mut bob).await.unwrap();
    pb.liar.lies.lock().unwrap().lose_op_answer = true;
    assert!(audit::send(&mut bob).await.is_err());
    assert_eq!(audit::view(&bob).unwrap().pending, 2, "still waiting");
    let b = sync(&mut bob).await;
    assert_eq!(b.pending, 0);

    let got = audit::take(&mut alice).await.unwrap();
    let ids: Vec<&str> = got.facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        [first.as_str(), second.as_str()],
        "in their order, each once"
    );
    assert_eq!(
        got.facts.iter().map(|f| f.at).collect::<Vec<_>>(),
        [1_000, 2_000]
    );
    assert!(
        got.facts.iter().all(|f| f.seen > 2_000),
        "seen is the server's"
    );
    assert!(got.dropped.is_empty());
    drop((pa, pb));
    w.finish().await;
}
