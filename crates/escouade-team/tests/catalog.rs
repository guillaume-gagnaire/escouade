//! TS-8 against the real `escouade-team-server`: the team's list of projects, encrypted under the team's key. An
//! administrator adds a project, changes it and takes it out, a member reads it; the server reads neither its name
//! nor its addresses; a member's write is refused by the client, by the server and at the reading; a project with a
//! dangerous address is refused even signed by an administrator; a clone is known by the address of its repository
//! whatever its form; the projects of an administrator who left are written again for a device that comes later; a
//! lying server that hides or changes an entry is caught.

mod common;

use common::*;
use escouade_team::catalog::{
    self, Access, Alarm, Catalog, Entry, Listed, Refused, Repo, CATALOG_STREAM,
};
use escouade_team::crypto::Role;
use escouade_team::proposals::{self, PROPOSAL_STREAM};
use escouade_team::settings::SETTINGS_STREAM;
use escouade_team::{CryptoError, Error, Team};

fn lies(p: &Place) -> std::sync::MutexGuard<'_, Lies> {
    let mut l = p.liar.lies.lock().unwrap();
    l.project = Some(hex::encode(CATALOG_STREAM.0));
    l
}

fn api() -> Listed {
    Listed {
        name: "Acme API".into(),
        color: "#3b82f6".into(),
        logo: Some("data:image/png;base64,iVBORw0KGgo=".into()),
        repos: vec![Repo {
            url: "https://github.com/acme/api.git".into(),
            folder: String::new(),
        }],
        branch: "main".into(),
        access: Access::Team,
        board: String::new(),
    }
}

fn platform() -> Listed {
    Listed {
        name: "Plateforme".into(),
        repos: vec![
            Repo {
                url: "git@github.com:acme/front.git".into(),
                folder: "front".into(),
            },
            Repo {
                url: "ssh://git@gitlab.com/acme/infra".into(),
                folder: "infra".into(),
            },
        ],
        branch: "develop".into(),
        ..Listed::default()
    }
}

async fn sync(t: &mut Team) -> Catalog {
    catalog::sync(t).await.expect("the list is read")
}

async fn put(t: &mut Team, id: Option<&str>, p: Listed) -> String {
    catalog::put(t, id, p, NOW)
        .await
        .expect("the project is written")
        .0
}

/// Every byte the server keeps on its disk.
fn server_bytes(w: &World) -> Vec<u8> {
    fn walk(dir: &std::path::Path, out: &mut Vec<u8>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if let Ok(b) = std::fs::read(&p) {
                out.extend(b);
            }
        }
    }
    let mut out = Vec::new();
    walk(&w.server.folder, &mut out);
    out
}

