//! TS-15 against the real `escouade-team-server`: the administrator's circuit breaker. An administrator sets one and
//! lifts it, a member reads both; the server reads neither what it takes out nor why; a member's entry is refused by
//! the client, by the server and at the reading; a forged entry, a serial played again, an old lifting played again
//! are refused at the reading; two administrators who write at once both get in; the breakers of an administrator who
//! left are written again for a device that comes later; a lying server that hides an entry is caught.

mod common;

use common::*;
use escouade_team::breaker::{self, Entry, Refused, Scope, View, What, BREAKER_STREAM};
use escouade_team::crypto::content::SealedObject;
use escouade_team::crypto::Role;
use escouade_team::{wire, CryptoError, Error, Team};

fn lies(p: &Place) -> std::sync::MutexGuard<'_, Lies> {
    let mut l = p.liar.lies.lock().unwrap();
    l.project = Some(hex::encode(BREAKER_STREAM.0));
    l
}

async fn sync(t: &mut Team) -> View {
    breaker::sync(t).await.expect("the breakers are read")
}

fn codex() -> What {
    What::Harness { id: "codex".into() }
}

const WHY: &str = "fuite de clé chez le fournisseur X";

async fn cut(t: &mut Team, what: What) -> String {
    breaker::set(t, Scope::Team, what, WHY, "Claire Martin", NOW)
        .await
        .expect("the breaker is set")
        .0
}

