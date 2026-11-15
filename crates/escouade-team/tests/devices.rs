//! The devices against the real `escouade-team-server`: a stolen one cut (the server first, then the commit), the
//! live feed that comes back after a cut and a restart of the server, the keys without a keychain, the past epochs'
//! keys erased from the disk.

mod common;

use std::sync::atomic::Ordering;
use std::time::Duration;

use common::*;
use escouade_team::crypto::{Purpose, Role};
use escouade_team::{ClientError, CryptoError, Error, KeyPlace, LiveEvent, LiveFeed, Team};

/// Bob's own other device joins: Bob makes the link, an administrator asks the server's token, Bob admits it.
async fn own_device(alice: &mut Team, bob: &mut Team, place: &Place) -> Team {
    let (link, token) = bob
        .invite(Purpose::OwnDevice(member(2)), NOW)
        .await
        .unwrap();
    assert!(
        token.is_none(),
        "the server gives its token to an administrator alone"
    );
    let token = alice.client().enrollment_token(None).await.unwrap();
    let mut device = Team::join(place.setup(), &token, &link, member(2))
        .await
        .unwrap();
    bob.sync().await.unwrap();
    let request = bob.requests().last().cloned().unwrap();
    bob.admit(&request, NOW).await.unwrap();
    assert!(device.sync().await.unwrap().joined);
    alice.sync().await.unwrap();
    device
}

async fn next(feed: &mut LiveFeed) -> LiveEvent {
    tokio::time::timeout(Duration::from_secs(10), feed.next())
        .await
        .expect("an event in time")
        .expect("the feed goes on")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_stolen_device_is_cut_by_the_server_then_by_the_commit() {
    let w = World::new().await;
    let (pa, pb, pl, pp) = (w.place(), w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut laptop = own_device(&mut alice, &mut bob, &pl).await;
    let members = alice.members().unwrap();
    let bob_row = members.iter().find(|(m, _, _)| *m == member(2)).unwrap();
    assert_eq!(bob_row.2.len(), 2);

    // The laptop is stolen: Alice revokes it. The server cuts it first (its live feed says so), then the commit.
    let mut feed = laptop.live();
    assert_eq!(next(&mut feed).await, LiveEvent::Connected);
    let stolen = laptop.identity().device;
    alice.revoke_device(stolen).await.unwrap();
    let mut revoked = false;
    while let Ok(Some(event)) = tokio::time::timeout(Duration::from_secs(10), feed.next()).await {
        if event == LiveEvent::Revoked {
            revoked = true;
            break;
        }
    }
    assert!(revoked, "the live feed says the device is revoked");
    assert!(matches!(
        laptop.sync().await,
        Err(Error::Client(ClientError::SignInRefused))
    ));
    bob.sync().await.unwrap();
    assert!(bob
        .device()
        .leaves()
        .unwrap()
        .iter()
        .all(|i| i.device != stolen));
    assert_eq!(bob.roster().unwrap().role(&member(2)), Some(Role::Member));

    // Bob revokes his phone himself: the server lets an administrator alone cut another device, the commit does it.
    let phone = own_device(&mut alice, &mut bob, &pp).await;
    let phone_id = phone.identity().device;
    bob.revoke_device(phone_id).await.unwrap();
    alice.sync().await.unwrap();
    assert!(alice
        .device()
        .leaves()
        .unwrap()
        .iter()
        .all(|i| i.device != phone_id));
    // A member does not revoke another member's device.
    let alice_device = alice.identity().device;
    assert!(matches!(
        bob.revoke_device(alice_device).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_live_feed_comes_back_after_a_cut_and_a_restart() {
    let mut w = World::new().await;
    let (pa, pb) = (w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut feed = bob.client().live_with(escouade_team::LiveOptions {
        backoff: Duration::from_millis(50),
        max_backoff: Duration::from_millis(400),
    });
    assert_eq!(next(&mut feed).await, LiveEvent::Connected);

    // An entry of Alice's comes live.
    alice.rotate().await.unwrap();
    let mut seen_commit = false;
    for _ in 0..10 {
        if let LiveEvent::Team(e) = next(&mut feed).await {
            if e.kind == escouade_team::wire::Kind::Commit {
                seen_commit = true;
                break;
            }
        }
    }
    assert!(seen_commit);

    // The connection is cut: the feed comes back by itself, and the stream read from its number has what came.
    w.relay.cut();
    loop {
        if next(&mut feed).await == LiveEvent::Connected {
            break;
        }
    }
    bob.sync().await.unwrap();
    assert_eq!(bob.epoch().unwrap(), alice.epoch().unwrap());

    // The server restarts (its sessions are in memory): the feed signs in again, and so do the calls.
    let before = pb.liar.sign_ins.load(Ordering::SeqCst);
    w.server.stop();
    let again = TestServer::start(w.server.folder.clone(), |_| {}).await;
    w.relay.point_to(again.addr);
    loop {
        if next(&mut feed).await == LiveEvent::Connected {
            break;
        }
    }
    assert!(pb.liar.sign_ins.load(Ordering::SeqCst) > before);
    alice.rotate().await.unwrap();
    let mut seen = false;
    for _ in 0..20 {
        if let LiveEvent::Team(_) = next(&mut feed).await {
            seen = true;
            break;
        }
    }
    assert!(seen, "the new server's entries come live");
    bob.sync().await.unwrap();
    assert_eq!(bob.checkpoint(), alice.checkpoint());
    w.server = again;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn without_a_keychain_the_keys_go_to_a_private_file_and_say_so() {
    let w = World::new().await;
    let pa = w.place();
    pa.keychain.refuse.store(true, Ordering::SeqCst);
    let mut alice = w.create(&pa, member(1)).await;
    assert!(matches!(alice.key_place(), KeyPlace::File(_)));
    alice.rotate().await.unwrap();
    let (id, cp) = (alice.id(), alice.checkpoint());
    drop(alice);
    let alice = Team::open(pa.setup(), id).unwrap();
    assert_eq!(alice.checkpoint(), cp);
    assert!(matches!(alice.key_place(), KeyPlace::File(_)));
    pa.keychain.refuse.store(false, Ordering::SeqCst);
    assert!(pa.keychain.names().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_past_epochs_keys_leave_the_disk() {
    let w = World::new().await;
    let pa = w.place();
    let mut alice = w.create(&pa, member(1)).await;
    let secrets = |team: &Team| -> Vec<Vec<u8>> {
        team.device()
            .provider()
            .sqlite()
            .raw_rows()
            .unwrap()
            .into_iter()
            .filter(|(k, _)| {
                escouade_team::storage::SECRET_LABELS
                    .iter()
                    .any(|l| k.starts_with(l))
            })
            .map(|(_, v)| v)
            .collect()
    };
    let epoch0 = secrets(&alice);
    assert!(!epoch0.is_empty(), "epoch 0 has its secrets on the disk");
    alice.rotate().await.unwrap();
    alice.rotate().await.unwrap();
    let folder = alice.folder().to_path_buf();
    let mut disk = Vec::new();
    for e in std::fs::read_dir(&folder).unwrap() {
        disk.extend(std::fs::read(e.unwrap().path()).unwrap());
    }
    for old in &epoch0 {
        let probe = &old[old.len() - 32..];
        assert!(
            !disk.windows(32).any(|w| w == probe),
            "a secret of epoch 0 is still on the disk"
        );
    }
    // What is there is encrypted: no value of the state is JSON in the clear.
    assert!(!disk.windows(8).any(|w| w == b"\"epoch\":"));
}
