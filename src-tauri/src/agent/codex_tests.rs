use super::*;

fn rt() -> AgentRt {
    let dir = crate::paths::test_dir("esc37-codex-translate");
    AgentRt::new(
        AgentMeta {
            id: new_id(),
            engine: EngineId::CODEX,
            cwd: "C:/p".into(),
            mode: "default".into(),
            session_id: Some("thr".into()),
            ..Default::default()
        },
        &dir,
    )
}

fn frame(a: &mut AgentRt, method: &str, params: Value) -> Effects {
    let mut fx = Effects::default();
    a.handle_frame(&json!({ "method": method, "params": params }), &mut fx);
    fx
}

fn started(a: &mut AgentRt, turn: &str) {
    frame(
        a,
        "turn/started",
        json!({ "threadId": "thr", "turn": { "id": turn } }),
    );
}

fn item(a: &mut AgentRt, done: bool, item: Value) -> Effects {
    let method = if done {
        "item/completed"
    } else {
        "item/started"
    };
    frame(
        a,
        method,
        json!({ "threadId": "thr", "turnId": "t1", "item": item }),
    )
}

fn completed(a: &mut AgentRt, turn: Value) -> Effects {
    frame(
        a,
        "turn/completed",
        json!({ "threadId": "thr", "turn": turn }),
    )
}

fn last(a: &mut AgentRt, kind: &str) -> Value {
    a.conv
        .items()
        .into_iter()
        .rev()
        .find(|i| i["kind"] == kind)
        .unwrap_or_default()
}

#[test]
fn its_tools_take_claude_codes_names_and_a_change_of_file_its_lines() {
    let mut a = rt();
    started(&mut a, "t1");
    let change = json!({
        "path": "C:/p/src/app.ts", "kind": { "type": "update", "move_path": null },
        "diff": "@@ -1,2 +1,2 @@\n-const a = 1;\n+const a = 2;\n+const b = 3;\n keep\n",
    });
    item(
        &mut a,
        false,
        json!({ "type": "fileChange", "id": "f1", "changes": [change], "status": "inProgress" }),
    );
    let fx = item(
        &mut a,
        true,
        json!({ "type": "fileChange", "id": "f1", "changes": [change], "status": "completed" }),
    );
    assert!(fx.files_changed);
    let edit = a.conv.get("f1").cloned().unwrap();
    assert_eq!(edit["name"], "Edit");
    assert_eq!(edit["status"], "ok");
    assert_eq!(
        (
            edit["result"]["add"].as_u64(),
            edit["result"]["del"].as_u64()
        ),
        (Some(2), Some(1))
    );
    assert_eq!(edit["result"]["patch"][0]["oldStart"], 1);
    assert_eq!(a.meta.touched_files, ["src/app.ts"]);
    // A new file is a Write, all its lines added.
    let add = json!({ "path": "C:/p/b.txt", "kind": { "type": "add" }, "diff": "un\ndeux\n" });
    item(
        &mut a,
        true,
        json!({ "type": "fileChange", "id": "f2", "changes": [add], "status": "completed" }),
    );
    let write = a.conv.get("f2").cloned().unwrap();
    assert_eq!(write["name"], "Write");
    assert_eq!(write["result"]["add"], 2);
    // A command, its words as the model wrote them (not wrapped in pwsh).
    item(
        &mut a,
        false,
        json!({
            "type": "commandExecution", "id": "c1", "command": "\"pwsh.exe\" -Command 'ls'",
            "commandActions": [{ "type": "listFiles", "command": "ls" }], "status": "inProgress",
        }),
    );
    assert_eq!(a.conv.get("c1").unwrap()["input"]["command"], "ls");
    item(
        &mut a,
        true,
        json!({
            "type": "commandExecution", "id": "c1", "command": "x", "commandActions": [{ "command": "ls" }],
            "status": "failed", "exitCode": 1, "aggregatedOutput": "nope",
        }),
    );
    let bash = a.conv.get("c1").cloned().unwrap();
    assert_eq!(
        (bash["name"].as_str(), bash["status"].as_str()),
        (Some("Bash"), Some("error"))
    );
    assert_eq!(bash["result"]["text"], "nope");
    // An MCP tool, a web search.
    item(
        &mut a,
        true,
        json!({
            "type": "mcpToolCall", "id": "m1", "server": "escouade", "tool": "get_board",
            "arguments": { "project": "x" }, "status": "completed", "result": { "content": [{ "type": "text", "text": "ok" }] },
        }),
    );
    assert_eq!(
        a.conv.get("m1").unwrap()["name"],
        "mcp__escouade__get_board"
    );
    assert_eq!(a.conv.get("m1").unwrap()["result"]["text"], "ok");
    item(
        &mut a,
        true,
        json!({ "type": "webSearch", "id": "w1", "query": "rust serde" }),
    );
    assert_eq!(a.conv.get("w1").unwrap()["name"], "WebSearch");
    assert_eq!(a.conv.get("w1").unwrap()["input"]["query"], "rust serde");
}

