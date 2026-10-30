//! Tests of the projects closed with their history kept (`closed`): real git repositories and the
//! fake `claude` CLI. Closing a project keeps its conversations, its tickets and its settings;
//! adding its folder again gives them back with its id.

use crate::agent::TurnRow;
use crate::closed::{ClosedProject, ClosedProjectInfo};
use crate::conv;
use crate::core::{AgentOptions, Core};
use crate::core_tests::{extra_account, harness, repo, wt_step, Harness};
use crate::model::*;
use crate::tickets::TicketDraft;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri::test::{mock_app, MockRuntime};

const COLOR: &str = "oklch(0.6 0.1 20)";

fn draft(title: &str) -> TicketDraft {
    TicketDraft {
        title: title.into(),
        description: String::new(),
        criteria: Vec::new(),
        max_loops: 5,
        after: Vec::new(),
        branch: String::new(),
        images: None,
        files: None,
        engine: None,
        ..Default::default()
    }
}

fn as_json<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}

/// The project closed, its history kept.
async fn close(h: &Harness, id: &str) {
    h.core.remove_project(id, false).await.unwrap();
}

/// The folder added again as the window does, the history coming back with it if there is one.
async fn add_again(h: &Harness, folder: &Path) -> Project {
    h.core
        .create_project(&folder.to_string_lossy(), "", COLOR, false, None, true)
        .await
        .unwrap()
}

/// The board of the project, its autopilot off: nothing starts by itself.
fn quiet_board(h: &Harness, project_id: &str) {
    let mut board = h.core.project(project_id).unwrap().board;
    board.autopilot = false;
    h.core.board_set(project_id, board).unwrap();
}

/// A plain agent of the project that took a message and ended its turn: its conversation has
/// content. Its name, given after its first message (ESC-156), is waited for: what the project
/// keeps when it closes is then the same on every run.
async fn agent_that_talked(h: &Harness, project_id: &str) -> String {
    let id = h.core.create_agent(project_id, None).await.unwrap().meta.id;
    h.turn(&id, "Bonjour").await;
    h.wait("its name", |h| h.agent(&id).named).await;
    id
}

fn conversation_log(h: &Harness, agent_id: &str) -> PathBuf {
    conv::log_path(&h.core.data.conversations(), agent_id)
}

/// What the setup of the agent's worktree wrote: a log the history keeps.
fn setup_log_of(h: &Harness, agent_id: &str) -> PathBuf {
    let log = h.core.data.setup_log(agent_id);
    std::fs::create_dir_all(log.parent().unwrap()).unwrap();
    std::fs::write(&log, "npm install\nadded 42 packages\n").unwrap();
    log
}

/// What the window that adds a project is told of the history of the folder `path`.
async fn history_of(core: &Arc<Core<MockRuntime>>, path: &str) -> Option<ClosedProjectInfo> {
    core.closed_project_info_for(path, None).await
}

/// `history_of`, the history `id` preferred if it is of that folder.
async fn history_with(
    core: &Arc<Core<MockRuntime>>,
    path: &str,
    id: &str,
) -> Option<ClosedProjectInfo> {
    core.closed_project_info_for(path, Some(id)).await
}

