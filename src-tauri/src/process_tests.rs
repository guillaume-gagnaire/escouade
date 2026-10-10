//! Integration tests: the real process driver and agent runtime against a fake `claude`
//! (tests/fixtures/fake-claude.mjs, run with node).

use crate::agent::{AgentHandle, AgentRt, Effects};
use crate::board::TurnEnd;
use crate::claude::{ClaudeProcess, SpawnOpts};
use crate::core::claude_args;
use crate::model::{AgentMeta, AgentStatus, RateWindow};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("fake-claude.mjs")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    dir
}

fn new_agent(dir: &std::path::Path) -> AgentHandle {
    let meta = AgentMeta {
        id: dir.file_name().unwrap().to_string_lossy().to_string(),
        project_id: "p".into(),
        name: "test".into(),
        model: "sonnet".into(),
        effort: "medium".into(),
        mode: "auto".into(),
        cwd: dir.to_string_lossy().to_string(),
        ..Default::default()
    };
    Arc::new(parking_lot::Mutex::new(AgentRt::new(
        meta,
        &dir.join("conversations"),
    )))
}

/// Spawns the fake CLI for `h`, wiring frames and exit into the runtime like Core does.
fn spawn(h: &AgentHandle, log: &std::path::Path) -> Arc<ClaudeProcess> {
    spawn_recording(h, log, Arc::default())
}

/// Like `spawn`, noting how each turn ends in `ends` (what Core hands to the board).
fn spawn_recording(
    h: &AgentHandle,
    log: &std::path::Path,
    ends: Arc<parking_lot::Mutex<Vec<TurnEnd>>>,
) -> Arc<ClaudeProcess> {
    spawn_with(h, log, None, ends, Arc::default())
}

/// What a launch of the fake CLI saw: its turns' ends, the quota windows it told.
type Rates = Arc<parking_lot::Mutex<Vec<(Option<RateWindow>, Option<RateWindow>)>>>;

/// Like `spawn_recording`, with `CLAUDE_CONFIG_DIR` set to `config_dir` when given (an account
/// of its own), and the quota windows it tells noted in `rates`.
fn spawn_with(
    h: &AgentHandle,
    log: &std::path::Path,
    config_dir: Option<&std::path::Path>,
    ends: Arc<parking_lot::Mutex<Vec<TurnEnd>>>,
    rates: Rates,
) -> Arc<ClaudeProcess> {
    let (opts, gen) = {
        let mut rt = h.lock();
        rt.gen += 1;
        let mut args = vec![fixture().to_string_lossy().to_string()];
        args.extend(claude_args(&rt.meta));
        let mut env = vec![("FAKE_CLAUDE_LOG".into(), log.to_string_lossy().to_string())];
        if let Some(dir) = config_dir {
            env.push((
                "CLAUDE_CONFIG_DIR".into(),
                dir.to_string_lossy().to_string(),
            ));
        }
        let opts = SpawnOpts {
            program: PathBuf::from("node"),
            cwd: rt.meta.cwd.clone(),
            args,
            env,
        };
        (opts, rt.gen)
    };
    let (h1, h2) = (h.clone(), h.clone());
    let (ends1, ends2) = (ends.clone(), ends);
    let proc = ClaudeProcess::spawn(
        opts,
        move |frame| {
            let mut rt = h1.lock();
            if rt.gen == gen {
                let mut fx = Effects::default();
                rt.handle_frame(&frame, &mut fx);
                ends1.lock().extend(fx.turn_end);
                rates.lock().extend(fx.rate);
            }
        },
        move |code, stderr| {
            let mut fx = Effects::default();
            h2.lock().on_exit(gen, code, &stderr, &mut fx);
            ends2.lock().extend(fx.turn_end);
        },
    )
    .expect("node must be installed to run the process tests");
    h.lock().attach(proc.clone());
    proc
}

fn send(h: &AgentHandle, proc: &ClaudeProcess, text: &str) {
    let uid = uuid::Uuid::new_v4().to_string();
    proc.send(&json!({ "type": "user", "message": { "role": "user", "content": text }, "parent_tool_use_id": null, "uuid": uid }))
        .unwrap();
    h.lock()
        .push_user(&uid, text, 0, &[], &mut Effects::default());
}

