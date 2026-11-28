//! AU-3: the streams of the audit of the stations, against the real server, on SQLite then on PostgreSQL. Every device
//! drops facts in `escouade/audit/f`; only an administrator or a machine of the team reads them, by the stream, the live
//! or a file; only they write the key's stream; no file joins either.

#[macro_use]
mod common;

use std::time::Duration;

use common::*;
use escouade_team_server::api::wire::*;
use escouade_team_server::api::{AUDIT_FACTS_STREAM, AUDIT_KEY_STREAM};
use escouade_team_server::db::Role;
use escouade_team_server::util::random;
use reqwest::{Method, StatusCode};

both!(
    the_facts_are_dropped_by_all_and_read_by_the_emitters_alone,
    the_key_of_the_audit_is_written_by_the_emitters_alone,
);

async fn set_role(admin: &Device, device: &Device, role: Role) {
    let r = admin
        .client
        .request(
            Method::POST,
            &format!("/v1/devices/{}/role", device.id_hex()),
        )
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(
            &RoleRequest { role },
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
}

fn stream(project: &[u8; 16]) -> String {
    format!("/v1/projects/{}/stream?since=0", hex::encode(project))
}

/// Alice (the administrator), Bob and Carol (members), and a machine of the team.
async fn team(s: &TestServer) -> (Device, Device, Device, Device) {
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let carol = s.add_device(&alice).await;
    let machine = s.add_device(&alice).await;
    set_role(&alice, &machine, Role::Service).await;
    (alice, bob, carol, machine)
}

async fn the_facts_are_dropped_by_all_and_read_by_the_emitters_alone(b: Backend) {
    let s = TestServer::start(&b).await;
    let (alice, bob, carol, machine) = team(&s).await;
    let mut alice_live = alice.live(&s).await;
    let mut machine_live = machine.live(&s).await;
    let mut carol_live = carol.live(&s).await;

    // Every device drops there, the members included.
    bob.append_project(&AUDIT_FACTS_STREAM, b"sealed facts of bob")
        .await
        .ok();
    machine
        .append_project(&AUDIT_FACTS_STREAM, b"sealed facts of the machine")
        .await
        .ok();
    // The emitters hear it live; a member does not.
    for live in [&mut alice_live, &mut machine_live] {
        match live.next(Duration::from_secs(2)).await {
            Some(LiveEvent::Frame(f)) => {
                assert_eq!(f.stream, "project");
                assert_eq!(f.project.as_deref(), Some(&AUDIT_FACTS_STREAM[..]));
            }
            _ => panic!("an emitter hears the facts live"),
        }
    }
    assert!(
        carol_live.next(Duration::from_millis(500)).await.is_none(),
        "a member hears nothing of them"
    );

    // The administrator and the machine read the stream; a member, its own facts included, never.
    let read: Entries = alice.client.get(&stream(&AUDIT_FACTS_STREAM)).await.ok();
    assert_eq!(read.entries.len(), 2);
    assert_eq!(read.entries[0].device, bob.id);
    let read: Entries = machine.client.get(&stream(&AUDIT_FACTS_STREAM)).await.ok();
    assert_eq!(read.entries.len(), 2);
    for member in [&bob, &carol] {
        let (status, problem) = member
            .client
            .get::<Entries>(&stream(&AUDIT_FACTS_STREAM))
            .await
            .refused();
        assert_eq!(status, StatusCode::FORBIDDEN, "{problem:?}");
    }
    // A member reads the key's stream (to seal to it).
    let _: Entries = bob.client.get(&stream(&AUDIT_KEY_STREAM)).await.ok();

    // No file joins the streams of the audit, and a member reads none under the facts.
    let file = random::<16>();
    for p in [AUDIT_FACTS_STREAM, AUDIT_KEY_STREAM] {
        let (status, _) = alice
            .file_in(Method::PUT, &p, &file, Some(b"bytes".to_vec()))
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
    let (status, _) = bob
        .file_in(Method::GET, &AUDIT_FACTS_STREAM, &file, None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    s.finish().await;
}

async fn the_key_of_the_audit_is_written_by_the_emitters_alone(b: Backend) {
    let s = TestServer::start(&b).await;
    let (alice, bob, _carol, machine) = team(&s).await;
    alice
        .append_project(&AUDIT_KEY_STREAM, b"a key of the administrator")
        .await
        .ok();
    machine
        .append_project(&AUDIT_KEY_STREAM, b"a key of the machine")
        .await
        .ok();
    let (status, _) = bob
        .append_project(&AUDIT_KEY_STREAM, b"a key of a member")
        .await
        .refused();
    assert_eq!(status, StatusCode::FORBIDDEN);
    let read: Entries = bob.client.get(&stream(&AUDIT_KEY_STREAM)).await.ok();
    assert_eq!(read.entries.len(), 2);
    s.finish().await;
}
