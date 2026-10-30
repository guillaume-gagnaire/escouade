use super::*;

/// The worktree of the real runs (`tests/fixtures/copilot/`).
const WT: &str = "C:\\Users\\user\\AppData\\Local\\Temp\\esc23\\wt-agent1";

/// An agent of Copilot in `cwd`, in `mode`.
fn agent(cwd: &str, mode: &str) -> AgentRt {
    let meta = AgentMeta {
        id: new_id(),
        cwd: cwd.into(),
        engine: EngineId::COPILOT,
        model: "gpt-6-luna".into(),
        mode: mode.into(),
        ..Default::default()
    };
    AgentRt::new(
        meta,
        // A folder of its own: the tests run at once, and `test_dir` empties the one it names.
        &crate::paths::test_dir(&format!("esc39-cp-conv-{}", new_id())),
    )
}

fn feed(rt: &mut AgentRt, frames: &[Value]) -> Vec<Effects> {
    frames
        .iter()
        .map(|f| {
            let mut fx = Effects::default();
            rt.copilot_frame(f, &mut fx);
            fx
        })
        .collect()
}

/// The recorded stream `file`, every line (what was said to Copilot is left out by the translation).
fn recorded(file: &str) -> Vec<Value> {
    crate::engine::copilot::bench::stream(file)
}

/// The stream `file` translated for an agent in `mode` in the worktree of the real runs.
fn replay(file: &str, mode: &str) -> (AgentRt, Vec<Effects>) {
    let mut rt = agent(WT, mode);
    let fxs = feed(&mut rt, &recorded(file));
    (rt, fxs)
}

fn items_of<'a>(items: &'a [Value], kind: &str) -> Vec<&'a Value> {
    items.iter().filter(|i| i["kind"] == kind).collect()
}

const S: &str = "ses-own";

fn ev(kind: &str, data: Value) -> Value {
    json!({ "method": "session.event", "params": { "sessionId": S, "event": {
        "id": new_id(), "type": kind, "data": data,
    } } })
}

fn ev_of(agent: &str, kind: &str, data: Value) -> Value {
    json!({ "method": "session.event", "params": { "sessionId": S, "event": {
        "id": new_id(), "type": kind, "data": data, "agentId": agent,
    } } })
}

fn start(rt: &mut AgentRt) {
    feed(
        rt,
        &[json!({ "method": "escouade.session", "params": { "sessionId": S, "start": "new" } })],
    );
}

#[test]
fn a_first_turn_reads_edits_and_runs_with_a_card_for_each_permission_and_its_diff() {
    let (mut rt, fxs) = replay("first-turn.jsonl", "default");
    let items = rt.conv.items();
    assert_eq!(
        rt.meta.session_id.as_deref(),
        Some("6cdfa766-5a59-4c4c-97fd-85b10d4f0579")
    );
    let tools: Vec<&str> = items_of(&items, "tool")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(tools, ["Read", "Bash", "Edit", "Bash", "Bash"], "{tools:?}");
    assert!(items_of(&items, "tool").iter().all(|t| t["status"] == "ok"));
    // No process in a replay: each permission has its card, closed by Copilot's own answer.
    let cards = items_of(&items, "permission");
    let names: Vec<&str> = cards
        .iter()
        .filter_map(|c| c["toolName"].as_str())
        .collect();
    assert_eq!(names, ["Read", "Bash", "Edit", "Bash", "Bash"]);
    let write = cards.iter().find(|c| c["toolName"] == "Edit").unwrap();
    assert!(
        write["input"]["file_path"]
            .as_str()
            .unwrap()
            .ends_with("calc.js"),
        "{write}"
    );
    let patch = write["patch"].to_string();
    assert!(
        patch.contains("return a + b"),
        "the diff of the write: {patch}"
    );
    assert!(cards.iter().all(|c| c["decision"] == "allow"));
    let text = items_of(&items, "text");
    assert!(text.last().unwrap()["text"]
        .as_str()
        .unwrap()
        .contains("node calc.js"));
    // The file changed is among those it touched, and the turn says its tokens and cost.
    assert!(rt.meta.touched_files.iter().any(|f| f.ends_with("calc.js")));
    let turn = items_of(&items, "turn")[0].clone();
    assert_eq!(turn["isError"], false);
    // The time it took, from the start of its turn (Copilot tells none of its own).
    assert!(turn["durationMs"].is_i64(), "{turn}");
    assert!(turn["tokens"].as_u64().unwrap() > 50_000);
    let cost = turn["cost"].as_f64().unwrap();
    assert!((cost - 0.002_272_275).abs() < 1e-9, "{cost}");
    assert!(fxs.iter().any(|f| f.turn_end.is_some()));
    assert_eq!(rt.meta.status, AgentStatus::Done);
    assert!(rt.meta.last_entry.is_some(), "where a copy starts from");
}

