//! The shared Kanban (ESC-190): the format of a shared ticket's file, its schema, and how the
//! tickets of a project and the files of its folder are brought in step.

use std::collections::BTreeMap;

use crate::board;
use crate::core_tests::harness;
use crate::i18n::Lang;
use crate::model::*;
use crate::sharing::{self as sh, Read, TicketDoc};

fn alice() -> Person {
    Person {
        name: "Alice Martin".into(),
        id: "a91f03c2".into(),
    }
}

fn bob() -> Person {
    Person {
        name: "Bob Durand".into(),
        id: "5d0e77b1".into(),
    }
}

fn doc() -> TicketDoc {
    TicketDoc {
        version: 1,
        id: "q4w7c2na".into(),
        key: "DEM-7".into(),
        title: "Journal d’audit des connexions".into(),
        status: Column::Todo,
        author: alice(),
        taken_by: None,
        updated_by: Some(alice()),
        created: "2026-10-26T09:12:04+01:00".into(),
        updated: "2026-10-26T15:02:40+01:00".into(),
        after: vec!["p2v8c4ma".into()],
        description:
            "Chaque tentative laisse une ligne : qui, quand.\n\nD’où, et si elle a réussi.".into(),
        criteria: vec![
            "Chaque connexion, réussie ou non, est journalisée".into(),
            "Le journal se filtre par compte".into(),
            "Tests verts".into(),
        ],
        images: vec![SharedMedia {
            name: "maquette.png".into(),
            file: "3f2a9c01b7de.png".into(),
        }],
        files: vec![],
        external: Some(SharedExternal {
            source: "jira".into(),
            key: "ACME-41".into(),
            url: "https://acme.atlassian.net/browse/ACME-41".into(),
        }),
        outcome: None,
    }
}

fn ok(text: &str) -> TicketDoc {
    match sh::read(text) {
        Read::Ok(f, false) => *f,
        other => panic!("not read: {other:?}\n{text}"),
    }
}

#[test]
fn a_file_written_reads_back_as_it_was_whatever_its_texts() {
    let f = doc();
    let text = sh::write(&f);
    assert_eq!(ok(&text), f, "{text}");
    // Texts that YAML would read otherwise: quoted, and read back the same.
    let tricky = TicketDoc {
        title: "yes: « on » # pas un commentaire".into(),
        key: "DEM-12".into(),
        description: "  commence par des espaces\net finit par un deux-points:".into(),
        criteria: vec![
            "- un tiret".into(),
            "true".into(),
            "12".into(),
            "a, b".into(),
        ],
        outcome: Some("⤵ Mergé dans main · squash".into()),
        status: Column::Done,
        taken_by: Some(bob()),
        external: None,
        ..doc()
    };
    let text = sh::write(&tricky);
    assert_eq!(ok(&text), tricky, "{text}");
}

#[test]
fn a_file_says_what_it_is_has_its_schema_and_keeps_its_order() {
    let text = sh::write(&doc());
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("# "));
    assert!(lines[1].contains("never"), "{text}");
    assert_eq!(
        lines[2],
        format!("# yaml-language-server: $schema={}", sh::SCHEMA_URL)
    );
    // Its fields in the order of FIELDS, one change of status one line.
    let at = |k: &str| {
        text.find(&format!("\n{k}:"))
            .unwrap_or_else(|| panic!("{k}"))
    };
    let order: Vec<usize> = sh::FIELDS.iter().map(|f| at(f.name)).collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{text}");
    let moved = sh::write(&TicketDoc {
        status: Column::Review,
        ..doc()
    });
    let changed = text
        .lines()
        .zip(moved.lines())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(changed, 1);
    // A long text over several lines: a literal block.
    assert!(
        text.contains("description: |-\n  Chaque tentative"),
        "{text}"
    );
}

