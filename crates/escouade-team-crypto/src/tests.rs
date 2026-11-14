//! The properties of the mock-up, each against a server that lies (`relay.rs`, its fields public).

use openmls::prelude::tls_codec::Serialize as _;
use openmls::prelude::{
    CredentialWithKey, KeyPackage, LeafNodeParameters, MlsGroup, MlsGroupJoinConfig,
    MlsMessageBodyIn, MlsMessageOut, ProtocolVersion, MIXED_CIPHERTEXT_WIRE_FORMAT_POLICY,
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::OpenMlsProvider;

use crate::device::{capabilities, decode, encode};
use crate::invite::{join_mac, InviteLink, JoinRequest, WelcomeDelivery, DEFAULT_LIFETIME};
use crate::keys::{aead_open, expand, project_key};
use crate::recovery::EscrowWrap;
use crate::relay::Relay;
use crate::roster::{Role, TeamFingerprint};
use crate::*;

const NOW: u64 = 1_800_000_000;
const SERVER: &str = "team.example.org";

fn member(n: u8) -> MemberId {
    MemberId([n; 16])
}

fn project(n: u8) -> ProjectId {
    ProjectId([n; 16])
}

fn object(n: u8) -> ObjectId {
    ObjectId([n; 16])
}

fn publish(relay: &mut Relay, publication: Publication) {
    relay.publish(publication).expect("the server takes it");
}

/// The owner creates the team.
fn new_team(escrow: Option<roster::EscrowPublic>) -> (Relay, TeamDevice, RecoveryKey) {
    let mut relay = Relay::default();
    let mut owner = TeamDevice::new(member(1)).unwrap();
    let (recovery, publication) = owner.create_team(NOW, escrow).unwrap();
    publish(&mut relay, publication);
    (relay, owner, recovery)
}

/// The whole invitation, as the link travels (a URL, parsed again), the admin online.
fn join(relay: &mut Relay, admin: &mut TeamDevice, purpose: Purpose, who: MemberId) -> TeamDevice {
    let link = admin
        .invite(purpose, NOW, DEFAULT_LIFETIME, SERVER)
        .unwrap();
    let link = InviteLink::parse(&link.to_url()).unwrap();
    let mut device = TeamDevice::new(who).unwrap();
    let request = device.request_join(&link).unwrap();
    relay.join_requests.push(request.clone());
    publish(relay, admin.admit(&request, NOW).unwrap());
    let delivery = relay.welcomes[&link.id].clone();
    device.accept_welcome(&delivery).unwrap();
    device.read_history(|e| relay.history_link(e)).unwrap();
    device
}

fn sync(relay: &Relay, devices: &mut [&mut TeamDevice]) {
    for d in devices.iter_mut() {
        d.sync(&relay.commits).unwrap();
    }
}

/// A KeyPackage of a device that is not in the team (Mallory's).
fn outsider_key_package(who: MemberId) -> KeyPackage {
    let mut outsider = TeamDevice::new(who).unwrap();
    let link = InviteLink {
        server: SERVER.into(),
        id: [0; 16],
        secret: [0; 32],
        team: [0; 32],
    };
    let request = outsider.request_join(&link).unwrap();
    match decode(&request.key_package).unwrap().extract() {
        MlsMessageBodyIn::KeyPackage(kp) => kp
            .validate(
                OpenMlsRustCrypto::default().crypto(),
                ProtocolVersion::Mls10,
            )
            .unwrap(),
        _ => unreachable!(),
    }
}

#[test]
fn the_team_is_an_mls_group_of_the_chosen_suite() {
    let (relay, mut owner, recovery) = new_team(None);
    assert_eq!(owner.mls().2.ciphersuite(), CIPHERSUITE);
    assert_eq!(owner.epoch().unwrap(), 0);
    assert_eq!(owner.group_id().unwrap().len(), 32);
    let roster = owner.roster().unwrap();
    assert_eq!(roster.role(&member(1)), Some(Role::Admin));
    assert_eq!(roster.genesis.owner_key, owner.signature_key());
    assert_eq!(roster.recovery, recovery.public(owner.crypto()).unwrap());
    assert!(roster.escrow.is_none());
    assert_eq!(owner.fingerprint().unwrap().words().len(), 6);
    // The recovery key's wrap of epoch 0, no escrow wrap, no commit yet.
    assert!(relay.recovery.contains_key(&0));
    assert!(relay.escrow.is_empty());
    assert!(relay.commits.is_empty());
}

#[test]
fn an_admin_adds_and_removes_a_member() {
    let (mut relay, mut alice, _) = new_team(None);
    let mut bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    assert_eq!(bob.epoch().unwrap(), 1);
    assert_eq!(bob.roster().unwrap().role(&member(2)), Some(Role::Member));
    assert_eq!(bob.fingerprint().unwrap(), alice.fingerprint().unwrap());

    let note = alice
        .seal(project(1), object(1), 1, b"ticket ESC-1")
        .unwrap();
    assert_eq!(bob.open(&note).unwrap(), b"ticket ESC-1");
    let reply = bob.seal(project(1), object(2), 1, b"spec page").unwrap();
    assert_eq!(alice.open(&reply).unwrap(), b"spec page");

    publish(&mut relay, alice.remove_member(member(2)).unwrap());
    assert_eq!(alice.epoch().unwrap(), 2);
    assert!(alice.roster().unwrap().role(&member(2)).is_none());
    assert_eq!(
        bob.process_commit(&relay.commits[1]).unwrap(),
        Processed::Removed
    );
    assert!(bob.is_removed());
}

#[test]
fn a_removed_member_reads_nothing_new() {
    let (mut relay, mut alice, _) = new_team(None);
    let mut bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    let mut carol = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(3),
    );
    sync(&relay, &mut [&mut bob]);
    let before = alice.seal(project(1), object(1), 1, b"before").unwrap();
    assert_eq!(bob.open(&before).unwrap(), b"before");

    publish(&mut relay, alice.remove_member(member(2)).unwrap());
    sync(&relay, &mut [&mut bob, &mut carol]);
    assert!(bob.is_removed());
    publish(&mut relay, alice.rotate().unwrap());
    sync(&relay, &mut [&mut carol]);
    let after = alice.seal(project(1), object(1), 2, b"after").unwrap();
    assert_eq!(carol.open(&after).unwrap(), b"after");

    // Through the API: nothing.
    assert!(bob.open(&after).is_err());
    // With every key Bob ever held: neither the new object, nor the history links of the new epochs.
    let new_epochs: Vec<u64> = relay.history.keys().copied().filter(|e| *e > 2).collect();
    assert!(!new_epochs.is_empty());
    for epoch in bob.known_epochs() {
        let record = bob.record(epoch).unwrap();
        let key = keys::object_key(
            &project_key(&record.content_secret, &project(1)),
            &object(1),
        );
        assert!(aead_open(&key, &after.nonce, &after.header.aad(), &after.ciphertext).is_err());
        let group_id = alice.group_id().unwrap();
        for e in &new_epochs {
            let mut forged = record.clone();
            forged.epoch = *e;
            assert!(relay.history[e].open(&group_id, &forged).is_err());
        }
    }
    // And the commits after his removal are not his to process.
    assert!(bob.process_commit(&relay.commits[3]).is_err());
}