#[test]
fn a_full_session_asks_writes_for_the_session_plans_and_runs_a_subagent() {
    let (mut rt, _) = replay("full-turn.jsonl", "default");
    let items = rt.conv.items();
    // The question, once, answered by the first answer that came.
    let questions = items_of(&items, "question");
    assert_eq!(
        questions.len(),
        4,
        "the ask_user of the first turn, two of the plan and the form"
    );
    assert_eq!(
        questions[0]["questions"][0]["question"],
        "Quelle couleur préfères-tu ?"
    );
    assert_eq!(questions[0]["questions"][0]["options"][1]["label"], "bleu");
    assert_eq!(
        questions[0]["answers"]["Quelle couleur préfères-tu ?"],
        "bleu"
    );
    // The write approved for the session; the address refused.
    let cards = items_of(&items, "permission");
    let couleur = cards
        .iter()
        .find(|c| c["toolName"] == "Write")
        .expect("couleur.txt created");
    assert_eq!(couleur["decision"], "always");
    assert_eq!(couleur["alwaysScope"], "session");
    let web = cards.iter().find(|c| c["toolName"] == "WebFetch").unwrap();
    assert_eq!(web["input"]["url"], "https://example.com");
    assert_eq!(web["decision"], "deny");
    // The MCP tools, under Claude Code's names, with what they do.
    let mcp = cards
        .iter()
        .find(|c| c["toolName"] == "mcp__essai__ping")
        .expect("the ping");
    assert!(mcp["description"].as_str().unwrap().contains("essai"));
    assert!(items_of(&items, "tool")
        .iter()
        .any(|t| t["name"] == "mcp__essai__formulaire"));
    // The form, a card of its fields.
    let form = questions
        .iter()
        .find(|q| q["form"].is_object())
        .expect("the form's card");
    assert_eq!(form["form"]["message"], "Qui es-tu ?");
    assert_eq!(form["questions"][0]["question"], "Prénom");
    assert_eq!(form["questions"][1]["field"], "number");
    assert_eq!(
        form["cancelled"], true,
        "answered by the trial's client, not here"
    );
    // The subagent, under its Task, its tools under it.
    let task = items_of(&items, "tool")
        .into_iter()
        .find(|t| t["name"] == "Task")
        .expect("the subagent's call");
    assert_eq!(task["status"], "ok");
    let call = task["id"].as_str().unwrap();
    assert!(items
        .iter()
        .any(|i| i["kind"] == "tool" && i["parent"] == call && i["name"] == "Read"));
    // The model its messages came from, not the one its start named (its definition's).
    let row = rt
        .meta
        .plan
        .as_ref()
        .unwrap()
        .agents
        .iter()
        .find(|a| a.id == call)
        .cloned();
    assert_eq!(
        row.and_then(|r| r.model).as_deref(),
        Some("mai-code-1.1-flash")
    );
    // The plan to approve, twice sent back.
    let plans: Vec<&&Value> = cards
        .iter()
        .filter(|c| c["toolName"] == "ExitPlanMode")
        .collect();
    assert_eq!(plans.len(), 2);
    assert!(plans.iter().all(|p| p["decision"] == "deny"));
    assert!(plans[0]["input"]["plan"].as_str().unwrap().len() > 20);
    // The tasks, in the plan's strip.
    let plan = rt.meta.plan.clone().expect("a plan");
    assert!(
        plan.agents.iter().any(|a| a.title.contains("README")),
        "{:?}",
        plan.agents
    );
    // The stop: its turn interrupted.
    let turns = items_of(&items, "turn");
    assert!(turns.iter().any(|t| t["interrupted"] == true));
    assert!(items.iter().any(
        |i| i["kind"] == "notice" && i["text"].as_str().is_some_and(|t| t.contains("compact"))
    ));
}

