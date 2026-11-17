//! The docs of a team's project on the team server (TS-9), in the core, against the real `escouade-team-server`: two
//! data folders of one machine, each its own core, its own repository of the project, its own copy of the team. A page,
//! a decision and a document written on one are read on the other, and the server keeps neither their titles, nor
//! their paths, nor their texts; a page written on both sides merges three ways or shows its conflict; two decisions
//! accepted at the same moment never share a number; a ticket's draft is no one's before its validation, then poured;
//! offline, then back; the repository's docs imported (ids and numbers kept, nothing removed); « dans le dépôt »;
//! a member removed reads nothing written after.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core_tests::{harness, Harness};
use crate::docs::{Ref, Space, State, ROOT};
use crate::docs_tests::proposal;
use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::model::Project;
use crate::teams::Teams;

pub(crate) struct Server {
    server: Arc<escouade_team_server::Server>,
    host: String,
    /// The relay's address, the one the data folders know; the server's own, behind it.
    addr: std::net::SocketAddr,
    target: std::net::SocketAddr,
    relay: Vec<tokio::task::JoinHandle<()>>,
    conns: Arc<parking_lot::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    dir: tempfile::TempDir,
}

impl Server {
    async fn start() -> Server {
        let dir = tempfile::tempdir().unwrap();
        let mut config = escouade_team_server::config::Config::in_folder(dir.path().join("server"));
        // Two data folders of one address, many calls: never refused for their rate.
        config.rates.per_address.burst = 100_000.0;
        config.rates.per_address.per_second = 100_000.0;
        config.rates.per_device.burst = 100_000.0;
        config.rates.per_device.per_second = 100_000.0;
        let server = Arc::new(escouade_team_server::Server::open(config).await.unwrap());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = listener.local_addr().unwrap();
        let s = server.clone();
        tokio::spawn(async move {
            let _ = s.serve(listener, None, std::future::pending()).await;
        });
        let front = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = front.local_addr().unwrap();
        let mut s = Server {
            server,
            host: format!("127.0.0.1:{}", addr.port()),
            addr,
            target,
            relay: Vec::new(),
            conns: Arc::default(),
            dir,
        };
        s.relay_on(front);
        s
    }

    fn relay_on(&mut self, front: tokio::net::TcpListener) {
        let (to, conns) = (self.target, self.conns.clone());
        self.relay.push(tokio::spawn(async move {
            while let Ok((mut inbound, _)) = front.accept().await {
                let task = tokio::spawn(async move {
                    if let Ok(mut outbound) = tokio::net::TcpStream::connect(to).await {
                        let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
                    }
                });
                conns.lock().push(task);
            }
        }));
    }

    /// The server unreachable: nothing listens at its address, and every connection is cut.
    async fn stop(&mut self) {
        for t in self.relay.drain(..) {
            t.abort();
            let _ = t.await;
        }
        for t in self.conns.lock().drain(..) {
            t.abort();
        }
    }

    async fn again(&mut self) {
        let mut tries = 0;
        let front = loop {
            match tokio::net::TcpListener::bind(self.addr).await {
                Ok(l) => break l,
                Err(_) if tries < 50 => {
                    tries += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                Err(e) => panic!("{e}"),
            }
        };
        self.relay_on(front);
    }

    /// Every byte the server keeps.
    fn kept(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut todo = vec![self.dir.path().join("server")];
        while let Some(d) = todo.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    todo.push(p);
                } else if let Ok(b) = std::fs::read(&p) {
                    out.extend(b);
                }
            }
        }
        out
    }
}

