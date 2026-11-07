//! The client and the translation on their own: what the process is told, what its lines make. The rest (a real process of
//! the fake `claude`) is in the core's tests: `process_tests.rs`, `engine/claude/bench.rs`, and every test of an agent.

use crate::client::Agent;
use crate::translate::Translator;
use escouade_plugin_protocol::engine::{
    AnswerPermission, Attach, Grant, Item, LaunchParams, RemoteLink, StartAt,
};
use serde_json::{json, Value};

fn agent() -> Agent {
    Agent::new(&Attach {
        agent: "a1".into(),
        lang: "fr".into(),
        ..Attach::default()
    })
}

fn params() -> LaunchParams {
    LaunchParams {
        agent: "a1".into(),
        cwd: "/w".into(),
        model: "opus".into(),
        effort: "high".into(),
        mode: "auto".into(),
        ..LaunchParams::default()
    }
}

fn parsed(line: &str) -> Value {
    serde_json::from_str(line).unwrap()
}

fn items(f: Value) -> Vec<Item> {
    let mut out = Vec::new();
    Translator::default().frame(&f, "fr", &mut out);
    out
}

fn answer(a: &mut Agent, id: &str, response: Value) -> Vec<Item> {
    let line = json!({
        "type": "control_response",
        "response": { "subtype": "success", "request_id": id, "response": response },
    });
    a.frames(&[line.to_string()])
}

#[test]
fn its_manifest_declares_claude_code_with_its_own_folder_variable() {
    let m: Value = serde_json::from_str(crate::MANIFEST).unwrap();
    let h = &m["contributes"]["harnesses"][0];
    assert_eq!(h["id"], "claude");
    assert_eq!(h["accountVar"], "CLAUDE_CONFIG_DIR");
    assert_eq!(h["opens"], true);
    assert_eq!(m["run"]["wasm"], "Cargo.toml");
}

#[test]
fn a_launch_says_where_it_starts_from_and_asks_initialize_first() {
    let mut a = agent();
    let l = a.launch(&LaunchParams {
        start: StartAt::Fork {
            of: "s1".into(),
            at: Some("e1".into()),
        },
        read_dirs: vec!["/files".into()],
        append_prompt: Some("Protocole\n  du ticket".into()),
        ..params()
    });
    let at = |flag: &str| l.args.iter().position(|x| x == flag).unwrap();
    assert_eq!(l.args[at("--model") + 1], "opus");
    assert_eq!(l.args[at("--effort") + 1], "high");
    assert_eq!(l.args[at("--permission-mode") + 1], "auto");
    assert_eq!(
        l.args[at("--append-system-prompt") + 1],
        "Protocole du ticket"
    );
    assert!(l
        .args
        .windows(3)
        .any(|w| w == ["--resume=s1", "--fork-session", "--resume-session-at=e1"]));
    assert!(l.args.contains(&"--add-dir=/files".to_string()));
    assert_eq!(l.writes.len(), 1);
    assert_eq!(parsed(&l.writes[0])["request"]["subtype"], "initialize");
}

#[test]
fn the_effort_follows_what_its_account_told_else_the_rule() {
    let has = |p: LaunchParams| agent().launch(&p).args.contains(&"--effort".to_string());
    let haiku = LaunchParams {
        model: "haiku".into(),
        ..params()
    };
    assert!(!has(haiku.clone()));
    assert!(has(LaunchParams {
        takes_effort: Some(true),
        ..haiku
    }));
    assert!(!has(LaunchParams {
        takes_effort: Some(false),
        ..params()
    }));
}

#[test]
fn escouades_server_goes_by_its_file_and_never_by_its_token() {
    let granted = agent().launch(&LaunchParams {
        escouade: Some(Grant {
            url: "http://127.0.0.1:1/mcp".into(),
            token: "secret".into(),
        }),
        escouade_file: Some("/own/a1.json".into()),
        ..params()
    });
    assert!(granted
        .args
        .windows(2)
        .any(|w| w == ["--mcp-config", "/own/a1.json"]));
    assert!(!granted.args.iter().any(|a| a.contains("secret")));
    // Refused: kept out by the core's file, its tools refused, what follows no value of theirs.
    let refused = agent().launch(&LaunchParams {
        refusal_file: Some("/own/refusal.json".into()),
        ..params()
    });
    let n = refused.args.len();
    assert_eq!(
        refused.args[n - 4..],
        [
            "--settings",
            "/own/refusal.json",
            "--disallowedTools",
            "mcp__escouade"
        ]
    );
}

