//! `get_workflow` and the `workflow` of an agent in `list_agents` and `get_agent_summary` (ESC-198),
//! called by a real MCP client (rmcp's) on a test core: the steps, doors and ways back of a
//! project, the agents on each step, the history of an agent's steps, and nothing of a private
//! project for the agents of the others.

use super::private_tests::make_private;
use super::tools_tests::{as_agent, call, external, other_project, project, put_agent, read};
use crate::board::TurnEnd;
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::{json, Value};
use std::path::Path;

/// Two commands that name each other: `one` launches the sub-agent `worker` and goes to `two`,
/// which goes back to `one` when red.
fn write_workflow(dir: &Path) {
    let cmds = dir.join(".claude/commands");
    std::fs::create_dir_all(&cmds).unwrap();
    std::fs::write(
        cmds.join("one.md"),
        "Do one with subagent_type=\"worker\", then /two.\n",
    )
    .unwrap();
    std::fs::write(
        cmds.join("two.md"),
        "Do two. Back to /one when red, up to 3 times.\n",
    )
    .unwrap();
}

fn append(item: Value) -> ConvOp {
    ConvOp::Append { item }
}

fn user(h: &Harness, p: &Project, agent: &str, text: &str) {
    h.core.track_workflow(
        agent,
        &p.id,
        &[append(
            json!({"kind": "user", "id": format!("u{text}"), "text": text}),
        )],
        0.0,
        None,
    );
}

/// The agent `name` of `p` goes through `/one`, a sub-agent, the end of its turn (it cost 0.42),
/// then `/two`.
fn run(h: &Harness, p: &Project, name: &str, at: i64) -> AgentMeta {
    let a = put_agent(h, p, name, at);
    user(h, p, &a.id, "/one");
    h.core.track_workflow(
        &a.id,
        &p.id,
        &[append(
            json!({"kind": "tool", "id": "c1", "name": "Agent", "input": {"subagent_type": "worker", "prompt": "p"}, "parent": null}),
        )],
        0.0,
        None,
    );
    h.core.track_workflow(
        &a.id,
        &p.id,
        &[],
        0.42,
        Some(&TurnEnd::Finished("ok".into())),
    );
    user(h, p, &a.id, "/two");
    a
}

