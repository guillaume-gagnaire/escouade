use super::*;

/// The worktree of the real runs (`tests/fixtures/cursor/`).
const WT: &str = "C:\\Users\\user\\AppData\\Local\\Temp\\esc24\\wt-agent1";

const S: &str = "ses-own";

/// An agent of Cursor in `cwd`, in `mode`, its session `S` opened.
fn agent(cwd: &str, mode: &str) -> AgentRt {
    let meta = AgentMeta {
        id: new_id(),
        cwd: cwd.into(),
        engine: EngineId::CURSOR,
        model: "default".into(),
        mode: mode.into(),
        ..Default::default()
    };
    let mut rt = AgentRt::new(
        meta,
        &crate::paths::test_dir(&format!("esc42-cu-conv-{}", new_id())),
    );
    feed(
        &mut rt,
        &[json!({ "method": "escouade.session", "params": { "sessionId": S, "start": "new" } })],
    );
    rt
}

fn feed(rt: &mut AgentRt, frames: &[Value]) -> Vec<Effects> {
    frames
        .iter()
        .map(|f| {
            let mut fx = Effects::default();
            rt.cursor_frame(f, &mut fx);
            fx
        })
        .collect()
}

fn up(session: &str, update: Value) -> Value {
    json!({ "method": "session/update", "params": { "sessionId": session, "update": update } })
}

fn chunk(kind: &str, text: &str) -> Value {
    up(
        S,
        json!({ "sessionUpdate": kind, "content": { "type": "text", "text": text } }),
    )
}

fn end(reason: &str) -> Value {
    json!({ "method": "escouade.prompt_end", "params": { "sessionId": S, "stopReason": reason } })
}

/// The agent's turn started, as the core starts it.
fn start(rt: &mut AgentRt, text: &str) {
    let mut fx = Effects::default();
    rt.push_user_from(&new_id(), text, 0, &[], None, &mut fx);
}

fn items_of(rt: &mut AgentRt, kind: &str) -> Vec<Value> {
    rt.conv
        .items()
        .into_iter()
        .filter(|i| i["kind"] == kind)
        .collect()
}

#[test]
fn texts_and_reasonings_in_pieces_make_one_item_each_and_the_reply_ends_the_turn() {
    let mut rt = agent(WT, "default");
    start(&mut rt, "bonjour");
    let fxs = feed(
        &mut rt,
        &[
            chunk("agent_thought_chunk", "Je "),
            chunk("agent_thought_chunk", "réfléchis."),
            chunk("agent_message_chunk", "Bon"),
            chunk("agent_message_chunk", "jour !"),
            end("end_turn"),
        ],
    );
    let thinking = items_of(&mut rt, "thinking");
    assert_eq!(thinking.len(), 1);
    assert_eq!(thinking[0]["text"], "Je réfléchis.");
    assert_eq!(thinking[0]["streaming"], false);
    let texts = items_of(&mut rt, "text");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0]["text"], "Bonjour !");
    assert_eq!(
        fxs.last().unwrap().turn_end,
        Some(TurnEnd::Finished("Bonjour !".into()))
    );
    assert_eq!(rt.meta.status, AgentStatus::Done);
}

