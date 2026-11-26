//! A scenario of `escouade-team-crypto` (`src/tests.rs`) played through the real server instead of `relay.rs`: the
//! team is an MLS group, its commits and messages go into the team's stream, sealed objects into a project's stream.
//! Add, remove, rotate; then two administrators commit at the same epoch, and the server keeps the first.
//!
//! The server sees only what it numbers: the kind of an entry (a commit and its epoch, or a message), its size, its
//! device, its hour. The envelope of the messages (`Msg`) is the client's, in the payload.

#[macro_use]
mod common;

use std::collections::BTreeMap;

use common::*;
use escouade_team_crypto::content::SealedObject;
use escouade_team_crypto::history::HistoryLink;
use escouade_team_crypto::invite::{JoinRequest, WelcomeDelivery, DEFAULT_LIFETIME};
use escouade_team_crypto::recovery::{EscrowWrap, RecoveryWrap};
use escouade_team_crypto::{
    Error, MemberId, ObjectId, Processed, ProjectId, Publication, Purpose, Role, TeamDevice,
};
use escouade_team_server::db::Kind;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

const NOW: u64 = 1_800_000_000;

/// What a client puts in a message of the team's stream: the server does not read it.
#[derive(Serialize, Deserialize)]
enum Msg {
    JoinRequest(JoinRequest),
    Welcome([u8; 16], WelcomeDelivery),
    History(HistoryLink),
    Recovery(RecoveryWrap),
    Escrow(EscrowWrap),
}

/// One member's device: its MLS device, and its device on the server (its own Ed25519 key, to sign in).
struct Member {
    mls: TeamDevice,
    server: Device,
}

/// What a device makes of the team's stream: the commits by epoch, the history links, the Welcomes and the join
/// requests.
#[derive(Default)]
struct View {
    commits: Vec<Vec<u8>>,
    history: BTreeMap<u64, HistoryLink>,
    welcomes: BTreeMap<[u8; 16], WelcomeDelivery>,
    requests: Vec<JoinRequest>,
}

async fn view(device: &Device) -> View {
    let mut out = View::default();
    let mut since = 0;
    loop {
        let page = device.read_team(since).await;
        for e in &page.entries {
            since = e.seq;
            match e.kind {
                Kind::Commit => {
                    assert_eq!(
                        e.epoch,
                        Some(out.commits.len() as u64),
                        "the commits come in epoch order"
                    );
                    out.commits.push(e.payload.clone());
                }
                Kind::Message => match serde_json::from_slice::<Msg>(&e.payload).unwrap() {
                    Msg::JoinRequest(r) => out.requests.push(r),
                    Msg::Welcome(id, w) => {
                        out.welcomes.insert(id, w);
                    }
                    Msg::History(l) => {
                        out.history.insert(l.epoch, l);
                    }
                    Msg::Recovery(_) | Msg::Escrow(_) => {}
                },
                Kind::Void => {}
            }
        }
        if !page.more {
            return out;
        }
    }
}

async fn send(device: &Device, msg: &Msg) {
    device.message(&serde_json::to_vec(msg).unwrap()).await;
}

/// Publishes what an operation gives: its commit first (refused if its epoch is taken), then the rest.
async fn publish(device: &Device, publication: Publication) -> Result<(), (StatusCode, Problem)> {
    if let Some((epoch, commit)) = publication.commit {
        // Said in clear to be of its own devices: the server takes it from a member then; the devices read the scope
        // the commit signs (SR2-02).
        if let Answer::No(s, p) = device.commit_own(epoch, &commit).await {
            return Err((s, p));
        }
    }
    if let Some((id, welcome)) = publication.welcome {
        send(device, &Msg::Welcome(id, welcome)).await;
    }
    if let Some(link) = publication.history {
        send(device, &Msg::History(link)).await;
    }
    if let Some(wrap) = publication.recovery {
        send(device, &Msg::Recovery(wrap)).await;
    }
    if let Some(wrap) = publication.escrow {
        send(device, &Msg::Escrow(wrap)).await;
    }
    Ok(())
}

async fn sync(member: &mut Member) {
    let v = view(&member.server).await;
    member.mls.sync(&v.commits).unwrap();
}

/// The invitation, end to end: the administrator's link and its server token, the invitee's join request through
/// the server, the admission, the Welcome and the history read back from the server.
async fn join(s: &TestServer, admin: &mut Member, who: MemberId) -> Member {
    let link = admin
        .mls
        .invite(
            Purpose::NewMember(Role::Member),
            NOW,
            DEFAULT_LIFETIME,
            "team.example.org",
        )
        .unwrap();
    let token = admin.server.enrollment_token().await;
    let mut server = Device::enroll(s, &token).await.unwrap();
    server.sign_in().await.unwrap();
    let mut mls = TeamDevice::new(who).unwrap();
    send(&server, &Msg::JoinRequest(mls.request_join(&link).unwrap())).await;

    let request = view(&admin.server)
        .await
        .requests
        .pop()
        .expect("the admin sees the request");
    // The six words the invitee shows, confirmed by the person who invites (SR2-08).
    let words = mls.join_code().unwrap().words().join(" ");
    publish(
        &admin.server,
        admin.mls.admit(&request, &words, NOW).unwrap(),
    )
    .await
    .unwrap();

    let v = view(&server).await;
    mls.accept_welcome(&v.welcomes[&link.id]).unwrap();
    mls.read_history(|e| v.history.get(&e).cloned()).unwrap();
    Member { mls, server }
}

