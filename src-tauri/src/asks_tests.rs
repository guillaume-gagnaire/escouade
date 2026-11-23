//! The box « À toi » (ESC-509, docs/INBOX.md): a request's model, bounds and states, its files (their type
//! read from their content, their bounds, their copy that outlives the worktree), the answer given back to
//! its author at rest and at work, a ticket that waits for one, the delegation and its refusals, the
//! requests kept across a restart and while the MCP server is off.

use crate::asks::*;
use crate::core::Core;
use crate::core_tests::{harness, Harness};
use crate::i18n::Lang;
use crate::model::*;
use crate::paths::DataDir;
use std::path::Path;
use std::sync::Arc;
use tauri::test::mock_app;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
const PDF: &[u8] = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n";

fn choice(title: &str) -> AskDraft {
    AskDraft {
        kind: AskKind::Choice,
        title: title.into(),
        options: ["Colonnes", "Cartes", "Tableau"]
            .iter()
            .map(|l| AskOption {
                label: (*l).into(),
                ..Default::default()
            })
            .collect(),
        wake: true,
        ..Default::default()
    }
}

fn confirm(title: &str, gate: bool) -> AskDraft {
    AskDraft {
        kind: AskKind::Confirm,
        title: title.into(),
        gate,
        wake: true,
        ..Default::default()
    }
}

fn bytes(name: &str, b: &[u8], option: Option<&str>) -> Incoming {
    Incoming {
        name: name.into(),
        declared: None,
        option: option.map(str::to_string),
        source: Source::Bytes(b.to_vec()),
    }
}

fn agent_author(id: &str) -> AskAuthor {
    AskAuthor {
        agent: Some(id.into()),
        name: format!("agent-{id}"),
        chef: false,
    }
}

fn external() -> AskAuthor {
    AskAuthor {
        agent: None,
        name: "Claude · client MCP".into(),
        chef: false,
    }
}

/// An agent of `p` kept as the app keeps one, without a process.
fn put_agent(h: &Harness, p: &Project, name: &str) -> String {
    let meta = AgentMeta {
        id: format!("ag-{name}"),
        project_id: p.id.clone(),
        name: name.into(),
        cwd: p.path.clone(),
        account: crate::accounts::PRINCIPAL.into(),
        ..Default::default()
    };
    let rt = crate::agent::AgentRt::new(meta.clone(), &h.core.data.conversations());
    h.core
        .agents
        .write()
        .insert(meta.id.clone(), Arc::new(parking_lot::Mutex::new(rt)));
    meta.id
}

fn on(h: &Harness) {
    h.core.settings.write().mcp_enabled = true;
}

fn files_on_disk(h: &Harness) -> usize {
    fn count(p: &Path) -> usize {
        std::fs::read_dir(p)
            .map(|d| {
                d.flatten()
                    .map(|e| {
                        if e.path().is_dir() {
                            count(&e.path())
                        } else {
                            1
                        }
                    })
                    .sum()
            })
            .unwrap_or(0)
    }
    count(&h.core.data.asks())
}

// ---------- the model and its bounds ----------

