//! « Projet privé » (ESC-139), called by a real MCP client (rmcp's) on a test core: a private
//! project is not found by the agents of the other projects, whatever a tool names of it (its id,
//! its name, a ticket or an agent of it), in the same words as a project that does not exist; and
//! the agents of a private project see it alone. Claude outside Escouade sees every project, and
//! the projects that are not private see each other as before.

use super::tests::{ACTING, AGENTS_ONLY, EXPOSED, READING};
use super::tools_tests::{
    as_agent, call, external, link, other_project, project, put, put_agent, read, ticket, Client,
};
use crate::core_tests::{harness, Harness};
use crate::model::*;
use serde_json::{json, Value};

/// The asked name a tool is given in place of the private one: nothing has it.
const MISSING: &str = "zz-introuvable";

/// `p` made private, as « Projet privé » saves it.
pub(super) fn make_private(h: &Harness, p: &Project) -> Project {
    let mut projects = h.core.projects.write();
    let found = projects.iter_mut().find(|x| x.id == p.id).unwrap();
    found.private = true;
    found.clone()
}

/// Three projects: `open` and `neighbour`, not private, and `vault`, private; an agent and a
/// ticket in each (the agent on the ticket, which is in « En cours »).
struct World {
    h: Harness,
    open: Project,
    neighbour: Project,
    vault: Project,
    /// The agents, by project: `open`'s, `neighbour`'s, `vault`'s.
    agents: [AgentMeta; 3],
    /// The tickets, by project.
    tickets: [Ticket; 3],
}

async fn world(name: &str) -> World {
    let h = harness(name);
    let open = project(&h).await;
    let neighbour = other_project(&h, "Voisin").await;
    let vault = other_project(&h, "Coffre").await;
    let vault = make_private(&h, &vault);
    let tickets = [
        put(&h, ticket(&open, "PUB-1", "Ouvert", Column::Doing, 1)),
        put(&h, ticket(&neighbour, "VOI-1", "Voisin", Column::Doing, 2)),
        put(&h, ticket(&vault, "SEC-1", "Secret", Column::Doing, 3)),
    ];
    let agents = [
        put_agent(&h, &open, "Alice", 1),
        put_agent(&h, &neighbour, "Victor", 2),
        put_agent(&h, &vault, "Sigrid", 3),
    ];
    for (a, t) in agents.iter().zip(&tickets) {
        link(&h, &a.id, &t.id);
    }
    World {
        h,
        open,
        neighbour,
        vault,
        agents,
        tickets,
    }
}

/// What a tool names, and the ways it may name it.
#[derive(Clone, Copy)]
enum Names {
    Project,
    Ticket,
    Agent,
}