#[test]
fn the_fence_is_the_settings_file_the_core_wrote() {
    let fence = vec!["C:/prive".to_string()];
    let l = agent().launch(&LaunchParams {
        fence: fence.clone(),
        settings: Some("/own/settings.json".into()),
        refusal_file: Some("/own/refusal.json".into()),
        ..params()
    });
    assert!(l.fence_applied);
    // Claude Code reads the last `--settings` only: its own holds the refusal too.
    assert!(l
        .args
        .windows(2)
        .any(|w| w == ["--settings", "/own/settings.json"]));
    assert!(!l.args.contains(&"/own/refusal.json".to_string()));
    // Without its file, no fence: the core refuses the launch.
    let bare = agent().launch(&LaunchParams { fence, ..params() });
    assert!(!bare.fence_applied);
}

#[test]
fn a_discussion_is_refused_what_writes_in_one_value() {
    let l = agent().launch(&LaunchParams {
        read_only: true,
        ..params()
    });
    assert!(l.read_only);
    assert!(l
        .args
        .contains(&"--disallowedTools=Edit,Write,MultiEdit,NotebookEdit,Bash,PowerShell,Monitor,EnterWorktree".to_string()));
}

#[test]
fn the_answer_to_initialize_tells_the_core_and_opens_the_agent() {
    let mut a = agent();
    let l = a.launch(&params());
    let id = parsed(&l.writes[0])["request_id"]
        .as_str()
        .unwrap()
        .to_string();
    let got = answer(
        &mut a,
        &id,
        json!({ "commands": [{ "name": "clear" }], "models": [{ "value": "opus" }] }),
    );
    assert_eq!(
        got,
        vec![
            Item::Told {
                commands: json!([{ "name": "clear" }]),
                models: json!([{ "value": "opus" }]),
            },
            Item::Ready { models: json!([]) },
        ]
    );
    // Told once: the same id again is nobody's.
    assert!(answer(&mut a, &id, json!({})).is_empty());
}

#[test]
fn a_refused_initialize_stops_it_with_its_words() {
    let mut a = agent();
    let l = a.launch(&params());
    let id = parsed(&l.writes[0])["request_id"].clone();
    let line = json!({ "type": "control_response",
        "response": { "subtype": "error", "request_id": id, "error": "non" } });
    assert_eq!(
        a.frames(&[line.to_string()]),
        vec![Item::Stop {
            reason: "non".into()
        }]
    );
}

#[test]
fn what_is_asked_has_its_answer_under_its_id_and_the_quota_comes_as_usage() {
    let mut a = agent();
    let asked = a.set_mode("plan");
    let sent = parsed(&asked.writes[0]);
    assert_eq!(sent["request"]["subtype"], "set_permission_mode");
    assert_eq!(sent["request_id"], json!(asked.ask));
    assert_eq!(
        answer(&mut a, &asked.ask, json!({ "ok": true })),
        vec![Item::Answer {
            ask: asked.ask.clone(),
            answer: json!({ "ok": true }),
            error: None,
        }]
    );
    let lines = a.read_usage();
    let id = parsed(&lines[0])["request_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        answer(&mut a, &id, json!({ "rate_limits": { "five_hour": {} } })),
        vec![Item::Usage {
            usage: json!({ "five_hour": {} })
        }]
    );
    let lines = a.read_usage();
    let id = parsed(&lines[0])["request_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        answer(&mut a, &id, json!({})),
        vec![Item::Usage { usage: Value::Null }]
    );
}

#[test]
fn a_line_that_is_no_frame_is_kept_for_what_its_end_says() {
    let mut a = agent();
    assert_eq!(
        a.frames(&["w\0s\0l\0: erreur".to_string(), "   ".to_string()]),
        vec![Item::Said {
            line: "wsl: erreur".into()
        }]
    );
}

