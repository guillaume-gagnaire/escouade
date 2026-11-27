//! `open_browser` and `close_browser` (ESC-678, AB-6): the addresses an agent may open (security
//! first: http and https alone, never Escouade's, this machine at the declared ports alone), the
//! artifacts found in its `Artifact` tool's results, the origins asked of the user (C3), the bounds,
//! who may, a private project, « Agents et navigateurs » off, and what closing does. Called by a
//! real MCP client on a test core without a webview: an opening that passes every refusal ends on
//! « Le navigateur ne s’est pas ouvert » (there is no window to put it in), the proof it passed.
//! Opening a real tab beside a conversation is the e2e's (browsers-open.spec.ts).

use super::browsers_open::{artifact_address, artifact_version, artifacts_of, place_of};
use super::browsers_tests::{browser, pages, world};
use super::private_tests::make_private;
use super::tools_tests::{as_agent, call, external, other_project, put_agent, read, refused};
use crate::core_tests::{harness, Harness};
use crate::devpage::{OpenLeave, PageView, Placement, Wish};
use crate::model::*;
use serde_json::json;
use tauri::Manager;

/// The words an opening that passed every refusal ends on, without a window.
const PASSED: &str = "ne s’est pas ouvert";

/// Escouade served from a dev server at `http://localhost:1420`: its pages are Escouade's own.
fn dev_server(h: &Harness) {
    let url = tauri::Url::parse("http://localhost:1420/").unwrap();
    h.core.app.manage(crate::navigation::Home {
        url: url.clone(),
        dev: Some(url),
    });
}

/// The agent `id` drives the browser `key`.
fn drive(h: &Harness, key: &str, id: &str) {
    pages(h).set_driver(key, Some(id));
}

/// A tab of project `p` that agent `id` opened, at `url`.
fn tab(h: &Harness, p: &Project, key: &str, url: &str, id: &str) {
    browser(h, p, key, url, Placement::Beside);
    pages(h).set_opener(key, id);
    pages(h).set_driver(key, Some(id));
}

/// An `Artifact` tool's call in the conversation of `id`, its result's text `text`.
fn artifact_call(h: &Harness, id: &str, call_id: &str, action: Option<&str>, text: &str) {
    let mut input = json!({ "file_path": "C:/x/page.html" });
    if let Some(a) = action {
        input["action"] = json!(a);
    }
    let item = json!({
        "kind": "tool", "id": call_id, "name": "Artifact", "input": input,
        "status": "ok", "result": { "isError": false, "text": text },
    });
    h.core
        .agent(id)
        .unwrap()
        .lock()
        .conv
        .apply(&ConvOp::Append { item });
}

#[test]
fn an_artifacts_address_is_claude_ais_and_nothing_else() {
    for (text, want) in [
        (
            "https://claude.ai/artifact/abc-123",
            Some("https://claude.ai/artifact/abc-123"),
        ),
        (
            "https://claude.ai/code/artifact/0f1e-2d3c?x=1#top",
            Some("https://claude.ai/code/artifact/0f1e-2d3c"),
        ),
        (
            "https://claude.ai/artifact/abc/",
            Some("https://claude.ai/artifact/abc"),
        ),
        ("http://claude.ai/artifact/abc", None),
        ("https://claude.ai.evil.example/artifact/abc", None),
        ("https://evil.example/claude.ai/artifact/abc", None),
        ("https://user:pw@claude.ai/artifact/abc", None),
        ("https://claude.ai:8443/artifact/abc", None),
        ("https://claude.ai/chat/abc", None),
        ("https://claude.ai/artifact/", None),
        ("https://claude.ai/artifact/a/b", None),
        ("https://claude.ai/artifact/%2e%2e", None),
        ("https://claude.ai/code/artifact/abc/../../settings", None),
    ] {
        assert_eq!(artifact_address(text).as_deref(), want, "{text}");
    }
}