/// Every tool that names a project, a ticket or an agent, with the arguments it is called with
/// (`{}` where the name goes): enough for each to get to finding what it names.
fn naming_calls() -> Vec<(&'static str, Names, Value)> {
    vec![
        ("list_agents", Names::Project, json!({ "project": "{}" })),
        ("list_tickets", Names::Project, json!({ "project": "{}" })),
        ("get_board", Names::Project, json!({ "project": "{}" })),
        ("get_workflow", Names::Project, json!({ "project": "{}" })),
        ("list_requests", Names::Project, json!({ "project": "{}" })),
        ("get_spec_tree", Names::Project, json!({ "project": "{}" })),
        (
            "read_spec_page",
            Names::Project,
            json!({ "project": "{}", "path": "a/b" }),
        ),
        (
            "write_spec_page",
            Names::Project,
            json!({ "project": "{}", "path": "a/b", "body": "Texte" }),
        ),
        (
            "search_spec",
            Names::Project,
            json!({ "project": "{}", "query": "texte" }),
        ),
        (
            "index_search",
            Names::Project,
            json!({ "project": "{}", "query": "texte" }),
        ),
        (
            "index_context",
            Names::Project,
            json!({ "project": "{}", "task": "texte" }),
        ),
        (
            "index_body",
            Names::Project,
            json!({ "project": "{}", "symbol": "texte" }),
        ),
        (
            "index_callers",
            Names::Project,
            json!({ "project": "{}", "name": "texte" }),
        ),
        (
            "index_callees",
            Names::Project,
            json!({ "project": "{}", "name": "texte" }),
        ),
        (
            "index_impact",
            Names::Project,
            json!({ "project": "{}", "target": "texte" }),
        ),
        ("list_decisions", Names::Project, json!({ "project": "{}" })),
        (
            "read_decision",
            Names::Project,
            json!({ "project": "{}", "number": 1 }),
        ),
        (
            "propose_decision",
            Names::Project,
            json!({ "project": "{}", "title": "Une décision" }),
        ),
        (
            "replace_decision",
            Names::Project,
            json!({ "project": "{}", "number": 1, "title": "Une autre" }),
        ),
        ("list_documents", Names::Project, json!({ "project": "{}" })),
        (
            "get_docs_to_share",
            Names::Project,
            json!({ "project": "{}" }),
        ),
        (
            "share_docs",
            Names::Project,
            json!({ "project": "{}", "userAskedForIt": true }),
        ),
        (
            "read_document",
            Names::Project,
            json!({ "project": "{}", "path": "a.md" }),
        ),
        (
            "search_documents",
            Names::Project,
            json!({ "project": "{}", "query": "plan" }),
        ),
        (
            "add_document",
            Names::Project,
            json!({ "project": "{}", "name": "a.md", "text": "x" }),
        ),
        (
            "search_memory",
            Names::Project,
            json!({ "project": "{}", "query": "e2e" }),
        ),
        (
            "read_memory",
            Names::Project,
            json!({ "project": "{}", "name": "e2e" }),
        ),
        (
            "write_memory",
            Names::Project,
            json!({ "project": "{}", "name": "e2e", "description": "d", "body": "b" }),
        ),
        (
            "forget_memory",
            Names::Project,
            json!({ "project": "{}", "name": "e2e" }),
        ),
        (
            "create_ticket",
            Names::Project,
            json!({ "project": "{}", "title": "Un ticket" }),
        ),
        (
            "create_agent",
            Names::Project,
            json!({ "project": "{}", "message": "Bonjour" }),
        ),
        (
            "set_board",
            Names::Project,
            json!({ "project": "{}", "maxParallel": 2 }),
        ),
        ("get_ticket", Names::Ticket, json!({ "ticket": "{}" })),
        (
            "get_ticket_image",
            Names::Ticket,
            json!({ "ticket": "{}", "index": 0 }),
        ),
        (
            "get_ticket_file",
            Names::Ticket,
            json!({ "ticket": "{}", "index": 0 }),
        ),
        ("wait_ticket", Names::Ticket, json!({ "tickets": ["{}"] })),
        (
            "update_ticket",
            Names::Ticket,
            json!({ "ticket": "{}", "title": "Autre" }),
        ),
        (
            "move_ticket",
            Names::Ticket,
            json!({ "ticket": "{}", "position": "top" }),
        ),
        ("start_ticket", Names::Ticket, json!({ "ticket": "{}" })),
        ("approve_ticket", Names::Ticket, json!({ "ticket": "{}" })),
        (
            "reject_ticket",
            Names::Ticket,
            json!({ "ticket": "{}", "comment": "Non" }),
        ),
        ("resume_ticket", Names::Ticket, json!({ "ticket": "{}" })),
        ("resolve_conflict", Names::Ticket, json!({ "ticket": "{}" })),
        ("dismiss_block", Names::Ticket, json!({ "ticket": "{}" })),
        ("delete_ticket", Names::Ticket, json!({ "ticket": "{}" })),
        ("get_agent_summary", Names::Agent, json!({ "agent": "{}" })),
        (
            "list_background_work",
            Names::Agent,
            json!({ "agent": "{}" }),
        ),
        (
            "get_background_output",
            Names::Agent,
            json!({ "agent": "{}", "id": "toolu_1" }),
        ),
        ("list_requests", Names::Agent, json!({ "agent": "{}" })),
        (
            "send_message",
            Names::Agent,
            json!({ "agent": "{}", "text": "Bonjour" }),
        ),
        ("stop_agent", Names::Agent, json!({ "agent": "{}" })),
        (
            "message_subagent",
            Names::Agent,
            json!({ "agent": "{}", "subagent": "toolu_1", "text": "Bonjour" }),
        ),
        (
            "stop_background_work",
            Names::Agent,
            json!({ "agent": "{}", "id": "toolu_1" }),
        ),
        (
            "answer_permission",
            Names::Agent,
            json!({ "agent": "{}", "request": "r1", "decision": "allow" }),
        ),
        (
            "answer_question",
            Names::Agent,
            json!({ "agent": "{}", "request": "r1", "answers": [{ "text": "oui" }] }),
        ),
    ]
}