/// The schema the repository publishes (`schemas/escouade-ticket.schema.json`, served by the site)
/// is the one FIELDS make: `ESCOUADE_WRITE_SCHEMA=1` writes it again.
#[test]
fn the_published_schema_is_up_to_date_and_knows_every_field() {
    let schema = sh::schema();
    for f in sh::FIELDS {
        assert!(schema["properties"][f.name].is_object(), "{}", f.name);
        assert!(!f.about.is_empty());
    }
    let required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        required,
        ["version", "id", "key", "title", "status", "author"]
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let made = serde_json::to_string_pretty(&schema).unwrap() + "\n";
    let files = [
        root.join("schemas/escouade-ticket.schema.json"),
        root.join("website/public/schemas/escouade-ticket.schema.json"),
    ];
    if std::env::var_os("ESCOUADE_WRITE_SCHEMA").is_some() {
        for f in &files {
            std::fs::write(f, &made).unwrap();
        }
    }
    // Compared as JSON: the repository's formatter lays the files out its own way.
    for f in &files {
        let published: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(f).unwrap_or_default())
                .unwrap_or_default();
        assert_eq!(published, schema, "{} is not up to date", f.display());
    }
}

#[test]
fn a_file_in_conflict_or_that_cannot_be_read_says_so_and_where() {
    let text = sh::write(&doc());
    let conflicted = text.replace(
        "status: todo\n",
        "<<<<<<< HEAD\nstatus: todo\n=======\nstatus: done\n>>>>>>> origin/main\n",
    );
    assert_eq!(sh::read(&conflicted), Read::Conflict);
    // Each side of it, as git would keep it.
    assert_eq!(
        ok(&sh::conflict_side(&conflicted, true)).status,
        Column::Todo
    );
    assert_eq!(
        ok(&sh::conflict_side(&conflicted, false)).status,
        Column::Done
    );
    // A field of the wrong kind: the line in cause.
    let bad = text.replace("criteria:\n", "criteria: { a: 1 }\nwas_criteria:\n");
    match sh::read(&bad) {
        Read::Invalid(Some(line), why) => {
            assert!(why.contains("criteria"), "{why}");
            let at = bad
                .lines()
                .position(|l| l.starts_with("criteria:"))
                .unwrap() as u32
                + 1;
            assert_eq!(line, at);
        }
        other => panic!("{other:?}"),
    }
    // A field it must have, or a status it does not know.
    assert!(
        matches!(sh::read(&text.replace("title:", "titre:")), Read::Invalid(_, why) if why.contains("title"))
    );
    assert!(matches!(
        sh::read(&text.replace("status: todo", "status: blocked")),
        Read::Invalid(..)
    ));
    // Not YAML at all.
    assert!(matches!(sh::read("id: [\n"), Read::Invalid(..)));
    // An id that would leave the folder.
    assert!(matches!(
        sh::read(&text.replace("id: q4w7c2na", "id: ../x")),
        Read::Invalid(..)
    ));
}

