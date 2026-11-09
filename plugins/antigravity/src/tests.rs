//! The client on its own: what it asks of the core for each thing `agy` says and the user answers. The fake's runs are
//! `src-tauri/src/engine/agy/bench.rs` and `src-tauri/src/agent/agy_tests.rs`.

use crate::client::Client;
use crate::words;
use escouade_plugin_protocol::engine::{
    AnswerPermission, Attach, Gone, Grant, Item, LaunchParams, StartAt,
};
use serde_json::{json, Value};
use std::path::PathBuf;

fn home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("esc283-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn params(home: &std::path::Path) -> LaunchParams {
    LaunchParams {
        agent: "agy-a1".into(),
        cwd: if cfg!(windows) { "C:\\wt" } else { "/wt" }.into(),
        model: "gemini-3.8-flash".into(),
        effort: "high".into(),
        mode: "default".into(),
        lang: "fr".into(),
        home: Some(home.to_string_lossy().into_owned()),
        ..LaunchParams::default()
    }
}

fn started(p: &LaunchParams) -> (Client, Vec<Item>) {
    let mut c = Client::new(&Attach {
        agent: p.agent.clone(),
        cwd: p.cwd.clone(),
        lang: "fr".into(),
        ..Attach::default()
    });
    c.launch(p);
    let out = c.frames(&[json!({ "event": "init", "conversation_id": "conv-1" }).to_string()]);
    (c, out)
}

fn step(index: u64, tool: &str, state: &str, args: Value, error: Option<&str>) -> String {
    let mut info = json!({ "parameters": args });
    if let Some(e) = error {
        info["error"] = json!({ "type": "TOOL_ERROR", "message": e });
    }
    json!({ "event": "step_update", "step_update": {
        "step_index": index, "step_type": "tool", "tool_name": tool, "state": state, "tool_info": info,
    } })
    .to_string()
}

fn result() -> String {
    json!({ "event": "result", "result": { "status": "SUCCESS" } }).to_string()
}

fn request(items: &[Item]) -> String {
    items
        .iter()
        .find_map(|i| match i {
            Item::Permission { request, .. } | Item::Question { request, .. } => {
                Some(request.clone())
            }
            _ => None,
        })
        .expect("a card")
}

fn answer(request: &str, allow: bool, always: bool) -> AnswerPermission {
    AnswerPermission {
        agent: "agy-a1".into(),
        request: request.into(),
        allow,
        always,
        input: Value::Null,
        message: None,
        resume: None,
        call: String::new(),
        rules: Value::Null,
    }
}

#[test]
fn its_launch_has_its_rules_and_its_server_in_files_never_its_token_in_its_arguments() {
    let dir = home("launch");
    let mut p = params(&dir);
    p.escouade = Some(Grant {
        url: "http://127.0.0.1:9/mcp".into(),
        token: "s3cr3t".into(),
    });
    p.fence = vec![if cfg!(windows) { "C:\\prive" } else { "/prive" }.into()];
    let mut c = Client::new(&Attach {
        agent: p.agent.clone(),
        ..Attach::default()
    });
    let launch = c.launch(&p);
    assert!(launch.args.iter().all(|a| !a.contains("s3cr3t")));
    assert!(launch
        .args
        .windows(2)
        .any(|w| w == ["--model", "gemini-3.8-flash"]));
    assert!(launch.fence_applied);
    // Its rules, its server, and the mark of its temporary folder (ESC-388).
    assert_eq!(launch.files.len(), 3);
    assert!(launch.files[0].text.contains("prive"));
    assert!(launch.files[1].text.contains("Bearer s3cr3t"));
    assert!(launch.remove_env.iter().any(|v| v == "GEMINI_API_KEY"));
    assert!(launch
        .env
        .iter()
        .any(|v| v.name == "AGY_CLI_DISABLE_AUTO_UPDATE" && v.value == "1"));
    assert!(launch
        .env
        .iter()
        .any(|v| v.name == "USERPROFILE" && v.value == dir.to_string_lossy()));
    // A discussion holds by its rules.
    let mut p = params(&dir);
    p.read_only = true;
    let launch = c.launch(&p);
    assert!(launch.read_only);
    assert!(launch.files[0].text.contains("write_file(*)"));
    // An account by its key keeps it, and has the Gemini API as its provider.
    let mut p = params(&dir);
    p.by_key = true;
    let launch = c.launch(&p);
    assert!(!launch.remove_env.iter().any(|v| v == "GEMINI_API_KEY"));
    assert!(launch.files[0].text.contains("modelProvider"));
}