fn closed_files(h: &Harness) -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(h.core.data.closed_projects()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = dir
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

fn toasts(h: &Harness) -> Vec<String> {
    h.events
        .lock()
        .iter()
        .filter(|e| e["type"] == "toast")
        .map(|e| e["text"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_project_closed_comes_back_with_everything_it_had_when_its_folder_is_added_again() {
    let h = harness("closed-roundtrip");
    let (p, r) = h.project(false).await;
    // Its settings.
    h.core
        .update_project(Project {
            run_commands: vec![RunCommand {
                id: "run1".into(),
                name: "dev".into(),
                command: "npm run dev".into(),
                ..Default::default()
            }],
            worktree_setup: vec![wt_step("npm install", "")],
            worktree_teardown: vec![wt_step("npm run clean", "web")],
            worktree_dir: ".wt".into(),
            commit_mode: CommitMode::Direct,
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let mut board = h.core.project(&p.id).unwrap().board;
    board.autopilot = false;
    board.action = "keep".into();
    board.max_parallel = 3;
    h.core.board_set(&p.id, board).unwrap();
    // Its agents: one that talked, one archived. Its tickets. Its statistics.
    let a1 = agent_that_talked(&h, &p.id).await;
    let a2 = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.archive_agent(&a2, true).await.unwrap();
    h.core
        .ui
        .write()
        .selected_agent
        .insert(p.id.clone(), a1.clone());
    let t1 = h.core.ticket_create(&p.id, draft("Premier")).await.unwrap();
    let t2 = h.core.ticket_create(&p.id, draft("Second")).await.unwrap();
    h.core.stats.record_turns(
        "principal",
        &a1,
        &p.id,
        &[TurnRow {
            model: "claude-sonnet-5".into(),
            input: 10,
            cache: 0,
            output: 5,
            cost: 1.5,
            ..Default::default()
        }],
    );
    let project = as_json(&h.core.project(&p.id).unwrap());
    let conversation = h.items(&a1);
    assert!(conversation.iter().any(|i| i["kind"] == "user"));
    let tickets = [
        h.core.ticket(&t1.id).unwrap(),
        h.core.ticket(&t2.id).unwrap(),
    ];
    let (first, second) = (h.core.agent(&a1).unwrap(), h.core.agent(&a2).unwrap());

    close(&h, &p.id).await;

    assert!(h.core.project(&p.id).is_err());
    assert!(h.core.agent(&a1).is_err() && h.core.agent(&a2).is_err());
    assert!(h.core.ticket(&t1.id).is_err() && h.core.ticket(&t2.id).is_err());
    // Listed, with what the window shows of it.
    let listed = h.core.closed_projects();
    assert_eq!(listed.len(), 1);
    let info = &listed[0];
    assert_eq!(
        (info.id.as_str(), info.name.as_str(), info.path.as_str()),
        (p.id.as_str(), "demo", p.path.as_str())
    );
    assert_eq!((info.agents, info.tickets), (1, 2));
    assert!(info.closed_at > 0 && info.last_activity > 0 && info.size > 0);
    assert!(info.last_activity <= info.closed_at, "{info:?}");
    // Its statistics are still its own, though its agents are no one's of the app for now.
    let stats = h.core.stats_view("day", None);
    assert_eq!(stats.by_project[0].key, p.id);
    assert_eq!(stats.by_agent[0].agent_id, a1);
    assert_eq!(stats.by_agent[0].name, None);

    let seen = h.events.lock().len();
    let back = h
        .core
        .create_project(
            &r.to_string_lossy(),
            "",
            "oklch(0.7 0.2 90)",
            true,
            None,
            true,
        )
        .await
        .unwrap();

    // The same project, with the name of its history and the colour and worktrees it was given.
    assert_eq!(back.id, p.id);
    assert_eq!(back.name, "demo");
    assert_eq!(back.color, "oklch(0.7 0.2 90)");
    assert!(back.worktree_per_agent);
    let mut want = project;
    want["color"] = json!("oklch(0.7 0.2 90)");
    want["worktreePerAgent"] = json!(true);
    assert_eq!(as_json(&back), want);
    assert_eq!(as_json(&h.core.project(&p.id).unwrap()), want);
    // Its agents, as they were when it closed, without a process; their conversation.
    for (id, handle) in [(&a1, &first), (&a2, &second)] {
        let kept = as_json(&handle.lock().meta);
        assert_eq!(as_json(&h.agent(id)), kept, "{id}");
        assert!(!h.alive(id));
    }
    assert!(h.agent(&a2).archived);
    assert_eq!(h.items(&a1), conversation);
    // Its tickets, the key numbering with them.
    assert_eq!(h.core.ticket(&t1.id).unwrap(), tickets[0]);
    assert_eq!(h.core.ticket(&t2.id).unwrap(), tickets[1]);
    // The agent selected, the project shown.
    let ui = h.core.ui.read().clone();
    assert_eq!(ui.selected_agent.get(&p.id), Some(&a1));
    assert_eq!(ui.active_project.as_deref(), Some(p.id.as_str()));
    assert_eq!(ui.view, "project");
    // Its statistics have their agents' names again.
    let stats = h.core.stats_view("day", None);
    assert_eq!(stats.by_project[0].key, p.id);
    assert_eq!(
        stats.by_agent[0].name.as_deref(),
        Some(h.agent(&a1).name.as_str())
    );
    // The window was told of each agent and ticket before the answer.
    let told: Vec<Value> = h.events.lock()[seen..].to_vec();
    let of = |kind: &str, id: &str| {
        told.iter()
            .any(|e| e["type"] == kind && e[kind]["id"] == id)
    };
    assert!(of("agent", &a1) && of("agent", &a2));
    assert!(of("ticket", &t1.id) && of("ticket", &t2.id));
    // The state is saved, and the history is no longer a closed project's.
    let state: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.state_file()).unwrap()).unwrap();
    assert!(state["projects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == p.id.as_str()));
    assert!(state["agents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == a1.as_str()));
    assert!(h.core.closed_projects().is_empty());
    assert!(closed_files(&h).is_empty());
}

#[tokio::test]
async fn a_name_that_comes_once_its_project_is_closed_renames_nothing_not_even_the_agent_back() {
    let h = harness("closed-late-name");
    let (p, r) = h.project(true).await;
    quiet_board(&h, &p.id);
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // The fake Haiku names it 3 s after its first message: the project is closed before, and its
    // folder added again, the agent back with the same id.
    h.turn(&a, "Bonjour [lent]").await;
    let before = h.agent(&a);
    assert!(!before.named);
    let branch = before.worktree.as_ref().unwrap().branch.clone();
    close(&h, &p.id).await;
    add_again(&h, &r).await;

    h.wait("the name dropped", |h| {
        h.core.names_dropped.load(Ordering::Acquire) == 1
    })
    .await;
    // The agent back is as its history kept it: its name, its branch, in git as well.
    let back = h.agent(&a);
    assert_eq!(
        (back.name.as_str(), back.named),
        (before.name.as_str(), false)
    );
    assert_eq!(back.worktree.unwrap().branch, branch);
    let branches = std::process::Command::new("git")
        .args(["branch", "--list", "--format=%(refname:short)"])
        .current_dir(&r)
        .output()
        .unwrap();
    let branches = String::from_utf8_lossy(&branches.stdout);
    assert!(branches.lines().any(|b| b == branch), "{branches}");
    assert!(!branches.contains("bonjour"), "{branches}");
    h.core.stop_for_good();
}

#[tokio::test]
async fn closing_keeps_the_conversations_and_the_setup_logs_and_stops_what_ran() {
    let h = harness("closed-keeps-files");
    let (p, _) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    let handle = h.core.agent(&a).unwrap();
    assert!(h.alive(&a));
    let log = conversation_log(&h, &a);
    let conversation = std::fs::read(&log).unwrap();
    let setup = setup_log_of(&h, &a);
    let setup_text = std::fs::read(&setup).unwrap();

    close(&h, &p.id).await;

    // Its process is stopped, as before; nothing of its files goes.
    assert!(handle.lock().proc.is_none());
    assert_eq!(std::fs::read(&log).unwrap(), conversation);
    assert_eq!(std::fs::read(&setup).unwrap(), setup_text);
    assert_eq!(closed_files(&h), [format!("{}.json", p.id)]);
    // Its size counts them.
    let size = h.core.closed_projects()[0].size;
    assert!(
        size > conversation.len() as u64 + setup_text.len() as u64,
        "{size}"
    );
}

#[tokio::test]
async fn an_agent_in_the_middle_of_a_turn_comes_back_done() {
    let h = harness("closed-mid-turn");
    let (p, r) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .send_message(&a, "slow".into(), vec![])
        .await
        .unwrap();
    h.wait("the turn running", |h| {
        h.agent(&a).status == AgentStatus::Running
    })
    .await;
    close(&h, &p.id).await;
    // Kept done, as the app does for a turn a restart cuts.
    let kept: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.closed_project_file(&p.id)).unwrap())
            .unwrap();
    assert_eq!(kept["agents"][0]["status"], as_json(&AgentStatus::Done));
    add_again(&h, &r).await;
    assert_eq!(h.agent(&a).status, AgentStatus::Done);
    assert!(!h.alive(&a));
}

#[tokio::test]
async fn a_blank_project_is_a_new_one_and_the_history_stays_among_the_closed_projects() {
    let h = harness("closed-blank");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a = agent_that_talked(&h, &p.id).await;
    let t = h.core.ticket_create(&p.id, draft("Un")).await.unwrap();
    close(&h, &p.id).await;

    let q = h
        .core
        .create_project(&r.to_string_lossy(), "neuf", COLOR, false, None, false)
        .await
        .unwrap();
    assert_ne!(q.id, p.id);
    assert_eq!(q.name, "neuf");
    assert!(h.core.project_agents(&q.id).is_empty());
    assert!(h.core.tickets.read().iter().all(|x| x.project_id != p.id));
    assert!(h.core.ticket(&t.id).is_err());
    assert!(h.core.agent(&a).is_err());
    // Nothing was erased: the old history is listed, its files there.
    let listed = h.core.closed_projects();
    assert_eq!(
        listed.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        [p.id.as_str()]
    );
    assert!(conversation_log(&h, &a).exists());

    // Closed too, the blank project is the folder's latest history; the old one stays listed.
    tokio::time::sleep(Duration::from_millis(5)).await;
    close(&h, &q.id).await;
    let ids: Vec<String> = h.core.closed_projects().into_iter().map(|c| c.id).collect();
    assert_eq!(ids, [q.id.clone(), p.id.clone()]);
    assert_eq!(history_of(&h.core, &p.path).await.unwrap().id, q.id);
    let again = add_again(&h, &r).await;
    assert_eq!(again.id, q.id);
    assert_eq!(
        h.core
            .closed_projects()
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        [p.id.as_str()]
    );
}

#[tokio::test]
async fn erasing_a_history_removes_its_conversations_its_setup_logs_and_its_file() {
    let h = harness("closed-erase");
    let (p, _) = h.project(false).await;
    let a1 = agent_that_talked(&h, &p.id).await;
    let a2 = agent_that_talked(&h, &p.id).await;
    h.core.archive_agent(&a2, true).await.unwrap();
    let other_dir = repo(&h.dir.join("b"));
    let q = h
        .core
        .create_project(
            &other_dir.to_string_lossy(),
            "autre",
            COLOR,
            false,
            None,
            true,
        )
        .await
        .unwrap();
    let b = agent_that_talked(&h, &q.id).await;
    let logs = [
        setup_log_of(&h, &a1),
        setup_log_of(&h, &a2),
        setup_log_of(&h, &b),
    ];
    close(&h, &p.id).await;
    for a in [&a1, &a2] {
        assert!(conversation_log(&h, a).exists());
    }

    h.core.erase_closed_project(&p.id).unwrap();

    for a in [&a1, &a2] {
        assert!(!conversation_log(&h, a).exists(), "{a}");
    }
    assert!(!logs[0].exists() && !logs[1].exists());
    assert!(closed_files(&h).is_empty());
    assert!(h.core.closed_projects().is_empty());
    // The open project's files are not touched.
    assert!(logs[2].exists() && conversation_log(&h, &b).exists());
    assert!(!h.items(&b).is_empty());
    // Gone: asked again, it says so.
    let e = h.core.erase_closed_project(&p.id).unwrap_err();
    assert_eq!(e.to_string(), "Ce projet fermé n'existe plus.");
    assert!(h.core.erase_closed_project("inconnu").is_err());
}

#[tokio::test]
async fn closing_with_the_history_erased_writes_nothing_and_removes_the_files() {
    let h = harness("closed-close-erase");
    let (p, _) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    let setup = setup_log_of(&h, &a);
    // A history of that id a stop left behind: it goes too.
    std::fs::create_dir_all(h.core.data.closed_projects()).unwrap();
    let stale = ClosedProject {
        project: p.clone(),
        agents: vec![h.agent(&a)],
        tickets: Vec::new(),
        selected_agent: None,
        closed_at: now_ms(),
    };
    std::fs::write(
        h.core.data.closed_project_file(&p.id),
        serde_json::to_vec(&stale).unwrap(),
    )
    .unwrap();

    h.core.remove_project(&p.id, true).await.unwrap();

    assert!(h.core.project(&p.id).is_err());
    assert!(!conversation_log(&h, &a).exists());
    assert!(!setup.exists());
    assert!(closed_files(&h).is_empty());
    assert!(h.core.closed_projects().is_empty());
    assert!(history_of(&h.core, &p.path).await.is_none());
}

#[tokio::test]
async fn the_folder_is_found_again_whatever_the_spelling_of_its_path() {
    let h = harness("closed-paths");
    let (p, r) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;

    let typed = r.to_string_lossy().to_string();
    let sep = std::path::MAIN_SEPARATOR;
    let mut spellings = vec![format!("{typed}{sep}"), typed.replace('\\', "/")];
    if cfg!(windows) {
        spellings.push(typed.to_uppercase());
        spellings.push(typed.to_lowercase());
        spellings.push(format!("{}/", typed.replace('\\', "/")));
    }
    for (i, spelling) in spellings.iter().enumerate() {
        let info = history_of(&h.core, spelling)
            .await
            .unwrap_or_else(|| panic!("no history found for {spelling}"));
        assert_eq!(info.id, p.id, "{spelling}");
        // The name sent is the one it takes; none sent: the history's.
        let name = if i == 0 { "Renommé" } else { "" };
        let back = h
            .core
            .create_project(spelling, name, COLOR, false, None, true)
            .await
            .unwrap();
        assert_eq!(back.id, p.id, "{spelling}");
        // The same folder: its path stays the one its agents and worktrees were made from.
        assert_eq!(back.path, p.path, "{spelling}");
        assert_eq!(back.name, "Renommé");
        assert!(h.core.agent(&a).is_ok());
        close(&h, &back.id).await;
    }
    // Another folder has none: not a folder above or beside it either.
    let b = repo(&h.dir.join("b"));
    assert!(history_of(&h.core, &b.to_string_lossy()).await.is_none());
    let parent = r.parent().unwrap().to_string_lossy().to_string();
    assert!(history_of(&h.core, &parent).await.is_none());
    assert!(history_of(&h.core, &format!("{typed}2")).await.is_none());
    assert!(history_of(&h.core, &r.join("src").to_string_lossy())
        .await
        .is_none());
}

#[tokio::test]
async fn a_folder_that_is_gone_cannot_be_added_and_its_history_stays() {
    let h = harness("closed-folder-gone");
    let (p, r) = h.project(false).await;
    close(&h, &p.id).await;
    std::fs::remove_dir_all(&r).unwrap();
    assert!(history_of(&h.core, &r.to_string_lossy()).await.is_some());
    let e = h
        .core
        .create_project(&r.to_string_lossy(), "", COLOR, false, None, true)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("n'existe pas"), "{e}");
    assert_eq!(h.core.closed_projects().len(), 1);
}

/// The ids of the projects the account `id` is reserved to.
fn reserved_to(h: &Harness, id: &str) -> Vec<String> {
    h.core
        .settings
        .read()
        .accounts
        .iter()
        .find(|a| a.id == id)
        .unwrap()
        .projects
        .clone()
}

/// The account `id` reserved to `projects` (« Réservé aux projets »).
fn reserve(h: &Harness, id: &str, projects: &[&str]) {
    let account = h
        .core
        .settings
        .read()
        .accounts
        .iter()
        .find(|a| a.id == id)
        .unwrap()
        .clone();
    h.core
        .update_claude_account(Account {
            projects: projects.iter().map(|x| x.to_string()).collect(),
            ..account
        })
        .unwrap();
}

#[tokio::test]
async fn the_accounts_a_project_is_reserved_to_stay_so_while_its_history_is_kept_and_when_it_comes_back(
) {
    let h = harness("closed-accounts-back");
    let (p, r) = h.project(false).await;
    let pro = extra_account(&h, "pro", "Pro");
    let team = extra_account(&h, "equipe", "Équipe");
    reserve(&h, "principal", &[&p.id]);
    reserve(&h, &pro.id, &[&p.id, "autre"]);
    // Team is reserved to others: it was not to this one.
    reserve(&h, &team.id, &["autre"]);
    h.core
        .update_project(Project {
            account: pro.id.clone(),
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();

    close(&h, &p.id).await;
    // Nothing changes for the accounts: its history is kept, so is its place in their lists.
    assert_eq!(reserved_to(&h, "principal"), [p.id.as_str()]);
    assert_eq!(reserved_to(&h, &pro.id), [p.id.as_str(), "autre"]);
    assert_eq!(reserved_to(&h, &team.id), ["autre"]);

    let back = add_again(&h, &r).await;
    // Back, nothing is put again (nothing was taken away): the lists are as they were.
    assert_eq!(reserved_to(&h, "principal"), [p.id.as_str()]);
    assert_eq!(reserved_to(&h, &pro.id), [p.id.as_str(), "autre"]);
    assert_eq!(reserved_to(&h, &team.id), ["autre"]);
    // Its preferred account is still there.
    assert_eq!(back.account, "pro");
}

#[tokio::test]
async fn the_only_project_an_account_is_reserved_to_closed_leaves_it_serving_no_other_project_until_the_history_is_erased(
) {
    let h = harness("closed-accounts-only");
    let (p, _) = h.project(false).await;
    let q = h
        .core
        .create_project(
            &repo(&h.dir.join("b")).to_string_lossy(),
            "client",
            COLOR,
            false,
            None,
            true,
        )
        .await
        .unwrap();
    reserve(&h, "principal", &[&p.id]);
    let serves_q =
        || crate::accounts::serves_any(&h.core.settings.read(), EngineId::Claude, Some(&q.id));
    assert!(!serves_q());

    close(&h, &p.id).await;

    // The list still holds the closed project: Principal serves no other one (as before the close).
    assert_eq!(reserved_to(&h, "principal"), [p.id.as_str()]);
    assert!(!serves_q());
    // The history erased, the list is empty: Principal serves every project.
    h.core.erase_closed_project(&p.id).unwrap();
    assert!(reserved_to(&h, "principal").is_empty());
    assert!(serves_q());
}

#[tokio::test]
async fn a_history_that_still_lists_its_accounts_is_read_all_the_same() {
    let h = harness("closed-accounts-old-file");
    let (p, r) = h.project(false).await;
    reserve(&h, "principal", &[&p.id]);
    close(&h, &p.id).await;
    // As the file was written before the accounts' lists kept the closed projects.
    let mut kept = read_closed(&h, &p.id);
    kept["accounts"] = json!(["principal"]);
    std::fs::write(
        h.core.data.closed_project_file(&p.id),
        serde_json::to_vec(&kept).unwrap(),
    )
    .unwrap();

    let back = add_again(&h, &r).await;

    assert_eq!(back.id, p.id);
    assert_eq!(reserved_to(&h, "principal"), [p.id.as_str()]);
}

#[tokio::test]
async fn an_account_removed_meanwhile_is_left_out_and_so_is_a_preferred_account_that_is_gone() {
    let h = harness("closed-accounts-gone");
    let (p, r) = h.project(false).await;
    let pro = extra_account(&h, "pro", "Pro");
    let team = extra_account(&h, "equipe", "Équipe");
    for account in [&pro, &team] {
        h.core
            .update_claude_account(Account {
                projects: vec![p.id.clone()],
                ..account.clone()
            })
            .unwrap();
    }
    h.core
        .update_project(Project {
            account: pro.id.clone(),
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    close(&h, &p.id).await;
    h.core.remove_claude_account(&pro.id).unwrap();

    let back = add_again(&h, &r).await;

    let accounts = h.core.settings.read().accounts.clone();
    assert!(accounts.iter().all(|a| a.id != "pro"));
    let team_now = accounts.iter().find(|a| a.id == "equipe").unwrap();
    assert_eq!(team_now.projects, [p.id.as_str()]);
    assert_eq!(back.account, "");
    assert_eq!(h.core.project(&p.id).unwrap().account, "");
    assert!(toasts(&h).is_empty());
}

#[tokio::test]
async fn a_ticket_at_work_comes_back_blocked_and_is_resumed_by_hand() {
    let h = harness("closed-ticket-doing");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a = agent_that_talked(&h, &p.id).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("En cours"))
        .await
        .unwrap();
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            t.agent_id = Some(a.clone());
            t.started_at = Some(now_ms());
            Ok(())
        })
        .unwrap();
    h.core.agent(&a).unwrap().lock().meta.ticket_id = Some(t.id.clone());
    // One in review with a conflict (no validation runs: the project is not closed while one
    // does), and one at work that is blocked already.
    let review = h
        .core
        .ticket_create(&p.id, draft("A tester"))
        .await
        .unwrap();
    h.core
        .edit_ticket(&review.id, |t| {
            t.column = Column::Review;
            t.conflict = true;
            Ok(())
        })
        .unwrap();
    let held = h.core.ticket_create(&p.id, draft("Retenu")).await.unwrap();
    h.core
        .edit_ticket(&held.id, |t| {
            t.column = Column::Doing;
            t.blocked = Some("limite d'usage atteinte".into());
            Ok(())
        })
        .unwrap();
    let todo = h.core.ticket_create(&p.id, draft("À faire")).await.unwrap();

    close(&h, &p.id).await;
    add_again(&h, &r).await;

    let doing = h.core.ticket(&t.id).unwrap();
    assert_eq!(doing.column, Column::Doing);
    assert_eq!(
        doing.blocked.as_deref(),
        Some("Projet fermé pendant le ticket")
    );
    assert_eq!(doing.agent_id.as_deref(), Some(a.as_str()));
    let in_review = h.core.ticket(&review.id).unwrap();
    assert_eq!(in_review.column, Column::Review);
    assert_eq!(in_review.step, None);
    assert_eq!(in_review.blocked, None);
    assert!(in_review.conflict);
    // What was held, or not started, is as it was.
    let held = h.core.ticket(&held.id).unwrap();
    assert_eq!(held.blocked.as_deref(), Some("limite d'usage atteinte"));
    let todo = h.core.ticket(&todo.id).unwrap();
    assert_eq!((todo.column, todo.blocked), (Column::Todo, None));

    // « Reprendre »: its agent is asked to go on.
    h.core.ticket_resume(&t.id).await.unwrap();
    let asked = crate::board::resume_message(crate::i18n::Lang::Fr, &doing.key);
    h.wait("the agent asked to go on", |h| {
        h.items(&a)
            .iter()
            .any(|i| i["kind"] == "user" && i["text"] == asked.as_str())
    })
    .await;
    assert!(h.alive(&a));
}

#[tokio::test]
async fn the_block_of_a_ticket_at_work_is_written_in_the_language_of_the_interface() {
    let h = harness("closed-ticket-en");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let t = h.core.ticket_create(&p.id, draft("Working")).await.unwrap();
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Doing;
            Ok(())
        })
        .unwrap();
    h.core.settings.write().language = "en".into();

    close(&h, &p.id).await;
    add_again(&h, &r).await;

    assert_eq!(
        h.core.ticket(&t.id).unwrap().blocked.as_deref(),
        Some("Project closed during the ticket")
    );
}

#[test]
fn a_validation_found_under_way_when_the_history_is_written_is_cut_in_the_language_asked() {
    // A project is not closed while one runs, but a history never keeps one under way if it is.
    let mut ticket = Ticket {
        column: Column::Review,
        step: Some("Tests…".into()),
        conflict: true,
        ..Default::default()
    };
    ticket.blocked = None;
    for (lang, said) in [
        (crate::i18n::Lang::Fr, "Validation interrompue"),
        (crate::i18n::Lang::En, "Approval interrupted"),
    ] {
        let kept = crate::closed::closed_ticket(ticket.clone(), lang);
        assert_eq!(kept.step, None);
        assert!(!kept.conflict);
        assert_eq!(kept.blocked.as_deref(), Some(said));
        assert_eq!(kept.column, Column::Review);
    }
}

#[tokio::test]
async fn the_keys_of_the_tickets_are_not_used_again_after_a_project_comes_back() {
    let h = harness("closed-keys");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let mut keys = Vec::new();
    for title in ["Un", "Deux", "Trois"] {
        keys.push(h.core.ticket_create(&p.id, draft(title)).await.unwrap());
    }
    // The last one is deleted: its number is not given again either.
    h.core.ticket_delete(&keys[2].id).await.unwrap();
    close(&h, &p.id).await;
    add_again(&h, &r).await;

    let next = h.core.ticket_create(&p.id, draft("Quatre")).await.unwrap();

    assert_eq!(keys[0].key, "DEM-1");
    assert_eq!(keys[2].key, "DEM-3");
    assert_eq!(next.key, "DEM-4");
    let mut all: Vec<String> = h
        .core
        .tickets
        .read()
        .iter()
        .map(|t| t.key.clone())
        .collect();
    all.sort();
    all.dedup();
    assert_eq!(all, ["DEM-1", "DEM-2", "DEM-4"]);
}

#[tokio::test]
async fn a_project_whose_history_has_nowhere_to_go_stays_open() {
    let h = harness("closed-no-folder");
    let (p, _) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a = agent_that_talked(&h, &p.id).await;
    let t = h.core.ticket_create(&p.id, draft("Un")).await.unwrap();
    // A file where the folder of the closed projects should be.
    std::fs::write(h.core.data.closed_projects(), "x").unwrap();

    let e = h.core.remove_project(&p.id, false).await.unwrap_err();

    assert!(e.to_string().contains("le projet reste ouvert"), "{e}");
    assert!(h.core.project(&p.id).is_ok());
    assert!(h.alive(&a));
    assert!(h.core.ticket(&t.id).is_ok());
    assert!(conversation_log(&h, &a).exists());
    // Without a history to keep, nothing needs the folder.
    h.core.remove_project(&p.id, true).await.unwrap();
    assert!(h.core.project(&p.id).is_err());
}

#[tokio::test]
async fn a_history_that_cannot_be_written_keeps_the_project_open_before_anything_stops() {
    let h = harness("closed-write-fails");
    let (p, _) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    // A folder where its file goes: the file cannot take its place.
    std::fs::create_dir_all(h.core.data.closed_project_file(&p.id)).unwrap();

    let e = h.core.remove_project(&p.id, false).await.unwrap_err();

    // Written before anything stops: nothing stopped, nothing left the app.
    assert!(
        e.to_string()
            .starts_with("L'historique du projet n'a pas pu être écrit ("),
        "{e}"
    );
    assert!(
        e.to_string().ends_with(") : le projet reste ouvert."),
        "{e}"
    );
    assert!(h.core.project(&p.id).is_ok());
    assert!(h.alive(&a));
    assert!(toasts(&h).is_empty());
    assert!(conversation_log(&h, &a).exists());
}

#[tokio::test]
async fn what_a_turn_cut_by_the_closing_left_open_comes_back_closed() {
    let h = harness("closed-open-items");
    let (p, r) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // Its turn waits for a permission, its tool not run yet.
    h.core
        .send_message(&a, "permission".into(), vec![])
        .await
        .unwrap();
    h.wait("the permission asked", |h| {
        h.items(&a).iter().any(|i| i["kind"] == "permission")
    })
    .await;
    close(&h, &p.id).await;
    add_again(&h, &r).await;

    let items = h.items(&a);
    let asked: Vec<&Value> = items.iter().filter(|i| i["kind"] == "permission").collect();
    assert!(
        !asked.is_empty() && asked.iter().all(|i| i["cancelled"] == true),
        "{asked:?}"
    );
    assert!(
        items
            .iter()
            .all(|i| i["streaming"] != true && !(i["kind"] == "tool" && i["status"] == "running")),
        "{items:?}"
    );
}

#[tokio::test]
async fn an_erased_history_is_erased_by_one_function_whichever_way_it_goes() {
    // Closing with the history erased, and erasing a closed project's: the same files go.
    let h = harness("closed-erase-both");
    let (p, _) = h.project(false).await;
    let other_dir = repo(&h.dir.join("b"));
    let q = h
        .core
        .create_project(
            &other_dir.to_string_lossy(),
            "autre",
            COLOR,
            false,
            None,
            true,
        )
        .await
        .unwrap();
    let (a, b) = (
        agent_that_talked(&h, &p.id).await,
        agent_that_talked(&h, &q.id).await,
    );
    let files = |id: &str| [conversation_log(&h, id), setup_log_of(&h, id)];
    let (of_a, of_b) = (files(&a), files(&b));
    assert!(of_a.iter().chain(&of_b).all(|f| f.exists()));

    h.core.remove_project(&p.id, true).await.unwrap();
    assert!(of_a.iter().all(|f| !f.exists()));
    assert!(of_b.iter().all(|f| f.exists()));

    close(&h, &q.id).await;
    assert!(of_b.iter().all(|f| f.exists()));
    h.core.erase_closed_project(&q.id).unwrap();
    assert!(of_b.iter().all(|f| !f.exists()));
}

/// The app started again on the data of the harness: what it finds is what was last saved.
fn restarted(h: &Harness) -> (Arc<Core<MockRuntime>>, tauri::App<MockRuntime>) {
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
    (core, app)
}

/// The ticket at work on the agent.
fn at_work(h: &Harness, ticket_id: &str, agent_id: &str) {
    h.core
        .edit_ticket(ticket_id, |t| {
            t.column = Column::Doing;
            t.agent_id = Some(agent_id.to_string());
            t.started_at = Some(now_ms());
            Ok(())
        })
        .unwrap();
}

fn read_closed(h: &Harness, id: &str) -> Value {
    serde_json::from_slice(&std::fs::read(h.core.data.closed_project_file(id)).unwrap()).unwrap()
}

#[tokio::test]
async fn the_history_kept_is_the_project_as_it_is_when_it_leaves_not_as_it_was_when_the_closing_began(
) {
    let h = harness("closed-latest-project");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a = agent_that_talked(&h, &p.id).await;
    let first = h.core.ticket_create(&p.id, draft("Un")).await.unwrap();
    // A start is under way (a warm-up is one): the closing waits for it, as an agent's deletion does.
    let lock = h.core.spawn_lock(&a);
    let guard = lock.lock().await;
    let closing = tokio::spawn({
        let (core, id) = (h.core.clone(), p.id.clone());
        async move { core.remove_project(&id, false).await }
    });
    h.wait("the first history written", |h| {
        h.core.data.closed_project_file(&p.id).exists()
    })
    .await;

    // Meanwhile the project goes on: a ticket takes a number, a setting changes.
    let second = h.core.ticket_create(&p.id, draft("Deux")).await.unwrap();
    h.core
        .update_project(Project {
            name: "Renommé".into(),
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    drop(guard);
    closing.await.unwrap().unwrap();

    let back = add_again(&h, &r).await;
    assert_eq!(back.name, "Renommé");
    let third = h.core.ticket_create(&p.id, draft("Trois")).await.unwrap();
    assert_eq!(
        [&first.key, &second.key, &third.key],
        ["DEM-1", "DEM-2", "DEM-3"]
    );
}

#[tokio::test]
async fn closing_a_project_twice_at_once_keeps_every_agent_and_ticket() {
    let h = harness("closed-twice");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a1 = agent_that_talked(&h, &p.id).await;
    let a2 = agent_that_talked(&h, &p.id).await;
    let t1 = h.core.ticket_create(&p.id, draft("Un")).await.unwrap();
    let t2 = h.core.ticket_create(&p.id, draft("Deux")).await.unwrap();
    // Both closings are under way together: the first waits for a start, the second is told.
    let lock = h.core.spawn_lock(&a1);
    let guard = lock.lock().await;

    let (x, y, ()) = tokio::join!(
        h.core.remove_project(&p.id, false),
        h.core.remove_project(&p.id, false),
        async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            drop(guard);
        }
    );

    x.unwrap();
    y.unwrap();
    let kept = read_closed(&h, &p.id);
    assert_eq!(kept["agents"].as_array().unwrap().len(), 2, "{kept}");
    assert_eq!(kept["tickets"].as_array().unwrap().len(), 2, "{kept}");
    add_again(&h, &r).await;
    assert!(h.core.agent(&a1).is_ok() && h.core.agent(&a2).is_ok());
    assert!(h.core.ticket(&t1.id).is_ok() && h.core.ticket(&t2.id).is_ok());
    // Closed once more afterwards: the closing in progress was forgotten.
    close(&h, &p.id).await;
    assert!(h.core.project(&p.id).is_err());
    assert_eq!(h.core.closed_projects().len(), 1);
}

#[tokio::test]
async fn a_state_that_cannot_be_saved_leaves_the_file_of_the_project_that_came_back() {
    let h = harness("closed-state-fails");
    let (p, r) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    // A folder where the state's temporary file goes: nothing can be saved.
    let tmp = h.core.data.state_file().with_extension("tmp");
    std::fs::create_dir_all(&tmp).unwrap();

    let back = add_again(&h, &r).await;

    assert_eq!(back.id, p.id);
    assert!(h.core.agent(&a).is_ok());
    // The state does not have the project: its history is still on the disk, ignored while the
    // project is open.
    assert!(h.core.data.closed_project_file(&p.id).exists());
    assert!(h.core.closed_projects().is_empty());
    // Saved at last, and the app started again: the project and its agent are there, the file is not.
    std::fs::remove_dir(&tmp).unwrap();
    h.core.save_now();
    let (re, _app) = restarted(&h);
    assert!(re.project(&p.id).is_ok() && re.agent(&a).is_ok());
    assert!(closed_files(&h).is_empty());
}

#[tokio::test]
async fn a_stop_in_the_middle_of_a_closing_gives_back_the_agents_and_tickets_the_state_missed() {
    let h = harness("closed-crash");
    let (p, _) = h.project(false).await;
    quiet_board(&h, &p.id);
    // A project closed long ago: it stays one.
    let other = h
        .core
        .create_project(
            &repo(&h.dir.join("b")).to_string_lossy(),
            "autre",
            COLOR,
            false,
            None,
            true,
        )
        .await
        .unwrap();
    close(&h, &other.id).await;
    let a1 = agent_that_talked(&h, &p.id).await;
    let a2 = agent_that_talked(&h, &p.id).await;
    let t1 = h.core.ticket_create(&p.id, draft("Un")).await.unwrap();
    let t2 = h.core.ticket_create(&p.id, draft("Deux")).await.unwrap();
    at_work(&h, &t2.id, &a2);
    // A save in the middle of the closing found the project with one agent and one ticket.
    h.core.save_now();
    let file = h.core.data.state_file();
    let mut state: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    state["agents"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["id"] != a2.as_str());
    state["tickets"]
        .as_array_mut()
        .unwrap()
        .retain(|t| t["id"] != t2.id.as_str());
    std::fs::write(&file, serde_json::to_vec(&state).unwrap()).unwrap();
    // The file written before anything stopped has them all. The app stops there.
    close(&h, &p.id).await;
    assert_eq!(closed_files(&h).len(), 2);

    let (re, _app) = restarted(&h);

    assert!(re.project(&p.id).is_ok());
    assert!(re.agent(&a1).is_ok() && re.agent(&a2).is_ok());
    assert!(re.ticket(&t1.id).is_ok());
    // A ticket at work in the file is held there already: taken up by hand.
    assert_eq!(
        re.ticket(&t2.id).unwrap().blocked.as_deref(),
        Some("Projet fermé pendant le ticket")
    );
    // Saved, and the file is gone; the other closed project is not touched.
    let saved: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(saved["agents"].as_array().unwrap().len(), 2);
    assert_eq!(saved["tickets"].as_array().unwrap().len(), 2);
    assert_eq!(closed_files(&h), [format!("{}.json", other.id)]);
    let listed: Vec<String> = re.closed_projects().into_iter().map(|c| c.id).collect();
    assert_eq!(listed, [other.id]);
}

#[tokio::test]
async fn the_ports_of_a_closed_projects_agents_are_not_given_away_meanwhile() {
    let h = harness("closed-ports");
    let (p, r) = h.project(false).await;
    let of_agent = |base: u16| AgentOptions {
        port_base: Some(base),
        ..Default::default()
    };
    let held = h.core.reserve_ports().unwrap();
    let a = h
        .core
        .create_agent_with(&p.id, of_agent(held))
        .await
        .unwrap()
        .meta
        .id;
    h.core.unreserve_ports(Some(held));
    // An archived agent gives its block back, as ever.
    let spare = h.core.reserve_ports().unwrap();
    let b = h
        .core
        .create_agent_with(&p.id, of_agent(spare))
        .await
        .unwrap()
        .meta
        .id;
    h.core.unreserve_ports(Some(spare));
    h.core.agent(&b).unwrap().lock().meta.archived = true;
    assert_ne!(held, spare);

    close(&h, &p.id).await;

    let next = h.core.reserve_ports().unwrap();
    assert_ne!(next, held, "the block of a closed project's agent");
    assert_eq!(next, spare);
    h.core.unreserve_ports(Some(next));
    // Back, its agent holds the block alone.
    add_again(&h, &r).await;
    assert_eq!(h.agent(&a).port_base, Some(held));
    let other = h.core.reserve_ports().unwrap();
    assert_ne!(other, held);
    h.core.unreserve_ports(Some(other));
    // Closed again, then its history erased: the index gives the block back.
    close(&h, &p.id).await;
    let taken = h.core.reserve_ports().unwrap();
    assert_ne!(taken, held);
    h.core.unreserve_ports(Some(taken));
    h.core.erase_closed_project(&p.id).unwrap();
    assert_eq!(h.core.reserve_ports(), Some(held));
}

#[tokio::test]
async fn the_ports_of_a_project_being_closed_stay_taken_while_its_agents_leave_the_app() {
    let h = harness("closed-ports-closing");
    let (p, _) = h.project(false).await;
    let held = h.core.reserve_ports().unwrap();
    let a = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                port_base: Some(held),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id;
    h.core.unreserve_ports(Some(held));
    // The closing is under way: its history is written, its agent is out of the app already, the
    // project is not yet.
    let _closing = crate::closed::Closing::start(&h.core.closing, &p.id).unwrap();
    h.core
        .write_history(
            h.core.project(&p.id).unwrap(),
            vec![h.agent(&a)],
            Vec::new(),
            None,
        )
        .unwrap();
    h.core.agents.write().remove(&a);

    let next = h.core.reserve_ports().unwrap();

    assert_ne!(
        next, held,
        "the block of an agent of the project being closed"
    );
}

#[tokio::test]
async fn the_remote_control_agents_of_a_project_that_comes_back_start_again() {
    let h = harness("closed-remote");
    let (p, r) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.set_remote_control(&a, true).await.unwrap();
    let session = h.agent(&a).remote_session.unwrap();
    // One archived that had remote control: it stays down.
    let b = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.archive_agent(&b, true).await.unwrap();
    h.core.agent(&b).unwrap().lock().meta.remote_control = true;
    assert!(h.agent(&b).remote_control && h.agent(&b).archived);
    close(&h, &p.id).await;
    add_again(&h, &r).await;
    assert!(!h.alive(&a), "started by the app only");

    // As the app does when it starts: the process is up, linked to its remote session again.
    let linked = |h: &Harness| {
        h.alive(&a)
            && h.fake_log(&r, "control")
                .last()
                .is_some_and(|c| c["reattach_session_id"] == session.as_str())
    };
    tokio::time::timeout(Duration::from_secs(15), h.wait("the remote agent", linked))
        .await
        .expect("the remote control agent was not started again");
    assert!(!h.alive(&b));
}

/// Whether the Remote Control of the agent is in force from `source` and linked to claude.ai
/// (with none: out, and not linked).
fn remote_held(h: &Harness, id: &str, source: Option<RemoteSource>) -> bool {
    let handle = h.core.agent(id).unwrap();
    let rt = handle.lock();
    rt.meta.remote_control == source.is_some()
        && rt.meta.remote_source == source
        && rt.remote_linked == source.is_some()
}

/// The Remote Control requests the fake CLI started in `cwd` got so far.
fn remote_asked(h: &Harness, cwd: &str) -> Vec<Value> {
    h.fake_log(Path::new(cwd), "control")
}

#[tokio::test]
async fn the_agents_of_a_project_with_remote_control_come_back_each_as_it_was_set_whatever_the_global_setting_did_meanwhile(
) {
    use RemoteSource::{Global, Hand, Project as OfProject};
    let h = harness("closed-remote-settings");
    let (p, r) = h.project(true).await;
    // « Remote control pour les agents de ce projet ».
    let mut project = h.core.project(&p.id).unwrap();
    project.remote_control = true;
    h.core.update_project(project).unwrap();
    // One covered by that setting, one set apart by hand, one turned on by hand.
    let covered = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let apart = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let by_hand = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let all = [covered.clone(), apart.clone(), by_hand.clone()];
    h.wait("the project's setting in force for its three agents", |h| {
        all.iter().all(|id| remote_held(h, id, Some(OfProject)))
    })
    .await;
    h.core.set_remote_control(&apart, false).await.unwrap();
    h.core.set_remote_control(&by_hand, true).await.unwrap();
    assert!(remote_held(&h, &covered, Some(OfProject)));
    assert!(remote_held(&h, &apart, None) && h.agent(&apart).remote_off);
    assert!(remote_held(&h, &by_hand, Some(Hand)));
    // Each has a folder of its own (a worktree), so a log of requests of its own.
    let cwds: Vec<String> = all.iter().map(|id| h.agent(id).cwd).collect();
    assert!(cwds[0] != cwds[1] && cwds[1] != cwds[2] && cwds[0] != cwds[2]);
    // Linked: the fake CLI got their requests.
    assert!(!remote_asked(&h, &cwds[0]).is_empty());
    assert!(!remote_asked(&h, &cwds[2]).is_empty());
    let session_of = |id: &str| h.agent(id).remote_session.unwrap();
    let (covered_session, hand_session) = (session_of(&covered), session_of(&by_hand));
    assert!(h.agent(&apart).remote_session.is_none());

    close(&h, &p.id).await;

    assert!(all.iter().all(|id| h.core.agent(id).is_err()));
    let asked: Vec<usize> = cwds.iter().map(|c| remote_asked(&h, c).len()).collect();
    let launched: Vec<usize> = cwds
        .iter()
        .map(|c| h.launches(Path::new(c)).len())
        .collect();
    // The global setting turned on while the project is closed: nothing of it is started for it,
    // no request is made for its agents.
    h.remote_all(true);
    h.settle_remote().await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(all.iter().all(|id| h.core.agent(id).is_err()));
    let still: Vec<usize> = cwds.iter().map(|c| remote_asked(&h, c).len()).collect();
    assert_eq!(still, asked);
    for (cwd, before) in cwds.iter().zip(&launched) {
        assert_eq!(h.launches(Path::new(cwd)).len(), *before, "{cwd}");
    }

    let back = add_again(&h, &r).await;

    // The project's setting is still on; the agent set apart still is.
    assert_eq!(back.id, p.id);
    assert!(back.remote_control && h.core.project(&p.id).unwrap().remote_control);
    let m = h.agent(&apart);
    assert!(m.remote_off && !m.remote_control && m.remote_source.is_none());
    // The other two hold Remote Control again, the covered one from the global setting now, and
    // their process is up and linked to the remote session they had.
    tokio::time::timeout(
        Duration::from_secs(15),
        h.wait("the remote agents linked again", |h| {
            h.alive(&covered)
                && h.alive(&by_hand)
                && remote_held(h, &covered, Some(Global))
                && remote_held(h, &by_hand, Some(Hand))
        }),
    )
    .await
    .expect("the remote control agents were not started again");
    h.settle_remote().await;
    for (i, session) in [(0, &covered_session), (2, &hand_session)] {
        let tail = remote_asked(&h, &cwds[i]).split_off(asked[i]);
        assert_eq!(tail.len(), 1, "{tail:?}");
        assert_eq!(tail[0]["enabled"], true, "{tail:?}");
        assert_eq!(tail[0]["reattach_session_id"], session.as_str(), "{tail:?}");
        assert_eq!(h.launches(Path::new(&cwds[i])).len(), launched[i] + 1);
    }
    assert_eq!(
        h.agent(&covered).remote_session.as_deref(),
        Some(covered_session.as_str())
    );
    assert_eq!(
        h.agent(&by_hand).remote_session.as_deref(),
        Some(hand_session.as_str())
    );
    // The one set apart has none: no request for it, no process.
    let m = h.agent(&apart);
    assert!(m.remote_off && !m.remote_control && m.remote_source.is_none());
    assert!(m.remote_session.is_none() && m.remote_url.is_none());
    assert_eq!(remote_asked(&h, &cwds[1]).len(), asked[1]);
    assert_eq!(h.launches(Path::new(&cwds[1])).len(), launched[1]);
    assert!(!h.alive(&apart));
}

#[tokio::test]
async fn closing_does_not_read_the_conversation_of_an_agent_that_had_nothing_running() {
    let h = harness("closed-light");
    let (p, r) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    add_again(&h, &r).await;
    let handle = h.core.agent(&a).unwrap();
    // Back from the disk: no process, its conversation not read.
    assert!(!handle.lock().conv.is_loaded());

    close(&h, &p.id).await;

    assert!(!handle.lock().conv.is_loaded());
}

#[tokio::test]
async fn adding_the_folder_twice_at_once_gives_the_project_that_came_back_not_a_blank_one() {
    let h = harness("closed-add-twice");
    let (p, r) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;

    let (x, y) = tokio::join!(add_again(&h, &r), add_again(&h, &r));

    assert_eq!(
        (x.id.as_str(), y.id.as_str()),
        (p.id.as_str(), p.id.as_str())
    );
    assert_eq!(h.core.projects.read().len(), 1);
    assert_eq!(h.core.project_agents(&p.id).len(), 1);
    assert!(h.core.agent(&a).is_ok());
}

#[tokio::test]
async fn an_agent_whose_id_is_no_name_of_its_own_is_not_given_back() {
    let h = harness("closed-bad-id");
    let (p, r) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    // A history edited by hand: an agent whose id would name a log outside of the conversations.
    let mut kept = read_closed(&h, &p.id);
    let mut evil = kept["agents"][0].clone();
    evil["id"] = json!("../evil");
    kept["agents"].as_array_mut().unwrap().push(evil);
    std::fs::write(
        h.core.data.closed_project_file(&p.id),
        serde_json::to_vec(&kept).unwrap(),
    )
    .unwrap();

    add_again(&h, &r).await;

    assert!(h.core.agent("../evil").is_err());
    let ids: Vec<String> = h
        .core
        .project_agents(&p.id)
        .iter()
        .map(|h| h.lock().meta.id.clone())
        .collect();
    assert_eq!(ids, [a]);
}

#[tokio::test]
async fn a_closed_project_on_an_unreachable_share_neither_slows_nor_misleads_the_search_of_a_folder(
) {
    let h = harness("closed-unreachable");
    let (p, r) = h.project(false).await;
    close(&h, &p.id).await;
    // Closed later, on a share that is not there (a host never asked about before: the system
    // remembers a failure for a while).
    let host = format!("nohost-esc89-{}-{}", std::process::id(), now_ms());
    let share = if cfg!(windows) {
        format!(r"\\{host}\share\x")
    } else {
        format!("//{host}/share/x")
    };
    std::fs::write(
        h.core.data.closed_project_file("unc1"),
        serde_json::to_vec(&ClosedProject {
            project: Project {
                id: "unc1".into(),
                path: share,
                ..p.clone()
            },
            agents: Vec::new(),
            tickets: Vec::new(),
            selected_agent: None,
            closed_at: now_ms() + 1,
        })
        .unwrap(),
    )
    .unwrap();
    let elsewhere = repo(&h.dir.join("b"));
    // The folder of the other one, spelt through a link.
    let link = h.dir.join("link");
    let linked = crate::paths::make_dir_link(&r, &link);
    // The histories are read when the app starts.
    let (re, _app) = restarted(&h);

    let started = std::time::Instant::now();
    assert!(history_of(&re, &elsewhere.to_string_lossy())
        .await
        .is_none());
    assert_eq!(
        history_of(&re, &r.to_string_lossy()).await.unwrap().id,
        p.id
    );
    if linked {
        assert_eq!(
            history_of(&re, &link.to_string_lossy()).await.unwrap().id,
            p.id
        );
    }
    let took = started.elapsed();

    assert!(took < Duration::from_secs(2), "{took:?}");
}

/// A history written by hand in the closed projects' folder, as an editor or another version of
/// the app would leave it.
fn write_history_by_hand(h: &Harness, closed: &ClosedProject) {
    std::fs::create_dir_all(h.core.data.closed_projects()).unwrap();
    std::fs::write(
        h.core.data.closed_project_file(&closed.project.id),
        serde_json::to_vec(closed).unwrap(),
    )
    .unwrap();
}

/// The ids of the closed projects, in the order listed.
fn listed_ids(core: &Arc<Core<MockRuntime>>) -> Vec<String> {
    core.closed_projects().into_iter().map(|c| c.id).collect()
}

#[tokio::test]
async fn the_list_the_search_by_folder_and_the_ports_read_the_index_not_the_folder() {
    let h = harness("closed-index-memory");
    let (p, _) = h.project(false).await;
    agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    let first = h.core.reserve_ports().unwrap();
    h.core.unreserve_ports(Some(first));
    // A history that shows up in the folder while the app runs: the app does not look again.
    let other = repo(&h.dir.join("b"));
    let late = ClosedProject {
        project: Project {
            id: "late".into(),
            name: "tard".into(),
            path: other.to_string_lossy().to_string(),
            ..p.clone()
        },
        agents: vec![AgentMeta {
            id: "alate".into(),
            project_id: "late".into(),
            port_base: Some(first),
            ..Default::default()
        }],
        tickets: Vec::new(),
        selected_agent: None,
        closed_at: now_ms() + 1,
    };
    write_history_by_hand(&h, &late);

    assert_eq!(listed_ids(&h.core), [p.id.as_str()]);
    assert!(history_of(&h.core, &other.to_string_lossy())
        .await
        .is_none());
    assert_eq!(h.core.reserve_ports(), Some(first));
    h.core.unreserve_ports(Some(first));

    // At the next start the app reads the folder: the history is listed, found, and keeps its ports.
    let (re, _app) = restarted(&h);
    assert_eq!(listed_ids(&re), ["late".to_string(), p.id.clone()]);
    assert_eq!(
        history_of(&re, &other.to_string_lossy()).await.unwrap().id,
        "late"
    );
    let next = re.reserve_ports().unwrap();
    assert_ne!(next, first);
}

#[tokio::test]
async fn the_size_of_a_closed_project_is_counted_when_it_is_written_not_each_time_the_list_is_asked_for(
) {
    let h = harness("closed-size-once");
    let (p, _) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    let size = h.core.closed_projects()[0].size;
    assert!(size > 0);

    // Its conversation grows behind the app's back: the list does not look at it again.
    let mut log = std::fs::OpenOptions::new()
        .append(true)
        .open(conversation_log(&h, &a))
        .unwrap();
    std::io::Write::write_all(&mut log, &[b' '; 1000]).unwrap();
    drop(log);
    assert_eq!(h.core.closed_projects()[0].size, size);

    let (re, _app) = restarted(&h);
    assert_eq!(re.closed_projects()[0].size, size + 1000);
}

#[tokio::test]
async fn the_size_of_a_closed_project_counts_the_copies_of_the_files_joined_to_its_agents() {
    let h = harness("closed-agent-files-size");
    let (p, _) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    let archived = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core.archive_agent(&archived, true).await.unwrap();
    // Two copies of an agent, one of an archived agent of the project, one of another owner.
    crate::attachfiles_tests::copy(&h, &a, "rapport.xlsx", &vec![7u8; 600_000]);
    crate::attachfiles_tests::copy(&h, &a, "autre.zip", &vec![8u8; 300_000]);
    crate::attachfiles_tests::copy(&h, &archived, "vieux.bin", &vec![9u8; 100_000]);
    crate::attachfiles_tests::copy(&h, "draft", "brouillon.bin", &vec![1u8; 5_000_000]);
    close(&h, &p.id).await;
    let size = h.core.closed_projects()[0].size;
    // Closing keeps them, and the history counts them (not the drafts', which are no agent's).
    assert!(h.core.data.agent_files(&a).is_dir());
    assert!(size >= 1_000_000, "{size}");
    assert!(size < 2_000_000, "{size}");
    // Read again at the next start, the same.
    let (re, _app) = restarted(&h);
    assert_eq!(re.closed_projects()[0].size, size);
}

#[tokio::test]
async fn closing_a_project_keeps_the_files_of_its_tickets_and_erasing_its_history_removes_them() {
    use crate::tickets::DraftFile;
    let h = harness("closed-ticket-files");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let copy = crate::attachfiles_tests::copy(&h, "draft", "rapport.xlsx", &vec![7u8; 1_000_000]);
    let t = h
        .core
        .ticket_create(
            &p.id,
            TicketDraft {
                files: Some(vec![DraftFile {
                    name: copy.name,
                    path: copy.path,
                    ..Default::default()
                }]),
                ..draft("Avec fichier")
            },
        )
        .await
        .unwrap();
    let dir = h.core.data.ticket_dir(&t.id);
    let (uniq, name) = t.files[0].file.split_once('/').unwrap();
    let file = h.core.data.ticket_files(&t.id).join(uniq).join(name);
    close(&h, &p.id).await;
    // Closing keeps them, and the size of the history counts them (one folder below the ticket's).
    assert_eq!(std::fs::metadata(&file).unwrap().len(), 1_000_000);
    assert!(h.core.closed_projects()[0].size >= 1_000_000);
    // The project added again has its ticket with its file, where it was.
    add_again(&h, &r).await;
    assert_eq!(h.core.ticket(&t.id).unwrap().files, t.files);
    assert!(file.is_file());
    // Closed, and its history erased for good: the files go with it.
    close(&h, &p.id).await;
    h.core.erase_closed_project(&p.id).unwrap();
    assert!(!dir.exists());
}

#[tokio::test]
async fn removing_a_project_for_good_removes_the_files_of_its_tickets() {
    use crate::tickets::DraftFile;
    let h = harness("closed-ticket-files-erase");
    let (p, _) = h.project(false).await;
    quiet_board(&h, &p.id);
    let copy = crate::attachfiles_tests::copy(&h, "draft", "rapport.xlsx", b"xlsx");
    let t = h
        .core
        .ticket_create(
            &p.id,
            TicketDraft {
                files: Some(vec![DraftFile {
                    name: copy.name,
                    path: copy.path,
                    ..Default::default()
                }]),
                ..draft("Avec fichier")
            },
        )
        .await
        .unwrap();
    let dir = h.core.data.ticket_dir(&t.id);
    assert!(dir.join("files").is_dir());
    h.core.remove_project(&p.id, true).await.unwrap();
    assert!(!dir.exists());
    assert!(h.core.closed_projects().is_empty());
}

#[tokio::test]
async fn a_project_being_closed_is_not_a_closed_project_before_it_is_gone_from_the_app() {
    let h = harness("closed-not-yet-listed");
    let (p, _) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a = agent_that_talked(&h, &p.id).await;
    // A start is under way: the closing waits for it, its first history already written.
    let lock = h.core.spawn_lock(&a);
    let guard = lock.lock().await;
    let closing = tokio::spawn({
        let (core, id) = (h.core.clone(), p.id.clone());
        async move { core.remove_project(&id, false).await }
    });
    h.wait("the first history written", |h| {
        h.core.data.closed_project_file(&p.id).exists()
    })
    .await;

    // Its file is there, the project too: the history is the open project's own.
    assert!(h.core.closed_projects().is_empty());
    assert!(history_of(&h.core, &p.path).await.is_none());
    assert!(h.core.erase_closed_project(&p.id).is_err());
    drop(guard);
    closing.await.unwrap().unwrap();

    assert_eq!(listed_ids(&h.core), [p.id.as_str()]);
    assert!(history_of(&h.core, &p.path).await.is_some());
}

#[tokio::test]
async fn a_file_that_cannot_be_read_is_listed_as_unreadable_and_erased_alone() {
    let h = harness("closed-unreadable");
    let (p, _) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    let dir = h.core.data.closed_projects();
    // Not JSON; a history named after another project; names that are no plain ids; no history file.
    std::fs::write(dir.join("junk.json"), "{ not json").unwrap();
    let elsewhere = ClosedProject {
        project: Project {
            id: "zzz".into(),
            ..p.clone()
        },
        agents: Vec::new(),
        tickets: Vec::new(),
        selected_agent: None,
        closed_at: now_ms(),
    };
    std::fs::write(
        dir.join("another.json"),
        serde_json::to_vec(&elsewhere).unwrap(),
    )
    .unwrap();
    for left_out in ["not plain.json", "a.b.json", "notes.txt"] {
        std::fs::write(dir.join(left_out), "{").unwrap();
    }

    let (re, _app) = restarted(&h);

    let mut ids = listed_ids(&re);
    ids.sort();
    let mut want = vec![p.id.clone(), "junk".to_string(), "another".to_string()];
    want.sort();
    assert_eq!(ids, want);
    let listed = re.closed_projects();
    let junk = listed.iter().find(|c| c.id == "junk").unwrap();
    assert!(junk.closed_at > now_ms() - 60_000 && junk.closed_at <= now_ms());
    assert_eq!(
        as_json(junk),
        json!({
            "id": "junk", "name": "", "color": "", "path": "", "worktreePerAgent": false,
            "agents": 0, "tickets": 0, "lastActivity": junk.closed_at,
            "closedAt": junk.closed_at, "size": 10, "unreadable": true
        })
    );
    let another = listed.iter().find(|c| c.id == "another").unwrap();
    assert!(another.unreadable && another.name.is_empty());
    assert!(listed
        .iter()
        .find(|c| c.id == p.id)
        .is_some_and(|c| !c.unreadable));

    // « Effacer » removes that file and nothing else.
    re.erase_closed_project("junk").unwrap();
    assert!(!dir.join("junk.json").exists());
    for stays in [
        "another.json",
        "not plain.json",
        "a.b.json",
        "notes.txt",
        &format!("{}.json", p.id),
    ] {
        assert!(dir.join(stays).exists(), "{stays}");
    }
    assert!(conversation_log(&h, &a).exists());
    re.erase_closed_project("another").unwrap();
    assert!(!dir.join("another.json").exists());
    assert_eq!(listed_ids(&re), [p.id.as_str()]);
    // Gone, or never listed (a name that is no plain id): the same answer, nothing removed.
    for id in ["junk", "not plain", "a.b", "notes"] {
        let e = re.erase_closed_project(id).unwrap_err();
        assert_eq!(e.to_string(), "Ce projet fermé n'existe plus.", "{id}");
    }
    assert!(dir.join("not plain.json").exists() && dir.join("notes.txt").exists());
}

#[tokio::test]
async fn a_history_left_beside_its_open_project_is_ignored_everywhere() {
    let h = harness("closed-beside-open");
    let (p, r) = h.project(false).await;
    let a = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    // The state cannot be saved: the project is back, its file stays beside it.
    let tmp = h.core.data.state_file().with_extension("tmp");
    std::fs::create_dir_all(&tmp).unwrap();
    add_again(&h, &r).await;
    assert!(h.core.data.closed_project_file(&p.id).exists());
    let items = h.items(&a);

    assert!(h.core.closed_projects().is_empty());
    assert!(history_of(&h.core, &p.path).await.is_none());
    assert!(h.core.erase_closed_project(&p.id).is_err());
    // Adding the folder again makes a project of its own, and takes nothing from the file.
    let q = h
        .core
        .create_project(&r.to_string_lossy(), "jumeau", COLOR, false, None, true)
        .await
        .unwrap();
    assert_ne!(q.id, p.id);
    assert!(h.core.project_agents(&q.id).is_empty());
    // The open project and its conversation are as they were.
    assert_eq!(h.core.project(&p.id).unwrap().name, "demo");
    assert_eq!(h.items(&a), items);
    assert!(conversation_log(&h, &a).exists());
}

#[tokio::test]
async fn a_folder_with_several_histories_brings_back_the_one_asked_for_by_its_id() {
    let h = harness("closed-history-id");
    let (p, r) = h.project(false).await;
    let path = r.to_string_lossy().to_string();
    let a = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    // A blank project in the same folder, closed later: the folder's latest history.
    tokio::time::sleep(Duration::from_millis(5)).await;
    let (q, back) = h
        .core
        .add_project(&path, "neuf", COLOR, false, None, false, None)
        .await
        .unwrap();
    assert!(!back);
    let b = agent_that_talked(&h, &q.id).await;
    close(&h, &q.id).await;
    // And a project closed in another folder.
    let elsewhere = repo(&h.dir.join("b"));
    let o = h
        .core
        .create_project(
            &elsewhere.to_string_lossy(),
            "autre",
            COLOR,
            false,
            None,
            true,
        )
        .await
        .unwrap();
    close(&h, &o.id).await;

    assert_eq!(history_of(&h.core, &path).await.unwrap().id, q.id);
    assert_eq!(history_with(&h.core, &path, &p.id).await.unwrap().id, p.id);
    assert_eq!(history_with(&h.core, &path, &q.id).await.unwrap().id, q.id);
    // An id that names no history, or one of another folder: the latest of this one, as ever.
    assert_eq!(
        history_with(&h.core, &path, "inconnu").await.unwrap().id,
        q.id
    );
    assert_eq!(history_with(&h.core, &path, &o.id).await.unwrap().id, q.id);

    // Asked for, the older one comes back, with its own agents; the other stays closed.
    let (reopened, back) = h
        .core
        .add_project(&path, "", COLOR, false, None, true, Some(p.id.clone()))
        .await
        .unwrap();
    assert!(back);
    assert_eq!(reopened.id, p.id);
    assert_eq!(reopened.name, "demo");
    assert!(h.core.agent(&a).is_ok() && h.core.agent(&b).is_err());
    assert_eq!(listed_ids(&h.core), [o.id.clone(), q.id.clone()]);
}

#[tokio::test]
async fn a_history_id_is_not_followed_when_the_history_is_not_to_come_back() {
    let h = harness("closed-history-id-blank");
    let (p, r) = h.project(false).await;
    close(&h, &p.id).await;

    let (blank, back) = h
        .core
        .add_project(
            &r.to_string_lossy(),
            "vierge",
            COLOR,
            false,
            None,
            false,
            Some(p.id.clone()),
        )
        .await
        .unwrap();

    assert!(!back);
    assert_ne!(blank.id, p.id);
    assert_eq!(listed_ids(&h.core), [p.id.as_str()]);
}

/// A file that can be read but not removed, until this is dropped: what a file an antivirus is
/// scanning is.
struct Stuck {
    #[cfg(windows)]
    _open: std::fs::File,
    #[cfg(unix)]
    dir: PathBuf,
}

fn stick(file: &Path) -> Stuck {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Shared for reading only: no one can delete the file while this handle lives.
        let open = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(file)
            .unwrap();
        Stuck { _open: open }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // A folder no file can be removed from.
        let dir = file.parent().unwrap().to_path_buf();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        Stuck { dir }
    }
}

#[cfg(unix)]
impl Drop for Stuck {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.dir, std::fs::Permissions::from_mode(0o755));
    }
}

/// The ids the state saved on the disk keeps the files of (`leftoverHistories`).
fn saved_leftovers(h: &Harness) -> Vec<String> {
    let state: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.state_file()).unwrap()).unwrap();
    serde_json::from_value(state["leftoverHistories"].clone()).unwrap_or_default()
}