#[test]
fn a_file_of_a_newer_version_is_read_for_what_is_known() {
    let text = sh::write(&doc())
        .replace("version: 1", "version: 2")
        .replace("status: todo", "status: parked")
        + "priority: high\n";
    match sh::read(&text) {
        Read::Ok(f, true) => {
            assert_eq!(f.title, doc().title);
            assert_eq!(f.status, Column::Todo);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn ids_names_and_people() {
    let id = sh::new_share_id();
    assert_eq!(id.len(), 8);
    assert!(id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    assert!(!id.contains(['0', '1', 'o', 'l']));
    assert_ne!(sh::new_share_id(), id);
    assert_eq!(sh::id_of_file(&format!("{id}.yml")), Some(id.as_str()));
    assert_eq!(sh::id_of_file("notes.md"), None);
    assert_eq!(sh::id_of_file("../x.yml"), None);
    // The same e-mail, the same id, in any case; the e-mail written nowhere.
    let a = sh::person_id("Alice@Acme.fr").unwrap();
    assert_eq!(a.len(), 8);
    assert_eq!(sh::person_id(" alice@acme.fr ").unwrap(), a);
    assert_ne!(sh::person_id("bob@acme.fr").unwrap(), a);
    assert_eq!(sh::person_id(""), None);
    assert!(!a.contains("alice"));
}

// ---------- reconcile ----------

const P: &str = "p1";

fn ticket(key: &str, title: &str) -> Ticket {
    Ticket {
        id: crate::model::new_id(),
        project_id: P.into(),
        key: key.into(),
        title: title.into(),
        criteria: board::criteria_from(&["Tests verts".to_string()]),
        max_loops: 5,
        created_at: 1_761_466_324_000,
        ..Default::default()
    }
}

fn shared_by(t: &mut Ticket, who: &Person, sid: &str) {
    t.shared = Some(Shared {
        id: sid.into(),
        key: t.key.clone(),
        author: who.clone(),
        mine: true,
        ..Default::default()
    });
}

fn sync(
    tickets: &mut Vec<Ticket>,
    files: &mut BTreeMap<String, String>,
    me: &Person,
) -> sh::Outcome {
    let out = sh::reconcile(tickets, P, files, me, "2026-10-26T15:00:00+01:00", Lang::Fr);
    for (sid, text, _) in &out.writes {
        files.insert(sid.clone(), text.clone());
    }
    out
}

#[test]
fn a_shared_ticket_writes_its_file_then_at_each_status_and_only_then() {
    let mut tickets = vec![ticket("DEM-7", "Journal")];
    shared_by(&mut tickets[0], &bob(), "q4w7c2na");
    let mut files = BTreeMap::new();
    let out = sync(&mut tickets, &mut files, &bob());
    assert_eq!(out.writes.len(), 1);
    let f = ok(&files["q4w7c2na"]);
    assert_eq!(f.author, bob());
    assert_eq!(f.status, Column::Todo);
    assert_eq!(f.key, "DEM-7");
    // Nothing changed: nothing written.
    assert!(sync(&mut tickets, &mut files, &bob()).writes.is_empty());
    // Its agent took it: written, with who took it.
    tickets[0].column = Column::Doing;
    tickets[0].started_at = Some(1);
    tickets[0].agent_id = Some("a1".into());
    let out = sync(&mut tickets, &mut files, &bob());
    assert_eq!(out.writes.len(), 1);
    let f = ok(&files["q4w7c2na"]);
    assert_eq!(f.status, Column::Doing);
    assert_eq!(f.taken_by, Some(bob()));
    // A loop more, its criteria met: not a status, nothing written.
    tickets[0].iteration = 2;
    tickets[0].loops = 2;
    tickets[0].criteria[0].ok = true;
    assert!(sync(&mut tickets, &mut files, &bob()).writes.is_empty());
    // Validated: written, with what its validation did.
    tickets[0].column = Column::Done;
    tickets[0].outcome = Some("Mergé dans main · squash".into());
    sync(&mut tickets, &mut files, &bob());
    let f = ok(&files["q4w7c2na"]);
    assert_eq!(f.status, Column::Done);
    assert_eq!(f.outcome.as_deref(), Some("Mergé dans main · squash"));
}

#[test]
fn another_ones_ticket_comes_in_todo_with_its_author_never_started_by_the_autopilot() {
    // Alice's machine shares DEM-12.
    let mut hers = vec![ticket("DEM-12", "Export PDF")];
    shared_by(&mut hers[0], &alice(), "h8m2v6kd");
    let mut files = BTreeMap::new();
    sync(&mut hers, &mut files, &alice());
    // Bob pulls: a ticket of his board, in « À faire », Alice's.
    let mut his = vec![ticket("DEM-3", "Le sien")];
    let out = sync(&mut his, &mut files, &bob());
    assert!(out.writes.is_empty(), "reading writes nothing");
    let t = his.iter().find(|t| t.shared.is_some()).unwrap().clone();
    assert_eq!(t.key, "DEM-12");
    assert_eq!(t.title, "Export PDF");
    assert_eq!(t.column, Column::Todo);
    let s = t.shared.as_ref().unwrap();
    assert_eq!(s.author, alice());
    assert!(!s.mine);
    assert!(sh::theirs(&t));
    // His autopilot starts his own ticket, never hers; « Lancer » does.
    let board = BoardSettings {
        autopilot: true,
        max_parallel: 3,
        ..Default::default()
    };
    let started = board::to_start(&his, P, &board, |_| false);
    assert_eq!(started, vec![his[0].id.clone()]);
    let forced: Vec<Ticket> = his
        .iter()
        .map(|x| Ticket {
            forced: x.shared.is_some(),
            ..x.clone()
        })
        .collect();
    assert!(board::to_start(&forced, P, &board, |_| false).contains(&t.id));
    // Read again: still one ticket, nothing written.
    let out = sync(&mut his, &mut files, &bob());
    assert!(out.writes.is_empty() && out.changed.is_empty(), "{out:?}");
    assert_eq!(his.len(), 2);
    // And an agent of Escouade may not start it.
    assert!(sh::start_refusal(Lang::Fr, &t)
        .unwrap()
        .contains("Alice Martin"));
    assert!(sh::start_refusal(Lang::Fr, &his[0]).is_none());
}

#[test]
fn bob_starts_alices_ticket_and_its_status_comes_back_to_her() {
    let mut hers = vec![ticket("DEM-12", "Export PDF")];
    shared_by(&mut hers[0], &alice(), "h8m2v6kd");
    let mut files = BTreeMap::new();
    sync(&mut hers, &mut files, &alice());
    let mut his = Vec::new();
    sync(&mut his, &mut files, &bob());
    // Started by hand on Bob's machine: his file says so.
    his[0].column = Column::Doing;
    his[0].started_at = Some(2);
    his[0].agent_id = Some("b1".into());
    sync(&mut his, &mut files, &bob());
    let f = ok(&files["h8m2v6kd"]);
    assert_eq!(f.status, Column::Doing);
    assert_eq!(f.taken_by, Some(bob()));
    assert_eq!(f.author, alice(), "it keeps its author");
    // Alice pulls: « En cours », taken by Bob; her autopilot leaves it.
    let out = sync(&mut hers, &mut files, &alice());
    assert!(out.writes.is_empty(), "{out:?}");
    assert_eq!(hers[0].column, Column::Doing);
    assert_eq!(hers[0].shared.as_ref().unwrap().taken_by, Some(bob()));
    // Under way at Bob's: not taken up here, and no place taken here.
    assert!(sh::elsewhere(&hers[0]));
    let board = BoardSettings {
        autopilot: true,
        max_parallel: 1,
        ..Default::default()
    };
    let mut more = hers.clone();
    more.push(ticket("DEM-13", "Le suivant"));
    assert_eq!(board::to_start(&more, P, &board, |_| false).len(), 1);
    // Bob validates it; Alice pulls: « Terminé », with what was done.
    his[0].column = Column::Done;
    his[0].outcome = Some("PR #41 → main".into());
    sync(&mut his, &mut files, &bob());
    sync(&mut hers, &mut files, &alice());
    assert_eq!(hers[0].column, Column::Done);
    assert_eq!(hers[0].outcome.as_deref(), Some("PR #41 → main"));
    assert!(hers[0].done_at.is_some());
    assert!(sync(&mut hers, &mut files, &alice()).writes.is_empty());
}

#[test]
fn a_file_changed_outside_is_followed_unless_an_agent_here_holds_it() {
    let mut tickets = vec![ticket("DEM-7", "Journal")];
    shared_by(&mut tickets[0], &bob(), "q4w7c2na");
    let mut files = BTreeMap::new();
    sync(&mut tickets, &mut files, &bob());
    // Alice edits the title and takes it, at hers.
    let mut f = ok(&files["q4w7c2na"]);
    f.title = "Journal d’audit".into();
    f.status = Column::Doing;
    f.taken_by = Some(alice());
    files.insert("q4w7c2na".into(), sh::write(&f));
    sync(&mut tickets, &mut files, &bob());
    assert_eq!(tickets[0].title, "Journal d’audit");
    assert_eq!(tickets[0].column, Column::Doing);
    assert!(tickets[0].shared.as_ref().unwrap().mine, "still Bob's");
    // Bob's agent had it too: his column stays, and his next write says he took it.
    tickets[0].column = Column::Review;
    tickets[0].agent_id = Some("b1".into());
    tickets[0].started_at = Some(3);
    f.status = Column::Todo;
    files.insert("q4w7c2na".into(), sh::write(&f));
    sync(&mut tickets, &mut files, &bob());
    assert_eq!(tickets[0].column, Column::Review);
    sync(&mut tickets, &mut files, &bob());
    let now = ok(&files["q4w7c2na"]);
    assert_eq!(now.status, Column::Review);
    assert_eq!(now.taken_by, Some(bob()));
}

#[test]
fn a_file_in_conflict_is_neither_read_nor_written_until_settled() {
    let mut tickets = vec![ticket("DEM-7", "Journal")];
    shared_by(&mut tickets[0], &bob(), "q4w7c2na");
    let mut files = BTreeMap::new();
    sync(&mut tickets, &mut files, &bob());
    let text = files["q4w7c2na"].clone();
    let conflicted = text.replace(
        "status: todo\n",
        "<<<<<<< HEAD\nstatus: todo\n=======\nstatus: done\n>>>>>>> theirs\n",
    );
    files.insert("q4w7c2na".into(), conflicted.clone());
    sync(&mut tickets, &mut files, &bob());
    let s = tickets[0].shared.clone().unwrap();
    assert!(s.conflict && s.problem.is_some());
    assert_eq!(tickets[0].column, Column::Todo, "its last state kept");
    // A status changes here meanwhile: the file is left alone.
    tickets[0].column = Column::Doing;
    assert!(sync(&mut tickets, &mut files, &bob()).writes.is_empty());
    assert_eq!(files["q4w7c2na"], conflicted);
    // Settled by hand (their side): read again.
    files.insert("q4w7c2na".into(), sh::conflict_side(&conflicted, false));
    sync(&mut tickets, &mut files, &bob());
    assert!(tickets[0].shared.as_ref().unwrap().problem.is_none());
    assert_eq!(tickets[0].column, Column::Done);
}

#[test]
fn an_unreadable_file_of_nobody_here_is_a_card_that_never_starts() {
    let mut tickets = Vec::new();
    let mut files = BTreeMap::new();
    files.insert(
        "x3t9f5rb".into(),
        "version: 1\nid: x3t9f5rb\ncriteria: { a: 1 }\n".into(),
    );
    let out = sync(&mut tickets, &mut files, &bob());
    assert!(out.writes.is_empty());
    assert_eq!(tickets.len(), 1);
    let t = &tickets[0];
    assert_eq!(t.title, "x3t9f5rb.yml");
    assert!(t.shared.as_ref().unwrap().problem.is_some());
    let board = BoardSettings {
        autopilot: true,
        ..Default::default()
    };
    let forced = vec![Ticket {
        forced: true,
        ..t.clone()
    }];
    assert!(board::to_start(&forced, P, &board, |_| false).is_empty());
}

#[test]
fn a_ticket_no_longer_shared_goes_unless_it_was_taken_here() {
    let mut hers = vec![ticket("DEM-12", "Export"), ticket("DEM-13", "Avoirs")];
    shared_by(&mut hers[0], &alice(), "h8m2v6kd");
    shared_by(&mut hers[1], &alice(), "k3m9p2xw");
    let mut files = BTreeMap::new();
    sync(&mut hers, &mut files, &alice());
    let mut his = Vec::new();
    sync(&mut his, &mut files, &bob());
    assert_eq!(his.len(), 2);
    let taken = his.iter().position(|t| t.key == "DEM-13").unwrap();
    his[taken].started_at = Some(4);
    // Alice stops sharing both.
    files.clear();
    let out = sync(&mut his, &mut files, &bob());
    assert_eq!(out.removed.len(), 1);
    assert_eq!(his.len(), 1);
    assert_eq!(his[0].key, "DEM-13");
    let s = his[0].shared.as_ref().unwrap();
    assert!(s.left && s.gone);
    // Her own: not written again while its file is gone.
    let out = sync(&mut hers, &mut files, &alice());
    assert!(out.writes.is_empty());
    assert!(hers.iter().all(|t| t.shared.as_ref().unwrap().gone));
}

#[test]
fn keys_after_and_imports_of_the_same_outside_ticket() {
    let mut hers = vec![ticket("DEM-6", "Limiter"), ticket("DEM-7", "Journal")];
    shared_by(&mut hers[0], &alice(), "p2v8c4ma");
    shared_by(&mut hers[1], &alice(), "q4w7c2na");
    hers[1].after = vec![hers[0].id.clone()];
    hers[1].external = Some(ExternalRef {
        service: Service::JIRA,
        key: "ACME-41".into(),
        url: "https://acme/ACME-41".into(),
        ..Default::default()
    });
    let mut files = BTreeMap::new();
    sync(&mut hers, &mut files, &alice());
    assert_eq!(ok(&files["q4w7c2na"]).after, vec!["p2v8c4ma".to_string()]);
    assert_eq!(ok(&files["q4w7c2na"]).external.unwrap().key, "ACME-41");
    // Bob has a DEM-6 of his own already, and imported ACME-41 before his pull.
    let mut mine = ticket("DEM-6", "Le mien");
    mine.created_at = 2;
    let mut imported = ticket("DEM-9", "ACME-41 importé");
    imported.external = Some(ExternalRef {
        service: Service::JIRA,
        key: "ACME-41".into(),
        ..Default::default()
    });
    let mut his = vec![mine, imported];
    sync(&mut his, &mut files, &bob());
    // His DEM-6 keeps its key, hers is told apart.
    assert!(his.iter().any(|t| t.key == "DEM-6" && t.shared.is_none()));
    let hers6 = his.iter().find(|t| t.key == "DEM-6@alice").unwrap().clone();
    // His import, not started, became the shared one: no twin.
    assert_eq!(
        his.len(),
        3,
        "{:?}",
        his.iter().map(|t| &t.key).collect::<Vec<_>>()
    );
    let journal = his
        .iter()
        .find(|t| t.shared.as_ref().is_some_and(|s| s.id == "q4w7c2na"))
        .unwrap();
    assert_eq!(journal.title, "Journal");
    // « Après » found by the files' ids.
    assert_eq!(journal.after, vec![hers6.id.clone()]);
    // Nothing to write back: what he read is what the files say.
    assert!(sync(&mut his, &mut files, &bob()).writes.is_empty());
}

// ---------- on the core: two machines on one repository ----------

fn set_me(h: &crate::core_tests::Harness, who: &Person) {
    let mut s = h.core.settings.write();
    s.user_name = who.name.clone();
    s.user_id = who.id.clone();
    s.name_asked = true;
}

#[tokio::test]
async fn alice_shares_bob_sees_it_in_todo_and_it_is_never_committed() {
    let a = harness("share-alice");
    let b = harness("share-bob");
    set_me(&a, &alice());
    set_me(&b, &bob());
    let (pa, repo) = a.project(false).await;
    {
        let mut projects = a.core.projects.write();
        projects
            .iter_mut()
            .find(|p| p.id == pa.id)
            .unwrap()
            .board
            .autopilot = false;
    }
    let t = a
        .core
        .ticket_create(
            &pa.id,
            crate::tickets::TicketDraft {
                title: "Export des factures en PDF".into(),
                criteria: vec!["Chaque facture s’exporte en PDF".into()],
                shared: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let s = t.shared.clone().expect("shared");
    assert!(s.mine);
    let file = repo.join(sh::DIR).join(format!("{}.yml", s.id));
    let f = ok(&std::fs::read_to_string(&file).unwrap());
    assert_eq!(f.author, alice());
    assert_eq!(f.key, t.key);
    // Escouade commits nothing: the file waits in git's status, untracked.
    let status = crate::core_tests::git(&repo, &["status", "--porcelain"]);
    assert!(status.contains(".escouade/"), "{status}");
    let log = crate::core_tests::git(&repo, &["log", "--oneline"]);
    assert_eq!(log.lines().count(), 1, "{log}");
    // Bob opens the same folder: Alice's ticket in « À faire », hers.
    let pb = b
        .core
        .create_project(&repo.to_string_lossy(), "demo", "", false, None, true)
        .await
        .unwrap();
    b.core.sync_shared(&pb.id);
    let seen: Vec<Ticket> = b
        .core
        .tickets
        .read()
        .iter()
        .filter(|x| x.project_id == pb.id)
        .cloned()
        .collect();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].title, "Export des factures en PDF");
    assert_eq!(seen[0].column, Column::Todo);
    let sb = seen[0].shared.as_ref().unwrap();
    assert!(!sb.mine);
    assert_eq!(sb.author.name, "Alice Martin");
    // Stopping the share deletes the file, to be committed; the ticket stays hers.
    a.core.ticket_share(&t.id, false).unwrap();
    assert!(!file.exists());
    assert!(a.core.ticket(&t.id).unwrap().shared.is_none());
    b.core.sync_shared(&pb.id);
    assert!(b.core.tickets.read().iter().all(|x| x.project_id != pb.id));
}

#[tokio::test]
async fn sharing_without_a_name_is_refused_and_the_name_of_the_project_comes_first() {
    let h = harness("share-name");
    let (p, _r) = h.project(false).await;
    let t = h
        .core
        .ticket_create(
            &p.id,
            crate::tickets::TicketDraft {
                title: "Un ticket".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    {
        let mut s = h.core.settings.write();
        s.user_name.clear();
        s.user_id = "5d0e77b1".into();
    }
    // A machine where git has no name either.
    h.core
        .git_names
        .lock()
        .insert(p.path.clone(), String::new());
    assert!(h.core.ticket_share(&t.id, true).is_err());
    h.core
        .git_names
        .lock()
        .insert(p.path.clone(), "Bobby (git)".into());
    let project = h.core.project(&p.id).unwrap();
    assert_eq!(h.core.me_in(&project).name, "Bobby (git)");
    h.core.settings.write().user_name = "Bob Durand".into();
    let project = Project {
        user_name: "Robert".into(),
        ..h.core.project(&p.id).unwrap()
    };
    assert_eq!(h.core.me_in(&project).name, "Robert");
    assert_eq!(
        h.core.me_in(&h.core.project(&p.id).unwrap()).name,
        "Bob Durand"
    );
    let shared = h.core.ticket_share(&t.id, true).unwrap();
    assert_eq!(shared.shared.unwrap().author.name, "Bob Durand");
}

/// Without a name given (« Plus tard », ESC-243), sharing takes the name git knows in the project's
/// folder — a repository's own identity before the global one — and writes it in the ticket's file
/// alone, never in `.escouade.yml`.
#[tokio::test]
async fn without_a_name_sharing_takes_the_name_git_knows_in_the_projects_folder() {
    let h = harness("share-git-name");
    let (p, r) = h.project(false).await;
    let git = |args: &[&str]| {
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(&r)
            .args(args)
            .status()
            .unwrap()
            .success())
    };
    // The test repository has its own identity, whatever the machine's global one.
    git(&["config", "user.name", "Alice du dépôt"]);
    let t = h
        .core
        .ticket_create(
            &p.id,
            crate::tickets::TicketDraft {
                title: "Un ticket".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    {
        let mut s = h.core.settings.write();
        s.user_name.clear();
        s.user_id = "a91f03c2".into();
    }
    assert_eq!(
        h.core.git_user_name(Some(p.id.clone())).await,
        "Alice du dépôt"
    );
    let shared = h.core.ticket_share(&t.id, true).unwrap();
    assert_eq!(
        shared.shared.as_ref().unwrap().author.name,
        "Alice du dépôt"
    );
    let file = r
        .join(".escouade/tickets")
        .join(format!("{}.yml", shared.shared.unwrap().id));
    assert!(std::fs::read_to_string(file)
        .unwrap()
        .contains("Alice du dépôt"));
    // Never in the project's shared settings, nor in the project or the app's settings.
    let yml = std::fs::read_to_string(r.join(".escouade.yml")).unwrap_or_default();
    assert!(!yml.contains("Alice"), "{yml}");
    assert!(h.core.project(&p.id).unwrap().user_name.is_empty());
    assert!(h.core.settings.read().user_name.is_empty());
    // Read again when the window asks: a name changed in git is the new one.
    git(&["config", "user.name", "Alice Martin"]);
    assert_eq!(
        h.core.git_user_name(Some(p.id.clone())).await,
        "Alice Martin"
    );
    assert_eq!(
        h.core.me_in(&h.core.project(&p.id).unwrap()).name,
        "Alice Martin"
    );
}

/// The shared tickets' files block no merge (choice 121): `git status` lines about them are left out.
#[test]
fn the_tickets_files_are_told_apart_in_git_status() {
    use crate::git::is_shared_ticket_line as tickets;
    assert!(tickets(" M .escouade/tickets/q4w7c2na.yml"));
    assert!(tickets("A  api/.escouade/tickets/q4w7c2na/maquette.png"));
    assert!(tickets(
        "R  .escouade/tickets/a.yml -> .escouade/tickets/b.yml"
    ));
    assert!(tickets(" M .escouade/memory/e2e-build.md"));
    assert!(tickets("?? api/.escouade/memory/MEMORY.md"));
    assert!(!tickets(" M .escouade.yml"));
    assert!(!tickets(" M src/app.ts"));
}

#[test]
fn an_id_that_is_a_device_name_of_windows_is_no_file_name() {
    for id in ["con", "CON", "aux", "nul", "prn", "com1", "LPT9", "com0"] {
        assert!(!sh::plain_id(id), "{id}");
        assert_eq!(sh::id_of_file(&format!("{id}.yml")), None);
    }
    for id in ["q4w7c2na", "com10", "console", "con-1", "a_b"] {
        assert!(sh::plain_id(id), "{id}");
    }
}

#[test]
fn a_folder_of_tickets_that_is_a_link_is_neither_read_nor_written() {
    let root = crate::paths::test_dir("sh-link-dir");
    let outside = root.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let project = root.join("project");
    std::fs::create_dir_all(project.join(".escouade")).unwrap();
    let dir = project.join(sh::DIR);
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&outside, &dir).is_ok();
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_dir(&outside, &dir).is_ok()
        || std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(dir.to_string_lossy().replace('/', "\\"))
            .arg(outside.to_string_lossy().replace('/', "\\"))
            .output()
            .is_ok_and(|o| o.status.success());
    if !made {
        eprintln!("no symbolic link allowed here: skipped");
        return;
    }
    assert!(crate::workflowfiles::unsafe_dir(&dir).is_some());
}