#[test]
fn a_write_asked_outside_the_folder_shows_its_diff_and_always_holds_for_the_agent() {
    let mut rt = agent(WT, "default");
    start(&mut rt, "fichier dehors");
    let file = "C:\\Users\\user\\AppData\\Local\\Temp\\esc24\\dehors.txt";
    let call = "call-1\nfc_1";
    feed(
        &mut rt,
        &[
            up(
                S,
                json!({ "sessionUpdate": "tool_call", "toolCallId": call, "title": "Edit File",
                "kind": "edit", "status": "pending", "rawInput": {} }),
            ),
            json!({ "method": "session/request_permission", "id": "cursor-x-1", "params": {
                "sessionId": S,
                "toolCall": { "toolCallId": call, "kind": "edit", "title": "Edit", "content": [
                    { "type": "diff", "path": file, "oldText": "avant\n", "newText": "avant\nnote de Cursor\n" },
                ] },
                "options": [
                    { "optionId": "allow-once", "kind": "allow_once" },
                    { "optionId": "allow-always", "kind": "allow_always" },
                    { "optionId": "reject-once", "kind": "reject_once" },
                ],
            } }),
        ],
    );
    let card = rt.conv.get("cursor-x-1").cloned().unwrap();
    assert_eq!(card["toolName"], "Edit");
    assert_eq!(card["toolUseId"], "call-1~fc_1", "no line break in an id");
    assert_eq!(card["alwaysScope"], "agent");
    assert_eq!(card["canAlways"], true);
    assert!(card["description"]
        .as_str()
        .unwrap()
        .contains("Hors du dossier"));
    let lines = card["patch"][0]["lines"].to_string();
    assert!(
        lines.contains("+note de Cursor") && lines.contains(" avant"),
        "{lines}"
    );
    assert_eq!(rt.meta.status, AgentStatus::Waiting);
    assert_eq!(rt.waiting_views().next().unwrap().tool, "Edit");
    // Done: its result has the diff, the file is beside the folder (not among the agent's).
    feed(
        &mut rt,
        &[up(
            S,
            json!({ "sessionUpdate": "tool_call_update", "toolCallId": call, "status": "completed" }),
        )],
    );
    let tool = rt.conv.get("call-1~fc_1").cloned().unwrap();
    assert_eq!(tool["status"], "ok");
    assert_eq!(tool["result"]["add"], 1);
    assert!(rt.meta.touched_files.is_empty());
}

#[test]
fn a_command_says_why_cursor_asks_and_its_output() {
    let mut rt = agent(WT, "default");
    start(&mut rt, "commande");
    let full = crate::engine::cursor::bench::recorded("full-turn.jsonl");
    let asked = full
        .iter()
        .find(|m| m["method"] == "session/request_permission")
        .unwrap();
    let call = asked["params"]["toolCall"]["toolCallId"].as_str().unwrap();
    let mut started = full
        .iter()
        .find(|m| m["params"]["update"]["toolCallId"] == call)
        .unwrap()
        .clone();
    started["params"]["sessionId"] = json!(S);
    let mut card = asked.clone();
    card["id"] = json!("r1");
    card["params"]["sessionId"] = json!(S);
    feed(&mut rt, &[started, card]);
    let item = rt.conv.get("r1").cloned().unwrap();
    assert_eq!(item["toolName"], "Bash");
    assert_eq!(
        item["input"]["command"],
        "git status --short; git log --oneline -1"
    );
    assert_eq!(item["reason"], "Not in allowlist: git status, git log");
    feed(
        &mut rt,
        &[up(
            S,
            json!({ "sessionUpdate": "tool_call_update", "toolCallId": call, "status": "completed",
            "rawOutput": { "exitCode": 0, "stdout": "3d54c85 init\n", "stderr": "" } }),
        )],
    );
    let tool = rt.conv.get(&call_id(call)).cloned().unwrap();
    assert_eq!(tool["result"]["text"], "3d54c85 init");
    assert_eq!(tool["status"], "ok");
}

