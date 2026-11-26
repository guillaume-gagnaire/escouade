//! TS-17: passing an agent, in the core, two data folders against the real `escouade-team-server` and a bare
//! repository as their host. Alice passes her ticket's agent, its turn running and its work uncommitted: refused out of
//! the project, for a personal project, and with a host that does not answer (nothing lost, the commit « en cours »
//! undone); passed as a patch, then cancelled (the claim back, the offer gone); passed again, Bob takes it: the claim's
//! token grew, the branch and the work are in his worktree, his new session's first message is the handover file
//! with the conversation of before above it. Bob then passes it, as a commit « en cours », to Alice's machine in
//! server mode, which takes it on its own.

use super::*;
use crate::core_tests::{git, harness, Harness};
use crate::integrations::secrets::MemoryStore;
use crate::model::Ticket;
use crate::teamboard::tests::{draft, read, server, teams};
use std::path::Path;

fn ticket(h: &Harness, id: &str) -> Ticket {
    h.core.ticket(id).unwrap()
}

fn token(t: &Ticket) -> Option<u64> {
    t.team.as_ref().and_then(|x| x.token)
}

/// The project of `h` on a clone of `origin` (a bare repository).
async fn project_on(h: &Harness, origin: &Path, name: &str) -> crate::model::Project {
    let clone = h.dir.join(name);
    git(
        &h.dir,
        &[
            "clone",
            "-q",
            &origin.to_string_lossy(),
            &clone.to_string_lossy(),
        ],
    );
    git(&clone, &["config", "user.email", "t@t"]);
    git(&clone, &["config", "user.name", "t"]);
    git(&clone, &["config", "core.autocrlf", "false"]);
    h.core
        .create_project(
            &clone.to_string_lossy(),
            "demo",
            "oklch(0.72 0.12 48)",
            true,
            None,
            true,
        )
        .await
        .unwrap()
}

/// The ticket's agent, once it runs its turn.
async fn agent_running(h: &Harness, id: &str) -> String {
    h.wait("the ticket's agent", |h| {
        ticket(h, id).agent_id.as_deref().is_some_and(|a| {
            h.core
                .agent(a)
                .is_ok_and(|x| x.lock().meta.status.is_active())
        })
    })
    .await;
    ticket(h, id).agent_id.unwrap()
}