#[test]
fn a_rotation_changes_the_keys_without_encrypting_the_history_again() {
    let (mut relay, mut alice, _) = new_team(None);
    let mut bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    let old = alice.seal(project(1), object(1), 1, b"v1").unwrap();
    let old_bytes = serde_json::to_vec(&old).unwrap();
    let old_epoch = alice.epoch().unwrap();

    publish(&mut relay, alice.rotate().unwrap());
    sync(&relay, &mut [&mut bob]);
    let new_epoch = alice.epoch().unwrap();
    assert_eq!(new_epoch, old_epoch + 1);
    let (r0, r1) = (
        alice.record(old_epoch).unwrap(),
        alice.record(new_epoch).unwrap(),
    );
    assert_ne!(r0.project_key(&project(1)), r1.project_key(&project(1)));
    assert_ne!(r0.head, r1.head);

    // The old object is untouched (C8-A) and still read by both.
    assert_eq!(serde_json::to_vec(&old).unwrap(), old_bytes);
    assert_eq!(bob.open(&old).unwrap(), b"v1");
    // The new one is under the new epoch: an old record does not open it.
    let new = alice.seal(project(1), object(1), 2, b"v2").unwrap();
    assert_eq!(new.header.epoch, new_epoch);
    assert_eq!(bob.open(&new).unwrap(), b"v2");
    let mut stolen = new.clone();
    stolen.header.epoch = old_epoch;
    stolen.header.head = r0.head;
    assert!(bob.open(&stolen).is_err());
}