#[tokio::test]
async fn a_history_whose_file_could_not_be_removed_does_not_bring_back_what_was_deleted_since() {
    let h = harness("closed-leftover");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a1 = agent_that_talked(&h, &p.id).await;
    let a2 = agent_that_talked(&h, &p.id).await;
    let t1 = h.core.ticket_create(&p.id, draft("Un")).await.unwrap();
    let t2 = h.core.ticket_create(&p.id, draft("Deux")).await.unwrap();
    close(&h, &p.id).await;
    let file = h.core.data.closed_project_file(&p.id);
    let stuck = stick(&file);

    add_again(&h, &r).await;

    // The project is back, its file could not go. An agent and a ticket are deleted since.
    assert!(file.exists());
    h.core.delete_agent(&a2, false).await.unwrap();
    h.core.ticket_delete(&t2.id).await.unwrap();
    h.core.save_now();

    // The app starts while the file is still held: the open project is not given them again.
    let (re, _app) = restarted(&h);
    assert!(re.agent(&a1).is_ok() && re.ticket(&t1.id).is_ok());
    assert!(re.agent(&a2).is_err(), "the agent deleted came back");
    assert!(re.ticket(&t2.id).is_err(), "the ticket deleted came back");
    assert!(re.closed_projects().is_empty());
    assert!(file.exists());
    assert_eq!(saved_leftovers(&h), [p.id.as_str()]);

    // Let go of the file: the next start removes it, and the list forgets it.
    drop(stuck);
    let (again, _app) = restarted(&h);
    assert!(!file.exists());
    assert!(again.agent(&a2).is_err() && again.ticket(&t2.id).is_err());
    assert!(again.agent(&a1).is_ok() && again.ticket(&t1.id).is_ok());
    again.save_now();
    assert!(saved_leftovers(&h).is_empty());
}

