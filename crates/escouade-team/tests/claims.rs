//! The Kanban of a team (TS-6) against the real `escouade-team-server`, once on SQLite and once on PostgreSQL
//! (`ESCOUADE_TEST_POSTGRES_URL`): two claims at once (one passes, the other is told who holds it, by name), a lease
//! renewed then run out (« interrompu »), a write under a token taken over refused and dropped, a member removed who
//! takes nothing more, what the server keeps (opaque ids, devices, hours), the tickets' numbers, offline then back
//! (no claim offline, the outbox leaves, the conflicts show, a description merged three ways).

mod common;

use std::collections::BTreeMap;
use std::time::Duration;

use common::*;
use escouade_team::client::ClientError;
use escouade_team::crypto::ops::Value;
use escouade_team::crypto::{DeviceId, ProjectId, Role};
use escouade_team::merge::merge3;
use escouade_team::replica::Fence;
use escouade_team::{Error, Team};
use serde_json::json;

both!(
    two_claims_at_once_one_passes,
    a_lease_is_renewed_runs_out_and_is_given_back,
    a_write_under_a_token_taken_over_is_refused,
    a_member_removed_takes_nothing,
    the_server_sees_opaque_ids_devices_and_hours,
    the_numbers_of_the_tickets_come_from_the_server,
    offline_no_claim_then_the_conflicts_show,
    a_ticket_handed_with_its_deposit,
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
    settle(&mut crew).await;
    crew
}

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

fn claim_id() -> [u8; 16] {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).unwrap();
    b
}

/// A project of the whole team, followed by every device, its names written.
async fn project(c: &mut Crew) -> ProjectId {
    let p = c[0].1.create_project(None).await.unwrap();
    for (i, (_, t)) in c.iter_mut().enumerate() {
        t.follow(p).unwrap();
        t.sync_project(p).await.unwrap();
        t.set_name(p, ["Alice", "Bob", "Carol"][i]).unwrap();
        t.sync_project(p).await.unwrap();
    }
    for (_, t) in c.iter_mut() {
        t.sync_project(p).await.unwrap();
    }
    p
}

fn device(t: &Team) -> DeviceId {
    DeviceId(t.client().device())
}

fn refused(e: Error) -> (String, escouade_team::wire::Claim) {
    match e {
        Error::Client(ClientError::Claim { code, claim }) => (code, *claim),
        e => panic!("not a refused claim: {e}"),
    }
}

/// Alice and Bob take the same ticket at the same moment: one passes; the other is told who holds it, and says it by
/// the name the holder wrote in the project (the server only knows a device's id).
async fn two_claims_at_once_one_passes(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2).await;
    let p = project(&mut c).await;
    let claim = claim_id();
    let (first, second) = c.split_at_mut(1);
    let (alice, bob) = (&mut first[0].1, &mut second[0].1);
    let (a, b2) = tokio::join!(
        alice.take_claim(p, claim, None),
        bob.take_claim(p, claim, None)
    );
    let (winner, loser, mut loser_team) = match (a, b2) {
        (Ok(won), Err(e)) => (won, refused(e), 1),
        (Err(e), Ok(won)) => (won, refused(e), 0),
        (a, b) => panic!(
            "one claim passes, the other is refused: {:?} {:?}",
            a.is_ok(),
            b.is_ok()
        ),
    };
    assert_eq!(loser.0, "taken");
    assert_eq!(loser.1.holder, winner.claim.holder);
    assert_eq!(
        winner.claim.until - winner.now,
        600,
        "a lease of 10 minutes"
    );
    let holder = DeviceId(winner.claim.holder.clone().unwrap().try_into().unwrap());
    let name = c[loser_team].1.name_of(p, &holder);
    assert_eq!(
        name.as_deref(),
        Some(if loser_team == 1 { "Alice" } else { "Bob" })
    );
    // The claims list it, with the server's time.
    let list = c[loser_team].1.claims(p).await.unwrap();
    assert_eq!(list.claims.len(), 1);
    assert!(list.claims[0].until > list.now);
    loser_team = 1 - loser_team;
    let _ = loser_team;
    w.finish().await;
}

