//! The files of the team server (SR2-03, `docs/TEAM-SERVER.md` §2.6): each attached to a project at its putting (or to
//! the team, under one of its own streams), read by the devices that have access to that project, deleted by the
//! device that put it or an administrator alone. A file attached to nothing (put by the old route, or before SR2-03)
//! is its author's and the administrators' until one of them reads it under a project. Each test on SQLite, then on
//! PostgreSQL when `ESCOUADE_TEST_POSTGRES_URL` is set; on the disk, and in S3 when `ESCOUADE_TEST_S3_ENDPOINT` is.

#[macro_use]
mod common;

use common::*;
use escouade_team_server::api::wire::*;
use escouade_team_server::api::PLUGINS_STREAM;
use escouade_team_server::config::FilesConfig;
use escouade_team_server::util::random;
use reqwest::{Method, StatusCode};

both!(
    a_file_follows_the_access_of_its_project_on_the_disk,
    a_file_follows_the_access_of_its_project_in_s3,
    a_file_attached_to_nothing_stays_its_authors_and_the_administrators,
    the_files_put_before_are_attached_to_nothing,
);

/// Keeps `project` to `devices` (it is written once first, so that the server knows it).
async fn keep(admin: &Device, project: &[u8; 16], devices: &[&Device]) {
    admin.append_project(project, b"op").await.ok();
    let r = admin
        .client
        .request(
            Method::PUT,
            &format!("/v1/projects/{}/access", hex::encode(project)),
        )
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(&AccessWire {
            everyone: false,
            devices: devices
                .iter()
                .map(|d| serde_bytes::ByteBuf::from(d.id.clone()))
                .collect(),
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
}

async fn put(d: &Device, p: &[u8; 16], f: &[u8; 16], bytes: &[u8]) -> StatusCode {
    d.file_in(Method::PUT, p, f, Some(bytes.to_vec())).await.0
}

async fn get(d: &Device, p: &[u8; 16], f: &[u8; 16]) -> (StatusCode, Vec<u8>) {
    d.file_in(Method::GET, p, f, None).await
}

async fn delete(d: &Device, p: &[u8; 16], f: &[u8; 16]) -> StatusCode {
    d.file_in(Method::DELETE, p, f, None).await.0
}

async fn a_file_follows_the_access_of_its_project_on_the_disk(b: Backend) {
    let s = TestServer::start_with(&b, |c| assert!(matches!(c.files, FilesConfig::Disk(_)))).await;
    a_file_follows_the_access_of_its_project(s).await;
}

async fn a_file_follows_the_access_of_its_project_in_s3(b: Backend) {
    let Some(s3) = s3() else {
        eprintln!("ESCOUADE_TEST_S3_ENDPOINT is not set: the S3 variant is skipped");
        return;
    };
    let s = TestServer::start_with(&b, |c| c.files = s3).await;
    a_file_follows_the_access_of_its_project(s).await;
}

async fn a_file_follows_the_access_of_its_project(s: TestServer) {
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    let carol = s.add_device(&admin).await;
    let (kept, open, other) = (random::<16>(), random::<16>(), random::<16>());
    keep(&admin, &kept, &[&admin, &bob]).await;
    let (f, g, h) = (random::<16>(), random::<16>(), random::<16>());
    let bytes: Vec<u8> = (0..1000).map(|i| (i * 7) as u8).collect();

    // A device outside the project puts nothing there, reads nothing there, deletes nothing there.
    assert_eq!(put(&carol, &kept, &g, b"x").await, StatusCode::FORBIDDEN);
    assert_eq!(put(&bob, &kept, &f, &bytes).await, StatusCode::CREATED);
    assert_eq!(get(&carol, &kept, &f).await.0, StatusCode::FORBIDDEN);
    assert_eq!(carol.get_file(&f).await.0, StatusCode::FORBIDDEN);
    assert_eq!(delete(&carol, &kept, &f).await, StatusCode::FORBIDDEN);
    assert_eq!(carol.delete_file(&f).await, StatusCode::FORBIDDEN);
    // Nor under a project it may read: the file is not found there.
    assert_eq!(get(&carol, &open, &f).await.0, StatusCode::NOT_FOUND);
    assert_eq!(delete(&carol, &open, &f).await, StatusCode::NOT_FOUND);
    assert_eq!(get(&bob, &other, &f).await.0, StatusCode::NOT_FOUND);
    // The devices of the project read it, by the route of its project or the old one.
    assert_eq!(get(&bob, &kept, &f).await, (StatusCode::OK, bytes.clone()));
    assert_eq!(
        get(&admin, &kept, &f).await,
        (StatusCode::OK, bytes.clone())
    );
    assert_eq!(bob.get_file(&f).await, (StatusCode::OK, bytes.clone()));
    // A file is written once.
    assert_eq!(put(&admin, &kept, &f, b"y").await, StatusCode::CONFLICT);

    // In a project open to the team, every device reads; only the author or an administrator deletes.
    assert_eq!(
        put(&carol, &open, &g, b"carol's").await,
        StatusCode::CREATED
    );
    assert_eq!(
        get(&bob, &open, &g).await,
        (StatusCode::OK, b"carol's".to_vec())
    );
    assert_eq!(delete(&bob, &open, &g).await, StatusCode::FORBIDDEN);
    assert_eq!(bob.delete_file(&g).await, StatusCode::FORBIDDEN);
    assert_eq!(get(&admin, &open, &g).await.0, StatusCode::OK);
    assert_eq!(delete(&carol, &open, &g).await, StatusCode::NO_CONTENT);
    assert_eq!(get(&carol, &open, &g).await.0, StatusCode::NOT_FOUND);
    assert_eq!(put(&bob, &open, &h, b"bob's").await, StatusCode::CREATED);
    assert_eq!(delete(&admin, &open, &h).await, StatusCode::NO_CONTENT);
    assert_eq!(get(&bob, &open, &h).await.0, StatusCode::NOT_FOUND);
    // An administrator deletes the file of a project it is not kept out of, its author too.
    assert_eq!(delete(&admin, &kept, &f).await, StatusCode::NO_CONTENT);
    assert_eq!(get(&bob, &kept, &f).await.0, StatusCode::NOT_FOUND);
    let f2 = random::<16>();
    assert_eq!(put(&bob, &kept, &f2, b"again").await, StatusCode::CREATED);
    assert_eq!(bob.delete_file(&f2).await, StatusCode::NO_CONTENT);

    // A file of the team (its private store) is read by every device of the team.
    let p = random::<16>();
    assert_eq!(
        put(&carol, &PLUGINS_STREAM, &p, b"archive").await,
        StatusCode::CREATED
    );
    assert_eq!(get(&bob, &PLUGINS_STREAM, &p).await.0, StatusCode::OK);
    assert_eq!(
        delete(&bob, &PLUGINS_STREAM, &p).await,
        StatusCode::FORBIDDEN
    );

    // The place of what was deleted came back.
    let usage: Usage = admin.client.get("/v1/files/usage").await.ok();
    assert_eq!(usage.used, 7 + 2);
    s.finish().await;
}

async fn a_file_attached_to_nothing_stays_its_authors_and_the_administrators(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    let carol = s.add_device(&admin).await;
    let (kept, open) = (random::<16>(), random::<16>());
    keep(&admin, &kept, &[&admin, &bob]).await;
    let (l, m) = (random::<16>(), random::<16>());
    // Put by the old route: attached to nothing.
    assert_eq!(bob.put_file(&l, b"old".to_vec()).await, StatusCode::CREATED);
    assert_eq!(carol.get_file(&l).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&carol, &open, &l).await.0, StatusCode::NOT_FOUND);
    assert_eq!(carol.delete_file(&l).await, StatusCode::NOT_FOUND);
    assert_eq!(delete(&carol, &open, &l).await, StatusCode::NOT_FOUND);
    // An administrator reads it by the old route without attaching it.
    assert_eq!(admin.get_file(&l).await, (StatusCode::OK, b"old".to_vec()));
    assert_eq!(get(&carol, &open, &l).await.0, StatusCode::NOT_FOUND);
    // Its author reads it under its project, which attaches it there: the project's devices alone read it then.
    assert_eq!(
        get(&bob, &kept, &l).await,
        (StatusCode::OK, b"old".to_vec())
    );
    assert_eq!(get(&admin, &kept, &l).await.0, StatusCode::OK);
    assert_eq!(get(&carol, &kept, &l).await.0, StatusCode::FORBIDDEN);
    assert_eq!(get(&carol, &open, &l).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&bob, &open, &l).await.0, StatusCode::NOT_FOUND);
    // A device outside the project cannot attach a file to it, nor a member another's file.
    assert_eq!(
        carol.put_file(&m, b"carol's".to_vec()).await,
        StatusCode::CREATED
    );
    assert_eq!(get(&bob, &kept, &m).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&bob, &open, &m).await.0, StatusCode::NOT_FOUND);
    assert_eq!(get(&carol, &kept, &m).await.0, StatusCode::FORBIDDEN);
    // An administrator attaches it to a project open to the team: everyone reads it then.
    assert_eq!(get(&admin, &open, &m).await.0, StatusCode::OK);
    assert_eq!(
        get(&bob, &open, &m).await,
        (StatusCode::OK, b"carol's".to_vec())
    );
    // Its author deletes it, a member does not.
    assert_eq!(bob.delete_file(&m).await, StatusCode::FORBIDDEN);
    assert_eq!(carol.delete_file(&m).await, StatusCode::NO_CONTENT);
    s.finish().await;
}