#[test]
fn its_first_init_opens_the_agent_and_a_conversation_to_resume_gone_is_said() {
    let dir = home("init");
    let (_, out) = started(&params(&dir));
    assert!(out.iter().any(|i| matches!(i, Item::Ready { .. })));
    assert!(!out.iter().any(|i| matches!(i, Item::Notice { .. })));
    let mut p = params(&dir);
    p.start = StartAt::Resume {
        session: "gone".into(),
    };
    let (_, out) = started(&p);
    assert!(out.iter().any(|i| matches!(i, Item::Notice { .. })));
}

#[test]
fn a_command_refused_is_asked_at_the_end_of_the_turn_then_its_program_starts_again() {
    let dir = home("denied");
    let (mut c, _) = started(&params(&dir));
    c.send("commande");
    let args = json!({ "CommandLine": "echo hi" });
    let mut out = c.frames(&[
        step(4, "run_command", "ACTIVE", args.clone(), None),
        step(
            4,
            "run_command",
            "ERROR",
            args,
            Some("permission check failed"),
        ),
    ]);
    assert!(!out.iter().any(|i| matches!(i, Item::Permission { .. })));
    out = c.frames(&[result()]);
    assert!(!out.iter().any(|i| matches!(i, Item::Turn { .. })));
    let rid = request(&out);
    let out = c.answer_permission(&answer(&rid, true, true)).unwrap();
    // Kept for good by the plugin alone (no file the agent could write), its rule written for the next turn, then its
    // program again and the message that goes on, in that order.
    assert!(!out
        .iter()
        .any(|i| matches!(i, Item::File { path, .. } if !path.ends_with("settings.json"))));
    let at = |f: &dyn Fn(&Item) -> bool| out.iter().position(f).expect("an item");
    let rules = at(
        &|i| matches!(i, Item::File { path, text } if path.ends_with("settings.json") && text.contains("command(echo hi)")),
    );
    let relaunch = at(&|i| matches!(i, Item::Relaunch { .. }));
    let write = at(&|i| matches!(i, Item::Write { .. }));
    assert!(rules < relaunch && relaunch < write);
    let Some(Item::Relaunch { launch }) = out.get(relaunch) else {
        panic!("a relaunch");
    };
    assert!(launch
        .args
        .windows(2)
        .any(|w| w == ["--conversation", "conv-1"]));
    assert!(launch.files[0].text.contains("command(echo hi)"));
}

#[test]
fn a_turn_stopped_pauses_its_program_and_leaves_what_it_wrote_aside() {
    let dir = home("interrupt");
    let (mut c, _) = started(&params(&dir));
    c.send("lent");
    let out = c.interrupt(false);
    assert_eq!(out[0], Item::Pause);
    assert!(out.iter().any(|i| matches!(
        i,
        Item::Turn {
            interrupted: true,
            ..
        }
    )));
    let late = c.frames(&[step(9, "run_command", "ACTIVE", json!({}), None), result()]);
    assert!(late.is_empty(), "{late:?}");
    let out = c.send("reprends");
    assert!(matches!(out[0], Item::Relaunch { .. }));
    assert!(matches!(out[1], Item::Write { .. }));
    // Its next program is heard.
    let out = c.frames(&[
        json!({ "event": "init", "conversation_id": "conv-1" }).to_string(),
        result(),
    ]);
    assert!(out.iter().any(|i| matches!(i, Item::Turn { .. })));
}

#[test]
fn a_message_during_a_turn_waits_for_its_end() {
    let dir = home("queue");
    let (mut c, _) = started(&params(&dir));
    c.send("un");
    assert!(c.send("deux").is_empty());
    let out = c.frames(&[result()]);
    let (turn, write) = (
        out.iter().position(|i| matches!(i, Item::Turn { .. })),
        out.iter().position(|i| matches!(i, Item::Write { .. })),
    );
    assert!(turn.unwrap() < write.unwrap());
    assert!(out.iter().any(|i| matches!(i, Item::Start { .. })));
}

#[test]
fn a_mode_or_a_model_changed_starts_its_program_again_at_the_next_message_only() {
    let dir = home("settings");
    let (mut c, _) = started(&params(&dir));
    let out = c.set_mode("bypassPermissions");
    assert!(matches!(&out[0], Item::File { text, .. } if text.contains("command(*)")));
    // « Tout accepter » is its rules': no `--mode` to change, but its rules must hold from the next message on.
    assert!(c.send("un").iter().any(
        |i| matches!(i, Item::Relaunch { launch } if !launch.args.iter().any(|a| a == "--mode"))
    ));
    c.frames(&[
        json!({ "event": "init", "conversation_id": "conv-1" }).to_string(),
        result(),
    ]);
    c.set_mode("plan");
    let out = c.send("deux");
    assert!(
        matches!(&out[0], Item::Relaunch { launch } if launch.args.windows(2).any(|w| w == ["--mode", "plan"]))
    );
}

