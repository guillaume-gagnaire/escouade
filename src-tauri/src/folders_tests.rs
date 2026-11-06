use crate::core::blank_project;
use crate::folders::*;
use crate::model::*;
use serde_json::{json, Value};

fn project() -> Project {
    blank_project("/x")
}

fn members(ids: &[&'static str]) -> Vec<Member<'static>> {
    ids.iter()
        .enumerate()
        .map(|(i, id)| Member {
            id,
            created_at: i as i64,
            waiting: false,
            archived: false,
        })
        .collect()
}

fn folder(id: &str, principal: Option<&str>, agents: &[&str]) -> AgentFolder {
    AgentFolder {
        id: id.into(),
        name: id.into(),
        principal: principal.map(Into::into),
        agents: agents.iter().map(|a| a.to_string()).collect(),
        ..Default::default()
    }
}

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn settle_lists_old_agents_before_a_new_one() {
    let mut p = project();
    let m = members(&["a1", "a2", "a3"]);
    assert!(settle(&mut p, &m));
    assert_eq!(p.agent_order, ids(&["a1", "a2", "a3"]));
    assert!(!settle(&mut p, &m));
    add_agent(&mut p, "a4", None);
    assert_eq!(p.agent_order, ids(&["a1", "a2", "a3", "a4"]));
}

#[test]
fn move_item_refuses_a_folder_into_a_folder() {
    let mut p = project();
    p.agent_folders = vec![folder("f1", None, &[]), folder("f2", None, &[])];
    p.agent_order = ids(&["f1", "f2"]);
    assert!(move_item(&mut p, "f1", Some("f2"), None).is_err());
    assert!(move_item(&mut p, "ghost", None, None).is_err());
    assert!(move_item(&mut p, "f1", Some("ghost"), None).is_err());
    assert_eq!(p.agent_order, ids(&["f1", "f2"]));
}

#[test]
fn a_principal_moved_out_loses_its_star() {
    let mut p = project();
    p.agent_folders = vec![folder("f1", Some("a1"), &["a1", "a2"])];
    p.agent_order = ids(&["f1"]);
    move_item(&mut p, "a1", Some("f1"), Some(1)).unwrap();
    assert_eq!(p.agent_folders[0].principal.as_deref(), Some("a1"));
    assert_eq!(p.agent_folders[0].agents, ids(&["a2", "a1"]));
    move_item(&mut p, "a1", None, Some(0)).unwrap();
    assert_eq!(p.agent_folders[0].principal, None);
    assert_eq!(p.agent_folders[0].agents, ids(&["a2"]));
    assert_eq!(p.agent_order, ids(&["a1", "f1"]));
}

#[test]
fn removing_a_folder_puts_its_agents_in_its_place() {
    let mut p = project();
    p.agent_folders = vec![folder("f1", Some("a2"), &["a2", "a3"])];
    p.agent_order = ids(&["a1", "f1", "a4"]);
    remove_folder(&mut p, "f1").unwrap();
    assert!(p.agent_folders.is_empty());
    assert_eq!(p.agent_order, ids(&["a1", "a2", "a3", "a4"]));
    assert!(remove_folder(&mut p, "f1").is_err());
}

#[test]
fn remove_agent_clears_order_folder_and_principal() {
    let mut p = project();
    p.agent_folders = vec![folder("f1", Some("a1"), &["a1", "a2"])];
    p.agent_order = ids(&["f1", "a3"]);
    remove_agent(&mut p, "a1");
    remove_agent(&mut p, "a3");
    assert_eq!(p.agent_folders[0].principal, None);
    assert_eq!(p.agent_folders[0].agents, ids(&["a2"]));
    assert_eq!(p.agent_order, ids(&["f1"]));
    assert!(folder_of(&p, "a2").is_some());
    assert!(folder_of(&p, "a1").is_none());
}

#[test]
fn place_after_and_insert_folder_keep_the_levels() {
    let mut p = project();
    p.agent_folders = vec![folder("f1", None, &["a3"])];
    p.agent_order = ids(&["a1", "f1", "a2"]);
    place_after(&mut p, "a2", "a1");
    assert_eq!(p.agent_order, ids(&["a1", "a2", "f1"]));
    place_after(&mut p, "a2", "a3");
    assert_eq!(p.agent_folders[0].agents, ids(&["a3", "a2"]));
    insert_folder(&mut p, folder("f2", Some("a1"), &["a1", "a2"]), Some("a1"));
    assert_eq!(p.agent_order, ids(&["f2", "f1"]));
    assert_eq!(p.agent_folders[0].agents, ids(&["a3"]));
}

#[test]
fn old_state_reads_without_folders() {
    let mut project = serde_json::to_value(project()).unwrap();
    project["id"] = json!("p");
    let state = serde_json::from_value::<PersistedState>(json!({ "projects": [project] })).unwrap();
    assert!(state.projects[0].agent_folders.is_empty());
    assert!(state.projects[0].agent_order.is_empty());
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn shown_follows_the_shared_cases() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/sidebar-order.json")).unwrap();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let mut p = project();
        p.agent_folders = serde_json::from_value(case["project"]["agentFolders"].clone()).unwrap();
        p.agent_order = serde_json::from_value(case["project"]["agentOrder"].clone()).unwrap();
        let agents = case["agents"].as_array().unwrap();
        let m: Vec<Member> = agents
            .iter()
            .map(|a| Member {
                id: a["id"].as_str().unwrap(),
                created_at: a["createdAt"].as_i64().unwrap(),
                waiting: a["status"] == "waiting",
                archived: a["archived"].as_bool().unwrap(),
            })
            .collect();
        assert_eq!(shown_agents(&p, &m), strings(&case["shown"]), "{name}");
        let items: Vec<Value> = shown(&p, &m)
            .into_iter()
            .map(|s| match s {
                Shown::Agent(id) => json!(["agent", id]),
                Shown::Folder { id, agents } => json!(["folder", id, agents]),
            })
            .collect();
        assert_eq!(Value::Array(items), case["items"], "{name}");
        // Settling is idempotent and does not change what is shown.
        let mut q = p.clone();
        settle(&mut q, &m);
        assert!(!settle(&mut q, &m), "{name}");
        assert_eq!(shown_agents(&q, &m), strings(&case["shown"]), "{name}");
    }
}