/// `args` with `asked` where the name goes.
fn with_name(args: &Value, asked: &str) -> Value {
    serde_json::from_str(&args.to_string().replace("{}", asked)).unwrap()
}

/// The ways `p`'s project, ticket and agent are named: by id and by name (or key).
fn names_of(w: &World, i: usize, what: Names) -> Vec<String> {
    let (p, t, a) = (
        [&w.open, &w.neighbour, &w.vault][i],
        &w.tickets[i],
        &w.agents[i],
    );
    match what {
        Names::Project => vec![p.id.clone(), p.name.clone(), p.name.to_uppercase()],
        Names::Ticket => vec![t.key.clone(), t.id.clone(), t.key.to_lowercase()],
        Names::Agent => vec![a.id.clone(), a.name.clone()],
    }
}

/// What may tell of the project `i`: its id, its name, its folder, its ticket and its agent.
fn traces(w: &World, i: usize) -> Vec<String> {
    let (p, t, a) = (
        [&w.open, &w.neighbour, &w.vault][i],
        &w.tickets[i],
        &w.agents[i],
    );
    vec![
        p.id.clone(),
        p.name.clone(),
        t.key.clone(),
        t.id.clone(),
        t.title.clone(),
        a.id.clone(),
        a.name.clone(),
    ]
}

/// Every naming tool, called by `c` with each name of what the project `hidden` has: each answers
/// as it does for a name nothing has, word for word, and tells nothing of `hidden`.
async fn not_found_anywhere(w: &World, c: &Client, hidden: usize) {
    let traces = traces(w, hidden);
    for (tool, what, args) in naming_calls() {
        let (missing, error) = call(c, tool, with_name(&args, MISSING)).await;
        assert!(error, "{tool}: {missing}");
        assert!(missing.contains(MISSING), "{tool}: {missing}");
        for asked in names_of(w, hidden, what) {
            let (text, error) = call(c, tool, with_name(&args, &asked)).await;
            assert!(error, "{tool} {asked}: {text}");
            assert_eq!(text.replace(&asked, MISSING), missing, "{tool} {asked}");
            // What it was asked by is said back, as for any name: an id holds its project's.
            for trace in traces.iter().filter(|t| !asked.contains(t.as_str())) {
                assert!(!text.contains(trace.as_str()), "{tool} {asked}: {text}");
            }
        }
    }
}

/// The ids of the rows of a list.
fn ids(rows: &Value) -> Vec<String> {
    let mut ids: Vec<String> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids
}

fn sorted(mut ids: Vec<&str>) -> Vec<String> {
    ids.sort();
    ids.into_iter().map(String::from).collect()
}

#[tokio::test]
async fn every_tool_that_names_something_is_in_the_tests_of_the_private_projects() {
    // A tool added to the server is named here, or said to name nothing.
    let names_nothing = [
        "list_projects",
        "get_usage",
        "report_progress",
        "split_ticket",
    ];
    let named: Vec<&str> = naming_calls().iter().map(|(t, _, _)| *t).collect();
    for tool in EXPOSED {
        assert!(
            named.contains(tool) || names_nothing.contains(tool),
            "{tool} is not tested for the private projects"
        );
    }
    for tool in READING.iter().chain(ACTING).chain(AGENTS_ONLY) {
        assert!(EXPOSED.contains(tool), "{tool}");
    }
}

