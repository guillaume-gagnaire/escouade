//! The Kanban of a team's project in the core (TS-6): the fields it shares, who holds a ticket, what the autopilot
//! takes (C11-A), and two data folders against the real `escouade-team-server`: a ticket made on one is on the other
//! with the key of the server's counter, its claim passes once and says the holder's name, « Ne pas partager » keeps
//! a ticket home, a title changed on both sides shows its conflict to the loser, offline nothing is taken.

use super::*;
use crate::board::to_start;
use crate::core_tests::harness;
use crate::integrations::secrets::MemoryStore;
use crate::model::{BoardSettings, Criterion};
use crate::tickets::TicketDraft;

fn team_ticket(id: &str, column: Column, team: Option<TeamTicket>) -> Ticket {
    Ticket {
        id: id.into(),
        project_id: "p".into(),
        key: id.into(),
        column,
        rank: 1,
        team,
        ..Default::default()
    }
}

#[test]
fn the_shared_fields_go_and_come_back_whole() {
    let mut t = Ticket {
        key: "DEM-3".into(),
        title: "Le bail".into(),
        description: "Dix minutes".into(),
        criteria: vec![Criterion {
            text: "Renouvelé".into(),
            ok: true,
            note: "vu".into(),
        }],
        column: Column::Review,
        rank: 7,
        max_loops: 8,
        progress: vec!["la prise".into()],
        partial: true,
        iteration: 2,
        team: Some(TeamTicket {
            assignee: Some("aa".repeat(16)),
            ..Default::default()
        }),
        ..Default::default()
    };
    let fields = fields_of(&t);
    assert_eq!(fields.len(), SHARED_FIELDS.len());
    assert!(SHARED_FIELDS.iter().all(|f| fields.contains_key(*f)));
    assert_eq!(fields["column"], json!("review"));
    let mut back = Ticket {
        team: Some(TeamTicket::default()),
        ..Default::default()
    };
    for (k, v) in &fields {
        apply_field(&mut back, k, v);
    }
    t.team.as_mut().unwrap().assignee = back.team.as_ref().unwrap().assignee.clone();
    assert_eq!(fields_of(&back), fields);
    // A value it cannot read leaves the field as it is; an empty key is never taken.
    apply_field(&mut back, "column", &json!("nowhere"));
    apply_field(&mut back, "key", &json!(""));
    assert_eq!((back.column, back.key.as_str()), (Column::Review, "DEM-3"));
}

#[test]
fn the_holder_says_interrupted_and_unreachable() {
    let me = DeviceId([1; 16]);
    let claim = |holder: [u8; 16], until: i64, renewed: i64| Claim {
        ticket: vec![9; 16],
        holder: Some(holder.to_vec()),
        until,
        token: 1,
        version: 1,
        claimed_at: 100,
        renewed_at: renewed,
        last_holder: None,
    };
    let h = holder_of(&claim([2; 16], 800, 200), 300, &me, Some("Alice".into())).unwrap();
    assert_eq!(
        (h.name.as_str(), h.mine, h.interrupted, h.unreachable_min),
        ("Alice", false, false, None)
    );
    assert_eq!((h.since, h.until), (100_000, 800_000));
    // Its machine silent for 12 minutes: its lease ran out, it no longer answers.
    let h = holder_of(&claim([2; 16], 800, 200), 200 + 12 * 60, &me, None).unwrap();
    assert!(h.interrupted);
    assert_eq!(h.unreachable_min, Some(12));
    assert!(
        holder_of(&claim([1; 16], 800, 200), 300, &me, None)
            .unwrap()
            .mine
    );
    let free = Claim {
        holder: None,
        ..claim([2; 16], 0, 0)
    };
    assert!(holder_of(&free, 300, &me, None).is_none());
}