#[test]
fn a_program_gone_before_its_init_gives_a_new_conversation_or_says_why() {
    let dir = home("gone");
    let mut p = params(&dir);
    p.start = StartAt::Resume {
        session: "gone".into(),
    };
    let mut c = Client::new(&Attach {
        agent: p.agent.clone(),
        ..Attach::default()
    });
    assert!(c.launch(&p).args.iter().any(|a| a == "gone"));
    let gone = Gone {
        agent: p.agent.clone(),
        code: Some(1),
        said: "\nconversation \"gone\" not found\n".into(),
    };
    let out = c.program_gone(&gone);
    assert!(
        matches!(&out[0], Item::Relaunch { launch } if !launch.args.iter().any(|a| a == "gone"))
    );
    // The new one did not start either: why is said, with `agy`'s own last words.
    let out = c.program_gone(&gone);
    assert!(
        matches!(&out[0], Item::Stop { reason } if reason.contains("not found") && reason.contains("agy dans un terminal"))
    );
    // Once it runs, its end is the agent's: nothing to do.
    let (mut c, _) = started(&params(&dir));
    assert!(c.program_gone(&gone).is_empty());
}

#[test]
fn a_card_lost_by_a_restart_of_the_plugin_ends_the_turn() {
    let dir = home("restart");
    let p = params(&dir);
    // Given again after a crash: its program runs, no `init` comes, its turn goes on.
    let mut c = Client::new(&Attach {
        agent: p.agent.clone(),
        cwd: p.cwd.clone(),
        session: Some("conv-1".into()),
        home: p.home.clone(),
        running: true,
        lang: "fr".into(),
        ..Attach::default()
    });
    let out = c
        .answer_permission(&answer("acp-1-agy-a1-4", true, false))
        .unwrap();
    assert!(out.iter().any(|i| matches!(i, Item::Turn { .. })));
    assert!(c.answer_permission(&answer("x", true, false)).is_err());
}

#[test]
fn a_fenced_call_is_never_a_card() {
    let dir = home("fence");
    let mut p = params(&dir);
    let fence = if cfg!(windows) { "C:\\prive" } else { "/prive" };
    p.fence = vec![fence.into()];
    let (mut c, _) = started(&p);
    c.send("lis");
    let args = json!({ "CommandLine": format!("cat {fence}/secret") });
    c.frames(&[step(
        4,
        "run_command",
        "ERROR",
        args,
        Some("denied permission"),
    )]);
    let out = c.frames(&[result()]);
    assert!(!out.iter().any(|i| matches!(i, Item::Permission { .. })));
    assert!(out.iter().any(|i| matches!(i, Item::Turn { .. })));
}

fn text_done(step: u64) -> String {
    json!({ "event": "step_update", "step_update": {
        "step_index": step, "step_type": "agent_response", "state": "DONE", "usage": { "input_tokens": 1 },
    } })
    .to_string()
}

fn written(dir: &std::path::Path, line: Value) {
    let file = words::transcript(dir, "conv-1");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, format!("{line}\n")).unwrap();
}

#[test]
fn a_transcript_late_holds_what_follows_in_its_order_until_it_is_written() {
    let dir = home("late");
    let (mut c, _) = started(&params(&dir));
    c.send("lis");
    let out = c.frames(&[
        text_done(3),
        step(4, "run_command", "ACTIVE", json!({}), None),
    ]);
    assert!(out.is_empty(), "{out:?}");
    assert!(c.waits());
    // A batch that comes meanwhile goes behind it.
    assert!(c.frames(&[result()]).is_empty());
    // The first ring may be of the turn that held it: what it answered goes out first.
    assert!(c.tick().is_empty());
    assert!(c.tick().is_empty(), "nothing is written yet");
    written(
        &dir,
        json!({ "step_index": 3, "tool_calls": [{ "name": "run_command", "args": { "CommandLine": "echo tard" } }] }),
    );
    let out = c.tick();
    assert!(!c.waits());
    let at = |f: &dyn Fn(&Item) -> bool| out.iter().position(f).expect("an item");
    let call = at(&|i| format!("{i:?}").contains("echo tard"));
    let turn = at(&|i| matches!(i, Item::Turn { .. }));
    assert!(call < turn, "{out:?}");
}