/// The lease is renewed while the agent runs; run out (the device asleep), the ticket is free and the claims still say
/// whose it was (« interrompu chez Alice »); Alice's renewal then comes too late, and Bob gives it back.
async fn a_lease_is_renewed_runs_out_and_is_given_back(b: Backend) {
    let w = World::on_with(&b, |c| c.limits.claim_lease = Duration::from_secs(2)).await;
    let mut c = crew(&w, 2).await;
    let p = project(&mut c).await;
    let claim = claim_id();
    let a = c[0].1.take_claim(p, claim, None).await.unwrap();
    let renewed = c[0].1.renew_claim(p, claim, a.claim.token).await.unwrap();
    assert_eq!(
        renewed.claim.token, a.claim.token,
        "a renewal keeps the token"
    );
    assert_eq!(
        refused(c[1].1.take_claim(p, claim, None).await.unwrap_err()).0,
        "taken"
    );
    tokio::time::sleep(Duration::from_millis(3100)).await;
    let list = c[1].1.claims(p).await.unwrap();
    let seen = &list.claims[0];
    assert_eq!(seen.holder.as_deref(), Some(device(&c[0].1).0.as_slice()));
    assert!(seen.until <= list.now, "« interrompu chez Alice »");
    let bob = c[1]
        .1
        .take_claim(p, claim, Some(seen.version))
        .await
        .unwrap();
    assert_eq!(bob.claim.token, 2);
    assert_eq!(
        bob.claim.last_holder.as_deref(),
        Some(device(&c[0].1).0.as_slice())
    );
    let late = refused(
        c[0].1
            .renew_claim(p, claim, a.claim.token)
            .await
            .unwrap_err(),
    );
    assert_eq!(late.0, "claim_lost");
    let back = c[1]
        .1
        .release_claim(p, claim, bob.claim.token)
        .await
        .unwrap();
    assert_eq!((back.claim.holder, back.claim.until), (None, 0));
    w.finish().await;
}

/// Alice holds the ticket and moves it, but her move waits (she is cut off); her lease runs out, Bob takes the ticket
/// and moves it. Back, Alice's move is refused by the server: dropped from her outbox and her copy, which then says
/// Bob's column; her other write, under no claim, still leaves, and the chain of her operations has no hole.
async fn a_write_under_a_token_taken_over_is_refused(b: Backend) {
    let w = World::on_with(&b, |c| c.limits.claim_lease = Duration::from_secs(2)).await;
    let mut c = crew(&w, 2).await;
    let p = project(&mut c).await;
    let claim = claim_id();
    let t = c[0]
        .1
        .create_ticket(
            p,
            fields(&[
                ("title", json!("Le bail")),
                ("column", json!("todo")),
                ("claim", json!(hex::encode(claim))),
            ]),
        )
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.sync_project(p).await.unwrap();
    let a = c[0].1.take_claim(p, claim, None).await.unwrap();
    let fence_a = Fence {
        claim,
        token: a.claim.token,
    };
    let (first, second) = c.split_at_mut(1);
    let (alice, bob) = (&mut first[0].1, &mut second[0].1);
    let base = alice.field_stamp(&p, &t, "column").unwrap();
    alice
        .write_fenced(
            p,
            escouade_team::crypto::ops::OpBody::Move {
                object: t,
                column: "doing".into(),
                rank: None,
                base,
            },
            Some(fence_a),
        )
        .unwrap();
    alice
        .set_field(p, t, "description", json!("Écrit pendant la coupure"))
        .unwrap();
    // Nothing left yet: the lease runs out, Bob takes it over and moves it.
    tokio::time::sleep(Duration::from_millis(3100)).await;
    let bb = bob.take_claim(p, claim, None).await.unwrap();
    let base = bob.field_stamp(&p, &t, "column").unwrap();
    bob.write_fenced(
        p,
        escouade_team::crypto::ops::OpBody::Move {
            object: t,
            column: "review".into(),
            rank: None,
            base,
        },
        Some(Fence {
            claim,
            token: bb.claim.token,
        }),
    )
    .unwrap();
    bob.sync_project(p).await.unwrap();
    // Alice comes back.
    let back = alice.sync_project(p).await.unwrap();
    assert_eq!(back.sent, 1, "the description alone left");
    let refusals = alice.take_refused();
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].dropped, 1);
    assert_eq!(
        refusals[0].claim.holder.as_deref(),
        Some(device(bob).0.as_slice())
    );
    assert_eq!(alice.pending(&p).unwrap(), 0);
    let seen = alice.replica(&p).unwrap().object(&t).unwrap().unwrap();
    assert_eq!(seen.fields["column"], json!("review"), "Bob's column");
    bob.sync_project(p)
        .await
        .expect("Alice's chain has no hole");
    let theirs = bob.replica(&p).unwrap().object(&t).unwrap().unwrap();
    assert_eq!(theirs.fields["column"], json!("review"));
    assert_eq!(
        theirs.fields["description"],
        json!("Écrit pendant la coupure")
    );
    assert_eq!(
        alice.replica(&p).unwrap().state().unwrap(),
        bob.replica(&p).unwrap().state().unwrap()
    );
    w.finish().await;
}