#[test]
fn the_form_of_an_mcp_server_is_a_card_of_its_fields() {
    let mut rt = agent("C:/p", "default");
    start(&mut rt);
    let mut fx = Effects::default();
    rt.push_user("u1", "formulaire", 0, &[], &mut fx);
    feed(
        &mut rt,
        &[ev(
            "elicitation.requested",
            json!({
                "requestId": "el-1", "elicitationSource": "essai", "message": "Qui es-tu ?",
                "requestedSchema": { "type": "object", "properties": {
                    "age": { "type": "number", "title": "Âge" },
                    "prenom": { "type": "string", "title": "Prénom" },
                    "abonne": { "type": "boolean", "title": "Abonné" },
                    "langue": { "type": "string", "enum": ["fr", "en"] },
                }, "required": ["prenom"] },
            }),
        )],
    );
    let card = rt.conv.get("el-1").unwrap().clone();
    assert_eq!(card["kind"], "question");
    assert_eq!(card["form"]["message"], "Qui es-tu ?");
    assert_eq!(card["form"]["source"], "essai");
    let qs = card["questions"].as_array().unwrap();
    assert_eq!(qs[0]["question"], "Prénom", "the required field first");
    assert_eq!(qs[0]["field"], "text");
    assert_eq!(qs[0]["required"], true);
    let age = qs.iter().find(|q| q["id"] == "age").unwrap();
    assert_eq!(age["field"], "number");
    let abonne = qs.iter().find(|q| q["id"] == "abonne").unwrap();
    assert_eq!(abonne["options"].as_array().unwrap().len(), 2);
    let langue = qs.iter().find(|q| q["id"] == "langue").unwrap();
    assert_eq!(langue["options"][1]["label"], "en");
    assert_eq!(rt.meta.status, AgentStatus::Waiting);
    // The list of the agents says what the form is for.
    assert_eq!(
        rt.view().requests[0].description.as_deref(),
        Some("Qui es-tu ?")
    );
}

#[test]
fn a_permission_is_offered_for_the_session_on_the_list_too() {
    let mut rt = agent("C:/p", "default");
    start(&mut rt);
    feed(
        &mut rt,
        &[ev(
            "permission.requested",
            json!({ "requestId": "pr-1", "permissionRequest": {
                "kind": "shell", "toolCallId": "c1", "fullCommandText": "npm publish",
                "intention": "Publier", "commands": [{ "identifier": "npm" }],
            } }),
        )],
    );
    let view = &rt.view().requests[0];
    assert_eq!(view.kind, "permission");
    assert_eq!(
        (view.can_always, view.always_scope.as_deref()),
        (true, Some("session"))
    );
}

