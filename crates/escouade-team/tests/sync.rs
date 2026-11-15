//! The synchronization of the projects (TS-5) against the real `escouade-team-server`, once on SQLite and once on
//! PostgreSQL (`ESCOUADE_TEST_POSTGRES_URL`): two and three devices, concurrent writes, offline then back, a server
//! that plays an operation again, reorders, drops or forges one, a false signature, a member gone who reads nothing
//! new, a snapshot then a new device, the copy that resumes after a cut, an invitation that gives a project.

mod common;

use std::collections::BTreeMap;

use common::*;
use escouade_team::crypto::invite::{InviteLink, DEFAULT_LIFETIME};
use escouade_team::crypto::ops::{OpBody, Operation, Scope, What};
use escouade_team::crypto::{Purpose, Role};
use escouade_team::wire;
use escouade_team::{Alarm, Error, Team};
use serde_json::{json, Value};

both!(
    two_devices_share_a_projects_tickets,
    three_devices_converge_on_concurrent_writes,
    offline_writes_wait_and_leave_in_order,
    a_lying_server_is_caught,
    a_false_signature_is_refused,
    a_member_gone_reads_nothing_new,
    a_snapshot_then_a_new_device,
    the_copy_resumes_after_a_cut,
    an_invitation_gives_access_to_a_project,
);

type Crew = Vec<(Place, Team)>;

/// A team of `n` devices (member 1 its administrator), each having read the others and published its KeyPackage.
async fn crew(w: &World, n: u8, options: impl Fn(&mut Place)) -> Crew {
    let mut first = w.place();
    options(&mut first);
    let mut admin = w.create(&first, member(1)).await;
    let mut others = Vec::new();
    for i in 2..=n {
        let mut place = w.place();
        options(&mut place);
        let device = w.join(&mut admin, &place, member(i), Role::Member).await;
        others.push((place, device));
    }
    let mut crew = vec![(first, admin)];
    crew.extend(others);
    settle(&mut crew).await;
    crew
}

/// Every device reads the team and does its online work, twice (the KeyPackages read by all).
async fn settle(crew: &mut Crew) {
    for _ in 0..2 {
        for (_, t) in crew.iter_mut() {
            t.sync().await.unwrap();
            t.serve(NOW).await.unwrap();
        }
    }
}

fn fields(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn state(t: &mut Team, p: &escouade_team::crypto::ProjectId) -> escouade_team::replica::State {
    t.replica(p).unwrap().state().unwrap()
}

fn hex_of(p: &escouade_team::crypto::ProjectId) -> String {
    hex::encode(p.0)
}

/// A closed port: the relay pointed at it, the server cannot be reached.
async fn dead_address() -> std::net::SocketAddr {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    l.local_addr().unwrap()
}

async fn two_devices_share_a_projects_tickets(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2, |_| {}).await;
    let p = c[0].1.create_project(None).await.unwrap();
    let alice = &mut c[0].1;
    let t1 = alice
        .create_ticket(
            p,
            fields(&[
                ("title", json!("Écrire la synchro")),
                ("column", json!("todo")),
            ]),
        )
        .unwrap();
    let t2 = alice
        .create_ticket(p, fields(&[("title", json!("Brouillon"))]))
        .unwrap();
    alice
        .set_field(p, t1, "description", json!("Le journal d'opérations"))
        .unwrap();
    alice
        .move_ticket(p, t1, "doing", Some("a0".into()))
        .unwrap();
    alice.delete_ticket(p, t2).unwrap();
    // The interface reads the copy at once, before anything left.
    assert_eq!(
        alice.pending(&p).unwrap(),
        6,
        "the project's own creation, then five"
    );
    assert_eq!(
        alice
            .replica(&p)
            .unwrap()
            .objects(Some("ticket"))
            .unwrap()
            .len(),
        1
    );
    let done = alice.sync_project(p).await.unwrap();
    assert_eq!(done.sent, 6);
    assert_eq!(alice.pending(&p).unwrap(), 0);

    let bob = &mut c[1].1;
    bob.follow(p).unwrap();
    let read = bob.sync_project(p).await.unwrap();
    assert_eq!(read.applied, 6);
    let ticket = bob.replica(&p).unwrap().object(&t1).unwrap().unwrap();
    assert_eq!(ticket.fields["title"], json!("Écrire la synchro"));
    assert_eq!(ticket.fields["column"], json!("doing"));
    assert_eq!(ticket.fields["rank"], json!("a0"));
    assert_eq!(
        ticket.fields["description"],
        json!("Le journal d'opérations")
    );
    assert!(bob.replica(&p).unwrap().object(&t2).unwrap().is_none());

    // Bob writes back; Alice reads it.
    bob.set_field(p, t1, "title", json!("Écrire la synchro (TS-5)"))
        .unwrap();
    bob.sync_project(p).await.unwrap();
    c[0].1.sync_project(p).await.unwrap();
    let a = state(&mut c[0].1, &p);
    let b2 = state(&mut c[1].1, &p);
    assert_eq!(a, b2);

    // What the server keeps: no title, no field name, no object id, no author key (§3.8).
    let raw = c[0].1.client().read_project(&p.0, 0, None).await.unwrap();
    assert_eq!(raw.entries.len(), 7);
    let alice_key = c[0].1.device().author().key;
    for e in &raw.entries {
        for needle in [
            "crire".as_bytes(),
            b"title".as_slice(),
            &t1.0,
            &alice_key,
            &hex::encode(t1.0).into_bytes(),
        ] {
            assert!(
                !e.payload.windows(needle.len()).any(|x| x == needle),
                "the server sees {needle:?}"
            );
        }
    }
    w.finish().await;
}