fn holds(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

/// A data folder: its core, its teams, its project of the team, its repository.
pub(crate) struct Side {
    pub(crate) h: Harness,
    pub(crate) teams: Arc<Teams>,
    pub(crate) p: Project,
    pub(crate) repo: PathBuf,
}

impl Side {
    async fn new(name: &str, store: Arc<dyn SecretStore>) -> Side {
        let h = harness(name);
        let teams = Arc::new(Teams::open(
            h.core.data.root().join("teams"),
            store,
            Arc::new(crate::model::Settings::default),
            Arc::new(|| {}),
        ));
        // As the app does: the Kanban of the team (TS-6) and the docs read the same journal.
        h.core.set_teams(teams.clone());
        let (p, repo) = h.project(false).await;
        Side { h, teams, p, repo }
    }

    fn of_team(&mut self, team: &str) {
        let mut projects = self.h.core.projects.write();
        let p = projects.iter_mut().find(|x| x.id == self.p.id).unwrap();
        p.team = Some(team.to_string());
        self.p = p.clone();
    }

    /// The team read (as the watch does), then a pass of the project's docs.
    pub(crate) async fn round(&self, team: &str) {
        let _ = self.teams.sync(team).await;
        self.h.core.team_docs_pass(&self.p.id).await.unwrap();
    }

    async fn space(&self) -> Space {
        self.h.core.docs_space(&self.p.id, None).await.unwrap()
    }

    async fn changed(&self) {
        self.h.core.docs_changed(&self.p.id).await;
    }
}

/// Alice's team with Bob in it, and the project of each made the team's; its docs on the server (Alice's chosen
/// first: its repository's docs imported, then Bob's following its journal).
pub(crate) async fn pair(name: &str) -> (Server, Side, Side, String) {
    let server = Server::start().await;
    let store: Arc<dyn SecretStore> = Arc::new(MemoryStore::default());
    let mut alice = Side::new(&format!("{name}-a"), store.clone()).await;
    let mut bob = Side::new(&format!("{name}-b"), store.clone()).await;
    let token = server.server.new_team_token().await.unwrap();
    let id = alice
        .teams
        .create(&server.host, &token, "ACME", None)
        .await
        .unwrap()
        .team
        .id;
    let invite = alice.teams.invite(&id, "member", None).await.unwrap();
    bob.teams.join(&invite.url, "").await.unwrap();
    alice.teams.sync(&id).await.unwrap();
    assert!(bob.teams.sync(&id).await.unwrap().joined);
    alice.of_team(&id);
    bob.of_team(&id);
    (server, alice, bob, id)
}

pub(crate) async fn on_server(alice: &mut Side, bob: &mut Side) {
    let view = alice
        .h
        .core
        .team_docs_choose(&alice.p.id, true, None)
        .await
        .unwrap();
    assert!(view.on_server);
    let journal = view.journal.clone().unwrap();
    let view = bob
        .h
        .core
        .team_docs_choose(&bob.p.id, true, Some(journal))
        .await
        .unwrap();
    assert!(view.on_server);
    alice.p = alice.h.core.project(&alice.p.id).unwrap();
    bob.p = bob.h.core.project(&bob.p.id).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_page_a_decision_and_a_document_written_here_are_read_there_and_the_server_sees_none() {
    let (server, mut alice, mut bob, id) = pair("tdocs-read").await;
    on_server(&mut alice, &mut bob).await;
    let a = alice.space().await;
    // The window's copy, in Escouade's folder, not the repository.
    assert!(a
        .root
        .starts_with(alice.h.core.data.project_files(&alice.p.id)));
    a.spec_write("tarifs/licornes", |p| {
        p.title = "Tarif confidentiel des licornes".into();
        p.body = "Quarante-deux licornes par mois.".into();
    })
    .unwrap();
    a.decision_propose(None, proposal("Facturer en licornes"))
        .unwrap();
    a.document_add("contrats", "accord-dragons.md", b"Budget : 77 dragons.")
        .unwrap();
    alice.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    let b = bob.space().await;
    let page = b.spec_read("tarifs/licornes").unwrap();
    assert_eq!(page.title, "Tarif confidentiel des licornes");
    assert_eq!(page.body, "Quarante-deux licornes par mois.");
    let d = b.decision(&Ref::Id("facturer-en-licornes".into())).unwrap();
    assert_eq!((d.state, d.number), (State::Proposed, None));
    let text = b
        .document_text("contrats/accord-dragons.md", 0, 1000)
        .unwrap();
    assert!(text.text.unwrap().contains("77 dragons"));
    // What Alice wrote is a teammate's for Bob's agents, not for hers.
    let core_b = &bob.h.core;
    assert!(core_b.team_docs_teammate(&bob.p, "page", "tarifs/licornes"));
    assert!(core_b.team_docs_teammate(&bob.p, "decision", "facturer-en-licornes"));
    assert!(core_b.team_docs_teammate(&bob.p, "document", "contrats/accord-dragons.md"));
    assert!(!alice
        .h
        .core
        .team_docs_teammate(&alice.p, "page", "tarifs/licornes"));
    // The server keeps neither titles nor paths nor texts nor names.
    let kept = server.kept();
    for secret in [
        "licornes",
        "Tarif confidentiel",
        "Facturer",
        "accord-dragons",
        "77 dragons",
        "contrats",
        "tarifs",
    ] {
        assert!(!holds(&kept, secret), "the server keeps « {secret} »");
    }
    // Bob writes back: Alice reads it, as a teammate's.
    b.spec_write("tarifs/licornes", |p| {
        p.body = "Quarante-trois licornes par mois.".into();
    })
    .unwrap();
    bob.changed().await;
    bob.round(&id).await;
    alice.round(&id).await;
    assert_eq!(
        alice
            .space()
            .await
            .spec_read("tarifs/licornes")
            .unwrap()
            .body,
        "Quarante-trois licornes par mois."
    );
    assert!(alice
        .h
        .core
        .team_docs_teammate(&alice.p, "page", "tarifs/licornes"));
    // A private project never goes.
    let mut private = alice.p.clone();
    private.private = true;
    assert!(!alice.h.core.team_docs_on(&private));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_page_written_on_both_sides_merges_three_ways_or_shows_its_conflict() {
    let (_server, mut alice, mut bob, id) = pair("tdocs-merge").await;
    on_server(&mut alice, &mut bob).await;
    let body = "Un.\n\nDeux.\n\nTrois.";
    alice
        .space()
        .await
        .spec_write("produit/page", |p| p.body = body.into())
        .unwrap();
    alice.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    // Apart, on different lines: merged on each machine.
    alice
        .space()
        .await
        .spec_write("produit/page", |p| p.body = "UN.\n\nDeux.\n\nTrois.".into())
        .unwrap();
    bob.space()
        .await
        .spec_write("produit/page", |p| p.body = "Un.\n\nDeux.\n\nTROIS.".into())
        .unwrap();
    alice.changed().await;
    bob.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    alice.round(&id).await;
    bob.round(&id).await;
    for side in [&alice, &bob] {
        assert_eq!(
            side.space().await.spec_read("produit/page").unwrap().body,
            "UN.\n\nDeux.\n\nTROIS."
        );
        assert!(side
            .h
            .core
            .team_docs_view(&side.p.id)
            .await
            .unwrap()
            .conflicts
            .is_empty());
    }
    // The same line, two ways: a conflict shown, each keeping its own meanwhile.
    alice
        .space()
        .await
        .spec_write("produit/page", |p| {
            p.body = "UN.\n\nDeux (Alice).\n\nTROIS.".into()
        })
        .unwrap();
    bob.space()
        .await
        .spec_write("produit/page", |p| {
            p.body = "UN.\n\nDeux (Bob).\n\nTROIS.".into()
        })
        .unwrap();
    alice.changed().await;
    bob.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    alice.round(&id).await;
    let va = alice.h.core.team_docs_view(&alice.p.id).await.unwrap();
    assert_eq!(va.conflicts.len(), 1, "{va:?}");
    let c = &va.conflicts[0];
    assert_eq!(c.path, "produit/page");
    assert!(c.mine.contains("Deux (Alice)") && c.theirs.contains("Deux (Bob)"));
    assert!(c.base.contains("Deux.\n"));
    assert!(alice
        .space()
        .await
        .spec_read("produit/page")
        .unwrap()
        .body
        .contains("Alice"));
    let vb = bob.h.core.team_docs_view(&bob.p.id).await.unwrap();
    assert_eq!(vb.conflicts.len(), 1);
    // « Prendre la sienne » on Alice's: Bob's, on both, and the conflict gone.
    alice
        .h
        .core
        .team_docs_settle(&alice.p.id, "produit/page", "theirs")
        .await
        .unwrap();
    alice.round(&id).await;
    bob.round(&id).await;
    for side in [&alice, &bob] {
        assert!(side
            .space()
            .await
            .spec_read("produit/page")
            .unwrap()
            .body
            .contains("Deux (Bob)"));
        assert!(side
            .h
            .core
            .team_docs_view(&side.p.id)
            .await
            .unwrap()
            .conflicts
            .is_empty());
    }
    // A decision's field written on both sides: the last writer wins, the one who lost sees it.
    let a = alice.space().await;
    a.decision_propose(None, proposal("Facturer au mois"))
        .unwrap();
    alice.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    let r = Ref::Id("facturer-au-mois".into());
    let mut d = a.decision(&r).unwrap();
    d.context = "Le contexte d’Alice.".into();
    a.decision_propose(Some(&r), d).unwrap();
    alice.changed().await;
    let b = bob.space().await;
    let mut d = b.decision(&r).unwrap();
    d.context = "Le contexte de Bob.".into();
    b.decision_propose(Some(&r), d).unwrap();
    bob.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    alice.round(&id).await;
    let ca = alice.space().await.decision(&r).unwrap().context;
    let cb = bob.space().await.decision(&r).unwrap().context;
    assert_eq!(ca, cb, "the same on both");
    let loser = if ca.contains("Bob") { &alice } else { &bob };
    let lost = loser
        .h
        .core
        .team_docs_view(&loser.p.id)
        .await
        .unwrap()
        .decision_conflicts;
    assert_eq!(lost.len(), 1, "{lost:?}");
    assert_eq!(lost[0].field, "context");
    let stamp = lost[0].stamp.clone();
    let after = loser
        .h
        .core
        .team_docs_dismiss(&loser.p.id, &stamp)
        .await
        .unwrap();
    assert!(after.decision_conflicts.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_decisions_accepted_at_the_same_moment_never_share_a_number() {
    let (mut server, mut alice, mut bob, id) = pair("tdocs-numbers").await;
    on_server(&mut alice, &mut bob).await;
    let a = alice.space().await;
    a.decision_propose(None, proposal("Choix d’Alice")).unwrap();
    a.spec_write("produit/choix", |p| {
        p.body = "Le choix.".into();
        p.decisions = vec![Ref::Id("choix-d-alice".into())];
    })
    .unwrap();
    alice.changed().await;
    bob.space()
        .await
        .decision_propose(None, proposal("Choix de Bob"))
        .unwrap();
    bob.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    alice.round(&id).await;
    // Each accepts its own, neither having read the other's acceptance: the server is away meanwhile.
    server.stop().await;
    let da = alice
        .h
        .core
        .docs_accept_decision(&alice.p.id, &Ref::Id("choix-d-alice".into()))
        .await
        .unwrap();
    let db = bob
        .h
        .core
        .docs_accept_decision(&bob.p.id, &Ref::Id("choix-de-bob".into()))
        .await
        .unwrap();
    // Accepted, the number waits for the server's order.
    assert_eq!((da.state, da.number), (State::Accepted, None));
    assert_eq!((db.state, db.number), (State::Accepted, None));
    let va = alice.h.core.team_docs_view(&alice.p.id).await.unwrap();
    assert_eq!(va.waiting_numbers, vec!["choix-d-alice".to_string()]);
    server.again().await;
    for _ in 0..2 {
        alice.round(&id).await;
        bob.round(&id).await;
    }
    let mut numbers = Vec::new();
    for side in [&alice, &bob] {
        let s = side.space().await;
        let a = s.decision(&Ref::Id("choix-d-alice".into())).unwrap();
        let b = s.decision(&Ref::Id("choix-de-bob".into())).unwrap();
        numbers.push((a.number, b.number));
    }
    assert_eq!(numbers[0], numbers[1], "the same numbers on both");
    let (na, nb) = numbers[0];
    let (na, nb) = (na.unwrap(), nb.unwrap());
    assert_ne!(na, nb);
    let mut both = [na, nb];
    both.sort();
    assert_eq!(both, [1, 2]);
    // The page that cited Alice's by its id cites its number, on both.
    for side in [&alice, &bob] {
        assert_eq!(
            side.space()
                .await
                .spec_read("produit/choix")
                .unwrap()
                .decisions,
            vec![Ref::Number(na)]
        );
    }
    assert!(alice
        .h
        .core
        .team_docs_view(&alice.p.id)
        .await
        .unwrap()
        .waiting_numbers
        .is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_tickets_draft_is_no_ones_before_its_validation_then_poured() {
    let (_server, mut alice, mut bob, id) = pair("tdocs-draft").await;
    on_server(&mut alice, &mut bob).await;
    alice
        .space()
        .await
        .spec_write("paiement/echeances", |p| p.body = "Trois échéances.".into())
        .unwrap();
    alice.changed().await;
    alice.round(&id).await;
    // A ticket of the team's Kanban (TS-6): this board takes those « À faire » (C11-A), as one launched by hand.
    {
        let mut projects = alice.h.core.projects.write();
        let p = projects.iter_mut().find(|x| x.id == alice.p.id).unwrap();
        p.board.team_take_todo = true;
    }
    let (t, _wt) = crate::tickets_tests::reviewed(&alice.h, &alice.p.id, "Fichier [ok]").await;
    let agent = t.agent_id.clone().unwrap();
    let draft = alice
        .h
        .core
        .docs_space(&alice.p.id, Some(&agent))
        .await
        .unwrap();
    let team = alice.space().await;
    assert_ne!(draft.root, team.root, "its agent writes in a draft");
    assert_eq!(
        draft.spec_read("paiement/echeances").unwrap().body,
        "Trois échéances."
    );
    draft
        .spec_write("paiement/echeances", |p| {
            p.body = "Quatre échéances.".into()
        })
        .unwrap();
    let mut d = proposal("Quatre échéances");
    d.origin = "ticket".into();
    d.origin_ref = t.key.clone();
    draft.decision_propose(None, d).unwrap();
    alice.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    // Before the validation: the team's docs, here and there, say what is merged.
    for side in [&alice, &bob] {
        let s = side.space().await;
        assert_eq!(
            s.spec_read("paiement/echeances").unwrap().body,
            "Trois échéances."
        );
        assert!(s.decisions().is_empty());
    }
    alice.h.core.ticket_approve(&t.id).await.unwrap();
    assert_eq!(alice.h.ticket(&t.id).column, crate::model::Column::Done);
    for _ in 0..2 {
        alice.round(&id).await;
        bob.round(&id).await;
    }
    for side in [&alice, &bob] {
        let s = side.space().await;
        assert_eq!(
            s.spec_read("paiement/echeances").unwrap().body,
            "Quatre échéances."
        );
        let ds = s.decisions();
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].state, State::Accepted);
        assert_eq!(ds[0].number, Some(1), "its number from the team's counter");
        assert!(ds[0].tickets.contains(&t.key));
    }
    assert!(!alice.h.core.ticket_docs_dir(&alice.p.id, &t.key).exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn offline_the_docs_read_and_write_and_the_changes_leave_when_back() {
    let (mut server, mut alice, mut bob, id) = pair("tdocs-offline").await;
    on_server(&mut alice, &mut bob).await;
    alice
        .space()
        .await
        .spec_write("a/page", |p| p.body = "En ligne.".into())
        .unwrap();
    alice.changed().await;
    alice.round(&id).await;
    server.stop().await;
    // Offline: read and written from the copy; the change waits.
    let a = alice.space().await;
    assert_eq!(a.spec_read("a/page").unwrap().body, "En ligne.");
    a.spec_write("a/page", |p| p.body = "Hors ligne.".into())
        .unwrap();
    a.spec_write("a/autre", |p| p.body = "Écrite hors ligne.".into())
        .unwrap();
    alice.changed().await;
    alice.round(&id).await;
    let view = alice.h.core.team_docs_view(&alice.p.id).await.unwrap();
    assert_eq!(view.online, Some(false));
    assert!(view.pending >= 2, "{view:?}");
    assert_eq!(
        alice.space().await.spec_read("a/page").unwrap().body,
        "Hors ligne."
    );
    // Back: the outbox leaves; Bob reads both.
    server.again().await;
    alice.round(&id).await;
    let view = alice.h.core.team_docs_view(&alice.p.id).await.unwrap();
    assert_eq!((view.online, view.pending), (Some(true), 0));
    bob.round(&id).await;
    let b = bob.space().await;
    assert_eq!(b.spec_read("a/page").unwrap().body, "Hors ligne.");
    assert_eq!(b.spec_read("a/autre").unwrap().body, "Écrite hors ligne.");
}

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_repositorys_docs_are_imported_ids_and_numbers_kept_and_nothing_removed() {
    let (_server, mut alice, mut bob, id) = pair("tdocs-import").await;
    // Alice's repository has its docs, as before the team.
    let repo = Space::new(&alice.repo, None);
    repo.spec_write("compte/connexion", |p| p.body = "Par mot de passe.".into())
        .unwrap();
    repo.decision_propose(None, proposal("Ancienne décision"))
        .unwrap();
    repo.decision_accept(
        &Ref::Id("ancienne-decision".into()),
        "Léa",
        &crate::docs::Register {
            next: 4,
            taken: Vec::new(),
        },
        None,
    )
    .unwrap();
    repo.decision_propose(None, proposal("Encore ouverte"))
        .unwrap();
    write(&alice.repo, ".escouade/documents/cahier.md", "# Le cahier");
    let before = crate::teamdocs::repo_doc_files(&alice.repo);
    assert_eq!(before.len(), 4, "{before:?}");
    on_server(&mut alice, &mut bob).await;
    bob.round(&id).await;
    let b = bob.space().await;
    assert_eq!(
        b.spec_read("compte/connexion").unwrap().body,
        "Par mot de passe."
    );
    let old = b.decision(&Ref::Id("ancienne-decision".into())).unwrap();
    assert_eq!((old.number, old.state), (Some(4), State::Accepted));
    let open = b.decision(&Ref::Id("encore-ouverte".into())).unwrap();
    assert_eq!((open.number, open.state), (None, State::Proposed));
    assert!(b
        .document_text("cahier.md", 0, 100)
        .unwrap()
        .text
        .unwrap()
        .contains("cahier"));
    // Nothing removed from the repository: the window names the files to remove.
    assert_eq!(crate::teamdocs::repo_doc_files(&alice.repo), before);
    let view = alice.h.core.team_docs_view(&alice.p.id).await.unwrap();
    assert_eq!(view.repo_files, before);
    // The next one accepted takes 5.
    b.decision_propose(None, proposal("Nouvelle")).unwrap();
    bob.changed().await;
    bob.h
        .core
        .docs_accept_decision(&bob.p.id, &Ref::Id("nouvelle".into()))
        .await
        .unwrap();
    bob.round(&id).await;
    bob.round(&id).await;
    assert_eq!(
        bob.space()
            .await
            .decision(&Ref::Id("nouvelle".into()))
            .unwrap()
            .number,
        Some(5)
    );
    // « Retirer du dépôt »: removed from its folder (the user commits).
    let view = alice
        .h
        .core
        .team_docs_remove_repo_files(&alice.p.id)
        .await
        .unwrap();
    assert!(view.repo_files.is_empty());
    assert!(crate::teamdocs::repo_doc_files(&alice.repo).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn docs_kept_in_the_repository_stay_there_and_going_back_loses_nothing() {
    let (_server, mut alice, mut bob, id) = pair("tdocs-repo").await;
    on_server(&mut alice, &mut bob).await;
    alice
        .space()
        .await
        .spec_write("a/sur-le-serveur", |p| p.body = "Du serveur.".into())
        .unwrap();
    alice.changed().await;
    alice.round(&id).await;
    // « Les docs restent dans le dépôt »: the team's choice, for both.
    let view = alice
        .h
        .core
        .team_docs_choose(&alice.p.id, false, None)
        .await
        .unwrap();
    assert!(!view.on_server);
    // Nothing lost: the server's docs written in the repository.
    let repo = Space::new(&alice.repo, None);
    assert_eq!(
        repo.spec_read("a/sur-le-serveur").unwrap().body,
        "Du serveur."
    );
    let window = alice.space().await;
    assert_eq!(window.root, alice.repo);
    window
        .spec_write("a/dans-le-depot", |p| p.body = "Du dépôt.".into())
        .unwrap();
    alice.changed().await;
    bob.round(&id).await;
    assert!(!bob.h.core.team_docs_on(&bob.p));
    assert_eq!(bob.space().await.root, bob.repo);
    // Back to the server: the repository's docs go as new versions.
    alice
        .h
        .core
        .team_docs_choose(&alice.p.id, true, None)
        .await
        .unwrap();
    alice.round(&id).await;
    let _ = bob.teams.sync(&id).await;
    bob.h
        .core
        .team_docs_choose(&bob.p.id, true, None)
        .await
        .unwrap();
    bob.round(&id).await;
    let b = bob.space().await;
    assert_eq!(b.spec_read("a/dans-le-depot").unwrap().body, "Du dépôt.");
    assert_eq!(b.spec_read("a/sur-le-serveur").unwrap().body, "Du serveur.");
    assert!(!b.root.join(ROOT).join("spec").starts_with(&bob.repo));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_member_removed_reads_nothing_written_after() {
    let (_server, mut alice, mut bob, id) = pair("tdocs-removed").await;
    on_server(&mut alice, &mut bob).await;
    alice
        .space()
        .await
        .spec_write("a/avant", |p| p.body = "Avant son départ.".into())
        .unwrap();
    alice.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    assert!(bob.space().await.spec_read("a/avant").is_ok());
    let bob_member = {
        let team = bob.teams.team(&id).unwrap();
        let t = team.lock().await;
        t.identity().member
    };
    {
        let team = alice.teams.team(&id).unwrap();
        let mut t = team.lock().await;
        t.remove_member(bob_member).await.unwrap();
    }
    alice
        .space()
        .await
        .spec_write("a/apres", |p| p.body = "Après son départ.".into())
        .unwrap();
    alice.changed().await;
    alice.round(&id).await;
    bob.round(&id).await;
    let b = bob.space().await;
    assert!(b.spec_read("a/apres").is_err(), "written after he left");
    assert!(b.spec_read("a/avant").is_ok(), "what he read stays");
}