/// The files of a base made before SR2-03 are attached to nothing at its migration: no device reads more of them
/// than before, the others than their author and the administrators read less, until one of them attaches each.
async fn the_files_put_before_are_attached_to_nothing(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    let carol = s.add_device(&admin).await;
    let (open, f) = (random::<16>(), random::<16>());
    assert_eq!(put(&bob, &open, &f, b"before").await, StatusCode::CREATED);
    assert_eq!(get(&carol, &open, &f).await.0, StatusCode::OK);
    // The base as it was before: no project for a file.
    let drop = "ALTER TABLE files DROP COLUMN project_id";
    match s.schema() {
        None => {
            let db = rusqlite::Connection::open(s.dir.path().join("team.db")).unwrap();
            db.execute_batch(drop).unwrap();
        }
        Some(schema) => {
            let url = std::env::var("ESCOUADE_TEST_POSTGRES_URL").unwrap();
            let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
                .await
                .unwrap();
            tokio::spawn(connection);
            client
                .batch_execute(&format!("SET search_path TO {schema}; {drop}"))
                .await
                .unwrap();
        }
    }
    let store = &s.server.app().store;
    store.migrate().await.unwrap();
    let stored = store.file(id(&admin.team), f).await.unwrap().unwrap();
    assert_eq!((stored.project, &stored.device[..]), (None, &bob.id[..]));
    // Out of sight of the others, open to its author and the administrators.
    assert_eq!(get(&carol, &open, &f).await.0, StatusCode::NOT_FOUND);
    assert_eq!(carol.get_file(&f).await.0, StatusCode::NOT_FOUND);
    assert_eq!(carol.delete_file(&f).await, StatusCode::NOT_FOUND);
    assert_eq!(
        admin.get_file(&f).await,
        (StatusCode::OK, b"before".to_vec())
    );
    // Its author attaches it again by reading it under its project.
    assert_eq!(get(&bob, &open, &f).await.0, StatusCode::OK);
    assert_eq!(
        get(&carol, &open, &f).await,
        (StatusCode::OK, b"before".to_vec())
    );
    s.finish().await;
}

fn id(v: &[u8]) -> [u8; 16] {
    v.try_into().unwrap()
}
