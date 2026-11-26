//! TS-3 against the real `escouade-team-server`: creating a team (its six words, its recovery key and the three words
//! to type back), inviting by a link that serves once and expires, an administrator online who lets the invitee in,
//! the second use told to the invitee and to the administrator, another device of one's own by the QR code's link and
//! by a short code (a wrong code spends its one try), the server's roles in step with the roster, the escrow key that
//! reads the content, the recovery by the 24 words through the server, and the fork as an alarm.

mod common;

use std::time::Duration;

use common::*;
use escouade_team::crypto::content::SealedObject;
use escouade_team::crypto::invite::{InvitationError, InviteLink};
use escouade_team::crypto::{Change, EscrowKey, ObjectId, ProjectId, Purpose, Role};
use escouade_team::pairing::{self, PairCode};
use escouade_team::wire::{self, Kind, TeamMessage};
use escouade_team::{Alarm, CryptoError, Error, PairState, Team};
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::OpenMlsProvider;

const PROJECT: ProjectId = ProjectId([9; 16]);

/// The invitee's side, as it gets the link: as text.
fn as_text(url: &str) -> InviteLink {
    InviteLink::parse(url).unwrap()
}

/// The admin reads the stream and does its work.
async fn online(admin: &mut Team, now: u64) -> escouade_team::Served {
    admin.sync_at(now).await.unwrap();
    admin.serve(now).await.unwrap()
}