#[tokio::test]
async fn closing_again_a_project_whose_file_stayed_keeps_its_new_history() {
    let h = harness("closed-leftover-again");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a1 = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    let file = h.core.data.closed_project_file(&p.id);
    let stuck = stick(&file);
    add_again(&h, &r).await;
    assert!(file.exists());
    // Let go of it, and the project is closed again with more in it: the file is written anew.
    drop(stuck);
    let a2 = agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    h.core.save_now();
    assert!(saved_leftovers(&h).is_empty());

    // The next start does not take the new history for a leftover.
    let (re, _app) = restarted(&h);

    assert!(re.project(&p.id).is_err());
    assert_eq!(listed_ids(&re), [p.id.as_str()]);
    let kept = read_closed(&h, &p.id);
    assert_eq!(kept["agents"].as_array().unwrap().len(), 2, "{kept}");
    assert!(re.agent(&a1).is_err() && re.agent(&a2).is_err());
}

#[tokio::test]
async fn a_history_erased_by_the_closing_whose_file_cannot_go_is_not_listed_again() {
    let h = harness("closed-leftover-erase");
    let (p, r) = h.project(false).await;
    agent_that_talked(&h, &p.id).await;
    close(&h, &p.id).await;
    let file = h.core.data.closed_project_file(&p.id);
    let stuck = stick(&file);
    add_again(&h, &r).await;

    h.core.remove_project(&p.id, true).await.unwrap();
    h.core.save_now();

    assert!(file.exists());
    assert!(h.core.closed_projects().is_empty());
    let (re, _app) = restarted(&h);
    assert!(
        re.closed_projects().is_empty(),
        "an erased history is listed again"
    );
    drop(stuck);
    let (again, _app) = restarted(&h);
    assert!(!file.exists());
    assert!(again.closed_projects().is_empty());
}