#[test]
fn a_transcript_never_written_lets_go_after_its_patience() {
    let dir = home("never");
    let (mut c, _) = started(&params(&dir));
    c.send("lis");
    assert!(c.frames(&[text_done(3), result()]).is_empty());
    let ticks: Vec<Vec<Item>> = (0..16).map(|_| c.tick()).collect();
    assert!(ticks[..15].iter().all(Vec::is_empty), "{ticks:?}");
    assert!(ticks[15].iter().any(|i| matches!(i, Item::Turn { .. })));
    assert!(!c.waits());
}

#[test]
fn a_write_refused_in_a_transcript_late_is_a_card_all_the_same() {
    let dir = home("refused");
    let (mut c, _) = started(&params(&dir));
    c.send("écris");
    // Out of the agent's folder: a card.
    let path = if cfg!(windows) {
        "C:\\autre\\a"
    } else {
        "/autre/a"
    };
    let args = json!({ "Path": path, "Content": "x" });
    let out = c.frames(&[
        step(4, "write_file", "ACTIVE", args.clone(), None),
        step(4, "write_file", "DONE", args, None),
        result(),
    ]);
    assert!(!out.iter().any(|i| matches!(i, Item::Turn { .. })));
    written(
        &dir,
        json!({ "step_index": 4, "status": "ERROR", "error": "denied permission" }),
    );
    c.tick();
    let out = c.tick();
    assert!(
        out.iter().any(|i| matches!(i, Item::Permission { .. })),
        "{out:?}"
    );
}

#[test]
fn a_turn_stopped_reads_at_once_what_was_held() {
    let dir = home("hurry");
    let (mut c, _) = started(&params(&dir));
    c.send("lent");
    assert!(c
        .frames(&[
            text_done(3),
            step(4, "run_command", "ACTIVE", json!({}), None)
        ])
        .is_empty());
    let out = c.interrupt(false);
    let at = |f: &dyn Fn(&Item) -> bool| out.iter().position(f).expect("an item");
    let call = at(&|i| !matches!(i, Item::Pause | Item::Turn { .. }));
    assert!(call < at(&|i| matches!(i, Item::Pause)), "{out:?}");
    assert!(!c.waits());
}

#[test]
fn what_the_agent_wrote_in_its_home_gives_it_no_right() {
    // C8: `escouade-always.json` is no longer read, `toolPermission` and `agentMode` are taken out, and only Escouade's
    // server is kept in its MCP config.
    let dir = home("always");
    std::fs::write(dir.join("escouade-always.json"), r#"["command(*)"]"#).unwrap();
    let settings = words::settings_file(&dir);
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(
        &settings,
        r#"{"toolPermission":"always-proceed","agentMode":"auto","trustedWorkspaces":["x"]}"#,
    )
    .unwrap();
    let mcp = words::mcp_file(&dir);
    std::fs::create_dir_all(mcp.parent().unwrap()).unwrap();
    std::fs::write(
        &mcp,
        r#"{"mcpServers":{"planted":{"command":"node","args":["x.js"]}}}"#,
    )
    .unwrap();
    let mut p = params(&dir);
    p.escouade = Some(Grant {
        url: "http://127.0.0.1:9/mcp".into(),
        token: "t".into(),
    });
    let mut c = Client::new(&Attach {
        agent: p.agent.clone(),
        ..Attach::default()
    });
    let launch = c.launch(&p);
    let rules: Value = serde_json::from_str(&launch.files[0].text).unwrap();
    assert_eq!(rules["permissions"]["allow"], json!([]));
    // What it planted is not kept: what no rule allows needs a review (ESC-388).
    assert_eq!(rules["toolPermission"], "request-review");
    assert!(rules.get("agentMode").is_none());
    // What `agy` itself keeps there (the trust of its interface) stays.
    assert_eq!(rules["trustedWorkspaces"], json!(["x"]));
    let servers: Value = serde_json::from_str(&launch.files[1].text).unwrap();
    assert_eq!(
        servers["mcpServers"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        [escouade_acp::SERVER]
    );
}

#[test]
fn the_note_of_a_card_leaves_the_list_its_choices() {
    // With a card's reason, the list beside the conversation takes a note of 240 characters at most (`LIST_NOTE_MAX`).
    for lang in ["fr", "en"] {
        let n = words::once_note(lang).chars().count();
        assert!(n <= words::ONCE_NOTE_MAX, "{lang}: {n}");
    }
    assert!(words::ONCE_NOTE_MAX + "Hors du dossier de l’agent\n".chars().count() <= 240);
}
