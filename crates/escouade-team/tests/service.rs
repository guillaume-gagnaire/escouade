//! OE-1 against the real `escouade-team-server`, once on SQLite and once on PostgreSQL (`ESCOUADE_TEST_POSTGRES_URL`):
//! a machine of the team enrolled by an administrator's short code under the role of service (tied to the team, neither
//! a member nor an administrator); the server that keeps it off its administrators' routes and off the team's own
//! streams; every device that refuses what it signs out of its role even when a lying server plays it to them (a
//! setting, a circuit breaker, a private plugin, a proposal); the content of an opened project read and written, a
//! ticket taken; the machine revoked as a device, and the keys turned after it.

mod common;

use std::collections::BTreeMap;
use std::time::Duration;

use common::*;
use escouade_team::breaker::{self, BREAKER_STREAM};
use escouade_team::catalog::CATALOG_STREAM;
use escouade_team::crypto::ops::Value;
use escouade_team::crypto::{DeviceId, MemberId, ObjectId, Role};
use escouade_team::pairing::{self, PairCode};
use escouade_team::plugins::{self, PLUGINS_STREAM};
use escouade_team::proposals::PROPOSAL_STREAM;
use escouade_team::settings::{
    self, digest, Classify, Entry, Refused, Sort, Status, TeamFile, Version, SETTINGS_STREAM,
};
use escouade_team::{wire, CryptoError, Error, PairState, Team};
use serde_json::json;

both!(
    a_machine_enrolls_by_a_code_under_the_role_of_service,
    the_server_keeps_a_machine_off_the_administrators_routes,
    every_device_refuses_what_a_machine_signs_out_of_its_role,
    a_machine_reads_and_writes_an_opened_project_and_takes_a_ticket,
    a_machine_is_revoked_as_a_device_and_the_keys_turn,
);

/// The keys of the settings the tests write.
struct Keys;

impl Classify for Keys {
    fn sort(&self, key: &str) -> Option<Sort> {
        match key {
            "telemetry.enabled" => Some(Sort::Restrict),
            _ => None,
        }
    }
}

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

/// The whole enrolment of a machine of the team: the administrator's short code, typed on the machine
/// (`escouade-server team pair`), its request, its six words compared, the admission, the server's role in step.
async fn enroll(alice: &mut Team, machine_place: &Place) -> Team {
    let mut pairing = alice.pair_start_service(NOW).await.unwrap();
    let code = PairCode::parse(&pairing.code.shown()).unwrap();
    let transport = machine_place.liar.clone();
    let typed = tokio::spawn(async move {
        pairing::fetch_link(&*transport, &code, Duration::from_secs(20)).await
    });
    assert_eq!(step_until_done(alice, &mut pairing).await, PairState::Sent);
    let link = typed.await.unwrap().unwrap();
    assert_eq!(
        link.member, None,
        "a new member of the team, not a device of Alice"
    );
    let mut machine = Team::join_link(machine_place.setup(), &link).await.unwrap();
    alice.sync_at(NOW).await.unwrap();
    let shown = alice.admissions(NOW);
    assert_eq!(shown.len(), 1);
    let words = machine
        .join_words()
        .expect("the machine shows its six words");
    assert_eq!(shown[0].code.words(), words);
    alice
        .admit_invitation(shown[0].invitation, &words.join(" "), NOW)
        .await
        .unwrap();
    assert!(machine.sync_at(NOW).await.unwrap().joined);
    alice.serve(NOW).await.unwrap();
    machine.sync().await.unwrap();
    machine
}

/// Alice (the owner, an administrator), Bob (a member) and the machine of the team.
async fn three(w: &World) -> (Place, Team, Place, Team, Place, Team) {
    let (pa, pb, pm) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let machine = enroll(&mut alice, &pm).await;
    bob.sync().await.unwrap();
    (pa, alice, pb, bob, pm, machine)
}

fn device(t: &Team) -> [u8; 16] {
    t.client().device()
}

fn role_on_server(devices: &[wire::DeviceInfo], t: &Team) -> wire::Role {
    devices
        .iter()
        .find(|d| d.id == device(t))
        .expect("the device is on the server")
        .role
}

