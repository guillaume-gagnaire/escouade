//! A project's snapshots on the server (SR2-01, `docs/TEAM-SERVER.md` §2.4): none on the team's own streams; the
//! messages a snapshot holds kept for the retention, then forgotten only when no device flagged it unreadable; the
//! administrators told of a flagged one, and able to remove it. Each test on SQLite, then on PostgreSQL when
//! `ESCOUADE_TEST_POSTGRES_URL` is set.

#[macro_use]
mod common;

use common::*;
use escouade_team_server::api::wire::*;
use escouade_team_server::api::TEAM_STREAMS;
use escouade_team_server::db::Kind;
use escouade_team_server::util::{now, random};
use reqwest::{Method, StatusCode};

both!(
    the_team_streams_take_no_snapshot,
    a_snapshot_keeps_its_messages_for_the_retention,
    a_flagged_snapshot_keeps_its_messages_and_the_administrators_are_told,
    an_administrator_removes_a_snapshot,
    the_snapshot_before_another_is_served,
);

fn path(p: &[u8; 16]) -> String {
    format!("/v1/projects/{}/snapshot", hex::encode(p))
}

async fn put(d: &Device, p: &[u8; 16], upto: u64, payload: Vec<u8>) -> Answer<SnapshotKept> {
    d.client.put(&path(p), &SnapshotPut { upto, payload }).await
}

async fn flag(d: &Device, p: &[u8; 16], upto: u64) -> StatusCode {
    d.client
        .request(Method::POST, &format!("{}/unreadable", path(p)))
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(
            &SnapshotFlag { upto },
        ))
        .send()
        .await
        .unwrap()
        .status()
}

async fn remove(d: &Device, p: &[u8; 16], upto: u64) -> StatusCode {
    d.client
        .status(Method::DELETE, &format!("{}?upto={upto}", path(p)), None)
        .await
        .0
}

async fn used(d: &Device) -> u64 {
    let u: Usage = d.client.get("/v1/files/usage").await.ok();
    u.used
}

fn seqs(e: &Entries) -> Vec<u64> {
    e.entries.iter().map(|e| e.seq).collect()
}

/// The retention, played at once: what the sweep does when `days` days passed.
async fn prune_after(s: &TestServer, days: i64) -> u64 {
    let retention = s.server.app().limits.snapshot_retention.as_secs() as i64;
    s.server
        .app()
        .store
        .prune_snapshots(now() + days * 24 * 3600 - retention)
        .await
        .unwrap()
}

/// No device, administrator or member, puts a snapshot on the team's own streams: their entries stay as their writers
/// signed them.
async fn the_team_streams_take_no_snapshot(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let member = s.add_device(&admin).await;
    for stream in TEAM_STREAMS {
        for i in 0..3u8 {
            admin.append_project(&stream, &[i; 64]).await.ok();
        }
        for d in [&member, &admin] {
            let (code, p) = put(d, &stream, 3, vec![1; 32]).await.refused();
            assert_eq!(code, StatusCode::FORBIDDEN);
            assert!(p.message.contains("no snapshot"), "{}", p.message);
        }
        let read = admin.read_project(&stream, 0).await;
        assert_eq!(seqs(&read), [1, 2, 3]);
        assert_eq!((read.snapshot, read.latest), (None, None));
    }
    s.finish().await;
}

/// A snapshot forgets nothing when it comes: the messages it holds stay readable for the retention (30 days), its bytes
/// count; past it, they go (the commits stay) and their bytes come back. Until then, a device that finds the snapshot
/// unreadable reads them instead.
async fn a_snapshot_keeps_its_messages_for_the_retention(b: Backend) {
    let s = TestServer::start(&b).await;
    assert_eq!(
        s.server.app().limits.snapshot_retention.as_secs(),
        30 * 24 * 3600
    );
    let admin = s.new_team().await;
    let member = s.add_device(&admin).await;
    let p = random::<16>();
    for i in 1..=3u8 {
        admin.append_project(&p, &[i; 1000]).await.ok();
    }
    let commit: Answer<Appended> = admin
        .client
        .post(
            &format!("/v1/projects/{}/stream", hex::encode(p)),
            &AppendRequest {
                fence: None,
                kind: Kind::Commit,
                epoch: Some(0),
                payload: vec![9; 10],
            },
        )
        .await;
    assert_eq!(commit.ok().seq, 4);
    for i in 5..=6u8 {
        admin.append_project(&p, &[i; 1000]).await.ok();
    }
    let before = used(&admin).await;
    assert_eq!(put(&member, &p, 5, vec![7; 100]).await.ok().pruned, 0);
    assert_eq!(used(&admin).await, before + 100);
    assert_eq!(seqs(&admin.read_project(&p, 0).await), [1, 2, 3, 4, 5, 6]);
    // A day after, then 29: still there.
    assert_eq!(prune_after(&s, 1).await, 0);
    assert_eq!(prune_after(&s, 29).await, 0);
    assert_eq!(seqs(&admin.read_project(&p, 0).await), [1, 2, 3, 4, 5, 6]);
    // Past the retention: the messages it holds go, its commit stays, the snapshot takes their place.
    assert_eq!(prune_after(&s, 31).await, 4);
    let read = admin.read_project(&p, 0).await;
    assert_eq!(seqs(&read), [4, 6]);
    assert_eq!(read.snapshot, Some(5));
    assert_eq!(used(&admin).await, before + 100 - 4000);
    // Once only.
    assert_eq!(prune_after(&s, 62).await, 0);
    s.finish().await;
}