#[test]
fn a_reply_comes_by_deltas_then_whole_and_ends_the_turn_with_its_text() {
    let mut a = rt();
    started(&mut a, "t1");
    assert_eq!(a.meta.status, AgentStatus::Running);
    item(
        &mut a,
        false,
        json!({ "type": "agentMessage", "id": "msg1", "text": "", "phase": "final_answer" }),
    );
    frame(
        &mut a,
        "item/agentMessage/delta",
        json!({ "threadId": "thr", "itemId": "msg1", "delta": "Bon" }),
    );
    frame(
        &mut a,
        "item/agentMessage/delta",
        json!({ "threadId": "thr", "itemId": "msg1", "delta": "jour" }),
    );
    assert_eq!(a.conv.get("msg1").unwrap()["text"], "Bonjour");
    item(
        &mut a,
        true,
        json!({ "type": "agentMessage", "id": "msg1", "text": "Bonjour", "phase": "final_answer" }),
    );
    assert_eq!(a.conv.get("msg1").unwrap()["streaming"], false);
    let fx = completed(
        &mut a,
        json!({ "id": "t1", "status": "completed", "durationMs": 12 }),
    );
    assert_eq!(fx.turn_end, Some(TurnEnd::Finished("Bonjour".into())));
    assert_eq!(a.meta.status, AgentStatus::Done);
    assert_eq!(a.meta.last_entry.as_deref(), Some("t1"));
    assert_eq!(last(&mut a, "turn")["engine"], "codex");
}

#[test]
fn a_voice_conversation_has_its_passage_to_read_aloud_apart_as_with_claude_code() {
    let mut a = rt();
    started(&mut a, "t1");
    item(
        &mut a,
        false,
        json!({ "type": "agentMessage", "id": "msg1", "text": "" }),
    );
    item(
        &mut a,
        true,
        json!({
            "type": "agentMessage", "id": "msg1",
            "text": "J'ai corrigé `a.rs`.\n\n<spoken>C'est corrigé, **les tests passent**.</spoken>",
        }),
    );
    let text = a.conv.get("msg1").cloned().unwrap();
    assert_eq!(text["text"], "J'ai corrigé `a.rs`.");
    assert_eq!(text["spoken"], "C'est corrigé, les tests passent.");
    // Never a subagent's.
    frame(
        &mut a,
        "item/started",
        json!({ "threadId": "thr", "item": {
        "type": "subAgentActivity", "id": "call1", "kind": "started", "agentThreadId": "child", "agentPath": "/root/x",
    } }),
    );
    frame(
        &mut a,
        "item/completed",
        json!({ "threadId": "child", "item": {
        "type": "agentMessage", "id": "msg2", "text": "<spoken>non</spoken>",
    } }),
    );
    let sub = a.conv.get("msg2").cloned().unwrap();
    assert_eq!(sub["parent"], "call1");
    assert!(sub["spoken"].is_null());
}

#[test]
fn a_subagents_thread_shows_under_its_task_and_its_report_ends_it() {
    let mut a = rt();
    started(&mut a, "t1");
    item(
        &mut a,
        false,
        json!({
            "type": "subAgentActivity", "id": "call1", "kind": "started", "agentThreadId": "child",
            "agentPath": "/root/cause_du_bug", "model": "gpt-6-luna",
        }),
    );
    let task = a.conv.get("call1").cloned().unwrap();
    assert_eq!(
        (task["name"].as_str(), task["input"]["description"].as_str()),
        (Some("Task"), Some("cause du bug"))
    );
    // Its own turn's end is not the agent's.
    frame(
        &mut a,
        "turn/started",
        json!({ "threadId": "child", "turn": { "id": "ct" } }),
    );
    frame(
        &mut a,
        "item/completed",
        json!({ "threadId": "child", "item": { "type": "agentMessage", "id": "r", "text": "Trouvé." } }),
    );
    let fx = frame(
        &mut a,
        "turn/completed",
        json!({ "threadId": "child", "turn": { "id": "ct", "status": "completed" } }),
    );
    assert!(fx.turn_end.is_none());
    assert_eq!(a.meta.status, AgentStatus::Running);
    item(
        &mut a,
        true,
        json!({ "type": "subAgentActivity", "id": "done1", "kind": "completed", "agentThreadId": "child" }),
    );
    let task = a.conv.get("call1").cloned().unwrap();
    assert_eq!(
        (task["status"].as_str(), task["result"]["text"].as_str()),
        (Some("ok"), Some("Trouvé."))
    );
    let plan = a.meta.plan.clone().unwrap();
    assert_eq!(plan.agents.len(), 1);
    assert_eq!(plan.agents[0].status, crate::plan::RunStatus::Done);
}