/// The handover file sent to the agent that took the ticket, once it is in its conversation (the ones before came with
/// the conversation of before).
async fn handed_file(h: &Harness, id: &str) -> (String, String) {
    // After the last separator « Passé par … »: the conversation of before is above it.
    let after = |items: &[Value]| -> Option<String> {
        let sep = items
            .iter()
            .rposition(|i| i["kind"] == "handover" && i["by"].is_string())?;
        items[sep..]
            .iter()
            .find(|i| i["kind"] == "user" && i["origin"] == "handover")
            .and_then(|i| i["text"].as_str().map(str::to_string))
    };
    h.wait("the agent passed", |h| {
        ticket(h, id)
            .agent_id
            .as_deref()
            .is_some_and(|a| after(&h.items(a)).is_some())
    })
    .await;
    let agent = ticket(h, id).agent_id.unwrap();
    let text = after(&h.items(&agent)).unwrap();
    (agent, text)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_agent_passes_to_a_teammate_then_to_the_teams_machine() {
    let (server, host, task, dir) = server().await;
    let store = Arc::new(MemoryStore::default());
    let alice_teams = teams(&dir.path().join("a"), store.clone());
    let bob_teams = teams(&dir.path().join("b"), store.clone());
    let token_ = server.new_team_token().await.unwrap();
    let team = alice_teams
        .create(&host, &token_, "ACME", None)
        .await
        .unwrap()
        .team
        .id;
    let invite = alice_teams.invite(&team, "member", None).await.unwrap();
    bob_teams.join(&invite.url, "").await.unwrap();
    crate::teams_tests::let_in(&alice_teams, &bob_teams, &team).await;

    let ha = harness("handoff-alice");
    let hb = harness("handoff-bob");
    ha.core.settings.write().user_name = "Alice Martin".into();
    hb.core.settings.write().user_name = "Bob Durand".into();
    ha.core.settings.write().mcp_enabled = true;
    hb.core.settings.write().mcp_enabled = true;
    // Their host: a bare repository both clone.
    let origin = ha.dir.join("origin.git");
    git(
        &ha.dir,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            &origin.to_string_lossy(),
        ],
    );
    let seed = crate::core_tests::repo(&ha.dir);
    git(&seed, &["push", "-q", &origin.to_string_lossy(), "main"]);
    let pa = project_on(&ha, &origin, "alice").await;
    let pb = project_on(&hb, &origin, "bob").await;
    ha.core.set_teams(alice_teams.clone());
    hb.core.set_teams(bob_teams.clone());
    ha.core
        .set_project_team(&pa.id, Some(team.clone()))
        .unwrap();
    hb.core
        .set_project_team(&pb.id, Some(team.clone()))
        .unwrap();
    let linked = ha.core.link_team_board(&pa.id, None).await.unwrap();
    hb.core
        .link_team_board(&pb.id, linked.team_project.clone())
        .await
        .unwrap();

    // Alice's ticket runs, its turn long ([lent]); she leaves work uncommitted in its worktree.
    let mut d = draft("Écrire la passation", false);
    d.description = "[lent]".into();
    let t = ha.core.ticket_create(&pa.id, d).await.unwrap();
    read(&ha, &pa.id).await;
    ha.core.ticket_start(&t.id).unwrap();
    let agent = agent_running(&ha, &t.id).await;
    let first_token = token(&ticket(&ha, &t.id)).expect("Alice holds it");
    let wt = std::path::PathBuf::from(ha.agent(&agent).cwd);
    std::fs::write(wt.join("passation.txt"), "le travail en cours\n").unwrap();
    read(&hb, &pb.id).await;
    read(&ha, &pa.id).await;

    // What the window shows: the branch and its remote, the work, Bob.
    let plan = ha.core.team_handoff_plan(&t.id).await.unwrap();
    assert_eq!(plan.refusal, None);
    assert!(plan.running, "its turn runs: it is interrupted first");
    assert_eq!(plan.remote, "origin");
    assert!(plan.branch.starts_with("ticket/"), "{}", plan.branch);
    assert!(plan.dirty.iter().any(|l| l.contains("passation.txt")));
    let bob = plan
        .targets
        .iter()
        .find(|x| x.name == "Bob Durand")
        .cloned()
        .expect("Bob is of the project");
    assert!(!bob.machine);

    // Refused: a device out of the project; an agent of a personal project. Nothing changed.
    let out = ha
        .core
        .team_handoff(&t.id, &"ab".repeat(16), Wip::Patch)
        .await
        .unwrap_err();
    assert!(out.to_string().contains("pas du projet"), "{out}");
    let personal = project_on(&ha, &origin, "perso").await;
    let mut pd = draft("Perso", true);
    pd.team_local = None;
    let own = ha.core.ticket_create(&personal.id, pd).await.unwrap();
    let plan_own = ha.core.team_handoff_plan(&own.id).await.unwrap();
    assert!(
        plan_own
            .refusal
            .as_deref()
            .unwrap_or_default()
            .contains("projet personnel"),
        "{:?}",
        plan_own.refusal
    );
    // A host that does not answer: nothing passes, the commit « en cours » is undone, the work is there.
    let head = git(&wt, &["rev-parse", "HEAD"]);
    git(
        &wt,
        &[
            "remote",
            "set-url",
            "origin",
            &ha.dir.join("nowhere.git").to_string_lossy(),
        ],
    );
    let out = ha
        .core
        .team_handoff(&t.id, &bob.device, Wip::Commit)
        .await
        .unwrap_err();
    assert!(out.to_string().contains("ne répond pas"), "{out}");
    git(
        &wt,
        &["remote", "set-url", "origin", &origin.to_string_lossy()],
    );
    assert_eq!(git(&wt, &["rev-parse", "HEAD"]), head);
    assert!(git(&wt, &["status", "--short"]).contains("passation.txt"));
    let still = ticket(&ha, &t.id);
    assert_eq!(token(&still), Some(first_token));
    assert!(still.team.as_ref().unwrap().handoff.is_none());

    // Passed as a patch: the claim is Bob's under a new token, the branch on the host, the work still here.
    ha.core
        .team_handoff(&t.id, &bob.device, Wip::Patch)
        .await
        .unwrap();
    let passed = ticket(&ha, &t.id);
    assert_eq!(token(&passed), None);
    let hd = passed.team.as_ref().unwrap().handoff.clone().unwrap();
    assert!(hd.outgoing && hd.patch);
    assert_eq!(hd.to_name, "Bob Durand");
    assert_eq!(passed.agent_id, None);
    assert!(
        !ha.agent(&agent).status.is_active(),
        "stopped between two turns"
    );
    assert!(git(&wt, &["status", "--short"]).contains("passation.txt"));
    assert!(!git(&origin, &["branch", "--list", &plan.branch]).is_empty());
    read(&hb, &pb.id).await;
    let theirs = crate::teamboard::tests::find(&hb, &pb.id, "Écrire la passation").unwrap();
    let offer = theirs
        .team
        .as_ref()
        .unwrap()
        .handoff
        .clone()
        .expect("offered to Bob");
    assert!(offer.incoming);
    assert_eq!(offer.from_name, "Alice Martin");
    // In Bob's box « À toi », by the box's own path.
    let views = hb.core.inbox_views(None).await;
    let v = views
        .views
        .iter()
        .find(|v| v.key == format!("handoff:{}", theirs.id))
        .expect("in the box");
    assert_eq!(v.by.as_deref(), Some("Alice Martin"));

    // Cancelled: the claim is Alice's again, under a new token; the offer is gone from Bob's box.
    ha.core.team_handoff_cancel(&t.id).await.unwrap();
    let back = ticket(&ha, &t.id);
    assert!(token(&back).unwrap() > first_token);
    assert_eq!(back.agent_id.as_deref(), Some(agent.as_str()));
    read(&hb, &pb.id).await;
    assert!(ticket(&hb, &theirs.id).team.unwrap().handoff.is_none());
    assert!(hb
        .core
        .inbox_views(None)
        .await
        .views
        .iter()
        .all(|v| v.source != crate::inboxviews::ViewSource::Handoff));

    // Passed again; Bob takes it: his token is above every one before, his worktree has the branch and the work.
    ha.core
        .team_handoff(&t.id, &bob.device, Wip::Patch)
        .await
        .unwrap();
    let deposit = ticket(&ha, &t.id).team.unwrap().handoff.unwrap().file;
    let deposit = crate::teamboard::object_of(&deposit).expect("the deposit's id");
    let (_, _, pid) = ha
        .core
        .team_project_of(&ha.core.project(&pa.id).unwrap())
        .unwrap();
    read(&hb, &pb.id).await;
    let choice = hb.core.team_handoff_engines(&theirs.id).unwrap();
    assert_eq!(choice.preferred, Some(crate::model::EngineId::CLAUDE));
    hb.core
        .inbox_view_act(
            &format!("handoff:{}", theirs.id),
            crate::inboxviews::ViewAct::Take { engine: None },
        )
        .await
        .unwrap();
    let taken = ticket(&hb, &theirs.id);
    assert!(
        token(&taken).unwrap() > token(&back).unwrap() + 1,
        "the token grows"
    );
    assert!(taken.team.as_ref().unwrap().handoff.is_none());
    let (bob_agent, file) = handed_file(&hb, &theirs.id).await;
    assert!(file.contains("Alice Martin"), "{file}");
    assert!(file.contains("Passation"), "{file}");
    let bob_wt = std::path::PathBuf::from(hb.agent(&bob_agent).cwd);
    assert_eq!(
        std::fs::read_to_string(bob_wt.join("passation.txt")).unwrap(),
        "le travail en cours\n"
    );
    assert_eq!(git(&bob_wt, &["branch", "--show-current"]), plan.branch);
    // Its account is Bob's own; the conversation of before is above the separator.
    let items = hb.items(&bob_agent);
    let sep = items
        .iter()
        .position(|i| i["kind"] == "handover" && i["by"] == "Alice Martin")
        .expect("the separator");
    assert!(items[..sep].iter().any(|i| i["kind"] == "user"));
    // The deposit is Alice's (SR2-03): Bob, who took it, cannot erase it; her machine does once it reads the offer gone.
    let at_bob = bob_teams.team(&team).unwrap();
    assert!(at_bob.lock().await.delete_file(pid, deposit).await.is_err());
    assert!(at_bob
        .lock()
        .await
        .get_file(pid, deposit)
        .await
        .unwrap()
        .is_some());
    read(&ha, &pa.id).await;
    let at_alice = ticket(&ha, &t.id);
    let holder = at_alice.team.as_ref().unwrap().holder.clone().unwrap();
    assert_eq!(holder.name, "Bob Durand");
    assert!(at_alice.team.as_ref().unwrap().handoff.is_none());
    assert_eq!(
        at_bob.lock().await.get_file(pid, deposit).await.unwrap(),
        None
    );

    // Alice's machine in server mode: Bob passes it there, as a commit « en cours »; it takes it on its own.
    ha.core.set_team_machine(true);
    read(&ha, &pa.id).await;
    read(&hb, &pb.id).await;
    std::fs::write(bob_wt.join("suite.txt"), "la suite\n").unwrap();
    let plan_b = hb.core.team_handoff_plan(&theirs.id).await.unwrap();
    let machine = plan_b
        .targets
        .iter()
        .find(|x| x.machine)
        .cloned()
        .expect("the team's machine");
    hb.core
        .team_handoff(&theirs.id, &machine.device, Wip::Commit)
        .await
        .unwrap();
    assert!(git(&origin, &["log", "-1", "--format=%s", &plan.branch]).starts_with("wip:"));
    // Bob is a member: the machine does not take on its own what he passes (SR3-05); it waits for a human of the
    // machine, in its box « À toi ».
    read(&ha, &pa.id).await;
    let refused = ha.core.team_handoff_take_alone(&t.id).await;
    let why = refused.expect_err("not taken on its own").to_string();
    assert!(why.contains("humain"), "{why}");
    let waiting = ticket(&ha, &t.id);
    let hd = waiting
        .team
        .as_ref()
        .unwrap()
        .handoff
        .clone()
        .expect("it waits");
    assert!(hd.incoming && hd.machine);
    assert!(ha
        .core
        .inbox_views(None)
        .await
        .views
        .iter()
        .any(|v| v.key == format!("handoff:{}", t.id)));
    // A rule an administrator wrote in the team's file lets it take a member's on its own.
    let rule = escouade_team::settings::Effective {
        enforced: [(MACHINE_TAKES.to_string(), serde_json::json!("members"))].into(),
        ..Default::default()
    };
    ha.core.set_team_machine_takes(&team, Some(&rule));
    assert_eq!(ha.core.team_machine_takes(&team), MachineTakes::Members);
    read(&ha, &pa.id).await;
    let (machine_agent, file) = handed_file(&ha, &t.id).await;
    assert_ne!(machine_agent, agent);
    assert!(file.contains("Bob Durand"), "{file}");
    let m_wt = std::path::PathBuf::from(ha.agent(&machine_agent).cwd);
    assert!(m_wt.join("suite.txt").is_file());
    assert!(token(&ticket(&ha, &t.id)).is_some());

    drop(ha);
    drop(hb);
    task.abort();
}