async fn three_devices_converge_on_concurrent_writes(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 3, |_| {}).await;
    let p = c[0].1.create_project(None).await.unwrap();
    let t = c[0]
        .1
        .create_ticket(
            p,
            fields(&[("title", json!("A")), ("column", json!("todo"))]),
        )
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    for (_, d) in c.iter_mut().skip(1) {
        d.follow(p).unwrap();
        d.sync_project(p).await.unwrap();
    }
    // Each writes before reading the others: the same field by two, other fields by the third.
    c[0].1.set_field(p, t, "title", json!("Alice")).unwrap();
    c[1].1.set_field(p, t, "title", json!("Bob")).unwrap();
    c[1].1.move_ticket(p, t, "doing", None).unwrap();
    c[2].1
        .set_field(p, t, "description", json!("Carol"))
        .unwrap();
    for _ in 0..2 {
        for (_, d) in c.iter_mut() {
            d.sync_project(p).await.unwrap();
        }
    }
    let states: Vec<_> = c.iter_mut().map(|(_, d)| state(d, &p)).collect();
    assert_eq!(states[0], states[1]);
    assert_eq!(states[1], states[2]);
    let r = c[2].1.replica(&p).unwrap();
    let ticket = r.object(&t).unwrap().unwrap();
    assert_eq!(ticket.fields["description"], json!("Carol"));
    assert_eq!(ticket.fields["column"], json!("doing"));
    let title = ticket.fields["title"].clone();
    assert!(title == json!("Alice") || title == json!("Bob"));
    // The one that lost is kept, the same on every copy.
    let conflicts = r.conflicts(&t).unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].field, "title");
    assert_ne!(conflicts[0].value, title);
    assert_eq!(conflicts[0].winner, title);
    w.finish().await;
}

