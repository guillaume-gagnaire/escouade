//! The commit that wins an epoch but that no device takes (SR2-02): the server keeps it without reading it, an
//! administrator's device declares it null by a void, and the epoch is free again, without a fork: the commit stays
//! served as it was, the void after it, then the next commit of the same epoch, kept to the administrators until
//! theirs. In defense in depth, a member's commit says it changes their own devices only, and only an administrator
//! declares a commit null. On SQLite, and on PostgreSQL when `ESCOUADE_TEST_POSTGRES_URL` is set.

mod common;

use common::{Answer, Backend, Device, TestServer};
use escouade_team_server::api::wire::*;
use escouade_team_server::db::{entry_digest, Kind, StoreError, Stream};
use escouade_team_server::util::Id;
use reqwest::StatusCode;

both!(
    a_commit_declared_null_lets_its_epoch_be_taken_again,
    a_projects_commit_declared_null_lets_its_epoch_be_taken_again,
    a_void_names_the_last_commit_alone,
);

const PROJECT: [u8; 16] = *b"a project kept\0\0";

fn team_of(d: &Device) -> Id {
    d.team.clone().try_into().unwrap()
}

fn path(project: Option<&[u8; 16]>) -> String {
    match project {
        None => "/v1/team/stream".into(),
        Some(p) => format!("/v1/projects/{}/stream", hex::encode(p)),
    }
}

async fn put(
    d: &Device,
    project: Option<&[u8; 16]>,
    kind: Kind,
    epoch: u64,
    own: bool,
    payload: &[u8],
) -> Answer<Appended> {
    d.client
        .post(
            &path(project),
            &AppendRequest {
                own,
                lease: None,
                fence: None,
                kind,
                epoch: Some(epoch),
                payload: payload.to_vec(),
            },
        )
        .await
}

fn said(a: Answer<Appended>) -> (StatusCode, String) {
    let (s, p) = a.refused();
    (s, p.error)
}

async fn read(d: &Device, project: Option<&[u8; 16]>) -> Entries {
    match project {
        None => d.read_team(0).await,
        Some(p) => d.read_project(p, 0).await,
    }
}

/// The whole round, on the team's stream or a project's.
async fn round(b: Backend, project: Option<&[u8; 16]>) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    let team = team_of(&admin);
    let stream = match project {
        None => Stream::Team,
        Some(p) => Stream::Project(*p),
    };
    assert_eq!(
        put(&admin, project, Kind::Commit, 0, false, b"admin: 0 -> 1")
            .await
            .ok()
            .seq,
        1
    );
    // A member's commit that does not say it is of their own devices: refused.
    assert_eq!(
        said(put(&bob, project, Kind::Commit, 1, false, b"bob").await),
        (StatusCode::FORBIDDEN, "forbidden".into())
    );
    // One that says so, of bytes no device reads: kept, the first of its epoch.
    assert_eq!(
        put(&bob, project, Kind::Commit, 1, true, b"garbage")
            .await
            .ok()
            .seq,
        2
    );
    let garbage = read(&admin, project).await.entries.remove(1);
    assert_eq!(
        said(put(&admin, project, Kind::Commit, 1, false, b"admin: 1 -> 2").await),
        (StatusCode::CONFLICT, "epoch_taken".into())
    );
    // A member does not declare it null.
    assert_eq!(
        said(put(&bob, project, Kind::Void, 1, false, b"bob's void").await),
        (StatusCode::FORBIDDEN, "forbidden".into())
    );
    // An administrator does: the epoch is free again, kept to the administrators.
    assert_eq!(
        put(&admin, project, Kind::Void, 1, false, b"void of 2")
            .await
            .ok()
            .seq,
        3
    );
    assert_eq!(
        said(put(&admin, project, Kind::Void, 1, false, b"again").await),
        (StatusCode::CONFLICT, "void_refused".into()),
        "a commit is declared null once"
    );
    assert_eq!(
        said(put(&bob, project, Kind::Commit, 1, true, b"garbage again").await),
        (StatusCode::CONFLICT, "epoch_reserved".into())
    );
    assert_eq!(
        put(&admin, project, Kind::Commit, 1, false, b"admin: 1 -> 2")
            .await
            .ok()
            .seq,
        4
    );
    // The epoch is held again, by the administrator's commit: the winner a loser is shown.
    let (status, p) = put(&admin, project, Kind::Commit, 1, false, b"late")
        .await
        .refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::CONFLICT, "epoch_taken")
    );
    let detail: EpochTaken = p.detail.unwrap().deserialized().unwrap();
    assert_eq!(detail.expected_epoch, 2);
    assert_eq!(detail.preceding.first().map(|e| e.seq), Some(4));
    // The members' own changes go on.
    assert_eq!(
        put(&bob, project, Kind::Commit, 2, true, b"bob: 2 -> 3")
            .await
            .ok()
            .seq,
        5
    );
    // Nothing is rewritten: the null commit is served as before, the void after it.
    let entries = read(&bob, project).await.entries;
    let shape: Vec<(u64, Kind, Option<u64>)> =
        entries.iter().map(|e| (e.seq, e.kind, e.epoch)).collect();
    assert_eq!(
        shape,
        [
            (1, Kind::Commit, Some(0)),
            (2, Kind::Commit, Some(1)),
            (3, Kind::Void, Some(1)),
            (4, Kind::Commit, Some(1)),
            (5, Kind::Commit, Some(2)),
        ]
    );
    assert_eq!(entry_digest(&entries[1]), entry_digest(&garbage));
    // The operator removes neither the void nor the commit it declared null: the devices read past them already.
    let store = &s.server.app().store;
    for seq in [2, 3] {
        assert!(matches!(
            store
                .drop_entry(team, stream, seq, "operator".into(), 1_000)
                .await,
            Err(StoreError::Drop(_))
        ));
    }
    s.finish().await;
}

async fn a_commit_declared_null_lets_its_epoch_be_taken_again(b: Backend) {
    round(b, None).await;
}

async fn a_projects_commit_declared_null_lets_its_epoch_be_taken_again(b: Backend) {
    round(b, Some(&PROJECT)).await;
}

async fn a_void_names_the_last_commit_alone(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    // No commit yet: nothing to declare null.
    assert_eq!(
        said(put(&admin, None, Kind::Void, 0, false, b"void").await),
        (StatusCode::CONFLICT, "void_refused".into())
    );
    put(&admin, None, Kind::Commit, 0, false, b"0 -> 1")
        .await
        .ok();
    put(&admin, None, Kind::Commit, 1, false, b"1 -> 2")
        .await
        .ok();
    // Not the last one's epoch.
    assert_eq!(
        said(put(&admin, None, Kind::Void, 0, false, b"void").await),
        (StatusCode::CONFLICT, "void_refused".into())
    );
    assert_eq!(
        said(put(&admin, None, Kind::Void, 2, false, b"void").await),
        (StatusCode::CONFLICT, "void_refused".into())
    );
    // A void says its epoch.
    let (status, p) = admin
        .client
        .post::<_, Appended>(
            "/v1/team/stream",
            &AppendRequest {
                own: false,
                lease: None,
                fence: None,
                kind: Kind::Void,
                epoch: None,
                payload: b"void".to_vec(),
            },
        )
        .await
        .refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::BAD_REQUEST, "no_epoch")
    );
    put(&admin, None, Kind::Void, 1, false, b"void").await.ok();
    s.finish().await;
}