#[test]
fn project_keys_differ_by_project_and_by_epoch() {
    let (mut relay, mut alice, _) = new_team(None);
    publish(&mut relay, alice.rotate().unwrap());
    let (r0, r1) = (
        alice.record(0).unwrap().clone(),
        alice.record(1).unwrap().clone(),
    );
    let keys = [
        r0.project_key(&project(1)),
        r0.project_key(&project(2)),
        r1.project_key(&project(1)),
        r1.project_key(&project(2)),
    ];
    for (i, a) in keys.iter().enumerate() {
        for b in &keys[i + 1..] {
            assert_ne!(a, b);
        }
    }
    // An object of a project does not open as an object of another.
    let mut moved = alice.seal(project(1), object(1), 1, b"secret").unwrap();
    moved.header.project = project(2);
    assert!(alice.open(&moved).is_err());
}

#[test]
fn a_new_device_reads_the_whole_history() {
    let (mut relay, mut alice, _) = new_team(None);
    let mut written = vec![(
        alice.seal(project(1), object(1), 1, b"epoch 0").unwrap(),
        b"epoch 0".to_vec(),
    )];
    let mut bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    written.push((
        bob.seal(project(1), object(2), 1, b"epoch 1, by bob")
            .unwrap(),
        b"epoch 1, by bob".to_vec(),
    ));
    publish(&mut relay, alice.rotate().unwrap());
    sync(&relay, &mut [&mut bob]);
    written.push((
        alice.seal(project(2), object(3), 1, b"epoch 2").unwrap(),
        b"epoch 2".to_vec(),
    ));
    let mut carol = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(3),
    );
    sync(&relay, &mut [&mut bob]);
    written.push((
        carol.seal(project(2), object(4), 1, b"epoch 3").unwrap(),
        b"epoch 3".to_vec(),
    ));

    // Bob adds his own second device himself (a member may): it was in none of these epochs.
    let bob2 = join(
        &mut relay,
        &mut bob,
        Purpose::OwnDevice(member(2)),
        member(2),
    );
    sync(&relay, &mut [&mut alice, &mut carol]);
    assert_eq!(
        bob2.known_epochs(),
        (0..=bob2.epoch().unwrap()).collect::<Vec<_>>()
    );
    for (sealed, plain) in &written {
        assert_eq!(&bob2.open(sealed).unwrap(), plain);
    }
    // The authors are checked against the leaves of their epoch, kept in the history.
    let mut forged = written[0].0.clone();
    forged.header.author = carol.signature_key();
    assert!(matches!(bob2.open(&forged), Err(Error::BadSignature)));
}

#[test]
fn a_member_adds_their_own_devices_only_and_never_changes_the_roster() {
    let (mut relay, mut alice, _) = new_team(None);
    let mut bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    assert!(matches!(
        bob.invite(
            Purpose::NewMember(Role::Member),
            NOW,
            DEFAULT_LIFETIME,
            SERVER
        ),
        Err(Error::NotAdmin)
    ));
    assert!(matches!(
        bob.set_role(member(2), Role::Admin),
        Err(Error::NotAdmin)
    ));

    // Bob's device, playing outside the API: an Add of Mallory, committed by raw MLS.
    let mallory = outsider_key_package(member(9));
    let commit = {
        let (provider, signer, group) = bob.mls();
        let bundle = group
            .commit_builder()
            .propose_adds([mallory])
            .load_psks(provider.storage())
            .unwrap()
            .build(provider.rand(), provider.crypto(), signer, |_| true)
            .unwrap()
            .stage_commit(provider)
            .unwrap();
        encode(bundle.commit()).unwrap()
    };
    assert!(matches!(
        alice.process_commit(&commit),
        Err(Error::Policy(_))
    ));

    // Or making himself an administrator.
    let commit = {
        let mut roster = bob.roster().unwrap();
        roster.members.insert(member(2), Role::Admin);
        let (provider, signer, group) = bob.mls();
        group.clear_pending_commit(provider.storage()).unwrap();
        let bundle = group
            .commit_builder()
            .propose_group_context_extensions(roster.to_extensions().unwrap())
            .unwrap()
            .load_psks(provider.storage())
            .unwrap()
            .build(provider.rand(), provider.crypto(), signer, |_| true)
            .unwrap()
            .stage_commit(provider)
            .unwrap();
        encode(bundle.commit()).unwrap()
    };
    assert!(matches!(
        alice.process_commit(&commit),
        Err(Error::Policy(_))
    ));
    assert_eq!(alice.roster().unwrap().role(&member(2)), Some(Role::Member));
}