#[test]
fn a_request_is_checked_against_its_bounds_in_one_sentence() {
    let fr = Lang::Fr;
    assert!(checked(fr, choice("Choisis la direction")).is_ok());
    let long = "x".repeat(141);
    assert!(checked(fr, choice(&long))
        .unwrap_err()
        .contains("140 caractères, sur une ligne"));
    assert!(checked(fr, choice("deux\nlignes"))
        .unwrap_err()
        .contains("une ligne"));
    let mut one = choice("Un seul");
    one.options.truncate(1);
    assert_eq!(
        checked(fr, one).unwrap_err(),
        "Une demande a 2 à 6 options."
    );
    let mut seven = choice("Sept");
    seven.options = (0..7)
        .map(|i| AskOption {
            label: format!("O{i}"),
            ..Default::default()
        })
        .collect();
    assert_eq!(
        checked(Lang::En, seven).unwrap_err(),
        "A choice has 2 to 6 options."
    );
    // Ids given by rank.
    let ok = checked(fr, choice("Ids")).unwrap();
    assert_eq!(
        ok.options.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    let mut rec = choice("Reco");
    rec.recommend = Some(AskRecommend {
        option: Some("z".into()),
        ..Default::default()
    });
    assert!(checked(fr, rec)
        .unwrap_err()
        .contains("n’est pas une des options"));
    let mut task = AskDraft {
        kind: AskKind::Task,
        title: "Essai".into(),
        steps: (0..11).map(|i| format!("étape {i}")).collect(),
        ..Default::default()
    };
    assert!(checked(fr, task.clone()).unwrap_err().contains("10 étapes"));
    task.steps.truncate(3);
    assert!(checked(fr, task).is_ok());
    let mut days = choice("Durée");
    days.expires_in_days = Some(31);
    assert!(checked(fr, days).unwrap_err().contains("1 à 30 jours"));
}

#[test]
fn a_secret_in_a_request_is_refused() {
    for secret in [
        "ghp_0123456789abcdefghijklmnopqrstuvwxyz",
        "voici sk-ant-api03-abcdefghijklmnopqrstuvwxyz0123",
        "xoxb-1234567890-abcdef",
        "AKIAIOSFODNN7EXAMPLE",
        "-----BEGIN OPENSSH PRIVATE KEY-----\nb3Blbn",
    ] {
        let mut d = choice("Choisis");
        d.body = secret.into();
        assert!(
            checked(Lang::Fr, d)
                .unwrap_err()
                .contains("ressemble à un secret"),
            "{secret}"
        );
    }
    // Words that only look like the start of one are not.
    for fine in ["risk-free", "task-list", "sk-1", "le ghp_ seul"] {
        assert!(!looks_secret(fine), "{fine}");
    }
}

#[test]
fn a_files_type_is_its_contents() {
    assert_eq!(sniff(PNG, "a.png"), (AskFileKind::Image, "image/png"));
    assert_eq!(sniff(b"\xFF\xD8\xFF\xE0", "x.bin").0, AskFileKind::Image);
    assert_eq!(sniff(b"GIF89a....", "g").1, "image/gif");
    assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 ", "w").1, "image/webp");
    assert_eq!(
        sniff(b"\0\0\0\x1cftypavif\0\0", "a.avif"),
        (AskFileKind::Image, "image/avif")
    );
    assert_eq!(
        sniff(PDF, "rapport.txt"),
        (AskFileKind::Pdf, "application/pdf")
    );
    assert_eq!(
        sniff(b"PK\x03\x04....word/document.xml", "note.docx").0,
        AskFileKind::Word
    );
    assert_eq!(
        sniff(b"PK\x03\x04....", "maquettes.zip").1,
        "application/zip"
    );
    assert_eq!(sniff(b"# Titre\n", "lisez.md").0, AskFileKind::Markdown);
    assert_eq!(sniff(b"a,b\n1,2\n", "couts.csv").0, AskFileKind::Csv);
    // An SVG is a file, never an image: it may hold a script.
    assert_eq!(
        sniff(
            b"<svg xmlns='http://www.w3.org/2000/svg'><script/></svg>",
            "x.png"
        ),
        (AskFileKind::Other, "image/svg+xml")
    );
    assert_eq!(sniff(b"\0\x01\x02", "x.bin").0, AskFileKind::Other);
    assert!(claims_image("a.PNG", None));
    assert!(claims_image("a.dat", Some("image/jpeg")));
    assert!(!claims_image("a.svg", Some("image/svg+xml")));
}

// ---------- the files ----------

#[tokio::test]
async fn a_file_that_says_it_is_an_image_and_is_none_is_refused_and_nothing_is_left() {
    let h = harness("asks-lying-type");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    let e = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Choisis"),
            vec![
                bytes("ok.png", PNG, Some("a")),
                bytes("x.png", PDF, Some("b")),
            ],
        )
        .unwrap_err()
        .to_string();
    assert_eq!(
        e,
        "Le fichier « x.png » n’est pas une image PNG, JPEG, GIF, WebP ou AVIF."
    );
    // Neither the request nor a file of it.
    assert!(h.core.asks_all().is_empty());
    assert_eq!(files_on_disk(&h), 0);
    // A name whose extension lies about what is no image keeps its name, typed by its content.
    let kept = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Choisis"),
            vec![bytes("notes.txt", PDF, None)],
        )
        .unwrap();
    assert_eq!(kept.files[0].name, "notes.txt");
    assert_eq!(kept.files[0].kind, AskFileKind::Pdf);
}

#[tokio::test]
async fn the_files_bounds_are_held_before_reading() {
    let h = harness("asks-file-bounds");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    let many: Vec<Incoming> = (0..13)
        .map(|i| bytes(&format!("{i}.png"), PNG, None))
        .collect();
    let e = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, choice("Trop"), many)
        .unwrap_err();
    assert_eq!(
        e.to_string(),
        "12 fichiers au plus par demande (13 donnés)."
    );
    // 25 MB each, said from the length of the file opened before a byte is read.
    let dir = h.dir.join("gros");
    std::fs::create_dir_all(&dir).unwrap();
    let big = dir.join("rapport.pdf");
    let f = std::fs::File::create(&big).unwrap();
    f.set_len(FILE_MAX + 1).unwrap();
    let opened = std::fs::File::open(&big).unwrap();
    let e = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Gros"),
            vec![Incoming {
                name: "rapport.pdf".into(),
                declared: None,
                option: None,
                source: Source::File(opened, FILE_MAX + 1, None),
            }],
        )
        .unwrap_err();
    assert_eq!(e.to_string(), "« rapport.pdf » dépasse 25 Mo.");
    // 64 MB together.
    let third = vec![0u8; 22 << 20];
    let e = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Lourd"),
            vec![
                bytes("1.bin", &third, None),
                bytes("2.bin", &third, None),
                bytes("3.bin", &third, None),
            ],
        )
        .unwrap_err();
    assert!(e.to_string().contains("64 Mo"), "{e}");
    assert_eq!(files_on_disk(&h), 0);
    // The box holds 1 GB of files.
    h.core.asks.lock().list.push(Ask {
        id: "a-900".into(),
        project_id: p.id.clone(),
        state: AskState::Answered,
        author: external(),
        files: vec![AskFile {
            file: "x/y.bin".into(),
            size: BOX_MAX - 10,
            ..Default::default()
        }],
        expires_at: i64::MAX,
        ..Default::default()
    });
    let e = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Plein"),
            vec![bytes("a.png", PNG, None)],
        )
        .unwrap_err();
    assert!(e.to_string().contains("1 Go"), "{e}");
}