async fn last_seq(t: &Team) -> u64 {
    t.client()
        .read_project(&BREAKER_STREAM.0, 0, None)
        .await
        .unwrap()
        .entries
        .last()
        .map_or(0, |e| e.seq)
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

/// Alice (administrator, the owner) and Bob (a member).
async fn two(w: &World) -> (Place, Team, Place, Team) {
    let (pa, pb) = (w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    (pa, alice, pb, bob)
}

async fn an_administrator_sets_and_lifts_and_a_member_reads(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, _pb, mut bob) = two(&w).await;
    assert!(sync(&mut bob).await.active.is_empty());

    let id = cut(&mut alice, codex()).await;
    let agents = breaker::set(
        &mut alice,
        Scope::Project {
            project: "ab".repeat(16),
            name: "Acme API".into(),
        },
        What::Agents,
        "audit en cours",
        "Claire Martin",
        NOW + 1,
    )
    .await
    .unwrap()
    .0;
    let seen = sync(&mut bob).await;
    assert_eq!(seen.active.len(), 2);
    assert_eq!(seen.serial, 2);
    // The most recent first.
    assert_eq!(seen.active[0].id, agents);
    let b = &seen.active[1];
    assert_eq!(
        (b.id.as_str(), &b.what, b.reason.as_str(), b.by.as_str()),
        (id.as_str(), &codex(), WHY, "Claire Martin")
    );
    assert_eq!(b.member, member(1).to_string());
    assert_eq!(b.scope, Scope::Team);
    assert_eq!(b.serial, 1);
    assert!(seen.refusals.is_empty());

    // The server reads nothing of it.
    if matches!(backend, Backend::Sqlite) {
        let disk = server_bytes(&w);
        for clear in ["codex", "fournisseur", "Claire Martin", "Acme API", "audit"] {
            assert!(
                !disk.windows(clear.len()).any(|x| x == clear.as_bytes()),
                "the server keeps « {clear} » in the clear"
            );
        }
    }

    // Lifted: Bob reads it gone, with who lifted it.
    breaker::lift(&mut alice, &id, "Claire Martin", NOW + 2)
        .await
        .unwrap();
    let seen = sync(&mut bob).await;
    assert_eq!(seen.active.len(), 1);
    assert_eq!(seen.lifted.len(), 1);
    assert_eq!(seen.lifted[0].breaker.id, id);
    assert_eq!(seen.lifted[0].at, NOW + 2);
    assert_eq!(seen.serial, 3);
    // A breaker not in force is not lifted.
    assert!(matches!(
        breaker::lift(&mut alice, &id, "Claire Martin", NOW).await,
        Err(Error::Refused(_))
    ));
    // Read again from the disk, offline: the same.
    assert_eq!(breaker::view(&bob).unwrap(), seen);
    w.finish().await;
}

async fn a_member_cannot_set_nor_lift(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, pb, mut bob) = two(&w).await;
    let id = cut(&mut alice, codex()).await;
    sync(&mut bob).await;
    // The client refuses it before it leaves.
    assert!(matches!(
        breaker::set(&mut bob, Scope::Team, What::Agents, "r", "Bob", NOW).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    assert!(matches!(
        breaker::lift(&mut bob, &id, "Bob", NOW).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    // Past the client's check, sealed by Bob's leaf: the server refuses to keep it.
    let lift = Entry::Lift {
        id: {
            let mut b = [0u8; 16];
            hex::decode_to_slice(&id, &mut b).unwrap();
            b
        },
        serial: 9,
        by: "Bob".into(),
        at: NOW,
    };
    let payload = breaker::seal(&bob, &lift).unwrap();
    let refused = bob
        .client()
        .append_project(&BREAKER_STREAM.0, &payload)
        .await
        .expect_err("the server refuses a member's entry");
    assert_eq!(refused.status(), Some(403));

    // A server that keeps it anyway (or plays it to the others): every device refuses it at the reading.
    let seq = last_seq(&alice).await;
    lies(&pb).project_inject = vec![(seq + 1, payload)];
    let seen = sync(&mut bob).await;
    assert_eq!(seen.active.len(), 1, "Alice's breaker holds");
    assert!(seen
        .refusals
        .iter()
        .any(|r| r.seq == seq + 1 && r.reason == Refused::NotAdmin));
    // Nor can a member restrict who reads the stream.
    let refused = bob
        .client()
        .set_access(
            &BREAKER_STREAM.0,
            &wire::AccessWire {
                everyone: false,
                devices: vec![serde_bytes::ByteBuf::from(bob.client().device().to_vec())],
            },
        )
        .await
        .expect_err("nobody restricts the team's streams");
    assert_eq!(refused.status(), Some(403));
    w.finish().await;
}

async fn a_forged_entry_or_a_serial_played_again_is_refused(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, pb, mut bob) = two(&w).await;
    let id = cut(&mut alice, codex()).await;
    let first = alice
        .client()
        .read_project(&BREAKER_STREAM.0, 0, None)
        .await
        .unwrap()
        .entries[0]
        .payload
        .clone();
    breaker::lift(&mut alice, &id, "Claire Martin", NOW + 1)
        .await
        .unwrap();
    let seq = last_seq(&alice).await;

    // Forged: Alice's entry, its signature changed by a byte (the server cannot sign as her).
    let mut forged: SealedObject = wire::decode(&first).unwrap();
    forged.signature[0] ^= 1;
    // Forged: bytes that are no sealed object at all.
    // Replayed: Alice's first entry, served again after its lifting (its serial is below the highest).
    lies(&pb).project_inject = vec![
        (seq + 1, wire::encode(&forged)),
        (seq + 2, b"not a sealed object".to_vec()),
        (seq + 3, first.clone()),
    ];
    let seen = sync(&mut bob).await;
    assert!(seen.active.is_empty(), "the lifted breaker stays lifted");
    let reasons: Vec<(u64, Refused)> = seen
        .refusals
        .iter()
        .map(|r| (r.seq, r.reason.clone()))
        .collect();
    assert_eq!(reasons[0], (seq + 1, Refused::Unsigned));
    assert_eq!(reasons[1].0, seq + 2);
    assert!(matches!(reasons[1].1, Refused::Unreadable { .. }));
    assert_eq!(
        reasons[2],
        (
            seq + 3,
            Refused::Replayed {
                serial: 1,
                highest: 2
            }
        )
    );
    w.finish().await;
}

async fn two_administrators_who_write_at_once_both_get_in(backend: Backend) {
    let w = World::on(&backend).await;
    let (pa, pc) = (w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut carol = w.join(&mut alice, &pc, member(3), Role::Admin).await;
    alice.serve(NOW).await.unwrap();
    // Both read serial 0 and write 1 at once: the one read second is refused, its author writes it again above.
    sync(&mut alice).await;
    sync(&mut carol).await;
    let (a, c) = tokio::join!(
        breaker::set(&mut alice, Scope::Team, codex(), WHY, "Alice", NOW),
        breaker::set(
            &mut carol,
            Scope::Team,
            What::Model {
                id: "gpt-5.5".into()
            },
            WHY,
            "Carol",
            NOW
        ),
    );
    let (a, c) = (a.unwrap().0, c.unwrap().0);
    assert_ne!(a, c);
    let seen = sync(&mut alice).await;
    assert_eq!(seen.active.len(), 2);
    assert_ne!(seen.active[0].serial, seen.active[1].serial);
    assert_eq!(sync(&mut carol).await.active, seen.active);
    w.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_breakers_of_an_administrator_who_left_are_written_again() {
    let w = World::new().await;
    let (pa, pb, pc, pd) = (w.place(), w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut carol = w.join(&mut alice, &pc, member(3), Role::Admin).await;
    bob.sync().await.unwrap();
    alice.serve(NOW).await.unwrap();
    // Carol sets a breaker, then leaves the team.
    let id = cut(&mut carol, codex()).await;
    assert_eq!(sync(&mut bob).await.active.len(), 1);
    sync(&mut alice).await;
    alice.remove_member(member(3)).await.unwrap();
    bob.sync().await.unwrap();
    // Bob lived her epoch: her breaker holds.
    assert_eq!(sync(&mut bob).await.active.len(), 1);
    // Dave comes after: her entry does not count for him, until Alice's device writes it again.
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Member).await;
    let seen = sync(&mut dave).await;
    assert!(seen.active.is_empty());
    assert!(seen.refusals.iter().any(|r| r.reason == Refused::NotAdmin));
    assert_eq!(breaker::serve(&mut alice, NOW).await.unwrap(), 1);
    assert_eq!(breaker::serve(&mut alice, NOW).await.unwrap(), 0, "once");
    let seen = sync(&mut dave).await;
    assert_eq!(seen.active.len(), 1);
    assert_eq!(seen.active[0].id, id);
    assert_eq!(seen.active[0].member, member(1).to_string());
    assert_eq!(seen.active[0].reason, WHY);
    // Bob too: the same breaker, set again above.
    assert_eq!(sync(&mut bob).await.active.len(), 1);
    // A member's device writes nothing again.
    assert_eq!(breaker::serve(&mut bob, NOW).await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_lying_server_that_hides_an_entry_is_caught() {
    let w = World::new().await;
    let (_pa, mut alice, pb, mut bob) = two(&w).await;
    cut(&mut alice, codex()).await;
    let first = last_seq(&alice).await;
    cut(&mut alice, What::Agents).await;
    assert_eq!(sync(&mut bob).await.active.len(), 2);
    // The server hides the second entry from Bob: a rollback, and what was in force stays in force.
    lies(&pb).project_hide_after = Some(first);
    let seen = sync(&mut bob).await;
    assert_eq!(seen.alarm, Some(breaker::Alarm::Rollback));
    assert_eq!(seen.active.len(), 2);
    lies(&pb).project_hide_after = None;
    let seen = breaker::retry(&mut bob).await.unwrap();
    assert_eq!(seen.alarm, None);
    assert_eq!(seen.active.len(), 2);
    // An administrator sends nothing under an alarm.
    w.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_devices_online_are_counted() {
    let w = World::new().await;
    let (_pa, alice, _pb, bob) = two(&w).await;
    assert_eq!(breaker::online(&alice).await.unwrap(), 0);
    let _feed = bob.live();
    // Bob's live feed connects: Alice counts one device online.
    let mut n = 0;
    for _ in 0..100 {
        n = breaker::online(&alice).await.unwrap();
        if n == 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert_eq!(n, 1);
    w.finish().await;
}

both!(
    an_administrator_sets_and_lifts_and_a_member_reads,
    a_member_cannot_set_nor_lift,
    a_forged_entry_or_a_serial_played_again_is_refused,
    two_administrators_who_write_at_once_both_get_in,
);