#[test]
fn an_answer_to_a_card_says_what_was_chosen() {
    let a = agent();
    let base = AnswerPermission {
        agent: "a1".into(),
        request: "r1".into(),
        allow: true,
        always: false,
        input: json!({ "command": "ls" }),
        message: None,
        resume: None,
        call: "c1".into(),
        rules: Value::Null,
    };
    let said = |p: &AnswerPermission| parsed(&a.answer_permission(p)[0])["response"].clone();
    let allowed = said(&base);
    assert_eq!(allowed["request_id"], "r1");
    assert_eq!(allowed["response"]["behavior"], "allow");
    assert_eq!(allowed["response"]["toolUseID"], "c1");
    assert_eq!(allowed["response"]["updatedInput"]["command"], "ls");
    assert!(allowed["response"].get("updatedPermissions").is_none());
    let always = said(&AnswerPermission {
        always: true,
        rules: json!([{ "type": "addRules" }]),
        ..base.clone()
    });
    assert_eq!(
        always["response"]["updatedPermissions"],
        json!([{ "type": "addRules" }])
    );
    let plan = said(&AnswerPermission {
        resume: Some("auto".into()),
        ..base.clone()
    });
    assert_eq!(
        plan["response"]["updatedPermissions"],
        json!([{ "type": "setMode", "mode": "auto", "destination": "session" }])
    );
    let denied = said(&AnswerPermission {
        allow: false,
        message: Some("non".into()),
        ..base
    });
    assert_eq!(denied["response"]["behavior"], "deny");
    assert_eq!(denied["response"]["message"], "non");
}

#[test]
fn a_link_to_claude_ai_says_only_what_it_needs() {
    let mut a = agent();
    let off = a.remote(&RemoteLink {
        agent: "a1".into(),
        enabled: false,
        name: "x".into(),
        keep: true,
        reattach: Some("s".into()),
    });
    assert_eq!(
        parsed(&off.writes[0])["request"],
        json!({ "subtype": "remote_control", "enabled": false })
    );
    let on = a.remote(&RemoteLink {
        agent: "a1".into(),
        enabled: true,
        name: "agent".into(),
        keep: true,
        reattach: Some("s".into()),
    });
    assert_eq!(
        parsed(&on.writes[0])["request"],
        json!({ "subtype": "remote_control", "enabled": true, "name": "agent",
                "keep_session_on_exit": true, "reattach_session_id": "s" })
    );
}

#[test]
fn a_message_streamed_then_whole_keeps_its_rows() {
    let mut t = Translator::default();
    let mut out = Vec::new();
    let stream = |event: Value| json!({ "type": "stream_event", "event": event });
    for f in [
        stream(
            json!({ "type": "message_start", "message": { "id": "m1", "model": "claude-opus-5-5",
            "usage": { "input_tokens": 3 } } }),
        ),
        stream(
            json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "text" } }),
        ),
        stream(json!({ "type": "content_block_delta", "index": 0,
            "delta": { "type": "text_delta", "text": "Bon" } })),
        stream(json!({ "type": "content_block_start", "index": 1,
            "content_block": { "type": "tool_use", "id": "t1", "name": "Bash" } })),
        stream(json!({ "type": "message_delta", "usage": { "output_tokens": 9 } })),
        json!({ "type": "assistant", "uuid": "e1", "message": { "id": "m1", "content": [
            { "type": "text", "text": "Bonjour" },
            { "type": "tool_use", "id": "t1", "name": "Bash", "input": { "command": "ls" } },
        ] } }),
    ] {
        t.frame(&f, "fr", &mut out);
    }
    assert_eq!(t.main_model, "claude-opus-5-5");
    assert_eq!(
        out,
        vec![
            Item::Message {
                id: "m1".into(),
                model: "claude-opus-5-5".into(),
                usage: json!({ "input_tokens": 3 }),
                main: true,
            },
            Item::Delta {
                id: "m1:0".into(),
                text: String::new(),
                thinking: false,
                parent: None,
            },
            Item::Delta {
                id: "m1:0".into(),
                text: "Bon".into(),
                thinking: false,
                parent: None,
            },
            Item::Tool {
                id: "t1".into(),
                name: "Bash".into(),
                input: json!({}),
                parent: None,
                unplanned: true,
            },
            Item::MessageOutput {
                id: "m1".into(),
                output: 9,
            },
            Item::Entry {
                entry: Some("e1".into()),
            },
            Item::Text {
                text: "Bonjour".into(),
                parent: None,
                id: Some("m1:0".into()),
            },
            Item::Call {
                id: "t1".into(),
                name: "Bash".into(),
                input: json!({ "command": "ls" }),
                parent: None,
            },
        ]
    );
}