#[tokio::test]
async fn a_private_project_is_not_found_by_the_agents_of_the_other_projects() {
    let w = world("mcp-private-outsider").await;
    let c = as_agent(&w.h, &w.agents[0].id).await;
    not_found_anywhere(&w, &c, 2).await;
    // Nothing of it was changed, nor made, by the calls.
    let vault = w.h.core.ticket(&w.tickets[2].id).unwrap();
    assert_eq!(
        (vault.title.as_str(), vault.column),
        ("Secret", Column::Doing)
    );
    let tickets = w.h.core.tickets.read().clone();
    assert_eq!(
        tickets
            .iter()
            .filter(|t| t.project_id == w.vault.id)
            .count(),
        1
    );
    let agents = w.h.core.agents.read().len();
    assert_eq!(agents, 3);
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn the_agents_of_a_private_project_find_no_other_project() {
    let w = world("mcp-private-insider").await;
    let c = as_agent(&w.h, &w.agents[2].id).await;
    // Neither a project that is not private, nor another private one.
    not_found_anywhere(&w, &c, 0).await;
    not_found_anywhere(&w, &c, 1).await;
    let other = other_project(&w.h, "Autre coffre").await;
    let other = make_private(&w.h, &other);
    let (text, error) = call(&c, "get_board", json!({ "project": other.name })).await;
    assert!(error && !text.contains(&other.id), "{text}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn a_ticket_comes_after_no_ticket_of_a_private_project_nor_the_other_way() {
    // ESC-142: `after` names tickets of the ticket's own project; a private one's is not found,
    // in the words of a ticket nothing has, and the other projects' are not found by its agents.
    let w = world("mcp-private-after").await;
    // A ticket « À faire » of each, the one whose `after` changes.
    let todo = [
        put(&w.h, ticket(&w.open, "PUB-2", "À faire", Column::Todo, 4)),
        put(&w.h, ticket(&w.vault, "SEC-2", "À faire", Column::Todo, 5)),
    ];
    for (caller, own, hidden) in [(0, 0, 2), (2, 2, 0), (2, 2, 1)] {
        let c = as_agent(&w.h, &w.agents[caller].id).await;
        let project = [&w.open, &w.neighbour, &w.vault][own].name.clone();
        let own_ticket = todo[own / 2].key.clone();
        let calls = [
            (
                "create_ticket",
                json!({ "project": project, "title": "Après", "after": ["{}"] }),
            ),
            (
                "update_ticket",
                json!({ "ticket": own_ticket, "after": ["{}"] }),
            ),
        ];
        let traces = traces(&w, hidden);
        for (tool, args) in calls {
            let (missing, error) = call(&c, tool, with_name(&args, MISSING)).await;
            assert!(error && missing.contains(MISSING), "{tool}: {missing}");
            for asked in names_of(&w, hidden, Names::Ticket) {
                let (text, error) = call(&c, tool, with_name(&args, &asked)).await;
                assert!(error, "{tool} {asked}: {text}");
                assert_eq!(text.replace(&asked, MISSING), missing, "{tool} {asked}");
                for trace in traces.iter().filter(|t| !asked.contains(t.as_str())) {
                    assert!(!text.contains(trace.as_str()), "{tool} {asked}: {text}");
                }
            }
        }
        c.cancel().await.unwrap();
    }
    // Nothing was made, nor does any ticket wait for another.
    let tickets = w.h.core.tickets.read().clone();
    assert_eq!(tickets.len(), 5);
    assert!(tickets.iter().all(|t| t.after.is_empty()));
}

#[tokio::test]
async fn the_lists_show_each_caller_the_projects_it_sees() {
    let w = world("mcp-private-lists").await;
    let (open, neighbour, vault) = (&w.open.id, &w.neighbour.id, &w.vault.id);
    let all_agents: Vec<&str> = w.agents.iter().map(|a| a.id.as_str()).collect();
    for (who, projects, agents) in [
        // An agent of a project that is not private: all but the private one.
        (
            Some(0),
            vec![open, neighbour],
            vec![all_agents[0], all_agents[1]],
        ),
        (
            Some(1),
            vec![open, neighbour],
            vec![all_agents[0], all_agents[1]],
        ),
        // An agent of the private one: it alone.
        (Some(2), vec![vault], vec![all_agents[2]]),
        // Claude outside Escouade, the user's own token: every one.
        (None, vec![open, neighbour, vault], all_agents.clone()),
    ] {
        let c = match who {
            Some(i) => as_agent(&w.h, &w.agents[i].id).await,
            None => external(&w.h).await,
        };
        let projects: Vec<&str> = projects.iter().map(|s| s.as_str()).collect();
        let listed = read(&c, "list_projects", json!({})).await;
        assert_eq!(ids(&listed), sorted(projects), "{who:?}: {listed}");
        let listed = read(&c, "list_agents", json!({})).await;
        assert_eq!(ids(&listed), sorted(agents.clone()), "{who:?}: {listed}");
        // No request waits: an empty list for every one, which names no project.
        let listed = read(&c, "list_requests", json!({})).await;
        assert_eq!(listed, json!([]), "{who:?}");
        // The quotas are the accounts': they name no project.
        let usage = read(&c, "get_usage", json!({})).await.to_string();
        assert!(
            !usage.contains(vault) && !usage.contains("Coffre"),
            "{usage}"
        );
        c.cancel().await.unwrap();
    }
}

#[tokio::test]
async fn the_requests_of_a_private_projects_agent_are_listed_to_its_project_and_the_user_alone() {
    use super::requests_tests::{asks, bash, live};
    let w = world("mcp-private-requests").await;
    // A permission each, from an agent of each project: the two that are not private and the
    // private one.
    let mut askers = Vec::new();
    for (p, name) in [
        (&w.open, "ask-a"),
        (&w.neighbour, "ask-b"),
        (&w.vault, "ask-c"),
    ] {
        let a = live(&w.h, p, name).await;
        asks(&w.h, &a.id, "r1", bash("ls"));
        askers.push(a);
    }
    let agents_of = |rows: &Value| {
        let mut ids: Vec<String> = rows
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["agent"]["id"].as_str().unwrap().to_string())
            .collect();
        ids.sort();
        ids
    };
    let ids: Vec<&str> = askers.iter().map(|a| a.id.as_str()).collect();
    let c = as_agent(&w.h, &w.agents[0].id).await;
    let rows = read(&c, "list_requests", json!({})).await;
    assert_eq!(agents_of(&rows), sorted(vec![ids[0], ids[1]]));
    let c = as_agent(&w.h, &w.agents[2].id).await;
    let rows = read(&c, "list_requests", json!({})).await;
    assert_eq!(agents_of(&rows), sorted(vec![ids[2]]));
    let c = external(&w.h).await;
    let rows = read(&c, "list_requests", json!({})).await;
    assert_eq!(agents_of(&rows), sorted(ids.clone()));
}

#[tokio::test]
async fn the_agents_of_a_private_project_read_and_act_in_it_as_before() {
    let w = world("mcp-private-own").await;
    let c = as_agent(&w.h, &w.agents[2].id).await;
    let vault = &w.vault;
    let board = read(&c, "list_tickets", json!({ "project": vault.name })).await;
    assert_eq!(board[0]["key"], "SEC-1");
    let t = read(&c, "get_ticket", json!({ "ticket": "sec-1" })).await;
    assert_eq!(t["key"], "SEC-1");
    let own = read(&c, "get_ticket", json!({})).await;
    assert_eq!(own["key"], "SEC-1");
    let settings = read(&c, "get_board", json!({ "project": vault.id })).await;
    assert_eq!(settings["projectId"], vault.id.as_str());
    let made = read(
        &c,
        "create_ticket",
        json!({ "project": vault.id, "title": "Dans le coffre" }),
    )
    .await;
    let made = read(&c, "get_ticket", json!({ "ticket": made["key"] })).await;
    assert_eq!(made["title"], "Dans le coffre");
    read(&c, "report_progress", json!({ "line": "Je travaille" })).await;
    let line =
        w.h.core
            .agent(&w.agents[2].id)
            .unwrap()
            .lock()
            .meta
            .progress_line
            .clone();
    assert_eq!(line.as_deref(), Some("Je travaille"));
    let split = read(
        &c,
        "split_ticket",
        json!({ "tickets": [{ "title": "Une part" }] }),
    )
    .await;
    let part =
        w.h.core
            .tickets
            .read()
            .iter()
            .find(|t| t.title == "Une part")
            .cloned()
            .unwrap();
    assert_eq!(part.project_id, vault.id, "{split}");
}

#[tokio::test]
async fn the_projects_that_are_not_private_still_read_each_other_and_the_user_reads_every_one() {
    let w = world("mcp-private-shared").await;
    let c = as_agent(&w.h, &w.agents[0].id).await;
    let t = read(&c, "get_ticket", json!({ "ticket": "VOI-1" })).await;
    assert_eq!(t["key"], "VOI-1");
    let summary = read(&c, "get_agent_summary", json!({ "agent": "Victor" })).await;
    assert_eq!(summary["id"], w.agents[1].id.as_str());
    let tickets = read(&c, "list_tickets", json!({ "project": "Voisin" })).await;
    assert_eq!(tickets[0]["key"], "VOI-1");
    let c = external(&w.h).await;
    let t = read(&c, "get_ticket", json!({ "ticket": "SEC-1" })).await;
    assert_eq!(t["key"], "SEC-1");
    let summary = read(&c, "get_agent_summary", json!({ "agent": "Sigrid" })).await;
    assert_eq!(summary["id"], w.agents[2].id.as_str());
    let settings = read(&c, "get_board", json!({ "project": "Coffre" })).await;
    assert_eq!(settings["projectId"], w.vault.id.as_str());
}

#[tokio::test]
async fn a_project_made_private_disappears_at_the_next_call_and_comes_back_once_it_is_not() {
    let w = world("mcp-private-switch").await;
    let c = as_agent(&w.h, &w.agents[0].id).await;
    let listed = read(&c, "list_projects", json!({})).await;
    assert_eq!(listed.as_array().unwrap().len(), 2);
    // The window's « Projet privé » turned off on the vault, then on for the neighbour.
    let mut vault = w.h.core.project(&w.vault.id).unwrap();
    vault.private = false;
    w.h.core.update_project(vault).unwrap();
    let mut neighbour = w.h.core.project(&w.neighbour.id).unwrap();
    neighbour.private = true;
    w.h.core.update_project(neighbour).unwrap();
    let listed = read(&c, "list_projects", json!({})).await;
    assert_eq!(
        ids(&listed),
        sorted(vec![w.open.id.as_str(), w.vault.id.as_str()])
    );
    let (text, error) = call(&c, "get_ticket", json!({ "ticket": "VOI-1" })).await;
    assert!(error && !text.contains("Voisin"), "{text}");
    // Saved with the project: read back as it was set.
    assert!(w.h.core.project(&w.neighbour.id).unwrap().private);
}

#[tokio::test]
async fn a_new_project_is_private_when_the_default_for_new_projects_says_so() {
    let h = harness("mcp-private-default");
    let first = other_project(&h, "Premier").await;
    assert!(!first.private, "off by default");
    h.core.settings.write().private_projects = true;
    let second = other_project(&h, "Second").await;
    assert!(second.private);
    // A project saved before the setting existed reads as not private.
    let mut saved = serde_json::to_value(&first).unwrap();
    saved.as_object_mut().unwrap().remove("private");
    let read_back: Project = serde_json::from_value(saved).unwrap();
    assert!(!read_back.private);
    let settings: Settings = serde_json::from_value(json!({})).unwrap();
    assert!(!settings.private_projects);
}
