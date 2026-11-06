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

mod core_hooks {
    use crate::core::{AgentOptions, Core};
    use crate::core_tests::{harness, Harness};
    use tauri::test::mock_app;

    async fn agent(h: &Harness, pid: &str, folder: Option<&str>) -> String {
        h.core
            .create_agent_with(
                pid,
                AgentOptions {
                    folder: folder.map(Into::into),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .meta
            .id
    }

    fn order(h: &Harness, pid: &str) -> Vec<String> {
        h.core.project(pid).unwrap().agent_order
    }

    fn folder_agents(h: &Harness, pid: &str, f: &str) -> Vec<String> {
        h.core
            .project(pid)
            .unwrap()
            .agent_folders
            .iter()
            .find(|x| x.id == f)
            .unwrap()
            .agents
            .clone()
    }

    #[tokio::test]
    async fn a_new_agent_goes_to_the_bottom_or_into_its_folder() {
        let h = harness("fo-new");
        let (p, _) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        let f = h
            .core
            .folder_create(&p.id, None, std::slice::from_ref(&a), None, Some(&a))
            .unwrap();
        let b = agent(&h, &p.id, Some(&f.id)).await;
        let c = agent(&h, &p.id, None).await;
        let d = agent(&h, &p.id, Some("gone")).await;
        assert_eq!(folder_agents(&h, &p.id, &f.id), vec![a, b]);
        assert_eq!(order(&h, &p.id), vec![f.id, c, d]);
    }

    #[tokio::test]
    async fn the_copy_follows_its_original_in_its_folder() {
        let h = harness("fo-copy");
        let (p, _) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        let b = agent(&h, &p.id, None).await;
        let f = h
            .core
            .folder_create(&p.id, None, &[a.clone(), b.clone()], None, None)
            .unwrap();
        let copy = h.core.duplicate_agent(&a).await.unwrap().meta.id;
        // Placed after its original at the original's level: inside the folder.
        assert_eq!(folder_agents(&h, &p.id, &f.id), vec![a, copy, b]);
    }

    #[tokio::test]
    async fn a_deleted_agent_leaves_its_folder() {
        let h = harness("fo-del");
        let (p, _) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        let b = agent(&h, &p.id, None).await;
        let f = h
            .core
            .folder_create(&p.id, None, &[a.clone(), b.clone()], Some(&a), None)
            .unwrap();
        h.core.delete_agent(&a, false).await.unwrap();
        let pf = h.core.project(&p.id).unwrap().agent_folders;
        assert_eq!(pf[0].agents, vec![b]);
        assert_eq!(pf[0].principal, None);
        assert_eq!(pf[0].id, f.id);
    }

    #[tokio::test]
    async fn an_archived_principal_comes_back_as_principal() {
        let h = harness("fo-arch");
        let (p, _) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        let b = agent(&h, &p.id, None).await;
        let f = h
            .core
            .folder_create(&p.id, None, &[a.clone(), b.clone()], Some(&a), None)
            .unwrap();
        h.core.archive_agent(&a, true).await.unwrap();
        let fo = h.core.project(&p.id).unwrap().agent_folders[0].clone();
        assert_eq!(fo.principal.as_deref(), Some(a.as_str()));
        assert_eq!(h.core.shown_agents_of(&p.id), vec![b.clone()]);
        h.core.archive_agent(&a, false).await.unwrap();
        let fo = h.core.project(&p.id).unwrap().agent_folders[0].clone();
        assert_eq!(fo.principal.as_deref(), Some(a.as_str()));
        assert_eq!(fo.id, f.id);
        assert_eq!(h.core.shown_agents_of(&p.id), vec![a, b]);
    }

    #[tokio::test]
    async fn a_restored_agent_comes_back_at_the_bottom_of_its_folder() {
        let h = harness("fo-restore");
        let (p, _) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        let b = agent(&h, &p.id, None).await;
        let c = agent(&h, &p.id, None).await;
        let f = h
            .core
            .folder_create(&p.id, None, &[a.clone(), b.clone()], None, None)
            .unwrap();
        h.core.archive_agent(&a, true).await.unwrap();
        h.core.archive_agent(&a, false).await.unwrap();
        assert_eq!(folder_agents(&h, &p.id, &f.id), vec![b, a]);
        h.core.archive_agent(&c, true).await.unwrap();
        h.core.archive_agent(&c, false).await.unwrap();
        assert_eq!(order(&h, &p.id), vec![f.id, c]);
    }

    #[tokio::test]
    async fn an_auto_named_folder_follows_its_principal_until_renamed() {
        let h = harness("fo-name");
        let (p, _) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        let f = h
            .core
            .folder_create(&p.id, None, std::slice::from_ref(&a), Some(&a), None)
            .unwrap();
        assert_eq!(f.name, "Dossier");
        assert!(f.named);
        // An unnamed folder (as `update_folder` or the window's drop may leave) follows.
        h.core.projects.write()[0].agent_folders[0].named = false;
        h.core.rename_agent(&a, "alpha").await.unwrap();
        assert_eq!(
            h.core.project(&p.id).unwrap().agent_folders[0].name,
            "alpha"
        );
        h.core.folder_rename(&p.id, &f.id, "Mine").unwrap();
        h.core.rename_agent(&a, "beta").await.unwrap();
        assert_eq!(h.core.project(&p.id).unwrap().agent_folders[0].name, "Mine");
        let g = h.core.folder_create(&p.id, None, &[], None, None).unwrap();
        assert_eq!(g.name, "Dossier");
    }

    #[tokio::test]
    async fn folders_survive_a_restart() {
        let h = harness("fo-restart");
        let (p, _) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        let f = h
            .core
            .folder_create(
                &p.id,
                Some("Docs"),
                std::slice::from_ref(&a),
                Some(&a),
                None,
            )
            .unwrap();
        h.core.folder_fold(&p.id, &f.id, true).unwrap();
        h.core.save_now();
        let app = mock_app();
        let (re, _rx) = Core::load(app.handle().clone(), h.core.data.clone());
        let rp = re.project(&p.id).unwrap();
        assert_eq!(rp.agent_folders.len(), 1);
        assert_eq!(rp.agent_folders[0].name, "Docs");
        assert!(rp.agent_folders[0].folded);
        assert_eq!(rp.agent_folders[0].agents, vec![a]);
        assert_eq!(rp.agent_order, vec![f.id]);
    }

    #[tokio::test]
    async fn folders_come_back_with_a_closed_project() {
        let h = harness("fo-closed");
        let (p, dir) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        let f = h.core.folder_create(&p.id, None, &[a], None, None).unwrap();
        crate::closed_tests::close(&h, &p.id).await;
        let back = crate::closed_tests::add_again(&h, &dir).await;
        assert_eq!(back.agent_folders.len(), 1);
        assert_eq!(back.agent_folders[0].id, f.id);
    }

    #[tokio::test]
    #[allow(unused_assignments)]
    async fn each_change_tells_the_window() {
        let h = harness("fo-events");
        let (p, _) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        let f = h
            .core
            .folder_create(&p.id, None, std::slice::from_ref(&a), None, None)
            .unwrap();
        let tells = |h: &Harness| {
            let n = h
                .events
                .lock()
                .iter()
                .filter(|e| e["type"] == "project")
                .count();
            n
        };
        let mut seen = tells(&h);
        macro_rules! step {
            ($e:expr) => {{
                $e.unwrap();
                let n = tells(&h);
                assert!(n > seen);
                let _ = &mut seen;
                seen = n;
            }};
        }
        step!(h.core.folder_rename(&p.id, &f.id, "X"));
        step!(h.core.folder_principal(&p.id, &f.id, Some(&a)));
        step!(h.core.folder_fold(&p.id, &f.id, true));
        step!(h.core.item_move(&p.id, &a, None, None));
        step!(h.core.folder_delete(&p.id, &f.id));
    }

    #[tokio::test]
    async fn a_move_to_a_folder_that_went_is_refused() {
        let h = harness("fo-refuse");
        let (p, _) = h.project(false).await;
        let a = agent(&h, &p.id, None).await;
        assert!(h.core.item_move(&p.id, &a, Some("gone"), None).is_err());
        assert!(h.core.item_move(&p.id, "ghost", None, None).is_err());
        assert!(h.core.folder_principal(&p.id, "gone", None).is_err());
        let f = h.core.folder_create(&p.id, None, &[], None, None).unwrap();
        assert!(h.core.folder_principal(&p.id, &f.id, Some(&a)).is_err());
        assert!(h.core.folder_rename(&p.id, &f.id, " ").is_err());
        assert_eq!(order(&h, &p.id), vec![a, f.id]);
    }
}
