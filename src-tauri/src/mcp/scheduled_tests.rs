//! The tools of the scheduled tasks (ESC-448), called by a real MCP client on a test core:
//! created, listed, changed, paused and deleted as the window does it, refused when what they are
//! given is no rhythm, and out of reach of an agent of another project when the project is
//! private (`private_tests` checks every name of it).

use super::private_tests::make_private;
use super::tools_tests::{as_agent, external, other_project, project, put_agent, read, refused};
use crate::core_tests::harness;
use serde_json::json;

#[tokio::test]
async fn a_scheduled_task_is_created_listed_changed_paused_and_deleted_by_the_tools() {
    let h = harness("mcp-scheduled");
    let p = project(&h).await;
    let c = external(&h).await;
    let made = read(
        &c,
        "create_scheduled_task",
        json!({
            "project": p.name,
            "title": "Revue des dépendances",
            "description": "Mets à jour les dépendances.",
            "criteria": ["Les tests passent"],
            "rhythm": "weekly",
            "days": [1, 4],
            "time": "07:30",
            "model": "sonnet",
        }),
    )
    .await;
    assert_eq!(made["rhythm"], "weekly");
    assert_eq!(made["days"], json!([1, 4]));
    assert_eq!(made["time"], "07:30");
    assert_eq!(made["cron"], "30 7 * * 1,4");
    assert_eq!(made["catchUp"], true);
    assert_eq!(made["paused"], false);
    assert_eq!(
        (made["model"].as_str(), made["effort"].as_str()),
        (Some("sonnet"), Some("auto"))
    );
    assert!(made["next"].as_str().unwrap().contains("T07:30:00"));
    assert!(made["description"]
        .as_str()
        .unwrap()
        .starts_with("Mets à jour les dépendances."));
    let id = made["id"].as_str().unwrap().to_string();
    // The window's list has it, made for the project.
    let task = h.core.scheduled_view(&id).unwrap().task;
    assert_eq!(task.project_id, p.id);
    let list = read(&c, "list_scheduled_tasks", json!({ "project": p.id })).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    // Named by its title, in any case: a cron rhythm now, nothing else changes.
    let changed = read(
        &c,
        "update_scheduled_task",
        json!({ "project": p.name, "task": "revue des DÉPENDANCES", "rhythm": "cron", "cron": "0 18 * * 5", "catchUp": false }),
    )
    .await;
    assert_eq!(
        (changed["rhythm"].as_str(), changed["cron"].as_str()),
        (Some("cron"), Some("0 18 * * 5"))
    );
    assert_eq!(changed["catchUp"], false);
    assert_eq!(changed["title"], "Revue des dépendances");
    assert!(changed.get("time").is_none());
    let paused = read(
        &c,
        "pause_scheduled_task",
        json!({ "project": p.name, "task": id }),
    )
    .await;
    assert_eq!(
        (paused["paused"].as_bool(), paused["next"].as_str()),
        (Some(true), None)
    );
    let resumed = read(
        &c,
        "pause_scheduled_task",
        json!({ "project": p.name, "task": id, "paused": false }),
    )
    .await;
    assert_eq!(resumed["paused"], false);
    let gone = read(
        &c,
        "delete_scheduled_task",
        json!({ "project": p.name, "task": id }),
    )
    .await;
    assert_eq!(gone["deleted"], true);
    assert!(h.core.scheduled_view(&id).is_err());
    let again = refused(
        &c,
        "delete_scheduled_task",
        json!({ "project": p.name, "task": id }),
    )
    .await;
    assert!(again.contains("list_scheduled_tasks"), "{again}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn a_rhythm_that_is_none_is_refused_and_nothing_is_made() {
    let h = harness("mcp-scheduled-refused");
    let p = project(&h).await;
    let c = external(&h).await;
    for (args, says) in [
        (json!({ "time": "07:00" }), "rythme"),
        (json!({ "rhythm": "weekly", "time": "07:00" }), "jour"),
        (json!({ "rhythm": "daily", "time": "7h" }), "HH:MM"),
        (
            json!({ "rhythm": "cron", "cron": "0 0 7 * * 1" }),
            "cinq champs",
        ),
        (json!({ "rhythm": "cron", "cron": "0 0 31 2 *" }), "jamais"),
    ] {
        let mut a = json!({ "project": p.id, "title": "Bancale" });
        a.as_object_mut()
            .unwrap()
            .extend(args.as_object().unwrap().clone());
        let text = refused(&c, "create_scheduled_task", a).await;
        assert!(text.contains(says), "{args}: {text}");
    }
    assert!(h.core.scheduled.read().is_empty());
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn an_agent_of_another_project_does_not_reach_a_private_projects_tasks() {
    let h = harness("mcp-scheduled-private");
    let open = project(&h).await;
    let vault = other_project(&h, "Coffre").await;
    let vault = make_private(&h, &vault);
    let c = external(&h).await;
    read(
        &c,
        "create_scheduled_task",
        json!({ "project": vault.id, "title": "Secret", "rhythm": "daily", "time": "08:00" }),
    )
    .await;
    c.cancel().await.unwrap();
    let alice = put_agent(&h, &open, "Alice", 1);
    let a = as_agent(&h, &alice.id).await;
    let text = refused(&a, "list_scheduled_tasks", json!({ "project": vault.name })).await;
    assert!(
        !text.contains("Secret") && !text.contains(&vault.id),
        "{text}"
    );
    let text = refused(
        &a,
        "delete_scheduled_task",
        json!({ "project": vault.id, "task": "Secret" }),
    )
    .await;
    assert!(!text.contains("Secret"), "{text}");
    assert_eq!(h.core.scheduled.read().len(), 1);
    a.cancel().await.unwrap();
}
