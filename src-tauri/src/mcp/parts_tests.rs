//! The MCP tools of a project's parts switched off (ESC-658), called by a real MCP client on a test core: gone from the
//! list its agents are given, refused with the reason when they are called for it all the same (by its agents, by Claude
//! outside Escouade naming it, by an agent of another project naming it), and back with their data once the part is on
//! again. Another project keeps all of them.

use super::tests::names;
use super::tools_tests::{
    as_agent, call, external, other_project, project, put, put_agent, read, refused, ticket,
};
use crate::core_tests::harness;
use crate::model::Column;
use crate::parts::{tools_of, Part};
use serde_json::json;

async fn listed(c: &super::tools_tests::Client, part: Part) -> usize {
    let all = names(&c.list_all_tools().await.unwrap());
    tools_of(part)
        .iter()
        .filter(|t| all.iter().any(|n| n == *t))
        .count()
}

#[test]
fn each_part_has_its_tools_and_memory_has_the_four_of_the_shared_memory() {
    assert!(tools_of(Part::Docs).contains(&"write_spec_page"));
    assert!(tools_of(Part::Kanban).contains(&"create_ticket"));
    // The workflows and the scheduled tasks go with the Kanban: their runs are tickets.
    assert!(tools_of(Part::Kanban).contains(&"get_workflow"));
    assert!(tools_of(Part::Kanban).contains(&"create_scheduled_task"));
    assert_eq!(
        tools_of(Part::Memory),
        vec![
            "search_memory",
            "read_memory",
            "write_memory",
            "forget_memory"
        ]
    );
    assert!(tools_of(Part::Launch).is_empty());
    // The essential is of no part.
    for tool in ["list_agents", "ask_user", "list_projects", "get_usage"] {
        assert_eq!(crate::parts::of_tool(tool), None, "{tool}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_part_off_its_tools_leave_its_agents_list_are_refused_for_it_and_come_back_with_their_data(
) {
    let h = harness("mcp-parts");
    let p = project(&h).await;
    let other = other_project(&h, "autre").await;
    put(&h, ticket(&p, "P-1", "Garder ce ticket", Column::Todo, 1));
    let me = put_agent(&h, &p, "Moi", 1);
    let them = put_agent(&h, &other, "Eux", 2);
    let c = as_agent(&h, &me.id).await;
    let outside = external(&h).await;
    let x = as_agent(&h, &them.id).await;

    // Data written while every part is on.
    read(
        &c,
        "write_memory",
        json!({ "name": "garde", "description": "Elle reste", "body": "Rien n’est effacé." }),
    )
    .await;
    read(
        &c,
        "write_spec_page",
        json!({ "project": p.id, "path": "general/garde", "body": "Elle reste." }),
    )
    .await;

    for part in [Part::Docs, Part::Memory, Part::Kanban] {
        h.core.set_project_part(&p.id, part, Some(false)).unwrap();
        // Gone from the list of its agents, not from another project's nor from Claude outside Escouade's.
        assert_eq!(listed(&c, part).await, 0, "{part:?}");
        assert_eq!(listed(&x, part).await, tools_of(part).len(), "{part:?}");
        assert_eq!(
            listed(&outside, part).await,
            tools_of(part).len(),
            "{part:?}"
        );
        let name = part.name(true);
        // Its own agent, without naming the project.
        let first = tools_of(part)[0];
        let why = refused(&c, first, json!({})).await;
        assert!(
            why.contains(name) && why.contains("demo") && why.contains("Parties du projet"),
            "{first}: {why}"
        );
        let args = |project: &str| match part {
            Part::Memory => json!({ "project": project, "query": "garde" }),
            _ => json!({ "project": project }),
        };
        // Anyone naming it.
        for client in [&outside, &x] {
            let why = refused(client, first, args(&p.id)).await;
            assert!(why.contains(name), "{first}: {why}");
        }
        // The log counts it as refused.
        let last = h.core.mcp.activity.entries().pop().unwrap();
        assert_eq!(last.outcome, "refused");
        // Another project has it.
        let (text, error) = call(&x, first, args(&other.id)).await;
        assert!(!error, "{first}: {text}");
        // On again: back, with its data.
        h.core.set_project_part(&p.id, part, Some(true)).unwrap();
        assert_eq!(listed(&c, part).await, tools_of(part).len(), "{part:?}");
    }
    // A ticket's tool named by its key: its project's Kanban.
    h.core
        .set_project_part(&p.id, Part::Kanban, Some(false))
        .unwrap();
    let why = refused(&outside, "get_ticket", json!({ "ticket": "P-1" })).await;
    assert!(why.contains("Kanban"), "{why}");
    h.core.set_project_part(&p.id, Part::Kanban, None).unwrap();

    let memory = read(&c, "read_memory", json!({ "name": "garde" })).await;
    assert!(memory["memory"]["body"]
        .as_str()
        .unwrap()
        .contains("Rien n’est effacé."));
    let page = read(
        &outside,
        "read_spec_page",
        json!({ "project": p.id, "path": "general/garde" }),
    )
    .await;
    assert!(page["body"].as_str().unwrap().contains("Elle reste."));
    let list = read(&outside, "list_tickets", json!({ "project": p.id })).await;
    assert!(list.to_string().contains("Garder ce ticket"), "{list}");
    for client in [c, outside, x] {
        client.cancel().await.unwrap();
    }
    h.core.mcp.stop();
}
