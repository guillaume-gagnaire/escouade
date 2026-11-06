//! The client on its own, without Codex: what it describes, what it writes, what it makes of the answers. The bench of
//! the harnesses plays it on the fake Codex through its process (`src-tauri/src/engine/codex/bench.rs`), the real
//! streams through the translation (`src-tauri/src/agent/codex_tests.rs`).

use crate::client::Agent;
use crate::words::{self, FENCE_PROFILE};
use escouade_plugin_protocol::engine::{
    AnswerPermission, Attach, Grant, Item, LaunchParams, StartAt,
};
use serde_json::{json, Value};

const CWD: &str = "C:\\wt";

fn attach() -> Attach {
    Attach {
        agent: "a1".into(),
        cwd: CWD.into(),
        model: "gpt-6-luna".into(),
        mode: "default".into(),
        lang: "fr".into(),
        ..Attach::default()
    }
}

fn params() -> LaunchParams {
    LaunchParams {
        agent: "a1".into(),
        cwd: CWD.into(),
        model: "gpt-6-luna".into(),
        mode: "default".into(),
        lang: "fr".into(),
        ..LaunchParams::default()
    }
}

fn parsed(line: &str) -> Value {
    serde_json::from_str(line).unwrap()
}

/// The lines among `items`.
fn writes(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Write { line } => Some(line.clone()),
            _ => None,
        })
        .collect()
}

/// `lines` answered one after the other, by `answer` of their method, until nothing is written any more: the items
/// made, and the methods asked.
fn play(
    agent: &mut Agent,
    lines: Vec<String>,
    answer: &dyn Fn(&str) -> Value,
) -> (Vec<Item>, Vec<String>) {
    let (mut items, mut asked, mut todo) = (Vec::new(), Vec::new(), lines);
    while !todo.is_empty() {
        let mut next = Vec::new();
        for line in todo {
            let m = parsed(&line);
            let Some(method) = m["method"].as_str() else {
                continue;
            };
            asked.push(method.to_string());
            if m["id"].is_null() {
                continue;
            }
            let reply = json!({ "id": m["id"], "result": answer(method) }).to_string();
            let made = agent.frames(&[reply]);
            next.extend(writes(&made));
            items.extend(made);
        }
        todo = next;
    }
    (items, asked)
}

fn plain(method: &str) -> Value {
    match method {
        "thread/start" | "thread/resume" | "thread/fork" => json!({ "thread": { "id": "thr" } }),
        "model/list" => json!({ "data": [{ "id": "gpt-6-luna", "displayName": "GPT-6-Luna" }] }),
        _ => json!({}),
    }
}

/// An agent whose thread is open.
fn opened(p: LaunchParams) -> Agent {
    let mut a = Agent::new(&attach());
    let launch = a.launch(&p);
    play(&mut a, launch.writes, &plain);
    a
}

fn started(a: &mut Agent) {
    a.frames(&[
        json!({ "method": "turn/started", "params": { "threadId": "thr", "turn": { "id": "t1" } } })
            .to_string(),
    ]);
}

fn answer(request: &str, allow: bool, message: Option<&str>) -> AnswerPermission {
    AnswerPermission {
        agent: "a1".into(),
        request: request.into(),
        allow,
        always: false,
        input: Value::Null,
        message: message.map(str::to_string),
        resume: None,
        call: String::new(),
        rules: Value::Null,
    }
}

#[test]
fn its_manifest_declares_codex_with_its_session_opened_over_the_protocol() {
    let m: Value = serde_json::from_str(crate::MANIFEST).unwrap();
    let h = &m["contributes"]["harnesses"][0];
    assert_eq!(h["id"], "codex");
    assert_eq!(h["opens"], true);
    assert_eq!(h["accountVar"], "CODEX_HOME");
    assert_eq!(h["tested"]["oldest"], "0.163.0");
    assert_eq!(m["run"]["wasm"], "Cargo.toml");
}

#[test]
fn it_opens_a_thread_then_reads_every_page_of_the_models_then_says_it_is_ready() {
    let mut a = Agent::new(&attach());
    let launch = a.launch(&params());
    assert_eq!(launch.args[0], "app-server");
    let page = std::cell::Cell::new(0);
    let (items, asked) = play(&mut a, launch.writes, &|method| match method {
        "model/list" => {
            page.set(page.get() + 1);
            if page.get() == 1 {
                json!({ "data": [{ "id": "one" }], "nextCursor": "c" })
            } else {
                json!({ "data": [{ "id": "two" }] })
            }
        }
        m => plain(m),
    });
    assert_eq!(
        asked,
        [
            "initialize",
            "initialized",
            "thread/start",
            "model/list",
            "model/list"
        ]
    );
    assert!(matches!(&items[..], [.., Item::Ready { models }] if models[1]["id"] == "two"));
    assert!(items
        .iter()
        .any(|i| matches!(i, Item::Session { session } if session == "thr")));
}