#[test]
fn a_question_and_a_plan_have_their_cards_and_the_plans_tasks_go_to_the_plan() {
    let mut rt = agent(WT, "plan");
    start(&mut rt, "question");
    feed(
        &mut rt,
        &[
            json!({ "method": "cursor/ask_question", "id": "q1", "params": {
            "toolCallId": "c1", "title": "Couleur", "questions": [
                { "id": "couleur", "prompt": "Quelle couleur ?", "allowMultiple": false,
                  "options": [{ "id": "r", "label": "rouge" }, { "id": "b", "label": "bleu" }] },
            ],
        } }),
        ],
    );
    let q = rt.conv.get("q1").cloned().unwrap();
    assert_eq!(q["kind"], "question");
    assert_eq!(q["questions"][0]["question"], "Quelle couleur ?");
    assert_eq!(q["questions"][0]["options"][1]["label"], "bleu");
    assert_eq!(q["questions"][0]["header"], "Couleur");
    assert_eq!(rt.waiting_views().next().unwrap().kind, "question");

    feed(
        &mut rt,
        &[
            json!({ "method": "cursor/create_plan", "id": "p1", "params": {
            "toolCallId": "c2", "name": "Ajouter sub", "overview": "Une soustraction.",
            "plan": "1. Écrire sub.", "todos": [
                { "id": "t1", "content": "Écrire sub", "status": "pending" },
                { "id": "t2", "content": "Tester", "status": "pending" },
            ],
        } }),
        ],
    );
    let plan = rt.conv.get("p1").cloned().unwrap();
    assert_eq!(plan["toolName"], "ExitPlanMode");
    assert_eq!(
        plan["input"]["plan"],
        "# Ajouter sub\n\nUne soustraction.\n\n1. Écrire sub."
    );
    let banner = rt.meta.plan.clone().expect("the plan's tasks");
    assert_eq!(banner.tasks.len(), 2);
}

#[test]
fn the_tasks_merge_by_id_and_a_subagent_speaks_under_its_task() {
    let mut rt = agent(WT, "default");
    start(&mut rt, "sous-agent");
    let todos = |merge: bool, list: Value| json!({ "method": "cursor/update_todos", "params": { "toolCallId": "t", "todos": list, "merge": merge } });
    feed(
        &mut rt,
        &[
            todos(
                false,
                json!([
                    { "id": "1", "content": "Lire", "status": "in_progress" },
                    { "id": "2", "content": "Résumer", "status": "pending" },
                ]),
            ),
            todos(
                true,
                json!([{ "id": "1", "content": "Lire", "status": "completed" }]),
            ),
        ],
    );
    let plan = rt.meta.plan.clone().unwrap();
    assert_eq!(plan.tasks.len(), 2, "merged, not replaced");
    feed(
        &mut rt,
        &[
            up(
                S,
                json!({ "sessionUpdate": "tool_call", "toolCallId": "task\n1", "title": "Task: Résumer",
                "kind": "other", "status": "pending",
                "rawInput": { "_toolName": "task", "prompt": "Résume.", "description": "Résumer" } }),
            ),
            up(
                S,
                json!({ "sessionUpdate": "subagent_spawned", "subagentSessionId": "child",
                "_meta": { "cursor": { "toolCallId": "task\n1", "model": "default" } } }),
            ),
            up(
                "child",
                json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "Le README dit bonjour." } }),
            ),
            up(
                S,
                json!({ "sessionUpdate": "tool_call_update", "toolCallId": "task\n1", "status": "completed",
                "rawOutput": { "durationMs": 10 } }),
            ),
            end("end_turn"),
        ],
    );
    let child = items_of(&mut rt, "text")
        .into_iter()
        .find(|t| t["parent"] == "task~1")
        .expect("the subagent's text under its Task");
    assert_eq!(child["text"], "Le README dit bonjour.");
    let task = rt.conv.get("task~1").cloned().unwrap();
    assert_eq!(task["name"], "Task");
    assert_eq!(task["result"]["text"], "Le README dit bonjour.");
    let plan = rt.meta.plan.clone().unwrap();
    assert_eq!(plan.agents.len(), 1);
    assert_eq!(plan.agents[0].status, crate::plan::RunStatus::Done);
}