async fn offline_writes_wait_and_leave_in_order(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2, |_| {}).await;
    let p = c[0].1.create_project(None).await.unwrap();
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.follow(p).unwrap();
    c[1].1.sync_project(p).await.unwrap();

    w.relay.point_to(dead_address().await);
    let bob = &mut c[1].1;
    let t = bob
        .create_ticket(p, fields(&[("title", json!("Hors ligne"))]))
        .unwrap();
    bob.set_field(p, t, "title", json!("Écrit hors ligne"))
        .unwrap();
    bob.move_ticket(p, t, "review", None).unwrap();
    assert!(bob.sync_project(p).await.is_err());
    let s = bob.sync_state();
    assert_eq!((s.online, s.pending), (Some(false), 3));
    // Read offline: the copy is whole.
    assert_eq!(
        bob.replica(&p).unwrap().object(&t).unwrap().unwrap().fields["column"],
        json!("review")
    );

    w.relay.point_to(w.server.addr);
    let back = bob.sync_project(p).await.unwrap();
    assert_eq!(back.sent, 3);
    let s = bob.sync_state();
    assert_eq!((s.online, s.pending), (Some(true), 0));
    let read = c[0].1.sync_project(p).await.unwrap();
    assert_eq!(read.applied, 3);
    assert_eq!(state(&mut c[0].1, &p), state(&mut c[1].1, &p));

    // An answer lost: the operation is sent again, and counted once.
    c[1].0.liar.lies.lock().unwrap().lose_op_answer = true;
    let bob = &mut c[1].1;
    bob.set_field(p, t, "title", json!("Une fois")).unwrap();
    let first = bob.sync_project(p).await.unwrap();
    assert_eq!(first.sent, 0);
    assert_eq!(bob.pending(&p).unwrap(), 1);
    bob.sync_project(p).await.unwrap();
    assert_eq!(bob.pending(&p).unwrap(), 0);
    let read = c[0].1.sync_project(p).await.unwrap();
    assert_eq!((read.entries, read.applied), (2, 1));
    assert_eq!(state(&mut c[0].1, &p), state(&mut c[1].1, &p));
    w.finish().await;
}

/// A project of three operations by Alice, followed by Bob: the lie set on Bob's reads of it.
async fn lied_to(c: &mut Crew, set: impl FnOnce(&mut Lies)) -> escouade_team::crypto::ProjectId {
    let p = c[0].1.create_project(None).await.unwrap();
    for i in 0..3 {
        c[0].1
            .create_ticket(p, fields(&[("title", json!(format!("T{i}")))]))
            .unwrap();
    }
    c[0].1.sync_project(p).await.unwrap();
    {
        let mut lies = c[1].0.liar.lies.lock().unwrap();
        *lies = Lies::default();
        lies.project = Some(hex_of(&p));
        set(&mut lies);
    }
    c[1].1.follow(p).unwrap();
    p
}

fn journal_alarm(e: &Error) -> bool {
    e.alarm() == Some(Alarm::Journal)
}

async fn a_lying_server_is_caught(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2, |_| {}).await;

    let p = lied_to(&mut c, |l| l.project_replay = Some(1)).await;
    let e = c[1].1.sync_project(p).await.unwrap_err();
    assert!(matches!(e, Error::Replayed { seq: 5 }), "{e}");
    assert!(journal_alarm(&e));

    let p = lied_to(&mut c, |l| l.project_swap = Some((1, 2))).await;
    let e = c[1].1.sync_project(p).await.unwrap_err();
    assert!(matches!(e, Error::Reordered { seq: 1 }), "{e}");
    assert!(journal_alarm(&e));

    let p = lied_to(&mut c, |l| l.project_drop = Some(2)).await;
    let e = c[1].1.sync_project(p).await.unwrap_err();
    assert!(matches!(e, Error::Reordered { seq: 3 }), "{e}");

    let p = lied_to(&mut c, |l| l.project_flip = Some(2)).await;
    let e = c[1].1.sync_project(p).await.unwrap_err();
    assert!(matches!(e, Error::Forged { seq: 2 }), "{e}");
    assert!(journal_alarm(&e));

    // An operation of Bob the server took, then never shows.
    let p = lied_to(&mut c, |l| l.project_hide_after = Some(4)).await;
    c[1].1.sync_project(p).await.unwrap();
    let t = c[1]
        .1
        .create_ticket(p, fields(&[("title", json!("Le mien"))]))
        .unwrap();
    let _ = t;
    let e = c[1].1.sync_project(p).await.unwrap_err();
    assert!(matches!(e, Error::Withheld { seq: 5 }), "{e}");
    assert!(journal_alarm(&e));

    // An entry read, then changed; then hidden.
    let p = lied_to(&mut c, |_| {}).await;
    c[1].1.sync_project(p).await.unwrap();
    c[1].0.liar.lies.lock().unwrap().project_flip = Some(4);
    let e = c[1].1.sync_project(p).await.unwrap_err();
    assert!(matches!(e, Error::Rewritten { seq: 4 }), "{e}");
    {
        let mut lies = c[1].0.liar.lies.lock().unwrap();
        lies.project_flip = None;
        lies.project_hide_after = Some(1);
    }
    let e = c[1].1.sync_project(p).await.unwrap_err();
    assert!(matches!(e, Error::ServerRollback { seen: 4 }), "{e}");
    assert_eq!(e.alarm(), Some(Alarm::Rollback));

    // Honest again: Bob reads on.
    *c[1].0.liar.lies.lock().unwrap() = Lies::default();
    c[1].1.sync_project(p).await.unwrap();
    assert_eq!(state(&mut c[0].1, &p), state(&mut c[1].1, &p));
    w.finish().await;
}