#[tokio::test]
async fn a_file_is_copied_and_outlives_the_worktree_it_came_from() {
    let h = harness("asks-outlives-worktree");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    let wt = h.dir.join("worktree");
    std::fs::create_dir_all(wt.join("design")).unwrap();
    let path = wt.join("design").join("b.png");
    std::fs::write(&path, PNG).unwrap();
    let opened = std::fs::File::open(&path).unwrap();
    let kept = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Choisis"),
            vec![
                Incoming {
                    name: "b.png".into(),
                    declared: Some("image/png".into()),
                    option: Some("b".into()),
                    source: Source::File(opened, PNG.len() as u64, None),
                },
                bytes("etude.pdf", PDF, None),
            ],
        )
        .unwrap();
    assert_eq!(kept.files.len(), 2);
    assert_eq!(kept.files[0].option.as_deref(), Some("b"));
    // The worktree goes.
    std::fs::remove_dir_all(&wt).unwrap();
    let (f, copy) = h
        .core
        .ask_file(&kept, Some(&kept.files[0].file), None)
        .unwrap();
    assert!(copy.starts_with(dunce(&h.core.data.ask_dir(&kept.id))));
    assert_eq!(std::fs::read(&copy).unwrap(), PNG);
    assert_eq!(f.kind, AskFileKind::Image);
    // The window reads it by the request and the file, never by a path.
    on(&h);
    let chunk = h
        .core
        .inbox_file(&kept.id, &kept.files[1].file, 0, None)
        .unwrap();
    assert_eq!(chunk.kind, AskFileKind::Pdf);
    assert_eq!(chunk.size, PDF.len() as u64);
    assert!(h
        .core
        .inbox_file(&kept.id, "../../state.json", 0, None)
        .is_err());
}

fn dunce(p: &Path) -> std::path::PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

#[tokio::test]
async fn the_files_of_a_closed_request_go_after_thirty_days_the_request_stays() {
    let h = harness("asks-files-purge");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    let kept = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Choisis"),
            vec![bytes("a.png", PNG, None)],
        )
        .unwrap();
    h.core
        .ask_withdraw(&Asker::Agent(a.clone()), &kept.id, "plus besoin")
        .unwrap();
    h.core.asks_sweep();
    assert!(h.core.ask_dir(&kept.id).exists(), "kept within 30 days");
    let day = 24 * 3600 * 1000;
    h.core.asks.lock().list[0].closed_at = Some(now_ms() - 31 * day);
    h.core.asks_sweep();
    let after = h.core.ask(&kept.id).unwrap();
    assert!(after.files_erased_at.is_some());
    assert!(!h.core.ask_dir(&kept.id).exists());
    assert!(h
        .core
        .ask_file(&after, None, Some(0))
        .unwrap_err()
        .to_string()
        .contains("effacés"));
    assert!(after.history.iter().any(|e| e.what == "filesErased"));
    // After 90 days, the request itself.
    h.core.asks.lock().list[0].closed_at = Some(now_ms() - 91 * day);
    h.core.asks_sweep();
    assert!(h.core.asks_all().is_empty());
}

// ---------- the bounds of the box ----------

#[tokio::test]
async fn an_agent_has_ten_open_requests_and_twenty_an_hour() {
    let h = harness("asks-open-bounds");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    for i in 0..10 {
        h.core
            .ask_deposit(
                agent_author(&a),
                &p.id,
                None,
                choice(&format!("Q{i}")),
                vec![],
            )
            .unwrap();
    }
    let e = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, choice("Onze"), vec![])
        .unwrap_err();
    assert!(
        e.to_string().starts_with("Tu as déjà 10 demandes ouvertes"),
        "{e}"
    );
    // Withdrawn, they make room; but not more than 20 in the hour.
    for x in h.core.asks_all() {
        h.core
            .ask_withdraw(&Asker::Agent(a.clone()), &x.id, "")
            .unwrap();
    }
    for i in 0..10 {
        h.core
            .ask_deposit(
                agent_author(&a),
                &p.id,
                None,
                choice(&format!("R{i}")),
                vec![],
            )
            .unwrap();
    }
    for x in h.core.asks_all().iter().filter(|x| x.state.waiting()) {
        h.core
            .ask_withdraw(&Asker::Agent(a.clone()), &x.id, "")
            .unwrap();
    }
    let e = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, choice("Vingt et un"), vec![])
        .unwrap_err();
    assert_eq!(
        e.to_string(),
        "20 dépôts dans l’heure : regroupe tes demandes."
    );
    // Each request has a number of its own.
    let ids: std::collections::HashSet<String> =
        h.core.asks_all().into_iter().map(|a| a.id).collect();
    assert_eq!(ids.len(), 20);
}

// ---------- the states ----------