#[test]
fn a_thread_refused_stops_it_and_says_why() {
    let mut a = Agent::new(&attach());
    let launch = a.launch(&params());
    let (_, asked) = play(&mut a, launch.writes.clone(), &|_| json!({ "thread": {} }));
    assert_eq!(asked, ["initialize", "initialized", "thread/start"]);
    let mut a = Agent::new(&attach());
    let launch = a.launch(&params());
    let first = parsed(&launch.writes[0]);
    let refused = json!({ "id": first["id"], "error": { "message": "non" } }).to_string();
    let items = a.frames(&[refused]);
    assert!(matches!(&items[..], [Item::Stop { reason }] if reason == "initialize: non"));
}

#[test]
fn a_resumed_or_copied_thread_is_asked_as_such() {
    let thread_method = |start: StartAt| {
        let mut a = Agent::new(&attach());
        let launch = a.launch(&LaunchParams { start, ..params() });
        let id = parsed(&launch.writes[0])["id"].clone();
        let made = a.frames(&[json!({ "id": id, "result": {} }).to_string()]);
        parsed(&writes(&made)[1])
    };
    let resume = thread_method(StartAt::Resume {
        session: "s".into(),
    });
    assert_eq!(
        (
            resume["method"].as_str(),
            resume["params"]["threadId"].as_str()
        ),
        (Some("thread/resume"), Some("s"))
    );
    let fork = thread_method(StartAt::Fork {
        of: "s".into(),
        at: Some("t".into()),
    });
    assert_eq!(
        (
            fork["method"].as_str(),
            fork["params"]["threadId"].as_str(),
            fork["params"]["lastTurnId"].as_str()
        ),
        (Some("thread/fork"), Some("s"), Some("t"))
    );
}

#[test]
fn the_token_goes_by_a_variable_and_a_fence_by_its_profiles() {
    let mut a = Agent::new(&attach());
    let launch = a.launch(&LaunchParams {
        escouade: Some(Grant {
            url: "http://127.0.0.1:1/mcp".into(),
            token: "secret".into(),
        }),
        fence: vec![r"C:\code\secret".into()],
        ..params()
    });
    assert!(launch
        .args
        .iter()
        .all(|x| !x.contains("secret\"=") || x.contains("deny")));
    assert!(launch.args.iter().all(|x| !x.contains("=secret")));
    assert_eq!(launch.env[0].name, words::TOKEN_VAR);
    assert_eq!(launch.env[0].value, "secret");
    assert!(launch.fence_applied);
    assert!(launch.args.iter().any(|x| x.contains("C:/code/secret")));
    assert!(launch
        .args
        .contains(&format!("default_permissions=\"{FENCE_PROFILE}\"")));
}

#[test]
fn a_refused_server_turns_off_only_an_entry_the_user_declares() {
    // The arguments of a refusal, from a `config.toml` that declares it or not.
    let args = |text: &str| {
        words::app_server_args(&[], "default", false, None, || {
            words::declares_escouade(text)
        })
    };
    assert!(!args("").iter().any(|a| a.contains("mcp_servers")));
    assert!(args("[mcp_servers.escouade]\ncommand = \"x\"\n")
        .contains(&"mcp_servers.escouade.enabled=false".to_string()));
}

#[test]
fn a_message_during_the_turns_start_goes_into_it_once_it_has_its_id() {
    let mut a = opened(params());
    let first = a.send("un", None);
    assert_eq!(parsed(&first[0])["method"], "turn/start");
    assert!(a.send("deux", None).is_empty(), "held");
    let items = a.frames(&[
        json!({ "method": "turn/started", "params": { "threadId": "thr", "turn": { "id": "t1" } } })
            .to_string(),
    ]);
    let steer = parsed(&writes(&items)[0]);
    assert_eq!(steer["method"], "turn/steer");
    assert_eq!(steer["params"]["expectedTurnId"], "t1");
    assert_eq!(steer["params"]["input"][0]["text"], "deux");
}

#[test]
fn a_turn_refused_ends_in_error_with_its_reason() {
    let mut a = opened(params());
    let line = a.send("un", None).remove(0);
    let refused =
        json!({ "id": parsed(&line)["id"], "error": { "message": "modèle refusé" } }).to_string();
    let items = a.frames(&[refused]);
    assert!(items
        .iter()
        .any(|i| matches!(i, Item::Turn { error: Some(e), .. } if e == "modèle refusé")));
}