async fn a_false_signature_is_refused(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2, |_| {}).await;
    let p = c[0].1.create_project(None).await.unwrap();
    c[1].1.follow(p).unwrap();
    // Bob writes in the name of Alice: her author, his signature.
    let checkpoint = c[1].1.checkpoint().unwrap();
    let op = Operation {
        project: p,
        author: c[0].1.device().author(),
        dseq: 1,
        prev: [0; 32],
        team_epoch: checkpoint.epoch,
        team_head: checkpoint.head,
        hlc: escouade_team::crypto::hlc::Hlc {
            wall: 1,
            count: 0,
            device: c[0].1.device_id(),
        },
        body: OpBody::Delete {
            object: escouade_team::crypto::ObjectId([1; 16]),
        },
    };
    let sealed = c[1]
        .1
        .device()
        .seal_for(What::Operation, &p, Scope::Team, op.to_bytes())
        .unwrap();
    c[1].1
        .client()
        .append_project(&p.0, &wire::encode(&sealed))
        .await
        .unwrap();
    c[0].1.follow(p).unwrap();
    let e = c[0].1.sync_project(p).await.unwrap_err();
    assert!(matches!(e, Error::Forged { seq: 1 }), "{e}");
    w.finish().await;
}

async fn a_member_gone_reads_nothing_new(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 3, |_| {}).await;
    let whole = c[0].1.create_project(None).await.unwrap();
    let kept = c[0]
        .1
        .create_project(Some([member(1), member(2)].into_iter().collect()))
        .await
        .unwrap();
    settle(&mut c).await;
    assert!(c[1].1.device().in_project(&kept), "Bob is in its group");
    assert!(!c[2].1.device().in_project(&kept));
    c[0].1
        .create_ticket(kept, fields(&[("title", json!("Pour Alice et Bob"))]))
        .unwrap();
    c[0].1
        .create_ticket(whole, fields(&[("title", json!("Pour tous"))]))
        .unwrap();
    c[0].1.sync_project(kept).await.unwrap();
    c[0].1.sync_project(whole).await.unwrap();
    for (_, d) in c.iter_mut().skip(1) {
        d.follow(kept).unwrap();
        d.follow(whole).unwrap();
        d.sync_project(whole).await.unwrap();
    }
    c[1].1.sync_project(kept).await.unwrap();
    assert_eq!(state(&mut c[0].1, &kept), state(&mut c[1].1, &kept));
    // Carol, in the team, does not read the project kept to Alice and Bob.
    assert!(matches!(
        c[2].1.sync_project(kept).await,
        Err(Error::NoAccess)
    ));

    // Bob leaves the team: the next operations are at the next epoch, of the team and of the project's group.
    let before = c[0].1.device().project_epoch(&kept).unwrap();
    c[0].1.remove_member(member(2)).await.unwrap();
    c[0].1.serve(NOW).await.unwrap();
    let after = c[0].1.device().project_epoch(&kept).unwrap();
    assert!(after > before, "the project's group moved on");
    c[0].1
        .create_ticket(kept, fields(&[("title", json!("Après Bob"))]))
        .unwrap();
    c[0].1
        .create_ticket(whole, fields(&[("title", json!("Après Bob, pour tous"))]))
        .unwrap();
    c[0].1.sync_project(kept).await.unwrap();
    c[0].1.sync_project(whole).await.unwrap();
    // The server no longer serves Bob; and the keys of what is new, he never had.
    assert!(c[1].1.sync_project(whole).await.is_err());
    let raw = c[0]
        .1
        .client()
        .read_project(&whole.0, 0, None)
        .await
        .unwrap();
    let last: escouade_team::crypto::ops::Sealed =
        wire::decode(&raw.entries.last().unwrap().payload).unwrap();
    assert!(c[1]
        .1
        .device()
        .record_for(&whole, Scope::Team, last.epoch)
        .is_err());
    let raw = c[0]
        .1
        .client()
        .read_project(&kept.0, 0, None)
        .await
        .unwrap();
    let last: escouade_team::crypto::ops::Sealed =
        wire::decode(&raw.entries.last().unwrap().payload).unwrap();
    assert_eq!(last.epoch, after);
    assert!(c[1]
        .1
        .device()
        .record_for(&kept, Scope::Project, after)
        .is_err());
    // Carol reads what is new of the whole team's project.
    c[2].1.sync().await.unwrap();
    let read = c[2].1.sync_project(whole).await.unwrap();
    assert_eq!(read.applied, 1);
    w.finish().await;
}