#[tokio::test]
async fn an_answer_can_be_taken_back_until_it_goes_then_it_is_read() {
    let h = harness("asks-states");
    let (p, _) = h.project(false).await;
    on(&h);
    let kept = h
        .core
        .ask_deposit(external(), &p.id, None, choice("Choisis"), vec![])
        .unwrap();
    assert_eq!(kept.id, "a-1");
    assert_eq!(kept.state, AskState::Open);
    // An answer that is no option is refused.
    let wrong = AnswerIn {
        options: vec!["z".into()],
        ..Default::default()
    };
    assert!(h.core.ask_answer(&kept.id, wrong, None, vec![]).is_err());
    let b = AnswerIn {
        options: vec!["b".into()],
        note: Some("garde la courbe".into()),
        ..Default::default()
    };
    let answered = h
        .core
        .ask_answer(&kept.id, b.clone(), None, vec![])
        .unwrap();
    assert_eq!(answered.state, AskState::Answered);
    assert!(answered.answer.as_ref().unwrap().send_at.is_some());
    // Within its 10 seconds: taken back, open again.
    let again = h.core.ask_reopen(&kept.id).unwrap();
    assert_eq!(again.state, AskState::Open);
    assert!(again.answer.is_none());
    h.core.ask_answer(&kept.id, b, None, vec![]).unwrap();
    // Its time over, it goes; Claude outside Escouade has no conversation: kept for get_ask.
    h.core.asks.lock().list[0].answer.as_mut().unwrap().send_at = Some(now_ms() - 1);
    h.core.deliver_answers().await;
    let sent = h.core.ask(&kept.id).unwrap();
    assert_eq!(sent.state, AskState::Answered);
    assert!(sent.answer.as_ref().unwrap().sent_at.is_some());
    // Not read yet: it may still be taken back.
    assert!(h.core.ask_reopen(&kept.id).is_ok());
    h.core
        .ask_answer(
            &kept.id,
            AnswerIn {
                options: vec!["c".into()],
                ..Default::default()
            },
            None,
            vec![],
        )
        .unwrap();
    h.core.asks.lock().list[0].answer.as_mut().unwrap().send_at = Some(now_ms() - 1);
    h.core.deliver_answers().await;
    let read = h.core.ask_mark_read(&kept.id).unwrap();
    assert_eq!(read.state, AskState::Read);
    // Read, it no longer comes back.
    assert!(h
        .core
        .ask_reopen(&kept.id)
        .unwrap_err()
        .to_string()
        .contains("Changer d’avis"));
    let what: Vec<&str> = read.history.iter().map(|e| e.what.as_str()).collect();
    assert_eq!(
        what,
        [
            "deposited",
            "answered",
            "reopened",
            "answered",
            "sent",
            "reopened",
            "answered",
            "sent",
            "read"
        ]
    );
}

#[tokio::test]
async fn a_request_put_off_comes_back_one_past_its_date_expires_one_whose_ticket_went_is_taken_out()
{
    let h = harness("asks-sweep");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    let b = put_agent(&h, &p, "back");
    let t = Ticket {
        id: "t-1".into(),
        project_id: p.id.clone(),
        key: "DEM-1".into(),
        column: Column::Doing,
        ..Default::default()
    };
    h.core.tickets.write().push(t);
    let later = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, choice("Plus tard"), vec![])
        .unwrap();
    let old = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, choice("Périmée"), vec![])
        .unwrap();
    let orphan = h
        .core
        .ask_deposit(agent_author(&b), &p.id, None, choice("Orpheline"), vec![])
        .unwrap();
    let of_ticket = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            Some("t-1".into()),
            choice("Du ticket"),
            vec![],
        )
        .unwrap();
    assert!(of_ticket.ticket_started);
    h.core
        .ask_snooze(&later.id, Some(now_ms() + 60_000))
        .unwrap();
    assert_eq!(h.core.ask(&later.id).unwrap().state, AskState::Snoozed);
    {
        let mut box_ = h.core.asks.lock();
        box_.list
            .iter_mut()
            .find(|x| x.id == later.id)
            .unwrap()
            .snoozed_until = Some(now_ms() - 1);
        box_.list
            .iter_mut()
            .find(|x| x.id == old.id)
            .unwrap()
            .expires_at = now_ms() - 1;
    }
    h.core.agents.write().remove(&b);
    h.core.tickets.write()[0].column = Column::Todo;
    h.core.asks_sweep();
    let back = h.core.ask(&later.id).unwrap();
    assert_eq!(back.state, AskState::Open);
    assert!(back.woke);
    assert_eq!(h.core.ask(&old.id).unwrap().state, AskState::Expired);
    // Its agent deleted by hand, the project without a squad lead: it stays, with no living author
    // (ESC-585).
    let gone = h.core.ask(&orphan.id).unwrap();
    assert_eq!(gone.state, AskState::Open);
    assert_eq!(gone.author.agent, None);
    let handed = gone.handed.unwrap();
    assert_eq!(handed.cause, HandoverCause::Deleted);
    assert_eq!(handed.from, "agent-ag-back");
    let sent_back = h.core.ask(&of_ticket.id).unwrap();
    assert_eq!(sent_back.state, AskState::Retired);
    assert_eq!(
        sent_back.reason.as_deref(),
        Some("Son ticket a été renvoyé « À faire ».")
    );
}

// ---------- the answer given back ----------

/// The messages the fake `claude` read on its input, as text.
fn read_by_agent(h: &Harness, cwd: &Path) -> Vec<String> {
    h.stdin_messages(cwd)
        .iter()
        .filter_map(|m| m["message"]["content"].as_str().map(str::to_string))
        .collect()
}

fn due_now(h: &Harness, id: &str) {
    if let Some(x) = h.core.asks.lock().list.iter_mut().find(|x| x.id == id) {
        if let Some(ans) = x.answer.as_mut() {
            ans.send_at = Some(now_ms() - 1);
        }
    }
}