#[tokio::test]
async fn a_history_recovered_at_the_start_whose_file_cannot_be_removed_is_not_recovered_twice() {
    let h = harness("closed-leftover-recovered");
    let (p, _) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a1 = agent_that_talked(&h, &p.id).await;
    let a2 = agent_that_talked(&h, &p.id).await;
    h.core.save_now();
    // A save in the middle of the closing found the project with one of its agents only.
    let state_file = h.core.data.state_file();
    let mut state: Value = serde_json::from_slice(&std::fs::read(&state_file).unwrap()).unwrap();
    state["agents"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["id"] != a2.as_str());
    std::fs::write(&state_file, serde_json::to_vec(&state).unwrap()).unwrap();
    close(&h, &p.id).await;
    let file = h.core.data.closed_project_file(&p.id);
    let stuck = stick(&file);

    // The start gives the agent back, and cannot remove the file.
    let (re, _app) = restarted(&h);
    assert!(re.agent(&a1).is_ok() && re.agent(&a2).is_ok());
    assert!(file.exists());
    // It is deleted afterwards; the app is stopped and started again with the file still there.
    re.delete_agent(&a2, false).await.unwrap();
    re.save_now();
    let (again, _app) = restarted(&h);
    assert!(again.agent(&a1).is_ok());
    assert!(again.agent(&a2).is_err(), "the agent deleted came back");

    drop(stuck);
    let (last, _app) = restarted(&h);
    assert!(!file.exists());
    assert!(last.agent(&a2).is_err());
}