#[test]
fn a_model_out_of_the_plan_ends_the_turn_limited_with_what_to_do() {
    let mut rt = agent(WT, "default");
    rt.meta.model = "composer-2.5".into();
    start(&mut rt, "bonjour");
    let fxs = feed(
        &mut rt,
        &[
            chunk("agent_message_chunk", "Upgrade your plan to continue"),
            end("end_turn"),
        ],
    );
    assert_eq!(fxs.last().unwrap().turn_end, Some(TurnEnd::Limited));
    let turn = items_of(&mut rt, "turn").pop().unwrap();
    let reason = turn["error"].as_str().unwrap();
    assert!(
        reason.contains("composer-2.5") && reason.contains("Auto"),
        "{reason}"
    );
    assert_eq!(turn["limited"], true);
    assert_eq!(rt.meta.status, AgentStatus::Error);
}

/// ESC-43: the limit of the month, on Auto (« Upgrade your plan to continue » after the turn's own
/// words, as Cursor adds it), or on-demand usage without a payment method: the amber card, which
/// says it and where to see the usage; no resume planned, Escouade knowing no reset.
#[test]
fn the_usage_limit_of_cursor_ends_the_turn_limited_on_a_card_that_says_it() {
    for (said, words) in [
        (
            "Je commence.\n\nUpgrade your plan to continue",
            "Limite d’usage",
        ),
        ("\n\nAdd a payment method to continue", "moyen de paiement"),
    ] {
        let mut rt = agent(WT, "default");
        start(&mut rt, "bonjour");
        let fxs = feed(
            &mut rt,
            &[chunk("agent_message_chunk", said), end("end_turn")],
        );
        let fx = fxs.last().unwrap();
        assert_eq!(fx.turn_end, Some(TurnEnd::Limited), "{said}");
        assert_eq!(fx.limited, None, "no reset known: no resume planned");
        let turn = items_of(&mut rt, "turn").pop().unwrap();
        assert_eq!(turn["limited"], true);
        let reason = turn["error"].as_str().unwrap();
        assert!(reason.contains(words), "{reason}");
        assert!(
            reason.contains(crate::engine::cursor::DASHBOARD),
            "{reason}"
        );
    }
}

/// A sign-in or a key refused is an error, not the limit: for an account by its key, give the key
/// again; for the machine's, `agent login`.
#[test]
fn a_key_or_a_sign_in_refused_ends_the_turn_in_error_saying_what_to_do() {
    let said = "\n\nError: [unauthenticated] Backend rejected authentication. Verify this is a User API Key";
    for (account, words) in [("pro", "clé d’API"), ("principal-cursor", "agent login")] {
        let mut rt = agent(WT, "default");
        rt.meta.account = account.into();
        start(&mut rt, "bonjour");
        let fxs = feed(
            &mut rt,
            &[chunk("agent_message_chunk", said), end("end_turn")],
        );
        let Some(TurnEnd::Error(reason)) = fxs.last().unwrap().turn_end.clone() else {
            panic!("{:?}", fxs.last().unwrap().turn_end);
        };
        assert!(reason.contains(words), "{account}: {reason}");
        assert_eq!(items_of(&mut rt, "turn").pop().unwrap()["limited"], false);
    }
}

/// ESC-43: a write Cursor ends with no diff wrote nothing (a hook of the user's refused it, say, as
/// one of Claude Code did in the real run of ESC-42): the tool fails, saying so; no file is counted
/// as touched.
#[test]
fn a_write_that_ends_without_its_diff_fails_saying_nothing_was_written() {
    let mut rt = agent(WT, "default");
    start(&mut rt, "fichier");
    let file = format!("{WT}\\notes.txt");
    feed(
        &mut rt,
        &[
            up(
                S,
                json!({ "sessionUpdate": "tool_call", "toolCallId": "e1", "title": "Edit File",
                "kind": "edit", "status": "pending", "rawInput": {} }),
            ),
            up(
                S,
                json!({ "sessionUpdate": "tool_call_update", "toolCallId": "e1",
                "title": format!("Edit `{file}`"), "rawInput": { "path": file },
                "locations": [{ "path": file }] }),
            ),
            up(
                S,
                json!({ "sessionUpdate": "tool_call_update", "toolCallId": "e1", "status": "completed" }),
            ),
        ],
    );
    let tool = rt.conv.get("e1").cloned().unwrap();
    assert_eq!(tool["status"], "error");
    assert!(tool.to_string().contains("hook"), "{tool:#}");
    assert!(rt.meta.touched_files.is_empty());
}