/// Its own id, tied to the team: the roster has it under the role of service, the server as « service ».
async fn a_machine_enrolls_by_a_code_under_the_role_of_service(b: Backend) {
    let w = World::on(&b).await;
    let (_pa, mut alice, _pb, mut bob, _pm, machine) = three(&w).await;
    let me: MemberId = machine.identity().member;
    assert!(me != member(1) && me != member(2), "the machine's own id");
    assert!(machine.is_service());
    assert!(!machine.is_admin().unwrap());
    assert!(!bob.is_service() && !alice.is_service());
    let members = alice.members().unwrap();
    let row = members.iter().find(|(m, _, _)| *m == me).unwrap();
    assert_eq!(row.1, Role::Service);
    assert_eq!(row.2, vec![DeviceId(device(&machine))]);
    assert_eq!(alice.admins().unwrap(), 1, "a machine is no administrator");
    assert!(bob.is_machine(&me));
    let devices = alice.client().devices().await.unwrap();
    assert_eq!(role_on_server(&devices, &machine), wire::Role::Service);
    assert_eq!(role_on_server(&devices, &bob), wire::Role::Member);

    // A member enrolls no machine; nor does the machine enroll another.
    assert!(matches!(
        bob.pair_start_service(NOW).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    let mut machine = machine;
    assert!(matches!(
        machine.pair_start_service(NOW).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    // Nor is the role given or taken afterwards: a machine stays one, a person stays one.
    assert!(alice.set_role(me, Role::Admin).await.is_err());
    assert!(alice.set_role(member(2), Role::Service).await.is_err());
    assert_eq!(alice.roster().unwrap().role(&me), Some(Role::Service));
    w.finish().await;
}

/// The server, which knows the device's sort alone, refuses the machine on each route of an administrator and on the
/// team's own streams; nor does an administrator make it something else on the server.
async fn the_server_keeps_a_machine_off_the_administrators_routes(b: Backend) {
    let w = World::on(&b).await;
    let (_pa, alice, _pb, bob, _pm, machine) = three(&w).await;
    let client = machine.client();
    let forbidden = |r: Result<(), escouade_team::client::ClientError>| {
        assert_eq!(r.expect_err("refused").status(), Some(403));
    };
    forbidden(client.register_token([7; 32], 60).await.map(|_| ()));
    forbidden(client.set_role(&device(&bob), wire::Role::Admin).await);
    forbidden(client.set_role(&device(&machine), wire::Role::Admin).await);
    forbidden(client.revoke(&device(&bob)).await.map(|_| ()));
    forbidden(client.token_alerts().await.map(|_| ()));
    let payload = b"sealed, the server reads nothing of it".to_vec();
    for stream in [
        SETTINGS_STREAM,
        CATALOG_STREAM,
        BREAKER_STREAM,
        PLUGINS_STREAM,
        PROPOSAL_STREAM,
    ] {
        forbidden(client.append_project(&stream.0, &payload).await.map(|_| ()));
    }
    // A member writes in the proposals; the machine does not.
    bob.client()
        .append_project(&PROPOSAL_STREAM.0, &payload)
        .await
        .expect("a member proposes");
    // Its role on the server stays « service »: an administrator's client does not change it.
    let refused = alice
        .client()
        .set_role(&device(&machine), wire::Role::Admin)
        .await
        .expect_err("a machine stays one");
    assert_eq!(refused.code(), Some("service_stays"));
    let devices = alice.client().devices().await.unwrap();
    assert_eq!(role_on_server(&devices, &machine), wire::Role::Service);
    w.finish().await;
}

fn lies(p: &Place, stream: [u8; 16]) -> std::sync::MutexGuard<'_, Lies> {
    let mut l = p.liar.lies.lock().unwrap();
    l.project = Some(hex::encode(stream));
    l
}

async fn last_seq(t: &Team, stream: [u8; 16]) -> u64 {
    t.client()
        .read_project(&stream, 0, None)
        .await
        .unwrap()
        .entries
        .last()
        .map_or(0, |e| e.seq)
}

/// Past its client's checks, sealed by the machine's leaf, and played to Bob by a lying server: a version of the team's
/// settings, a circuit breaker, a plugin's dismissal and the team's key of the store. Bob refuses each.
async fn every_device_refuses_what_a_machine_signs_out_of_its_role(b: Backend) {
    let w = World::on(&b).await;
    let (_pa, mut alice, pb, mut bob, _pm, mut machine) = three(&w).await;
    let file = TeamFile {
        version: 1,
        enforced: [("telemetry.enabled".to_string(), json!(false))]
            .into_iter()
            .collect(),
        ..Default::default()
    };
    // Its client refuses each before it leaves.
    assert!(matches!(
        settings::propose(&mut machine, &Keys, file.clone(), "Machine", "", NOW).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    assert!(matches!(
        breaker::set(
            &mut machine,
            breaker::Scope::Team,
            breaker::What::Agents,
            "r",
            "Machine",
            NOW
        )
        .await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    assert!(matches!(
        plugins::dismiss(&mut machine, &"00".repeat(16), "no", NOW).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));

    // The settings: a version sealed by the machine.
    let version = Version {
        number: 1,
        previous: None,
        file: serde_json::to_string(&file).unwrap(),
        by: "Machine".into(),
        at: NOW,
        note: String::new(),
    };
    let body = wire::encode(&version);
    let d = digest(&body);
    let mut object = [0u8; 16];
    object.copy_from_slice(&d[..16]);
    let sealed = machine
        .device()
        .seal(
            SETTINGS_STREAM,
            ObjectId(object),
            1,
            &wire::encode(&Entry::Version { body }),
        )
        .unwrap();
    let seq = last_seq(&alice, SETTINGS_STREAM.0).await;
    lies(&pb, SETTINGS_STREAM.0).project_inject = vec![(seq + 1, wire::encode(&sealed))];
    let view = settings::sync(&mut bob, &Keys).await.unwrap();
    assert_eq!(view.applied, None, "{view:#?}");
    assert_eq!(
        view.versions[0].status,
        Status::Refused {
            reason: Refused::NotAdmin
        }
    );
    lies(&pb, SETTINGS_STREAM.0).project_inject.clear();

    // A circuit breaker.
    let cut = breaker::Entry::Set {
        id: [3; 16],
        serial: 1,
        scope: breaker::Scope::Team,
        what: breaker::What::Agents,
        reason: "the machine cuts".into(),
        by: "Machine".into(),
        at: NOW,
    };
    let payload = breaker::seal(&machine, &cut).unwrap();
    let seq = last_seq(&alice, BREAKER_STREAM.0).await;
    lies(&pb, BREAKER_STREAM.0).project_inject = vec![(seq + 1, payload)];
    let seen = breaker::sync(&mut bob).await.unwrap();
    assert!(seen.active.is_empty());
    assert!(seen
        .refusals
        .iter()
        .any(|r| r.seq == seq + 1 && r.reason == breaker::Refused::NotAdmin));
    lies(&pb, BREAKER_STREAM.0).project_inject.clear();

    // The private store: the team's key, and a dismissal (a member's own act), both refused as a machine's.
    let key = plugins::seal(
        &machine,
        &plugins::Entry::Key {
            keys: vec!["RWQ".into()],
            namespace: "fr.acme.*".into(),
            at: NOW,
        },
    )
    .unwrap();
    let dismiss = plugins::seal(
        &machine,
        &plugins::Entry::Dismiss {
            submission: [4; 16],
            why: "no".into(),
            at: NOW,
        },
    )
    .unwrap();
    let seq = last_seq(&alice, PLUGINS_STREAM.0).await;
    lies(&pb, PLUGINS_STREAM.0).project_inject = vec![(seq + 1, key), (seq + 2, dismiss)];
    let store = plugins::sync(&mut bob).await.unwrap();
    assert!(store.key.is_none());
    let reasons: Vec<_> = store.refusals.iter().map(|r| r.reason.clone()).collect();
    assert_eq!(
        reasons,
        vec![plugins::Refused::Machine, plugins::Refused::Machine]
    );
    lies(&pb, PLUGINS_STREAM.0).project_inject.clear();

    // An administrator's version still applies.
    let view = settings::propose(&mut alice, &Keys, file, "Alice", "", NOW)
        .await
        .unwrap();
    assert_eq!(view.applied, Some(1));
    w.finish().await;
}

fn fields(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

/// A project of the whole team: the machine reads Alice's ticket, writes its own and a field, takes a ticket; Alice
/// and Bob read what it wrote, and are told it holds the ticket.
async fn a_machine_reads_and_writes_an_opened_project_and_takes_a_ticket(b: Backend) {
    let w = World::on(&b).await;
    let (_pa, mut alice, _pb, mut bob, _pm, mut machine) = three(&w).await;
    for t in [&mut alice, &mut bob, &mut machine] {
        t.serve(NOW).await.unwrap();
    }
    let p = alice.create_project(None).await.unwrap();
    let ticket = alice
        .create_ticket(p, fields(&[("title", json!("Nightly build"))]))
        .unwrap();
    alice.sync_project(p).await.unwrap();
    for t in [&mut bob, &mut machine] {
        t.follow(p).unwrap();
        t.sync_project(p).await.unwrap();
    }
    machine.set_name(p, "Machine de l’équipe").unwrap();
    let seen = machine
        .replica(&p)
        .unwrap()
        .object(&ticket)
        .unwrap()
        .unwrap();
    assert_eq!(seen.fields.get("title"), Some(&json!("Nightly build")));
    let own = machine
        .create_ticket(p, fields(&[("title", json!("Rapport de nuit"))]))
        .unwrap();
    machine
        .set_field(p, ticket, "column", json!("doing"))
        .unwrap();
    machine.sync_project(p).await.unwrap();
    let claim = [5u8; 16];
    let taken = machine.take_claim(p, claim, None).await.unwrap();
    assert_eq!(
        taken.claim.holder.as_deref(),
        Some(device(&machine).as_slice())
    );

    alice.sync_project(p).await.unwrap();
    let replica = alice.replica(&p).unwrap();
    assert_eq!(
        replica.object(&own).unwrap().unwrap().fields.get("title"),
        Some(&json!("Rapport de nuit"))
    );
    assert_eq!(
        replica
            .object(&ticket)
            .unwrap()
            .unwrap()
            .fields
            .get("column"),
        Some(&json!("doing"))
    );
    let claims = bob.claims(p).await.unwrap();
    assert_eq!(claims.claims.len(), 1);
    let holder = DeviceId(claims.claims[0].holder.clone().unwrap().try_into().unwrap());
    bob.sync_project(p).await.unwrap();
    assert_eq!(
        bob.name_of(p, &holder).as_deref(),
        Some("Machine de l’équipe")
    );
    assert_eq!(bob.member_of(&holder), Some(machine.identity().member));
    assert!(bob.is_machine(&machine.identity().member));
    w.finish().await;
}

/// The machine leaves as a device does: cut on the server first, taken out of the group, the keys turned; it reads
/// nothing written after, takes nothing; Bob still reads and writes.
async fn a_machine_is_revoked_as_a_device_and_the_keys_turn(b: Backend) {
    let w = World::on(&b).await;
    let (_pa, mut alice, _pb, mut bob, _pm, mut machine) = three(&w).await;
    let p = alice.create_project(None).await.unwrap();
    alice.sync_project(p).await.unwrap();
    for t in [&mut bob, &mut machine] {
        t.follow(p).unwrap();
        t.sync_project(p).await.unwrap();
    }
    let before = alice.epoch().unwrap();
    let me = machine.identity().member;
    alice.remove_member(me).await.unwrap();
    alice.rotate().await.unwrap();
    assert_eq!(alice.epoch().unwrap(), before + 2);
    assert!(alice.roster().unwrap().role(&me).is_none());
    let devices = alice.client().devices().await.unwrap();
    assert!(
        devices
            .iter()
            .find(|d| d.id == device(&machine))
            .unwrap()
            .revoked
    );
    // Cut: neither a reading nor a claim passes.
    assert!(machine.sync().await.is_err() || machine.is_removed());
    assert!(machine.take_claim(p, [6; 16], None).await.is_err());
    // Bob is at the new epoch, and writes as before; the machine of before is still a machine for what it signed.
    bob.sync().await.unwrap();
    assert_eq!(bob.epoch().unwrap(), alice.epoch().unwrap());
    assert!(bob.is_machine(&me));
    let o = bob
        .create_ticket(p, fields(&[("title", json!("after"))]))
        .unwrap();
    bob.sync_project(p).await.unwrap();
    alice.sync_project(p).await.unwrap();
    assert!(alice.replica(&p).unwrap().object(&o).unwrap().is_some());
    w.finish().await;
}