#[test]
fn the_autopilot_takes_only_what_it_is_given() {
    let holder = |mine: bool, interrupted: bool| TeamHolder {
        name: "Alice".into(),
        mine,
        interrupted,
        ..Default::default()
    };
    let tickets = vec![
        // Of this machine: as before.
        team_ticket("local", Column::Todo, None),
        // The team's, given to nobody here.
        team_ticket("team", Column::Todo, Some(TeamTicket::default())),
        // Assigned to this machine's member.
        team_ticket(
            "assigned",
            Column::Todo,
            Some(TeamTicket {
                assigned_me: true,
                ..Default::default()
            }),
        ),
        // Assigned, but a teammate holds it; or held, its lease run out (« interrompu »).
        team_ticket(
            "held",
            Column::Todo,
            Some(TeamTicket {
                assigned_me: true,
                holder: Some(holder(false, false)),
                ..Default::default()
            }),
        ),
        team_ticket(
            "interrupted",
            Column::Todo,
            Some(TeamTicket {
                assigned_me: true,
                holder: Some(holder(false, true)),
                ..Default::default()
            }),
        ),
    ];
    let s = BoardSettings {
        max_parallel: 6,
        ..Default::default()
    };
    let mut ids = to_start(&tickets, "p", &s, |_| false);
    ids.sort();
    assert_eq!(ids, ["assigned", "local"]);
    // « À prendre » taken by this board: the team's « À faire » too, never one held.
    let s = BoardSettings {
        team_take_todo: true,
        ..s
    };
    let mut ids = to_start(&tickets, "p", &s, |_| false);
    ids.sort();
    assert_eq!(ids, ["assigned", "local", "team"]);
    // Launched by hand: the claim decides at the start.
    let mut forced = tickets.clone();
    forced[3].forced = true;
    assert!(to_start(&forced, "p", &s, |_| false).contains(&"held".to_string()));
    // A ticket a teammate holds is refused at once by the tools, with the name.
    assert!(start_refusal(&tickets[3]).unwrap().contains("Alice"));
    assert!(start_refusal(&tickets[4]).is_none());
    assert!(start_refusal(&tickets[2]).is_none());
}

#[test]
fn keys_of_the_team_and_of_this_machine() {
    assert_eq!(team_key("DEM", 12), "DEM-12");
    assert_eq!(provisional_key("DEM", 3), "DEM-L3");
}

async fn server() -> (
    Arc<escouade_team_server::Server>,
    String,
    tokio::task::JoinHandle<()>,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let config = escouade_team_server::config::Config::in_folder(dir.path().join("server"));
    let server = Arc::new(escouade_team_server::Server::open(config).await.unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let s = server.clone();
    let task = tokio::spawn(async move {
        let _ = s.serve(listener, None, std::future::pending()).await;
    });
    (server, format!("127.0.0.1:{}", addr.port()), task, dir)
}

/// A TCP relay before the server: cut, the server cannot be reached (its connections too).
struct Relay {
    addr: std::net::SocketAddr,
    on: Arc<std::sync::atomic::AtomicBool>,
    conns: Arc<parking_lot::Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl Relay {
    async fn to(target: &str) -> Relay {
        let target: std::net::SocketAddr = target.parse().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let on = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let conns: Arc<parking_lot::Mutex<Vec<tokio::task::JoinHandle<()>>>> = Arc::default();
        let (o, c) = (on.clone(), conns.clone());
        tokio::spawn(async move {
            while let Ok((mut inbound, _)) = listener.accept().await {
                if !o.load(std::sync::atomic::Ordering::Acquire) {
                    continue;
                }
                c.lock().push(tokio::spawn(async move {
                    if let Ok(mut out) = tokio::net::TcpStream::connect(target).await {
                        let _ = tokio::io::copy_bidirectional(&mut inbound, &mut out).await;
                    }
                }));
            }
        });
        Relay { addr, on, conns }
    }

    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.addr.port())
    }

    fn cut(&self) {
        self.on.store(false, std::sync::atomic::Ordering::Release);
        for c in self.conns.lock().drain(..) {
            c.abort();
        }
    }
}

fn teams(root: &std::path::Path, store: Arc<MemoryStore>) -> Arc<Teams> {
    Arc::new(Teams::open(
        root.to_path_buf(),
        store,
        Arc::new(crate::model::Settings::default),
        Arc::new(|| {}),
    ))
}

fn draft(title: &str, local: bool) -> TicketDraft {
    TicketDraft {
        title: title.into(),
        max_loops: 3,
        team_local: Some(local),
        ..Default::default()
    }
}

fn find(h: &crate::core_tests::Harness, project: &str, title: &str) -> Option<Ticket> {
    h.core
        .tickets
        .read()
        .iter()
        .find(|t| t.project_id == project && t.title == title)
        .cloned()
}

