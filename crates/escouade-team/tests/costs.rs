//! TS-16 against the real `escouade-team-server`: the totals of the costs, sealed to the administrators' key. Only
//! totals go, and exactly what the types say; the server reads nothing of them; a member reads only its own and the
//! summary of the projects; the key turns when an administrator leaves, and nothing is sealed to the old one; offline,
//! the totals wait, and a reply lost does not count them twice.

mod common;

use std::collections::BTreeSet;

use common::*;
use escouade_team::costs::{self, Day, Row, TicketRow, Totals, KEY_STREAM};
use escouade_team::crypto::Role;
use escouade_team::Team;

const MONTH: &str = "2026-11";
const BOARD: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90";

fn totals(member: Option<&str>, hour: i64, cost: f64) -> Totals {
    Totals {
        member: member.map(str::to_string),
        hour,
        days: vec![Day {
            day: "2026-11-20".into(),
            rows: vec![Row {
                project: BOARD.into(),
                harness: "claude".into(),
                model: "claude-opus-zebra".into(),
                plan: true,
                input: 1200,
                output: 340,
                cache: 9000,
                cost,
                quota: Some(42.0),
            }],
            tickets: vec![TicketRow {
                project: BOARD.into(),
                ticket: "APIX-1234".into(),
                input: 1200,
                output: 340,
                cache: 9000,
                cost,
            }],
        }],
    }
}

/// As the core reads a team: its stream first (the epochs), then the costs.
async fn sync(t: &mut Team) -> costs::View {
    t.sync().await.expect("the team is read");
    costs::sync(t, MONTH, 1_000)
        .await
        .expect("the costs are read")
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

fn keys(v: &serde_json::Value) -> BTreeSet<String> {
    v.as_object().unwrap().keys().cloned().collect()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

/// What goes, read back from its exact bytes: these fields and no other, at every level.
#[test]
fn the_totals_hold_these_fields_and_nothing_else() {
    let t = totals(Some("bob"), 7, 1.5);
    let bytes = costs::plain_bytes(&t);
    let back: serde_json::Value = ciborium::from_reader(bytes.as_slice()).unwrap();
    assert_eq!(keys(&back), set(&["member", "hour", "days"]));
    let day = &back["days"][0];
    assert_eq!(keys(day), set(&["day", "rows", "tickets"]));
    assert_eq!(
        keys(&day["rows"][0]),
        set(&["project", "harness", "model", "plan", "input", "output", "cache", "cost", "quota"])
    );
    assert_eq!(
        keys(&day["tickets"][0]),
        set(&["project", "ticket", "input", "output", "cache", "cost"])
    );
    // « Par projet », no member at all.
    let anon = costs::plain_bytes(&totals(None, 7, 1.5));
    let back: serde_json::Value = ciborium::from_reader(anon.as_slice()).unwrap();
    assert_eq!(keys(&back), set(&["hour", "days"]));
    // A field more (a prompt, a file) is refused both ways.
    let mut with = serde_json::to_value(&t).unwrap();
    with["days"][0]["rows"][0]["prompt"] = "rm -rf".into();
    assert!(serde_json::from_value::<Totals>(with).is_err());
    let mut with = serde_json::to_value(&t).unwrap();
    with["days"][0]["file"] = "src/secret.rs".into();
    assert!(serde_json::from_value::<Totals>(with).is_err());
}

/// Alice (administrator, the owner), Bob and Carol (members).
async fn three(w: &World) -> (Place, Team, Place, Team, Place, Team) {
    let (pa, pb, pc) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let carol = w.join(&mut alice, &pc, member(3), Role::Member).await;
    (pa, alice, pb, bob, pc, carol)
}

async fn only_totals_go_sealed_and_a_member_reads_only_its_own(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, _pb, mut bob, _pc, mut carol) = three(&w).await;
    // Alice's device draws the administrators' key and holds it.
    let a = sync(&mut alice).await;
    assert_eq!(a.epoch, Some(1));
    assert!(a.holds);
    // Bob's totals go, sealed to it.
    costs::queue(&bob, totals(None, 100, 2.5)).unwrap();
    let b = sync(&mut bob).await;
    assert!(b.pending.is_empty(), "sent");
    assert_eq!(b.sent, Some(totals(None, 100, 2.5)));
    assert!(!b.holds, "a member holds no key");
    assert!(b.received.is_empty());

    let a = sync(&mut alice).await;
    assert_eq!(a.received.len(), 1);
    let got = &a.received[0];
    assert_eq!(got.author, member(2).to_string());
    assert_eq!(got.member, None, "« par projet »: no member in the totals");
    let day = &got.days["2026-11-20"];
    assert_eq!(day.rows, totals(None, 100, 2.5).days[0].rows);
    assert_eq!(day.tickets[0].ticket, "APIX-1234");

    // Carol, a member, reads none of Bob's totals; she reads the summary of the projects, never a member's.
    let c = sync(&mut carol).await;
    assert!(c.received.is_empty());
    assert!(!c.holds);
    let s = c.summary.expect("the summary of the month");
    assert_eq!(s.month, MONTH);
    assert_eq!(s.projects[BOARD].cost, 2.5);
    assert_eq!(s.team.cost, 2.5);
    assert_eq!(s.team.tokens, 1200 + 340 + 9000);

    // The server reads nothing of it: no ticket key, no model, no project, no day.
    if matches!(backend, Backend::Sqlite) {
        let disk = server_bytes(&w);
        for secret in [
            "APIX-1234",
            "claude-opus-zebra",
            BOARD,
            "2026-11-20",
            "harness",
        ] {
            assert!(!holds(&disk, secret), "the server keeps {secret}");
        }
    }
    // A member cannot write the key's stream (the server refuses it: the totals would be sealed to its key).
    let refused = bob
        .client()
        .append_project(&KEY_STREAM.0, b"not a key")
        .await;
    assert!(refused.is_err());
    w.finish().await;
}

async fn the_member_is_named_only_par_membre(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, _pb, mut bob, _pc, _carol) = three(&w).await;
    sync(&mut alice).await;
    costs::queue(&bob, totals(Some(&member(2).to_string()), 100, 1.0)).unwrap();
    sync(&mut bob).await;
    let a = sync(&mut alice).await;
    assert_eq!(a.received[0].member, Some(member(2).to_string()));
    w.finish().await;
}