/// The ticket in « À tester », its validation under way (`step` set, as `approve_begin` does).
async fn being_approved(h: &Harness, project_id: &str, title: &str) -> Ticket {
    let t = h
        .core
        .ticket_create(project_id, draft(title))
        .await
        .unwrap();
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Review;
            t.step = Some("Merge…".into());
            Ok(())
        })
        .unwrap();
    h.core.ticket(&t.id).unwrap()
}

#[tokio::test]
async fn a_project_is_not_closed_while_a_validation_of_one_of_its_tickets_runs() {
    let h = harness("closed-validating");
    let (p, _) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a = agent_that_talked(&h, &p.id).await;
    let t = being_approved(&h, &p.id, "A valider").await;
    let refused = |erase: bool| {
        let (core, id) = (h.core.clone(), p.id.clone());
        async move {
            core.remove_project(&id, erase)
                .await
                .unwrap_err()
                .to_string()
        }
    };

    // With or without its history, and before anything is written or stopped.
    for erase in [false, true] {
        assert_eq!(
            refused(erase).await,
            format!(
                "La validation du ticket {} est en cours : attends sa fin pour fermer le projet.",
                t.key
            )
        );
    }
    h.core.settings.write().language = "en".into();
    assert_eq!(
        refused(false).await,
        format!(
            "The ticket {} is being approved: wait for the end of its approval to close the project.",
            t.key
        )
    );
    assert!(h.core.project(&p.id).is_ok() && h.alive(&a));
    assert!(closed_files(&h).is_empty());
    assert!(conversation_log(&h, &a).exists());
    assert_eq!(h.core.ticket(&t.id).unwrap().step, t.step);

    // Its validation over, the project closes, and a second closing is not held back by the first.
    h.core
        .edit_ticket(&t.id, |t| {
            t.step = None;
            Ok(())
        })
        .unwrap();
    h.core.remove_project(&p.id, false).await.unwrap();
    assert!(h.core.project(&p.id).is_err());
    assert_eq!(listed_ids(&h.core), [p.id.as_str()]);
}