/// SR3-05: whose passages a machine takes on its own, as the team's file says (an administrator's without it).
#[test]
fn the_machine_takes_on_its_own_what_the_rule_lets_it() {
    let of = |v: Option<&str>| {
        let mut e = escouade_team::settings::Effective::default();
        if let Some(v) = v {
            e.defaults
                .insert(MACHINE_TAKES.into(), serde_json::json!(v));
        }
        MachineTakes::of(&e)
    };
    assert_eq!(of(None), MachineTakes::Admins);
    assert_eq!(of(Some("anything")), MachineTakes::Admins);
    assert_eq!(of(Some("members")), MachineTakes::Members);
    assert_eq!(of(Some("nobody")), MachineTakes::Nobody);
    // Imposed wins over recommended.
    let mut e = escouade_team::settings::Effective::default();
    e.defaults
        .insert(MACHINE_TAKES.into(), serde_json::json!("members"));
    e.enforced
        .insert(MACHINE_TAKES.into(), serde_json::json!("nobody"));
    assert_eq!(MachineTakes::of(&e), MachineTakes::Nobody);
    assert!(MachineTakes::Admins.takes(true) && !MachineTakes::Admins.takes(false));
    assert!(MachineTakes::Members.takes(false));
    assert!(!MachineTakes::Nobody.takes(true));
}