#[test]
fn the_artifacts_are_those_its_artifact_tool_published() {
    let tool = |id: &str, name: &str, action: Option<&str>, status: &str, text: &str| {
        let mut input = json!({});
        if let Some(a) = action {
            input["action"] = json!(a);
        }
        json!({ "kind": "tool", "id": id, "name": name, "input": input, "status": status,
                "result": { "isError": status != "ok", "text": text } })
    };
    let items =
        vec![
        tool("1", "Artifact", None, "ok", "Published: https://claude.ai/code/artifact/first."),
        // Another tool's result names an artifact: not one the agent published.
        tool("2", "Bash", None, "ok", "https://claude.ai/code/artifact/bash"),
        // A read of another artifact: not a publication.
        tool("3", "Artifact", Some("read"), "ok", "https://claude.ai/artifact/someone-else"),
        // A publication that failed.
        tool("4", "Artifact", None, "error", "https://claude.ai/artifact/failed"),
        // A message of the agent's that names one: not a tool's result.
        json!({ "kind": "assistant", "id": "5", "text": "https://claude.ai/artifact/said" }),
        tool(
            "6",
            "Artifact",
            Some("publish"),
            "ok",
            "Updated (https://claude.ai/artifact/second) and https://claude.ai/code/artifact/first",
        ),
    ];
    assert_eq!(
        artifacts_of(&items),
        [
            "https://claude.ai/artifact/second",
            "https://claude.ai/code/artifact/first"
        ]
    );
}

#[test]
fn an_agents_browser_opens_where_it_wants_while_the_user_looks_at_its_conversation() {
    let pages = crate::devpage::Pages::default();
    assert_eq!(place_of(&pages, "a1", Wish::Beside), "waiting");
    assert_eq!(place_of(&pages, "a1", Wish::Quiet), "quiet");
    pages.set_looking(["a1".to_string()]);
    assert_eq!(place_of(&pages, "a1", Wish::Beside), "beside");
    assert_eq!(place_of(&pages, "a1", Wish::Below), "below");
    assert_eq!(place_of(&pages, "a1", Wish::Full), "full");
    assert_eq!(place_of(&pages, "a2", Wish::Full), "waiting");
}

#[test]
fn a_tab_keeps_its_projects_storage_never_the_windows() {
    // No cookie of the window's webview: a folder of its project's under Escouade's data, one per
    // project.
    let root = std::path::Path::new("C:/data");
    let a = crate::devpage::data_dir(root, "p-1");
    let b = crate::devpage::data_dir(root, "p-2");
    assert_eq!(a, root.join("pages").join("p-1"));
    assert_ne!(a, b);
    assert_ne!(a, root);
}

