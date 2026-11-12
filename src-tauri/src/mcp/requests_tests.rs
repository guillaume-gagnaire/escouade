//! The tools that show what an agent waits for the user to answer, and answer it (`list_requests`,
//! `answer_permission`, `answer_question`), called by a real MCP client (rmcp's) on a test core
//! with the fake `claude`: what a request says, who may answer it and how, and that the
//! conversation of the agent that asked says who answered.

use super::private_tests::make_private;
use super::tests::ANSWERING;
use super::tools_tests::{
    as_agent, call, external, last_entry, link, other_project, project, put, put_agent, read,
    refused, ticket,
};
use crate::agent::Effects;
use crate::conv::Conv;
use crate::core::AgentOptions;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::{json, Value};

/// A new agent of `p` named `name` (never renamed), its process up: it asks nothing.
pub(super) async fn live(h: &Harness, p: &Project, name: &str) -> AgentMeta {
    let meta = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                name: Some(name.into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    h.core.ensure_process(&meta.id).await.unwrap();
    meta
}

/// A new agent of `p` named `name`, told `what` ("permission", "question", "exit plan"): its fake
/// `claude` asks for it, and the agent waits for the answer.
pub(super) async fn asking(h: &Harness, p: &Project, name: &str, what: &str) -> AgentMeta {
    let meta = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                name: Some(name.into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    h.core
        .send_message(&meta.id, what.into(), vec![])
        .await
        .unwrap();
    h.wait("its request", |h| h.view(&meta.id).pending.len() == 1)
        .await;
    meta
}

/// Claude Code asks the agent `id` for `rid`, as `request` (its `can_use_tool`) says. The fake
/// `claude` of a live agent ignores the answer: nothing else happens to the agent.
pub(super) fn asks(h: &Harness, id: &str, rid: &str, mut request: Value) {
    request["subtype"] = json!("can_use_tool");
    h.core.agent(id).unwrap().lock().handle_frame(
        &json!({ "type": "control_request", "request_id": rid, "request": request }),
        &mut Effects::default(),
    );
}

/// A permission for the Bash tool.
pub(super) fn bash(command: &str) -> Value {
    json!({ "tool_name": "Bash", "tool_use_id": "tu", "input": { "command": command } })
}

/// A question of `AskUserQuestion` (what Claude Code sends: `questions`).
pub(super) fn question(questions: Value) -> Value {
    json!({ "tool_name": "AskUserQuestion", "tool_use_id": "tq", "input": { "questions": questions } })
}

pub(super) fn waiting(h: &Harness, id: &str) -> Vec<String> {
    h.view(id).pending
}

/// `chef` made the chef of `p`, with « Seulement le chef d’escouade » (ESC-423), as the user does in
/// the settings.
pub(super) fn make_chef(h: &Harness, p: &Project, chef: &str) {
    h.core.settings.write().agents_answer = AgentsAnswer::Chef;
    let mut q = h.core.project(&p.id).unwrap();
    q.chef = Some(chef.into());
    h.core.update_project(q).unwrap();
}

/// The request as the conversation of the agent keeps it.
fn item(h: &Harness, agent_id: &str, request: &str) -> Value {
    h.items(agent_id)
        .into_iter()
        .find(|i| i["id"] == request)
        .unwrap()
}

/// The last view of the agent the window was sent.
fn last_view(h: &Harness, id: &str) -> Value {
    h.events
        .lock()
        .iter()
        .rev()
        .find(|e| e["type"] == "agent" && e["agent"]["id"] == id)
        .unwrap()["agent"]
        .clone()
}

/// `row` without its time (which the test cannot know).
fn without_since(mut row: Value) -> Value {
    assert!(row["since"].as_i64().is_some_and(|t| t > 0), "{row}");
    row.as_object_mut().unwrap().remove("since");
    row
}

fn ids(rows: &Value) -> Vec<&str> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| r["request"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn the_answering_tools_offer_allow_and_deny_and_never_a_lasting_permission() {
    let h = harness("mcp-req-schemas");
    let c = external(&h).await;
    let tools = c.list_all_tools().await.unwrap();
    let tool = |name: &str| {
        let t = tools.iter().find(|t| t.name == name).unwrap();
        serde_json::to_value(t).unwrap()
    };
    let schema = |name: &str| tool(name)["inputSchema"].clone();
    let required = |name: &str| {
        let mut r: Vec<String> = serde_json::from_value(schema(name)["required"].clone()).unwrap();
        r.sort();
        r
    };
    assert_eq!(
        required("answer_permission"),
        ["agent", "decision", "request"]
    );
    assert_eq!(required("answer_question"), ["agent", "answers", "request"]);
    assert!(schema("list_requests")["required"]
        .as_array()
        .is_none_or(|r| r.is_empty()));
    // A permission is allowed or denied; « Toujours autoriser » is not among the choices.
    let decision = schema("answer_permission")["properties"]["decision"].clone();
    let mut choices: Vec<String> = serde_json::from_value(decision["enum"].clone())
        .unwrap_or_else(|e| panic!("{e}: {decision}"));
    choices.sort();
    assert_eq!(choices, ["allow", "deny"]);
    // Each answer is one of the options picked or a free text.
    let answer = schema("answer_question")["properties"]["answers"]["items"].to_string();
    for field in ["question", "choices", "text"] {
        assert!(answer.contains(field), "{answer}");
    }
    // What they do and refuse, in the description the model reads.
    for name in ANSWERING {
        let v = tool(name);
        let description = v["description"].as_str().unwrap();
        for word in [
            "no longer waits",
            "Claude Code view",
            "Refused to an agent by default",
            "squad lead",
        ] {
            assert!(description.contains(word), "{name}: {description}");
        }
        // They change something, and are no read.
        assert_eq!(v["annotations"]["readOnlyHint"], false, "{name}: {v}");
    }
    assert!(tool("answer_permission")["description"]
        .as_str()
        .unwrap()
        .contains("Always allow is not offered"));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_requests_waiting_are_listed_with_what_they_ask_oldest_first() {
    let h = harness("mcp-req-list");
    let p = project(&h).await;
    let t = put(&h, ticket(&p, "DEM-1", "Ajouter", Column::Doing, 1));
    let perm = asking(&h, &p, "agent-perm", "permission").await;
    link(&h, &perm.id, &t.id);
    let quest = asking(&h, &p, "agent-question", "question").await;
    // The first agent asks again, after the second one did.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let mut more = bash("npm test");
    more["description"] = json!("Lance les tests");
    more["default_to_no"] = json!(true);
    asks(&h, &perm.id, "r2", more);
    let none = put_agent(&h, &p, "agent-calme", 5);
    let c = external(&h).await;

    let all = read(&c, "list_requests", json!({})).await;
    // In the order they came, of every agent.
    assert_eq!(ids(&all), ["req_perm", "req_question", "r2"]);
    assert_eq!(
        without_since(all[0].clone()),
        json!({
            "request": "req_perm",
            "type": "permission",
            "agent": {
                "id": perm.id, "name": "agent-perm", "project": "demo", "projectId": p.id,
                "ticket": "DEM-1",
            },
            "tool": "Bash",
            "arg": "rm -rf build",
            "input": { "command": "rm -rf build", "description": "Supprime le dossier build" },
            "description": null,
            "reason": "Commande destructive",
            "defaultNo": false,
            "cut": false,
        })
    );
    assert_eq!(
        without_since(all[1].clone()),
        json!({
            "request": "req_question",
            "type": "question",
            "agent": {
                "id": quest.id, "name": "agent-question", "project": "demo", "projectId": p.id,
                "ticket": null,
            },
            "tool": "AskUserQuestion",
            "description": null,
            "reason": null,
            "defaultNo": false,
            "cut": false,
            "questions": [{
                "question": "Quelle base de données ?",
                "header": "Base",
                "multiSelect": false,
                "options": [
                    { "label": "PostgreSQL", "description": "Relationnelle" },
                    { "label": "SQLite", "description": "Embarquée" },
                ],
            }],
        })
    );
    // What Claude Code says besides the tool: why, and that it would refuse by default.
    assert_eq!(
        (
            all[2]["arg"].as_str(),
            all[2]["description"].as_str(),
            all[2]["defaultNo"].as_bool()
        ),
        (Some("npm test"), Some("Lance les tests"), Some(true))
    );
    // Each time is when it came, in that order.
    let times: Vec<i64> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["since"].as_i64().unwrap())
        .collect();
    assert!(
        times.windows(2).all(|w| w[0] <= w[1]) && times[0] > 0,
        "{times:?}"
    );
    // The call is in the activity log.
    assert_eq!(
        last_entry(&h),
        (
            "Claude (hors Escouade)".into(),
            "list_requests".into(),
            String::new(),
            "ok"
        )
    );

    // An agent that waits for nothing has none.
    let calm = read(&c, "list_requests", json!({ "agent": none.name })).await;
    assert_eq!(calm, json!([]));
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_requests_of_one_agent_or_one_project_are_listed_and_an_unknown_name_is_told() {
    let h = harness("mcp-req-scope");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let perm = asking(&h, &p, "agent-perm", "permission").await;
    let quest = asking(&h, &p, "agent-question", "question").await;
    let away = live(&h, &other, "agent-ailleurs").await;
    asks(&h, &away.id, "r9", bash("ls"));
    // One agent, by its name in any case or its id, with its project to tell two apart.
    let c = external(&h).await;
    let one = read(&c, "list_requests", json!({ "agent": "AGENT-QUESTION" })).await;
    assert_eq!(ids(&one), ["req_question"]);
    let one = read(&c, "list_requests", json!({ "agent": perm.id })).await;
    assert_eq!(ids(&one), ["req_perm"]);
    let one = read(
        &c,
        "list_requests",
        json!({ "agent": "agent-ailleurs", "project": "AUTRE" }),
    )
    .await;
    assert_eq!(ids(&one), ["r9"]);
    assert_eq!(
        last_entry(&h).2,
        "agent: agent-ailleurs · project: AUTRE".to_string()
    );
    // One project: its agents'.
    let demo = read(&c, "list_requests", json!({ "project": "demo" })).await;
    assert_eq!(ids(&demo), ["req_perm", "req_question"]);
    let there = read(&c, "list_requests", json!({ "project": other.id })).await;
    assert_eq!(ids(&there), ["r9"]);
    // Names that fit nothing are told, with the choices.
    let no_agent = refused(&c, "list_requests", json!({ "agent": "zzz" })).await;
    assert!(
        no_agent.contains("Aucun agent « zzz »") && no_agent.contains("agent-perm"),
        "{no_agent}"
    );
    let not_there = refused(
        &c,
        "list_requests",
        json!({ "agent": "agent-perm", "project": "autre" }),
    )
    .await;
    assert!(
        not_there.contains("Aucun agent « agent-perm » dans le projet autre"),
        "{not_there}"
    );
    let no_project = refused(&c, "list_requests", json!({ "project": "nope" })).await;
    assert!(no_project.contains("Aucun projet « nope »"), "{no_project}");
    assert_eq!(
        (waiting(&h, &quest.id).len(), waiting(&h, &perm.id).len()),
        (1, 1)
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_permission_gives_its_input_bounded_and_a_command_up_to_twenty_thousand_characters() {
    let h = harness("mcp-req-input");
    let p = project(&h).await;
    let a = live(&h, &p, "agent-1").await;
    // A tool of another server: its first text is all the summary says.
    asks(
        &h,
        &a.id,
        "rm",
        json!({ "tool_name": "mcp__github__create_issue", "tool_use_id": "tm", "input": {
            "title": "Un bug", "body": "d".repeat(25_000), "labels": ["a", "b"], "draft": true } }),
    );
    // A tool of Escouade's own: its card lists every argument, the summary only one.
    asks(
        &h,
        &a.id,
        "re",
        json!({ "tool_name": "mcp__escouade__create_ticket", "tool_use_id": "te", "input": {
            "project": "demo", "title": "Corriger", "criteria": ["Un", "Deux"] } }),
    );
    // A file written: its path is the summary, its content and its flags are the request.
    let file = format!("{}/src/app.ts", p.path.replace('\\', "/"));
    asks(
        &h,
        &a.id,
        "rw",
        json!({ "tool_name": "Write", "tool_use_id": "tw", "input": {
            "file_path": file, "content": "const a = 2;\n" } }),
    );
    // A command with flags that change what it does.
    asks(
        &h,
        &a.id,
        "rb",
        json!({ "tool_name": "Bash", "tool_use_id": "tb", "input": {
            "command": "npm test", "run_in_background": true, "dangerouslyDisableSandbox": true } }),
    );
    // A command of 5,000 characters: whole (the overview cuts at 2,000), and no cut said.
    let long = format!("echo {}", "mot ".repeat(1250));
    asks(&h, &a.id, "rl", bash(&long));
    // One of 30,000: its start and its end, and the cut said.
    let huge = format!("echo debut {} fin", "mot ".repeat(7500));
    asks(&h, &a.id, "rh", bash(&huge));
    let c = external(&h).await;
    let rows = read(&c, "list_requests", json!({ "agent": a.id })).await;
    assert_eq!(ids(&rows), ["rm", "re", "rw", "rb", "rl", "rh"]);
    // The summary is one text of the input (the first the keys give, here the body): it does not
    // say what the issue is about, the input does.
    assert!(
        !rows[0]["arg"].as_str().unwrap().contains("Un bug"),
        "{}",
        rows[0]
    );
    let input = &rows[0]["input"];
    assert_eq!(input["title"], "Un bug");
    assert_eq!(input["labels"], json!(["a", "b"]));
    assert_eq!(input["draft"], true);
    // Each text is capped at 20,000 characters, and the request says so.
    assert_eq!(input["body"].as_str().unwrap().chars().count(), 20_000);
    assert_eq!(rows[0]["cut"], true);
    assert_eq!(
        rows[1]["input"],
        json!({ "project": "demo", "title": "Corriger", "criteria": ["Un", "Deux"] })
    );
    assert_eq!(rows[1]["cut"], false);
    // What is written, not only where, and the flags of a command: none left out, none cut.
    assert_eq!(rows[2]["arg"], "src/app.ts");
    assert_eq!(rows[2]["input"]["content"], "const a = 2;\n");
    assert_eq!(rows[2]["cut"], false);
    assert_eq!(rows[3]["arg"], "npm test");
    assert_eq!(rows[3]["input"]["dangerouslyDisableSandbox"], true);
    assert_eq!(rows[3]["input"]["run_in_background"], true);
    assert_eq!(rows[3]["cut"], false);
    // A command is read up to 20,000 characters: nowhere else can it be read.
    assert_eq!(rows[4]["arg"].as_str(), Some(long.trim()));
    assert_eq!(rows[4]["input"]["command"].as_str(), Some(long.as_str()));
    assert_eq!(rows[4]["cut"], false);
    let shown = rows[5]["arg"].as_str().unwrap();
    assert!(
        shown.chars().count() <= 20_000 && shown.contains(" … "),
        "{}",
        shown.len()
    );
    assert!(shown.starts_with("echo debut") && shown.ends_with("fin"));
    assert_eq!(rows[5]["cut"], true);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_question_lists_its_options_and_whether_several_may_be_picked() {
    let h = harness("mcp-req-multi");
    let p = project(&h).await;
    let a = live(&h, &p, "agent-1").await;
    let long = "d".repeat(2500);
    asks(
        &h,
        &a.id,
        "rq",
        question(json!([
            { "question": "Quelles bases ?", "header": "Bases", "multiSelect": true, "options": [
                { "label": "SQLite", "description": "Embarquée" },
                { "label": "Postgres" },
            ] },
            { "question": "Quel thème ?", "options": [{ "label": "Clair", "description": long }] },
        ])),
    );
    let c = external(&h).await;
    let rows = read(&c, "list_requests", json!({ "agent": a.id })).await;
    let q = &rows[0];
    assert_eq!(q["type"], "question");
    assert_eq!(
        q["questions"][0],
        json!({
            "question": "Quelles bases ?",
            "header": "Bases",
            "multiSelect": true,
            "options": [
                { "label": "SQLite", "description": "Embarquée" },
                { "label": "Postgres" },
            ],
        })
    );
    // A question without a header or a choice of several: one pick, nothing said of a header.
    let second = &q["questions"][1];
    assert_eq!(second["multiSelect"], false);
    assert!(second.get("header").is_none(), "{second}");
    // A description too long is cut, and the request says so.
    let description = second["options"][0]["description"].as_str().unwrap();
    assert_eq!(description.chars().count(), 2000);
    assert_eq!(q["cut"], true);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_plan_to_approve_is_listed_as_a_plan_with_its_whole_text() {
    let h = harness("mcp-req-plan");
    let p = project(&h).await;
    let a = asking(&h, &p, "agent-1", "exit plan").await;
    let c = external(&h).await;
    let rows = read(&c, "list_requests", json!({})).await;
    assert_eq!(
        (
            rows[0]["type"].as_str(),
            rows[0]["tool"].as_str(),
            rows[0]["arg"].as_str(),
            rows[0]["cut"].as_bool()
        ),
        (
            Some("plan"),
            Some("ExitPlanMode"),
            Some("# Plan\n\n1. Écrire le test\n2. Faire passer le test"),
            Some(false)
        )
    );
    // A plan of several thousand characters is given whole (the overview caps at 2,000).
    let plan = format!("# Plan\n{}", "- une étape du plan\n".repeat(400));
    asks(
        &h,
        &a.id,
        "rp",
        json!({ "tool_name": "ExitPlanMode", "tool_use_id": "tp", "input": { "plan": plan } }),
    );
    let rows = read(&c, "list_requests", json!({})).await;
    assert_eq!(rows[1]["arg"].as_str(), Some(plan.trim()));
    assert_eq!(rows[1]["cut"], false);
    // A plan beyond what is given is cut at its start and its end, and says so.
    let huge = format!("# Plan\n{}fin", "- une étape du plan\n".repeat(2000));
    asks(
        &h,
        &a.id,
        "rh",
        json!({ "tool_name": "ExitPlanMode", "tool_use_id": "th", "input": { "plan": huge } }),
    );
    let rows = read(&c, "list_requests", json!({})).await;
    let shown = rows[2]["arg"].as_str().unwrap();
    assert!(
        shown.chars().count() <= 20_000 && shown.contains(" … "),
        "{}",
        shown.len()
    );
    assert!(shown.starts_with("# Plan") && shown.ends_with("fin"));
    assert_eq!(rows[2]["cut"], true);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn the_agents_and_their_summaries_say_how_many_requests_wait_and_of_which_type() {
    let h = harness("mcp-req-counts");
    let p = project(&h).await;
    let perm = asking(&h, &p, "agent-perm", "permission").await;
    asks(&h, &perm.id, "r2", bash("npm test"));
    let plan = live(&h, &p, "agent-plan").await;
    asks(
        &h,
        &plan.id,
        "rp",
        json!({ "tool_name": "ExitPlanMode", "tool_use_id": "tp", "input": { "plan": "# P" } }),
    );
    let quest = asking(&h, &p, "agent-question", "question").await;
    let calm = put_agent(&h, &p, "agent-calme", 5);
    let c = external(&h).await;
    let agents = read(&c, "list_agents", json!({})).await;
    let count = |name: &str| {
        let row = agents
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"] == name)
            .unwrap();
        (row["waiting"].clone(), row["requests"].clone())
    };
    assert_eq!(count("agent-perm"), (json!(true), json!(2)));
    assert_eq!(count("agent-question"), (json!(true), json!(1)));
    assert_eq!(count("agent-plan"), (json!(true), json!(1)));
    assert_eq!(count("agent-calme"), (json!(false), json!(0)));
    // The summary gives their types, in the order they came.
    let summary = read(&c, "get_agent_summary", json!({ "agent": perm.id })).await;
    assert_eq!(
        (&summary["requests"], &summary["waitingFor"]),
        (&json!(2), &json!(["permission", "permission"]))
    );
    let summary = read(&c, "get_agent_summary", json!({ "agent": quest.id })).await;
    assert_eq!(
        (&summary["requests"], &summary["waitingFor"]),
        (&json!(1), &json!(["question"]))
    );
    let summary = read(&c, "get_agent_summary", json!({ "agent": plan.id })).await;
    assert_eq!(summary["waitingFor"], json!(["plan"]));
    let summary = read(&c, "get_agent_summary", json!({ "agent": calm.id })).await;
    assert_eq!(
        (&summary["requests"], &summary["waitingFor"]),
        (&json!(0), &json!([]))
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_whose_process_is_gone_counts_no_request_that_can_be_answered() {
    let h = harness("mcp-req-gone");
    let p = project(&h).await;
    let a = live(&h, &p, "agent-1").await;
    asks(&h, &a.id, "r1", bash("npm test"));
    let c = external(&h).await;
    let count = |agents: &Value| {
        let row = agents
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == a.id.as_str())
            .unwrap();
        (row["waiting"].clone(), row["requests"].clone())
    };
    assert_eq!(
        count(&read(&c, "list_agents", json!({})).await),
        (json!(true), json!(1))
    );
    // Its process is detached (an account switch does), with its request still there: the window
    // shows it, but nothing can answer it, and the tools that count agree with those that list.
    let process = h.core.agent(&a.id).unwrap().lock().detach().unwrap();
    process.close_input();
    h.wait("the process to exit", |_| !process.is_alive()).await;
    assert_eq!(
        count(&read(&c, "list_agents", json!({})).await),
        (json!(true), json!(0))
    );
    assert_eq!(read(&c, "list_requests", json!({})).await, json!([]));
    let summary = read(&c, "get_agent_summary", json!({ "agent": a.id })).await;
    assert_eq!(summary["requests"], 0);
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_request_that_goes_between_the_check_and_the_answer_is_told_as_gone() {
    let h = harness("mcp-req-not-answered");
    let p = project(&h).await;
    let a = asking(&h, &p, "agent-1", "permission").await;
    let found =
        crate::mcp::resolve::agent(&h.core, &crate::mcp::Caller::External, &a.id, None).unwrap();
    let why = || anyhow::anyhow!("Claude ne tourne plus pour cet agent");
    // Still waiting: what the core said is the refusal.
    let still = super::act::not_answered(&h.core, &found, "req_perm", why());
    assert_eq!(
        still,
        super::tools::ToolError::Refused("Claude ne tourne plus pour cet agent".into())
    );
    // The user answers first, in the window: the answer that fails reads that it was too late.
    h.core
        .answer_permission(&a.id, "req_perm", "allow", None)
        .unwrap();
    let gone = super::act::not_answered(&h.core, &found, "req_perm", why());
    match gone {
        super::tools::ToolError::Refused(m) => {
            assert!(m.contains("n’attend plus de réponse"), "{m}")
        }
        other => panic!("{other:?}"),
    }
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_agent_in_the_claude_code_view_has_no_request_listed_and_none_answered() {
    let h = harness("mcp-req-terminal");
    let p = project(&h).await;
    let a = live(&h, &p, "agent-1").await;
    // In its own interface, in a terminal: it asks there, and Escouade's process asks nothing.
    h.core.agent(&a.id).unwrap().lock().meta.terminal = true;
    let c = external(&h).await;
    assert_eq!(
        read(&c, "list_requests", json!({ "agent": a.id })).await,
        json!([])
    );
    // Were a request there all the same (the view was switched while it waited), it is not offered
    // to answer: the terminal has it.
    asks(&h, &a.id, "r1", bash("npm test"));
    assert_eq!(
        read(&c, "list_requests", json!({ "agent": a.id })).await,
        json!([])
    );
    let summary = read(&c, "get_agent_summary", json!({ "agent": a.id })).await;
    assert_eq!(
        (&summary["requests"], &summary["waitingFor"]),
        (&json!(0), &json!([]))
    );
    for (tool, args) in [
        (
            "answer_permission",
            json!({ "agent": a.id, "request": "r1", "decision": "allow" }),
        ),
        (
            "answer_question",
            json!({ "agent": a.id, "request": "r1", "answers": [{ "text": "oui" }] }),
        ),
    ] {
        let why = refused(&c, tool, args).await;
        assert!(
            why.contains("vue Claude Code") && why.contains("terminal") && why.contains("agent-1"),
            "{tool}: {why}"
        );
        assert_eq!(last_entry(&h).3, "refused");
    }
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_permission_is_allowed_through_the_server_and_the_conversation_says_who_did() {
    let h = harness("mcp-req-allow");
    let p = project(&h).await;
    let a = asking(&h, &p, "agent-1", "permission").await;
    let c = external(&h).await;
    let v = read(
        &c,
        "answer_permission",
        json!({ "agent": "AGENT-1", "request": "req_perm", "decision": "allow" }),
    )
    .await;
    assert_eq!(
        v,
        json!({ "id": a.id, "name": "agent-1", "request": "req_perm", "decision": "allow" })
    );
    // The call is in the activity log, with its arguments.
    let (caller, tool, summary, outcome) = last_entry(&h);
    assert_eq!(
        (caller.as_str(), tool.as_str(), outcome),
        ("Claude (hors Escouade)", "answer_permission", "ok")
    );
    for part in ["agent: AGENT-1", "request: req_perm", "decision: allow"] {
        assert!(summary.contains(part), "{summary}");
    }
    // What the window does when the user answers: nothing waits any more, and the window is told.
    assert!(waiting(&h, &a.id).is_empty());
    let view = last_view(&h, &a.id);
    assert_eq!(view["pending"], json!([]));
    assert_eq!(view["requests"], json!([]));
    // Claude went on, and the card of the request says by whom it was allowed.
    h.wait("the turn to end", |h| {
        h.agent(&a.id).status == AgentStatus::Done
    })
    .await;
    assert!(h
        .items(&a.id)
        .iter()
        .any(|i| i["kind"] == "text" && i["text"] == "Commande exécutée."));
    let card = item(&h, &a.id, "req_perm");
    assert_eq!(
        (card["decision"].as_str(), card["by"].as_str()),
        (Some("allow"), Some("Claude (hors Escouade)"))
    );
    // Kept with the conversation, as the rest of what the card says.
    let saved = Conv::new(&h.core.data.conversations(), &a.id)
        .items()
        .into_iter()
        .find(|i| i["id"] == "req_perm")
        .unwrap();
    assert_eq!(saved["by"], "Claude (hors Escouade)");
    assert_eq!(saved["decision"], "allow");
    // It does not wait any more: a second answer is refused.
    let again = refused(
        &c,
        "answer_permission",
        json!({ "agent": a.id, "request": "req_perm", "decision": "deny" }),
    )
    .await;
    assert!(again.contains("n’attend plus de réponse"), "{again}");
    assert_eq!(last_entry(&h).3, "refused");
    assert_eq!(
        read(&c, "list_requests", json!({ "agent": a.id })).await,
        json!([])
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_permission_is_refused_through_the_server_and_claude_is_told_who_refused() {
    let h = harness("mcp-req-deny");
    let p = project(&h).await;
    let a = asking(&h, &p, "agent-1", "permission").await;
    let b = live(&h, &p, "agent-2").await;
    // Another agent of the project answers: the chef the user chose (ESC-423).
    make_chef(&h, &p, &b.id);
    let from_b = as_agent(&h, &b.id).await;
    let v = read(
        &from_b,
        "answer_permission",
        json!({
            "agent": a.name, "request": "req_perm", "decision": "deny",
            "message": "utilise npm run clean",
        }),
    )
    .await;
    assert_eq!(v["decision"], "deny");
    h.wait("the turn to end", |h| {
        h.agent(&a.id).status == AgentStatus::Done
    })
    .await;
    // Claude reads who refused and why, never that the user did.
    assert!(h
        .items(&a.id)
        .iter()
        .any(|i| i["text"] == "Compris : Refusé par agent-2 via Escouade : utilise npm run clean"));
    // The card keeps the message as written, and who refused.
    let card = item(&h, &a.id, "req_perm");
    assert_eq!(
        (
            card["decision"].as_str(),
            card["message"].as_str(),
            card["by"].as_str()
        ),
        (Some("deny"), Some("utilise npm run clean"), Some("agent-2"))
    );
    let logged = last_entry(&h);
    assert_eq!(
        (logged.0.as_str(), logged.1.as_str(), logged.3),
        ("agent-2", "answer_permission", "ok")
    );
    assert!(
        logged.2.contains("message: utilise npm run clean"),
        "{}",
        logged.2
    );

    // Without a word: refused all the same, and told by whom.
    h.core
        .send_message(&a.id, "permission encore".into(), vec![])
        .await
        .unwrap();
    h.wait("a second request", |h| waiting(h, &a.id).len() == 1)
        .await;
    read(
        &from_b,
        "answer_permission",
        json!({ "agent": a.id, "request": "req_perm", "decision": "deny" }),
    )
    .await;
    h.wait("the second turn to end", |h| {
        h.items(&a.id)
            .iter()
            .any(|i| i["text"] == "Compris : Refusé par agent-2 via Escouade.")
    })
    .await;
    from_b.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_plan_is_approved_or_sent_back_through_the_server_as_the_window_does() {
    let h = harness("mcp-req-plan-answer");
    let p = project(&h).await;
    let a = asking(&h, &p, "agent-1", "exit plan").await;
    let c = external(&h).await;
    // Sent back without a word: Claude is told to keep planning, as the window tells it.
    read(
        &c,
        "answer_permission",
        json!({ "agent": a.id, "request": "req_plan", "decision": "deny" }),
    )
    .await;
    h.wait("the turn to end", |h| {
        h.agent(&a.id).status == AgentStatus::Done
    })
    .await;
    assert!(h.items(&a.id).iter().any(|i| i["text"]
        == "Compris : Refusé par Claude (hors Escouade) via Escouade : Continue à planifier : le plan ne me convient pas encore."));
    let card = item(&h, &a.id, "req_plan");
    assert_eq!(card["by"].as_str(), Some("Claude (hors Escouade)"));
    assert_eq!(
        card["message"].as_str(),
        Some("Continue à planifier : le plan ne me convient pas encore.")
    );
    // Approved: an ordinary permission, allowed.
    h.core
        .send_message(&a.id, "exit plan encore".into(), vec![])
        .await
        .unwrap();
    h.wait("the plan again", |h| waiting(h, &a.id).len() == 1)
        .await;
    read(
        &c,
        "answer_permission",
        json!({ "agent": a.id, "request": "req_plan", "decision": "allow" }),
    )
    .await;
    h.wait("the plan approved", |h| {
        h.items(&a.id).iter().any(|i| i["text"] == "Plan approuvé.")
    })
    .await;
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn a_question_is_answered_through_the_server_as_the_question_card_does() {
    let h = harness("mcp-req-answer");
    let p = project(&h).await;
    let a = asking(&h, &p, "agent-1", "question").await;
    let b = live(&h, &p, "agent-2").await;
    make_chef(&h, &p, &b.id);
    let from_b = as_agent(&h, &b.id).await;
    // A single question: its answer needs no question named.
    let v = read(
        &from_b,
        "answer_question",
        json!({
            "agent": "agent-1", "request": "req_question",
            "answers": [{ "choices": ["SQLite"] }],
        }),
    )
    .await;
    assert_eq!(
        v,
        json!({
            "id": a.id, "name": "agent-1", "request": "req_question",
            "answers": { "Quelle base de données ?": "SQLite" },
        })
    );
    h.wait("the turn to end", |h| {
        h.agent(&a.id).status == AgentStatus::Done
    })
    .await;
    // Claude reads the answer with who gave it, never as the user's own words; the card keeps it
    // as it was given.
    assert!(h.items(&a.id).iter().any(|i| i["kind"] == "text"
        && i["text"] == "Choix retenu : SQLite (réponse de agent-2 via Escouade)"));
    let card = item(&h, &a.id, "req_question");
    assert_eq!(
        card["answers"],
        json!({ "Quelle base de données ?": "SQLite" })
    );
    assert_eq!(card["by"], "agent-2");
    let logged = last_entry(&h);
    assert_eq!(
        (logged.0.as_str(), logged.1.as_str(), logged.3),
        ("agent-2", "answer_question", "ok")
    );
    assert!(
        logged.2.contains("request: req_question") && logged.2.contains("SQLite"),
        "{}",
        logged.2
    );
    assert!(waiting(&h, &a.id).is_empty());
    from_b.cancel().await.unwrap();

    // A free text, the question named in any case, by Claude outside Escouade.
    h.core
        .send_message(&a.id, "question encore".into(), vec![])
        .await
        .unwrap();
    h.wait("a second question", |h| waiting(h, &a.id).len() == 1)
        .await;
    let c = external(&h).await;
    read(
        &c,
        "answer_question",
        json!({
            "agent": a.id, "request": "req_question",
            "answers": [{ "question": "quelle base de données ?", "text": "  Une base maison  " }],
        }),
    )
    .await;
    h.wait("the second turn to end", |h| {
        h.items(&a.id).iter().any(|i| {
            i["text"]
                == "Choix retenu : Une base maison (réponse de Claude (hors Escouade) via Escouade)"
        })
    })
    .await;
    let card = item(&h, &a.id, "req_question");
    assert_eq!(card["by"], "Claude (hors Escouade)");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn several_questions_are_answered_one_by_one_and_several_options_only_where_allowed() {
    let h = harness("mcp-req-answer-many");
    let p = project(&h).await;
    let a = live(&h, &p, "agent-1").await;
    asks(
        &h,
        &a.id,
        "rq",
        question(json!([
            { "question": "Quelles bases ?", "multiSelect": true, "options": [
                { "label": "SQLite" }, { "label": "Postgres" }, { "label": "MySQL" },
            ] },
            { "question": "Quel thème ?", "options": [{ "label": "Clair" }, { "label": "Sombre" }] },
        ])),
    );
    let c = external(&h).await;
    let answer = |answers: Value| {
        let c = &c;
        let id = a.id.clone();
        async move {
            call(
                c,
                "answer_question",
                json!({ "agent": id, "request": "rq", "answers": answers }),
            )
            .await
        }
    };
    let failed = |(text, error): (String, bool)| {
        assert!(error, "{text}");
        text
    };
    // Nothing is sent while an answer does not fit: the request still waits.
    let cases: [(Value, &[&str]); 10] = [
        // None, or not one for each question.
        (json!([]), &["elle en pose 2"]),
        (
            json!([{ "question": "Quelles bases ?", "choices": ["SQLite"] }]),
            &["Quel thème ?", "pas de réponse"],
        ),
        // The question must be named when there are several, and be one of them.
        (
            json!([{ "choices": ["SQLite"] }, { "choices": ["Clair"] }]),
            &["Quelles bases ?", "Quel thème ?", "argument question"],
        ),
        (
            json!([
                { "question": "Quelles bases ?", "choices": ["SQLite"] },
                { "question": "Quel autre ?", "choices": ["Clair"] },
            ]),
            &["Aucune question « Quel autre ? »", "Quel thème ?"],
        ),
        // Twice the same question.
        (
            json!([
                { "question": "Quelles bases ?", "choices": ["SQLite"] },
                { "question": "Quelles bases ?", "choices": ["MySQL"] },
                { "question": "Quel thème ?", "choices": ["Clair"] },
            ]),
            &["Deux réponses", "Quelles bases ?"],
        ),
        // Choices or a text, one of the two.
        (
            json!([
                { "question": "Quelles bases ?", "choices": ["SQLite"], "text": "x" },
                { "question": "Quel thème ?", "choices": ["Clair"] },
            ]),
            &["choices ou text", "Quelles bases ?"],
        ),
        (
            json!([
                { "question": "Quelles bases ?", "choices": ["SQLite"] },
                { "question": "Quel thème ?" },
            ]),
            &["choices ou text", "Quel thème ?"],
        ),
        (
            json!([
                { "question": "Quelles bases ?", "choices": ["SQLite"] },
                { "question": "Quel thème ?", "text": "   " },
            ]),
            &["choices ou text", "Quel thème ?"],
        ),
        // An option the question does not offer: its options are told, and the free text.
        (
            json!([
                { "question": "Quelles bases ?", "choices": ["Oracle"] },
                { "question": "Quel thème ?", "choices": ["Clair"] },
            ]),
            &["« Oracle »", "SQLite, Postgres, MySQL", "text"],
        ),
        // Several options where only one fits.
        (
            json!([
                { "question": "Quelles bases ?", "choices": ["SQLite"] },
                { "question": "Quel thème ?", "choices": ["Clair", "Sombre"] },
            ]),
            &["n’accepte qu’un choix", "Quel thème ?"],
        ),
    ];
    for (answers, parts) in cases {
        let why = failed(answer(answers.clone()).await);
        for part in parts {
            assert!(why.contains(part), "{answers}: {part} not in {why}");
        }
        assert_eq!(last_entry(&h).3, "error", "{answers}");
        assert_eq!(waiting(&h, &a.id), ["rq"], "{answers}");
    }
    // Several for a question that allows it, the labels in any case; one for the other; the text
    // of a question kept as the agent asked it.
    let v = read(
        &c,
        "answer_question",
        json!({
            "agent": a.id, "request": "rq",
            "answers": [
                { "question": "QUEL THÈME ?", "choices": ["sombre"] },
                { "question": "Quelles bases ?", "choices": ["sqlite", "MySQL", "SQLite"] },
            ],
        }),
    )
    .await;
    assert_eq!(
        v["answers"],
        json!({ "Quelles bases ?": "SQLite, MySQL", "Quel thème ?": "Sombre" })
    );
    let card = item(&h, &a.id, "rq");
    assert_eq!(card["answers"], v["answers"]);
    assert!(waiting(&h, &a.id).is_empty());
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_answer_is_refused_for_always_oneself_a_private_project_a_wrong_kind_or_a_request_gone()
{
    let h = harness("mcp-req-refusals");
    // Every agent may answer another (ESC-423): what is refused here is refused all the same.
    h.core.settings.write().agents_answer = AgentsAnswer::All;
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    let perm = asking(&h, &p, "agent-perm", "permission").await;
    let quest = asking(&h, &p, "agent-question", "question").await;
    let away = put_agent(&h, &other, "agent-ailleurs", 9);
    let c = external(&h).await;
    let nothing_changed = |h: &Harness| {
        assert_eq!(waiting(h, &perm.id), ["req_perm"]);
        assert_eq!(waiting(h, &quest.id), ["req_question"]);
    };

    // « Toujours autoriser » is the user's, in the window: refused, and the request still waits.
    let always = refused(
        &c,
        "answer_permission",
        json!({ "agent": perm.id, "request": "req_perm", "decision": "always" }),
    )
    .await;
    assert!(
        always.contains("Toujours autoriser") && always.contains("fenêtre"),
        "{always}"
    );
    assert_eq!(last_entry(&h).3, "refused");
    nothing_changed(&h);
    // A decision that is none: the router’s refusal, which does not repeat what was given and sends
    // back to the schema, where the two choices are listed, never « always ».
    let bad = refused(
        &c,
        "answer_permission",
        json!({ "agent": perm.id, "request": "req_perm", "decision": "peut-être" }),
    )
    .await;
    assert!(
        bad.contains("schéma") && !bad.contains("peut-être") && !bad.contains("always"),
        "{bad}"
    );
    nothing_changed(&h);

    // Not its own requests, when it is an agent.
    let from_perm = as_agent(&h, &perm.id).await;
    let itself = refused(
        &from_perm,
        "answer_permission",
        json!({ "agent": "AGENT-PERM", "request": "req_perm", "decision": "allow" }),
    )
    .await;
    assert!(
        itself.contains("ne répond pas à ses propres demandes"),
        "{itself}"
    );
    let logged = last_entry(&h);
    assert_eq!(
        (logged.0.as_str(), logged.1.as_str(), logged.3),
        ("agent-perm", "answer_permission", "refused")
    );
    let itself = refused(
        &from_perm,
        "answer_question",
        json!({ "agent": perm.id, "request": "req_perm", "answers": [{ "text": "x" }] }),
    )
    .await;
    assert!(
        itself.contains("ne répond pas à ses propres demandes"),
        "{itself}"
    );
    nothing_changed(&h);
    // It answers the requests of another agent of its project.
    let listed = read(
        &from_perm,
        "list_requests",
        json!({ "agent": "agent-question" }),
    )
    .await;
    assert_eq!(ids(&listed), ["req_question"]);
    from_perm.cancel().await.unwrap();

    // The agent of a private project acts on its own alone: the others are not found.
    let other = make_private(&h, &other);
    let from_away = as_agent(&h, &away.id).await;
    for (tool, args) in [
        (
            "answer_permission",
            json!({ "agent": perm.id, "request": "req_perm", "decision": "allow" }),
        ),
        (
            "answer_question",
            json!({ "agent": quest.id, "request": "req_question", "answers": [{ "text": "x" }] }),
        ),
    ] {
        let why = refused(&from_away, tool, args).await;
        assert!(why.starts_with("Aucun agent «"), "{tool}: {why}");
        assert!(
            !why.contains("agent-perm") && !why.contains("demo"),
            "{tool}: {why}"
        );
        let logged = last_entry(&h);
        assert_eq!(
            (logged.0.as_str(), logged.1.as_str(), logged.3),
            ("agent-ailleurs", tool, "error")
        );
    }
    nothing_changed(&h);
    from_away.cancel().await.unwrap();
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == other.id)
        .unwrap()
        .private = false;

    // A permission is no question, nor a question a permission.
    let wrong = refused(
        &c,
        "answer_question",
        json!({ "agent": perm.id, "request": "req_perm", "answers": [{ "text": "x" }] }),
    )
    .await;
    assert!(wrong.contains("answer_permission"), "{wrong}");
    let wrong = refused(
        &c,
        "answer_permission",
        json!({ "agent": quest.id, "request": "req_question", "decision": "allow" }),
    )
    .await;
    assert!(wrong.contains("answer_question"), "{wrong}");
    // A message only goes with a refusal.
    let said = refused(
        &c,
        "answer_permission",
        json!({ "agent": perm.id, "request": "req_perm", "decision": "allow", "message": "oui" }),
    )
    .await;
    assert!(said.contains("refus"), "{said}");
    // A request that is none of the agent's, an agent that does not exist.
    let unknown = refused(
        &c,
        "answer_permission",
        json!({ "agent": perm.id, "request": "req_question", "decision": "allow" }),
    )
    .await;
    assert!(unknown.contains("n’attend plus de réponse"), "{unknown}");
    // It says which ones wait, for a mistyped id is no answer from someone else.
    assert!(
        unknown.contains("req_perm") && !unknown.contains("req_question"),
        "{unknown}"
    );
    let none_waiting = refused(
        &c,
        "answer_permission",
        json!({ "agent": away.id, "request": "r1", "decision": "allow" }),
    )
    .await;
    assert!(
        none_waiting.contains("n’attend plus de réponse") && none_waiting.contains("aucune"),
        "{none_waiting}"
    );
    let nobody = refused(
        &c,
        "answer_permission",
        json!({ "agent": "zzz", "request": "req_perm", "decision": "allow" }),
    )
    .await;
    assert!(nobody.contains("Aucun agent « zzz »"), "{nobody}");
    nothing_changed(&h);

    // The user answers first, in the window: the server's answer is too late, and says so.
    h.core
        .answer_permission(&perm.id, "req_perm", "deny", Some("non".into()))
        .unwrap();
    let late = refused(
        &c,
        "answer_permission",
        json!({ "agent": perm.id, "request": "req_perm", "decision": "allow" }),
    )
    .await;
    assert!(
        late.contains("n’attend plus de réponse") && late.contains("l’utilisateur"),
        "{late}"
    );
    // The card keeps the user's answer, with nobody else's name on it.
    h.wait("the turn to end", |h| {
        h.agent(&perm.id).status == AgentStatus::Done
    })
    .await;
    let card = item(&h, &perm.id, "req_perm");
    assert_eq!(card["decision"], "deny");
    assert!(card.get("by").is_none(), "{card}");
    // An agent whose process is gone has nothing to answer.
    let p = h.core.agent(&quest.id).unwrap().lock().detach().unwrap();
    p.close_input();
    h.wait("the process to exit", |_| !p.is_alive()).await;
    let gone = refused(
        &c,
        "answer_question",
        json!({ "agent": quest.id, "request": "req_question", "answers": [{ "text": "x" }] }),
    )
    .await;
    assert!(gone.contains("n’attend plus de réponse"), "{gone}");
    c.cancel().await.unwrap();

    // Once its project is not private, it answers the requests of another project as of its own.
    let demo = h.core.project(&perm.project_id).unwrap();
    let third = asking(&h, &demo, "agent-trois", "permission").await;
    let from_away = as_agent(&h, &away.id).await;
    read(
        &from_away,
        "answer_permission",
        json!({ "agent": third.id, "request": "req_perm", "decision": "allow" }),
    )
    .await;
    assert!(waiting(&h, &third.id).is_empty());
    let logged = last_entry(&h);
    assert_eq!(
        (logged.0.as_str(), logged.1.as_str(), logged.3),
        ("agent-ailleurs", "answer_permission", "ok")
    );
    from_away.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn two_answers_at_once_to_a_request_let_one_through() {
    let h = harness("mcp-req-twice");
    let p = project(&h).await;
    let a = asking(&h, &p, "agent-1", "permission").await;
    let c = external(&h).await;
    let (first, second) = tokio::join!(
        call(
            &c,
            "answer_permission",
            json!({ "agent": a.id, "request": "req_perm", "decision": "allow" })
        ),
        call(
            &c,
            "answer_permission",
            json!({ "agent": a.id, "request": "req_perm", "decision": "deny" })
        ),
    );
    let outcomes = [first.1, second.1];
    assert_eq!(
        outcomes.iter().filter(|error| !**error).count(),
        1,
        "{first:?} {second:?}"
    );
    // The one that came second reads that it was answered already.
    let (late, _) = if first.1 { &first } else { &second };
    assert!(late.contains("n’attend plus de réponse"), "{late}");
    h.wait("the turn to end", |h| {
        h.agent(&a.id).status == AgentStatus::Done
    })
    .await;
    // One decision, one answer sent to Claude.
    let card = item(&h, &a.id, "req_perm");
    assert!(
        card["decision"] == "allow" || card["decision"] == "deny",
        "{card}"
    );
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn an_answer_leaves_the_agent_running_and_tells_the_window_as_the_window_does() {
    let h = harness("mcp-req-window");
    let p = project(&h).await;
    let a = live(&h, &p, "agent-1").await;
    // Two requests: the agent waits until both are answered. Its fake `claude` ignores them.
    asks(&h, &a.id, "r1", bash("npm ci"));
    asks(&h, &a.id, "r2", bash("npm test"));
    let c = external(&h).await;
    read(
        &c,
        "answer_permission",
        json!({ "agent": a.id, "request": "r1", "decision": "allow" }),
    )
    .await;
    assert_eq!(waiting(&h, &a.id), ["r2"]);
    let view = last_view(&h, &a.id);
    assert_eq!(view["pending"], json!(["r2"]));
    assert_eq!(view["requests"].as_array().unwrap().len(), 1);
    // The last one answered: it works again, and the window is told so.
    read(
        &c,
        "answer_permission",
        json!({ "agent": a.id, "request": "r2", "decision": "deny" }),
    )
    .await;
    let view = last_view(&h, &a.id);
    assert_eq!(view["pending"], json!([]));
    assert_eq!(view["status"], "running");
    c.cancel().await.unwrap();
    h.core.mcp.stop();
}