#[test]
fn the_server_cannot_slip_a_key_in() {
    let (mut relay, mut alice, _) = new_team(None);
    let link = alice
        .invite(
            Purpose::NewMember(Role::Member),
            NOW,
            DEFAULT_LIFETIME,
            SERVER,
        )
        .unwrap();
    let mut bob = TeamDevice::new(member(2)).unwrap();
    let request = bob.request_join(&link).unwrap();

    // The server puts its own KeyPackage in place of Bob's: it cannot make the MAC without the link's secret.
    let server_kp = encode(&MlsMessageOut::from(outsider_key_package(member(9)))).unwrap();
    let swapped = JoinRequest {
        key_package: server_kp.clone(),
        ..request.clone()
    };
    assert!(matches!(
        alice.admit(&swapped, NOW),
        Err(Error::Invitation(InvitationError::BadMac))
    ));
    let guessed = JoinRequest {
        mac: join_mac(&[7; 32], &request.invitation, &server_kp),
        key_package: server_kp,
        ..request.clone()
    };
    assert!(matches!(
        alice.admit(&guessed, NOW),
        Err(Error::Invitation(InvitationError::BadMac))
    ));
    // Bob's own request still goes through.
    publish(&mut relay, alice.admit(&request, NOW).unwrap());
    bob.accept_welcome(&relay.welcomes[&link.id]).unwrap();

    // Even a GroupInfo in the server's hands (it never has one in the clear) gives it no seat: its external commit
    // is refused by every member.
    let group_info = {
        let (provider, signer, group) = alice.mls();
        group
            .export_group_info(provider.crypto(), signer, true)
            .unwrap()
    };
    let group_info = match decode(&encode(&group_info).unwrap()).unwrap().extract() {
        MlsMessageBodyIn::GroupInfo(gi) => gi,
        _ => unreachable!(),
    };
    let provider = OpenMlsRustCrypto::default();
    let signer = SignatureKeyPair::new(CIPHERSUITE.signature_algorithm()).unwrap();
    let identity = Identity {
        member: member(9),
        device: DeviceId([9; 16]),
        kind: DeviceKind::Recovery,
    };
    let (_, bundle) = MlsGroup::external_commit_builder()
        .with_config(
            MlsGroupJoinConfig::builder()
                .wire_format_policy(MIXED_CIPHERTEXT_WIRE_FORMAT_POLICY)
                .build(),
        )
        .build_group(
            &provider,
            group_info,
            CredentialWithKey {
                credential: identity.credential(),
                signature_key: signer.public().into(),
            },
        )
        .unwrap()
        .leaf_node_parameters(
            LeafNodeParameters::builder()
                .with_capabilities(capabilities())
                .build(),
        )
        .load_psks(provider.storage())
        .unwrap()
        .build(provider.rand(), provider.crypto(), &signer, |_| true)
        .unwrap()
        .finalize(&provider)
        .unwrap();
    let commit = bundle.commit().tls_serialize_detached().unwrap();
    assert!(matches!(bob.process_commit(&commit), Err(Error::Policy(_))));
}