#[test]
fn safe_requests_go_without_a_card_only_when_a_process_answers_them() {
    let mut rt = agent(WT, "acceptEdits");
    let read = json!({ "kind": "read", "path": format!("{WT}\\calc.js") });
    let outside = json!({ "kind": "read", "path": "C:\\Windows\\win.ini" });
    let write = json!({ "kind": "write", "fileName": format!("{WT}\\a.txt") });
    assert!(rt.cp_safe(&read));
    assert!(!rt.cp_safe(&outside));
    assert!(rt.cp_safe(&write));
    rt.meta.mode = "plan".into();
    assert!(!rt.cp_safe(&write), "nothing written in the mode Plan");
    assert!(rt.cp_safe(&read));
    rt.meta.mode = "default".into();
    assert!(rt.cp_safe(&write));
    let npm = json!({ "kind": "shell", "fullCommandText": "npm test", "commands": [{ "identifier": "npm test" }] });
    assert!(!rt.cp_safe(&npm));
    // A discussion of the Docs tab (ESC-261): its reads alone, in any mode.
    rt.meta.discussion = Some(Default::default());
    let commit = json!({ "kind": "shell", "fullCommandText": "git commit -m x", "commands": [{ "identifier": "git commit" }] });
    for mode in ["default", "acceptEdits", "auto"] {
        rt.meta.mode = mode.into();
        assert!(!rt.cp_safe(&write), "{mode}");
        assert!(!rt.cp_safe(&commit), "{mode}");
        assert!(rt.cp_safe(&read), "{mode}");
    }
    assert!(inside(
        "c:/users/user/appdata/local/temp/esc23/wt-agent1/x",
        WT
    ));
    assert!(!inside(&format!("{WT}-autre\\x"), WT));
}

#[test]
fn a_subagents_events_go_under_its_task_and_the_tasks_into_the_plan() {
    let mut rt = agent("C:/p", "default");
    start(&mut rt);
    let mut fx = Effects::default();
    rt.push_user("u1", "sous-agent", 0, &[], &mut fx);
    feed(
        &mut rt,
        &[
            json!({ "method": "escouade.todos", "params": { "sessionId": S, "todos": { "rows": [
                { "id": "a", "title": "Lire", "status": "in_progress" },
                { "id": "b", "title": "Résumer", "status": "pending" },
            ], "dependencies": [] } } }),
            ev(
                "tool.execution_start",
                json!({ "toolCallId": "t1", "toolName": "task", "arguments": { "name": "Résumé", "prompt": "Résume", "agent_type": "explore" } }),
            ),
            ev_of(
                "ag1",
                "subagent.started",
                json!({ "toolCallId": "t1", "agentName": "explore" }),
            ),
            ev_of(
                "ag1",
                "tool.execution_start",
                json!({ "toolCallId": "v1", "toolName": "view", "arguments": { "path": "C:/p/README.md" } }),
            ),
            ev_of(
                "ag1",
                "assistant.message",
                json!({ "messageId": "m1", "content": "Un projet." }),
            ),
            ev_of(
                "ag1",
                "subagent.completed",
                json!({ "toolCallId": "t1", "totalTokens": 900, "totalToolCalls": 1 }),
            ),
            ev(
                "tool.execution_complete",
                json!({ "toolCallId": "t1", "success": true, "result": { "content": "" } }),
            ),
            ev_of(
                "ghost",
                "assistant.message",
                json!({ "messageId": "m2", "content": "rien" }),
            ),
        ],
    );
    assert_eq!(rt.conv.get("v1").unwrap()["parent"], "t1");
    assert_eq!(rt.conv.get("m1").unwrap()["parent"], "t1");
    assert!(!rt.conv.contains("m2"), "an agent not known is nobody's");
    let task = rt.conv.get("t1").unwrap().clone();
    assert_eq!(task["status"], "ok");
    assert!(task["result"].to_string().contains("Un projet."));
    let plan = rt.meta.plan.clone().unwrap();
    assert!(plan.launched >= 1);
}