#[tokio::test]
async fn never_another_scheme_nor_escouade_nor_this_machine_at_an_undeclared_port() {
    let h = harness("mcp-browsers-open-never");
    dev_server(&h);
    let (_, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    for url in [
        "file:///C:/Windows/win.ini",
        "data:text/html,<h1>x</h1>",
        "javascript:alert(1)",
        "blob:https://example.com/0f1e",
        "ftp://example.com/",
        "about:blank",
    ] {
        let e = refused(&c, "open_browser", json!({ "url": url })).await;
        assert!(
            e.contains("Seules les adresses http et https"),
            "{url}: {e}"
        );
    }
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "http://localhost:1420/" }),
    )
    .await;
    assert!(e.contains("adresse d’Escouade"), "{e}");
    #[cfg(windows)]
    {
        let e = refused(
            &c,
            "open_browser",
            json!({ "url": "http://tauri.localhost/" }),
        )
        .await;
        assert!(e.contains("adresse d’Escouade"), "{e}");
    }
    // This machine at a port no launch declares: refused, not asked (a service of the machine).
    for url in [
        "http://localhost:9000/",
        "http://localhost/",
        "http://127.0.0.1:1/",
        "http://127.1:5432/",
        "http://2130706433:6379/",
        "http://0x7f000001:6379/",
        "http://0.0.0.0:22/",
        "http://[::1]:9229/",
        "http://[::ffff:127.0.0.1]:8080/",
        "http://admin.localhost:9999/x",
        "http://LOCALHOST.:3000/",
    ] {
        let e = refused(&c, "open_browser", json!({ "url": url })).await;
        assert!(e.contains("ports que déclarent"), "{url}: {e}");
    }
    // Escouade's own server, at its port.
    let server = h.core.mcp.status().port;
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": format!("http://127.0.0.1:{server}/mcp") }),
    )
    .await;
    assert!(e.contains("ports que déclarent"), "{e}");
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "https://ada:pw@example.com/" }),
    )
    .await;
    assert!(e.contains("nom d’utilisateur"), "{e}");
    let long = format!("https://example.com/{}", "a".repeat(2100));
    let e = refused(&c, "open_browser", json!({ "url": long })).await;
    assert!(e.contains("trop longue"), "{e}");
    // Nothing was asked of the user.
    assert!(pages(&h).open_asks().is_empty());
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn the_launch_opens_without_asking_at_its_declared_ports() {
    let h = harness("mcp-browsers-open-launch");
    let (_, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    for url in [
        "http://localhost:4211/tarifs",
        "http://127.0.0.1:4215/",
        "http://[::1]:4100/",
        "http://localhost:4100/",
    ] {
        let e = refused(&c, "open_browser", json!({ "url": url })).await;
        assert!(e.contains(PASSED), "{url}: {e}");
    }
    // Its test's page, at its address or at another of its origin; never another origin.
    let e = refused(
        &c,
        "open_browser",
        json!({ "launch": true, "place": "full" }),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    let e = refused(
        &c,
        "open_browser",
        json!({ "launch": true, "url": "http://localhost:4211/autre" }),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    let e = refused(
        &c,
        "open_browser",
        json!({ "launch": true, "url": "https://docs.stripe.com/" }),
    )
    .await;
    assert!(e.contains("de l’origine de ton lancement"), "{e}");
    assert!(pages(&h).open_asks().is_empty());
    // Nothing to open, or two things at once.
    let e = refused(&c, "open_browser", json!({})).await;
    assert!(e.contains("Dis quoi ouvrir"), "{e}");
    let e = refused(
        &c,
        "open_browser",
        json!({ "artifact": "last", "url": "https://claude.ai/" }),
    )
    .await;
    assert!(e.contains("artifact s’ouvre seul"), "{e}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn another_origin_is_asked_once_and_a_refusal_holds() {
    let h = harness("mcp-browsers-open-ask");
    let (p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let v = read(
        &c,
        "open_browser",
        json!({ "url": "https://docs.stripe.com/api", "place": "below" }),
    )
    .await;
    assert_eq!(v["place"], "asked");
    assert_eq!(v["origin"], "https://docs.stripe.com");
    // Nothing opened: no row.
    assert_eq!(pages(&h).all().len(), 3);
    // Asked again before the answer: still one request, the first one's place.
    let v = read(
        &c,
        "open_browser",
        json!({ "url": "https://docs.stripe.com/other" }),
    )
    .await;
    assert_eq!(v["place"], "asked");
    let asks = pages(&h).open_asks();
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].agent, a.id);
    assert_eq!(asks[0].project_id, p.id);
    assert_eq!(asks[0].url, "https://docs.stripe.com/api");
    assert_eq!(asks[0].wish, Wish::Below);
    // The user refuses: told, and never asked again.
    let all = pages(&h);
    super::browsers_open::answer(&h.core, &all, &asks[0].id, false)
        .await
        .unwrap();
    assert!(pages(&h).open_asks().is_empty());
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "https://docs.stripe.com/api" }),
    )
    .await;
    assert!(
        e.contains("a refusé que tu ouvres https://docs.stripe.com"),
        "{e}"
    );
    // An answer to a request no longer there does nothing.
    super::browsers_open::answer(&h.core, &all, &asks[0].id, true)
        .await
        .unwrap();
    // Five requests at most.
    for n in 0..4 {
        let v = read(
            &c,
            "open_browser",
            json!({ "url": format!("https://site{n}.example/") }),
        )
        .await;
        assert_eq!(v["place"], "asked");
    }
    assert_eq!(
        all.ask_open(
            &a.id,
            &p.id,
            "https://five.example",
            "https://five.example/",
            Wish::Beside
        ),
        OpenLeave::Asked("o-6".into())
    );
    let e = refused(&c, "open_browser", json!({ "url": "https://six.example/" })).await;
    assert!(e.contains("5 de tes demandes"), "{e}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn an_origin_the_user_let_open_goes_to_its_tab_without_asking() {
    let h = harness("mcp-browsers-open-allowed");
    let (p, a) = world(&h).await;
    tab(
        &h,
        &p,
        "browser:stripe",
        "https://docs.stripe.com/api",
        &a.id,
    );
    pages(&h).allow_origin("browser:stripe", "https://docs.stripe.com");
    let c = as_agent(&h, &a.id).await;
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "https://docs.stripe.com/webhooks" }),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    assert!(pages(&h).open_asks().is_empty());
    // The leave was for that browser: once it is closed, the origin is asked again.
    let v = read(&c, "close_browser", json!({ "browser": "browser:stripe" })).await;
    assert_eq!(v["closed"], true);
    let v = read(
        &c,
        "open_browser",
        json!({ "url": "https://docs.stripe.com/webhooks" }),
    )
    .await;
    assert_eq!(v["place"], "asked");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn the_artifact_is_one_its_artifact_tool_published() {
    let h = harness("mcp-browsers-open-artifact");
    let (_, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let e = refused(&c, "open_browser", json!({ "artifact": "last" })).await;
    assert!(e.contains("Aucun artefact publié"), "{e}");
    artifact_call(
        &h,
        &a.id,
        "t1",
        None,
        "Published https://claude.ai/code/artifact/plan-24 (private).",
    );
    // Opened without asking, the artifact alone.
    let e = refused(&c, "open_browser", json!({ "artifact": "last" })).await;
    assert!(e.contains(PASSED), "{e}");
    let e = refused(
        &c,
        "open_browser",
        json!({ "artifact": "https://claude.ai/code/artifact/plan-24" }),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    // An address it did not publish is not its artifact; claude.ai itself is asked.
    let e = refused(
        &c,
        "open_browser",
        json!({ "artifact": "https://claude.ai/code/artifact/someone-elses" }),
    )
    .await;
    assert!(e.contains("pas un artefact que tu as publié"), "{e}");
    let v = read(
        &c,
        "open_browser",
        json!({ "url": "https://claude.ai/code/artifact/someone-elses" }),
    )
    .await;
    assert_eq!(v["place"], "asked");
    assert!(pages(&h).open_asks()[0].origin == "https://claude.ai");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn the_bounds_three_driven_and_ten_an_hour() {
    let h = harness("mcp-browsers-open-bounds");
    let (p, a) = world(&h).await;
    for k in ["browser:t1", "browser:t2", "browser:t3"] {
        tab(&h, &p, k, &format!("http://localhost:4100/{k}"), &a.id);
    }
    let c = as_agent(&h, &a.id).await;
    // A fourth browser driven: refused. Its own tab on that origin is one it drives already.
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "http://localhost:4212/" }),
    )
    .await;
    assert!(e.contains("Tu pilotes déjà 3 navigateurs"), "{e}");
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "http://localhost:4100/x" }),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    let v = read(&c, "close_browser", json!({ "browser": "browser:t1" })).await;
    assert_eq!(v["closed"], true);
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "http://localhost:4212/" }),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    // Ten openings an hour, asked ones included.
    let all = pages(&h);
    for _ in 0..10 {
        all.note_open(&a.id);
    }
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "https://docs.stripe.com/" }),
    )
    .await;
    assert!(e.contains("10 navigateurs dans l’heure"), "{e}");
    assert!(all.open_asks().is_empty());
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn closing_closes_its_own_tab_stops_driving_its_launch_and_never_the_users() {
    let h = harness("mcp-browsers-close");
    let (p, a) = world(&h).await;
    let b = put_agent(&h, &p, "Bob", 2);
    tab(&h, &p, "browser:mine", "http://localhost:4100/", &a.id);
    tab(&h, &p, "browser:bobs", "http://localhost:4100/", &b.id);
    drive(&h, &a.id, &a.id);
    let c = as_agent(&h, &a.id).await;
    // Its own tab: closed, its row gone.
    let v = read(&c, "close_browser", json!({ "browser": "browser:mine" })).await;
    assert_eq!(v["closed"], true);
    assert!(pages(&h).get("browser:mine").is_none());
    // Its launch's page: its driving stops, the row stays.
    let v = read(&c, "close_browser", json!({ "browser": a.id })).await;
    assert_eq!(v["closed"], false);
    let own: PageView = pages(&h).get(&a.id).unwrap();
    assert_eq!(own.driver, None);
    // The user's, another agent's, an agent's page: refused.
    let e = refused(&c, "close_browser", json!({ "browser": "browser:claude" })).await;
    assert!(e.contains("L’utilisateur a ouvert ce navigateur"), "{e}");
    assert!(pages(&h).get("browser:claude").is_some());
    let e = refused(&c, "close_browser", json!({ "browser": "browser:bobs" })).await;
    assert!(e.contains("n’est pas le tien"), "{e}");
    let e = refused(&c, "close_browser", json!({ "browser": "page:p-1" })).await;
    assert!(e.contains("close_page"), "{e}");
    let e = refused(&c, "close_browser", json!({ "browser": "browser:nothing" })).await;
    assert!(e.contains("Aucun navigateur"), "{e}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn a_tab_an_agent_opened_is_listed_as_its_own() {
    let h = harness("mcp-browsers-open-list");
    let (p, a) = world(&h).await;
    let d = put_agent(&h, &p, "Atelier", 3);
    h.core.agent(&d.id).unwrap().lock().meta.discussion = Some(Default::default());
    tab(&h, &p, "browser:atelier", "http://localhost:4100/", &d.id);
    tab(&h, &p, "browser:alice", "http://localhost:4100/", &a.id);
    let c = as_agent(&h, &a.id).await;
    let v = read(&c, "list_browsers", json!({})).await;
    let row = v["browsers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["browser"] == "browser:alice")
        .unwrap()
        .clone();
    assert_eq!(row["by"], "agent");
    c.cancel().await.unwrap();
    // A discussion sees the tabs it opened.
    let c = as_agent(&h, &d.id).await;
    let v = read(&c, "list_browsers", json!({})).await;
    let keys: Vec<&str> = v["browsers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["browser"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["browser:atelier"]);
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn who_may_open_the_switch_claude_outside_and_a_private_project() {
    let h = harness("mcp-browsers-open-who");
    let (vault, insider) = world(&h).await;
    // Claude outside Escouade has no conversation to open beside.
    let c = external(&h).await;
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "http://localhost:4100/" }),
    )
    .await;
    assert!(e.contains("n’ouvre aucun navigateur"), "{e}");
    let e = refused(&c, "close_browser", json!({ "browser": insider.id })).await;
    assert!(e.contains("n’ouvre aucun navigateur"), "{e}");
    c.cancel().await.unwrap();
    // A private project: its own agent opens; another project's agent knows none of its browsers.
    let vault = make_private(&h, &vault);
    crate::harness_scope_tests::turn_on_every_harness(&h, &vault.id);
    let c = as_agent(&h, &insider.id).await;
    let e = refused(
        &c,
        "open_browser",
        json!({ "url": "http://localhost:4100/" }),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    c.cancel().await.unwrap();
    let open = other_project(&h, "Ouvert").await;
    let outsider = put_agent(&h, &open, "Bob", 2);
    let c = as_agent(&h, &outsider.id).await;
    let (missing, error) = call(&c, "close_browser", json!({ "browser": "zz-introuvable" })).await;
    assert!(error);
    let (text, error) = call(&c, "close_browser", json!({ "browser": insider.id })).await;
    assert!(error);
    assert_eq!(text.replace(&insider.id, "zz-introuvable"), missing);
    // It opens in its own project alone: the vault's launch is not its own.
    let v = read(&c, "open_browser", json!({ "url": "https://example.com/" })).await;
    assert_eq!(v["place"], "asked");
    assert_eq!(pages(&h).open_asks()[0].project_id, open.id);
    c.cancel().await.unwrap();
    // « Agents et navigateurs » off: both refused.
    h.core.settings.write().agent_browsers = false;
    let c = as_agent(&h, &insider.id).await;
    for (tool, args) in [
        ("open_browser", json!({ "url": "http://localhost:4100/" })),
        ("close_browser", json!({ "browser": insider.id })),
    ] {
        let e = refused(&c, tool, args).await;
        assert!(e.contains("Agents et navigateurs"), "{tool}: {e}");
    }
    c.cancel().await.unwrap();
}

#[test]
fn a_file_uploaded_to_its_assets_is_no_version_and_the_version_is_the_results() {
    let upload = |input: serde_json::Value| {
        json!({ "kind": "tool", "id": "u", "name": "Artifact", "input": input, "status": "ok",
                "result": { "isError": false, "text": "Uploaded to https://claude.ai/code/artifact/plan-24" } })
    };
    for input in [
        json!({ "url": "https://claude.ai/code/artifact/plan-24", "asset": true, "file_path": "a.png" }),
        json!({ "url": "https://claude.ai/code/artifact/plan-24", "asset": true, "file_paths": ["a.png"] }),
        json!({ "url": "https://claude.ai/code/artifact/plan-24", "file_paths": ["a.png"] }),
    ] {
        assert!(artifacts_of(&[upload(input.clone())]).is_empty(), "{input}");
    }
    // What Claude Code's result says of the version, else nothing.
    for (text, want) in [
        ("Published p.html at https://claude.ai/artifact/a (Version 3, version id 17-12f1) Icon: \"page\".", Some(3)),
        ("Published https://claude.ai/artifact/a (Version 12)", Some(12)),
        ("Published https://claude.ai/artifact/a", None),
        ("Version 0", None),
        ("Version x", None),
    ] {
        assert_eq!(artifact_version(text), want, "{text}");
    }
}

#[tokio::test]
async fn a_new_version_reloads_the_tabs_it_opened_on_that_artifact_alone() {
    use super::browsers_drive::{artifact_published, artifact_tabs, reload_what};
    let h = harness("mcp-browsers-open-artifact-version");
    let (p, a) = world(&h).await;
    let other = put_agent(&h, &p, "Bob", 2);
    let plan = "https://claude.ai/code/artifact/plan-24";
    tab(&h, &p, "browser:a1", plan, &a.id);
    // Another agent's tab on the same artifact, its own tab on claude.ai elsewhere, on another artifact.
    tab(&h, &p, "browser:a2", plan, &other.id);
    tab(&h, &p, "browser:a3", "https://claude.ai/new", &a.id);
    tab(
        &h,
        &p,
        "browser:a4",
        "https://claude.ai/artifact/other",
        &a.id,
    );
    let pages = pages(&h);
    assert_eq!(artifact_tabs(&pages, &a.id, plan), ["browser:a1"]);
    assert_eq!(artifact_tabs(&pages, &other.id, plan), ["browser:a2"]);
    // An address that is not an artifact's, even one the tab is on, reloads nothing.
    assert!(artifact_tabs(&pages, &a.id, "https://claude.ai/new").is_empty());
    assert_eq!(reload_what(Some(3)), "Recharge : version 3");
    assert_eq!(reload_what(None), "Recharge : nouvelle version");
    // Without a window (no webview) nothing reloads and nothing breaks; the band is left as it was.
    artifact_published(&h.core, &a.id, plan, Some(2));
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(pages.drives.view("browser:a1").step, 0);
}
