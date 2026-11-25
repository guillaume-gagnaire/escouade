//! The operator's `drop-entry` (SR2-02) in the base: an entry that blocks a team (a commit no device reads past)
//! goes, its epoch is free again, the stream keeps its numbering, the audit keeps who, when and the digest, and the
//! devices read the list of what went. On SQLite, and on PostgreSQL when `ESCOUADE_TEST_POSTGRES_URL` is set.

mod common;

use common::{Answer, Backend, Device, TestServer};
use escouade_team_server::api::wire::*;
use escouade_team_server::db::{entry_digest, Kind, StoreError, Stream};
use escouade_team_server::util::Id;
use reqwest::StatusCode;

both!(
    a_commit_that_blocks_the_team_goes_and_its_epoch_is_free_again,
    only_the_last_commit_goes,
    a_projects_entry_goes_and_is_listed_to_its_devices_alone,
    nothing_goes_without_its_team_or_its_entry,
);

fn team_of(d: &Device) -> Id {
    d.team.clone().try_into().unwrap()
}

async fn dropped(d: &Device) -> Vec<DroppedInfo> {
    let r: DroppedEntries = d.client.get("/v1/dropped").await.ok();
    r.dropped
}

async fn a_commit_that_blocks_the_team_goes_and_its_epoch_is_free_again(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let bob = s.add_device(&admin).await;
    let team = team_of(&admin);
    assert_eq!(admin.commit(0, b"admin: epoch 0 -> 1").await.ok().seq, 1);
    admin.message(b"a message").await;
    // Bob takes the next epoch with bytes no device reads: the administrator's commit is refused.
    assert_eq!(bob.commit(1, b"garbage").await.ok().seq, 3);
    let (status, p) = admin.commit(1, b"admin: epoch 1 -> 2").await.refused();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::CONFLICT, "epoch_taken")
    );
    let served = admin.read_team(2).await.entries.remove(0);
    let store = &s.server.app().store;
    let before = store.used_bytes(team).await.unwrap();
    let d = store
        .drop_entry(team, Stream::Team, 3, "operator".into(), 1_000)
        .await
        .unwrap()
        .expect("the entry was there");
    assert_eq!((d.seq, d.kind, d.epoch), (3, Kind::Commit, Some(1)));
    assert_eq!(
        d.digest,
        entry_digest(&served),
        "the digest of the entry as served"
    );
    assert_eq!(d.device.to_vec(), bob.id);
    assert_eq!(d.size, 7);
    assert_eq!(
        store.used_bytes(team).await.unwrap(),
        before - 7,
        "its bytes go back to the quota"
    );
    // The epoch is free again; the stream keeps its numbering: the hole shows.
    assert_eq!(admin.commit(1, b"admin: epoch 1 -> 2").await.ok().seq, 4);
    let seqs: Vec<u64> = admin
        .read_team(0)
        .await
        .entries
        .iter()
        .map(|e| e.seq)
        .collect();
    assert_eq!(seqs, [1, 2, 4]);
    // The audit keeps who and when; the devices are told what went, never by whom.
    let audit = store.dropped(team).await.unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(
        (audit[0].operator.as_str(), audit[0].at),
        ("operator", 1_000)
    );
    for device in [&admin, &bob] {
        let list = dropped(device).await;
        assert_eq!(list.len(), 1);
        let l = &list[0];
        assert_eq!(
            (l.project.clone(), l.seq, l.kind, l.epoch),
            (None, 3, Kind::Commit, Some(1))
        );
        assert_eq!(l.digest, d.digest.to_vec());
        assert_eq!(l.at, 1_000);
    }
    // A message goes too, and a removed entry does not go twice.
    let m = admin.message(b"another message").await;
    assert!(store
        .drop_entry(team, Stream::Team, m, "operator".into(), 1_001)
        .await
        .unwrap()
        .is_some());
    assert!(store
        .drop_entry(team, Stream::Team, m, "operator".into(), 1_002)
        .await
        .unwrap()
        .is_none());
    assert_eq!(dropped(&admin).await.len(), 2);
    s.finish().await;
}