#[test]
fn the_server_cannot_show_another_team() {
    let (mut relay, mut alice, _) = new_team(None);
    let link = alice
        .invite(
            Purpose::NewMember(Role::Member),
            NOW,
            DEFAULT_LIFETIME,
            SERVER,
        )
        .unwrap();
    let mut bob = TeamDevice::new(member(2)).unwrap();
    let request = bob.request_join(&link).unwrap();

    // The server makes a team of its own and admits Bob into it.
    let (_, mut fake, _) = new_team(None);
    let fake_link = fake
        .invite(
            Purpose::NewMember(Role::Member),
            NOW,
            DEFAULT_LIFETIME,
            SERVER,
        )
        .unwrap();
    let fake_request = JoinRequest {
        invitation: fake_link.id,
        mac: join_mac(&fake_link.secret, &fake_link.id, &request.key_package),
        key_package: request.key_package.clone(),
    };
    let fake_welcome = fake.admit(&fake_request, NOW).unwrap().welcome.unwrap().1;
    assert!(matches!(
        bob.accept_welcome(&fake_welcome),
        Err(Error::Invitation(InvitationError::WrongTeam))
    ));

    // A fake Welcome burns the KeyPackage it was made for (MLS uses one once): Bob asks again, under the same
    // link, which no administrator has used yet.
    let again = bob.request_join(&link).unwrap();
    publish(&mut relay, alice.admit(&again, NOW).unwrap());
    // The real Welcome, its confirmation changed: refused too.
    let mut tampered = relay.welcomes[&link.id].clone();
    tampered.confirmation[0] ^= 1;
    assert!(matches!(
        bob.accept_welcome(&tampered),
        Err(Error::Invitation(InvitationError::BadConfirmation))
    ));
}

#[test]
fn a_leaked_link_gives_no_content() {
    let (mut relay, mut alice, _) = new_team(None);
    let bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    relay.objects.push(
        alice
            .seal(project(1), object(1), 1, b"the product's secrets")
            .unwrap(),
    );
    relay
        .objects
        .push(bob.seal(project(1), object(2), 1, b"the memory").unwrap());

    let link = alice
        .invite(
            Purpose::NewMember(Role::Member),
            NOW,
            DEFAULT_LIFETIME,
            SERVER,
        )
        .unwrap();
    let url = link.to_url();
    // The server sees the part before the `#` only: no secret in it.
    let seen = InviteLink::server_part(&url);
    assert!(!seen.contains('#'));
    assert!(!seen.contains(&base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        link.secret
    )));
    // The link holds an id, the secret and the team's fingerprint: no key.
    let leaked = InviteLink::parse(&url).unwrap();
    assert_eq!(leaked, link);

    // Whoever has the link and every blob of the server reads nothing with it.
    let group_id = alice.group_id().unwrap();
    let candidates = [
        leaked.secret,
        expand(&leaked.secret, b"escouade/join/v1", &leaked.id),
        expand(&leaked.secret, b"escouade/project/v1", &project(1).0),
    ];
    for key in &candidates {
        for sealed in &relay.objects {
            assert!(
                aead_open(key, &sealed.nonce, &sealed.header.aad(), &sealed.ciphertext).is_err()
            );
        }
        for link in relay.history.values() {
            let guess = keys::EpochRecord {
                epoch: link.epoch,
                content_secret: *key,
                history_key: *key,
                head: [0; 32],
                members: Vec::new(),
            };
            assert!(link.open(&group_id, &guess).is_err());
        }
    }

    // The legitimate invitee uses it: a second use, by whoever found the link, is refused, and the administrator
    // knows of it.
    let mut carol = TeamDevice::new(member(3)).unwrap();
    let request = carol.request_join(&leaked).unwrap();
    publish(&mut relay, alice.admit(&request, NOW).unwrap());
    let mut thief = TeamDevice::new(member(9)).unwrap();
    let stolen = thief.request_join(&leaked).unwrap();
    assert!(matches!(
        alice.admit(&stolen, NOW),
        Err(Error::Invitation(InvitationError::AlreadyUsed))
    ));
    // And an expired link lets nobody in.
    let late = alice
        .invite(Purpose::NewMember(Role::Member), NOW, 60, SERVER)
        .unwrap();
    let mut dave = TeamDevice::new(member(4)).unwrap();
    let request = dave.request_join(&late).unwrap();
    assert!(matches!(
        alice.admit(&request, NOW + 61),
        Err(Error::Invitation(InvitationError::Expired))
    ));
}