fn row<'a>(rows: &'a Value, id: &str) -> &'a Value {
    rows.as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("{id} not in {rows}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn get_workflow_gives_the_steps_the_ways_back_and_the_agents_on_each_step() {
    let h = harness("mcp-workflow-get");
    let p = project(&h).await;
    write_workflow(Path::new(&p.path));
    h.core.refresh_workflows(&p.id, true).await.unwrap();
    let alice = run(&h, &p, "Alice", 1);
    let bob = put_agent(&h, &p, "Bob", 2);
    user(&h, &p, &bob.id, "/one");
    let c = external(&h).await;

    let wf = read(&c, "get_workflow", json!({ "project": p.name })).await;
    assert_eq!(wf["projectId"], json!(p.id));
    let flows = wf["workflows"].as_array().unwrap();
    assert_eq!(flows.len(), 1, "{wf}");
    let flow = &flows[0];
    assert_eq!(flow["kind"], "cycle");
    let steps = flow["steps"].as_array().unwrap();
    let names: Vec<&str> = steps.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["one", "two"]);
    assert_eq!(steps[0]["agents"][0]["name"], "worker");
    assert!(!flow["doors"].as_array().unwrap().is_empty());
    let back = flow["transitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["back"] == true)
        .unwrap_or_else(|| panic!("no way back in {flow}"));
    assert_eq!(
        (back["from"].as_str(), back["to"].as_str()),
        (Some("two"), Some("one"))
    );
    assert_eq!(back["max"], 3);
    // The agents on each step.
    let on_one = steps[0]["agentsHere"].as_array().unwrap();
    assert_eq!(on_one.len(), 1, "{wf}");
    assert_eq!(
        (on_one[0]["id"].as_str(), on_one[0]["name"].as_str()),
        (Some(bob.id.as_str()), Some("Bob"))
    );
    let on_two = steps[1]["agentsHere"].as_array().unwrap();
    assert_eq!(on_two.len(), 1);
    assert_eq!(on_two[0]["id"], json!(alice.id));
    assert_eq!(on_two[0]["pass"], 1);
    assert!(wf["agentsElsewhere"].as_array().unwrap().is_empty());
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_project_without_workflow_answers_none_and_its_agents_have_a_null_workflow() {
    let h = harness("mcp-workflow-none");
    let p = project(&h).await;
    let a = put_agent(&h, &p, "Alice", 1);
    let c = external(&h).await;
    let wf = read(&c, "get_workflow", json!({ "project": p.id })).await;
    assert!(wf["workflows"].as_array().unwrap().is_empty(), "{wf}");
    let rows = read(&c, "list_agents", json!({})).await;
    assert!(row(&rows, &a.id)["workflow"].is_null());
    let s = read(&c, "get_agent_summary", json!({ "agent": a.id })).await;
    assert!(s["workflow"].is_null());
    let (text, error) = call(&c, "get_workflow", json!({ "project": "zz-introuvable" })).await;
    assert!(error, "{text}");
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn list_agents_and_the_summary_say_where_an_agent_is_and_the_summary_its_history() {
    let h = harness("mcp-workflow-agents");
    let p = project(&h).await;
    write_workflow(Path::new(&p.path));
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    let alice = run(&h, &p, "Alice", 1);
    let idle = put_agent(&h, &p, "Idle", 2);
    let c = external(&h).await;

    let rows = read(&c, "list_agents", json!({})).await;
    let w = &row(&rows, &alice.id)["workflow"];
    assert_eq!(w["name"], json!(scan.workflows[0].name));
    assert_eq!(w["step"], "two");
    assert!(w["turn"].is_null());
    assert!(w["since"].as_i64().unwrap() > 0);
    assert!(row(&rows, &idle.id)["workflow"].is_null());

    let s = read(&c, "get_agent_summary", json!({ "agent": "Alice" })).await;
    let w = &s["workflow"];
    assert_eq!(w["step"], "two");
    let history = w["history"].as_array().unwrap();
    assert_eq!(history.len(), 2, "{w}");
    assert_eq!(history[0]["step"], "one");
    assert_eq!(history[0]["outcome"], "done");
    assert_eq!(history[0]["did"], json!(["agent:worker"]));
    assert!((history[0]["cost"].as_f64().unwrap() - 0.42).abs() < 1e-9);
    assert_eq!(history[1]["step"], "two");
    assert_eq!(history[1]["outcome"], "running");
    // It does something else: outside the workflow, no step.
    let s = read(&c, "get_agent_summary", json!({ "agent": idle.id })).await;
    assert!(s["workflow"].is_null());
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_project_tells_nothing_of_its_workflows_to_the_agents_of_the_others() {
    let h = harness("mcp-workflow-private");
    let open = project(&h).await;
    let vault = other_project(&h, "Coffre").await;
    let vault = make_private(&h, &vault);
    write_workflow(Path::new(&open.path));
    write_workflow(Path::new(&vault.path));
    h.core.refresh_workflows(&open.id, true).await.unwrap();
    h.core.refresh_workflows(&vault.id, true).await.unwrap();
    let alice = run(&h, &open, "Alice", 1);
    let sigrid = run(&h, &vault, "Sigrid", 2);

    // An agent of the open project: the vault's workflows are not found, nor its agent's.
    let c = as_agent(&h, &alice.id).await;
    let (missing, error) = call(&c, "get_workflow", json!({ "project": "zz-introuvable" })).await;
    assert!(error);
    for asked in [vault.id.as_str(), "Coffre", "coffre"] {
        let (text, error) = call(&c, "get_workflow", json!({ "project": asked })).await;
        assert!(error, "{text}");
        assert_eq!(text.replace(asked, "zz-introuvable"), missing);
    }
    let rows = read(&c, "list_agents", json!({})).await;
    assert!(rows
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["id"] != json!(sigrid.id)));
    let (text, error) = call(&c, "get_agent_summary", json!({ "agent": sigrid.id })).await;
    assert!(error, "{text}");
    assert!(!text.contains("history"));
    let own = read(&c, "get_workflow", json!({ "project": open.id })).await;
    assert!(!own.to_string().contains(&sigrid.id));
    c.cancel().await.unwrap();

    // The vault's own agent sees its workflow and itself on it, and nothing of the other project.
    let c = as_agent(&h, &sigrid.id).await;
    let own = read(&c, "get_workflow", json!({ "project": "Coffre" })).await;
    let on_two = own["workflows"][0]["steps"][1]["agentsHere"]
        .as_array()
        .unwrap();
    assert_eq!(on_two[0]["id"], json!(sigrid.id));
    let (text, error) = call(&c, "get_workflow", json!({ "project": open.id })).await;
    assert!(error && !text.contains(&alice.id), "{text}");
    let rows = read(&c, "list_agents", json!({})).await;
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["workflow"]["step"], "two");
    c.cancel().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn get_workflow_gives_the_workflows_as_the_user_corrected_them() {
    let h = harness("mcp-workflow-fixed");
    let p = project(&h).await;
    write_workflow(Path::new(&p.path));
    let scan = h.core.refresh_workflows(&p.id, true).await.unwrap();
    let w = &scan.workflows[0];
    // « one » renamed in Settings › Projects › Kanban › Workflow.
    h.core
        .update_project(Project {
            workflow_fixes: vec![crate::workflowfix::Fix {
                workflow: w.id.clone(),
                steps: w
                    .steps
                    .iter()
                    .map(|s| crate::workflowfix::FixStep {
                        name: if s.name == "one" {
                            "Premier".into()
                        } else {
                            s.name.clone()
                        },
                        from: vec![s.name.clone()],
                        signals: Vec::new(),
                    })
                    .collect(),
                ..Default::default()
            }],
            ..h.core.project(&p.id).unwrap()
        })
        .unwrap();
    let c = external(&h).await;
    let got = read(&c, "get_workflow", json!({ "project": p.name })).await;
    let steps: Vec<&str> = got["workflows"][0]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(steps, ["Premier", "two"], "{got}");
    c.cancel().await.unwrap();
}