#[tokio::test]
async fn the_answer_wakes_its_agent_at_rest_marked_as_the_users_and_does_nothing_else() {
    let h = harness("asks-answer-at-rest");
    let (p, r) = h.project(false).await;
    on(&h);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&a, "Bonjour").await;
    let mut d = choice("Choisis la direction du tableau de bord");
    d.decision = true;
    let kept = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, d, vec![])
        .unwrap();
    let ans = AnswerIn {
        options: vec!["b".into()],
        note: Some("garde la courbe de A".into()),
        ..Default::default()
    };
    h.core.ask_answer(&kept.id, ans, None, vec![]).unwrap();
    // Not before its 10 seconds.
    h.core.deliver_answers().await;
    assert!(!read_by_agent(&h, &r)
        .iter()
        .any(|m| m.contains("[Escouade · ta demande")));
    due_now(&h, &kept.id);
    h.core.deliver_answers().await;
    h.wait("the answer read by the agent", |h| {
        read_by_agent(h, &r)
            .iter()
            .any(|m| m.contains("[Escouade · ta demande a-1"))
    })
    .await;
    let msg = read_by_agent(&h, &r)
        .into_iter()
        .find(|m| m.contains("[Escouade · ta demande"))
        .unwrap();
    assert!(msg.contains("L’utilisateur a choisi B · Cartes."), "{msg}");
    assert!(msg.contains("Son mot : « garde la courbe de A »"), "{msg}");
    assert!(msg.contains("propose_decision"), "{msg}");
    assert!(msg.contains("Rien d’autre n’a été fait"), "{msg}");
    // Its conversation shows it as an answer of the box.
    assert!(h
        .items(&a)
        .iter()
        .any(|i| i["kind"] == "user" && i["origin"] == "ask"));
    assert_eq!(h.core.ask(&kept.id).unwrap().state, AskState::Read);
    // A new turn ran on it, and nothing else started.
    h.wait("its turn", |h| !h.agent(&a).status.is_active())
        .await;
    assert!(h.core.tickets.read().is_empty());
}

#[tokio::test]
async fn the_answer_reaches_its_agent_during_its_turn() {
    let h = harness("asks-answer-at-work");
    let (p, r) = h.project(false).await;
    on(&h);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&a, "slow 3s".into(), vec![])
        .await
        .unwrap();
    h.wait("at work", |h| h.agent(&a).status.is_active()).await;
    let kept = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            confirm("Publier ?", true),
            vec![],
        )
        .unwrap();
    h.core
        .ask_answer(
            &kept.id,
            AnswerIn {
                yes: Some(true),
                ..Default::default()
            },
            None,
            vec![],
        )
        .unwrap();
    due_now(&h, &kept.id);
    h.core.deliver_answers().await;
    // Queued in the turn, as a message typed then is.
    assert!(h
        .items(&a)
        .iter()
        .any(|i| i["origin"] == "ask" && i["queued"] == true));
    h.wait("the answer read", |h| {
        read_by_agent(h, &r)
            .iter()
            .any(|m| m.contains("a répondu oui"))
    })
    .await;
    assert_eq!(h.core.ask(&kept.id).unwrap().state, AskState::Read);
}

#[tokio::test]
async fn without_wake_the_answer_waits_for_get_ask() {
    let h = harness("asks-no-wake");
    let (p, r) = h.project(false).await;
    on(&h);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let mut d = choice("Choisis");
    d.wake = false;
    let kept = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, d, vec![])
        .unwrap();
    h.core
        .ask_answer(
            &kept.id,
            AnswerIn {
                options: vec!["a".into()],
                ..Default::default()
            },
            None,
            vec![],
        )
        .unwrap();
    due_now(&h, &kept.id);
    h.core.deliver_answers().await;
    assert!(read_by_agent(&h, &r).is_empty());
    assert_eq!(h.core.ask(&kept.id).unwrap().state, AskState::Answered);
}

// ---------- a ticket that waits for a request ----------

#[tokio::test]
async fn a_ticket_waits_for_a_request_and_is_blocked_when_it_is_withdrawn() {
    let h = harness("asks-ticket-waits");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    for (id, key) in [("t-13", "DEM-13"), ("t-14", "DEM-14")] {
        h.core.tickets.write().push(Ticket {
            id: id.into(),
            project_id: p.id.clone(),
            key: key.into(),
            column: Column::Todo,
            ..Default::default()
        });
    }
    let mut d = choice("Choisis la direction");
    d.tickets = vec!["t-13".into()];
    let one = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, d.clone(), vec![])
        .unwrap();
    d.tickets = vec!["t-14".into()];
    let two = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, d, vec![])
        .unwrap();
    let waiting = h.core.tickets_waiting_asks();
    assert!(waiting.contains("t-13") && waiting.contains("t-14"));
    // The autopilot leaves them: what `start_ready` hands `to_start`.
    let held = |t: &Ticket| waiting.contains(&t.id) && !t.forced;
    let mut board = p.board.clone();
    board.autopilot = true;
    let started = crate::board::to_start(&h.core.tickets.read(), &p.id, &board, held);
    assert!(started.is_empty(), "{started:?}");
    // Answered, and its answer gone: DEM-13 may start.
    h.core
        .ask_answer(
            &one.id,
            AnswerIn {
                options: vec!["a".into()],
                ..Default::default()
            },
            None,
            vec![],
        )
        .unwrap();
    assert!(
        h.core.tickets_waiting_asks().contains("t-13"),
        "not before the answer goes"
    );
    due_now(&h, &one.id);
    h.core.deliver_answers().await;
    assert!(!h.core.tickets_waiting_asks().contains("t-13"));
    // Withdrawn: DEM-14 is blocked, with why.
    h.core
        .ask_withdraw(
            &Asker::Agent(a.clone()),
            &two.id,
            "choisie dans la conversation",
        )
        .unwrap();
    let t = h.core.ticket("t-14").unwrap();
    assert!(t.blocked.unwrap().contains("a-2"));
    assert!(!h.core.tickets_waiting_asks().contains("t-14"));
}