#[test]
fn a_mode_change_of_copilot_is_the_agents() {
    let mut rt = agent("C:/p", "plan");
    start(&mut rt);
    feed(
        &mut rt,
        &[ev(
            "session.mode_changed",
            json!({ "newMode": "interactive" }),
        )],
    );
    assert_eq!(rt.meta.mode, "default");
    rt.meta.mode = "acceptEdits".into();
    feed(
        &mut rt,
        &[ev(
            "session.mode_changed",
            json!({ "newMode": "interactive" }),
        )],
    );
    assert_eq!(rt.meta.mode, "acceptEdits");
    feed(
        &mut rt,
        &[json!({ "method": "escouade.mode", "params": { "sessionId": S, "mode": "default" } })],
    );
    assert_eq!(rt.meta.mode, "default");
    feed(
        &mut rt,
        &[ev("session.model_change", json!({ "newModel": "auto" }))],
    );
    assert_eq!(rt.meta.model, "gpt-6-luna", "never auto");
    feed(
        &mut rt,
        &[ev(
            "session.model_change",
            json!({ "newModel": "gpt-5-mini" }),
        )],
    );
    assert_eq!(rt.meta.model, "gpt-5-mini");
}

#[test]
fn an_account_of_the_github_cli_is_told() {
    let mut rt = agent("C:/p", "default");
    feed(
        &mut rt,
        &[
            json!({ "method": "escouade.session", "params": { "sessionId": S, "start": "new", "authType": "gh-cli" } }),
        ],
    );
    let notice = rt
        .conv
        .items()
        .into_iter()
        .find(|i| i["kind"] == "notice")
        .expect("a notice");
    assert!(notice["text"].as_str().unwrap().contains("copilot login"));
}

#[test]
fn the_tools_take_claude_codes_names() {
    let cwd = "C:\\p";
    let (n, i) = tool_of("powershell", &json!({ "command": "ls" }), None, cwd);
    assert_eq!((n.as_str(), i["command"].as_str()), ("Bash", Some("ls")));
    let (n, i) = tool_of("view", &json!({ "path": "a.js" }), None, cwd);
    assert_eq!(
        (n.as_str(), i["file_path"].as_str()),
        ("Read", Some("C:\\p\\a.js"))
    );
    let (n, _) = tool_of("rg", &json!({ "pattern": "x" }), None, cwd);
    assert_eq!(n, "Grep");
    let (n, i) = tool_of(
        "apply_patch",
        &json!("*** Begin Patch\n*** Add File: b.txt\n+x\n*** End Patch\n"),
        None,
        cwd,
    );
    assert_eq!(
        (n.as_str(), i["file_path"].as_str()),
        ("Write", Some("C:\\p\\b.txt"))
    );
    let (n, _) = tool_of("essai-ping", &json!({}), Some("essai"), cwd);
    assert_eq!(n, "mcp__essai__ping");
    let (n, _) = tool_of(
        "github-mcp-server/get_file_contents",
        &json!({}),
        Some("github-mcp-server"),
        cwd,
    );
    assert_eq!(n, "mcp__github-mcp-server__get_file_contents");
    let (n, _) = tool_of("sql", &json!({}), None, cwd);
    assert_eq!(n, "sql");
}

#[test]
fn a_message_refused_ends_its_turn_in_error() {
    let mut rt = agent("C:/p", "default");
    start(&mut rt);
    let mut fx = Effects::default();
    rt.push_user("u1", "x", 0, &[], &mut fx);
    let fxs = feed(
        &mut rt,
        &[
            json!({ "method": "escouade.refused", "params": { "sessionId": S, "message": "quota" } }),
        ],
    );
    assert!(matches!(fxs[0].turn_end, Some(TurnEnd::Error(ref m)) if m == "quota"));
    assert_eq!(rt.meta.status, AgentStatus::Error);
}

