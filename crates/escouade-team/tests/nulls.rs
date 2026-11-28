//! SR2-02: a commit that wins its epoch on the server but that MLS or the team's rules refuse (bytes of nothing, or a
//! member's commit that does what only an administrator may) freezes nobody. Every device keeps it null and reads on;
//! the administrator's device declares it null (a signed void), the epoch is taken again without a fork, the
//! administrator removes the member and changes the keys, and is told who posted it. The same in a project's own
//! group. On SQLite, and on PostgreSQL when `ESCOUADE_TEST_POSTGRES_URL` is set.

mod common;

use std::collections::BTreeSet;

use common::*;
use escouade_team::crypto::{CommitScope, ProjectId, Role};
use escouade_team::{CryptoError, Error, Team};
use openmls::prelude::tls_codec::Serialize as _;

both!(
    a_garbage_commit_freezes_nobody,
    a_commit_against_the_rules_freezes_nobody,
    a_garbage_commit_in_a_project_freezes_nobody,
    the_server_keeps_a_members_commit_to_their_own_devices,
    a_void_of_a_commit_this_device_took_is_a_fork,
);

/// The three devices, with their places: a place holds its folder (a temporary directory), dropped with it. Kept for the
/// whole test: a device that reads its state again from the disk (after a refused reading) must still find it.
async fn three(w: &World) -> (Team, Team, Team, [Place; 3]) {
    let (pa, pb, pc) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut carol = w.join(&mut alice, &pc, member(3), Role::Member).await;
    for t in [&mut alice, &mut bob, &mut carol] {
        t.sync().await.unwrap();
        t.serve(NOW).await.unwrap();
    }
    (alice, bob, carol, [pa, pb, pc])
}

/// After a null commit: the administrator removes the member and changes the keys; the others follow, on the same
/// epoch and roster; a device that joins afterwards reads the whole stream, the null commit and its void included.
async fn the_team_goes_on(w: &World, alice: &mut Team, carol: &mut Team, epoch: u64) {
    alice.remove_member(member(2)).await.unwrap();
    alice.rotate().await.unwrap();
    carol.sync().await.unwrap();
    assert_eq!(carol.epoch().unwrap(), alice.epoch().unwrap());
    assert!(alice.epoch().unwrap() >= epoch + 2);
    assert_eq!(carol.roster().unwrap(), alice.roster().unwrap());
    assert!(!alice.roster().unwrap().members.contains_key(&member(2)));
    let pe = w.place();
    let mut eve = w.join(alice, &pe, member(5), Role::Member).await;
    eve.sync().await.unwrap();
    assert_eq!(eve.epoch().unwrap(), alice.epoch().unwrap());
}

async fn a_garbage_commit_freezes_nobody(b: Backend) {
    let w = World::on(&b).await;
    let (mut alice, bob, mut carol, _places) = three(&w).await;
    let epoch = bob.epoch().unwrap();
    let seq = bob
        .client()
        .append_commit(epoch, b"\x00\x01 not an MLS commit", true)
        .await
        .expect("the server keeps it, unread");
    // Every device reads past it, again and again: the group stays at its epoch.
    assert_eq!(carol.sync().await.unwrap().nulls, 1);
    assert_eq!(carol.sync().await.unwrap().nulls, 0);
    assert_eq!(alice.sync().await.unwrap().nulls, 1);
    assert_eq!(carol.epoch().unwrap(), epoch);
    // The administrator is told who posted it; the void is in the stream.
    alice.sync().await.unwrap();
    let alerts = alice.null_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(
        (alerts[0].project, alerts[0].seq, alerts[0].epoch),
        (None, seq, epoch)
    );
    assert_eq!(alerts[0].written_by, bob.identity().device);
    assert!(alerts[0].voided);
    carol.sync().await.unwrap();
    assert!(carol.null_alerts()[0].voided);
    // Until an administrator's commit, the epoch is kept to the administrators: Bob's next try is refused.
    let refused = bob
        .client()
        .append_commit(epoch, b"garbage again", true)
        .await
        .unwrap_err();
    assert_eq!(refused.code(), Some("epoch_reserved"));
    the_team_goes_on(&w, &mut alice, &mut carol, epoch).await;
    w.finish().await;
}

/// A commit MLS takes but the team's rules refuse: Bob makes himself an administrator by raw MLS, and says it is a
/// change of his own devices.
async fn a_commit_against_the_rules_freezes_nobody(b: Backend) {
    let w = World::on(&b).await;
    let (mut alice, mut bob, mut carol, _places) = three(&w).await;
    let epoch = bob.epoch().unwrap();
    let commit = {
        let mut roster = bob.roster().unwrap();
        roster.members.insert(member(2), Role::Admin);
        let (provider, signer, group) = bob.device_mut().mls();
        group.set_aad(CommitScope::Own.team_aad());
        let bundle = group
            .commit_builder()
            .propose_group_context_extensions(roster.to_extensions().unwrap())
            .unwrap()
            .load_psks(openmls_traits::OpenMlsProvider::storage(provider))
            .unwrap()
            .build(
                openmls_traits::OpenMlsProvider::rand(provider),
                openmls_traits::OpenMlsProvider::crypto(provider),
                signer,
                |_| true,
            )
            .unwrap()
            .stage_commit(provider)
            .unwrap();
        bundle.commit().tls_serialize_detached().unwrap()
    };
    bob.client()
        .append_commit(epoch, &commit, true)
        .await
        .expect("the server keeps it, unread");
    assert_eq!(carol.sync().await.unwrap().nulls, 1);
    assert_eq!(alice.sync().await.unwrap().nulls, 1);
    assert_eq!(carol.roster().unwrap().role(&member(2)), Some(Role::Member));
    let alerts = alice.null_alerts();
    assert_eq!(alerts[0].written_by, bob.identity().device);
    assert!(alerts[0].why.contains("rules"), "{}", alerts[0].why);
    the_team_goes_on(&w, &mut alice, &mut carol, epoch).await;
    w.finish().await;
}