#[test]
fn the_plan_tools_steps_are_the_plans_tasks() {
    let mut a = rt();
    started(&mut a, "t1");
    frame(
        &mut a,
        "turn/plan/updated",
        json!({ "threadId": "thr", "plan": [
        { "step": "Écrire", "status": "completed" },
        { "step": "Commiter", "status": "inProgress" },
        { "step": "Vérifier", "status": "pending" },
    ] }),
    );
    let tasks = a.meta.plan.clone().unwrap().tasks;
    let states: Vec<_> = tasks.iter().map(|t| (t.title.clone(), t.status)).collect();
    use crate::plan::TaskStatus::*;
    assert_eq!(
        states,
        [
            ("Écrire".into(), Done),
            ("Commiter".into(), InProgress),
            ("Vérifier".into(), Pending)
        ]
    );
}

#[test]
fn the_plan_of_the_mode_plan_waits_for_its_approval_before_the_turn_ends() {
    let mut a = rt();
    started(&mut a, "t1");
    item(
        &mut a,
        true,
        json!({ "type": "plan", "id": "t1-plan", "text": "## Le plan\n- Faire" }),
    );
    let fx = completed(&mut a, json!({ "id": "t1", "status": "completed" }));
    assert!(
        fx.turn_end.is_none(),
        "the turn goes on once the plan is approved"
    );
    assert_eq!(a.meta.status, AgentStatus::Waiting);
    let card = last(&mut a, "permission");
    assert_eq!(card["toolName"], "ExitPlanMode");
    assert_eq!(card["input"]["plan"], "## Le plan\n- Faire");
    assert!(crate::engine::codex_is_plan_approval(
        card["id"].as_str().unwrap()
    ));
    assert_eq!(a.waiting().len(), 1);
    // Its work is the turn's next part, which ends it.
    started(&mut a, "t2");
    let fx = completed(&mut a, json!({ "id": "t2", "status": "completed" }));
    assert!(matches!(fx.turn_end, Some(TurnEnd::Finished(_))));
}

#[test]
fn a_question_takes_the_card_of_claude_codes_with_the_ids_of_codex() {
    let mut a = rt();
    started(&mut a, "t1");
    let mut fx = Effects::default();
    a.handle_frame(&json!({ "method": "item/tool/requestUserInput", "id": 4, "params": {
        "threadId": "thr", "itemId": "call9",
        "questions": [{ "id": "lang", "header": "Langage", "question": "Quel langage ?", "isOther": true,
            "options": [{ "label": "Python", "description": "Simple" }] }],
    } }), &mut fx);
    let card = a.conv.get("4").cloned().unwrap();
    assert_eq!(card["kind"], "question");
    assert_eq!(card["questions"][0]["id"], "lang");
    assert_eq!(card["questions"][0]["options"][0]["label"], "Python");
    assert_eq!(a.meta.status, AgentStatus::Waiting);
    assert!(fx.notify.is_some());
    // Closed by Codex (an interrupted turn): its card goes.
    frame(
        &mut a,
        "serverRequest/resolved",
        json!({ "threadId": "thr", "requestId": 4 }),
    );
    assert_eq!(a.conv.get("4").unwrap()["cancelled"], true);
    assert!(a.waiting().is_empty());
}