#[tokio::test]
async fn a_validation_does_not_start_on_a_project_being_closed() {
    let h = harness("closed-no-validation");
    let (p, r) = h.project(false).await;
    quiet_board(&h, &p.id);
    let a = agent_that_talked(&h, &p.id).await;
    let t = h
        .core
        .ticket_create(&p.id, draft("A valider"))
        .await
        .unwrap();
    h.core
        .edit_ticket(&t.id, |t| {
            t.column = Column::Review;
            Ok(())
        })
        .unwrap();
    // A start is under way: the closing waits for it, its first history already written.
    let lock = h.core.spawn_lock(&a);
    let guard = lock.lock().await;
    let closing = tokio::spawn({
        let (core, id) = (h.core.clone(), p.id.clone());
        async move { core.remove_project(&id, false).await }
    });
    h.wait("the first history written", |h| {
        h.core.data.closed_project_file(&p.id).exists()
    })
    .await;

    // Neither « Valider » nor « L'agent résout » starts, in the window or detached (the MCP).
    let said =
        "Le projet de ce ticket est en cours de fermeture : la validation ne peut pas démarrer.";
    assert_eq!(
        h.core.ticket_approve(&t.id).await.unwrap_err().to_string(),
        said
    );
    assert_eq!(
        h.core
            .ticket_approve_detached(&t.id)
            .unwrap_err()
            .to_string(),
        said
    );
    assert_eq!(
        h.core
            .ticket_resolve_conflict(&t.id)
            .await
            .unwrap_err()
            .to_string(),
        said
    );
    assert_eq!(
        crate::tickets::Refusal::ProjectClosing.text(crate::i18n::Lang::En),
        "This ticket’s project is being closed: the approval can’t start."
    );
    // The ticket was not touched.
    let held = h.core.ticket(&t.id).unwrap();
    assert!(held.step.is_none() && held.blocked.is_none());
    drop(guard);
    closing.await.unwrap().unwrap();

    // The history has it as it was: « À tester », not being validated.
    let kept = read_closed(&h, &p.id);
    assert_eq!(kept["tickets"][0]["column"], json!("review"));
    assert!(kept["tickets"][0]["step"].is_null());
    let back = add_again(&h, &r).await;
    assert_eq!(h.core.ticket(&t.id).unwrap().column, Column::Review);
    assert_eq!(back.id, p.id);
}