async fn a_garbage_commit_in_a_project_freezes_nobody(b: Backend) {
    let w = World::on(&b).await;
    let (mut alice, mut bob, mut carol, _places) = three(&w).await;
    let all: BTreeSet<_> = [member(1), member(2), member(3)].into_iter().collect();
    let kept: ProjectId = alice.create_project(Some(all)).await.unwrap();
    for _ in 0..2 {
        for t in [&mut alice, &mut bob, &mut carol] {
            t.sync().await.unwrap();
            t.serve(NOW).await.unwrap();
        }
    }
    assert!(bob.device().in_project(&kept) && carol.device().in_project(&kept));
    bob.follow(kept).unwrap();
    carol.follow(kept).unwrap();
    let epoch = bob.device().project_epoch(&kept).unwrap();
    let seq = bob
        .client()
        .append_project_commit(&kept.0, epoch, b"not a commit", true)
        .await
        .expect("the server keeps it, unread");
    assert_eq!(carol.sync_project(kept).await.unwrap().nulls, 1);
    assert_eq!(alice.sync_project(kept).await.unwrap().nulls, 1);
    let alert = alice
        .null_alerts()
        .into_iter()
        .find(|a| a.project == Some(kept))
        .expect("the administrator is told");
    assert_eq!((alert.seq, alert.epoch), (seq, epoch));
    assert_eq!(alert.written_by, bob.identity().device);
    // The administrator takes Bob out of the project; Carol follows, and reads what is written after.
    let without: BTreeSet<_> = [member(1), member(3)].into_iter().collect();
    alice
        .set_project_members(kept, without.clone())
        .await
        .unwrap();
    alice
        .create_ticket(
            kept,
            [(
                "title".to_string(),
                serde_json::json!("Après le commit nul"),
            )]
            .into_iter()
            .collect(),
        )
        .unwrap();
    alice.sync_project(kept).await.unwrap();
    carol.sync().await.unwrap();
    carol.sync_project(kept).await.unwrap();
    assert!(carol.null_alerts().iter().any(|a| a.voided));
    assert_eq!(carol.project_members(&kept).unwrap(), without);
    assert_eq!(
        carol.device().project_epoch(&kept).unwrap(),
        alice.device().project_epoch(&kept).unwrap()
    );
    assert_eq!(
        carol.replica(&kept).unwrap().state().unwrap(),
        alice.replica(&kept).unwrap().state().unwrap()
    );
    w.finish().await;
}

/// In defense in depth: the server takes a member's commit only when it says it is of their own devices, and a void
/// from an administrator alone.
async fn the_server_keeps_a_members_commit_to_their_own_devices(b: Backend) {
    let w = World::on(&b).await;
    let (_alice, bob, _carol, _places) = three(&w).await;
    let epoch = bob.epoch().unwrap();
    let refused = bob
        .client()
        .append_commit(epoch, b"garbage", false)
        .await
        .unwrap_err();
    assert_eq!(refused.status(), Some(403));
    let refused = bob
        .client()
        .append_void(None, epoch, b"a void")
        .await
        .unwrap_err();
    assert_eq!(refused.status(), Some(403));
    // A member's own change goes, said so.
    let mut bob = bob;
    bob.rotate().await.unwrap();
    assert_eq!(bob.epoch().unwrap(), epoch + 1);
    w.finish().await;
}

/// A device that took a commit an administrator then declares null shows another version of the team than the
/// administrators': a fork, said, never followed.
async fn a_void_of_a_commit_this_device_took_is_a_fork(b: Backend) {
    let w = World::on(&b).await;
    let (alice, mut bob, mut carol, _places) = three(&w).await;
    let epoch = bob.epoch().unwrap();
    bob.rotate().await.unwrap();
    carol.sync().await.unwrap();
    assert_eq!(carol.epoch().unwrap(), epoch + 1);
    let entries = alice.client().read_team(0, None).await.unwrap().entries;
    let taken = entries
        .iter()
        .rev()
        .find(|e| e.epoch == Some(epoch) && e.kind == escouade_team::wire::Kind::Commit)
        .unwrap();
    let digest: [u8; 32] = <sha2::Sha256 as sha2::Digest>::digest(&taken.payload).into();
    let void = alice
        .device()
        .sign_void(None, epoch, taken.seq, digest)
        .unwrap();
    alice
        .client()
        .append_void(None, epoch, &escouade_team::wire::encode(&void))
        .await
        .unwrap();
    assert!(matches!(
        carol.sync().await,
        Err(Error::Crypto(CryptoError::Fork { .. }))
    ));
    w.finish().await;
}
