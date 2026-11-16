use super::*;

fn state() -> serde_json::Value {
    serde_json::json!({
        "projects": [
            { "id": "p1", "name": "atelier", "path": "/a", "color": "#888" },
            { "id": "p2", "name": "boutique", "path": "/b", "color": "#888" }
        ],
        "agents": [
            { "id": "a1", "projectId": "p1", "name": "Relances" },
            { "id": "a2", "projectId": "p1", "name": "Ancien", "archived": true }
        ],
        "tickets": [
            { "id": "t1", "projectId": "p1", "key": "ATL-1", "title": "Relances par e-mail", "column": "todo" },
            { "id": "t2", "projectId": "p2", "key": "BTQ-1", "title": "Panier", "column": "todo" }
        ]
    })
}

fn bytes(v: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}

#[test]
fn a_state_says_its_projects_agents_and_tickets() {
    let s = summarize("state.json", &bytes(&state()));
    assert!(s.readable);
    assert_eq!(
        (s.projects, s.agents, s.tickets),
        (Some(2), Some(1), Some(2))
    );
    assert_eq!(s.accounts, None);
    assert_eq!(summarize("state.json", &[0; 64]), Summary::default());
}

#[test]
fn comparing_says_by_name_what_a_copy_gains_and_loses() {
    let copy = state();
    let mut today = state();
    // Today: the project boutique and its ticket gone, a new ticket in atelier.
    today["projects"].as_array_mut().unwrap().remove(1);
    today["tickets"].as_array_mut().unwrap().remove(1);
    today["tickets"].as_array_mut().unwrap().push(
        serde_json::json!({ "id": "t3", "projectId": "p1", "key": "ATL-2", "title": "Facture" }),
    );
    let d = diff("state.json", &bytes(&copy), &bytes(&today));
    let of = |k: &str| d.kinds.iter().find(|x| x.kind == k).unwrap().clone();
    assert_eq!(of("projects").gained, vec!["boutique"]);
    assert!(of("projects").lost.is_empty());
    assert_eq!(of("tickets").gained, vec!["BTQ-1 Panier"]);
    assert_eq!(of("tickets").lost, vec!["ATL-2 Facture"]);
    assert!(of("agents").gained.is_empty() && of("agents").lost.is_empty());
    assert_eq!(d.copy.projects, Some(2));
    assert_eq!(d.today.projects, Some(1));
    // Today not there: everything is gained.
    let d = diff("state.json", &bytes(&copy), &[]);
    let projects = d.kinds.iter().find(|x| x.kind == "projects").unwrap();
    assert_eq!(projects.gained.len(), 2);
}

#[test]
fn the_settings_and_the_integrations_give_names_never_a_secret() {
    let settings = serde_json::json!({
        "accounts": [{ "id": "principal", "name": "Principal" }, { "id": "c1", "name": "Client", "configDir": "/x" }],
        "proxyUrl": "http://user:hunter2@proxy"
    });
    let s = summarize("settings.json", &bytes(&settings));
    assert_eq!(s.accounts, Some(2));
    let integrations = serde_json::json!({
        "jira": { "site": "https://a.atlassian.net", "email": "ada@a.dev", "token": "SECRET-TOKEN" },
        "github": { "token": "ghp_SECRET" },
        "forges": [{ "kind": "gitlab", "url": "https://gitlab.com", "token": "glpat-SECRET" }]
    });
    let i = summarize("integrations.json", &bytes(&integrations));
    assert_eq!(i.accounts, Some(3));
    let d = diff("integrations.json", &bytes(&integrations), b"{}");
    let all = serde_json::to_string(&(i, d, s)).unwrap();
    assert!(all.contains("jira") && all.contains("gitlab"));
    for secret in [
        "SECRET",
        "ada@a.dev",
        "hunter2",
        "atlassian.net",
        "gitlab.com",
    ] {
        assert!(!all.contains(secret), "{secret} in {all}");
    }
}

#[test]
fn the_other_files_say_what_they_hold() {
    let p = summarize(
        "plugins.json",
        br#"{"version":1,"plugins":[{"id":"dev.escouade.claude-code"}]}"#,
    );
    assert_eq!(p.plugins, Some(1));
    let h = br#"{"project":{"id":"abc","name":"atelier"},"agents":[{"id":"a","name":"x"}],"tickets":[]}"#;
    let c = summarize("closed-projects/abc.json", h);
    assert_eq!(
        (c.name.as_deref(), c.agents, c.tickets),
        (Some("atelier"), Some(1), Some(0))
    );
    assert!(
        !summarize("closed-projects/other.json", h).readable,
        "not named after its project"
    );
    assert_eq!(summarize("sync-queue.json", b"[1,2,3]").entries, Some(3));
    assert_eq!(
        summarize("projects/p1/workflow-tracks.json", br#"{"a":{}}"#).entries,
        Some(1)
    );
    let t = summarize(
        "ticket-accounts.json",
        br#"{"accounts":[{"id":"jira-1","service":"jira","token":"S"}]}"#,
    );
    assert_eq!(t.accounts, Some(1));
}