#[test]
fn a_refusal_tells_the_agent_a_reason_of_the_users_and_only_that() {
    let request = |a: &mut Agent, id: u64| {
        a.frames(&[json!({
            "id": id, "method": "item/commandExecution/requestApproval",
            "params": { "threadId": "thr", "itemId": "c1", "command": "ls" },
        })
        .to_string()]);
    };
    let mut a = opened(params());
    started(&mut a);
    request(&mut a, 7);
    let lines = a.answer_permission(&answer("7", false, Some("Plutôt autre chose")));
    assert_eq!(parsed(&lines[0])["result"]["decision"], "decline");
    assert_eq!(parsed(&lines[1])["method"], "turn/steer");
    request(&mut a, 8);
    let lines = a.answer_permission(&answer(
        "8",
        false,
        Some("L'utilisateur a refusé cette action."),
    ));
    assert_eq!(lines.len(), 1, "no turn nobody asked for");
}

#[test]
fn a_plan_approved_starts_its_work_in_the_mode_left_for_it() {
    let mut a = opened(LaunchParams {
        mode: "plan".into(),
        ..params()
    });
    let lines = a.answer_permission(&AnswerPermission {
        resume: Some("acceptEdits".into()),
        ..answer("plan:t1", true, None)
    });
    let turn = parsed(&lines[0]);
    assert_eq!(turn["method"], "turn/start");
    assert_eq!(turn["params"]["input"][0]["text"], "Mets le plan en œuvre.");
    assert_eq!(turn["params"]["collaborationMode"]["mode"], "default");
}

#[test]
fn the_answers_of_a_card_go_back_by_the_ids_of_codexs_questions() {
    let mut a = opened(params());
    a.frames(&[json!({
        "id": "q1", "method": "item/tool/requestUserInput",
        "params": { "threadId": "thr", "itemId": "c", "questions": [{ "id": "lang", "question": "Quoi ?" }] },
    })
    .to_string()]);
    let lines = a.answer_question("q1", &json!({ "Quoi ?": "Rust" }));
    let m = parsed(&lines[0]);
    assert_eq!(m["id"], "q1");
    assert_eq!(m["result"]["answers"]["lang"]["answers"][0], "Rust");
}

#[test]
fn a_request_waiting_when_the_plugin_restarted_is_answered_by_its_id() {
    let mut a = Agent::new(&Attach {
        session: Some("thr".into()),
        pending: vec!["7".into()],
        ..attach()
    });
    let lines = a.answer_permission(&answer("7", true, None));
    let m = parsed(&lines[0]);
    assert_eq!(m["id"], 7);
    assert_eq!(m["result"]["decision"], "accept");
}

#[test]
fn the_quota_and_a_compaction_come_back_as_their_items() {
    let mut a = opened(params());
    let line = a.read_usage().remove(0);
    let limits = json!({ "rateLimits": { "primary": {
        "usedPercent": 2, "windowDurationMins": 300, "resetsAt": 10,
    } } });
    let items = a.frames(&[json!({ "id": parsed(&line)["id"], "result": limits }).to_string()]);
    assert!(matches!(&items[..], [Item::Usage { usage }] if usage["meters"][0]["minutes"] == 300));
    let line = a.compact().unwrap().remove(0);
    assert_eq!(parsed(&line)["method"], "thread/compact/start");
    let items = a.frames(&[json!({ "id": parsed(&line)["id"], "result": {} }).to_string()]);
    assert!(items.is_empty(), "a turn of its own ends it");
}

#[test]
fn a_git_commit_of_its_worktree_is_accepted_without_a_card_only_by_a_process_in_its_modes() {
    let dir = std::env::temp_dir().join(format!("esc225-git-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    let init = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&dir)
        .status();
    assert!(init.is_ok_and(|s| s.success()));
    let cwd = dir.to_string_lossy().into_owned();
    let mut a = opened(LaunchParams {
        cwd: cwd.clone(),
        mode: "acceptEdits".into(),
        ..params()
    });
    let script = "git add a.txt && git commit -m x";
    let ask = json!({
        "id": 9, "method": "item/commandExecution/requestApproval",
        "params": {
            "threadId": "thr", "itemId": "c1", "cwd": cwd,
            "command": format!("pwsh.exe -Command '{script}'"),
            "commandActions": [{ "command": script }],
        },
    })
    .to_string();
    let items = a.frames(std::slice::from_ref(&ask));
    let line = parsed(&writes(&items)[0]);
    assert_eq!(line["id"], 9);
    assert_eq!(line["result"]["decision"], "accept");
    assert!(!items
        .iter()
        .any(|i| matches!(i, Item::PermissionCard { .. })));
    // « Demander » has its card.
    let mut asks = opened(LaunchParams { cwd, ..params() });
    let items = asks.frames(&[ask]);
    assert!(matches!(&items[..], [Item::PermissionCard { .. }]));
}