async fn only_the_last_commit_goes(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let team = team_of(&admin);
    admin.commit(0, b"epoch 0 -> 1").await.ok();
    admin.commit(1, b"epoch 1 -> 2").await.ok();
    let store = &s.server.app().store;
    let refused = store
        .drop_entry(team, Stream::Team, 1, "operator".into(), 1)
        .await;
    assert!(matches!(refused, Err(StoreError::Drop(_))), "{refused:?}");
    assert_eq!(admin.read_team(0).await.entries.len(), 2, "nothing went");
    assert!(
        store.dropped(team).await.unwrap().is_empty(),
        "nor is it noted"
    );
    // From the last one back, they go.
    for seq in [2, 1] {
        store
            .drop_entry(team, Stream::Team, seq, "operator".into(), 1)
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(admin.commit(0, b"epoch 0 -> 1 again").await.ok().seq, 3);
    s.finish().await;
}

async fn a_projects_entry_goes_and_is_listed_to_its_devices_alone(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let carol = s.add_device(&admin).await;
    let outsider = s.add_device(&admin).await;
    let team = team_of(&admin);
    let project: [u8; 16] = *b"a-project-of-two";
    let path = format!("/v1/projects/{}/stream", hex::encode(project));
    let commit = |epoch: u64, payload: &[u8]| AppendRequest {
        fence: None,
        kind: Kind::Commit,
        epoch: Some(epoch),
        payload: payload.to_vec(),
    };
    let r: Answer<Appended> = admin.client.post(&path, &commit(0, b"epoch 0 -> 1")).await;
    assert_eq!(r.ok().seq, 1);
    // Kept to the administrator and Carol.
    let access = AccessWire {
        everyone: false,
        devices: [&admin.id, &carol.id]
            .map(|d| serde_bytes::ByteBuf::from(d.clone()))
            .to_vec(),
    };
    let r = admin
        .client
        .request(
            reqwest::Method::PUT,
            &format!("/v1/projects/{}/access", hex::encode(project)),
        )
        .header("content-type", "application/cbor")
        .body(escouade_team_server::transport::cbor::encode(&access))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    let r: Answer<Appended> = carol.client.post(&path, &commit(1, b"garbage")).await;
    assert_eq!(r.ok().seq, 2);
    let store = &s.server.app().store;
    let d = store
        .drop_entry(team, Stream::Project(project), 2, "operator".into(), 5)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(d.project, Some(project));
    let r: Answer<Appended> = admin.client.post(&path, &commit(1, b"epoch 1 -> 2")).await;
    assert_eq!(
        r.ok().seq,
        3,
        "the project's epoch is free again, its numbering kept"
    );
    assert_eq!(dropped(&admin).await.len(), 1);
    assert_eq!(dropped(&carol).await[0].project, Some(project.to_vec()));
    assert!(
        dropped(&outsider).await.is_empty(),
        "a device outside the project is not told of it"
    );
    s.finish().await;
}

async fn nothing_goes_without_its_team_or_its_entry(b: Backend) {
    let s = TestServer::start(&b).await;
    let admin = s.new_team().await;
    let team = team_of(&admin);
    admin.message(b"one").await;
    let store = &s.server.app().store;
    let other: Id = [7; 16];
    assert!(matches!(
        store
            .drop_entry(other, Stream::Team, 1, "operator".into(), 1)
            .await,
        Err(StoreError::Drop(_))
    ));
    assert!(store
        .drop_entry(team, Stream::Team, 9, "operator".into(), 1)
        .await
        .unwrap()
        .is_none());
    assert!(store
        .drop_entry(team, Stream::Project([3; 16]), 1, "operator".into(), 1)
        .await
        .unwrap()
        .is_none());
    assert_eq!(admin.read_team(0).await.entries.len(), 1);
    assert!(store.dropped(team).await.unwrap().is_empty());
    s.finish().await;
}