// ---------- the delegation ----------

#[tokio::test]
async fn a_request_is_left_to_an_agent_only_as_the_settings_say_never_a_go_ahead_a_task_or_its_own()
{
    let h = harness("asks-delegate");
    let (p, _) = h.project(false).await;
    on(&h);
    let author = put_agent(&h, &p, "design");
    let chef = put_agent(&h, &p, "chef");
    let other = put_agent(&h, &p, "autre");
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .chef = Some(chef.clone());
    let q = h
        .core
        .ask_deposit(
            agent_author(&author),
            &p.id,
            None,
            choice("Choisis"),
            vec![],
        )
        .unwrap();
    // « Personne », the default.
    let e = h
        .core
        .ask_delegate(&q.id, Some(&chef))
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("Les agents peuvent répondre aux demandes des autres agents"),
        "{e}"
    );
    h.core.settings.write().agents_answer = AgentsAnswer::Chef;
    assert!(h
        .core
        .ask_delegate(&q.id, Some(&other))
        .unwrap_err()
        .to_string()
        .contains("chef d’escouade"));
    assert!(h
        .core
        .ask_delegate(&q.id, Some(&author))
        .unwrap_err()
        .to_string()
        .contains("sa propre demande"));
    let left = h.core.ask_delegate(&q.id, Some(&chef)).unwrap();
    assert_eq!(left.delegated_to.as_deref(), Some(chef.as_str()));
    assert_eq!(
        h.core.inbox_get(&q.id).unwrap().delegates,
        vec![(chef.clone(), "chef".to_string())]
    );
    let gate = h
        .core
        .ask_deposit(
            agent_author(&author),
            &p.id,
            None,
            confirm("Publier ?", true),
            vec![],
        )
        .unwrap();
    assert!(h
        .core
        .ask_delegate(&gate.id, Some(&chef))
        .unwrap_err()
        .to_string()
        .contains("feu vert"));
    let task = AskDraft {
        kind: AskKind::Task,
        title: "Essaie sur un vrai téléphone".into(),
        ..Default::default()
    };
    let task = h
        .core
        .ask_deposit(agent_author(&author), &p.id, None, task, vec![])
        .unwrap();
    assert!(h.core.ask_delegate(&task.id, Some(&chef)).is_err());
}

// ---------- the server off, a restart ----------

#[tokio::test]
async fn the_box_is_closed_with_the_server_and_its_requests_are_kept() {
    let h = harness("asks-server-off");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    let kept = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Choisis"),
            vec![bytes("a.png", PNG, None)],
        )
        .unwrap();
    h.core.settings.write().mcp_enabled = false;
    let off = h.core.inbox_list(None);
    assert!(!off.enabled && off.asks.is_empty());
    assert!(h
        .core
        .inbox_get(&kept.id)
        .unwrap_err()
        .to_string()
        .contains("fermée avec le serveur MCP"));
    assert!(h
        .core
        .inbox_answer(&kept.id, AnswerIn::default(), vec![])
        .is_err());
    // Kept, frozen, with its file; the question before turning off counts it.
    h.core.asks_sweep();
    assert_eq!(h.core.ask(&kept.id).unwrap().state, AskState::Open);
    assert!(h.core.ask_dir(&kept.id).exists());
    assert_eq!(h.core.inbox_closing().open, 1);
    on(&h);
    let back = h.core.inbox_list(Some(&p.id));
    assert!(back.enabled);
    assert_eq!(back.asks.len(), 1);
    assert_eq!(back.open, 1);
    assert_eq!(back.asks[0].project_name, p.name);
}

#[tokio::test]
async fn the_requests_are_kept_across_a_restart() {
    let h = harness("asks-restart");
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    let kept = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Choisis"),
            vec![bytes("b.png", PNG, Some("b"))],
        )
        .unwrap();
    h.core.save_state().unwrap();
    let app = mock_app();
    let data = DataDir::new(h.dir.join("data"));
    let (again, _rx): (Arc<Core>, _) = Core::load(app.handle().clone(), data);
    let found = again.ask(&kept.id).unwrap();
    assert_eq!(found, kept);
    // The next one has a number of its own.
    let next = again
        .ask_deposit(external(), &p.id, None, choice("Encore"), vec![])
        .unwrap();
    assert_eq!(next.id, "a-2");
    assert!(again.ask_file(&found, None, Some(0)).is_ok());
}

// ---------- the screen's needs (ESC-510) ----------