#[test]
fn a_turn_counts_what_each_model_read_since_the_last_one() {
    let mut t = Translator::default();
    let result = |input: u64, cost: f64| {
        json!({ "type": "result", "subtype": "success", "is_error": false, "result": "ok",
            "duration_ms": 12, "uuid": "u1",
            "modelUsage": { "opus": { "inputTokens": input, "outputTokens": 1, "costUSD": cost,
                "contextWindow": 200000, "canonicalModel": "claude-opus-5-5" } } })
    };
    let mut out = Vec::new();
    t.frame(&result(10, 0.5), "fr", &mut out);
    t.frame(&result(25, 0.75), "fr", &mut out);
    let Item::Result {
        tokens,
        cost,
        models,
        window,
        failed,
        reply,
        ..
    } = &out[1]
    else {
        panic!("{out:?}");
    };
    assert_eq!(*tokens, 15);
    assert_eq!(*cost, 0.25);
    assert_eq!(models[0].model, "claude-opus-5-5");
    assert_eq!(*window, Some(200000));
    assert!(!failed);
    assert_eq!(reply, "ok");
}

#[test]
fn a_turn_stopped_by_a_limit_or_a_lost_sign_in_says_so() {
    let mut t = Translator::default();
    let mut out = Vec::new();
    t.frame(
        &json!({ "type": "rate_limit_event", "rate_limit_info": { "status": "rejected",
            "resetsAt": 1000, "rateLimitType": "seven_day_opus", "isUsingOverage": true,
            "unifiedWindows": { "five_hour": { "utilization": 0.5, "resetsAt": 2 },
                "seven_day_overage_included": { "utilization": 1.0, "resetsAt": 3 } } } }),
        "fr",
        &mut out,
    );
    assert_eq!(
        out,
        vec![
            Item::Limit { scoped: true },
            Item::Quota {
                meters: json!([
                    { "minutes": 300, "pct": 50.0, "resetsAt": 2000 },
                    { "minutes": 10080, "pct": 100.0, "resetsAt": 3000, "model": "" },
                ])
            },
            Item::Overage { on: true },
        ]
    );
    out.clear();
    t.frame(
        &json!({ "type": "assistant", "error": "rate_limit",
            "message": { "content": [{ "type": "text", "text": "limite" }] } }),
        "fr",
        &mut out,
    );
    t.frame(
        &json!({ "type": "result", "subtype": "error_during_execution", "is_error": true }),
        "fr",
        &mut out,
    );
    let Item::Result {
        limited,
        resets,
        failure,
        ..
    } = &out[1]
    else {
        panic!("{out:?}");
    };
    assert!(limited);
    assert_eq!(*resets, Some(1_000_000));
    assert_eq!(failure, "error_during_execution");
    // A sign-in lost: no error in red before its turn says it.
    let lost = "OAuth session expired and could not be refreshed";
    assert!(items(json!({ "type": "assistant", "error": "auth",
        "message": { "content": [{ "type": "text", "text": lost }] } }))
    .is_empty());
    let Item::Result { sign_in, .. } = &items(json!({
        "type": "result", "subtype": "error", "is_error": true, "result": lost,
    }))[0] else {
        panic!();
    };
    assert!(sign_in);
}

#[test]
fn a_task_of_the_foreground_shows_on_its_row_and_one_in_the_background_as_an_event() {
    let mut t = Translator::default();
    let mut out = Vec::new();
    for (id, bg) in [("fore", false), ("back", true)] {
        t.frame(
            &json!({ "type": "system", "subtype": "task_started", "task_id": id, "is_backgrounded": bg }),
            "fr",
            &mut out,
        );
    }
    out.clear();
    t.frame(
        &json!({ "type": "system", "subtype": "task_notification", "task_id": "fore" }),
        "fr",
        &mut out,
    );
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(matches!(out[0], Item::Tasks { .. }));
    out.clear();
    t.frame(
        &json!({ "type": "system", "subtype": "task_notification", "task_id": "back",
            "status": "completed", "summary": "fini", "uuid": "u1" }),
        "fr",
        &mut out,
    );
    let Item::Event {
        id, source, text, ..
    } = &out[1]
    else {
        panic!("{out:?}");
    };
    assert_eq!(id.as_deref(), Some("u1"));
    assert_eq!(source, "task");
    assert!(text.contains("<task-id>back</task-id>"));
}

