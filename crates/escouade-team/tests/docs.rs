//! The docs of a team's project in its journal (TS-9) against the real `escouade-team-server`, once on SQLite and once
//! on PostgreSQL (`ESCOUADE_TEST_POSTGRES_URL`): a page in whole versions written on one device and read on another,
//! what the server keeps of it (neither its path nor its text); the same page written on both sides, its base found
//! for the three-way merge; the numbers of a counter (an accepted decision's) given by the server's order, never twice,
//! two taken at the same moment, one taken offline that waits, one kept as it is (an import), kept by a snapshot.

mod common;

use common::*;
use escouade_team::crypto::{ObjectId, Role};
use escouade_team::merge::merge3;
use escouade_team::Team;
use serde_json::json;

both!(
    a_page_written_here_is_read_there,
    a_page_written_on_both_sides_has_a_base,
    two_acceptances_at_once_never_share_a_number,
    a_number_taken_offline_waits_for_the_server,
    numbers_kept_as_they_are_and_through_a_snapshot,
);

type Crew = Vec<(Place, Team)>;

async fn crew(w: &World, n: u8) -> Crew {
    let first = w.place();
    let mut admin = w.create(&first, member(1)).await;
    let mut others = Vec::new();
    for i in 2..=n {
        let place = w.place();
        let device = w.join(&mut admin, &place, member(i), Role::Member).await;
        others.push((place, device));
    }
    let mut crew = vec![(first, admin)];
    crew.extend(others);
    for _ in 0..2 {
        for (_, t) in crew.iter_mut() {
            t.sync().await.unwrap();
            t.serve(NOW).await.unwrap();
        }
    }
    crew
}

const PAGE: ObjectId = ObjectId([0x51; 16]);

/// Every file the server keeps, as bytes.
fn server_bytes(w: &World) -> Vec<u8> {
    let mut out = Vec::new();
    let mut todo = vec![w.server.folder.clone()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                todo.push(p);
            } else if let Ok(b) = std::fs::read(&p) {
                out.extend(b);
            }
        }
    }
    out
}

fn holds(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

async fn a_page_written_here_is_read_there(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2).await;
    let p = c[0].1.create_project(None).await.unwrap();
    let text = "# Rédaction confidentielle\n\nLe tarif secret est de 42 licornes.\n";
    let v1 = c[0]
        .1
        .write_version(
            p,
            PAGE,
            vec![],
            json!({ "path": "produit/tarif-secret", "text": text }),
        )
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    let bob = &mut c[1].1;
    bob.follow(p).unwrap();
    bob.sync_project(p).await.unwrap();
    let got = bob.replica(&p).unwrap().versions(&PAGE).unwrap();
    assert_eq!(got.current.as_ref().unwrap().id, v1);
    assert_eq!(got.current.unwrap().content["text"], json!(text));
    assert!(got.conflicts.is_empty());
    // The server keeps neither its path nor its title nor its text.
    let kept = server_bytes(&w);
    for secret in [
        "tarif-secret",
        "Rédaction confidentielle",
        "42 licornes",
        "produit",
    ] {
        assert!(!holds(&kept, secret), "the server keeps « {secret} »");
    }
    w.finish().await;
}

async fn a_page_written_on_both_sides_has_a_base(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2).await;
    let p = c[0].1.create_project(None).await.unwrap();
    let base = "un\ndeux\ntrois\n";
    let v0 = c[0]
        .1
        .write_version(p, PAGE, vec![], json!({ "text": base }))
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.follow(p).unwrap();
    c[1].1.sync_project(p).await.unwrap();
    // Both write after the same version, apart.
    let ours = c[0]
        .1
        .write_version(p, PAGE, vec![v0], json!({ "text": "UN\ndeux\ntrois\n" }))
        .unwrap();
    let theirs = c[1]
        .1
        .write_version(p, PAGE, vec![v0], json!({ "text": "un\ndeux\nTROIS\n" }))
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.sync_project(p).await.unwrap();
    c[0].1.sync_project(p).await.unwrap();
    for (_, t) in c.iter_mut() {
        let r = t.replica(&p).unwrap();
        let v = r.versions(&PAGE).unwrap();
        assert_eq!(v.conflicts.len(), 1, "two heads, the same on each copy");
        let found = r.common_base(&PAGE, &ours, &theirs).unwrap().unwrap();
        assert_eq!(found.id, v0);
        let merged = merge3(
            found.content["text"].as_str().unwrap(),
            "UN\ndeux\ntrois\n",
            "un\ndeux\nTROIS\n",
        );
        assert_eq!(merged.as_deref(), Some("UN\ndeux\nTROIS\n"));
    }
    // The merge written after both heads: one head again.
    let merged = c[0]
        .1
        .write_version(
            p,
            PAGE,
            vec![ours, theirs],
            json!({ "text": "UN\ndeux\nTROIS\n" }),
        )
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.sync_project(p).await.unwrap();
    let v = c[1].1.replica(&p).unwrap().versions(&PAGE).unwrap();
    assert_eq!(v.current.unwrap().id, merged);
    assert!(v.conflicts.is_empty());
    w.finish().await;
}

fn decision(n: u8) -> ObjectId {
    ObjectId([0xd0 + n; 16])
}