async fn a_snapshot_then_a_new_device(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2, |p| p.options.snapshot_every = 5).await;
    let p = c[0].1.create_project(None).await.unwrap();
    let mut tickets = Vec::new();
    for i in 0..2 {
        tickets.push(
            c[0].1
                .create_ticket(p, fields(&[("title", json!(format!("T{i}")))]))
                .unwrap(),
        );
    }
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.follow(p).unwrap();
    c[1].1.sync_project(p).await.unwrap();
    // Bob away; Alice writes on, past the threshold.
    for i in 2..8 {
        tickets.push(
            c[0].1
                .create_ticket(p, fields(&[("title", json!(format!("T{i}")))]))
                .unwrap(),
        );
    }
    c[0].1
        .set_field(p, tickets[0], "title", json!("T0 bis"))
        .unwrap();
    let done = c[0].1.sync_project(p).await.unwrap();
    assert!(done.snapshot_made);
    let raw = c[0].1.client().read_project(&p.0, 0, None).await.unwrap();
    assert!(raw.entries.is_empty(), "the server forgot the operations");
    assert_eq!(raw.snapshot, Some(10));

    // A new device of the team starts from the snapshot.
    let dave_place = w.place();
    let mut dave = w
        .join(&mut c[0].1, &dave_place, member(4), Role::Member)
        .await;
    dave.sync().await.unwrap();
    dave.follow(p).unwrap();
    let read = dave.sync_project(p).await.unwrap();
    assert!(read.from_snapshot);
    assert_eq!(state(&mut c[0].1, &p), state(&mut dave, &p));
    assert_eq!(
        dave.replica(&p)
            .unwrap()
            .objects(Some("ticket"))
            .unwrap()
            .len(),
        8
    );
    // Bob, who was away past it, too.
    c[1].1.sync().await.unwrap();
    let read = c[1].1.sync_project(p).await.unwrap();
    assert!(read.from_snapshot);
    assert_eq!(state(&mut c[0].1, &p), state(&mut c[1].1, &p));
    // Then the operations after it, as before.
    c[0].1
        .set_field(p, tickets[1], "title", json!("T1 bis"))
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    let read = dave.sync_project(p).await.unwrap();
    assert_eq!(read.applied, 1);
    assert_eq!(state(&mut c[0].1, &p), state(&mut dave, &p));
    // A snapshot signed by nobody of the team is refused: Mallory's bytes in its place.
    let forged = escouade_team::crypto::ops::Sealed {
        scope: Scope::Team,
        epoch: 0,
        nonce: [0; 24],
        ciphertext: vec![1; 64],
    };
    c[0].1
        .client()
        .put_snapshot(&p.0, 11, wire::encode(&forged))
        .await
        .unwrap();
    let erin_place = w.place();
    let mut erin = w
        .join(&mut c[0].1, &erin_place, member(5), Role::Member)
        .await;
    erin.follow(p).unwrap();
    let e = erin.sync_project(p).await.unwrap_err();
    assert!(matches!(e, Error::Forged { .. }), "{e}");
    w.finish().await;
}