/// SR3-06: the remote is one of the repository's, never an address the offer gives.
#[test]
fn a_passage_is_fetched_from_a_remote_of_the_repository_only() {
    let remotes = vec!["origin".to_string(), "upstream".to_string()];
    assert_eq!(
        remote_for(&remotes, "upstream").as_deref(),
        Some("upstream")
    );
    assert_eq!(
        remote_for(&remotes, "--upload-pack=touch /tmp/x").as_deref(),
        Some("origin")
    );
    assert_eq!(
        remote_for(&remotes, "ssh://evil.example/x").as_deref(),
        Some("origin")
    );
    assert_eq!(
        remote_for(&[], "ssh://evil.example/x"),
        None,
        "no remote: refused"
    );
}

/// SR3-06: a passage's branch never crushes a local branch other than the ticket's, and is a branch's name.
#[tokio::test]
async fn a_passage_does_not_crush_a_local_branch() {
    let h = harness("handoff-branch");
    let repo = h.dir.join("repo-branch");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "user.email", "t@t"]);
    git(&repo, &["config", "user.name", "t"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "x"]);
    git(&repo, &["branch", "release/2.3"]);
    git(&repo, &["branch", "ticket/dem-41"]);
    let root = repo.to_string_lossy().to_string();
    let why = passed_branch_ok(&root, "release/2.3", "ticket/dem-41")
        .await
        .unwrap_err()
        .to_string();
    assert!(why.contains("release/2.3"), "{why}");
    assert!(passed_branch_ok(&root, "main", "").await.is_err());
    // The ticket's own branch, or one this machine does not have: fetched.
    assert!(passed_branch_ok(&root, "ticket/dem-41", "ticket/dem-41")
        .await
        .is_ok());
    assert!(passed_branch_ok(&root, "ticket/dem-42", "").await.is_ok());
    // Not a branch's name: refused before git reads it.
    for bad in ["-x", "--upload-pack=x", "a..b", "@{-1}", ""] {
        assert!(passed_branch_ok(&root, bad, "").await.is_err(), "{bad}");
    }
}