async fn two_acceptances_at_once_never_share_a_number(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 3).await;
    let p = c[0].1.create_project(None).await.unwrap();
    c[0].1.sync_project(p).await.unwrap();
    for (_, t) in c.iter_mut().skip(1) {
        t.follow(p).unwrap();
        t.sync_project(p).await.unwrap();
    }
    // Each device accepts its decision having read the same journal: none knows of the others'.
    for (i, (_, t)) in c.iter_mut().enumerate() {
        t.take_number(p, decision(i as u8), "decision", None)
            .unwrap();
    }
    for (i, (_, t)) in c.iter_mut().enumerate() {
        // Not read back yet: no number, it waits.
        assert!(t
            .replica(&p)
            .unwrap()
            .number_waits("decision", &decision(i as u8))
            .unwrap());
        t.flush_project(p).await.unwrap();
    }
    for _ in 0..2 {
        for (_, t) in c.iter_mut() {
            t.sync_project(p).await.unwrap();
        }
    }
    let mut seen = Vec::new();
    for (_, t) in c.iter_mut() {
        let numbers = t.replica(&p).unwrap().numbers("decision").unwrap();
        let mut got: Vec<u32> = numbers.iter().map(|(_, n)| *n).collect();
        got.sort();
        assert_eq!(got, vec![1, 2, 3], "three numbers, none twice");
        seen.push(numbers);
    }
    assert!(
        seen.windows(2).all(|w| w[0] == w[1]),
        "each copy gives the same number to the same decision"
    );
    // A number asked again for a decision that has one changes nothing.
    let first = c[0]
        .1
        .replica(&p)
        .unwrap()
        .number("decision", &decision(0))
        .unwrap();
    c[0].1
        .take_number(p, decision(0), "decision", None)
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    assert_eq!(
        c[0].1
            .replica(&p)
            .unwrap()
            .number("decision", &decision(0))
            .unwrap(),
        first
    );
    w.finish().await;
}

async fn a_number_taken_offline_waits_for_the_server(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2).await;
    let p = c[0].1.create_project(None).await.unwrap();
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.follow(p).unwrap();
    c[1].1.sync_project(p).await.unwrap();
    // Alice is offline: her acceptance waits, without a number.
    w.relay.point_to(dead_address().await);
    c[0].1
        .take_number(p, decision(1), "decision", None)
        .unwrap();
    assert_eq!(c[0].1.flush_project(p).await.unwrap(), 0);
    assert!(c[0]
        .1
        .replica(&p)
        .unwrap()
        .number_waits("decision", &decision(1))
        .unwrap());
    assert!(c[0].1.sync_state().pending > 0);
    // Bob's reaches the server first: number 1.
    w.relay.point_to(w.server.addr);
    c[1].1
        .take_number(p, decision(2), "decision", None)
        .unwrap();
    c[1].1.sync_project(p).await.unwrap();
    assert_eq!(
        c[1].1
            .replica(&p)
            .unwrap()
            .number("decision", &decision(2))
            .unwrap(),
        Some(1)
    );
    // Alice back: hers takes the next one, on both copies.
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.sync_project(p).await.unwrap();
    for (_, t) in c.iter_mut() {
        let r = t.replica(&p).unwrap();
        assert_eq!(r.number("decision", &decision(2)).unwrap(), Some(1));
        assert_eq!(r.number("decision", &decision(1)).unwrap(), Some(2));
    }
    w.finish().await;
}

async fn numbers_kept_as_they_are_and_through_a_snapshot(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2).await;
    let p = c[0].1.create_project(None).await.unwrap();
    // An import keeps its numbers: 7 and 12; the next one taken is 13, and one asking 7 again gets the next.
    let alice = &mut c[0].1;
    alice
        .take_number(p, decision(1), "decision", Some(7))
        .unwrap();
    alice
        .take_number(p, decision(2), "decision", Some(12))
        .unwrap();
    alice.take_number(p, decision(3), "decision", None).unwrap();
    alice
        .take_number(p, decision(4), "decision", Some(7))
        .unwrap();
    alice.sync_project(p).await.unwrap();
    let want = vec![
        (decision(1), 7),
        (decision(2), 12),
        (decision(3), 13),
        (decision(4), 14),
    ];
    assert_eq!(
        alice.replica(&p).unwrap().numbers("decision").unwrap(),
        want
    );
    // A snapshot takes the place of the journal: a device that comes after has the same numbers.
    assert!(alice.make_snapshot(p).await.unwrap());
    let bob = &mut c[1].1;
    bob.follow(p).unwrap();
    let read = bob.sync_project(p).await.unwrap();
    assert!(read.from_snapshot);
    assert_eq!(bob.replica(&p).unwrap().numbers("decision").unwrap(), want);
    bob.take_number(p, decision(5), "decision", None).unwrap();
    bob.sync_project(p).await.unwrap();
    assert_eq!(
        bob.replica(&p)
            .unwrap()
            .number("decision", &decision(5))
            .unwrap(),
        Some(15)
    );
    // Rebuilt from its snapshot and its journal, the copy gives the same.
    bob.replica(&p).unwrap().rebuild().unwrap();
    assert_eq!(
        bob.replica(&p)
            .unwrap()
            .number("decision", &decision(5))
            .unwrap(),
        Some(15)
    );
    w.finish().await;
}

/// A closed port: the relay pointed at it, the server cannot be reached.
async fn dead_address() -> std::net::SocketAddr {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    l.local_addr().unwrap()
}