fn holds(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

/// Alice (administrator, the owner) and Bob (a member).
async fn two(w: &World) -> (Place, Team, Place, Team) {
    let (pa, pb) = (w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    (pa, alice, pb, bob)
}

async fn an_administrator_keeps_the_list_and_a_member_reads_it(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, _pb, mut bob) = two(&w).await;
    assert!(sync(&mut bob).await.projects.is_empty());

    let id = put(&mut alice, None, api()).await;
    let plat = put(&mut alice, None, platform()).await;
    let list = sync(&mut bob).await;
    assert_eq!(list.projects.len(), 2);
    let got = list.get(&id).expect("the project is in Bob's list");
    assert_eq!(got.project, api());
    assert_eq!(got.by, member(1).to_string());
    assert_eq!(
        list.get(&plat).unwrap().project.folders(),
        vec!["front", "infra"]
    );
    // By name.
    assert_eq!(list.projects[0].project.name, "Acme API");

    // The server reads nothing of it: no name, no address, no branch, no host in what it keeps.
    // The base is read as a file: SQLite's (PostgreSQL keeps the same rows, out of the server's folder).
    if matches!(backend, Backend::Sqlite) {
        let disk = server_bytes(&w);
        assert!(!disk.is_empty());
        for clear in [
            "Acme API",
            "Plateforme",
            "acme/api",
            "github.com",
            "gitlab.com",
            "develop",
            "#3b82f6",
        ] {
            assert!(
                !holds(&disk, clear),
                "the server keeps « {clear} » in the clear"
            );
        }
    }

    // Changed: the whole project, by its id.
    let mut changed = api();
    changed.name = "Acme API v2".into();
    changed.branch = "release".into();
    assert_eq!(put(&mut alice, Some(&id), changed.clone()).await, id);
    let list = sync(&mut bob).await;
    assert_eq!(list.projects.len(), 2);
    assert_eq!(list.get(&id).unwrap().project, changed);

    // Taken out.
    catalog::remove(&mut alice, &plat, NOW).await.unwrap();
    let list = sync(&mut bob).await;
    assert_eq!(list.projects.len(), 1);
    assert!(list.get(&plat).is_none());
    assert!(list.refusals.is_empty());

    // Read again from the disk, offline: the same list.
    assert_eq!(catalog::view(&bob).unwrap(), list);
    w.finish().await;
}

async fn a_member_cannot_write_the_list(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, pb, mut bob) = two(&w).await;
    // The client refuses it before it leaves.
    assert!(matches!(
        catalog::put(&mut bob, None, api(), NOW).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    assert!(matches!(
        catalog::remove(&mut bob, &"00".repeat(16), NOW).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    // Past the client's check, sealed by Bob's leaf: the server refuses to keep it.
    let entry = Entry::Put {
        id: [7; 16],
        project: api(),
        at: NOW,
    };
    let payload = catalog::seal(&bob, &entry).unwrap();
    let refused = bob
        .client()
        .append_project(&CATALOG_STREAM.0, &payload)
        .await
        .expect_err("the server refuses a member's entry");
    assert_eq!(refused.status(), Some(403));

    // A server that keeps it anyway (or plays it to the others): every device refuses it at the reading.
    put(&mut alice, None, platform()).await;
    let seq = alice
        .client()
        .read_project(&CATALOG_STREAM.0, 0, None)
        .await
        .unwrap()
        .entries
        .last()
        .unwrap()
        .seq;
    lies(&pb).project_inject = vec![(seq + 1, payload)];
    let list = sync(&mut bob).await;
    assert_eq!(list.projects.len(), 1, "only Alice's project");
    assert_eq!(list.projects[0].project.name, "Plateforme");
    assert!(list
        .refusals
        .iter()
        .any(|r| r.seq == seq + 1 && r.reason == Refused::NotAdmin));
    w.finish().await;
}

async fn a_dangerous_address_is_refused_even_signed(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, _pb, mut bob) = two(&w).await;
    for bad in [
        "file:///c/src/api",
        "--upload-pack=calc.exe",
        "ext::sh -c id",
        "C:\\src\\api",
        "/srv/git/api",
        "http://github.com/acme/api",
        "https://ghp_secret@github.com/acme/api",
        "https://github.com/acme/-oProxyCommand=calc",
    ] {
        let mut p = api();
        p.repos[0].url = bad.into();
        assert!(
            matches!(
                catalog::put(&mut alice, None, p, NOW).await,
                Err(Error::Refused(_))
            ),
            "{bad} is refused at the writing"
        );
    }
    // An administrator's client that skips the check (or a list written by another tool): refused at the reading.
    let mut p = api();
    p.repos[0].url = "file:///c/src/api".into();
    let payload = catalog::seal(
        &alice,
        &Entry::Put {
            id: [9; 16],
            project: p,
            at: NOW,
        },
    )
    .unwrap();
    let seq = alice
        .client()
        .append_project(&CATALOG_STREAM.0, &payload)
        .await
        .unwrap();
    let list = sync(&mut bob).await;
    assert!(list.projects.is_empty());
    assert!(matches!(
        &list.refusals[..],
        [r] if r.seq == seq && matches!(r.reason, Refused::Bad { .. })
    ));
    w.finish().await;
}

async fn a_clone_is_known_by_the_address_of_its_repository(backend: Backend) {
    let w = World::on(&backend).await;
    let (_pa, mut alice, _pb, mut bob) = two(&w).await;
    let id = put(&mut alice, None, api()).await;
    let plat = put(&mut alice, None, platform()).await;
    let list = sync(&mut bob).await;
    for local in [
        "https://github.com/acme/api",
        "https://github.com/acme/api.git",
        "https://GITHUB.com/acme/api/",
        "git@github.com:acme/api.git",
        "git@github.com:acme/api",
        "ssh://git@github.com/acme/api.git",
        "ssh://git@github.com:22/Acme/Api",
    ] {
        let (item, n) = list
            .holding(local)
            .unwrap_or_else(|| panic!("{local} is Acme API"));
        assert_eq!((item.id.as_str(), n), (id.as_str(), 0), "{local}");
    }
    // A repository of a project of several: its index says which.
    let (item, n) = list.holding("https://gitlab.com/acme/infra.git").unwrap();
    assert_eq!((item.id.as_str(), n), (plat.as_str(), 1));
    // Another repository, another host, a local path: no project of the list.
    for other in [
        "https://github.com/acme/web",
        "https://gitlab.com/acme/api",
        "C:/src/api",
        "file:///c/src/api",
    ] {
        assert!(list.holding(other).is_none(), "{other}");
    }
    w.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_projects_of_an_administrator_who_left_are_written_again() {
    let w = World::new().await;
    let (pa, pb, pc, pd) = (w.place(), w.place(), w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let mut bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let mut carol = w.join(&mut alice, &pc, member(3), Role::Admin).await;
    bob.sync().await.unwrap();
    // The server learns Carol's role from an administrator's device (`serve`, as the core does at each reading).
    alice.serve(NOW).await.unwrap();
    // Carol adds a project, then leaves the team.
    let id = put(&mut carol, None, api()).await;
    assert!(sync(&mut bob).await.get(&id).is_some());
    sync(&mut alice).await;
    alice.remove_member(member(3)).await.unwrap();
    bob.sync().await.unwrap();
    // Bob lived her epoch: her project stays in his list.
    assert!(sync(&mut bob).await.get(&id).is_some());
    // Dave comes after: Carol is no administrator of now, her entry does not count, until Alice's device writes it
    // again.
    let mut dave = w.join(&mut alice, &pd, member(4), Role::Member).await;
    let list = sync(&mut dave).await;
    assert!(list.get(&id).is_none());
    assert!(list.refusals.iter().any(|r| r.reason == Refused::NotAdmin));
    assert_eq!(catalog::serve(&mut alice, NOW).await.unwrap(), 1);
    assert_eq!(catalog::serve(&mut alice, NOW).await.unwrap(), 0, "once");
    let list = sync(&mut dave).await;
    let got = list.get(&id).expect("Dave reads Carol's project");
    assert_eq!(got.project, api());
    assert_eq!(got.by, member(1).to_string());
    // A member's device writes nothing again.
    assert_eq!(catalog::serve(&mut bob, NOW).await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_lying_server_that_hides_or_changes_an_entry_is_caught() {
    let w = World::new().await;
    let (_pa, mut alice, pb, mut bob) = two(&w).await;
    put(&mut alice, None, api()).await;
    let first = alice
        .client()
        .read_project(&CATALOG_STREAM.0, 0, None)
        .await
        .unwrap()
        .entries[0]
        .seq;
    put(&mut alice, None, platform()).await;
    assert_eq!(sync(&mut bob).await.projects.len(), 2);
    let second = first + 1;
    // It hides the second: a rollback, the reading stops, what was read stays.
    lies(&pb).project_hide_after = Some(first);
    let list = sync(&mut bob).await;
    assert_eq!(list.alarm, Some(Alarm::Rollback));
    assert_eq!(list.projects.len(), 2);
    // Nothing is written under the alarm.
    lies(&pb).project_hide_after = None;
    assert_eq!(catalog::retry(&mut bob).await.unwrap().alarm, None);
    // It changes the entry read last: rewritten.
    lies(&pb).project_flip = Some(second);
    assert_eq!(sync(&mut bob).await.alarm, Some(Alarm::Rewritten));
    lies(&pb).project_flip = None;
    assert_eq!(catalog::retry(&mut bob).await.unwrap().alarm, None);
}

/// TS-7: an entry without a Kanban (as before), then with one; a member proposes the Kanban of a project of the list,
/// an administrator reads the proposal and writes the entry; the server reads neither the board nor the proposal; the
/// team's own streams cannot be restricted by a member.
async fn an_entry_gains_its_kanban_by_an_administrator_or_a_proposal(backend: Backend) {
    let w = World::on(&backend).await;
    let (pa, mut alice, _pb, mut bob) = two(&w).await;
    let id = put(&mut alice, None, api()).await;
    let read = sync(&mut bob).await;
    assert_eq!(read.get(&id).unwrap().project.board, "", "no Kanban known");
    assert_eq!(read.get(&id).unwrap().project.board_id(), None);

    // Bob, a member, cannot write it: he proposes it.
    let board = "5a".repeat(16);
    let mut with = api();
    with.board = board.clone();
    assert!(matches!(
        catalog::put(&mut bob, Some(&id), with.clone(), NOW).await,
        Err(Error::Crypto(CryptoError::NotAdmin))
    ));
    let mine = proposals::propose(&mut bob, &id, &board, NOW)
        .await
        .unwrap();
    assert_eq!(mine.of(&id).map(|p| p.board.as_str()), Some(board.as_str()));
    let seen = proposals::sync(&mut alice).await.unwrap();
    let p = seen.of(&id).expect("Alice reads Bob's proposal");
    assert_eq!(
        (p.board.as_str(), p.by.clone()),
        (board.as_str(), member(2).to_string())
    );

    // A proposal no leaf of the team signed (forged by the server): refused at the reading.
    let at = alice
        .client()
        .read_project(&PROPOSAL_STREAM.0, 0, None)
        .await
        .unwrap()
        .entries
        .last()
        .unwrap()
        .seq;
    {
        let mut l = pa.liar.lies.lock().unwrap();
        l.project = Some(hex::encode(PROPOSAL_STREAM.0));
        l.project_inject = vec![(at + 1, b"not a sealed object".to_vec())];
    }
    let seen = proposals::sync(&mut alice).await.unwrap();
    assert_eq!(seen.refused, 1);
    assert_eq!(seen.proposals.len(), 1);
    pa.liar.lies.lock().unwrap().project_inject.clear();

    // Alice writes it: Bob reads the entry with its Kanban.
    put(&mut alice, Some(&id), with.clone()).await;
    let read = sync(&mut bob).await;
    let got = &read.get(&id).unwrap().project;
    assert_eq!(got.board, board);
    assert_eq!(
        got.board_id().map(|b| hex::encode(b.0)),
        Some(board.clone())
    );

    // What the server keeps holds neither the board nor the proposal in clear (its folder is empty under PostgreSQL).
    if matches!(backend, Backend::Sqlite) {
        let disk = server_bytes(&w);
        for clear in [board.as_bytes().to_vec(), vec![0x5a; 16]] {
            assert!(
                !disk.windows(clear.len()).any(|x| x == clear.as_slice()),
                "the board is not on the server's disk"
            );
        }
    }

    // The team's own streams stay the whole team's: a member cannot shut the others out of them.
    for stream in [CATALOG_STREAM, PROPOSAL_STREAM, SETTINGS_STREAM] {
        let refused = bob
            .client()
            .set_access(
                &stream.0,
                &escouade_team::wire::AccessWire {
                    everyone: false,
                    devices: vec![serde_bytes::ByteBuf::from(bob.client().device().to_vec())],
                },
            )
            .await
            .expect_err("nobody restricts the team's streams");
        assert_eq!(refused.status(), Some(403));
    }
    w.finish().await;
}

both!(
    an_entry_gains_its_kanban_by_an_administrator_or_a_proposal,
    an_administrator_keeps_the_list_and_a_member_reads_it,
    a_member_cannot_write_the_list,
    a_dangerous_address_is_refused_even_signed,
    a_clone_is_known_by_the_address_of_its_repository,
);

/// SR2-02: the server's operator removes the last entry of the list a member read (`drop-entry`): a hole, said to
/// the administrators, not a rollback; the list reads on.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_entry_the_operator_removed_is_no_rollback() {
    use escouade_team_server::db::Stream;
    let w = World::new().await;
    let (_pa, mut alice, _pb, mut bob) = two(&w).await;
    put(&mut alice, None, api()).await;
    put(&mut alice, None, platform()).await;
    assert_eq!(sync(&mut bob).await.projects.len(), 2);
    let last = alice
        .client()
        .read_project(&CATALOG_STREAM.0, 0, None)
        .await
        .unwrap()
        .entries
        .last()
        .unwrap()
        .seq;
    w.server
        .server
        .app()
        .store
        .drop_entry(
            alice.id(),
            Stream::Project(CATALOG_STREAM.0),
            last,
            "operator".into(),
            1,
        )
        .await
        .unwrap()
        .unwrap();
    let list = sync(&mut bob).await;
    assert_eq!(list.alarm, None, "a hole the operator made is no rollback");
    put(&mut alice, None, api()).await;
    assert_eq!(sync(&mut bob).await.alarm, None);
    assert_eq!(alice.serve(NOW + 400).await.unwrap().drop_alerts, 1);
}