/// TS-17b: who reaches the native resume. Claude Code on both sides only; any other harness, on either side, goes on
/// from the handover file, its session never copied.
#[test]
fn only_claude_code_on_both_sides_resumes_its_session_natively() {
    use crate::model::EngineId;
    assert!(resumes_natively(EngineId::CLAUDE, EngineId::CLAUDE));
    for other in ["codex", "copilot", "cursor", "opencode", "gemini"] {
        let e = EngineId::new(other);
        assert!(!resumes_natively(EngineId::CLAUDE, e), "{other}");
        assert!(!resumes_natively(e, EngineId::CLAUDE), "{other}");
        assert!(!resumes_natively(e, e), "{other}");
    }
    use crate::board::TurnEnd;
    assert!(native_failed(&TurnEnd::Error(
        "No conversation found".into()
    )));
    assert!(native_failed(&TurnEnd::Refused));
    for fine in [
        TurnEnd::Finished("ok".into()),
        TurnEnd::Interrupted,
        TurnEnd::Limited,
        TurnEnd::SignIn,
    ] {
        assert!(!native_failed(&fine), "{fine:?}");
    }
}

/// Each one's Claude Code on an account of its own folder (`CLAUDE_CONFIG_DIR`), where the fake `claude` keeps its
/// sessions as Claude Code does: the only account of the harness.
fn own_account(h: &Harness, id: &str) -> std::path::PathBuf {
    let dir = h.dir.join(format!("claude-{id}"));
    let account = crate::model::Account {
        id: id.into(),
        name: id.into(),
        config_dir: dir.to_string_lossy().into(),
        ..Default::default()
    };
    let mut s = h.core.settings.read().clone();
    s.accounts = vec![account];
    h.core.save_settings(s).unwrap();
    dir
}

/// The files of `projects/*/<session>.jsonl` in the account's folder `config`.
fn sessions_named(config: &Path, session: &str) -> Vec<std::path::PathBuf> {
    let Ok(folders) = std::fs::read_dir(config.join("projects")) else {
        return Vec::new();
    };
    folders
        .flatten()
        .map(|f| f.path().join(format!("{session}.jsonl")))
        .filter(|f| f.is_file())
        .collect()
}