#[test]
fn what_claude_code_asks_goes_on_a_card_and_the_rest_is_answered_here() {
    let asked = items(
        json!({ "type": "control_request", "request_id": "r1", "request": {
        "subtype": "can_use_tool", "tool_name": "Bash", "tool_use_id": "c1",
        "input": { "command": "ls" }, "permission_suggestions": [{ "type": "addRules" }],
        "default_to_no": true, "agent_id": "t1", "decision_reason": "pourquoi" } }),
    );
    assert_eq!(
        asked,
        vec![Item::Ask {
            request: "r1".into(),
            call: "c1".into(),
            tool: "Bash".into(),
            input: json!({ "command": "ls" }),
            title: None,
            description: None,
            reason: Some("pourquoi".into()),
            suggestions: json!([{ "type": "addRules" }]),
            no_always: false,
            default_no: true,
            task: Some("t1".into()),
        }]
    );
    let written = |subtype: &str| {
        let got = items(json!({ "type": "control_request", "request_id": "r2",
            "request": { "subtype": subtype } }));
        match &got[0] {
            Item::Write { line } => parsed(line)["response"].clone(),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(written("elicitation")["response"]["action"], "decline");
    assert_eq!(written("autre")["error"], "unsupported request: autre",);
    assert_eq!(
        items(json!({ "type": "control_cancel_request", "request_id": "r3" })),
        vec![Item::Answered {
            request: "r3".into(),
            decision: None,
            answers: None,
        }]
    );
}

#[test]
fn a_replay_says_who_it_comes_from_and_a_local_command_its_output() {
    assert_eq!(
        items(json!({ "type": "user", "isReplay": true, "uuid": "u1",
            "origin": { "kind": "peer", "from": "a2" },
            "message": { "content": [
                { "type": "text", "text": "Bonjour" }, { "type": "image" },
                { "type": "document", "title": "a.pdf" },
            ] } })),
        vec![Item::Replayed {
            id: Some("u1".into()),
            text: "Bonjour".into(),
            origin: Some("peer".into()),
            from: Some("a2".into()),
            synthetic: false,
            images: 1,
            files: vec!["a.pdf".into()],
        }]
    );
    assert_eq!(
        items(json!({ "type": "user", "uuid": "u2", "message": {
            "content": "<local-command-stdout>ok</local-command-stdout>" } })),
        vec![Item::Output {
            id: Some("u2:out0".into()),
            level: "output".into(),
            text: "ok".into(),
        }]
    );
}

#[test]
fn a_tools_result_says_the_file_it_changed() {
    assert_eq!(
        items(
            json!({ "type": "user", "uuid": "u1", "parent_tool_use_id": null,
            "tool_use_result": { "structuredPatch": [], "filePath": "src/a.rs" },
            "message": { "content": [{ "type": "tool_result", "tool_use_id": "c1",
                "content": [{ "text": "fait" }] }] } })
        ),
        vec![
            Item::Entry {
                entry: Some("u1".into())
            },
            Item::Touched {
                path: "src/a.rs".into()
            },
            Item::ToolResult {
                id: "c1".into(),
                output: "fait".into(),
                error: false,
                result: json!({ "structuredPatch": [], "filePath": "src/a.rs" }),
            },
        ]
    );
}

#[test]
fn a_new_conversation_is_told_in_the_language_of_the_app() {
    let got = |lang: &str| {
        let mut out = Vec::new();
        Translator::default().frame(
            &json!({ "type": "system", "subtype": "init", "session_id": "s1" }),
            lang,
            &mut out,
        );
        out
    };
    assert_eq!(
        got("en"),
        vec![Item::Conversation {
            session: "s1".into(),
            notice: "New Claude conversation (context cleared)".into(),
        }]
    );
}