async fn the_key_turns_when_an_administrator_leaves(backend: Backend) {
    let w = World::on(&backend).await;
    let (pa, pb, pd) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Admin).await;
    alice.serve(NOW).await.unwrap();
    // Alice draws the key; Dave's device writes its own key, then Alice wraps the pair for it.
    sync(&mut alice).await;
    sync(&mut dave).await;
    sync(&mut alice).await;
    let d = sync(&mut dave).await;
    assert_eq!(d.epoch, Some(1));
    assert!(d.holds, "Dave, an administrator, holds the key");
    costs::queue(&bob, totals(None, 100, 1.0)).unwrap();
    sync(&mut bob).await;
    assert_eq!(sync(&mut dave).await.received.len(), 1);

    // Dave leaves: Bob's device seals nothing to the old key, his totals wait.
    alice.remove_member(member(4)).await.unwrap();
    bob.sync().await.unwrap();
    costs::queue(&bob, totals(None, 200, 3.0)).unwrap();
    let b = sync(&mut bob).await;
    assert!(b.stale);
    assert_eq!(b.pending, vec!["2026-11-20".to_string()]);
    // Alice's device draws a new pair; Bob's totals go to it, and his month is asked again.
    let a = sync(&mut alice).await;
    assert_eq!(a.epoch, Some(2));
    bob.sync().await.unwrap();
    costs::read(&mut bob, MONTH, 1).await.unwrap();
    assert!(
        costs::wants_month(&bob).unwrap(),
        "the month goes again at a new key"
    );
    let b = costs::send(&mut bob).await.unwrap();
    assert_eq!(b.epoch, Some(2));
    assert!(b.pending.is_empty());
    assert!(!costs::wants_month(&bob).unwrap());
    let a = sync(&mut alice).await;
    assert_eq!(a.received[0].days["2026-11-20"].rows[0].cost, 3.0);
    drop((pa, pb, pd));
    w.finish().await;
}