/// Bob holds a ticket, then leaves the team: the server cuts his devices, he takes nothing more, and the ticket he held
/// is free. Carol, out of a project kept to Alice and Bob, neither reads it nor takes its tickets.
async fn a_member_removed_takes_nothing(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 3).await;
    let p = project(&mut c).await;
    let claim = claim_id();
    c[1].1.take_claim(p, claim, None).await.unwrap();
    assert_eq!(
        refused(c[0].1.take_claim(p, claim, None).await.unwrap_err()).0,
        "taken"
    );
    let kept = c[0]
        .1
        .create_project(Some([member(1), member(2)].into_iter().collect()))
        .await
        .unwrap();
    settle(&mut c).await;
    c[2].1.follow(kept).unwrap();
    assert!(matches!(
        c[2].1.take_claim(kept, claim_id(), None).await,
        Err(Error::NoAccess)
    ));
    assert!(matches!(
        c[2].1.sync_project(kept).await,
        Err(Error::NoAccess)
    ));
    assert!(matches!(c[2].1.next_key(kept).await, Err(Error::NoAccess)));
    c[1].1.follow(kept).unwrap();
    c[1].1.sync_project(kept).await.unwrap();
    c[1].1
        .take_claim(kept, claim_id(), None)
        .await
        .expect("Bob is of the project");

    c[0].1.remove_member(member(2)).await.unwrap();
    c[0].1.serve(NOW).await.unwrap();
    assert!(
        c[1].1.take_claim(p, claim_id(), None).await.is_err(),
        "Bob takes nothing more"
    );
    let mine = c[0].1.take_claim(p, claim, None).await.unwrap();
    assert_eq!(
        mine.claim.holder.as_deref(),
        Some(device(&c[0].1).0.as_slice())
    );
    w.finish().await;
}

