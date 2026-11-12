//! Who answers another agent's permission or question through the server (ESC-423, audit
//! SEC-03): Claude outside Escouade always; an agent as « Les agents peuvent répondre aux
//! questions et aux permissions des autres agents » says (nobody by default, the chef the user
//! chose for the project, or every agent); the principal of a folder is not the chef.

use super::requests_tests::{asking, live, waiting};
use super::tools_tests::{as_agent, external, project, read, refused};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::json;

/// « Les agents peuvent répondre… » set to `to` (read by the server at each call).
fn answering(h: &Harness, to: AgentsAnswer) {
    h.core.settings.write().agents_answer = to;
}

/// « Chef d’escouade » of `p` set to `chef`, as the project's settings do.
fn designate(h: &Harness, p: &Project, chef: Option<&str>) {
    let mut q = h.core.project(&p.id).unwrap();
    q.chef = chef.map(str::to_string);
    h.core.update_project(q).unwrap();
}

fn allow(agent: &str) -> serde_json::Value {
    json!({ "agent": agent, "request": "req_perm", "decision": "allow" })
}

fn sqlite(agent: &str) -> serde_json::Value {
    json!({ "agent": agent, "request": "req_question", "answers": [{ "choices": ["SQLite"] }] })
}

#[test]
fn nobody_answers_for_another_agent_by_default_and_in_settings_saved_before() {
    assert_eq!(Settings::default().agents_answer, AgentsAnswer::Nobody);
    // Settings saved before ESC-423 have no such field: nobody either.
    let before: Settings = serde_json::from_value(json!({ "mcpEnabled": true })).unwrap();
    assert_eq!(before.agents_answer, AgentsAnswer::Nobody);
    // The three values as the window sends them.
    for (word, value) in [
        ("nobody", AgentsAnswer::Nobody),
        ("chef", AgentsAnswer::Chef),
        ("all", AgentsAnswer::All),
    ] {
        let s: Settings = serde_json::from_value(json!({ "agentsAnswer": word })).unwrap();
        assert_eq!(s.agents_answer, value);
        assert_eq!(serde_json::to_value(s).unwrap()["agentsAnswer"], word);
    }
    // A project saved before it names no chef, and one with none writes none.
    let p: Project = serde_json::from_value(json!({
        "id": "p", "name": "p", "path": "/p", "color": "", "worktreePerAgent": false,
        "createdAt": 0, "runCommands": [],
    }))
    .unwrap();
    assert_eq!(p.chef, None);
    assert!(serde_json::to_value(&p).unwrap().get("chef").is_none());
}