async fn wait_for(h: &AgentHandle, what: &str, pred: impl Fn(&mut AgentRt) -> bool) {
    for _ in 0..500 {
        if pred(&mut h.lock()) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let rt = h.lock();
    panic!(
        "timed out waiting for {what}; status = {:?}",
        rt.meta.status
    );
}

fn items(h: &AgentHandle) -> Vec<Value> {
    h.lock().conv.items()
}

fn launches(log: &std::path::Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["argv"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap().to_string())
                .collect()
        })
        .collect()
}

#[tokio::test]
async fn streams_a_reply_and_records_usage() {
    let dir = temp_dir("reply");
    let h = new_agent(&dir);
    let proc = spawn(&h, &dir.join("log.jsonl"));
    let init = proc
        .control(json!({ "subtype": "initialize" }), Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(init["commands"][0]["name"], "compact");

    send(&h, &proc, "Bonjour");
    wait_for(&h, "turn end", |rt| rt.meta.status == AgentStatus::Done).await;

    let all = items(&h);
    let text = all.iter().find(|i| i["kind"] == "text").unwrap();
    assert_eq!(text["text"], "Bonjour, tu as dit : Bonjour");
    assert_eq!(text["streaming"], false);
    let turn = all.iter().find(|i| i["kind"] == "turn").unwrap();
    assert_eq!(turn["tokens"], 1120);
    let rt = h.lock();
    assert_eq!(rt.meta.tokens, 1120);
    assert!((rt.meta.cost - 0.05).abs() < 1e-9);
    assert!(rt.meta.session_id.as_deref().unwrap().starts_with("sess-"));
}

#[tokio::test]
async fn idle_stop_keeps_the_conversation_and_the_next_message_resumes_the_session() {
    let dir = temp_dir("idle");
    let log = dir.join("log.jsonl");
    let h = new_agent(&dir);
    let proc = spawn(&h, &log);
    send(&h, &proc, "Premier");
    wait_for(&h, "first turn", |rt| rt.meta.status == AgentStatus::Done).await;
    let session = h.lock().meta.session_id.clone().unwrap();

    // Idle stop: the process is detached at once, so a message sent right after never
    // reaches the closing process.
    let old = h.lock().detach().expect("a running process");
    old.close_input();
    assert!(h.lock().proc.is_none());

    let proc = spawn(&h, &log);
    send(&h, &proc, "Second");
    wait_for(&h, "second turn", |rt| {
        rt.meta.status == AgentStatus::Done
            && rt
                .conv
                .items()
                .iter()
                .filter(|i| i["kind"] == "turn")
                .count()
                == 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(300)).await; // let the old process exit

    let rt = h.lock();
    assert_eq!(
        rt.meta.status,
        AgentStatus::Done,
        "the old process exit must not flag an error"
    );
    assert_eq!(rt.meta.session_id.as_deref(), Some(session.as_str()));
    drop(rt);
    let texts: Vec<String> = items(&h)
        .iter()
        .filter(|i| i["kind"] == "text")
        .map(|i| i["text"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        texts,
        vec![
            "Bonjour, tu as dit : Premier",
            "Bonjour, tu as dit : Second"
        ]
    );
    let runs = launches(&log);
    assert_eq!(runs.len(), 2);
    assert!(!runs[0].iter().any(|a| a.starts_with("--resume")));
    assert!(runs[1].contains(&format!("--resume={session}")));
}

#[tokio::test]
async fn answering_a_question_resumes_the_turn() {
    let dir = temp_dir("question");
    let h = new_agent(&dir);
    let proc = spawn(&h, &dir.join("log.jsonl"));
    send(&h, &proc, "question");
    wait_for(&h, "question", |rt| rt.meta.status == AgentStatus::Waiting).await;
    assert_eq!(h.lock().view().pending, vec!["req_question".to_string()]);

    h.lock()
        .answer_question(
            "req_question",
            json!({ "Quelle base de données ?": "SQLite" }),
            &mut Effects::default(),
        )
        .unwrap();
    wait_for(&h, "turn end", |rt| rt.meta.status == AgentStatus::Done).await;

    let all = items(&h);
    let q = all.iter().find(|i| i["kind"] == "question").unwrap();
    assert_eq!(q["answers"]["Quelle base de données ?"], "SQLite");
    assert!(all
        .iter()
        .any(|i| i["kind"] == "text" && i["text"] == "Choix retenu : SQLite"));
    assert!(h.lock().view().pending.is_empty());
}

#[tokio::test]
async fn permission_always_forwards_the_suggested_rules_and_deny_forwards_the_reason() {
    let dir = temp_dir("perm");
    let h = new_agent(&dir);
    let proc = spawn(&h, &dir.join("log.jsonl"));

    send(&h, &proc, "permission");
    wait_for(&h, "permission", |rt| {
        rt.meta.status == AgentStatus::Waiting
    })
    .await;
    h.lock()
        .answer_permission("req_perm", "always", None, &mut Effects::default())
        .unwrap();
    wait_for(&h, "turn end", |rt| rt.meta.status == AgentStatus::Done).await;
    assert!(items(&h)
        .iter()
        .any(|i| i["text"] == "Commande exécutée (règle enregistrée)."));

    send(&h, &proc, "permission encore");
    wait_for(&h, "permission 2", |rt| {
        rt.meta.status == AgentStatus::Waiting
    })
    .await;
    h.lock()
        .answer_permission(
            "req_perm",
            "deny",
            Some("utilise npm run clean".into()),
            &mut Effects::default(),
        )
        .unwrap();
    wait_for(&h, "turn end 2", |rt| {
        rt.meta.status == AgentStatus::Done
            && rt
                .conv
                .items()
                .iter()
                .filter(|i| i["kind"] == "turn")
                .count()
                == 2
    })
    .await;
    let all = items(&h);
    assert!(all
        .iter()
        .any(|i| i["text"] == "Compris : utilise npm run clean"));
    let tool = all
        .iter()
        .rev()
        .find(|i| i["kind"] == "tool" && i["name"] == "Bash")
        .unwrap();
    assert_eq!(tool["status"], "error");
}

#[tokio::test]
async fn edits_are_counted_and_attributed_to_the_agent() {
    let dir = temp_dir("edit");
    let h = new_agent(&dir);
    let proc = spawn(&h, &dir.join("log.jsonl"));
    send(&h, &proc, "edit");
    wait_for(&h, "turn end", |rt| rt.meta.status == AgentStatus::Done).await;
    let all = items(&h);
    let tool = all
        .iter()
        .find(|i| i["kind"] == "tool" && i["name"] == "Edit")
        .unwrap();
    assert_eq!(tool["result"]["add"], 2);
    assert_eq!(tool["result"]["del"], 1);
    assert_eq!(h.lock().meta.touched_files, vec!["src/app.ts".to_string()]);
}

#[tokio::test]
async fn interrupt_ends_the_turn_without_error() {
    let dir = temp_dir("interrupt");
    let h = new_agent(&dir);
    let proc = spawn(&h, &dir.join("log.jsonl"));
    send(&h, &proc, "slow");
    wait_for(&h, "streaming", |rt| {
        rt.conv.items().iter().any(|i| i["kind"] == "text")
    })
    .await;
    h.lock().interrupted = true;
    proc.control(json!({ "subtype": "interrupt" }), Duration::from_secs(5))
        .await
        .unwrap();
    wait_for(&h, "turn end", |rt| !rt.meta.status.is_active()).await;
    assert_eq!(h.lock().meta.status, AgentStatus::Done);
    let turn = items(&h).into_iter().find(|i| i["kind"] == "turn").unwrap();
    assert_eq!(turn["interrupted"], true);
    assert_eq!(turn["isError"], false);
}

#[tokio::test]
async fn a_new_process_starts_without_the_activity_or_text_of_a_turn_that_never_ended() {
    let dir = temp_dir("fresh-turn");
    let log = dir.join("log.jsonl");
    let h = new_agent(&dir);
    let proc = spawn(&h, &log);
    send(&h, &proc, "slow");
    wait_for(&h, "the text of the turn", |rt| {
        rt.view().activity.as_deref() == Some("Rédige")
            && rt
                .conv
                .items()
                .iter()
                .any(|i| i["kind"] == "text" && i["streaming"] == false)
    })
    .await;

    // A new process takes over before the turn ever ended (no result, no exit seen); what the
    // old one still says is ignored.
    let old = proc;
    let proc = spawn(&h, &log);
    old.kill();
    assert_eq!(h.lock().view().activity, None);
    // The text of the lost turn does not leak into the next one's.
    let mut fx = Effects::default();
    h.lock().handle_frame(
        &json!({"type":"result","subtype":"success","is_error":false,"duration_ms":1,"result":"Résumé"}),
        &mut fx,
    );
    assert_eq!(fx.turn_end, Some(TurnEnd::Finished("Résumé".into())));
    proc.close_input();
}

#[tokio::test]
async fn a_session_claude_code_does_not_know_still_ends_the_turn_sent_to_it() {
    let dir = temp_dir("lost-session");
    let h = new_agent(&dir);
    {
        // The agent resumes a session that is gone, and a message is waiting for it.
        let mut rt = h.lock();
        rt.meta.session_id = Some("missing-1".into());
        rt.push_user("u1", "Bonjour", 0, &[], &mut Effects::default());
    }
    let ends = Arc::<parking_lot::Mutex<Vec<TurnEnd>>>::default();
    spawn_recording(&h, &dir.join("log.jsonl"), ends.clone());
    for _ in 0..500 {
        if !ends.lock().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        *ends.lock(),
        vec![TurnEnd::Error(
            "Session Claude introuvable : une nouvelle session sera démarrée au prochain message."
                .into()
        )]
    );
    let rt = h.lock();
    assert_eq!(rt.meta.status, AgentStatus::Done);
    assert_eq!(rt.meta.session_id, None);
}

/// The `projects/*/<session>.jsonl` of a Claude Code configuration folder.
fn session_files(config_dir: &std::path::Path, session: &str) -> Vec<PathBuf> {
    std::fs::read_dir(config_dir.join("projects"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path().join(format!("{session}.jsonl")))
        .filter(|p| p.is_file())
        .collect()
}

fn logged(log: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

async fn wait_ends(ends: &parking_lot::Mutex<Vec<TurnEnd>>, n: usize) {
    for _ in 0..500 {
        if ends.lock().len() >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for {n} turn ends: {:?}", ends.lock());
}

#[tokio::test]
async fn an_account_keeps_its_sessions_in_its_folder_and_resumes_them_from_there() {
    let dir = temp_dir("account-sessions");
    let (log, config) = (dir.join("log.jsonl"), dir.join("pro"));
    let h = new_agent(&dir);
    let proc = spawn_with(&h, &log, Some(&config), Arc::default(), Arc::default());
    send(&h, &proc, "Premier");
    wait_for(&h, "first turn", |rt| rt.meta.status == AgentStatus::Done).await;
    let session = h.lock().meta.session_id.clone().unwrap();
    // Where Claude Code keeps it: in a folder named after the project's, every character but
    // letters and digits a dash.
    let files = session_files(&config, &session);
    assert_eq!(files.len(), 1, "{files:?}");
    let folder = files[0].parent().unwrap().file_name().unwrap();
    let ours = format!("ccm-test-account-sessions-{}", std::process::id())
        .replace(|c: char| !c.is_ascii_alphanumeric(), "-");
    assert!(folder.to_string_lossy().ends_with(&ours), "{folder:?}");
    assert_eq!(
        logged(&log)[0]["configDir"],
        json!(config.to_string_lossy())
    );

    // A new process of the same account finds it.
    let old = h.lock().detach().expect("a running process");
    old.close_input();
    let proc = spawn_with(&h, &log, Some(&config), Arc::default(), Arc::default());
    send(&h, &proc, "Second");
    wait_for(&h, "second turn", |rt| {
        rt.meta.status == AgentStatus::Done
            && rt
                .conv
                .items()
                .iter()
                .filter(|i| i["kind"] == "turn")
                .count()
                == 2
    })
    .await;
    assert_eq!(h.lock().meta.session_id.as_deref(), Some(session.as_str()));
    assert!(launches(&log)[1].contains(&format!("--resume={session}")));
    let texts: Vec<Value> = items(&h)
        .into_iter()
        .filter(|i| i["kind"] == "text")
        .map(|i| i["text"].clone())
        .collect();
    assert_eq!(
        texts,
        [
            json!("Bonjour, tu as dit : Premier"),
            json!("Bonjour, tu as dit : Second")
        ]
    );
}

#[tokio::test]
async fn a_session_absent_from_the_accounts_folder_is_not_found() {
    let dir = temp_dir("account-elsewhere");
    let h = new_agent(&dir);
    {
        // A session kept in another account's folder, and a message waiting for it.
        let mut rt = h.lock();
        rt.meta.session_id = Some("sess-ailleurs".into());
        rt.push_user("u1", "Bonjour", 0, &[], &mut Effects::default());
    }
    let ends = Arc::<parking_lot::Mutex<Vec<TurnEnd>>>::default();
    let log = dir.join("log.jsonl");
    spawn_with(
        &h,
        &log,
        Some(&dir.join("pro")),
        ends.clone(),
        Arc::default(),
    );
    wait_ends(&ends, 1).await;
    assert_eq!(
        *ends.lock(),
        vec![TurnEnd::Error(
            "Session Claude introuvable : une nouvelle session sera démarrée au prochain message."
                .into()
        )]
    );
    assert_eq!(h.lock().meta.session_id, None);
}

#[tokio::test]
async fn a_session_kept_under_another_project_folder_of_the_account_is_resumed() {
    // A copy of an agent made in a worktree of its own resumes the original's session from
    // another folder: Claude Code finds it among all the account's projects.
    let dir = temp_dir("account-other-folder");
    let config = dir.join("pro");
    let elsewhere = config.join("projects").join("C--Users-x-ailleurs");
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(
        elsewhere.join("sess-ailleurs.jsonl"),
        "{\"type\":\"user\",\"sessionId\":\"sess-ailleurs\"}\n",
    )
    .unwrap();
    let h = new_agent(&dir);
    h.lock().meta.session_id = Some("sess-ailleurs".into());
    let (ends, log) = (
        Arc::<parking_lot::Mutex<Vec<TurnEnd>>>::default(),
        dir.join("log.jsonl"),
    );
    let proc = spawn_with(&h, &log, Some(&config), ends.clone(), Arc::default());
    send(&h, &proc, "Bonjour");
    wait_ends(&ends, 1).await;
    assert!(
        matches!(ends.lock()[0], TurnEnd::Finished(_)),
        "{:?}",
        ends.lock()
    );
    assert!(launches(&log)[0].contains(&"--resume=sess-ailleurs".to_string()));
    assert_eq!(h.lock().meta.session_id.as_deref(), Some("sess-ailleurs"));
}

#[tokio::test]
async fn an_accounts_fake_limit_stops_every_turn_at_the_limit_at_100_percent() {
    let dir = temp_dir("account-limit");
    let config = dir.join("pro");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("fake-limit"), "").unwrap();
    let h = new_agent(&dir);
    let (ends, rates) = (
        Arc::<parking_lot::Mutex<Vec<TurnEnd>>>::default(),
        Rates::default(),
    );
    let proc = spawn_with(
        &h,
        &dir.join("log.jsonl"),
        Some(&config),
        ends.clone(),
        rates.clone(),
    );
    let five_hour = |usage: Value| usage["rate_limits"]["five_hour"]["utilization"].clone();
    let usage = proc
        .control(json!({ "subtype": "get_usage" }), Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(five_hour(usage), json!(100));
    send(&h, &proc, "Bonjour");
    wait_ends(&ends, 1).await;
    assert_eq!(*ends.lock(), vec![TurnEnd::Limited]);
    let told = rates.lock().clone();
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(told[0].0.map(|w| w.pct), Some(100.0));

    // The file gone, the account has quota again.
    std::fs::remove_file(config.join("fake-limit")).unwrap();
    send(&h, &proc, "Encore");
    wait_ends(&ends, 2).await;
    assert!(matches!(ends.lock()[1], TurnEnd::Finished(_)));
    let pct = rates.lock()[1].0.map(|w| w.pct).unwrap();
    assert!((pct - 12.0).abs() < 1e-9, "{pct}");
    let usage = proc
        .control(json!({ "subtype": "get_usage" }), Duration::from_secs(10))
        .await
        .unwrap();
    assert_eq!(five_hour(usage), json!(12));
}

#[tokio::test]
async fn a_crash_during_a_turn_flags_the_agent_in_error() {
    let dir = temp_dir("crash");
    let h = new_agent(&dir);
    let proc = spawn(&h, &dir.join("log.jsonl"));
    send(&h, &proc, "crash");
    wait_for(&h, "exit", |rt| rt.proc.is_none()).await;
    assert_eq!(h.lock().meta.status, AgentStatus::Error);
    assert!(items(&h).iter().any(|i| i["kind"] == "notice"
        && i["level"] == "error"
        && i["text"].as_str().unwrap().contains("code 3")));
}