async fn offline_the_totals_wait_and_go_once(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, pb, mut bob, _pc, _carol) = three(&w).await;
    sync(&mut alice).await;
    sync(&mut bob).await;
    // Offline: the totals wait in the queue, the later ones of the day in place of the first.
    let dead = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    w.relay.point_to(dead.local_addr().unwrap());
    drop(dead);
    costs::queue(&bob, totals(None, 100, 1.0)).unwrap();
    assert!(costs::sync(&mut bob, MONTH, 1).await.is_err());
    costs::queue(&bob, totals(None, 200, 4.0)).unwrap();
    assert!(costs::sync(&mut bob, MONTH, 2).await.is_err());
    assert_eq!(costs::view(&bob).unwrap().pending.len(), 1);
    w.relay.point_to(w.server.addr);
    // Back: they go, but the answer is lost; sent again, they count once.
    sync(&mut bob).await; // reads the key again after the cut
    costs::queue(&bob, totals(None, 300, 5.0)).unwrap();
    // (`lose_op_answer` loses the answer of the next append to a project's stream: the totals.)
    pb.liar.lies.lock().unwrap().lose_op_answer = true;
    assert!(costs::sync(&mut bob, MONTH, 3).await.is_err());
    assert_eq!(costs::view(&bob).unwrap().pending.len(), 1, "still waiting");
    let b = sync(&mut bob).await;
    assert!(b.pending.is_empty());
    let a = sync(&mut alice).await;
    let s = a.summary.expect("the summary");
    assert_eq!(s.team.cost, 5.0, "counted once");
    assert_eq!(a.received[0].days.len(), 1);
    // The same day again, the same: nothing waits, nothing goes for nothing.
    costs::queue(&bob, totals(None, 400, 5.0)).unwrap();
    assert!(costs::view(&bob).unwrap().pending.is_empty());
    costs::queue(&bob, totals(None, 500, 6.0)).unwrap();
    assert_eq!(
        costs::view(&bob).unwrap().pending.len(),
        1,
        "a day changed goes"
    );
    w.finish().await;
}

/// SR3-03, SR3-08: a device's queue refuses what the administrators would refuse (a cost not finite, negative or past a
/// day's bound, another member named), and its state still reads afterwards; the total that is within bounds goes.
async fn the_queue_refuses_what_is_out_of_bounds_and_the_state_still_reads(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, _pb, mut bob, _pc, _carol) = three(&w).await;
    sync(&mut alice).await;
    for cost in [1.0e12, -2.0e12, f64::NAN, f64::INFINITY] {
        assert!(
            costs::queue(&bob, totals(None, 100, cost)).is_err(),
            "{cost} refused"
        );
    }
    assert!(
        costs::queue(&bob, totals(Some(&member(3).to_string()), 100, 1.0)).is_err(),
        "Bob's totals are not put to Carol"
    );
    let b = sync(&mut bob).await;
    assert!(b.sent.is_none(), "nothing went");
    costs::queue(&bob, totals(None, 200, 2.0)).unwrap();
    sync(&mut bob).await;
    let a = sync(&mut alice).await;
    let s = a.summary.expect("a summary");
    assert_eq!(s.team.cost, 2.0);
    assert_eq!(a.received[0].author, member(2).to_string());
    w.finish().await;
}

both!(
    only_totals_go_sealed_and_a_member_reads_only_its_own,
    the_member_is_named_only_par_membre,
    the_key_turns_when_an_administrator_leaves,
    offline_the_totals_wait_and_go_once,
    the_queue_refuses_what_is_out_of_bounds_and_the_state_still_reads,
);