#[tokio::test]
async fn with_nobody_an_agent_answers_neither_a_permission_nor_a_question_the_chef_neither() {
    let h = harness("mcp-rights-nobody");
    let p = project(&h).await;
    let perm = asking(&h, &p, "demandeur", "permission").await;
    let quest = asking(&h, &p, "curieux", "question").await;
    let other = live(&h, &p, "autre").await;
    // Even the chef the user chose: « Personne » is nobody.
    let chef = live(&h, &p, "chef").await;
    designate(&h, &p, Some(&chef.id));
    for agent in [&other, &chef] {
        let c = as_agent(&h, &agent.id).await;
        let why = refused(&c, "answer_permission", allow("demandeur")).await;
        assert!(
            why.contains("ne répond pas aux permissions ni aux questions d’un autre agent"),
            "{why}"
        );
        assert!(
            why.contains("Réglages › Agents › Piloter Escouade"),
            "{why}"
        );
        let why = refused(&c, "answer_question", sqlite("curieux")).await;
        assert!(why.contains("attend la réponse de l’utilisateur"), "{why}");
        c.cancel().await.unwrap();
    }
    // Both still wait for the user.
    assert_eq!(waiting(&h, &perm.id), ["req_perm"]);
    assert_eq!(waiting(&h, &quest.id), ["req_question"]);
    // Reading is not answering: list_requests gives them to an agent all the same.
    let c = as_agent(&h, &other.id).await;
    let rows = read(&c, "list_requests", json!({ "project": p.id })).await;
    assert_eq!(rows.as_array().map(Vec::len), Some(2), "{rows}");
    c.cancel().await.unwrap();

    // Claude outside Escouade answers.
    let e = external(&h).await;
    read(&e, "answer_permission", allow("demandeur")).await;
    read(&e, "answer_question", sqlite("curieux")).await;
    h.wait("both answered", |h| {
        waiting(h, &perm.id).is_empty() && waiting(h, &quest.id).is_empty()
    })
    .await;
    e.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn with_the_chef_only_the_designated_agent_answers_and_says_so() {
    let h = harness("mcp-rights-chef");
    let p = project(&h).await;
    answering(&h, AgentsAnswer::Chef);
    let perm = asking(&h, &p, "demandeur", "permission").await;
    let other = live(&h, &p, "autre").await;
    let chef = live(&h, &p, "chef").await;

    // No chef chosen yet: nobody answers, and the refusal says where it is chosen.
    let c = as_agent(&h, &chef.id).await;
    let why = refused(&c, "answer_permission", allow("demandeur")).await;
    assert!(why.contains("Chef d’escouade"), "{why}");
    c.cancel().await.unwrap();

    designate(&h, &p, Some(&chef.id));
    // Another agent is refused.
    let c = as_agent(&h, &other.id).await;
    let why = refused(&c, "answer_permission", allow("demandeur")).await;
    assert!(why.contains("Seul le chef d’escouade"), "{why}");
    c.cancel().await.unwrap();
    assert_eq!(waiting(&h, &perm.id), ["req_perm"]);

    // The chef answers, and the conversation of the agent that asked says who did.
    let c = as_agent(&h, &chef.id).await;
    read(&c, "answer_permission", allow("demandeur")).await;
    h.wait("the permission answered", |h| {
        waiting(h, &perm.id).is_empty()
    })
    .await;
    let card = h
        .items(&perm.id)
        .into_iter()
        .find(|i| i["id"] == "req_perm")
        .unwrap();
    assert_eq!(card["by"], "chef");

    // A question too.
    let quest = asking(&h, &p, "curieux", "question").await;
    read(&c, "answer_question", sqlite("curieux")).await;
    h.wait("the question answered", |h| {
        waiting(h, &quest.id).is_empty()
    })
    .await;
    c.cancel().await.unwrap();

    // The chef taken back by the user: refused again.
    designate(&h, &p, None);
    let perm2 = asking(&h, &p, "encore", "permission").await;
    let c = as_agent(&h, &chef.id).await;
    refused(&c, "answer_permission", allow("encore")).await;
    c.cancel().await.unwrap();
    assert_eq!(waiting(&h, &perm2.id), ["req_perm"]);
    h.core.mcp.stop();
}

/// The audit's proof (SEC-03): an agent creates another, which asks to run a command, and
/// answers « allow » for it. The creator is the principal of the new agent's folder: no chef.
#[tokio::test]
async fn an_agent_that_creates_another_cannot_allow_its_permission() {
    let h = harness("mcp-rights-poc");
    let p = project(&h).await;
    h.set_board(&p.id, |s| s.max_parallel = 6);
    let a = live(&h, &p, "a").await;
    let from_a = as_agent(&h, &a.id).await;
    let v = read(
        &from_a,
        "create_agent",
        json!({ "project": p.id, "message": "permission" }),
    )
    .await;
    let b = v["id"].as_str().unwrap().to_string();
    h.wait("its request", |h| waiting(h, &b) == ["req_perm"])
        .await;
    let folder = crate::folders::folder_of(&h.core.project(&p.id).unwrap(), &b)
        .cloned()
        .unwrap();
    assert_eq!(folder.principal.as_deref(), Some(a.id.as_str()));
    for setting in [AgentsAnswer::Nobody, AgentsAnswer::Chef] {
        answering(&h, setting);
        refused(&from_a, "answer_permission", allow(&b)).await;
        assert_eq!(waiting(&h, &b), ["req_perm"], "{setting:?}");
    }
    from_a.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn with_every_agent_any_agent_answers_another() {
    let h = harness("mcp-rights-all");
    let p = project(&h).await;
    answering(&h, AgentsAnswer::All);
    let perm = asking(&h, &p, "demandeur", "permission").await;
    let quest = asking(&h, &p, "curieux", "question").await;
    let other = live(&h, &p, "autre").await;
    let c = as_agent(&h, &other.id).await;
    read(&c, "answer_permission", allow("demandeur")).await;
    read(&c, "answer_question", sqlite("curieux")).await;
    h.wait("both answered", |h| {
        waiting(h, &perm.id).is_empty() && waiting(h, &quest.id).is_empty()
    })
    .await;
    // Its own requests stay the user's.
    let mine = asking(&h, &p, "moi", "permission").await;
    let c2 = as_agent(&h, &mine.id).await;
    let why = refused(&c2, "answer_permission", allow("moi")).await;
    assert!(why.contains("ses propres demandes"), "{why}");
    c.cancel().await.unwrap();
    c2.cancel().await.unwrap();
    h.core.mcp.stop();
}

#[tokio::test]
async fn no_tool_of_the_server_names_the_chef() {
    let h = harness("mcp-rights-no-tool");
    let e = external(&h).await;
    for t in e.list_all_tools().await.unwrap() {
        let schema = serde_json::to_value(&t.input_schema).unwrap();
        assert!(schema["properties"].get("chef").is_none(), "{}", t.name);
    }
    e.cancel().await.unwrap();
    h.core.mcp.stop();
}
