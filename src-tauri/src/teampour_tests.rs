//! « Verser à l'équipe » (TS-7) against the real `escouade-team-server`: two data folders that both have the files of
//! ESC-190 pour them at the same moment, then once more: each ticket is once on each board, under the object of its
//! file's id, with a key of the server's counter (an imported one keeps its own) and its former key kept; a ticket of
//! this machine chosen goes too; the files are frozen, still read by ESC-190's reader, nothing removed. And the
//! attachment: the Kanban of the team's list joined without an id to copy, a member's Kanban proposed to an
//! administrator, the key `team` of `.escouade.yml` written.

use std::time::Duration;

use super::*;
use crate::core_tests::{harness, Harness};
use crate::integrations::secrets::MemoryStore;
use crate::model::{Person, SharedExternal};
use crate::sharing::{TicketDoc, FROZEN};
use crate::teamboard::tests::{draft, find, read, server, teams};

fn doc(id: &str, key: &str, title: &str, external: bool) -> String {
    sharing::write(&TicketDoc {
        version: 1,
        id: id.into(),
        key: key.into(),
        title: title.into(),
        status: Column::Todo,
        author: Person {
            name: "Alice Martin".into(),
            id: "a11ce000".into(),
        },
        description: format!("{title}, la description"),
        criteria: vec!["Fait".into()],
        external: external.then(|| SharedExternal {
            source: "jira".into(),
            key: key.into(),
            url: format!("https://acme.atlassian.net/browse/{key}"),
        }),
        ..Default::default()
    })
}

/// The project's folder with two files of ESC-190, as a teammate committed them: read into its board.
fn with_files(h: &Harness, project: &str, path: &std::path::Path) {
    let dir = path.join(sharing::DIR);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("k7m2pq9x.yml"),
        doc("k7m2pq9x", "DEM-12", "Le bail", false),
    )
    .unwrap();
    std::fs::write(
        dir.join("r4tz8wn3.yml"),
        doc("r4tz8wn3", "ACME-7", "Importé de Jira", true),
    )
    .unwrap();
    h.core.sync_shared(project);
}

fn remote(path: &std::path::Path, url: &str) {
    let ok = std::process::Command::new("git")
        .args([
            "-C",
            &path.to_string_lossy(),
            "remote",
            "add",
            "origin",
            url,
        ])
        .status()
        .unwrap()
        .success();
    assert!(ok);
}

/// The tickets of a project of the team on a board, by title.
fn team_titles(h: &Harness, project: &str) -> Vec<String> {
    let mut out: Vec<String> = h
        .core
        .tickets
        .read()
        .iter()
        .filter(|t| t.project_id == project && t.team.is_some())
        .map(|t| t.title.clone())
        .collect();
    out.sort();
    out
}

struct Two {
    ha: Harness,
    hb: Harness,
    pa: crate::model::Project,
    pb: crate::model::Project,
    pa_path: std::path::PathBuf,
    pb_path: std::path::PathBuf,
    alice: Arc<crate::teams::Teams>,
    bob: Arc<crate::teams::Teams>,
    team: String,
    server: Arc<escouade_team_server::Server>,
    task: tokio::task::JoinHandle<()>,
    _dir: tempfile::TempDir,
}

/// Alice (the team's owner, an administrator) and Bob (a member), each with a project of a new repository.
async fn two(name: &str) -> Two {
    let (server, host, task, dir) = server().await;
    let store = Arc::new(MemoryStore::default());
    let alice = teams(&dir.path().join("a"), store.clone());
    let bob = teams(&dir.path().join("b"), store.clone());
    let token = server.new_team_token().await.unwrap();
    let team = alice
        .create(&host, &token, "ACME", None)
        .await
        .unwrap()
        .team
        .id;
    let invite = alice.invite(&team, "member", None).await.unwrap();
    bob.join(&invite.url, "").await.unwrap();
    crate::teams_tests::let_in(&alice, &bob, &team).await;
    let ha = harness(&format!("{name}-alice"));
    let hb = harness(&format!("{name}-bob"));
    ha.core.settings.write().user_name = "Alice Martin".into();
    hb.core.settings.write().user_name = "Bob Durand".into();
    let (pa, pa_path) = ha.project(false).await;
    let (pb, pb_path) = hb.project(false).await;
    ha.core.set_teams(alice.clone());
    hb.core.set_teams(bob.clone());
    Two {
        ha,
        hb,
        pa,
        pb,
        pa_path,
        pb_path,
        alice,
        bob,
        team,
        server,
        task,
        _dir: dir,
    }
}