/// A reading of the Kanban, the claims read too (a test has no live feed to tell of a claim).
async fn read(h: &crate::core_tests::Harness, project: &str) {
    h.core.team_claims_at.lock().clear();
    h.core.sync_team_board(project).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_folders_share_a_team_board_and_take_a_ticket_once() {
    let (server, host, task, dir) = server().await;
    let relay = Relay::to(&host).await;
    let host = relay.host();
    let store = Arc::new(MemoryStore::default());
    let alice_teams = teams(&dir.path().join("a"), store.clone());
    let bob_teams = teams(&dir.path().join("b"), store.clone());
    let token = server.new_team_token().await.unwrap();
    let id = alice_teams
        .create(&host, &token, "ACME", None)
        .await
        .unwrap()
        .team
        .id;
    let invite = alice_teams.invite(&id, "member", None).await.unwrap();
    bob_teams.join(&invite.url, "").await.unwrap();
    alice_teams.sync(&id).await.unwrap();
    assert!(bob_teams.sync(&id).await.unwrap().joined);

    let ha = harness("teamboard-alice");
    let hb = harness("teamboard-bob");
    ha.core.settings.write().user_name = "Alice Martin".into();
    hb.core.settings.write().user_name = "Bob Durand".into();
    let (pa, _) = ha.project(false).await;
    let (pb, _) = hb.project(false).await;
    ha.core.set_teams(alice_teams.clone());
    hb.core.set_teams(bob_teams.clone());
    ha.core.set_project_team(&pa.id, Some(id.clone())).unwrap();
    hb.core.set_project_team(&pb.id, Some(id.clone())).unwrap();
    // A ticket of Alice's before the Kanban is the team's: it stays home (TS-7 brings them in).
    let before = ha
        .core
        .ticket_create(&pa.id, draft("Avant l’équipe", false))
        .await
        .unwrap();
    let linked = ha.core.link_team_board(&pa.id, None).await.unwrap();
    let journal = linked.team_project.clone().expect("a journal of the team");
    assert!(ha.core.ticket(&before.id).unwrap().team_local);
    let linked_b = hb
        .core
        .link_team_board(&pb.id, Some(journal.clone()))
        .await
        .unwrap();
    assert_eq!(
        linked_b.board.prefix, linked.board.prefix,
        "the team's prefix"
    );

    // Alice makes two tickets: one for the team, one « Ne pas partager ».
    let shared = ha
        .core
        .ticket_create(&pa.id, draft("Écrire la prise", false))
        .await
        .unwrap();
    assert!(
        shared.key.ends_with("-L2") || shared.key.contains("-L"),
        "of this machine until the server numbers it: {}",
        shared.key
    );
    let home = ha
        .core
        .ticket_create(&pa.id, draft("Brouillon", true))
        .await
        .unwrap();
    read(&ha, &pa.id).await;
    let shared = ha.core.ticket(&shared.id).unwrap();
    let prefix = &linked.board.prefix;
    assert_eq!(shared.key, format!("{prefix}-1"), "the server's counter");
    assert!(shared.team.as_ref().unwrap().keyed);
    assert!(ha.core.ticket(&home.id).unwrap().team.is_none());

    bob_teams.sync(&id).await.unwrap();
    read(&hb, &pb.id).await;
    let seen = find(&hb, &pb.id, "Écrire la prise").expect("on Bob's board");
    assert_eq!(seen.key, shared.key);
    assert_eq!(seen.column, Column::Todo);
    assert!(
        find(&hb, &pb.id, "Brouillon").is_none(),
        "nothing of it leaves"
    );
    assert!(find(&hb, &pb.id, "Avant l’équipe").is_none());

    // Alice takes it; Bob sees whose it is, and is refused with her name.
    ha.core
        .team_take(&linked, &shared.id)
        .await
        .expect("Alice takes it");
    let held = ha.core.ticket(&shared.id).unwrap();
    assert!(held.team.as_ref().unwrap().token.is_some());
    read(&hb, &pb.id).await;
    let theirs = hb.core.ticket(&seen.id).unwrap();
    let holder = theirs.team.as_ref().unwrap().holder.clone().expect("held");
    assert_eq!(
        (holder.name.as_str(), holder.mine, holder.interrupted),
        ("Alice Martin", false, false)
    );
    let refused = hb.core.team_take(&linked_b, &seen.id).await.unwrap_err();
    assert!(refused.contains("Alice Martin"), "{refused}");
    assert!(start_refusal(&theirs).unwrap().contains("Alice Martin"));
    // Bob is no administrator: his move of the held ticket's column is undone, and said.
    hb.core
        .edit_ticket(&seen.id, |t| {
            t.column = Column::Done;
            Ok(())
        })
        .unwrap();
    read(&hb, &pb.id).await;
    let undone = hb.core.ticket(&seen.id).unwrap();
    assert_eq!(undone.column, Column::Todo);
    assert!(undone
        .team
        .as_ref()
        .unwrap()
        .refused
        .as_deref()
        .unwrap_or_default()
        .contains("Alice Martin"));

    // Alice moves it « En cours » under her token, then « À tester »: the claim is given back.
    ha.core
        .edit_ticket(&shared.id, |t| {
            t.column = Column::Doing;
            Ok(())
        })
        .unwrap();
    read(&ha, &pa.id).await;
    read(&hb, &pb.id).await;
    assert_eq!(hb.core.ticket(&seen.id).unwrap().column, Column::Doing);
    ha.core
        .edit_ticket(&shared.id, |t| {
            t.column = Column::Review;
            Ok(())
        })
        .unwrap();
    read(&ha, &pa.id).await;
    let given_back = ha.core.ticket(&shared.id).unwrap();
    assert!(given_back.team.as_ref().unwrap().token.is_none());
    read(&hb, &pb.id).await;
    let free = hb.core.ticket(&seen.id).unwrap();
    assert_eq!(free.column, Column::Review);
    assert!(free.team.as_ref().unwrap().holder.is_none());

    // A title changed on both sides at once: the last writer wins, the loser sees it with the winner's name.
    hb.core
        .edit_ticket(&seen.id, |t| {
            t.title = "Titre de Bob".into();
            Ok(())
        })
        .unwrap();
    ha.core
        .edit_ticket(&shared.id, |t| {
            t.title = "Titre d’Alice".into();
            Ok(())
        })
        .unwrap();
    read(&hb, &pb.id).await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    read(&ha, &pa.id).await;
    read(&hb, &pb.id).await;
    let bob_side = hb.core.ticket(&seen.id).unwrap();
    assert_eq!(bob_side.title, "Titre d’Alice");
    let c = &bob_side.team.as_ref().unwrap().conflicts;
    assert_eq!(c.len(), 1, "{c:?}");
    assert_eq!(
        (c[0].field.as_str(), c[0].by.as_str()),
        ("title", "Alice Martin")
    );
    assert_eq!(c[0].mine, json!("Titre de Bob"));
    assert!(
        ha.core
            .ticket(&shared.id)
            .unwrap()
            .team
            .as_ref()
            .unwrap()
            .conflicts
            .is_empty(),
        "the winner sees none"
    );
    // « Reprendre le mien ».
    hb.core.team_settle(&seen.id, "title", true).unwrap();
    read(&hb, &pb.id).await;
    read(&ha, &pa.id).await;
    assert_eq!(ha.core.ticket(&shared.id).unwrap().title, "Titre de Bob");
    assert!(hb
        .core
        .ticket(&seen.id)
        .unwrap()
        .team
        .as_ref()
        .unwrap()
        .conflicts
        .is_empty());

    // Deleted on Bob's: gone from Alice's.
    hb.core.ticket_delete(&seen.id).await.unwrap();
    read(&hb, &pb.id).await;
    read(&ha, &pa.id).await;
    assert!(ha.core.ticket(&shared.id).is_err());

    // Offline, a ticket of the team is not taken, and it is said.
    let other = ha
        .core
        .ticket_create(&pa.id, draft("Hors ligne", false))
        .await
        .unwrap();
    read(&ha, &pa.id).await;
    relay.cut();
    let offline = ha.core.team_take(&linked, &other.id).await.unwrap_err();
    assert!(offline.contains("Hors ligne"), "{offline}");
    // It is written all the same, and waits.
    ha.core
        .edit_ticket(&other.id, |t| {
            t.title = "Écrit hors ligne".into();
            Ok(())
        })
        .unwrap();
    read(&ha, &pa.id).await;
    assert_eq!(alice_teams.status(&id).await.unwrap().online, Some(false));
    assert!(alice_teams.status(&id).await.unwrap().pending > 0);
    task.abort();
}