async fn seal_to(member: &Member, project: ProjectId, object: u8, text: &[u8]) {
    let sealed = member
        .mls
        .seal(project, ObjectId([object; 16]), 1, text)
        .unwrap();
    member
        .server
        .append_project(&project.0, &serde_json::to_vec(&sealed).unwrap())
        .await
        .ok();
}

async fn objects(device: &Device, project: ProjectId) -> Vec<SealedObject> {
    device
        .read_project(&project.0, 0)
        .await
        .entries
        .iter()
        .map(|e| serde_json::from_slice(&e.payload).unwrap())
        .collect()
}

async fn scenario(b: Backend) {
    let s = TestServer::start(&b).await;
    let project = ProjectId([7; 16]);

    // Alice creates the team, on the server and in MLS.
    let mut alice = Member {
        server: s.new_team().await,
        mls: TeamDevice::new(MemberId([1; 16])).unwrap(),
    };
    let (_recovery, genesis) = alice.mls.create_team(NOW, None).unwrap();
    publish(&alice.server, genesis).await.unwrap();

    // Add: Bob joins (epoch 1), reads what Alice seals, and she what he seals.
    let mut bob = join(&s, &mut alice, MemberId([2; 16])).await;
    assert_eq!(bob.mls.epoch().unwrap(), 1);
    assert_eq!(
        bob.mls.fingerprint().unwrap(),
        alice.mls.fingerprint().unwrap()
    );
    seal_to(&alice, project, 1, b"ticket ESC-1").await;
    seal_to(&bob, project, 2, b"spec page").await;
    let objs = objects(&bob.server, project).await;
    assert_eq!(bob.mls.open(&objs[0]).unwrap(), b"ticket ESC-1");
    assert_eq!(alice.mls.open(&objs[1]).unwrap(), b"spec page");

    // Carol joins (epoch 2); Bob follows the commit from the server.
    let mut carol = join(&s, &mut alice, MemberId([3; 16])).await;
    sync(&mut bob).await;
    assert_eq!(bob.mls.epoch().unwrap(), 2);
    // Carol reads all the history, from before she came.
    let objs = objects(&carol.server, project).await;
    assert_eq!(carol.mls.open(&objs[0]).unwrap(), b"ticket ESC-1");

    // Remove: Alice removes Bob (epoch 3), and revokes his device on the server.
    publish(
        &alice.server,
        alice.mls.remove_member(MemberId([2; 16])).unwrap(),
    )
    .await
    .unwrap();
    let v = view(&bob.server).await;
    assert_eq!(
        bob.mls.process_commit(&v.commits[2]).unwrap(),
        Processed::Removed
    );
    assert_eq!(
        alice.server.revoke(&bob.server).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        bob.server
            .client
            .status(reqwest::Method::GET, "/v1/team/stream", None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    sync(&mut carol).await;
    assert_eq!(carol.mls.epoch().unwrap(), 3);

    // Rotate: Carol is no admin, Alice rotates (epoch 4); Carol follows.
    publish(&alice.server, alice.mls.rotate().unwrap())
        .await
        .unwrap();
    sync(&mut carol).await;
    assert_eq!(carol.mls.epoch().unwrap(), 4);
    seal_to(&alice, project, 3, b"after Bob").await;
    let objs = objects(&carol.server, project).await;
    let last = objs.last().unwrap();
    assert_eq!(carol.mls.open(last).unwrap(), b"after Bob");
    // Bob, given the bytes some other way, opens nothing new.
    assert!(bob.mls.open(last).is_err());

    // Two administrators commit at the same epoch: the server keeps the first, refuses the second with it.
    let mut dave = join(&s, &mut alice, MemberId([4; 16])).await;
    sync(&mut carol).await;
    publish(
        &alice.server,
        alice.mls.set_role(MemberId([4; 16]), Role::Admin).unwrap(),
    )
    .await
    .unwrap();
    sync(&mut carol).await;
    sync(&mut dave).await;
    let epoch = alice.mls.epoch().unwrap();
    assert_eq!(
        (carol.mls.epoch().unwrap(), dave.mls.epoch().unwrap()),
        (epoch, epoch)
    );
    let from_alice = alice.mls.rotate().unwrap();
    let from_dave = dave.mls.rotate().unwrap();
    let alice_commit = from_alice.commit.as_ref().unwrap().1.clone();
    publish(&alice.server, from_alice).await.unwrap();
    let (status, p) = publish(&dave.server, from_dave).await.unwrap_err();
    assert_eq!(
        (status, p.error.as_str()),
        (StatusCode::CONFLICT, "epoch_taken")
    );
    let detail: escouade_team_server::api::wire::EpochTaken =
        p.detail.unwrap().deserialized().unwrap();
    assert_eq!(detail.expected_epoch, epoch + 1);
    assert_eq!(detail.preceding[0].payload, alice_commit);
    // Every member who follows the server is on Alice's branch: no fork through it. Dave, who merged his own commit
    // before the server refused it, is on another branch, and what crosses shows it.
    sync(&mut carol).await;
    assert_eq!(carol.mls.epoch().unwrap(), epoch + 1);
    seal_to(&alice, project, 4, b"one branch").await;
    let objs = objects(&dave.server, project).await;
    assert!(matches!(
        dave.mls.open(objs.last().unwrap()),
        Err(Error::Fork { .. })
    ));
    assert_eq!(carol.mls.open(objs.last().unwrap()).unwrap(), b"one branch");

    // The server kept only what it cannot read.
    let all = view(&alice.server).await;
    assert_eq!(all.commits.len() as u64, epoch + 1);
    s.finish().await;
}

both!(scenario);