#[test]
fn a_fork_is_detected() {
    let (mut relay, mut alice, _) = new_team(None);
    let mut carol = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Admin),
        member(3),
    );
    let mut bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    let mut dave = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(4),
    );
    sync(&relay, &mut [&mut carol, &mut bob]);
    let epoch = alice.epoch().unwrap();
    assert!([
        carol.epoch().unwrap(),
        bob.epoch().unwrap(),
        dave.epoch().unwrap()
    ]
    .iter()
    .all(|e| *e == epoch));

    // Two administrators commit at the same epoch; the server keeps both and shows each to half the team.
    let from_alice = alice.rotate().unwrap().commit.unwrap().1;
    let from_carol = carol.rotate().unwrap().commit.unwrap().1;
    bob.process_commit(&from_alice).unwrap();
    dave.process_commit(&from_carol).unwrap();
    assert_eq!(bob.epoch().unwrap(), dave.epoch().unwrap());

    // The first object that crosses shows it.
    let from_bob = bob.seal(project(1), object(1), 1, b"hello").unwrap();
    assert!(matches!(dave.open(&from_bob), Err(Error::Fork { .. })));
    let from_dave = dave.seal(project(1), object(2), 1, b"hello").unwrap();
    assert!(matches!(bob.open(&from_dave), Err(Error::Fork { .. })));
    // So does a head shown by the server, or by a member.
    let dave_head = dave.checkpoint().unwrap();
    assert!(matches!(
        bob.check_head(dave_head.epoch, &dave_head.head),
        Err(Error::Fork { .. })
    ));
    assert_eq!(
        alice
            .check_head(
                bob.checkpoint().unwrap().epoch,
                &bob.checkpoint().unwrap().head
            )
            .unwrap(),
        HeadCheck::Same
    );
}

#[test]
fn a_rollback_is_refused() {
    let (mut relay, mut alice, _) = new_team(None);
    let mut bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    publish(&mut relay, alice.rotate().unwrap());
    publish(&mut relay, alice.rotate().unwrap());
    sync(&relay, &mut [&mut bob]);
    let seen = bob.checkpoint().unwrap();
    assert_eq!(seen.epoch, 3);

    // The server replays an old commit.
    let replay = relay.commits[1].clone();
    assert!(matches!(
        bob.process_commit(&replay),
        Err(Error::Stale {
            epoch: 1,
            current: 3
        })
    ));
    assert_eq!(bob.epoch().unwrap(), 3);
    // It says the team is at an older epoch, or hides the last entries.
    assert!(matches!(
        bob.check_server_state(2, None),
        Err(Error::Rollback { seen: 3, shown: 2 })
    ));
    assert_eq!(
        bob.check_server_state(3, Some(&seen.head)).unwrap(),
        HeadCheck::Same
    );
    assert_eq!(bob.check_server_state(5, None).unwrap(), HeadCheck::Same);
    // A device put back on an older state is refused by the checkpoint kept beside its keys.
    let ahead = Checkpoint {
        epoch: 4,
        head: [0; 32],
    };
    assert!(matches!(
        bob.check_restored(&ahead),
        Err(Error::Rollback { seen: 4, shown: 3 })
    ));
    assert!(matches!(
        bob.check_restored(&Checkpoint {
            epoch: 3,
            head: [1; 32]
        }),
        Err(Error::Fork { epoch: 3 })
    ));
    bob.check_restored(&seen).unwrap();
    // A history link put in place of another does not open.
    let group_id = alice.group_id().unwrap();
    let mut moved = relay.history[&3].clone();
    moved.epoch = 2;
    assert!(moved.open(&group_id, alice.record(2).unwrap()).is_err());
}