#[test]
fn a_cancelled_prompt_is_an_interrupted_turn_and_a_waiting_message_starts_the_next() {
    let mut rt = agent(WT, "default");
    start(&mut rt, "lent");
    // Sent during the turn: queued (Cursor would cancel the turn for it).
    start(&mut rt, "et maintenant");
    assert_eq!(rt.queued, 1);
    let fxs = feed(&mut rt, &[end("cancelled")]);
    assert_eq!(fxs[0].turn_end, Some(TurnEnd::Interrupted));
    assert_eq!(rt.queued, 0);
    assert_eq!(rt.meta.status, AgentStatus::Running, "the next one goes");
    let users = items_of(&mut rt, "user");
    assert_eq!(users[1]["queued"], false);
}

#[test]
fn a_session_not_resumed_says_so_and_the_mode_follows_cursor() {
    let mut rt = agent(WT, "default");
    let mut fx = Effects::default();
    rt.cursor_frame(
        &json!({ "method": "escouade.session", "params": { "sessionId": "other", "start": "new" } }),
        &mut fx,
    );
    assert!(items_of(&mut rt, "notice")[0]["text"]
        .as_str()
        .unwrap()
        .contains("Cursor"));
    feed(
        &mut rt,
        &[up(
            "other",
            json!({ "sessionUpdate": "current_mode_update", "currentModeId": "plan" }),
        )],
    );
    assert_eq!(rt.meta.mode, "plan");
    feed(
        &mut rt,
        &[up(
            "other",
            json!({ "sessionUpdate": "current_mode_update", "currentModeId": "agent" }),
        )],
    );
    assert_eq!(rt.meta.mode, "default");
}

#[test]
fn the_diff_of_a_write_keeps_three_lines_around_what_changed() {
    let old = "a\nb\nc\nd\ne\nf\ng\n";
    let new = "a\nb\nc\nd\nE\nf\ng\n";
    let h = line_hunks(old, new);
    assert_eq!(h[0]["oldStart"], 2);
    assert_eq!(
        h[0]["lines"],
        json!([" b", " c", " d", "-e", "+E", " f", " g"])
    );
    // A file added: all its lines.
    let added = line_hunks("", "x\ny\n");
    assert_eq!(added[0]["lines"], json!(["+x", "+y"]));
    assert_eq!(added[0]["oldStart"], 0);
    assert_eq!(line_hunks("same\n", "same\n"), json!([]));
}

#[test]
fn a_write_done_without_asking_shows_its_diff_and_a_file_added_is_a_write() {
    let mut rt = agent(WT, "default");
    start(&mut rt, "fichier");
    let file = format!("{WT}\\notes.txt");
    feed(
        &mut rt,
        &[
            up(
                S,
                json!({ "sessionUpdate": "tool_call", "toolCallId": "e1", "title": "Edit File",
                "kind": "edit", "status": "pending", "rawInput": {} }),
            ),
            up(
                S,
                json!({ "sessionUpdate": "tool_call_update", "toolCallId": "e1", "status": "completed",
                "content": [{ "type": "diff", "path": file, "oldText": null, "newText": "bonjour\n" }] }),
            ),
        ],
    );
    let tool = rt.conv.get("e1").cloned().unwrap();
    assert_eq!(tool["name"], "Write");
    assert_eq!(tool["result"]["add"], 1);
    assert_eq!(tool["result"]["patch"][0]["lines"], json!(["+bonjour"]));
    assert_eq!(rt.meta.touched_files, ["notes.txt"]);
}