#[test]
fn an_approval_is_a_card_with_its_reason_for_this_session_and_a_change_its_diff() {
    let mut a = rt();
    started(&mut a, "t1");
    let mut fx = Effects::default();
    a.handle_frame(&json!({ "method": "item/commandExecution/requestApproval", "id": 0, "params": {
        "threadId": "thr", "itemId": "c1", "reason": "Autorisez-vous git ?", "cwd": "C:/p",
        "command": "pwsh -Command 'git commit -m x'", "commandActions": [{ "command": "git commit -m x" }],
    } }), &mut fx);
    let card = a.conv.get("0").cloned().unwrap();
    assert_eq!(card["toolName"], "Bash");
    assert_eq!(card["input"]["command"], "git commit -m x");
    assert_eq!(card["reason"], "Autorisez-vous git ?");
    assert_eq!(
        (card["canAlways"].as_bool(), card["alwaysScope"].as_str()),
        (Some(true), Some("session"))
    );
    // The list of the agents offers it too.
    let view = &a.view().requests[0];
    assert_eq!(
        (view.can_always, view.always_scope.as_deref()),
        (true, Some("session"))
    );
    assert!(card["description"]
        .as_str()
        .unwrap()
        .contains("bac à sable"));
    // A change of a file shows its diff.
    let change = json!({ "path": "C:/p/a.ts", "kind": { "type": "update" }, "diff": "@@ -1 +1 @@\n-a\n+b\n" });
    item(
        &mut a,
        false,
        json!({ "type": "fileChange", "id": "f1", "changes": [change], "status": "inProgress" }),
    );
    a.handle_frame(
        &json!({ "method": "item/fileChange/requestApproval", "id": 1, "params": {
        "threadId": "thr", "itemId": "f1", "reason": null,
    } }),
        &mut fx,
    );
    let card = a.conv.get("1").cloned().unwrap();
    assert_eq!(card["toolName"], "Edit");
    assert_eq!(card["patch"][0]["lines"], json!(["-a", "+b"]));
    // More permissions, said in words.
    a.handle_frame(&json!({ "method": "item/permissions/requestApproval", "id": 2, "params": {
        "threadId": "thr", "itemId": "x", "cwd": "C:/p", "permissions": { "network": { "enabled": true } },
    } }), &mut fx);
    assert!(a.conv.get("2").unwrap()["description"]
        .as_str()
        .unwrap()
        .contains("réseau"));
    assert_eq!(a.waiting().len(), 3);
}

