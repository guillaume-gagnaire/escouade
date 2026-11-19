//! What waits in a core at a glance (ESC-539): what the piles and « Partout » read of another space.

use crate::core_tests::harness;
use crate::locks::{Locks, Managed, AGGREGATE_VIEW, CONTENTLESS, KEYS};
use crate::model::*;
use serde_json::{json, Value};

fn policy(pairs: &[(&str, Value)]) -> Locks {
    Locks::compute(
        KEYS,
        None,
        &Managed {
            values: pairs
                .iter()
                .map(|(k, v)| {
                    (
                        (*k).to_owned(),
                        (v.clone(), "the machine's policy".to_owned()),
                    )
                })
                .collect(),
            warnings: Vec::new(),
        },
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn a_glance_names_what_waits_and_a_policy_against_the_aggregate_view_leaves_the_numbers_alone(
) {
    let h = harness("glance");
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    // Nothing waits yet.
    let g = h.core.glance(false);
    assert_eq!((g.waiting, g.running), (0, 0));
    assert!(g.agents.is_empty());
    assert_eq!(g.colors, vec![p.color.clone()]);
    // A file outside the folders it reads without asking: the fake `claude` asks to read it.
    let outside = h.dir.join("ailleurs").join("budget.txt");
    std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
    std::fs::write(&outside, "secret").unwrap();
    h.core
        .send_message_files(
            &id,
            format!("lire-fichier {}", outside.display()),
            vec![],
            vec![],
        )
        .await
        .unwrap();
    h.wait("the request", |h| {
        h.agent(&id).status == AgentStatus::Waiting
    })
    .await;
    let g = h.core.glance(false);
    assert_eq!(g.waiting, 1);
    assert!(!g.counts_only);
    let a = &g.agents[0];
    assert_eq!(a.id, id);
    assert_eq!(a.status, AgentStatus::Waiting);
    let r = a.request.as_ref().expect("its request");
    assert_eq!(r.kind, "permission");
    assert_eq!(r.tool, "Read");
    assert_eq!(g.projects.len(), 1);
    assert_eq!(g.projects[0].name, p.name);
    // In JSON as the window reads it.
    let v = serde_json::to_value(&g).unwrap();
    assert_eq!(v["countsOnly"], json!(false));
    assert_eq!(v["agents"][0]["projectId"], json!(p.id));
    // The window's policy asks the numbers alone: no text, no request.
    let g = h.core.glance(true);
    assert_eq!(g.waiting, 1);
    assert!(g.counts_only && g.agents.is_empty() && g.projects.is_empty() && g.asks.is_empty());
    // The core's own policy denies the aggregate view (a work server's): the same, whatever is asked; and its
    // notifications without content are said.
    *h.core.locks.write() = policy(&[(AGGREGATE_VIEW, json!(false)), (CONTENTLESS, json!(true))]);
    let g = h.core.glance(false);
    assert!(g.counts_only && g.contentless);
    assert!(g.agents.is_empty());
    assert_eq!(g.waiting, 1);
    let text = serde_json::to_string(&g).unwrap();
    assert!(!text.contains("budget.txt"), "{text}");
}