#[test]
fn the_object_of_a_file_is_the_same_everywhere_and_unlike_its_id() {
    assert_eq!(object_of_file("k7m2pq9x"), object_of_file("k7m2pq9x"));
    assert_ne!(object_of_file("k7m2pq9x"), object_of_file("r4tz8wn3"));
    assert_eq!(object_of_file("k7m2pq9x").len(), 32);
    assert!(!object_of_file("k7m2pq9x").contains("k7m2pq9x"));
}

#[test]
fn a_frozen_file_is_read_by_the_reader_of_before() {
    let text = doc("k7m2pq9x", "DEM-12", "Le bail", false);
    let frozen = format!("{FROZEN}\n{text}");
    let (Read::Ok(a, _), Read::Ok(b, _)) = (sharing::read(&text), sharing::read(&frozen)) else {
        panic!("both read");
    };
    assert_eq!(a, b, "the line is a comment, nothing else changes");
    assert!(sharing::is_frozen(&frozen));
    // Frozen once: never twice.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("k7m2pq9x.yml"), &text).unwrap();
    assert_eq!(sharing::freeze(dir.path()), vec!["k7m2pq9x.yml"]);
    assert_eq!(sharing::freeze(dir.path()), vec!["k7m2pq9x.yml"]);
    let after = std::fs::read_to_string(dir.path().join("k7m2pq9x.yml")).unwrap();
    assert_eq!(after.matches(FROZEN).count(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_devices_pour_the_files_at_once_and_twice_without_a_double() {
    let w = two("pour").await;
    let (ha, hb) = (&w.ha, &w.hb);
    with_files(ha, &w.pa.id, &w.pa_path);
    with_files(hb, &w.pb.id, &w.pb_path);
    assert_eq!(
        hb.core
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == w.pb.id && t.shared.is_some())
            .count(),
        2,
        "Bob's board read the files"
    );
    // A ticket of Alice's machine, before the Kanban is the team's.
    let mine = ha
        .core
        .ticket_create(&w.pa.id, draft("Brouillon d’Alice", false))
        .await
        .unwrap();
    ha.core
        .set_project_team(&w.pa.id, Some(w.team.clone()))
        .unwrap();
    hb.core
        .set_project_team(&w.pb.id, Some(w.team.clone()))
        .unwrap();
    let linked = ha.core.link_team_board(&w.pa.id, None).await.unwrap();
    let journal = linked.team_project.clone().unwrap();
    hb.core
        .link_team_board(&w.pb.id, Some(journal.clone()))
        .await
        .unwrap();
    let prefix = linked.board.prefix.clone();
    // Linked: the files are frozen (C5-A), still there, still read; nothing of them left yet.
    for p in [&w.pa_path, &w.pb_path] {
        let text = std::fs::read_to_string(p.join(sharing::DIR).join("k7m2pq9x.yml")).unwrap();
        assert!(sharing::is_frozen(&text));
        assert!(matches!(sharing::read(&text), Read::Ok(..)));
    }
    assert!(
        team_titles(ha, &w.pa.id).is_empty(),
        "nothing leaves without the gesture"
    );
    assert!(ha.core.ticket(&mine.id).unwrap().team_local);

    // The preview says what would go.
    let preview = ha.core.team_pour_preview(&w.pa.id).await.unwrap();
    assert_eq!(preview.files.len(), 2);
    assert!(preview
        .files
        .iter()
        .all(|f| !f.poured && f.problem.is_none()));
    assert!(preview
        .files
        .iter()
        .any(|f| f.external && f.key == "ACME-7"));
    assert_eq!(preview.locals.len(), 1);
    assert_eq!(preview.locals[0].id, mine.id);
    assert_eq!(preview.folder, ".escouade/tickets");

    // Both pour at the same moment.
    let (a, b) = tokio::join!(
        ha.core
            .team_pour(&w.pa.id, true, vec![mine.id.clone()], false),
        hb.core.team_pour(&w.pb.id, true, vec![], false)
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(a.imported + a.already, 2);
    assert_eq!(b.imported + b.already, 2);
    assert_eq!(a.locals, 1);
    assert_eq!(a.files, vec!["k7m2pq9x.yml", "r4tz8wn3.yml"]);
    // Read until the boards are in step.
    for _ in 0..6 {
        w.alice.sync(&w.team).await.unwrap();
        w.bob.sync(&w.team).await.unwrap();
        read(ha, &w.pa.id).await;
        read(hb, &w.pb.id).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let want = vec![
        "Brouillon d’Alice".to_string(),
        "Importé de Jira".to_string(),
        "Le bail".to_string(),
    ];
    assert_eq!(
        team_titles(ha, &w.pa.id),
        want,
        "no double on Alice's board"
    );
    assert_eq!(team_titles(hb, &w.pb.id), want, "no double on Bob's board");
    for (h, p) in [(ha, &w.pa.id), (hb, &w.pb.id)] {
        let all: Vec<Ticket> = h
            .core
            .tickets
            .read()
            .iter()
            .filter(|t| t.project_id == *p)
            .cloned()
            .collect();
        assert_eq!(
            all.len(),
            3,
            "the file's ticket of before became the team's: {all:?}"
        );
        let bail = find(h, p, "Le bail").unwrap();
        let tt = bail.team.clone().unwrap();
        assert_eq!(tt.object, object_of_file("k7m2pq9x"), "its id kept");
        assert!(bail.shared.is_none());
        assert!(
            bail.key.starts_with(&format!("{prefix}-")) && !bail.key.contains("-L"),
            "a key of the team: {}",
            bail.key
        );
        assert!(
            bail.former_keys.contains(&"DEM-12".to_string()),
            "{:?}",
            bail.former_keys
        );
        let jira = find(h, p, "Importé de Jira").unwrap();
        assert_eq!(jira.key, "ACME-7", "an imported ticket keeps its key");
    }
    let bail_a = find(ha, &w.pa.id, "Le bail").unwrap();
    let bail_b = find(hb, &w.pb.id, "Le bail").unwrap();
    assert_eq!(bail_a.key, bail_b.key, "one key on both boards");
    assert_eq!(
        bail_a.team.unwrap().claim,
        bail_b.team.unwrap().claim,
        "one claim"
    );
    let draft_b = find(hb, &w.pb.id, "Brouillon d’Alice").unwrap();
    assert!(draft_b.key.starts_with(&format!("{prefix}-")));
    assert!(
        draft_b.former_keys.contains(&mine.key),
        "{:?} has {}",
        draft_b.former_keys,
        mine.key
    );

    // Once more, on each side: nothing new.
    let again = ha
        .core
        .team_pour(&w.pa.id, true, vec![], false)
        .await
        .unwrap();
    assert_eq!((again.imported, again.already), (0, 2));
    let again = hb
        .core
        .team_pour(&w.pb.id, true, vec![], false)
        .await
        .unwrap();
    assert_eq!((again.imported, again.already), (0, 2));
    let preview = hb.core.team_pour_preview(&w.pb.id).await.unwrap();
    assert!(preview.files.iter().all(|f| f.poured));
    // The files are all still there: Escouade removes nothing.
    assert_eq!(
        std::fs::read_dir(w.pa_path.join(sharing::DIR))
            .unwrap()
            .count(),
        2
    );
    // Escouade no longer reads nor writes them: a file changed by a version of before stays out of the board.
    let p = w.pa_path.join(sharing::DIR).join("k7m2pq9x.yml");
    std::fs::write(&p, doc("k7m2pq9x", "DEM-12", "Changé par la 2.2", false)).unwrap();
    ha.core.sync_shared(&w.pa.id);
    assert!(find(ha, &w.pa.id, "Changé par la 2.2").is_none());
    assert_eq!(
        std::fs::read_to_string(&p).unwrap(),
        doc("k7m2pq9x", "DEM-12", "Changé par la 2.2", false)
    );
    let _ = &w.server;
    w.task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_kanban_of_the_list_is_joined_without_an_id_and_a_member_proposes_his() {
    let w = two("attach").await;
    let (ha, hb) = (&w.ha, &w.hb);
    let url = "https://github.com/acme/api.git";
    remote(&w.pa_path, url);
    remote(&w.pb_path, "git@github.com:acme/api");
    // The server learns Alice's role (the core does it at each reading).
    w.alice.sync(&w.team).await.unwrap();
    let (item, _) = w
        .alice
        .catalog_put(
            &w.team,
            None,
            escouade_team::catalog::Listed {
                name: "Acme API".into(),
                repos: vec![escouade_team::catalog::Repo {
                    url: url.into(),
                    folder: String::new(),
                }],
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // Bob attaches his clone: the list names no Kanban yet, none is joined; the key `team` is written.
    let p = hb.core.team_attach(&w.pb.id, &w.team).await.unwrap();
    assert_eq!(p.team.as_deref(), Some(w.team.as_str()));
    assert!(p.team_project.is_none());
    let yml = std::fs::read_to_string(w.pb_path.join(".escouade.yml")).unwrap();
    assert!(yml.contains(&format!("id: {}", w.team)), "{yml}");
    assert!(yml.contains(&format!("project: {item}")), "{yml}");
    assert!(yml.contains("server: http://127.0.0.1:"), "{yml}");
    let hint = p.shared.team.clone().expect("read back");
    assert_eq!(
        (hint.team.as_str(), hint.project.as_deref()),
        (w.team.as_str(), Some(item.as_str()))
    );

    // Bob, a member, shares his Kanban: it is proposed to an administrator, not written.
    let shared = hb.core.link_team_board(&w.pb.id, None).await.unwrap();
    let journal = shared.team_project.clone().unwrap();
    let of = w.alice.catalog(&w.team).await.unwrap();
    assert_eq!(of.catalog.get(&item).unwrap().project.board, "");
    assert_eq!(
        of.proposals.of(&item).map(|p| p.board.clone()),
        Some(journal.clone())
    );
    // Alice writes it.
    ha.core.team_accept_board(&w.team, &item).await.unwrap();
    let of = w.alice.catalog(&w.team).await.unwrap();
    assert_eq!(of.catalog.get(&item).unwrap().project.board, journal);

    // Alice attaches her project: she joins Bob's Kanban without copying anything.
    let p = ha.core.team_attach(&w.pa.id, &w.team).await.unwrap();
    assert_eq!(p.team_project.as_deref(), Some(journal.as_str()));
    // A ticket of Bob's is on Alice's board.
    hb.core
        .ticket_create(&w.pb.id, draft("De Bob", false))
        .await
        .unwrap();
    read(hb, &w.pb.id).await;
    w.alice.sync(&w.team).await.unwrap();
    read(ha, &w.pa.id).await;
    assert!(find(ha, &w.pa.id, "De Bob").is_some());

    // Detached: the key `team` goes from the file.
    ha.core.team_detach(&w.pa.id).unwrap();
    let yml = std::fs::read_to_string(w.pa_path.join(".escouade.yml")).unwrap();
    assert!(!yml.contains(&w.team), "{yml}");
    // An unknown team is refused: attaching needs to be in it.
    assert!(ha
        .core
        .team_attach(&w.pa.id, &"ab".repeat(16))
        .await
        .is_err());
    w.task.abort();
}

#[tokio::test]
async fn the_key_team_is_never_written_through_what_is_not_a_plain_file() {
    let h = harness("teamkey-link");
    let (p, path) = h.project(false).await;
    // A repository whose `.escouade.yml` is not a file (a folder here; a link would be refused the same way).
    std::fs::create_dir_all(path.join(".escouade.yml")).unwrap();
    let key = crate::projectfile::TeamKey {
        server: "https://team.acme.fr".into(),
        id: "ab".repeat(16),
        project: None,
        name: "Acme".into(),
    };
    assert!(h.core.write_team_key(&p.id, Some(&key)).is_err());
    assert!(path.join(".escouade.yml").is_dir());
    #[cfg(unix)]
    {
        std::fs::remove_dir(path.join(".escouade.yml")).unwrap();
        let outside = h.dir.join("outside.yml");
        std::fs::write(&outside, "version: 1\n").unwrap();
        std::os::unix::fs::symlink(&outside, path.join(".escouade.yml")).unwrap();
        assert!(h.core.write_team_key(&p.id, Some(&key)).is_err());
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "version: 1\n");
    }
}

/// TS-7 with TS-9: attaching and joining the Kanban sends none of the repository's docs; the preview of « Verser à
/// l'équipe » names them; « Verser » imports them; a teammate then reads them from the server, and his preview says
/// nothing of his goes; a private project's never go.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_docs_of_the_repository_leave_only_by_the_gesture_named_in_the_preview() {
    let w = two("pourdocs").await;
    let (ha, hb) = (&w.ha, &w.hb);
    crate::docs::Space::new(&w.pa_path, None)
        .spec_write("general/accueil", |p| p.body = "Bonjour l'équipe.".into())
        .unwrap();
    ha.core
        .set_project_team(&w.pa.id, Some(w.team.clone()))
        .unwrap();
    hb.core
        .set_project_team(&w.pb.id, Some(w.team.clone()))
        .unwrap();
    let linked = ha.core.link_team_board(&w.pa.id, None).await.unwrap();
    hb.core
        .link_team_board(&w.pb.id, linked.team_project.clone())
        .await
        .unwrap();
    // The reading of the team passes over it: nothing leaves, the docs stay the repository's.
    ha.core.team_docs_after_read(&w.team).await;
    let pa = ha.core.project(&w.pa.id).unwrap();
    assert!(!ha.core.team_docs_on(&pa));
    assert_eq!(ha.core.team_docs_state(&pa).await, Some((None, false)));
    let space = ha.core.docs_space(&w.pa.id, None).await.unwrap();
    assert_eq!(
        space.spec_read("general/accueil").unwrap().body.trim(),
        "Bonjour l'équipe."
    );

    // The preview names them.
    let preview = ha.core.team_pour_preview(&w.pa.id).await.unwrap();
    assert_eq!(preview.docs.state, "pending");
    assert_eq!(preview.docs.pages, vec!["spec/general/accueil.md"]);
    assert!(preview.docs.decisions.is_empty() && preview.docs.documents.is_empty());
    // Pouring without them sends none.
    let done = ha
        .core
        .team_pour(&w.pa.id, true, vec![], false)
        .await
        .unwrap();
    assert!(done.docs.is_empty());
    assert!(!ha.core.team_docs_on(&ha.core.project(&w.pa.id).unwrap()));
    // With them: imported, named to be removed from the repository, nothing removed.
    let done = ha
        .core
        .team_pour(&w.pa.id, false, vec![], true)
        .await
        .unwrap();
    assert_eq!(done.docs, vec![".escouade/spec/general/accueil.md"]);
    assert!(w
        .pa_path
        .join(".escouade/spec/general/accueil.md")
        .is_file());
    let pa = ha.core.project(&w.pa.id).unwrap();
    assert!(ha.core.team_docs_on(&pa));
    assert_eq!(
        ha.core
            .team_pour_preview(&w.pa.id)
            .await
            .unwrap()
            .docs
            .state,
        "server"
    );
    // Once: a second gesture sends nothing more.
    assert!(ha
        .core
        .team_pour(&w.pa.id, false, vec![], true)
        .await
        .unwrap()
        .docs
        .is_empty());

    // Bob reads them from the server; nothing of his repository goes.
    w.bob.sync(&w.team).await.unwrap();
    hb.core.team_docs_after_read(&w.team).await;
    let pb = hb.core.project(&w.pb.id).unwrap();
    assert!(hb.core.team_docs_on(&pb));
    let bob = hb.core.docs_space(&w.pb.id, None).await.unwrap();
    assert_eq!(
        bob.spec_read("general/accueil").unwrap().body.trim(),
        "Bonjour l'équipe."
    );
    assert_eq!(
        hb.core
            .team_pour_preview(&w.pb.id)
            .await
            .unwrap()
            .docs
            .state,
        "server"
    );
    // A private project's never go.
    {
        let mut projects = hb.core.projects.write();
        projects
            .iter_mut()
            .find(|p| p.id == w.pb.id)
            .unwrap()
            .private = true;
    }
    assert_eq!(
        hb.core
            .team_pour_preview(&w.pb.id)
            .await
            .unwrap()
            .docs
            .state,
        "private"
    );
    w.task.abort();
}