#[test]
fn each_call_tells_the_months_credits_and_its_cost_comes_from_its_nano_aiu() {
    let (rt, fxs) = replay("first-turn.jsonl", "default");
    // Every `assistant.usage` of the real run says the credits of the month, given again on the
    // first of November.
    let meters: Vec<_> = fxs.iter().filter_map(|f| f.rate.clone()).collect();
    assert!(!meters.is_empty());
    let last = meters.last().unwrap();
    assert_eq!(last.len(), 1);
    assert_eq!(last[0].minutes, None, "the month's credits");
    assert!(last[0].pct > 0.0 && last[0].pct < 5.0, "{:?}", last[0]);
    assert_eq!(
        last[0].resets_at,
        Some(
            chrono::DateTime::parse_from_rfc3339("2026-11-01T00:00:00Z")
                .unwrap()
                .timestamp_millis()
        )
    );
    // The cost of its turns: its nano-AIU, a credit (10⁹) for a cent.
    let rows: Vec<_> = fxs.iter().flat_map(|f| f.turns.clone()).collect();
    assert!(!rows.is_empty());
    let nano: u64 = recorded("first-turn.jsonl")
        .iter()
        .filter(|l| l["dir"] == "s2c" && l["msg"]["params"]["event"]["type"] == "assistant.usage")
        .filter_map(|l| {
            l["msg"]["params"]["event"]["data"]["copilotUsage"]["totalNanoAiu"].as_u64()
        })
        .sum();
    let cost: f64 = rows.iter().map(|r| r.cost).sum();
    assert!((cost - nano as f64 / 1e11).abs() < 1e-9, "{cost} {nano}");
    assert!(rows.iter().all(|r| r.engine == EngineId::COPILOT));
    assert!((rt.meta.cost - cost).abs() < 1e-9);
}

#[test]
fn a_call_without_its_cost_is_priced_from_its_details_or_its_models_prices() {
    let mut rt = agent("C:/p", "default");
    start(&mut rt);
    let mut fx = Effects::default();
    rt.push_user("u1", "x", 0, &[], &mut fx);
    let prices = json!({ "gpt-6-luna": {
        "inputPrice": 10, "cacheReadPrice": 1, "cacheWritePrice": 12.5, "outputPrice": 50,
        "batchSize": 1_000_000,
    } });
    let fxs = feed(
        &mut rt,
        &[
            json!({ "method": "escouade.prices", "params": { "sessionId": S, "prices": prices } }),
            // The tokens of point 0's first turn, its cost untold: 227 227.5 credits per million.
            ev(
                "assistant.usage",
                json!({ "model": "gpt-6-luna", "inputTokens": 40_145 + 13_657 + 12,
                    "cacheReadTokens": 40_145, "cacheWriteTokens": 13_657, "outputTokens": 325 }),
            ),
            ev("session.idle", json!({})),
        ],
    );
    let row = fxs.iter().flat_map(|f| f.turns.clone()).next().unwrap();
    assert!(
        (row.cost - 227_227_500.0 / 1e11).abs() < 1e-9,
        "{}",
        row.cost
    );
}

#[test]
fn credits_spent_stop_the_turn_until_they_are_given_again() {
    let mut rt = agent("C:/p", "default");
    start(&mut rt);
    let mut fx = Effects::default();
    rt.push_user("u1", "x", 0, &[], &mut fx);
    let reset = "2026-11-01T00:00:00Z";
    let fxs = feed(
        &mut rt,
        &[
            ev(
                "assistant.usage",
                json!({ "model": "gpt-6-luna", "inputTokens": 10, "outputTokens": 1,
                    "copilotUsage": { "totalNanoAiu": 1000 },
                    "quotaSnapshots": { "chat": { "entitlementRequests": 200, "usedRequests": 200,
                        "remainingPercentage": 0, "resetDate": reset } } }),
            ),
            ev(
                "session.error",
                json!({ "errorType": "quota", "errorCode": "quota_exceeded",
                    "message": "You have exceeded your monthly quota." }),
            ),
            ev("session.idle", json!({})),
        ],
    );
    let at = chrono::DateTime::parse_from_rfc3339(reset)
        .unwrap()
        .timestamp_millis();
    assert_eq!(fxs[0].rate.as_ref().unwrap()[0].pct, 100.0);
    let end = fxs.last().unwrap();
    assert_eq!(end.turn_end, Some(TurnEnd::Limited));
    assert_eq!(end.limited, Some(Some(at)));
    assert_ne!(rt.meta.status, AgentStatus::Error);
    let items = rt.conv.items();
    assert_eq!(items_of(&items, "turn").last().unwrap()["limited"], true);
    // Another error is no limit.
    let mut rt = agent("C:/p", "default");
    start(&mut rt);
    rt.push_user("u1", "x", 0, &[], &mut fx);
    let fxs = feed(
        &mut rt,
        &[
            ev(
                "session.error",
                json!({ "errorType": "query", "message": "bad" }),
            ),
            ev("session.idle", json!({})),
        ],
    );
    assert!(matches!(
        fxs.last().unwrap().turn_end,
        Some(TurnEnd::Error(_))
    ));
}