#[test]
fn the_statistics_of_an_answer_say_its_sort_its_wait_and_the_recommendation_never_a_text() {
    let mut a = Ask {
        kind: AskKind::Choice,
        created_at: 1_000,
        recommend: Some(AskRecommend {
            option: Some("b".into()),
            text: None,
            why: "parce que".into(),
        }),
        ..Ask::default()
    };
    a.answer = Some(AskAnswer {
        options: vec!["b".into()],
        via: AskVia::Keyboard,
        at: 1_000 + 30 * 60_000,
        note: Some("garde la courbe".into()),
        ..AskAnswer::default()
    });
    assert_eq!(
        answered_event(&a),
        Some(crate::telemetry::Event::AskAnswered {
            kind: "choice",
            delay: "hour",
            via: "keyboard",
            followed: Some(true),
        })
    );
    a.answer.as_mut().unwrap().options = vec!["a".into()];
    a.answer.as_mut().unwrap().at = 1_000 + 3 * 24 * 3_600_000;
    assert_eq!(
        answered_event(&a),
        Some(crate::telemetry::Event::AskAnswered {
            kind: "choice",
            delay: "longer",
            via: "keyboard",
            followed: Some(false),
        })
    );
    // An answer of an agent the user left it to is not the user's.
    a.answer.as_mut().unwrap().by = Some("c1".into());
    assert_eq!(answered_event(&a), None);
    // A task has no recommendation to follow.
    let t = Ask {
        kind: AskKind::Task,
        answer: Some(AskAnswer {
            done: Some(true),
            via: AskVia::Voice,
            at: 5_000,
            ..AskAnswer::default()
        }),
        ..Ask::default()
    };
    assert_eq!(
        answered_event(&t),
        Some(crate::telemetry::Event::AskAnswered {
            kind: "task",
            delay: "ten_minutes",
            via: "voice",
            followed: None,
        })
    );
}

#[test]
fn the_files_of_closed_requests_are_kept_7_30_or_90_days_as_set() {
    assert_eq!(kept_days(7), 7);
    assert_eq!(kept_days(90), 90);
    assert_eq!(kept_days(30), 30);
    // Anything else (a hand-edited settings.json): 30.
    assert_eq!(kept_days(0), 30);
    assert_eq!(kept_days(365), 30);
}

#[tokio::test]
async fn a_shorter_keeping_erases_the_files_sooner_and_says_so() {
    let h = harness("asks-files-keep-7");
    h.core.settings.write().inbox_keep_days = 7;
    let (p, _) = h.project(false).await;
    let a = put_agent(&h, &p, "design");
    let kept = h
        .core
        .ask_deposit(
            agent_author(&a),
            &p.id,
            None,
            choice("Choisis"),
            vec![bytes("a.png", PNG, None)],
        )
        .unwrap();
    h.core
        .ask_withdraw(&Asker::Agent(a.clone()), &kept.id, "plus besoin")
        .unwrap();
    let day = 24 * 3600 * 1000;
    h.core.asks.lock().list[0].closed_at = Some(now_ms() - 8 * day);
    h.core.asks_sweep();
    let after = h.core.ask(&kept.id).unwrap();
    assert!(after.files_erased_at.is_some());
    assert!(h
        .core
        .ask_file(&after, None, Some(0))
        .unwrap_err()
        .to_string()
        .contains("7 jours"));
}

// ---------- a request that changes hands (ESC-585) ----------

/// The agent `dev` of a ticket under way (DEM-7) with three requests still due: one open, one put
/// off, one answered whose answer went and was not read; then its ticket validated (the ticket done,
/// the agent archived). The ids of the three.
async fn three_due_then_validated(h: &Harness, p: &Project) -> (String, String, String) {
    let dev = put_agent(h, p, "dev");
    h.core.tickets.write().push(Ticket {
        id: "t-7".into(),
        project_id: p.id.clone(),
        key: "DEM-7".into(),
        column: Column::Doing,
        ..Default::default()
    });
    let deposit = |title: &str| {
        h.core
            .ask_deposit(
                agent_author(&dev),
                &p.id,
                Some("t-7".into()),
                choice(title),
                vec![],
            )
            .unwrap()
            .id
    };
    let open = deposit("Les deux essais avant l'app mobile");
    let later = deposit("Le feu vert du trousseau");
    let answered = deposit("Le choix de l'étude");
    h.core
        .ask_snooze(&later, Some(now_ms() + 3_600_000))
        .unwrap();
    let ans = AnswerIn {
        options: vec!["b".into()],
        ..Default::default()
    };
    h.core.ask_answer(&answered, ans, None, vec![]).unwrap();
    {
        // Its answer went to `dev`, which did not read it.
        let mut box_ = h.core.asks.lock();
        let x = box_.list.iter_mut().find(|x| x.id == answered).unwrap();
        let ans = x.answer.as_mut().unwrap();
        ans.send_at = None;
        ans.sent_at = Some(now_ms());
    }
    h.core.tickets.write()[0].column = Column::Done;
    h.core.agents.read()[&dev].lock().meta.archived = true;
    (open, later, answered)
}

