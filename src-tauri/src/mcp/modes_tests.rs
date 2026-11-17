//! The tools and the mode a harness really runs in (ESC-489): `create_agent` — from a client of the server, or from an
//! agent that starts another one (a subagent of Escouade's) — and `start_ticket` refuse a harness whose one mode is
//! Bypass while Bypass is not allowed, saying why and what to do, and make nothing; allowed, the agent starts in Bypass.

use super::tools_tests::{as_agent, call, external, project, put, put_agent, ticket};
use crate::core_tests::harness;
use crate::engine::minimal::ONE_BYPASS;
use crate::model::*;
use crate::modes::tests::{allow_bypass, with_account};
use serde_json::json;

const ONLY_BYPASS: &str = "ne tourne qu’en Bypass";

#[tokio::test]
async fn no_tool_starts_an_agent_of_a_harness_whose_one_mode_is_bypass_without_bypass() {
    let h = harness("esc489-mcp");
    let p = project(&h).await;
    let e = with_account(&h, &ONE_BYPASS);
    // The project's harness: the one `create_agent` makes its agents on.
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .engine = Some(e);
    let parent = put_agent(&h, &p, "parent", 1);
    let before = h.core.project_agents(&p.id).len();
    // From a client of the server, and from an agent (the agents it starts are its subagents).
    for by_agent in [false, true] {
        // Each client starts the server again: one at a time.
        let c = match by_agent {
            false => external(&h).await,
            true => as_agent(&h, &parent.id).await,
        };
        let (text, error) = call(
            &c,
            "create_agent",
            json!({ "project": p.id, "message": "Bonjour" }),
        )
        .await;
        assert!(error, "{text}");
        assert!(text.contains(ONLY_BYPASS), "{text}");
        assert_eq!(h.core.project_agents(&p.id).len(), before, "nothing made");
    }
    // A ticket of that harness.
    let mut t = ticket(&p, "DEM-1", "Sans filet", Column::Todo, 1);
    t.engine = Some(e);
    let t = put(&h, t);
    let c = external(&h).await;
    let (text, error) = call(&c, "start_ticket", json!({ "ticket": t.key })).await;
    assert!(error, "{text}");
    assert!(text.contains(ONLY_BYPASS), "{text}");
    assert_eq!(h.core.ticket(&t.id).unwrap().column, Column::Todo);
    // Allowed: the agent is made, in the mode it runs in.
    allow_bypass(&h, true);
    // Saved settings start the server again.
    let c = external(&h).await;
    let (text, error) = call(
        &c,
        "create_agent",
        json!({ "project": p.id, "message": "Bonjour" }),
    )
    .await;
    assert!(!error, "{text}");
    let id = serde_json::from_str::<serde_json::Value>(&text).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(h.agent(&id).mode, crate::bypass::BYPASS);
}

/// The policies judge the same mode (ESC-497): « Autoriser le mode Bypass » on in the app, Bypass turned off by this
/// device's policy, then by the project's team: `create_agent` (from a client, from an agent) and `start_ticket` refuse
/// a harness whose one mode is Bypass, saying who turns it off, and make nothing.
#[tokio::test]
async fn no_tool_starts_an_agent_of_a_harness_of_bypass_alone_that_a_policy_turns_off() {
    use crate::locks::{Managed, TeamPolicies};
    for team in [false, true] {
        let h = harness(&format!("esc497-mcp-{team}"));
        let p = project(&h).await;
        let e = with_account(&h, &ONE_BYPASS);
        allow_bypass(&h, true);
        h.core
            .projects
            .write()
            .iter_mut()
            .find(|x| x.id == p.id)
            .unwrap()
            .engine = Some(e);
        let off = json!(false);
        let says = if team {
            h.core
                .projects
                .write()
                .iter_mut()
                .find(|x| x.id == p.id)
                .unwrap()
                .team = Some("acme".into());
            h.core.set_team_policies(Some(TeamPolicies {
                id: "acme".into(),
                team: "Acme".into(),
                signed: None,
                recommended: Default::default(),
                enforced: [("permissions.allowBypass".to_owned(), off)].into(),
            }));
            "imposé par l’équipe Acme (permissions.allowBypass)"
        } else {
            h.core.set_managed(Managed {
                values: [(
                    "permissions.allowBypass".to_owned(),
                    (off, "the machine's policy".to_owned()),
                )]
                .into(),
                warnings: Vec::new(),
            });
            "imposé par la stratégie de ce poste (permissions.allowBypass)"
        };
        let parent = put_agent(&h, &p, "parent", 1);
        let before = h.core.project_agents(&p.id).len();
        for by_agent in [false, true] {
            let c = match by_agent {
                false => external(&h).await,
                true => as_agent(&h, &parent.id).await,
            };
            let (text, error) = call(
                &c,
                "create_agent",
                json!({ "project": p.id, "message": "Bonjour" }),
            )
            .await;
            assert!(error, "{text}");
            assert!(
                text.contains("Sans Filet ne tourne qu’en « Bypass »"),
                "{text}"
            );
            assert!(text.contains(says), "{text}");
            assert_eq!(h.core.project_agents(&p.id).len(), before, "nothing made");
        }
        let mut t = ticket(&p, "DEM-1", "Sans filet", Column::Todo, 1);
        t.engine = Some(e);
        let t = put(&h, t);
        let c = external(&h).await;
        let (text, error) = call(&c, "start_ticket", json!({ "ticket": t.key })).await;
        assert!(error, "{text}");
        assert!(text.contains(says), "{text}");
        assert_eq!(h.core.ticket(&t.id).unwrap().column, Column::Todo);
    }
}