/// TS-17b (C16-B): Alice's agent of Claude Code passed between two turns takes its session with it; Bob, on Claude
/// Code, resumes it natively: the copy in his account's folder under his worktree, its paths rewritten, the agent started
/// by `--resume <id> --fork-session` (a new session of its own), the separator « Reprise de la session », and the
/// deposit erased from the server. A second ticket whose first turn fails at Bob's (an error of the API) goes on from
/// the handover file, the copy taken out of his account's folder.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_agent_of_claude_code_passed_resumes_its_own_session_else_the_handover_file() {
    let (server, host, task, dir) = server().await;
    let store = Arc::new(MemoryStore::default());
    let alice_teams = teams(&dir.path().join("a"), store.clone());
    let bob_teams = teams(&dir.path().join("b"), store.clone());
    let token_ = server.new_team_token().await.unwrap();
    let team = alice_teams
        .create(&host, &token_, "ACME", None)
        .await
        .unwrap()
        .team
        .id;
    let invite = alice_teams.invite(&team, "member", None).await.unwrap();
    bob_teams.join(&invite.url, "").await.unwrap();
    crate::teams_tests::let_in(&alice_teams, &bob_teams, &team).await;

    let ha = harness("handoff-native-alice");
    let hb = harness("handoff-native-bob");
    ha.core.settings.write().user_name = "Alice Martin".into();
    hb.core.settings.write().user_name = "Bob Durand".into();
    let alice_claude = own_account(&ha, "alice");
    let bob_claude = own_account(&hb, "bob");
    let origin = ha.dir.join("origin.git");
    git(
        &ha.dir,
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            &origin.to_string_lossy(),
        ],
    );
    let seed = crate::core_tests::repo(&ha.dir);
    git(&seed, &["push", "-q", &origin.to_string_lossy(), "main"]);
    let pa = project_on(&ha, &origin, "alice").await;
    let pb = project_on(&hb, &origin, "bob").await;
    ha.core.set_teams(alice_teams.clone());
    hb.core.set_teams(bob_teams.clone());
    ha.core
        .set_project_team(&pa.id, Some(team.clone()))
        .unwrap();
    hb.core
        .set_project_team(&pb.id, Some(team.clone()))
        .unwrap();
    let linked = ha.core.link_team_board(&pa.id, None).await.unwrap();
    hb.core
        .link_team_board(&pb.id, linked.team_project.clone())
        .await
        .unwrap();

    // Alice's ticket runs on Claude Code; its session is in her account's folder.
    let pass = |title: &'static str| {
        let (ha, hb, pa, pb) = (&ha, &hb, pa.clone(), pb.clone());
        let alice_claude = alice_claude.clone();
        async move {
            let mut d = draft(title, false);
            d.description = "[lent]".into();
            let t = ha.core.ticket_create(&pa.id, d).await.unwrap();
            read(ha, &pa.id).await;
            ha.core.ticket_start(&t.id).unwrap();
            let agent = agent_running(ha, &t.id).await;
            let session = {
                ha.wait("its session", |h| h.agent(&agent).session_id.is_some())
                    .await;
                ha.agent(&agent).session_id.unwrap()
            };
            ha.wait("its session kept", |_| {
                !sessions_named(&alice_claude, &session).is_empty()
            })
            .await;
            read(hb, &pb.id).await;
            read(ha, &pa.id).await;
            let plan = ha.core.team_handoff_plan(&t.id).await.unwrap();
            assert!(plan.session, "the window says its session goes with it");
            let bob = plan
                .targets
                .iter()
                .find(|x| x.name == "Bob Durand")
                .cloned()
                .unwrap();
            ha.core
                .team_handoff(&t.id, &bob.device, Wip::Patch)
                .await
                .unwrap();
            read(hb, &pb.id).await;
            let theirs = crate::teamboard::tests::find(hb, &pb.id, title).unwrap();
            let wt = ha.agent(&agent).cwd;
            (t, theirs, session, wt)
        }
    };

    let (_t, theirs, session, alice_wt) = pass("La reprise native").await;
    // The offer says its deposit holds the session; the deposit is on the server until it is taken.
    let pb_now = hb.core.project(&pb.id).unwrap();
    let (teams_b, team_id, pid) = hb.core.team_project_of(&pb_now).unwrap();
    let shared = teams_b.team(&team_id).unwrap();
    let (offer, file) = {
        let mut t = shared.lock().await;
        let (_, offer) = t.handoffs(pid).unwrap().into_iter().next().unwrap();
        let file = crate::teamboard::object_of(&offer.file).unwrap();
        assert!(t.get_file(pid, file).await.unwrap().is_some());
        (offer, file)
    };
    assert!(offer.session);
    hb.core.team_handoff_take(&theirs.id, None).await.unwrap();
    // Bob's agent: the copy in his account's folder, under his worktree, nothing of Alice's worktree left.
    let bob_agent = ticket(&hb, &theirs.id).agent_id.unwrap();
    let bob_wt = hb.agent(&bob_agent).cwd;
    let copy = crate::sessioncopy::session_path(&bob_claude, &bob_wt, &session);
    assert!(copy.is_file(), "{}", copy.display());
    let placed = std::fs::read_to_string(&copy).unwrap();
    assert!(
        placed.contains(&serde_json::to_string(&bob_wt).unwrap()),
        "{placed}"
    );
    crate::sessioncopy::verify(&placed, &alice_wt, &bob_wt).unwrap();
    // Started by `--resume <id> --fork-session`: a session of its own, never the one passed.
    hb.wait("the resumed turn", |h| {
        h.agent(&bob_agent)
            .session_id
            .as_deref()
            .is_some_and(|s| s != session)
    })
    .await;
    let launches = hb.launch_log(Path::new(&bob_wt));
    assert!(
        launches.iter().any(|l| {
            let argv: Vec<&str> = l["argv"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            argv.windows(2)
                .any(|w| w[0] == format!("--resume={session}") && w[1] == "--fork-session")
        }),
        "{launches:?}"
    );
    let items = hb.items(&bob_agent);
    let sep = items
        .iter()
        .position(|i| i["kind"] == "handover" && i["native"] == true)
        .expect("« Reprise de la session »");
    assert_eq!(items[sep]["by"], "Alice Martin");
    assert!(
        items[..sep].iter().any(|i| i["kind"] == "user"),
        "the conversation of before above"
    );
    assert!(items[sep..]
        .iter()
        .any(|i| i["kind"] == "user" && i["origin"] == "resume"));
    assert!(
        !items[sep..]
            .iter()
            .any(|i| i["kind"] == "user" && i["origin"] == "handover"),
        "no handover file"
    );
    // The first turn went well: it stays on its session.
    hb.wait("the first turn's end", |h| {
        !h.agent(&bob_agent).status.is_active()
    })
    .await;
    assert!(hb.core.native_resumes.lock().get(&bob_agent).is_none());
    assert!(copy.is_file());
    // Erased from the server once taken, by Alice's machine, its author (SR2-03), when it reads the offer gone.
    read(&ha, &pa.id).await;
    {
        let mut t = shared.lock().await;
        assert_eq!(t.get_file(pid, file).await.unwrap(), None);
        assert!(t.handoffs(pid).unwrap().is_empty());
    }

    // The second: its first turn fails at Bob's (an error of the API, once): the handover file, the copy gone.
    let (_t2, theirs2, session2, _) = pass("Le repli").await;
    std::fs::write(bob_claude.join("fake-error"), "1").unwrap();
    hb.core.team_handoff_take(&theirs2.id, None).await.unwrap();
    let bob_agent2 = ticket(&hb, &theirs2.id).agent_id.unwrap();
    let bob_wt2 = hb.agent(&bob_agent2).cwd;
    let copy2 = crate::sessioncopy::session_path(&bob_claude, &bob_wt2, &session2);
    let (agent2, text) = handed_file(&hb, &theirs2.id).await;
    assert_eq!(agent2, bob_agent2);
    assert!(text.contains("Passation"), "{text}");
    assert!(!copy2.exists(), "the copy out of the account's folder");
    let items = hb.items(&bob_agent2);
    let native = items
        .iter()
        .position(|i| i["kind"] == "handover" && i["native"] == true)
        .expect("resumed first");
    let back = items
        .iter()
        .rposition(|i| {
            i["kind"] == "handover" && i["native"].is_null() && i["by"] == "Alice Martin"
        })
        .expect("then the passation");
    assert!(back > native);
    assert!(items[native..back].iter().any(|i| i["kind"] == "notice"
        && i["text"]
            .as_str()
            .is_some_and(|t| t.contains("ne s’est pas reprise"))));
    // Its handover turn runs on a new session, not the copy's.
    hb.wait("the handover turn", |h| {
        !h.agent(&bob_agent2).status.is_active() && h.agent(&bob_agent2).session_id.is_some()
    })
    .await;
    assert_ne!(
        hb.agent(&bob_agent2).session_id.as_deref(),
        Some(session2.as_str())
    );
    assert!(hb.agent(&bob_agent2).fork_of.is_none());

    drop(ha);
    drop(hb);
    task.abort();
}