/// The device that invited, online: the request waits for its six words (SR2-08), nothing comes in by itself; the
/// person who invites sees the same six words as the invitee's screen, confirms them, and the invitee is let in.
async fn confirm(admin: &mut Team, invitee: &mut Team, now: u64) {
    assert_eq!(online(admin, now).await.waiting, 1);
    assert!(
        !invitee.sync_at(now).await.unwrap().joined,
        "no admission without the six words"
    );
    let shown = admin.admissions(now);
    assert_eq!(shown.len(), 1);
    let words = invitee.join_words().expect("the invitee's six words");
    assert_eq!(
        shown[0].code.words(),
        words,
        "the same six words on both screens"
    );
    admin
        .admit_invitation(shown[0].invitation, &words.join(" "), now)
        .await
        .unwrap();
    assert!(invitee.sync_at(now).await.unwrap().joined);
    assert!(invitee.join_words().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_team_is_created_and_an_invitation_serves_once() {
    let w = World::new().await;
    let (pa, pb, pm) = (w.place(), w.place(), w.place());
    let token = w.server.server.new_team_token().await.unwrap();
    let (mut alice, recovery) = Team::create(pa.setup(), &token, member(1), None, NOW)
        .await
        .unwrap();
    // The recovery key: 24 words, three of them typed back prove they were written down.
    let words = recovery.words();
    assert_eq!(words.len(), 24);
    assert!(recovery.confirms(&[
        (0, words[0]),
        (11, words[11]),
        (23, &words[23].to_uppercase())
    ]));
    assert!(!recovery.confirms(&[(0, words[0]), (11, words[12]), (23, words[23])]));
    assert!(!recovery.confirms(&[(0, words[0]), (11, words[11])]));
    // The team's six words, and one administrator (the interface recommends a second).
    let six = alice.fingerprint().unwrap().words();
    assert_eq!(six.len(), 6);
    assert_eq!(alice.admins().unwrap(), 1);
    assert!(alice.escrow().unwrap().is_none());

    // The invitation: 72 h, one use, a link whose secret is after the `#`.
    let invited = alice
        .invite_for(
            Purpose::NewMember(Role::Member),
            NOW,
            escouade_team::crypto::invite::DEFAULT_LIFETIME,
        )
        .await
        .unwrap();
    assert!(
        invited.ready,
        "an administrator of the server registers the token at once"
    );
    assert_eq!(invited.expires_at, NOW + 72 * 3600);
    let (before, after) = invited.url.split_once('#').unwrap();
    // The app's link (SR2-08): it opens Escouade, never a page of the server.
    assert!(before.starts_with("escouade://team/join/team.example.org/"));
    assert!(after.contains("s=") && after.contains("t="));
    assert!(!before.contains(&invited.link.server_token()));

    // Bob enrols and asks; he is not in until an administrator is online.
    let link = as_text(&invited.url);
    assert_eq!(
        link.team(),
        alice.fingerprint().unwrap(),
        "the invitee checks the six words of the link"
    );
    let mut bob = Team::join_link(pb.setup(), &link).await.unwrap();
    assert!(!bob.sync_at(NOW).await.unwrap().joined);
    confirm(&mut alice, &mut bob, NOW).await;
    assert_eq!(bob.fingerprint().unwrap(), link.team());
    assert_eq!(
        bob.roster().unwrap().role(&bob.identity().member),
        Some(Role::Member)
    );

    // Someone else with the same link: « invitation déjà utilisée »; the administrator is told.
    match Team::join_link(pm.setup(), &link).await {
        Err(Error::InvitationUsed) => {}
        other => panic!("a second use: {:?}", other.err()),
    }
    let served = online(&mut alice, NOW).await;
    assert_eq!(served.alerts, 1);
    let inv = alice
        .invitations()
        .into_iter()
        .find(|i| i.id == link.id)
        .unwrap();
    assert!(inv.used && inv.ready && inv.reused_at.is_some());
    // Told once.
    assert_eq!(online(&mut alice, NOW).await.alerts, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_invitation_expires_on_the_server_and_in_the_team() {
    let w = World::new().await;
    let (pa, pb, pc) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    // On the server: past its time, the link is unknown there.
    let short = alice
        .invite_for(Purpose::NewMember(Role::Member), NOW, 1)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(2100)).await;
    assert!(matches!(
        Team::join_link(pb.setup(), &as_text(&short.url)).await,
        Err(Error::InvitationUnknown)
    ));
    // In the team: an administrator online after its expiry does not let the invitee in.
    let invited = alice
        .invite_for(Purpose::NewMember(Role::Member), NOW, 3600)
        .await
        .unwrap();
    let mut carol = Team::join_link(pc.setup(), &as_text(&invited.url))
        .await
        .unwrap();
    assert_eq!(online(&mut alice, NOW + 7200).await.waiting, 0);
    assert!(!carol.sync_at(NOW).await.unwrap().joined);
    assert!(alice.requests().is_empty(), "the late request is dropped");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_server_roles_follow_the_roster() {
    let w = World::new().await;
    let (pa, pb, pc) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    // A member is not an administrator of the server: his invitation of a new member is refused by the team.
    assert!(matches!(
        bob.invite_for(Purpose::NewMember(Role::Member), NOW, 3600)
            .await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    // Named administrator in the roster, then on the server by Alice's device.
    alice.set_role(member(2), Role::Admin).await.unwrap();
    let served = online(&mut alice, NOW).await;
    assert_eq!(served.roles, 1);
    let devices = alice.client().devices().await.unwrap();
    let on_server = devices
        .iter()
        .find(|d| d.id == bob.identity().device.0)
        .unwrap();
    assert_eq!(on_server.role, wire::Role::Admin);
    // Bob now registers his invitations' tokens himself, and lets in whom he invites.
    bob.sync_at(NOW).await.unwrap();
    let invited = bob
        .invite_for(Purpose::NewMember(Role::Member), NOW, 3600)
        .await
        .unwrap();
    assert!(invited.ready);
    let mut carol = Team::join_link(pc.setup(), &as_text(&invited.url))
        .await
        .unwrap();
    confirm(&mut bob, &mut carol, NOW).await;
    // Back to member: the server follows.
    alice.set_role(member(2), Role::Member).await.unwrap();
    assert_eq!(online(&mut alice, NOW).await.roles, 1);
    let devices = alice.client().devices().await.unwrap();
    let on_server = devices
        .iter()
        .find(|d| d.id == bob.identity().device.0)
        .unwrap();
    assert_eq!(on_server.role, wire::Role::Member);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_member_adds_a_device_of_their_own_by_the_qr_codes_link() {
    let w = World::new().await;
    let (pa, pb, pb2) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    // Bob's QR code: a link of his own device, with his member id. Not an administrator of the server, he asks one
    // for its token in the stream.
    let invited = bob
        .invite_for(Purpose::OwnDevice(member(2)), NOW, 3600)
        .await
        .unwrap();
    assert!(!invited.ready);
    let link = as_text(&invited.url);
    assert_eq!(link.member, Some(member(2).0));
    assert!(matches!(
        Team::join_link(pb2.setup(), &link).await,
        Err(Error::InvitationUnknown)
    ));
    // An administrator online registers it, and says so.
    assert_eq!(online(&mut alice, NOW).await.granted, 1);
    bob.sync_at(NOW).await.unwrap();
    assert!(bob.invitations().iter().any(|i| i.id == link.id && i.ready));
    // The new device comes; Bob's other device lets it in, not an administrator.
    let mut bob2 = Team::join_link(pb2.setup(), &link).await.unwrap();
    confirm(&mut bob, &mut bob2, NOW).await;
    assert_eq!(bob2.identity().member, member(2));
    let members = alice.members().ok();
    alice.sync_at(NOW).await.unwrap();
    let row = alice
        .members()
        .unwrap()
        .into_iter()
        .find(|(m, _, _)| *m == member(2))
        .unwrap();
    assert_eq!(row.2.len(), 2, "{members:?}");
    // A token request for another member's device is refused by the administrator.
    let forged = bob
        .invite_for(Purpose::OwnDevice(member(1)), NOW, 3600)
        .await;
    assert!(matches!(forged, Err(Error::Crypto(CryptoError::NotAdmin))));
}

/// The device in the team steps its pairing until it ends.
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_short_code_adds_a_device_and_a_wrong_one_spends_its_only_try() {
    let w = World::new().await;
    let (pa, pa2, pm) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;

    // A wrong code typed: the device in the team sees it, closes the slot; the new device is told; the right code,
    // typed then, finds nothing left.
    let mut pairing = alice.pair_start(NOW).await.unwrap();
    let right = PairCode::parse(&pairing.code.shown()).unwrap();
    let wrong_text = {
        let shown = pairing.code.shown();
        let last = if shown.ends_with('0') { '1' } else { '0' };
        format!("{}{last}", &shown[..shown.len() - 1])
    };
    let wrong = PairCode::parse(&wrong_text).unwrap();
    let transport = pa2.liar.clone();
    let newcomer = tokio::spawn(async move {
        pairing::fetch_link(&*transport, &wrong, Duration::from_secs(20)).await
    });
    assert_eq!(
        step_until_done(&mut alice, &mut pairing).await,
        PairState::WrongCode
    );
    assert!(matches!(newcomer.await.unwrap(), Err(Error::WrongCode)));
    assert!(matches!(
        pairing::fetch_link(&*pa2.liar, &right, Duration::from_secs(2)).await,
        Err(Error::CodeGone)
    ));

    // One try per slot, whoever makes it: a guess first, and the right code after it is refused.
    let mut pairing = alice.pair_start(NOW).await.unwrap();
    let right = PairCode::parse(&pairing.code.shown()).unwrap();
    let guess = PairCode::parse(&format!("{}ZZZZZ", pairing.code.slot)).unwrap();
    let transport = pm.liar.clone();
    let guesser = tokio::spawn(async move {
        pairing::fetch_link(&*transport, &guess, Duration::from_millis(500)).await
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(matches!(
        pairing::fetch_link(&*pa2.liar, &right, Duration::from_secs(2)).await,
        Err(Error::CodeTried)
    ));
    assert_eq!(
        step_until_done(&mut alice, &mut pairing).await,
        PairState::WrongCode
    );
    assert!(guesser.await.unwrap().is_err());

    // The right code: the link of Alice's own device arrives sealed; the new device joins by it.
    let mut pairing = alice.pair_start(NOW).await.unwrap();
    let code = PairCode::parse(&pairing.code.shown().to_lowercase()).unwrap();
    let transport = pa2.liar.clone();
    let newcomer = tokio::spawn(async move {
        pairing::fetch_link(&*transport, &code, Duration::from_secs(20)).await
    });
    assert_eq!(
        step_until_done(&mut alice, &mut pairing).await,
        PairState::Sent
    );
    let link = newcomer.await.unwrap().unwrap();
    assert_eq!(link, pairing.link);
    assert_eq!(link.member, Some(member(1).0));
    let mut alice2 = Team::join_link(pa2.setup(), &link).await.unwrap();
    confirm(&mut alice, &mut alice2, NOW).await;
    assert_eq!(alice2.identity().member, member(1));
    assert!(alice2.is_admin().unwrap());
}

/// SR2-08: the link alone lets no one in. A request made with it elsewhere (the link leaked, or the server read it)
/// proves the secret, but its KeyPackage is not the invitee's: its six words are others than those of the invitee's
/// screen, and the device that invited admits it with none of the invitee's words; it admits the invitee's request.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_request_from_the_link_alone_has_other_words_than_the_invitees() {
    let w = World::new().await;
    let (pa, pb, pc) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let invited = alice
        .invite_for(Purpose::NewMember(Role::Member), NOW, 3600)
        .await
        .unwrap();
    let mut carol = Team::join_link(pc.setup(), &as_text(&invited.url))
        .await
        .unwrap();
    // Another request with the same link, posted in the team's stream as the server could: its MAC passes.
    let mut other = escouade_team::crypto::TeamDevice::new(member(9)).unwrap();
    let request = other.request_join(&as_text(&invited.url)).unwrap();
    bob.client()
        .append_message(&wire::encode(&TeamMessage::JoinRequest(request.clone())))
        .await
        .unwrap();

    assert_eq!(online(&mut alice, NOW).await.waiting, 2);
    let carols = carol.join_words().unwrap();
    let shown = alice.admissions(NOW);
    assert_eq!(shown.len(), 2);
    assert!(shown.iter().any(|a| a.code.words() == carols));
    let others = other.join_code().unwrap().words();
    assert_ne!(others, carols, "other words than the invitee's");
    // Carol's words for the other request: refused, nothing admitted, the invitation still open.
    assert!(matches!(
        alice.admit(&request, &carols.join(" "), NOW).await,
        Err(Error::Crypto(CryptoError::Invitation(
            InvitationError::CodeMismatch
        )))
    ));
    assert!(!carol.sync_at(NOW).await.unwrap().joined);
    assert!(alice
        .invitations()
        .iter()
        .any(|i| i.id == invited.link.id && !i.used));
    // Words nobody showed: refused too.
    assert!(matches!(
        alice
            .admit_invitation(
                invited.link.id,
                "abandon ability able about above absent",
                NOW
            )
            .await,
        Err(Error::Crypto(CryptoError::Invitation(
            InvitationError::CodeMismatch
        )))
    ));
    // Carol's words: Carol's request, Carol in; the other request has no invitation left.
    alice
        .admit_invitation(invited.link.id, &carols.join(" "), NOW)
        .await
        .unwrap();
    assert!(carol.sync_at(NOW).await.unwrap().joined);
    assert_eq!(
        alice.roster().unwrap().role(&carol.identity().member),
        Some(Role::Member)
    );
    assert_eq!(online(&mut alice, NOW).await.waiting, 0);
    assert!(alice.admissions(NOW).is_empty());
}

/// SR2-08: the words differ (the invitee's screen shows none, or others): the request is refused and the invitation
/// cancelled; the device that asked is cut on the server, and the other administrators are told.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn different_words_refuse_the_request_and_cancel_the_invitation() {
    let w = World::new().await;
    let (pa, pd, pm, pc) = (w.place(), w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Admin).await;
    online(&mut alice, NOW).await;
    let invited = alice
        .invite_for(Purpose::NewMember(Role::Member), NOW, 3600)
        .await
        .unwrap();
    // Whoever has the link comes first; the invitee, after, is told it was used, and shows no words.
    let mut first = Team::join_link(pm.setup(), &as_text(&invited.url))
        .await
        .unwrap();
    assert!(matches!(
        Team::join_link(pc.setup(), &as_text(&invited.url)).await,
        Err(Error::InvitationUsed)
    ));
    assert_eq!(online(&mut alice, NOW).await.waiting, 1);
    assert_eq!(alice.admissions(NOW).len(), 1);

    alice.refuse(invited.link.id, NOW).await.unwrap();
    let inv = alice
        .invitations()
        .into_iter()
        .find(|i| i.id == invited.link.id)
        .unwrap();
    assert!(inv.used && inv.refused);
    assert!(alice.admissions(NOW).is_empty());
    assert!(alice.requests().is_empty());
    // Not admitted afterwards, whatever the words.
    let words = first.join_words().unwrap().join(" ");
    assert!(alice
        .admit_invitation(invited.link.id, &words, NOW)
        .await
        .is_err());
    assert_eq!(online(&mut alice, NOW).await.waiting, 0);
    // The device that asked is cut on the server: it reads nothing more.
    assert!(first.sync_at(NOW).await.is_err());
    // Told to this device and to the other administrators.
    assert_eq!(alice.refusals().len(), 1);
    dave.sync_at(NOW).await.unwrap();
    let told = dave.refusals();
    assert_eq!(told.len(), 1);
    assert_eq!(told[0].invitation, invited.link.id);
    assert_eq!(told[0].by, member(1));
    assert_eq!(told[0].purpose, Purpose::NewMember(Role::Member));
    // An invitation of another device, or none: refused.
    assert!(dave.refuse(invited.link.id, NOW).await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_escrow_key_is_shown_to_every_member_and_reads_the_content() {
    let w = World::new().await;
    let (pa, pb) = (w.place(), w.place());
    let provider = OpenMlsRustCrypto::default();
    let escrow = EscrowKey::generate(provider.crypto(), provider.rand(), "le RSSI d'ACME").unwrap();
    let token = w.server.server.new_team_token().await.unwrap();
    let (mut alice, _) = Team::create(pa.setup(), &token, member(1), Some(escrow.team_half()), NOW)
        .await
        .unwrap();
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    // Shown to every member.
    assert_eq!(bob.escrow().unwrap().unwrap().holder, "le RSSI d'ACME");
    bob.write_object(PROJECT, ObjectId([1; 16]), 1, b"ticket ESC-9")
        .await
        .unwrap();

    // The organisation reads it, with the escrow key and what the server keeps.
    let group = alice.device().group_id().unwrap();
    let epoch = bob.epoch().unwrap();
    let stream = alice.client().read_team(0, None).await.unwrap();
    let wrap = stream
        .entries
        .iter()
        .filter(|e| e.kind == Kind::Message)
        .filter_map(|e| match wire::decode::<TeamMessage>(&e.payload) {
            Ok(TeamMessage::Escrow(w)) if w.epoch == epoch => Some(w),
            _ => None,
        })
        .next()
        .expect("the escrow wrap of the epoch");
    let team = alice.fingerprint().unwrap();
    let record = escrow
        .open(provider.crypto(), &team, &group, &wrap)
        .unwrap();
    let ops = alice
        .client()
        .read_project(&PROJECT.0, 0, None)
        .await
        .unwrap();
    let sealed: SealedObject = wire::decode(&ops.entries[0].payload).unwrap();
    assert_eq!(sealed.open(&record).unwrap(), b"ticket ESC-9");

    // Taken away: every member sees it gone.
    alice.set_escrow(None).await.unwrap();
    bob.sync_at(NOW).await.unwrap();
    assert!(bob.escrow().unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_recovery_key_brings_the_team_back_through_the_server() {
    let w = World::new().await;
    let (pa, pb, rescue, carol_place, other) =
        (w.place(), w.place(), w.place(), w.place(), w.place());
    let token = w.server.server.new_team_token().await.unwrap();
    let (mut alice, recovery) = Team::create(pa.setup(), &token, member(1), None, NOW)
        .await
        .unwrap();
    let words = recovery.words().join(" ");
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    alice
        .write_object(PROJECT, ObjectId([1; 16]), 1, b"before the loss")
        .await
        .unwrap();
    let team = alice.id();
    let six = alice.fingerprint().unwrap();
    let six_words = six.words().join(" ");
    // Alice loses her only device: the team has no administrator left.
    drop(alice);

    // A word wrong: refused before the token is spent.
    let token = w.server.server.new_recovery_token(team).await.unwrap();
    let typo = words.replacen(recovery.words()[3], "zoo", 1);
    assert!(matches!(
        Team::recover(rescue.setup(), &token, &typo, &six_words).await,
        Err(Error::Crypto(CryptoError::Words))
    ));
    // The six words of the team are asked too (a sheet printed without them cannot recover).
    assert!(matches!(
        Team::recover(rescue.setup(), &token, &words, "").await,
        Err(Error::Crypto(CryptoError::OtherTeam))
    ));

    // The words, through the server: the recovery leaf comes back, lets Carol's new device in as an administrator,
    // which gives the team a new recovery key; the old one's leaf leaves.
    let (mut carol, new_recovery) = Team::recover_onto(
        rescue.setup(),
        carol_place.setup(),
        &token,
        &words,
        &six_words,
        NOW,
    )
    .await
    .unwrap();
    assert_eq!(carol.id(), team);
    assert_eq!(carol.fingerprint().unwrap(), six, "the same team");
    assert!(carol.is_admin().unwrap());
    assert_ne!(new_recovery.words(), recovery.words());
    let read = carol.read_objects(PROJECT, 0).await.unwrap();
    assert_eq!(read[0].1.as_ref().unwrap(), b"before the loss");
    assert!(carol
        .device()
        .leaves()
        .unwrap()
        .iter()
        .all(|l| l.kind == escouade_team::crypto::DeviceKind::Device));
    // Carol is an administrator of the server: she invites.
    assert!(
        carol
            .invite_for(Purpose::NewMember(Role::Member), NOW, 3600)
            .await
            .unwrap()
            .ready
    );
    // Bob follows.
    bob.sync_at(NOW).await.unwrap();
    assert_eq!(bob.roster().unwrap(), carol.roster().unwrap());
    assert!(
        rescue.keychain.names().is_empty(),
        "the leaf's keys are forgotten"
    );

    // The old words open nothing now.
    let token = w.server.server.new_recovery_token(team).await.unwrap();
    assert!(Team::recover(other.setup(), &token, &words, &six_words)
        .await
        .is_err());
}

/// SR2-09: a wrap that a member writes in the stream, sealed to the team's recovery key, for a group of the member's
/// own with the team's id and genesis, is passed over; the recovery lands in the team's own group.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_recovery_passes_over_a_wrap_no_administrator_anchored() {
    use escouade_team::crypto::anchor::Anchor;
    use escouade_team::crypto::recovery::{RecoveryWrap, WrapAuthor};
    use escouade_team::crypto::TeamDevice;
    let w = World::new().await;
    let (pa, pb, pr) = (w.place(), w.place(), w.place());
    let token = w.server.server.new_team_token().await.unwrap();
    let (mut alice, recovery) = Team::create(pa.setup(), &token, member(1), None, NOW)
        .await
        .unwrap();
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    bob.sync().await.unwrap();
    let real = alice.device().group_id().unwrap();
    let six = alice.fingerprint().unwrap();

    // Bob's group: the team's id and genesis (the same six words), the team's recovery key, Bob its administrator.
    let roster = bob.roster().unwrap();
    let mut forged = roster.clone();
    forged.members.insert(member(2), Role::Admin);
    let made_up = Anchor::from_seed([0x5a; 32]);
    forged.recovery.anchor_key = made_up.public();
    let mut fake = TeamDevice::new(member(2)).unwrap();
    fake.create_team(NOW, None).unwrap();
    let wrap = {
        let (provider, signer, group) = fake.mls();
        let _ = group;
        let group = openmls::prelude::MlsGroup::builder()
            .with_group_id(openmls::prelude::GroupId::from_slice(&real))
            .ciphersuite(escouade_team::crypto::CIPHERSUITE)
            .use_ratchet_tree_extension(true)
            .with_group_context_extensions(forged.to_extensions().unwrap())
            .with_capabilities(escouade_team::crypto::device::capabilities())
            .build(
                provider,
                signer,
                openmls::prelude::CredentialWithKey {
                    credential: escouade_team::crypto::Identity {
                        member: member(2),
                        device: escouade_team::crypto::DeviceId([0x42; 16]),
                        kind: escouade_team::crypto::DeviceKind::Device,
                    }
                    .credential(),
                    signature_key: signer.to_public_vec().into(),
                },
            )
            .unwrap();
        use openmls::prelude::tls_codec::Serialize as _;
        let info = group
            .export_group_info(provider.crypto(), signer, true)
            .unwrap()
            .tls_serialize_detached()
            .unwrap();
        let by = WrapAuthor {
            group_id: &real,
            team: six,
            leaf: signer,
            anchor: &made_up,
        };
        let record = bob.device().records().last().unwrap().clone();
        RecoveryWrap::seal(
            provider.crypto(),
            &forged.recovery,
            &by,
            &record,
            info,
            None,
        )
        .unwrap()
    };
    bob.client()
        .append_message(&wire::encode(&TeamMessage::Recovery {
            group: real.clone(),
            wrap,
        }))
        .await
        .unwrap();
    // Bob's device does not count it as the epoch's wrap.
    bob.sync().await.unwrap();

    // Every administrator lost their devices: the words and the six words bring the team back, its own group.
    drop(alice);
    let rtoken = w.server.server.new_recovery_token(bob.id()).await.unwrap();
    let words = recovery.words().join(" ");
    let back = Team::recover(pr.setup(), &rtoken, &words, &six.words().join(" "))
        .await
        .unwrap();
    assert_eq!(back.device().group_id().unwrap(), real);
    assert_eq!(back.fingerprint().unwrap(), six);
    bob.sync().await.unwrap();
    assert_eq!(bob.epoch().unwrap(), back.epoch().unwrap(), "Bob follows");
}

/// SR2-09: the anchor reaches a second administrator's device through the stream (never a member's), and when a
/// member's commit moves the team on, an administrator's device writes the wrap of the new epoch: the recovery
/// still works once every administrator is gone.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_anchor_reaches_the_administrators_and_the_wraps_follow_a_members_commit() {
    let w = World::new().await;
    let (pa, pd, pb, pr) = (w.place(), w.place(), w.place(), w.place());
    let token = w.server.server.new_team_token().await.unwrap();
    let (mut alice, recovery) = Team::create(pa.setup(), &token, member(1), None, NOW)
        .await
        .unwrap();
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Admin).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let anchor = recovery.anchor().public();
    online(&mut alice, NOW).await;
    dave.sync_at(NOW).await.unwrap();
    bob.sync_at(NOW).await.unwrap();
    assert!(dave.device().holds_anchor(&anchor), "Dave got the anchor");
    assert!(!bob.device().holds_anchor(&anchor), "Bob never does");
    assert!(!alice.recovery_stale());

    // Bob, a member, changes the keys: his commit writes no wrap; Dave's device writes the one of the epoch.
    bob.rotate().await.unwrap();
    let epoch = bob.epoch().unwrap();
    drop(alice);
    online(&mut dave, NOW).await;
    let stream = dave.client().read_team(0, None).await.unwrap();
    let latest = stream
        .entries
        .iter()
        .filter(|e| e.kind == Kind::Message)
        .filter_map(|e| match wire::decode::<TeamMessage>(&e.payload) {
            Ok(TeamMessage::Recovery { wrap, .. }) => Some(wrap.epoch),
            _ => None,
        })
        .max();
    assert_eq!(latest, Some(epoch));

    // Every administrator gone: the recovery joins at that epoch.
    let six = dave.fingerprint().unwrap().words().join(" ");
    drop(dave);
    let rtoken = w.server.server.new_recovery_token(bob.id()).await.unwrap();
    let back = Team::recover(pr.setup(), &rtoken, &recovery.words().join(" "), &six)
        .await
        .unwrap();
    bob.sync().await.unwrap();
    assert_eq!(bob.epoch().unwrap(), back.epoch().unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fork_is_an_alarm_and_the_reading_stays_stopped() {
    let w = World::new().await;
    let (pa, pd, pb) = (w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Admin).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    dave.sync().await.unwrap();
    let epoch = alice.epoch().unwrap();
    let staged = dave.device_mut().stage(&Change::Rotate).unwrap();
    let (_, dave_commit) = staged.commit.unwrap();
    dave.device_mut().abandon_pending().unwrap();
    alice.rotate().await.unwrap();
    alice.sync().await.unwrap();
    pb.liar.lies.lock().unwrap().swap_commit = Some((epoch, dave_commit));
    let e = bob.sync().await.unwrap_err();
    assert_eq!(e.alarm(), Some(Alarm::Fork { epoch: epoch + 1 }));
    // It stays: the next readings say it again, nothing is merged past it.
    let again = bob.sync().await.unwrap_err();
    assert!(again.alarm().is_some());
    // The six words of the head differ: what the members compare out of band.
    assert_ne!(bob.head_words().unwrap(), alice.head_words().unwrap());
    // A server that goes back, or rewrites what was read, is an alarm too.
    assert_eq!(
        Error::ServerRollback { seen: 3 }.alarm(),
        Some(Alarm::Rollback)
    );
    assert_eq!(Error::Rewritten { seq: 3 }.alarm(), Some(Alarm::Rewritten));
    assert_eq!(Error::InvitationUsed.alarm(), None);
}

/// SR2-09: an administrator made a member again still holds the anchor of the current words. The card says to make a
/// new recovery key; until then, a wrap the anchor signed with a member's leaf does not hide the administrators' own.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_former_administrators_wrap_does_not_hide_the_teams_own() {
    use escouade_team::crypto::recovery::{RecoveryWrap, WrapAuthor};
    use openmls::prelude::tls_codec::Serialize as _;
    let w = World::new().await;
    let (pa, pd, pr) = (w.place(), w.place(), w.place());
    let token = w.server.server.new_team_token().await.unwrap();
    let (mut alice, recovery) = Team::create(pa.setup(), &token, member(1), None, NOW)
        .await
        .unwrap();
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Admin).await;
    online(&mut alice, NOW).await;
    dave.sync_at(NOW).await.unwrap();
    assert!(dave.device().holds_anchor(&recovery.anchor().public()));
    alice.set_role(member(4), Role::Member).await.unwrap();
    assert!(
        alice.recovery_stale(),
        "a former administrator may hold the anchor"
    );
    dave.sync_at(NOW).await.unwrap();

    // Dave, a member now, signs a wrap of the team's group with the anchor and his own leaf.
    let real = alice.device().group_id().unwrap();
    let team = alice.fingerprint().unwrap();
    let roster = dave.roster().unwrap();
    let record = dave.device().records().last().unwrap().clone();
    let anchor = recovery.anchor();
    let wrap = {
        let (provider, signer, group) = dave.device_mut().mls();
        let info = group
            .export_group_info(provider.crypto(), signer, true)
            .unwrap()
            .tls_serialize_detached()
            .unwrap();
        let by = WrapAuthor {
            group_id: &real,
            team,
            leaf: signer,
            anchor: &anchor,
        };
        RecoveryWrap::seal(
            provider.crypto(),
            &roster.recovery,
            &by,
            &record,
            info,
            None,
        )
        .unwrap()
    };
    assert_eq!(wrap.epoch, alice.epoch().unwrap());
    dave.client()
        .append_message(&wire::encode(&TeamMessage::Recovery {
            group: real.clone(),
            wrap,
        }))
        .await
        .unwrap();

    // The recovery passes it over and follows Alice's wrap of the same epoch.
    let rtoken = w
        .server
        .server
        .new_recovery_token(alice.id())
        .await
        .unwrap();
    let back = Team::recover(
        pr.setup(),
        &rtoken,
        &recovery.words().join(" "),
        &team.words().join(" "),
    )
    .await
    .unwrap();
    assert_eq!(back.device().group_id().unwrap(), real);

    // A new recovery key: nobody but the administrators holds its anchor.
    alice.sync_at(NOW).await.unwrap();
    alice.replace_recovery().await.unwrap();
    assert!(!alice.recovery_stale());
}