/// What the server keeps of the claims, the counter and the access: opaque ids, ids of devices, hours and numbers.
/// Never a title, a prefix, a name, a ticket's object id, nor a member's id (the team's own stream, which serves the roster,
/// is TS-2's: §3.8).
async fn the_server_sees_opaque_ids_devices_and_hours(b: Backend) {
    if !matches!(b, Backend::Sqlite) {
        return; // The base is read as a file: SQLite's.
    }
    let w = World::on(&b).await;
    let mut c = crew(&w, 2).await;
    let p = project(&mut c).await;
    let claim = claim_id();
    let n = c[0].1.next_key(p).await.unwrap();
    let t = c[0]
        .1
        .create_ticket(
            p,
            fields(&[
                ("title", json!("Titre secret du ticket")),
                ("key", json!(format!("DEMSECRET-{n}"))),
                ("claim", json!(hex::encode(claim))),
            ]),
        )
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    let a = c[0].1.take_claim(p, claim, None).await.unwrap();
    c[0].1.renew_claim(p, claim, a.claim.token).await.unwrap();
    let db = rusqlite::Connection::open(w.server.folder.join("team.db")).unwrap();
    let mut everything: Vec<u8> = Vec::new();
    for table in [
        "claims",
        "project_keys",
        "project_access",
        "project_entries",
        "projects",
        "project_snapshots",
    ] {
        let mut stmt = db.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let cols = stmt.column_count();
        let mut rows = stmt.query([]).unwrap();
        while let Some(r) = rows.next().unwrap() {
            for i in 0..cols {
                match r.get_ref(i).unwrap() {
                    rusqlite::types::ValueRef::Blob(b) => everything.extend_from_slice(b),
                    rusqlite::types::ValueRef::Text(t) => everything.extend_from_slice(t),
                    rusqlite::types::ValueRef::Integer(n) => {
                        everything.extend_from_slice(n.to_string().as_bytes())
                    }
                    _ => {}
                }
            }
        }
    }
    let has = |needle: &[u8]| everything.windows(needle.len()).any(|w| w == needle);
    for secret in [
        &b"Titre secret"[..],
        b"DEMSECRET",
        b"Alice",
        b"Bob",
        &t.0,
        &member(1).0,
        &member(2).0,
    ] {
        assert!(!has(secret), "the server keeps {secret:?}");
    }
    // What it keeps of the claim: its opaque id, the device, the hours, the token.
    let (ticket, holder, until, token): (Vec<u8>, Vec<u8>, i64, i64) = db
        .query_row(
            "SELECT ticket_id, holder, until, token FROM claims",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(ticket, claim.to_vec());
    assert_eq!(holder, device(&c[0].1).0.to_vec());
    assert!(until > 0);
    assert_eq!(token, 1);
    let cols: Vec<String> = db
        .prepare("SELECT name FROM pragma_table_info('claims')")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(
        cols,
        [
            "team_id",
            "project_id",
            "ticket_id",
            "holder",
            "until",
            "token",
            "version",
            "claimed_at",
            "renewed_at",
            "last_holder",
            // The device that handed the ticket, while it is not taken (TS-17).
            "handed_by"
        ]
    );
    let keys: i64 = db
        .query_row("SELECT n FROM project_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(keys as u64, n);
    w.finish().await;
}

/// The numbers of a project's tickets, one each, from the server's counter, asked at once by two devices.
async fn the_numbers_of_the_tickets_come_from_the_server(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2).await;
    let p = project(&mut c).await;
    let mut got = Vec::new();
    for _ in 0..5 {
        let (first, second) = c.split_at_mut(1);
        let (a, b2) = tokio::join!(first[0].1.next_key(p), second[0].1.next_key(p));
        got.push(a.unwrap());
        got.push(b2.unwrap());
    }
    got.sort();
    assert_eq!(got, (1..=10).collect::<Vec<u64>>());
    w.finish().await;
}

/// Offline, Bob reads and writes his tickets but takes none (« hors ligne »): the error says the server is not
/// reached. Meanwhile Alice changes the same title and another line of the description. Back, his outbox leaves; the
/// title is the last writer's and the loser sees his own value kept as a conflict; the description changed on two
/// lines merges three ways.
async fn offline_no_claim_then_the_conflicts_show(b: Backend) {
    let w = World::on(&b).await;
    let mut c = crew(&w, 2).await;
    let p = project(&mut c).await;
    let base_text = "Contexte\nÀ faire\nCritères\n";
    let t = c[0]
        .1
        .create_ticket(
            p,
            fields(&[("title", json!("Titre")), ("description", json!(base_text))]),
        )
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.sync_project(p).await.unwrap();

    w.relay.point_to({
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        l.local_addr().unwrap()
    });
    let (first, second) = c.split_at_mut(1);
    let (alice, bob) = (&mut first[0].1, &mut second[0].1);
    assert!(matches!(
        bob.take_claim(p, claim_id(), None).await,
        Err(Error::Client(ClientError::Net(_)))
    ));
    assert_eq!(bob.sync_state().online, Some(false));
    bob.set_field(p, t, "title", json!("Titre de Bob")).unwrap();
    bob.set_field(
        p,
        t,
        "description",
        json!("Contexte de Bob\nÀ faire\nCritères\n"),
    )
    .unwrap();
    assert_eq!(bob.sync_state().pending, 2);
    // Alice writes too, from the same bases (her writes are later: hers win the title).
    tokio::time::sleep(Duration::from_millis(20)).await;
    alice
        .set_field(p, t, "title", json!("Titre d'Alice"))
        .unwrap();
    alice
        .set_field(
            p,
            t,
            "description",
            json!("Contexte\nÀ faire\nCritères d'Alice\n"),
        )
        .unwrap();
    w.relay.point_to(w.server.addr);
    alice.sync_project(p).await.unwrap();
    let back = bob.sync_project(p).await.unwrap();
    assert_eq!(back.sent, 2);
    alice.sync_project(p).await.unwrap();
    for t2 in [&mut *alice, &mut *bob] {
        let seen = t2.replica(&p).unwrap().object(&t).unwrap().unwrap();
        assert_eq!(seen.fields["title"], json!("Titre d'Alice"));
    }
    let conflicts = bob.replica(&p).unwrap().conflicts(&t).unwrap();
    let title = conflicts
        .iter()
        .find(|c| c.field == "title")
        .expect("the title's conflict");
    assert_eq!(title.value, json!("Titre de Bob"));
    assert_eq!(
        title.stamp.device,
        device(bob),
        "Bob lost: his card says so"
    );
    assert_eq!(title.winner_stamp.device, device(alice));
    let desc = conflicts
        .iter()
        .find(|c| c.field == "description")
        .expect("the description's");
    let merged = merge3(
        desc.base.as_ref().unwrap().as_str().unwrap(),
        desc.value.as_str().unwrap(),
        desc.winner.as_str().unwrap(),
    )
    .expect("two lines apart: merged");
    assert_eq!(merged, "Contexte de Bob\nÀ faire\nCritères d'Alice\n");
    // « Reprendre le mien »: a write after the winner settles it, on both devices.
    bob.set_field(p, t, "title", json!("Titre de Bob")).unwrap();
    bob.set_field(p, t, "description", json!(merged)).unwrap();
    bob.sync_project(p).await.unwrap();
    alice.sync_project(p).await.unwrap();
    assert!(alice.replica(&p).unwrap().conflicts(&t).unwrap().is_empty());
    let seen = alice.replica(&p).unwrap().object(&t).unwrap().unwrap();
    assert_eq!(seen.fields["title"], json!("Titre de Bob"));
    w.finish().await;
}

/// TS-17: Alice passes a ticket to Bob. Its deposit (the conversation, a patch) is encrypted on the server, the offer is
/// in the journal for Bob alone to act on, the claim is Bob's under a new token at once (Alice's old one writes no
/// more). Abandoned, Alice takes it back and the deposit is erased; offered again, Bob takes it (a new token), reads the
/// deposit, and erases it with the offer. A machine in server mode says so in the project.
async fn a_ticket_handed_with_its_deposit(b: Backend) {
    use escouade_team::handoff::Offer;
    let w = World::on(&b).await;
    let mut c = crew(&w, 2).await;
    let p = project(&mut c).await;
    let claim = claim_id();
    let bob_device = device(&c[1].1);
    let alice_device = device(&c[0].1);
    let a = c[0].1.take_claim(p, claim, None).await.unwrap();
    let deposit = b"{\"items\":[\"le journal\"],\"patch\":\"diff --git\"}".to_vec();
    let offer_for = |file: String| Offer {
        ticket: "00".repeat(16),
        claim: hex::encode(claim),
        from: hex::encode(alice_device.0),
        to: hex::encode(bob_device.0),
        file,
        branch: "ticket/dem-41".into(),
        remote: "origin".into(),
        patch: true,
        engine: "claude".into(),
        agent: "Agent 1".into(),
        at: 1,
    };
    let file = c[0].1.deposit(p, &deposit).await.unwrap();
    let offer = offer_for(hex::encode(file.0));
    let object = c[0].1.offer_handoff(p, &offer).unwrap();
    c[0].1.sync_project(p).await.unwrap();
    let h = c[0]
        .1
        .hand_claim(p, claim, a.claim.token, bob_device)
        .await
        .unwrap();
    assert_eq!(h.claim.token, a.claim.token + 1, "the token grows");
    assert_eq!(h.claim.holder, Some(bob_device.0.to_vec()));
    assert_eq!(h.claim.handed_by, Some(alice_device.0.to_vec()));
    // A hand to oneself, or under the old token, is refused.
    let again = c[0].1.hand_claim(p, claim, a.claim.token, bob_device).await;
    assert_eq!(refused(again.unwrap_err()).0, "claim_lost");
    // Bob reads the offer and the deposit, which the server keeps encrypted.
    c[1].1.sync_project(p).await.unwrap();
    let offers = c[1].1.handoffs(p).unwrap();
    assert_eq!(offers, vec![(object, offer.clone())]);
    assert_eq!(c[1].1.get_file(file).await.unwrap(), Some(deposit.clone()));
    if matches!(b, Backend::Sqlite) {
        for entry in walk(&w.server.folder) {
            let bytes = std::fs::read(&entry).unwrap_or_default();
            assert!(
                !bytes.windows(10).any(|x| x == b"le journal"),
                "the deposit is never in clear on the server: {}",
                entry.display()
            );
        }
    }
    // Abandoned: Alice takes it back; the deposit and the offer go.
    let back = c[0]
        .1
        .withdraw_claim(p, claim, h.claim.token)
        .await
        .unwrap();
    assert_eq!(back.claim.holder, Some(alice_device.0.to_vec()));
    c[0].1.end_handoff(p, object).unwrap();
    c[0].1.delete_file(file).await.unwrap();
    c[0].1.sync_project(p).await.unwrap();
    c[1].1.sync_project(p).await.unwrap();
    assert!(c[1].1.handoffs(p).unwrap().is_empty());
    assert_eq!(c[1].1.get_file(file).await.unwrap(), None);
    // Offered again: Bob takes it under a new token; Alice no longer takes it back.
    let file = c[0].1.deposit(p, &deposit).await.unwrap();
    let object = c[0]
        .1
        .offer_handoff(p, &offer_for(hex::encode(file.0)))
        .unwrap();
    c[0].1.sync_project(p).await.unwrap();
    let h = c[0]
        .1
        .hand_claim(p, claim, back.claim.token, bob_device)
        .await
        .unwrap();
    let mine = c[1].1.take_claim(p, claim, None).await.unwrap();
    assert_eq!(mine.claim.token, h.claim.token + 1);
    assert_eq!(mine.claim.handed_by, None);
    let late = c[0].1.withdraw_claim(p, claim, h.claim.token).await;
    assert_eq!(refused(late.unwrap_err()).0, "claim_lost");
    c[1].1.sync_project(p).await.unwrap();
    assert_eq!(c[1].1.get_file(file).await.unwrap(), Some(deposit));
    c[1].1.end_handoff(p, object).unwrap();
    c[1].1.delete_file(file).await.unwrap();
    c[1].1.sync_project(p).await.unwrap();
    c[0].1.sync_project(p).await.unwrap();
    assert!(c[0].1.handoffs(p).unwrap().is_empty());
    assert_eq!(c[0].1.get_file(file).await.unwrap(), None);
    // Bob's device says it is a machine in server mode: Alice sees it.
    assert!(c[1].1.set_machine(p, true).unwrap());
    assert!(!c[1].1.set_machine(p, true).unwrap());
    c[1].1.sync_project(p).await.unwrap();
    c[0].1.sync_project(p).await.unwrap();
    assert_eq!(
        c[0].1.machines(p).unwrap().into_iter().collect::<Vec<_>>(),
        vec![bob_device]
    );
    w.finish().await;
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}