#[test]
fn the_sessions_ceiling_of_credits_is_a_card_whose_choices_answer_copilot() {
    let mut rt = agent("C:/p", "default");
    start(&mut rt);
    let mut fx = Effects::default();
    rt.push_user("u1", "x", 0, &[], &mut fx);
    let data = json!({ "requestId": "lim1", "usedAiCredits": 5.2, "maxAiCredits": 5 });
    feed(
        &mut rt,
        &[ev("session_limits_exhausted.requested", data.clone())],
    );
    let items = rt.conv.items();
    let card = items_of(&items, "question")[0];
    assert_eq!(card["id"], "lim1");
    let q = &card["questions"][0];
    let text = q["question"].as_str().unwrap();
    assert!(text.contains("5.2") && text.contains('5'), "{text}");
    let labels: Vec<&str> = q["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels.len(), 3);
    assert_eq!(rt.meta.status, AgentStatus::Waiting);
    // Each choice gives Copilot its action, in either language.
    for (label, action) in labels.iter().zip(["add", "unset", "cancel"]) {
        assert_eq!(
            crate::engine::copilot_limit_answer(&data, label)["action"],
            action
        );
    }
    assert_eq!(
        crate::engine::copilot_limit_answer(&data, labels[0])["additionalAiCredits"],
        5.0
    );
    assert_eq!(
        crate::engine::copilot_limit_answer(&data, "autre chose")["action"],
        "cancel"
    );
}

#[test]
fn it_says_what_it_does_with_the_words_of_claude_code() {
    let mut rt = agent("C:/p", "default");
    let claude_says = |name: &str, input: Value| crate::agent::tool_activity(name, &input, "C:/p");
    let mut tool = |n: usize, tool: &str, args: Value| {
        feed(
            &mut rt,
            &[ev(
                "tool.execution_start",
                json!({ "toolCallId": format!("t{n}"), "toolName": tool, "arguments": args }),
            )],
        );
        rt.activity.clone()
    };
    assert_eq!(
        tool(1, "view", json!({ "path": "C:/p/src/db.ts" })),
        claude_says("Read", json!({ "file_path": "C:/p/src/db.ts" }))
    );
    assert_eq!(
        tool(
            2,
            "edit",
            json!({ "path": "src/auth/jwt.ts", "old_str": "a", "new_str": "b" })
        ),
        claude_says("Edit", json!({ "file_path": "C:/p/src/auth/jwt.ts" }))
    );
    assert_eq!(
        tool(
            3,
            "create",
            json!({ "path": "C:/p/b.txt", "file_text": "un" })
        ),
        claude_says("Write", json!({ "file_path": "C:/p/b.txt" }))
    );
    assert_eq!(
        tool(4, "rg", json!({ "pattern": "jwt" })),
        claude_says("Grep", json!({ "pattern": "jwt" }))
    );
    assert_eq!(
        tool(
            5,
            "powershell",
            json!({ "command": "npm test", "description": "Tests" })
        ),
        claude_says("Bash", json!({ "command": "npm test" }))
    );
    assert!(tool(
        6,
        "apply_patch",
        json!({ "input": "*** Begin Patch\n*** Update File: src/x.ts\n" })
    )
    .is_some_and(|a| a.ends_with(" src/x.ts")));
}
