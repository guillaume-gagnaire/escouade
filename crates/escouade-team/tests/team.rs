//! The client against the real `escouade-team-server`, on a temporary SQLite base: the members, the rotation, the
//! concurrent commits, a device that starts again, an old backup refused, a server that lies.

mod common;

use std::sync::atomic::Ordering;
use std::time::Duration;

use common::*;
use escouade_team::crypto::{Change, DeviceKind, ObjectId, ProjectId, Role};
use escouade_team::{Applied, ClientError, CryptoError, Error, KeyPlace, Team};

const PROJECT: ProjectId = ProjectId([7; 16]);

fn object(n: u8) -> ObjectId {
    ObjectId([n; 16])
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn members_join_leave_and_the_keys_rotate() {
    let w = World::new().await;
    let (pa, pb, pc) = (w.place(), w.place(), w.place());

    // Alice creates the team, and writes before anyone joins.
    let mut alice = w.create(&pa, member(1)).await;
    assert_eq!(alice.epoch().unwrap(), 0);
    assert_eq!(*alice.key_place(), KeyPlace::Keychain);
    alice
        .write_object(PROJECT, object(1), 1, b"ticket ESC-1")
        .await
        .unwrap();

    // Bob joins (epoch 1) and reads what was written before him, through the history link.
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    assert_eq!(bob.epoch().unwrap(), 1);
    let read = bob.read_objects(PROJECT, 0).await.unwrap();
    assert_eq!(read[0].1.as_ref().unwrap(), b"ticket ESC-1");
    assert_eq!(bob.roster().unwrap(), alice.roster().unwrap());

    // Carol joins (epoch 2); Bob follows.
    let mut carol = w.join(&mut alice, &pc, member(3), Role::Member).await;
    bob.sync().await.unwrap();
    assert_eq!(bob.epoch().unwrap(), 2);
    assert_eq!(bob.head_words().unwrap(), carol.head_words().unwrap());

    // The verified journal of the members: the roster of the group, each device in it.
    let members = carol.members().unwrap();
    assert_eq!(members.len(), 3);
    let alice_row = members.iter().find(|(m, _, _)| *m == member(1)).unwrap();
    assert_eq!(alice_row.1, Role::Admin);
    assert_eq!(alice_row.2, vec![alice.identity().device]);

    // A member cannot remove another.
    assert!(matches!(
        carol.remove_member(member(2)).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));

    // Alice removes Bob: the server cuts him first, then the commit (epoch 3).
    let bob_copy = bob
        .client()
        .read_team(0, None)
        .await
        .expect("Bob still reads before");
    assert!(!bob_copy.entries.is_empty());
    assert_eq!(
        alice.remove_member(member(2)).await.unwrap(),
        Applied::Done { epoch: 3 }
    );
    assert!(matches!(
        bob.sync().await,
        Err(Error::Client(ClientError::SignInRefused))
    ));
    carol.sync().await.unwrap();
    assert_eq!(carol.epoch().unwrap(), 3);
    assert!(carol.roster().unwrap().role(&member(2)).is_none());
    assert!(carol
        .device()
        .leaves()
        .unwrap()
        .iter()
        .all(|i| i.member != member(2)));

    // The rotation (epoch 4), by Alice; Carol follows; the old content is not encrypted again (C8-A) and still opens.
    assert_eq!(alice.rotate().await.unwrap(), Applied::Done { epoch: 4 });
    carol.sync().await.unwrap();
    assert_eq!(carol.epoch().unwrap(), 4);
    let seq = alice
        .write_object(PROJECT, object(2), 1, b"after Bob")
        .await
        .unwrap();
    let read = carol.read_objects(PROJECT, 0).await.unwrap();
    assert_eq!(read[0].1.as_ref().unwrap(), b"ticket ESC-1");
    assert_eq!(read.last().unwrap().1.as_ref().unwrap(), b"after Bob");

    // Bob, given the new object by another way, opens nothing of it.
    let sealed = carol
        .client()
        .read_project(&PROJECT.0, seq - 1, None)
        .await
        .unwrap();
    let sealed: escouade_team::crypto::content::SealedObject =
        escouade_team::wire::decode(&sealed.entries[0].payload).unwrap();
    assert!(bob.device().open(&sealed).is_err());

    // Every link of the history is on the server: a device that joins now reads back to epoch 0.
    let pd = w.place();
    let dave = w.join(&mut alice, &pd, member(4), Role::Member).await;
    assert_eq!(dave.device().known_epochs(), (0..=5).collect::<Vec<_>>());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_administrators_commit_at_once_one_wins_the_other_plays_again() {
    let w = World::new().await;
    let (pa, pd, pc) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Admin).await;
    let mut carol = w.join(&mut alice, &pc, member(3), Role::Member).await;
    // Dave an administrator on the server too (the commits are the administrators', SR2-02).
    alice.serve(NOW).await.unwrap();
    dave.sync().await.unwrap();
    assert!(dave.is_admin().unwrap());
    let epoch = alice.epoch().unwrap();
    assert_eq!(dave.epoch().unwrap(), epoch);

    // Dave's commit is held on its way; Alice commits in between; the server refuses Dave's (409 epoch_taken).
    let (there, go) = pd.liar.hold_next_commit();
    let (from_dave, held) = tokio::join!(dave.rotate(), async {
        if tokio::time::timeout(Duration::from_secs(20), there)
            .await
            .is_err()
        {
            return false;
        }
        assert_eq!(
            alice.rotate().await.unwrap(),
            Applied::Done { epoch: epoch + 1 }
        );
        go.send(()).unwrap();
        true
    });
    assert!(held, "Dave's commit reached the server: {from_dave:?}");
    // Dave played Alice's commit, then his own again.
    assert_eq!(from_dave.unwrap(), Applied::Done { epoch: epoch + 2 });
    assert_eq!(pd.liar.conflicts.load(Ordering::SeqCst), 1);
    alice.sync().await.unwrap();
    carol.sync().await.unwrap();
    for t in [&alice, &dave, &carol] {
        assert_eq!(t.epoch().unwrap(), epoch + 2);
        assert_eq!(t.checkpoint(), alice.checkpoint());
    }
    // One branch: what Dave writes, Carol reads.
    dave.write_object(PROJECT, object(1), 1, b"one branch")
        .await
        .unwrap();
    let read = carol.read_objects(PROJECT, 0).await.unwrap();
    assert_eq!(read.last().unwrap().1.as_ref().unwrap(), b"one branch");

    // Both remove Carol at once: Dave loses the epoch, and finds the change done.
    let (there, go) = pd.liar.hold_next_commit();
    let (from_dave, held) = tokio::join!(dave.remove_member(member(3)), async {
        if tokio::time::timeout(Duration::from_secs(20), there)
            .await
            .is_err()
        {
            return false;
        }
        alice.remove_member(member(3)).await.unwrap();
        go.send(()).unwrap();
        true
    });
    assert!(held, "Dave's commit reached the server: {from_dave:?}");
    assert_eq!(from_dave.unwrap(), Applied::AlreadyDone);
    assert_eq!(dave.checkpoint(), alice.checkpoint());
    // The server holds exactly one commit per epoch.
    let all = alice.client().read_team(0, None).await.unwrap();
    let commits: Vec<u64> = all.entries.iter().filter_map(|e| e.epoch).collect();
    assert_eq!(commits, (0..alice.epoch().unwrap()).collect::<Vec<_>>());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_device_starts_again_from_its_state() {
    let w = World::new().await;
    let (pa, pb) = (w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    alice.rotate().await.unwrap();
    bob.sync().await.unwrap();
    let (id, epoch, head) = (bob.id(), bob.epoch().unwrap(), bob.checkpoint());
    drop(bob);

    // Bob starts again: the same epoch and head, the history, the stream from where it was.
    let mut bob = Team::open(pb.setup(), id).unwrap();
    assert_eq!((bob.epoch().unwrap(), bob.checkpoint()), (epoch, head));
    assert_eq!(bob.device().known_epochs(), (0..=epoch).collect::<Vec<_>>());
    alice
        .write_object(PROJECT, object(1), 1, b"while away")
        .await
        .unwrap();
    alice.rotate().await.unwrap();
    bob.sync().await.unwrap();
    assert_eq!(bob.checkpoint(), alice.checkpoint());
    let read = bob.read_objects(PROJECT, 0).await.unwrap();
    assert_eq!(read[0].1.as_ref().unwrap(), b"while away");

    // Bob commits, the server takes it, the answer is lost; Bob stops. Started again, he finds his commit in the
    // stream and merges it: the same epoch as Alice, one branch.
    pb.liar.lies.lock().unwrap().lose_commit_answer = true;
    assert!(matches!(
        bob.rotate().await,
        Err(Error::Client(ClientError::Net(_)))
    ));
    assert!(bob.device().pending().is_some());
    drop(bob);
    let mut bob = Team::open(pb.setup(), id).unwrap();
    assert!(bob.device().pending().is_some());
    let synced = bob.sync().await.unwrap();
    assert_eq!(synced.commits, 1);
    assert!(bob.device().pending().is_none());
    alice.sync().await.unwrap();
    assert_eq!(bob.checkpoint(), alice.checkpoint());

    // The keychain's checkpoint follows the state.
    let kept = pb
        .keychain
        .names()
        .into_iter()
        .find(|n| n.ends_with(".checkpoint"))
        .unwrap();
    let cp: escouade_team::crypto::Checkpoint =
        serde_json::from_str(&pb.keychain.entry(&kept).unwrap()).unwrap();
    assert_eq!(Some(cp), bob.checkpoint());
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_old_backup_put_back_is_refused() {
    let w = World::new().await;
    let (pa, pb) = (w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let (id, folder) = (bob.id(), bob.folder().to_path_buf());
    drop(bob);
    // A backup of Bob's folder at epoch 1.
    let backup = pb.folder.path().join("backup");
    copy_dir(&folder, &backup);

    alice.rotate().await.unwrap();
    alice.rotate().await.unwrap();
    let mut bob = Team::open(pb.setup(), id).unwrap();
    bob.sync().await.unwrap();
    assert_eq!(bob.epoch().unwrap(), 3);
    drop(bob);

    // The backup comes back: the keychain's checkpoint (epoch 3) refuses it.
    std::fs::remove_dir_all(&folder).unwrap();
    copy_dir(&backup, &folder);
    match Team::open(pb.setup(), id) {
        Err(Error::Crypto(CryptoError::Rollback { seen: 3, shown: 1 })) => {}
        Err(e) => panic!("refused, but not as a rollback: {e}"),
        Ok(_) => panic!("an old backup was taken"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_server_that_goes_back_or_rewrites_is_caught() {
    let w = World::new().await;
    let (pa, pb) = (w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    alice.rotate().await.unwrap();
    alice.rotate().await.unwrap();
    // Twice: the second reads Bob's own beacon, and sends nothing more.
    bob.sync().await.unwrap();
    bob.sync().await.unwrap();
    let last = bob
        .client()
        .read_team(0, None)
        .await
        .unwrap()
        .entries
        .last()
        .unwrap()
        .seq;

    // The server hides its last entries: Bob refuses to go back.
    pb.liar.lies.lock().unwrap().hide_after = Some(last - 3);
    match bob.sync().await {
        Err(Error::ServerRollback { seen }) => assert_eq!(seen, last),
        other => panic!("a rollback was taken: {other:?}"),
    }
    // Honest again, it goes on.
    pb.liar.lies.lock().unwrap().hide_after = None;
    bob.sync().await.unwrap();
    bob.sync().await.unwrap();
    let last = bob
        .client()
        .read_team(0, None)
        .await
        .unwrap()
        .entries
        .last()
        .unwrap()
        .seq;

    // The server changes an entry Bob read.
    pb.liar.lies.lock().unwrap().flip = Some(last);
    assert!(matches!(bob.sync().await, Err(Error::Rewritten { .. })));
    pb.liar.lies.lock().unwrap().flip = None;
    bob.sync().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_server_that_forks_the_team_is_caught() {
    let w = World::new().await;
    let (pa, pd, pb) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Admin).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    dave.sync().await.unwrap();
    let epoch = alice.epoch().unwrap();

    // Dave stages a commit for the epoch and keeps it (the server gets it another way); Alice's goes in.
    let staged = dave.device_mut().stage(&Change::Rotate).unwrap();
    let (_, dave_commit) = staged.commit.unwrap();
    dave.device_mut().abandon_pending().unwrap();
    alice.rotate().await.unwrap();
    // Alice's beacon of her new epoch.
    alice.sync().await.unwrap();

    // The server shows Bob Dave's commit in place of Alice's: a valid commit, of an administrator. Bob takes it, and
    // is on another branch; Alice's beacon crosses: signed by a leaf Bob knows, another head. Bob says fork.
    pb.liar.lies.lock().unwrap().swap_commit = Some((epoch, dave_commit));
    match bob.sync().await {
        Err(Error::Crypto(CryptoError::Fork { epoch: e })) => assert_eq!(e, epoch + 1),
        other => panic!("the fork went unseen: {other:?}"),
    }
    assert_eq!(bob.epoch().unwrap(), epoch + 1);
    assert_ne!(bob.checkpoint(), alice.checkpoint());
    assert_ne!(bob.head_words().unwrap(), alice.head_words().unwrap());

    // What crosses the other way shows it to Alice: an object Bob writes on his branch.
    bob.write_object(PROJECT, object(2), 1, b"branch B")
        .await
        .unwrap();
    assert!(matches!(
        alice.read_objects(PROJECT, 0).await,
        Err(Error::Crypto(CryptoError::Fork { .. }))
    ));
    // And what Alice writes on hers, Bob sees as a fork too.
    alice
        .write_object(PROJECT, object(1), 1, b"branch A")
        .await
        .unwrap();
    assert!(matches!(
        bob.read_objects(PROJECT, 0).await,
        Err(Error::Crypto(CryptoError::Fork { .. }))
    ));

    // Bob's beacon, once it goes, shows it to Alice in her stream too.
    let beacon = bob.device().beacon(NOW).unwrap();
    bob.client()
        .append_message(&escouade_team::wire::encode(
            &escouade_team::wire::TeamMessage::Beacon(beacon),
        ))
        .await
        .unwrap();
    assert!(matches!(
        alice.sync().await,
        Err(Error::Crypto(CryptoError::Fork { .. }))
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_missing_history_link_is_written_by_another_member() {
    let w = World::new().await;
    let (pa, pc, pe) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut carol = w.join(&mut alice, &pc, member(3), Role::Member).await;
    alice
        .write_object(PROJECT, object(1), 1, b"before")
        .await
        .unwrap();

    // Carol rotates, but the link of her epoch never reaches the server (she drops it from her outbox).
    let staged = carol.device_mut().stage(&Change::Rotate).unwrap();
    let (epoch, commit) = staged.commit.unwrap();
    carol
        .client()
        .append_commit(epoch, &commit, true)
        .await
        .unwrap();
    carol.device_mut().confirm_pending().unwrap();
    drop(carol);

    // Alice, who holds both epochs, writes the missing link; Eve, who joins after, reads back to epoch 0.
    alice.sync().await.unwrap();
    let written = alice
        .client()
        .read_team(0, None)
        .await
        .unwrap()
        .entries
        .into_iter()
        .filter(|e| e.device == alice.identity().device.0)
        .filter_map(|e| {
            escouade_team::wire::decode::<escouade_team::wire::TeamMessage>(&e.payload).ok()
        })
        .any(|m| matches!(m, escouade_team::wire::TeamMessage::History(l) if l.epoch == epoch + 1));
    assert!(written, "Alice wrote the link of Carol's epoch");
    let mut eve = w.join(&mut alice, &pe, member(5), Role::Member).await;
    eve.sync().await.unwrap();
    assert_eq!(eve.device().known_epochs()[0], 0);
    let read = eve.read_objects(PROJECT, 0).await.unwrap();
    assert_eq!(read[0].1.as_ref().unwrap(), b"before");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_session_renews_itself_and_comes_back_after_a_restart() {
    let w = World::with(|c| c.limits.session_ttl = Duration::from_secs(2)).await;
    let pa = w.place();
    let mut alice = w.create(&pa, member(1)).await;
    let before = pa.liar.sign_ins.load(Ordering::SeqCst);
    alice.client().usage().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2500)).await;
    // The session ended: renewed by itself.
    alice.client().usage().await.unwrap();
    assert!(pa.liar.sign_ins.load(Ordering::SeqCst) > before);

    // Encrypted files.
    alice
        .put_file(PROJECT, object(9), b"a document")
        .await
        .unwrap();
    assert_eq!(
        alice.get_file(PROJECT, object(9)).await.unwrap().unwrap(),
        b"a document"
    );
    let raw = alice
        .client()
        .get_file(&PROJECT.0, &object(9).0)
        .await
        .unwrap()
        .unwrap();
    assert!(!raw.windows(10).any(|win| win == b"a document"));
    assert!(alice.client().usage().await.unwrap().used > 0);
    alice.delete_file(PROJECT, object(9)).await.unwrap();
    assert!(alice.get_file(PROJECT, object(9)).await.unwrap().is_none());
    let _ = DeviceKind::Device;
}

/// SR2-02, the operator's way out: a member's commit of bytes nobody reads, that no administrator declared null yet
/// (`nulls.rs`), removed by the server's operator (`drop-entry`); every device reads on past the hole, the
/// administrator removes the member, and the administrators are told what went. The last entry a device read,
/// removed, is no rollback either.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_operator_removes_the_commit_that_froze_the_team() {
    use escouade_team_server::db::Stream;
    let w = World::new().await;
    let (pa, pb, pc) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut carol = w.join(&mut alice, &pc, member(3), Role::Member).await;
    bob.sync().await.unwrap();
    carol.sync().await.unwrap();
    let epoch = alice.epoch().unwrap();
    let seq = bob
        .client()
        .append_commit(epoch, b"garbage", true)
        .await
        .unwrap();
    assert_eq!(carol.sync().await.unwrap().nulls, 1, "null, passed");

    let store = &w.server.server.app().store;
    store
        .drop_entry(alice.id(), Stream::Team, seq, "operator".into(), 42)
        .await
        .unwrap()
        .unwrap();
    alice.sync().await.unwrap();
    carol.sync().await.unwrap();
    alice.remove_member(member(2)).await.unwrap();
    carol.sync().await.unwrap();
    assert_eq!(carol.epoch().unwrap(), alice.epoch().unwrap());
    assert_eq!(carol.roster().unwrap(), alice.roster().unwrap());
    // The administrator is told; a member is not.
    let served = alice.serve(NOW).await.unwrap();
    assert_eq!(served.drop_alerts, 1);
    let alerts = alice.drop_alerts();
    assert_eq!(alerts.len(), 1);
    assert_eq!(
        (
            alerts[0].project,
            alerts[0].seq,
            alerts[0].commit,
            alerts[0].epoch
        ),
        (None, seq, true, Some(epoch))
    );
    assert_eq!(alerts[0].dropped_at, 42);
    assert_eq!(
        alice.serve(NOW + 1).await.unwrap().drop_alerts,
        0,
        "told once"
    );
    carol.serve(NOW).await.unwrap();
    assert!(carol.drop_alerts().is_empty());

    // The last entry Carol read, removed by the operator: a hole, not a rollback.
    let m = alice
        .client()
        .append_message(b"not understood")
        .await
        .unwrap();
    carol.sync().await.unwrap();
    store
        .drop_entry(alice.id(), Stream::Team, m, "operator".into(), 43)
        .await
        .unwrap()
        .unwrap();
    carol.sync().await.unwrap();
    alice.rotate().await.unwrap();
    carol.sync().await.unwrap();
    assert_eq!(carol.epoch().unwrap(), alice.epoch().unwrap());
    // Reading the list again (five minutes later), the administrator is told of the second one.
    assert_eq!(alice.serve(NOW + 400).await.unwrap().drop_alerts, 1);
    assert_eq!(alice.drop_alerts()[0].seq, m, "the newest first");
    w.finish().await;
}