/// A device that finds a snapshot unreadable flags it: its messages then stay past the retention, and the team's
/// administrators (not its members) read which snapshot, posted by which device, and who found it.
async fn a_flagged_snapshot_keeps_its_messages_and_the_administrators_are_told(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let poster = s.add_device(&admin).await;
    let reader = s.add_device(&admin).await;
    let p = random::<16>();
    for i in 1..=4u8 {
        admin.append_project(&p, &[i; 100]).await.ok();
    }
    put(&poster, &p, 4, b"not a sealed snapshot".to_vec())
        .await
        .ok();
    assert_eq!(flag(&reader, &p, 3).await, StatusCode::NOT_FOUND);
    assert_eq!(flag(&reader, &p, 4).await, StatusCode::NO_CONTENT);
    // A second flag keeps the first device and time.
    assert_eq!(flag(&admin, &p, 4).await, StatusCode::NO_CONTENT);
    let list: UnreadableSnapshots = admin.client.get("/v1/snapshots/unreadable").await.ok();
    assert_eq!(list.snapshots.len(), 1);
    let u = &list.snapshots[0];
    assert_eq!(
        (&u.project[..], u.upto, &u.device, &u.flagged_by),
        (&p[..], 4, &poster.id, &reader.id)
    );
    let refused: Answer<UnreadableSnapshots> = reader.client.get("/v1/snapshots/unreadable").await;
    assert_eq!(refused.refused().0, StatusCode::FORBIDDEN);
    // Past the retention, flagged: nothing goes.
    assert_eq!(prune_after(&s, 365).await, 0);
    assert_eq!(seqs(&admin.read_project(&p, 0).await), [1, 2, 3, 4]);
    // A device outside a project kept to others flags none of its snapshots.
    let kept = random::<16>();
    admin.append_project(&kept, b"op").await.ok();
    let r = admin
        .client
        .request(
            Method::PUT,
            &format!("/v1/projects/{}/access", hex::encode(kept)),
        )
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(&AccessWire {
            everyone: false,
            devices: vec![serde_bytes::ByteBuf::from(admin.id.clone())],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    put(&admin, &kept, 1, vec![1; 8]).await.ok();
    assert_eq!(flag(&reader, &kept, 1).await, StatusCode::FORBIDDEN);
    s.finish().await;
}

/// An administrator removes a snapshot (a member cannot): the one before takes its place, its bytes come back. The one
/// the stream was pruned to stays: its messages are gone, nothing would take its place.
async fn an_administrator_removes_a_snapshot(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let member = s.add_device(&admin).await;
    let p = random::<16>();
    for i in 1..=4u8 {
        admin.append_project(&p, &[i; 100]).await.ok();
    }
    put(&admin, &p, 2, vec![2; 50]).await.ok();
    assert_eq!(prune_after(&s, 31).await, 2);
    put(&member, &p, 4, vec![4; 50]).await.ok();
    flag(&admin, &p, 4).await;
    let before = used(&admin).await;
    assert_eq!(remove(&member, &p, 4).await, StatusCode::FORBIDDEN);
    assert_eq!(remove(&admin, &p, 3).await, StatusCode::NOT_FOUND);
    assert_eq!(remove(&admin, &p, 4).await, StatusCode::NO_CONTENT);
    assert_eq!(used(&admin).await, before - 50);
    let list: UnreadableSnapshots = admin.client.get("/v1/snapshots/unreadable").await.ok();
    assert!(list.snapshots.is_empty());
    let read = admin.read_project(&p, 0).await;
    assert_eq!(seqs(&read), [3, 4]);
    assert_eq!((read.snapshot, read.latest), (Some(2), Some(2)));
    let got: SnapshotInfo = member.client.get(&path(&p)).await.ok();
    assert_eq!(got.upto, 2);
    // The base of the stream: refused.
    let (code, body) = admin
        .client
        .status(Method::DELETE, &format!("{}?upto=2", path(&p)), None)
        .await;
    assert_eq!(
        code,
        StatusCode::CONFLICT,
        "{}",
        String::from_utf8_lossy(&body)
    );
    // On the team's own streams too, an administrator removes what a server before SR2-01 let a member put: none here.
    assert_eq!(
        remove(&admin, &TEAM_STREAMS[0], 1).await,
        StatusCode::NOT_FOUND
    );
    s.finish().await;
}

/// The last snapshot, and the one before a number (a device that found the last unreadable goes back to it). They all
/// stay while the retention runs; past it, the snapshots before the one the stream is pruned to go.
async fn the_snapshot_before_another_is_served(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let p = random::<16>();
    for i in 1..=6u8 {
        admin.append_project(&p, &[i; 10]).await.ok();
    }
    for upto in [2, 4, 6] {
        put(&admin, &p, upto, vec![upto as u8; 8]).await.ok();
    }
    let at = |q: &'static str| {
        let c = admin.client.clone();
        let path = format!("{}{q}", path(&p));
        async move {
            let a: Answer<SnapshotInfo> = c.get(&path).await;
            match a {
                Answer::Ok(i) => Some((i.upto, i.payload[0])),
                a => {
                    assert_eq!(a.refused().0, StatusCode::NOT_FOUND);
                    None
                }
            }
        }
    };
    assert_eq!(at("").await, Some((6, 6)));
    assert_eq!(at("?before=6").await, Some((4, 4)));
    assert_eq!(at("?before=4").await, Some((2, 2)));
    assert_eq!(at("?before=2").await, None);
    // Not after the last one: refused.
    assert_eq!(
        put(&admin, &p, 5, vec![1]).await.refused().1.error,
        "snapshot_refused"
    );
    assert_eq!(prune_after(&s, 31).await, 6);
    assert_eq!(at("").await, Some((6, 6)));
    assert_eq!(at("?before=6").await, None);
    s.finish().await;
}

/// PostgreSQL: a base from before the retention (one snapshot per project, its messages already forgotten) keeps its
/// snapshots, and the stream says it is pruned to them. Skipped without `ESCOUADE_TEST_POSTGRES_URL`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_postgres_base_from_before_the_retention_keeps_its_snapshots() {
    use escouade_team_server::db::postgres::Postgres;
    use escouade_team_server::db::{Store, StoreError};
    let Some(Backend::Postgres(url)) = Backend::postgres() else {
        return;
    };
    let schema = format!("t_{}", hex::encode(random::<8>()));
    let store = Postgres::connect(&url, Some(&schema)).await.unwrap();
    store.migrate().await.unwrap();
    let (raw, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .unwrap();
    tokio::spawn(connection);
    let (team, project) = ([1u8; 16], [2u8; 16]);
    raw.batch_execute(&format!(
        "SET search_path TO {schema};
         CREATE TABLE project_snapshots (
            team_id BYTEA NOT NULL, project_id BYTEA NOT NULL, upto BIGINT NOT NULL,
            device_id BYTEA NOT NULL, at BIGINT NOT NULL, payload BYTEA NOT NULL,
            PRIMARY KEY (team_id, project_id));"
    ))
    .await
    .unwrap();
    raw.execute(
        "INSERT INTO teams (id, created_at, stored) VALUES ($1, 0, 3)",
        &[&team.as_slice()],
    )
    .await
    .unwrap();
    raw.execute(
        "INSERT INTO projects (team_id, id, seq) VALUES ($1, $2, 9)",
        &[&team.as_slice(), &project.as_slice()],
    )
    .await
    .unwrap();
    raw.execute(
        "INSERT INTO project_snapshots VALUES ($1, $2, 7, $3, 5, $4)",
        &[
            &team.as_slice(),
            &project.as_slice(),
            &[3u8; 16].as_slice(),
            &vec![1u8, 2, 3],
        ],
    )
    .await
    .unwrap();
    store.migrate().await.unwrap();
    let s = store.snapshot(team, project, None).await.unwrap().unwrap();
    assert_eq!(
        (s.upto, s.device, s.at, s.payload),
        (7, [3u8; 16], 5, vec![1, 2, 3])
    );
    assert!(matches!(
        store.remove_snapshot(team, project, 7).await,
        Err(StoreError::Snapshot(_))
    ));
    let old: bool = raw
        .query_one("SELECT to_regclass('project_snapshots') IS NOT NULL", &[])
        .await
        .unwrap()
        .get(0);
    assert!(!old);
    store.migrate().await.unwrap();
    store.drop_schema(&schema).await.unwrap();
}