#[test]
fn the_recovery_key_recovers_the_team() {
    let (mut relay, mut alice, recovery) = new_team(None);
    let words = recovery.words();
    assert_eq!(words.len(), 24);
    assert!(recovery.confirms(&[(0, words[0]), (11, words[11]), (23, words[23])]));
    assert!(!recovery.confirms(&[(0, words[1]), (11, words[11]), (23, words[23])]));
    assert!(!recovery.confirms(&[(0, words[0])]));

    let mut bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    let old = alice
        .seal(project(1), object(1), 1, b"before the loss")
        .unwrap();
    publish(&mut relay, alice.rotate().unwrap());
    sync(&relay, &mut [&mut bob]);
    let group_id = alice.group_id().unwrap();
    let lost_device = alice.identity().device;
    // Alice, the only administrator, loses her only device.
    drop(alice);

    // A wrong word, or a checksum that fails: refused.
    let mut wrong = words.clone();
    wrong.swap(0, 1);
    assert!(matches!(
        TeamDevice::recover(
            &wrong.join(" "),
            relay.latest_recovery().unwrap(),
            &group_id
        ),
        Err(Error::Words)
    ));
    // The 24 words, typed again (any case, any spaces).
    let typed = words.join("  ").to_uppercase();
    let (mut rescue, publication) =
        TeamDevice::recover(&typed, relay.latest_recovery().unwrap(), &group_id).unwrap();
    publish(&mut relay, publication);
    assert!(rescue.is_admin().unwrap());
    sync(&relay, &mut [&mut bob]);
    assert_eq!(bob.epoch().unwrap(), rescue.epoch().unwrap());

    // The recovery leaf brings in Alice's new device; it reads all that came before.
    let mut alice2 = join(
        &mut relay,
        &mut rescue,
        Purpose::OwnDevice(member(1)),
        member(1),
    );
    sync(&relay, &mut [&mut bob]);
    assert!(alice2.is_admin().unwrap());
    assert_eq!(alice2.open(&old).unwrap(), b"before the loss");

    // Alice revokes the lost device and puts a new recovery key in place: the recovery leaf leaves.
    publish(&mut relay, alice2.remove_device(lost_device).unwrap());
    sync(&relay, &mut [&mut bob, &mut rescue]);
    let renewed = RecoveryKey::generate(OpenMlsRustCrypto::default().rand()).unwrap();
    publish(
        &mut relay,
        alice2
            .replace_recovery(renewed.public(alice2.crypto()).unwrap())
            .unwrap(),
    );
    sync(&relay, &mut [&mut bob, &mut rescue]);
    assert!(rescue.is_removed());
    let after = alice2.seal(project(1), object(2), 1, b"after").unwrap();
    assert_eq!(bob.open(&after).unwrap(), b"after");

    // The old words open no new wrap; with an old one, their external commit is refused.
    assert!(matches!(
        TeamDevice::recover(
            &words.join(" "),
            relay.latest_recovery().unwrap(),
            &group_id
        ),
        Err(Error::Decrypt)
    ));
    let (_, stale) = TeamDevice::recover(&words.join(" "), &relay.recovery[&2], &group_id).unwrap();
    assert!(bob.process_commit(&stale.commit.unwrap().1).is_err());
    // The new words would.
    let (_, fresh) = TeamDevice::recover(
        &renewed.words().join(" "),
        relay.latest_recovery().unwrap(),
        &group_id,
    )
    .unwrap();
    assert!(bob.process_commit(&fresh.commit.unwrap().1).is_ok());
}

#[test]
fn the_escrow_key_reads_everything_and_every_member_sees_it() {
    let crypto_provider = OpenMlsRustCrypto::default();
    let escrow = EscrowKey::generate(
        crypto_provider.crypto(),
        crypto_provider.rand(),
        "the CISO of ACME",
    )
    .unwrap();
    let (mut relay, mut alice, _) = new_team(Some(escrow.public()));
    let mut bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    assert_eq!(alice.escrow().unwrap(), Some(escrow.public()));
    assert_eq!(bob.escrow().unwrap().unwrap().holder, "the CISO of ACME");

    let first = alice.seal(project(1), object(1), 1, b"epoch 1").unwrap();
    publish(&mut relay, alice.rotate().unwrap());
    sync(&relay, &mut [&mut bob]);
    let second = bob.seal(project(1), object(2), 1, b"epoch 2").unwrap();

    // The organisation opens the last wrap, walks the history back, and reads both.
    let group_id = alice.group_id().unwrap();
    let crypto = crypto_provider.crypto();
    let latest = escrow
        .open(crypto, &group_id, relay.latest_escrow().unwrap())
        .unwrap();
    let mut records = vec![latest.clone()];
    records.extend(history::walk_back(&group_id, &latest, 0, |e| relay.history_link(e)).unwrap());
    let read = |sealed: &content::SealedObject| {
        let record = records
            .iter()
            .find(|r| r.epoch == sealed.header.epoch)
            .unwrap();
        sealed.open(record).unwrap()
    };
    assert_eq!(read(&first), b"epoch 1");
    assert_eq!(read(&second), b"epoch 2");

    // Removed by an administrator: shown as gone, and the next epochs are not sealed to it.
    let last_wrapped = *relay.escrow.keys().next_back().unwrap();
    publish(&mut relay, alice.set_escrow(None).unwrap());
    sync(&relay, &mut [&mut bob]);
    assert_eq!(bob.escrow().unwrap(), None);
    assert_eq!(*relay.escrow.keys().next_back().unwrap(), last_wrapped);
    // A wrap opened for one epoch is not one of another.
    let mut moved: EscrowWrap = relay.escrow[&last_wrapped].clone();
    moved.epoch = 0;
    assert!(escrow.open(crypto, &group_id, &moved).is_err());

    // A team without one has none.
    let (relay, alice, _) = new_team(None);
    assert!(relay.escrow.is_empty());
    assert_eq!(alice.escrow().unwrap(), None);
}