#[tokio::test]
async fn a_validated_tickets_requests_still_due_go_to_the_squad_lead_with_their_trace() {
    let h = harness("asks-handover-chef");
    let (p, r) = h.project(false).await;
    on(&h);
    let chef = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.turn(&chef, "Bonjour").await;
    let chef_name = h.agent(&chef).name.clone();
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .chef = Some(chef.clone());
    let (open, later, answered) = three_due_then_validated(&h, &p).await;
    let created = |id: &str| h.core.ask(id).unwrap().created_at;
    let before = [created(&open), created(&later), created(&answered)];
    h.core.asks_sweep();
    for (id, state) in [
        (&open, AskState::Open),
        (&later, AskState::Snoozed),
        (&answered, AskState::Answered),
    ] {
        let a = h.core.ask(id).unwrap();
        assert_eq!(a.state, state, "{id}");
        assert_eq!(a.author.agent.as_deref(), Some(chef.as_str()), "{id}");
        assert!(a.author.chef);
        assert_eq!(a.author.name, chef_name);
        let handed = a.handed.clone().unwrap();
        assert_eq!(handed.from, "agent-ag-dev");
        assert_eq!(handed.cause, HandoverCause::Ticket);
        assert_eq!(handed.ticket_key.as_deref(), Some("DEM-7"));
        // Its date stays; its trace says it.
        assert!(before.contains(&a.created_at));
        let last = a.history.last().unwrap();
        assert_eq!(last.what, "handed");
        assert_eq!(
            last.detail,
            format!("Reprise par {chef_name} : le ticket DEM-7 est validé")
        );
    }
    // The answer not read goes again, to the lead, as a message.
    assert!(h
        .core
        .ask(&answered)
        .unwrap()
        .answer
        .unwrap()
        .send_at
        .is_some());
    h.core.deliver_answers().await;
    h.wait("the answer read by the lead", |h| {
        read_by_agent(h, &r)
            .iter()
            .any(|m| m.contains(&format!("[Escouade · ta demande {answered}")))
    })
    .await;
    let msg = read_by_agent(&h, &r)
        .into_iter()
        .find(|m| m.contains("[Escouade · ta demande"))
        .unwrap();
    assert!(
        msg.contains("Elle était de agent-ag-dev et t’est revenue : le ticket DEM-7 est validé."),
        "{msg}"
    );
    assert_eq!(h.core.ask(&answered).unwrap().state, AskState::Read);
    // Another pass leaves them with the lead: never out because of a ticket that is done.
    h.core.asks_sweep();
    assert_eq!(h.core.ask(&open).unwrap().state, AskState::Open);
    assert_eq!(h.core.ask(&later).unwrap().state, AskState::Snoozed);
    // The lead owns them now: it may withdraw one.
    h.core
        .ask_withdraw(&Asker::Agent(chef.clone()), &open, "Plus utile")
        .unwrap();
    h.wait("its turn", |h| !h.agent(&chef).status.is_active())
        .await;
}

#[tokio::test]
async fn without_a_squad_lead_they_stay_with_no_living_author_until_the_project_has_one() {
    let h = harness("asks-handover-none");
    let (p, _) = h.project(false).await;
    on(&h);
    let (open, later, answered) = three_due_then_validated(&h, &p).await;
    h.core.asks_sweep();
    for (id, state) in [
        (&open, AskState::Open),
        (&later, AskState::Snoozed),
        (&answered, AskState::Answered),
    ] {
        let a = h.core.ask(id).unwrap();
        assert_eq!(a.state, state, "{id}");
        assert_eq!(a.author.agent, None);
        assert_eq!(a.author.name, "agent-ag-dev");
        assert_eq!(a.handed.as_ref().unwrap().cause, HandoverCause::Ticket);
        assert_eq!(
            a.history.last().unwrap().detail,
            "Sans auteur vivant : le ticket DEM-7 est validé, et le projet n’a pas de chef d’escouade"
        );
    }
    // No one outside Escouade takes it for its own.
    assert!(h.core.ask_withdraw(&Asker::External, &open, "x").is_err());
    // Passes go by: it stays (its date still runs out).
    h.core.asks_sweep();
    assert_eq!(h.core.ask(&open).unwrap().state, AskState::Open);
    // The project names a lead: they go to it, the answer not read with them.
    let chef = put_agent(&h, &p, "chef");
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .chef = Some(chef.clone());
    h.core.asks_sweep();
    for id in [&open, &later, &answered] {
        let a = h.core.ask(id).unwrap();
        assert_eq!(a.author.agent.as_deref(), Some(chef.as_str()), "{id}");
        let handed = a.handed.unwrap();
        assert_eq!(handed.from, "agent-ag-dev");
        assert_eq!(handed.cause, HandoverCause::Ticket);
        assert_eq!(
            a.history.last().unwrap().detail,
            "Reprise par chef : le ticket DEM-7 est validé"
        );
    }
    assert!(h
        .core
        .ask(&answered)
        .unwrap()
        .answer
        .unwrap()
        .send_at
        .is_some());
}

#[tokio::test]
async fn an_agent_archived_or_deleted_by_hand_hands_its_requests_over_too() {
    let h = harness("asks-handover-by-hand");
    let (p, _) = h.project(false).await;
    on(&h);
    let chef = put_agent(&h, &p, "chef");
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .chef = Some(chef.clone());
    let a = put_agent(&h, &p, "design");
    let b = put_agent(&h, &p, "back");
    let of_a = h
        .core
        .ask_deposit(agent_author(&a), &p.id, None, choice("A"), vec![])
        .unwrap()
        .id;
    let of_b = h
        .core
        .ask_deposit(agent_author(&b), &p.id, None, choice("B"), vec![])
        .unwrap()
        .id;
    h.core.agents.read()[&a].lock().meta.archived = true;
    h.core.agents.write().remove(&b);
    h.core.asks_sweep();
    let x = h.core.ask(&of_a).unwrap();
    assert_eq!(x.author.agent.as_deref(), Some(chef.as_str()));
    assert_eq!(
        x.history.last().unwrap().detail,
        "Reprise par chef : agent-ag-design a été archivé"
    );
    let y = h.core.ask(&of_b).unwrap();
    assert_eq!(y.state, AskState::Open);
    assert_eq!(
        y.history.last().unwrap().detail,
        "Reprise par chef : agent-ag-back a été supprimé"
    );
    // The lead archived in its turn: no living author, never out.
    h.core.agents.read()[&chef].lock().meta.archived = true;
    h.core.asks_sweep();
    let x = h.core.ask(&of_a).unwrap();
    assert_eq!(x.state, AskState::Open);
    assert_eq!(x.author.agent, None);
    assert_eq!(x.handed.unwrap().from, "chef");
}