#[test]
fn folders_are_the_same_whatever_their_slashes_and_case() {
    assert!(same_dir(r"C:\p\wt\", "c:/p/wt"));
}

#[test]
fn the_mode_follows_what_codex_says_of_the_threads_settings() {
    let mut a = rt();
    frame(
        &mut a,
        "thread/settings/updated",
        json!({ "threadId": "thr", "threadSettings": {
        "approvalPolicy": "on-request", "sandboxPolicy": { "type": "readOnly" },
        "collaborationMode": { "mode": "plan" },
    } }),
    );
    assert_eq!(a.meta.mode, "plan");
    frame(
        &mut a,
        "thread/settings/updated",
        json!({ "threadId": "thr", "threadSettings": {
        "approvalPolicy": "never", "sandboxPolicy": { "type": "dangerFullAccess" },
        "collaborationMode": { "mode": "default" },
    } }),
    );
    assert_eq!(a.meta.mode, "bypassPermissions");
}

#[test]
fn a_turn_stopped_by_the_usage_limit_tells_when_it_resets() {
    let mut a = rt();
    started(&mut a, "t1");
    let fx = frame(
        &mut a,
        "account/rateLimits/updated",
        json!({ "rateLimits": {
        "primary": { "usedPercent": 100, "windowDurationMins": 300, "resetsAt": 1800000000 },
        "secondary": null, "rateLimitReachedType": "rate_limit_reached",
    } }),
    );
    assert_eq!(fx.rate.unwrap()[0].minutes, Some(300));
    let fx = completed(
        &mut a,
        json!({ "id": "t1", "status": "failed", "error": {
        "message": "You've hit your usage limit.", "codexErrorInfo": "usageLimitExceeded",
    } }),
    );
    assert_eq!(fx.turn_end, Some(TurnEnd::Limited));
    assert_eq!(fx.limited, Some(Some(1_800_000_000_000)));
    let turn = last(&mut a, "turn");
    assert_eq!(
        (turn["limited"].as_bool(), turn["isError"].as_bool()),
        (Some(true), Some(true))
    );
    assert_eq!(a.meta.status, AgentStatus::Done);
}

#[test]
fn a_message_sent_while_a_turn_runs_goes_into_it_and_is_queued_behind_nothing() {
    let mut a = rt();
    let mut fx = Effects::default();
    a.push_user("u1", "première", 0, &[], &mut fx);
    started(&mut a, "t1");
    assert!(!a.push_user("u2", "en cours de route", 0, &[], &mut fx));
    assert_eq!(a.conv.get("u2").unwrap()["queued"], false);
    assert_eq!(a.meta.status, AgentStatus::Running);
}

#[test]
fn an_unknown_message_item_or_field_of_a_newer_codex_leaves_the_turn_going() {
    let mut a = rt();
    started(&mut a, "t1");
    frame(
        &mut a,
        "thread/somethingNew",
        json!({ "threadId": "thr", "x": 1 }),
    );
    item(&mut a, true, json!({ "type": "hologram", "id": "h1" }));
    let mut fx = Effects::default();
    a.handle_frame(&json!({ "jsonrpc": "2.0", "weird": true }), &mut fx);
    item(
        &mut a,
        true,
        json!({ "type": "agentMessage", "id": "m", "text": "Fini", "newField": [1, 2] }),
    );
    let fx = completed(
        &mut a,
        json!({ "id": "t1", "status": "completed", "brandNew": {} }),
    );
    assert_eq!(fx.turn_end, Some(TurnEnd::Finished("Fini".into())));
    assert!(a.conv.get("h1").is_none());
}

#[test]
fn a_turn_codex_refused_to_start_ends_in_error_with_its_reason() {
    let mut a = rt();
    let mut fx = Effects::default();
    a.push_user("u1", "bonjour", 0, &[], &mut fx);
    let fx = frame(
        &mut a,
        "error",
        json!({
            "error": { "message": "a turn is already running" }, "willRetry": false, "refused": "turn/start",
        }),
    );
    assert_eq!(
        fx.turn_end,
        Some(TurnEnd::Error("a turn is already running".into()))
    );
    assert_eq!(a.meta.status, AgentStatus::Error);
}

#[test]
fn a_thread_that_could_not_be_resumed_is_a_new_conversation_told_so() {
    let mut a = rt();
    let fx = frame(
        &mut a,
        "thread/started",
        json!({ "thread": { "id": "thr-2", "parentThreadId": null } }),
    );
    assert!(fx.save);
    assert_eq!(a.meta.session_id.as_deref(), Some("thr-2"));
    assert_eq!(last(&mut a, "notice")["level"], "warn");
    // The same thread again (the process tells it after its answer): nothing more.
    let before = a.conv.items().len();
    frame(
        &mut a,
        "thread/started",
        json!({ "thread": { "id": "thr-2" } }),
    );
    assert_eq!(a.conv.items().len(), before);
}

#[test]
fn a_decision_of_codexs_automatic_reviewer_is_told_in_the_conversation() {
    let mut a = rt();
    started(&mut a, "t1");
    frame(
        &mut a,
        "guardianWarning",
        json!({ "threadId": "thr",
        "message": "Automatic approval review approved (risk: low)" }),
    );
    let notice = last(&mut a, "notice");
    assert!(notice["text"]
        .as_str()
        .unwrap()
        .contains("Automatic approval review approved"));
}

#[test]
fn a_turns_tokens_are_its_threads_since_it_began_and_its_cost_is_estimated_from_them() {
    let mut a = rt();
    a.meta.model = "gpt-6-luna".into();
    let usage = |a: &mut AgentRt, thread: &str, input: u64, cached: u64, output: u64| {
        frame(
            a,
            "thread/tokenUsage/updated",
            json!({ "threadId": thread, "turnId": "t1", "tokenUsage": {
                "total": { "inputTokens": input, "cachedInputTokens": cached, "outputTokens": output,
                           "reasoningOutputTokens": 0, "totalTokens": input + output },
                "last": { "inputTokens": 1000, "cachedInputTokens": 0, "outputTokens": 10 },
                "modelContextWindow": 258400,
            } }),
        )
    };
    // What the thread had counted before this turn is not this turn's.
    usage(&mut a, "thr", 500_000, 0, 1_000);
    completed(&mut a, json!({ "id": "t0", "status": "completed" }));
    let before = a.meta.cost;
    started(&mut a, "t1");
    usage(&mut a, "thr", 1_500_000, 400_000, 101_000);
    // A subagent's thread counts in its parent's turn.
    usage(&mut a, "child", 1_000_000, 0, 100_000);
    assert_eq!((a.context_tokens, a.context_window), (1000, 258_400));
    let fx = completed(&mut a, json!({ "id": "t1", "status": "completed" }));
    let row = &fx.turns[0];
    assert_eq!(
        (row.input, row.cache, row.output),
        (1_600_000, 400_000, 200_000)
    );
    // gpt-6-luna: $0.10 a million of input, $0.01 cached, $0.50 of output.
    let cost = 1.6 * 0.10 + 0.4 * 0.01 + 0.2 * 0.50;
    assert!((row.cost - cost).abs() < 1e-9, "{}", row.cost);
    let turn = last(&mut a, "turn");
    assert!((turn["cost"].as_f64().unwrap() - cost).abs() < 1e-9);
    assert!((a.meta.cost - before - cost).abs() < 1e-9);
    // A model OpenAI's grid does not price (a Claude model left by a handover) costs nothing told,
    // its tokens counted all the same.
    a.meta.model = "opus".into();
    started(&mut a, "t2");
    usage(&mut a, "thr", 2_500_000, 400_000, 101_000);
    let fx = completed(&mut a, json!({ "id": "t2", "status": "completed" }));
    assert_eq!((fx.turns[0].input, fx.turns[0].cost), (1_000_000, 0.0));
}

/// What Claude Code's agent says it does with the tool `name` on `input` (cwd `C:/p`).
fn claude_says(name: &str, input: Value) -> Option<String> {
    crate::agent::tool_activity(name, &input, "C:/p")
}

#[test]
fn it_says_what_it_does_with_the_words_of_claude_code() {
    let mut a = rt();
    started(&mut a, "t1");
    let change = json!({ "path": "C:/p/src/auth/jwt.ts", "kind": { "type": "update", "move_path": null }, "diff": "" });
    item(
        &mut a,
        false,
        json!({ "type": "fileChange", "id": "f1", "changes": [change], "status": "inProgress" }),
    );
    let edit = json!({ "file_path": "C:/p/src/auth/jwt.ts" });
    assert_eq!(a.activity, claude_says("Edit", edit));
    let add = json!({ "path": "C:/p/b.txt", "kind": { "type": "add" }, "diff": "un\n" });
    item(
        &mut a,
        false,
        json!({ "type": "fileChange", "id": "f2", "changes": [add], "status": "inProgress" }),
    );
    assert_eq!(
        a.activity,
        claude_says("Write", json!({ "file_path": "C:/p/b.txt" }))
    );
    // A command that reads a file is a Read, one that searches a Grep: Claude Code has tools for them.
    item(
        &mut a,
        false,
        json!({
            "type": "commandExecution", "id": "c1", "command": "\"pwsh.exe\" -Command 'cat src/db.ts'",
            "commandActions": [{ "type": "read", "command": "cat src/db.ts", "name": "db.ts", "path": "C:/p/src/db.ts" }],
            "status": "inProgress",
        }),
    );
    assert_eq!(
        a.activity,
        claude_says("Read", json!({ "file_path": "C:/p/src/db.ts" }))
    );
    item(
        &mut a,
        false,
        json!({
            "type": "commandExecution", "id": "c2", "command": "rg -n jwt src",
            "commandActions": [{ "type": "search", "command": "rg -n jwt src", "query": "jwt", "path": "src" }],
            "status": "inProgress",
        }),
    );
    assert_eq!(a.activity, claude_says("Grep", json!({ "pattern": "jwt" })));
    // Anything else, or several actions at once, is run as Claude Code runs a command.
    item(
        &mut a,
        false,
        json!({
            "type": "commandExecution", "id": "c3", "command": "npm test",
            "commandActions": [{ "type": "unknown", "command": "npm test" }], "status": "inProgress",
        }),
    );
    assert_eq!(
        a.activity,
        claude_says("Bash", json!({ "command": "npm test" }))
    );
    item(
        &mut a,
        false,
        json!({
            "type": "commandExecution", "id": "c4", "command": "x",
            "commandActions": [
                { "type": "read", "command": "cat a.ts", "name": "a.ts", "path": "C:/p/a.ts" },
                { "type": "read", "command": "cat b.ts", "name": "b.ts", "path": "C:/p/b.ts" },
            ],
            "status": "inProgress",
        }),
    );
    assert_eq!(
        a.activity,
        claude_says("Bash", json!({ "command": "cat a.ts && cat b.ts" }))
    );
    // What it is told by Claude Code's words: a verb, then what it acts on, relative to its folder.
    assert!(a
        .activity
        .as_deref()
        .unwrap()
        .ends_with(" cat a.ts && cat b.ts"));
}