// ---------- the tile's letters and logo (ESC-167) ----------

const LOGO_PNG: &[u8] = b"\x89PNG\r\n\x1a\n-logo";

/// A logo of the project chosen from a file of `dir`, which goes once copied.
fn logo_from(h: &Harness, id: &str, dir: &Path) -> Project {
    let source = dir.join("logo.png");
    std::fs::write(&source, LOGO_PNG).unwrap();
    let p = h
        .core
        .set_project_logo(id, Some(&source.to_string_lossy()))
        .unwrap();
    std::fs::remove_file(&source).unwrap();
    p
}

#[tokio::test]
async fn the_tiles_letters_and_logo_are_saved_and_come_back_with_the_project() {
    let h = harness("closed-tile");
    let (p, r) = h.project(false).await;
    // The letters are cut as the window cuts them; the logo the window sends is not taken.
    h.core
        .update_project(Project {
            tile_letters: " apiX ".into(),
            logo: "../../secret.png".into(),
            ..p.clone()
        })
        .unwrap();
    let saved = h.core.project(&p.id).unwrap();
    assert_eq!(saved.tile_letters, "api");
    assert!(saved.logo.is_empty());
    assert!(h.core.project_logo(&p.id).is_none());

    let with = logo_from(&h, &p.id, &h.dir);
    assert!(with.logo.starts_with("logo-") && with.logo.ends_with(".png"));
    assert_eq!(h.core.project(&p.id).unwrap().logo, with.logo);
    let url = h.core.project_logo(&p.id).unwrap();
    assert!(url.starts_with("data:image/png;base64,"), "{url}");
    // A file refused keeps the logo there is.
    let gif = h.dir.join("anim.gif");
    std::fs::write(&gif, b"GIF89a").unwrap();
    let e = h
        .core
        .set_project_logo(&p.id, Some(&gif.to_string_lossy()))
        .unwrap_err();
    assert!(e.to_string().contains("n’est pas une image"), "{e}");
    assert_eq!(h.core.project_logo(&p.id), Some(url.clone()));
    // Saved in the state, not in its `.escouade.yml`.
    h.core.save_state().unwrap();
    let state: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.state_file()).unwrap()).unwrap();
    let kept = state["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == p.id.as_str());
    assert_eq!(kept.unwrap()["tileLetters"], "api");
    assert_eq!(kept.unwrap()["logo"], with.logo.as_str());
    assert!(!r.join(".escouade.yml").exists());

    // Closed then added again: both come back, the copy readable.
    close(&h, &p.id).await;
    assert!(h.core.data.project_files(&p.id).join(&with.logo).exists());
    let back = add_again(&h, &r).await;
    assert_eq!(back.id, p.id);
    assert_eq!(back.tile_letters, "api");
    assert_eq!(back.logo, with.logo);
    assert_eq!(h.core.project_logo(&p.id), Some(url));

    // « Retirer »: no logo, its copy gone.
    let without = h.core.set_project_logo(&p.id, None).unwrap();
    assert!(without.logo.is_empty());
    assert!(h.core.project_logo(&p.id).is_none());
    assert!(!h.core.data.project_files(&p.id).join(&with.logo).exists());
    assert!(h.core.set_project_logo("inconnu", None).is_err());
}

#[tokio::test]
async fn a_project_gone_for_good_erases_its_logo() {
    let h = harness("closed-tile-erase");
    let (p, _) = h.project(false).await;
    let (q, r2) = {
        let dir = repo(&h.dir.join("b"));
        let q = h
            .core
            .create_project(&dir.to_string_lossy(), "autre", COLOR, false, None, true)
            .await
            .unwrap();
        (q, dir)
    };
    logo_from(&h, &p.id, &h.dir);
    logo_from(&h, &q.id, &r2);
    // Closed with its history erased: its copy goes, the other project's stays.
    h.core.remove_project(&p.id, true).await.unwrap();
    assert!(!h.core.data.project_files(&p.id).exists());
    assert!(h.core.project_logo(&q.id).is_some());
    // Closed with its history kept, then the history erased.
    close(&h, &q.id).await;
    assert!(h.core.data.project_files(&q.id).exists());
    h.core.erase_closed_project(&q.id).unwrap();
    assert!(!h.core.data.project_files(&q.id).exists());
}

#[test]
fn a_project_saved_before_the_tiles_settings_has_its_initials_and_no_logo() {
    let p: Project = serde_json::from_value(json!({
        "id": "p1", "name": "demo", "path": "C:/demo", "color": "red"
    }))
    .unwrap();
    assert!(p.tile_letters.is_empty() && p.logo.is_empty());
    let back = as_json(&p);
    assert_eq!(back["tileLetters"], "");
    assert_eq!(back["logo"], "");
}

// ---------- the label under the tile (ESC-168) ----------

#[tokio::test]
async fn the_tiles_label_is_saved_and_comes_back_with_the_project() {
    let h = harness("closed-label");
    let (p, r) = h.project(false).await;
    // On one line, trimmed: as the window cuts it.
    h.core
        .update_project(Project {
            tile_label: true,
            tile_label_text: "  Démo\n  API ".into(),
            ..p.clone()
        })
        .unwrap();
    let saved = h.core.project(&p.id).unwrap();
    assert!(saved.tile_label);
    assert_eq!(saved.tile_label_text, "Démo API");
    // In the state, not in its `.escouade.yml`.
    h.core.save_state().unwrap();
    let state: Value =
        serde_json::from_slice(&std::fs::read(h.core.data.state_file()).unwrap()).unwrap();
    let kept = state["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == p.id.as_str())
        .unwrap();
    assert_eq!(kept["tileLabel"], true);
    assert_eq!(kept["tileLabelText"], "Démo API");
    assert!(!r.join(".escouade.yml").exists());
    // Closed then added again: it comes back.
    close(&h, &p.id).await;
    let back = add_again(&h, &r).await;
    assert!(back.tile_label);
    assert_eq!(back.tile_label_text, "Démo API");
    // Off, its text kept for the wide rail.
    h.core
        .update_project(Project {
            tile_label: false,
            ..back
        })
        .unwrap();
    let off = h.core.project(&p.id).unwrap();
    assert!(!off.tile_label);
    assert_eq!(off.tile_label_text, "Démo API");
}

#[test]
fn a_project_saved_before_the_label_has_none() {
    let p: Project = serde_json::from_value(json!({
        "id": "p1", "name": "demo", "path": "C:/demo", "color": "red", "tileLetters": "DM"
    }))
    .unwrap();
    assert!(!p.tile_label && p.tile_label_text.is_empty());
    assert_eq!(p.tile_letters, "DM");
    let back = as_json(&p);
    assert_eq!(back["tileLabel"], false);
    assert_eq!(back["tileLabelText"], "");
}