async fn the_copy_resumes_after_a_cut(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2, |_| {}).await;
    let p = c[0].1.create_project(None).await.unwrap();
    for i in 0..4 {
        c[0].1
            .create_ticket(p, fields(&[("title", json!(format!("T{i}")))]))
            .unwrap();
    }
    c[0].1.sync_project(p).await.unwrap();
    {
        let mut lies = c[1].0.liar.lies.lock().unwrap();
        lies.project = Some(hex_of(&p));
        lies.project_hide_after = Some(2);
    }
    c[1].1.follow(p).unwrap();
    let first = c[1].1.sync_project(p).await.unwrap();
    assert_eq!(first.applied, 2);
    *c[1].0.liar.lies.lock().unwrap() = Lies::default();

    // The device stops; it opens again from its folder and reads from the last number it saw.
    let id = c[1].1.id();
    let (place, team) = c.pop().unwrap();
    drop(team);
    let mut bob = Team::open(place.setup(), id).unwrap();
    assert_eq!(bob.projects(), vec![p]);
    let again = bob.sync_project(p).await.unwrap();
    assert_eq!((again.entries, again.applied), (3, 3));
    assert_eq!(state(&mut c[0].1, &p), state(&mut bob, &p));
    // The copy replayed from its journal is the same.
    let before = state(&mut bob, &p);
    bob.replica(&p).unwrap().rebuild().unwrap();
    assert_eq!(before, state(&mut bob, &p));
    w.finish().await;
}

async fn an_invitation_gives_access_to_a_project(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2, |_| {}).await;
    let kept = c[0]
        .1
        .create_project(Some([member(1)].into_iter().collect()))
        .await
        .unwrap();
    c[0].1
        .create_ticket(kept, fields(&[("title", json!("Avant Erin"))]))
        .unwrap();
    c[0].1.sync_project(kept).await.unwrap();
    let invited = c[0]
        .1
        .invite_with_projects(
            Purpose::NewMember(Role::Member),
            NOW,
            DEFAULT_LIFETIME,
            vec![kept],
        )
        .await
        .unwrap();
    let link = InviteLink::parse(&invited.url).unwrap();
    let place = w.place();
    let mut erin = Team::join_link(place.setup(), &link).await.unwrap();
    c[0].1.sync().await.unwrap();
    c[0].1.serve(NOW).await.unwrap();
    assert!(erin.sync().await.unwrap().joined);
    erin.serve(NOW).await.unwrap();
    c[0].1.sync().await.unwrap();
    c[0].1.serve(NOW).await.unwrap();
    assert!(c[0]
        .1
        .project_members(&kept)
        .unwrap()
        .contains(&erin.identity().member));
    erin.sync().await.unwrap();
    assert!(erin.device().in_project(&kept));
    erin.follow(kept).unwrap();
    // She reads what came before her, by the project's history.
    let read = erin.sync_project(kept).await.unwrap();
    assert_eq!(read.applied, 1);
    assert_eq!(state(&mut c[0].1, &kept), state(&mut erin, &kept));
    // Bob, not invited to it, still does not.
    c[1].1.follow(kept).unwrap();
    assert!(matches!(
        c[1].1.sync_project(kept).await,
        Err(Error::NoAccess)
    ));
    w.finish().await;
}
