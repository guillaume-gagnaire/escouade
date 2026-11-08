//! What only the crate reaches: the words it writes, and the trial on a real organization (ignored). The rest of Azure Boards'
//! tests play its module through the app: `src-tauri/src/integrations/azure_tests.rs`.

use super::*;
use crate::pulls::Azure;
use escouade_plugin_kit::http::reqwest;
use escouade_plugin_kit::protocol::tickets as plugin_tickets;
use serde_json::json;

fn r_of(container: &str, id: &str) -> plugin_tickets::ExternalRef {
    plugin_tickets::ExternalRef {
        service: "azure".into(),
        id: id.into(),
        key: format!("#{id}"),
        container: container.into(),
        ..Default::default()
    }
}

#[test]
fn a_state_no_type_has_goes_as_it_is() {
    // Azure says why it refuses it.
    assert_eq!(
        target_state(&[], "Task", "Whatever"),
        "Whatever".to_string()
    );
}

#[test]
fn a_token_without_the_work_items_right_is_told_in_english_too() {
    assert_eq!(
        no_right(Lang::En, "Azure DevOps denies access (403)", "https://x/tokens"),
        "Azure DevOps denies access (403). The Azure DevOps token needs the Work Items right (read and write): make one on https://x/tokens, then replace it in Settings › Connected services › Code hosts."
    );
}

/// Against the user's own Azure DevOps, never in the suites: `cargo test real_azure_boards --
/// --ignored --nocapture` with `ESCOUADE_TEST_AZURE_URL` (the organization's address,
/// `https://dev.azure.com/<org>`), `ESCOUADE_TEST_AZURE_PAT` (a token with the Work Items right,
/// read and write) and `ESCOUADE_TEST_AZURE_PROJECT` (a project of it, where a Task is made, moved,
/// commented on, then put in the recycle bin).
#[tokio::test]
#[ignore]
async fn real_azure_boards() {
    let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("{k} is needed"));
    let url = var("ESCOUADE_TEST_AZURE_URL")
        .trim_end_matches('/')
        .to_string();
    let project = var("ESCOUADE_TEST_AZURE_PROJECT");
    let org = url.rsplit('/').next().unwrap().to_string();
    let b = Boards::new(vec![Instance {
        org: org.clone(),
        client: Azure::new(
            reqwest::Client::new(),
            url.clone(),
            None,
            var("ESCOUADE_TEST_AZURE_PAT"),
        ),
        token_page: format!("{url}/_usersSettings/tokens"),
    }]);
    let container = format!("{org}/{project}");
    println!("me: {:?}", b.me().await.unwrap());
    let projects = b.containers().await.unwrap();
    println!("{} projects", projects.len());
    assert!(projects.iter().any(|c| c.id == container), "{projects:?}");
    let view = b.states_view(&container).await.unwrap();
    println!("states: {:?}\ndefaults: {:?}", view.states, view.defaults);
    assert!(!view.states.is_empty());

    // A task of its own, to move and comment on.
    let inst = &b.instances[0];
    let patch = json!([
        { "op": "add", "path": "/fields/System.Title", "value": "Escouade : essai réel d'Azure Boards (à supprimer)" },
        { "op": "add", "path": "/fields/System.Description", "value": "<div>Fait par real_azure_boards.</div>" },
        { "op": "add", "path": "/fields/System.Tags", "value": "escouade-essai" }
    ]);
    let (made, _) = b
        .call(
            inst,
            Method::POST,
            v(
                format!(
                    "{}/wit/workitems/$Task",
                    Boards::project_api(inst, &project)
                ),
                VERSION,
            ),
            Some((&patch, "application/json-patch+json")),
        )
        .await
        .unwrap();
    let id = made["id"].as_u64().unwrap().to_string();
    println!("made #{id}");
    let outcome = async {
        let page = b
            .issues(
                &container,
                &plugin_tickets::Query {
                    text: format!("#{id}"),
                    ..Default::default()
                },
            )
            .await?;
        println!(
            "found: {:?}",
            page.issues.first().map(|i| (&i.key, &i.meta))
        );
        anyhow::ensure!(
            page.issues.len() == 1,
            "the task is not found by its number"
        );
        let page = b
            .issues(
                &container,
                &plugin_tickets::Query {
                    label: Some("escouade-essai".into()),
                    ..Default::default()
                },
            )
            .await?;
        anyhow::ensure!(
            page.issues.iter().any(|i| i.id == id),
            "not found by its tag"
        );
        let doing = view
            .defaults
            .get("doing")
            .cloned()
            .ok_or_else(|| anyhow!("no state in progress"))?;
        b.set_state(&r_of(&container, &id), &doing).await?;
        b.comment(
            &r_of(&container, &id),
            "**Escouade** : essai réel, commentaire en markdown.",
        )
        .await?;
        let item = b
            .get(
                inst,
                v(
                    format!("{}/wit/workitems/{id}", Boards::project_api(inst, &project)),
                    VERSION,
                ),
            )
            .await?;
        println!("state now: {}", item["fields"]["System.State"]);
        Ok::<_, anyhow::Error>(())
    }
    .await;
    // To the recycle bin, whatever happened.
    let gone = b
        .call(
            inst,
            Method::DELETE,
            v(
                format!("{}/wit/workitems/{id}", Boards::project_api(inst, &project)),
                VERSION,
            ),
            None,
        )
        .await;
    println!("deleted: {}", gone.is_ok());
    outcome.unwrap();
}