#[test]
fn an_object_cannot_be_forged_or_moved() {
    let (mut relay, mut alice, _) = new_team(None);
    let bob = join(
        &mut relay,
        &mut alice,
        Purpose::NewMember(Role::Member),
        member(2),
    );
    let sealed = alice.seal(project(1), object(1), 3, b"criteria").unwrap();
    assert_eq!(bob.open(&sealed).unwrap(), b"criteria");

    let mut older = sealed.clone();
    older.header.version = 2;
    assert!(bob.open(&older).is_err());
    let mut other = sealed.clone();
    other.header.object = object(2);
    assert!(bob.open(&other).is_err());
    let mut flipped = sealed.clone();
    flipped.ciphertext[0] ^= 1;
    assert!(matches!(bob.open(&flipped), Err(Error::BadSignature)));
    // Bob signs a ciphertext as himself and claims Alice wrote it: the signature is not hers.
    let mut claimed = bob.seal(project(1), object(1), 4, b"forged").unwrap();
    claimed.header.author = alice.signature_key();
    assert!(matches!(bob.open(&claimed), Err(Error::BadSignature)));
    // An outsider's key is not an author of the epoch.
    let outsider = TeamDevice::new(member(9)).unwrap();
    let mut foreign = sealed;
    foreign.header.author = outsider.signature_key();
    assert!(matches!(bob.open(&foreign), Err(Error::BadSignature)));
}

#[test]
fn an_invitation_link_round_trips_and_rejects_garbage() {
    let link = InviteLink {
        server: SERVER.into(),
        id: [1; 16],
        secret: [2; 32],
        team: [3; 32],
    };
    assert_eq!(InviteLink::parse(&link.to_url()).unwrap(), link);
    assert_eq!(link.team(), TeamFingerprint([3; 32]));
    for bad in [
        "http://team.example.org/join/01#s=AA&t=AA",
        "https://team.example.org/join/0101#s=AA&t=AA",
        "https://team.example.org/join/01010101010101010101010101010101",
        "https://team.example.org/join/01010101010101010101010101010101#t=AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM",
        "https://team.example.org/join/01010101010101010101010101010101#s=AA&t=AA&x=1",
    ] {
        assert!(InviteLink::parse(bad).is_err(), "{bad}");
    }
    let _ = WelcomeDelivery {
        welcome: Vec::new(),
        confirmation: [0; 32],
    };
}

#[test]
fn the_words_are_bip39() {
    // The official BIP-39 vectors for 256 bits (trezor/python-mnemonic, vectors.json).
    let zeros = RecoveryKey::from_entropy([0; 32]).words().join(" ");
    assert_eq!(zeros, format!("{}art", "abandon ".repeat(23)));
    let sevens = RecoveryKey::from_entropy([0x7f; 32]).words().join(" ");
    assert_eq!(
        sevens,
        "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful \
         legal winner thank year wave sausage worth title"
    );
    // The fingerprint's six words are the first six of its bytes as a mnemonic: 66 bits, 11 by word.
    let fingerprint = TeamFingerprint([0x49; 32]);
    let mnemonic = bip39::Mnemonic::from_entropy(&fingerprint.0).unwrap();
    assert_eq!(
        fingerprint.words().to_vec(),
        mnemonic.words().take(6).collect::<Vec<_>>()
    );
}
