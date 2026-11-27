//! `drive_browser` (ESC-679, AB-7 with AP-6), called by a real MCP client (rmcp's) on a test core without a webview: who
//! drives what (Claude outside Escouade, the user's tab, another agent's, a row on no screen), the tiers on the launch,
//! outside it (to act with the leave, C4) and on an agent's page (its template's keys, tab and version alone), the
//! refusals (a secret of Escouade, an address of another origin), the rule of the hand, and the switches. A gesture that
//! passes every refusal ends on « Ce navigateur vient de se fermer » (there is no webview to drive), the proof it
//! passed. Driving a real page is the e2e's (agent-browsers-drive.spec.ts).

use super::browsers_tests::{browser, pages, world};
use super::tests::names;
use super::tools_tests::{as_agent, external, put_agent, read, refused};
use crate::browserdrive::HandMove;
use crate::core_tests::harness;
use crate::devpage::Placement;
use serde_json::json;

/// The words a gesture that passed every refusal ends on, without a webview.
const PASSED: &str = "vient de se fermer";

#[tokio::test]
async fn an_agent_drives_its_launch_on_screen_and_nothing_else() {
    let h = harness("mcp-drive-who");
    let (p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let tools = names(&c.list_all_tools().await.unwrap());
    assert!(tools.contains(&"drive_browser".to_string()), "{tools:?}");
    // Its test's page, on the launch: to show and to act pass every refusal.
    for args in [
        json!({ "action": "scroll_to", "target": "#tarifs" }),
        json!({ "action": "scroll", "by": 300 }),
        json!({ "action": "highlight", "target": "text=Offre Pro", "label": "Offre Pro" }),
        json!({ "action": "clear" }),
        json!({ "action": "width", "width": "mobile" }),
        json!({ "action": "reload" }),
        json!({ "action": "back" }),
        json!({ "action": "wait", "for": "target", "target": "#tarifs", "timeout": 2 }),
        json!({ "action": "click", "target": "#go" }),
        json!({ "action": "type", "target": "#name", "text": "Ada", "submit": true }),
        json!({ "action": "select", "target": "#plan", "value": "pro" }),
        json!({ "action": "press", "key": "Enter" }),
        json!({ "action": "tab", "target": "#onglet-2" }),
        json!({ "action": "navigate", "url": "http://localhost:4211/compte" }),
    ] {
        let mut args = args;
        args["browser"] = json!(a.id);
        let e = refused(&c, "drive_browser", args.clone()).await;
        assert!(e.contains(PASSED), "{args}: {e}");
    }
    // version is an agent's page's.
    let e = refused(
        &c,
        "drive_browser",
        json!({ "browser": a.id, "action": "version", "n": 1 }),
    )
    .await;
    assert!(e.contains("pas pour un onglet"), "{e}");
    // The bounds of the arguments.
    for (args, said) in [
        (json!({ "action": "scroll", "by": 9000 }), "-5 000 à 5 000"),
        (json!({ "action": "click" }), "Donne l’élément"),
        (
            json!({ "action": "click", "target": "div:has(a)" }),
            "sélecteur n’est pas permis",
        ),
        (
            json!({ "action": "wait", "timeout": 60 }),
            "1 à 30 secondes",
        ),
        (
            json!({ "action": "highlight", "target": "#a", "label": "x".repeat(61) }),
            "60 caractères",
        ),
        (
            json!({ "action": "type", "target": "#a", "text": "x".repeat(2001) }),
            "2 000 caractères",
        ),
        (json!({ "action": "navigate" }), "une adresse (url)"),
    ] {
        let mut args = args;
        args["browser"] = json!(a.id);
        let e = refused(&c, "drive_browser", args.clone()).await;
        assert!(e.contains(said), "{args}: {e}");
    }
    // Claude outside Escouade drives nothing.
    let x = external(&h).await;
    let e = refused(
        &x,
        "drive_browser",
        json!({ "browser": a.id, "action": "scroll", "by": 100 }),
    )
    .await;
    assert!(e.contains("Claude hors d’Escouade"), "{e}");
    x.cancel().await.unwrap();
    // The user's tab, another agent's test page, a row on no screen, a key that is none.
    let e = refused(
        &c,
        "drive_browser",
        json!({ "browser": "browser:claude", "action": "scroll", "by": 100 }),
    )
    .await;
    assert!(e.contains("L’utilisateur a ouvert ce navigateur"), "{e}");
    let b = put_agent(&h, &p, "Bob", 2);
    browser(&h, &p, &b.id, "http://localhost:4100/", Placement::Below);
    let e = refused(
        &c,
        "drive_browser",
        json!({ "browser": b.id, "action": "scroll", "by": 100 }),
    )
    .await;
    assert!(e.contains("n’est pas le tien"), "{e}");
    pages(&h).set_opener("browser:hidden", &a.id);
    let e = refused(
        &c,
        "drive_browser",
        json!({ "browser": "browser:hidden", "action": "scroll", "by": 100 }),
    )
    .await;
    assert!(e.contains("n’est pas affiché"), "{e}");
    let e = refused(
        &c,
        "drive_browser",
        json!({ "browser": "browser:nope", "action": "clear" }),
    )
    .await;
    assert!(e.contains("Aucun navigateur « browser:nope »"), "{e}");
    // list_browsers says which it drives.
    let rows = read(&c, "list_browsers", json!({})).await;
    let row = |k: &str| {
        rows["browsers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["browser"] == k)
            .cloned()
            .unwrap()
    };
    assert_eq!(row(&a.id)["drivable"], true);
    assert_eq!(row("browser:claude")["drivable"], false);
    assert_eq!(row(&b.id)["drivable"], false);
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn outside_the_launch_it_shows_without_asking_and_acts_with_the_leave() {
    let h = harness("mcp-drive-outside");
    let (_, a) = world(&h).await;
    let pages = pages(&h);
    pages.set_opener("browser:claude", &a.id);
    let c = as_agent(&h, &a.id).await;
    let drive = |args: serde_json::Value| {
        let mut args = args;
        args["browser"] = json!("browser:claude");
        args
    };
    // To show: without asking (« trouvé » or not, never the page's text).
    for args in [
        json!({ "action": "scroll", "by": 400 }),
        json!({ "action": "highlight", "target": "#artifact" }),
        json!({ "action": "width", "width": "mobile" }),
        json!({ "action": "reload" }),
    ] {
        let e = refused(&c, "drive_browser", drive(args.clone())).await;
        assert!(e.contains(PASSED), "{args}: {e}");
    }
    // In the frame of the artifact (ESC-682, AB-11): scroll_to and highlight, without asking either; `frame` is for
    // them alone, by a selector.
    for args in [
        json!({ "action": "scroll_to", "frame": "iframe#artifact", "target": "#risques" }),
        json!({ "action": "highlight", "frame": "iframe#artifact", "target": "text=Risques", "label": "Les risques" }),
        json!({ "action": "highlight", "frame": "iframe#artifact" }),
        json!({ "action": "scroll_to", "frame": "iframe#artifact" }),
    ] {
        let e = refused(&c, "drive_browser", drive(args.clone())).await;
        assert!(e.contains(PASSED), "{args}: {e}");
    }
    for (args, said) in [
        (
            json!({ "action": "click", "frame": "iframe#artifact", "target": "#a" }),
            "scroll_to et highlight seuls",
        ),
        (
            json!({ "action": "highlight", "frame": "text=Artefact" }),
            "pas par text=",
        ),
        (
            json!({ "action": "highlight", "frame": "iframe:has(p)" }),
            "sélecteur n’est pas permis",
        ),
        (
            json!({ "action": "scroll_to", "frame": "#art", "target": "div:has(a)" }),
            "sélecteur n’est pas permis",
        ),
        (
            json!({ "action": "highlight", "frame": "#art", "label": "x".repeat(61) }),
            "60 caractères",
        ),
    ] {
        let e = refused(&c, "drive_browser", drive(args.clone())).await;
        assert!(e.contains(said), "{args}: {e}");
    }
    assert!(pages.consents("browser:claude").asks.is_empty());
    // To act: asked of the user, once per origin and per browser.
    let v = read(
        &c,
        "drive_browser",
        drive(json!({ "action": "click", "target": "text=Partager" })),
    )
    .await;
    assert_eq!(v["asked"], true, "{v}");
    assert_eq!(v["origin"], "https://claude.ai");
    assert!(
        v["message"].as_str().unwrap().contains("lire et d’agir"),
        "{v}"
    );
    let asks = pages.consents("browser:claude").asks;
    assert_eq!(asks.len(), 1);
    let again = read(
        &c,
        "drive_browser",
        drive(json!({ "action": "type", "target": "#q", "text": "x" })),
    )
    .await;
    assert_eq!(again["asked"], true);
    assert_eq!(pages.consents("browser:claude").asks.len(), 1);
    // With the leave: it acts.
    assert!(pages.answer("browser:claude", &a.id, "https://claude.ai", true));
    let e = refused(
        &c,
        "drive_browser",
        drive(json!({ "action": "click", "target": "text=Partager" })),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    // An address of its origin, yes; another origin goes by open_browser; never another scheme.
    let e = refused(
        &c,
        "drive_browser",
        drive(json!({ "action": "navigate", "url": "https://claude.ai/code/artifact/x" })),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    let e = refused(
        &c,
        "drive_browser",
        drive(json!({ "action": "navigate", "url": "https://evil.example/" })),
    )
    .await;
    assert!(e.contains("ouvre-la par open_browser"), "{e}");
    for url in [
        "file:///C:/Windows/win.ini",
        "javascript:alert(1)",
        "http://127.0.0.1:9/",
    ] {
        let e = refused(
            &c,
            "drive_browser",
            drive(json!({ "action": "navigate", "url": url })),
        )
        .await;
        assert!(!e.contains(PASSED), "{url}: {e}");
    }
    // Refused once: not asked again.
    pages.revoke("browser:claude");
    let _ = read(
        &c,
        "drive_browser",
        drive(json!({ "action": "click", "target": "#x" })),
    )
    .await;
    assert!(pages.answer("browser:claude", &a.id, "https://claude.ai", false));
    let e = refused(
        &c,
        "drive_browser",
        drive(json!({ "action": "click", "target": "#x" })),
    )
    .await;
    assert!(e.contains("ne le redemande pas"), "{e}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn a_secret_of_escouade_is_never_typed() {
    let h = harness("mcp-drive-secret");
    let (_, a) = world(&h).await;
    h.core.settings.write().proxy_password_in_file = "hunter2-proxy-679".into();
    let c = as_agent(&h, &a.id).await;
    for text in [
        "mon mot de passe hunter2-proxy-679",
        "ghp_0123456789abcdefghijklmnopqrstuvwxyzAB",
    ] {
        let e = refused(
            &c,
            "drive_browser",
            json!({ "browser": a.id, "action": "type", "target": "#q", "text": text }),
        )
        .await;
        assert!(e.contains("secret d’Escouade"), "{text}: {e}");
    }
    // The agent's own token of the MCP server is one too.
    let token = h.core.mcp.token_values();
    assert!(!token.is_empty());
    let e = refused(
        &c,
        "drive_browser",
        json!({ "browser": a.id, "action": "type", "target": "#q", "text": token[0] }),
    )
    .await;
    assert!(e.contains("secret d’Escouade"), "{e}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn the_rule_of_the_hand_and_the_switches() {
    let h = harness("mcp-drive-hand");
    let (_, a) = world(&h).await;
    let pages = pages(&h);
    let c = as_agent(&h, &a.id).await;
    let scroll = json!({ "browser": a.id, "action": "scroll", "by": 100 });
    // « Reprendre la main »: refused, nothing done.
    pages.drives.moved(&a.id, HandMove::Take);
    let e = refused(&c, "drive_browser", scroll.clone()).await;
    assert!(e.contains("L’utilisateur a repris la main"), "{e}");
    let rows = read(&c, "list_browsers", json!({})).await;
    let hand = rows["browsers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["browser"] == a.id.as_str())
        .unwrap()["hand"]
        .clone();
    assert_eq!(hand, json!(null), "no agent drove it yet");
    // « La rendre »: it drives again.
    pages.drives.moved(&a.id, HandMove::Give);
    let e = refused(&c, "drive_browser", scroll.clone()).await;
    assert!(e.contains(PASSED), "{e}");
    // « Ne plus laisser piloter »: until the row closes.
    pages.drives.moved(&a.id, HandMove::Stop);
    let e = refused(&c, "drive_browser", scroll.clone()).await;
    assert!(e.contains("ne laisse plus piloter"), "{e}");
    pages.drives.forget(&a.id);
    // « Agents et navigateurs » off: the tabs are refused; the tool stays while « Pages des agents » is on.
    h.core.settings.write().agent_browsers = false;
    let e = refused(&c, "drive_browser", scroll.clone()).await;
    assert!(e.contains("Agents et navigateurs"), "{e}");
    let tools = names(&c.list_all_tools().await.unwrap());
    assert!(tools.contains(&"drive_browser".to_string()));
    // Both off: it leaves the list.
    h.core.settings.write().agent_pages = false;
    let tools = names(&c.list_all_tools().await.unwrap());
    assert!(!tools.contains(&"drive_browser".to_string()), "{tools:?}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn on_its_page_it_shows_by_the_keys_of_its_template_and_never_acts() {
    let h = harness("mcp-drive-page");
    let (p, a) = world(&h).await;
    let c = as_agent(&h, &a.id).await;
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Revue de POR-41", "template": "review", "data": {
            "subject": { "ticket": "POR-41", "branch": "ticket/por-41", "target": "main" },
            "criteria": [{ "text": "25 par page", "state": "met" }, { "text": "Le tri", "state": "unmet" }],
        } }),
    )
    .await;
    assert_eq!(v["browser"], "page:p-1");
    // Its anchors are kept beside the version, never in what the shelter serves.
    let dir = crate::pages::version_dir(h.core.data.root(), &p.id, "p-1", 1);
    let anchors = std::fs::read_to_string(crate::pages::anchors_file(&dir)).unwrap();
    assert!(
        anchors.contains("criteria.1") && anchors.contains("#crit-2"),
        "{anchors}"
    );
    assert!(!dir.join("anchors-v1.json").exists());
    // Not on screen yet.
    let drive = |args: serde_json::Value| {
        let mut args = args;
        args["browser"] = json!("page:p-1");
        args
    };
    let e = refused(&c, "drive_browser", drive(json!({ "action": "clear" }))).await;
    assert!(e.contains("n’est pas à l’écran"), "{e}");
    h.core.page_placed("p-1", Placement::Beside);
    // To show, by a key of its template, a selector or a text; tab and version.
    for args in [
        json!({ "action": "scroll_to", "target": "criteria.1" }),
        json!({ "action": "highlight", "target": "criteria.0", "label": "Critère 1" }),
        json!({ "action": "highlight", "target": "#subject" }),
        json!({ "action": "scroll", "by": 200 }),
        json!({ "action": "width", "width": "tablet" }),
        json!({ "action": "wait" }),
        json!({ "action": "tab", "target": "#tab-all" }),
        json!({ "action": "version", "n": 1 }),
    ] {
        let e = refused(&c, "drive_browser", drive(args.clone())).await;
        assert!(e.contains(PASSED), "{args}: {e}");
    }
    let e = refused(
        &c,
        "drive_browser",
        drive(json!({ "action": "scroll_to", "target": "criteria.9" })),
    )
    .await;
    assert!(
        e.contains("Aucune partie « criteria.9 »") && e.contains("criteria.1"),
        "{e}"
    );
    // Never to act, nor its history.
    for (args, said) in [
        (
            json!({ "action": "click", "target": "#subject" }),
            "Sur une page d’agent",
        ),
        (
            json!({ "action": "type", "target": "#q", "text": "x" }),
            "Sur une page d’agent",
        ),
        (
            json!({ "action": "press", "key": "Enter" }),
            "Sur une page d’agent",
        ),
        (
            json!({ "action": "navigate", "url": "http://localhost:4211/" }),
            "ni historique",
        ),
        (json!({ "action": "back" }), "ni historique"),
    ] {
        let e = refused(&c, "drive_browser", drive(args.clone())).await;
        assert!(e.contains(said), "{args}: {e}");
    }
    // Another agent's page is not found; « Pages des agents » off refuses the pages.
    let b = put_agent(&h, &p, "Bob", 2);
    let cb = as_agent(&h, &b.id).await;
    let e = refused(&cb, "drive_browser", drive(json!({ "action": "clear" }))).await;
    assert!(e.contains("Aucune page « p-1 »"), "{e}");
    cb.cancel().await.unwrap();
    h.core.settings.write().agent_pages = false;
    let e = refused(&c, "drive_browser", drive(json!({ "action": "clear" }))).await;
    assert!(e.contains("Pages des agents"), "{e}");
    c.cancel().await.unwrap();
}

#[tokio::test]
async fn a_private_projects_browsers_and_pages_are_driven_by_its_own_agents_alone() {
    let h = harness("mcp-drive-private");
    let (p, insider) = world(&h).await;
    let vault = super::private_tests::make_private(&h, &p);
    crate::harness_scope_tests::turn_on_every_harness(&h, &vault.id);
    // Its own agent drives its launch and its page.
    let c = as_agent(&h, &insider.id).await;
    let e = refused(
        &c,
        "drive_browser",
        json!({ "browser": insider.id, "action": "scroll", "by": 100 }),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    let v = read(
        &c,
        "show_page",
        json!({ "title": "Revue", "template": "html", "html": "<!doctype html><title>t</title><p id=a>x</p>" }),
    )
    .await;
    assert_eq!(v["browser"], "page:p-1");
    h.core.page_placed("p-1", Placement::Beside);
    let e = refused(
        &c,
        "drive_browser",
        json!({ "browser": "page:p-1", "action": "scroll_to", "target": "#a" }),
    )
    .await;
    assert!(e.contains(PASSED), "{e}");
    c.cancel().await.unwrap();
    // An agent of another project knows none of them: the same words as for what does not exist.
    let open = super::tools_tests::other_project(&h, "Ouvert").await;
    let outsider = put_agent(&h, &open, "Bob", 2);
    let c = as_agent(&h, &outsider.id).await;
    for (key, missing) in [
        (insider.id.as_str(), "zz-introuvable"),
        ("page:p-1", "page:p-99"),
    ] {
        let ask = |k: &str| json!({ "browser": k, "action": "scroll", "by": 100 });
        let (gone, error) = super::tools_tests::call(&c, "drive_browser", ask(missing)).await;
        assert!(error);
        let (text, error) = super::tools_tests::call(&c, "drive_browser", ask(key)).await;
        assert!(error, "{text}");
        assert!(!text.contains(PASSED), "{text}");
        let (k, m) = (
            key.trim_start_matches("page:"),
            missing.trim_start_matches("page:"),
        );
        assert_eq!(text.replace(k, m), gone, "{key}");
        assert!(!text.contains(&vault.name), "{text}");
    }
    c.cancel().await.unwrap();
    // Claude outside Escouade drives nothing, a private project's least of all.
    let x = external(&h).await;
    let e = refused(
        &x,
        "drive_browser",
        json!({ "browser": insider.id, "action": "scroll", "by": 100 }),
    )
    .await;
    assert!(
        e.contains("Claude hors d’Escouade") && !e.contains(&vault.name),
        "{e}"
    );
    x.cancel().await.unwrap();
}

#[tokio::test]
async fn reprendre_la_main_from_the_band_stops_the_agent_in_the_core_at_once() {
    // ESC-680 (AB-8): the band's « Reprendre la main » is the core's, not the screen's: the very next call is refused
    // without a gesture, « La rendre » lets the agent go on, « Continuer » with nothing waiting changes nothing, and a
    // pin of the rail that is not kept is said.
    let h = harness("mcp-drive-band");
    let (_, a) = world(&h).await;
    let pages = pages(&h);
    let c = as_agent(&h, &a.id).await;
    let scroll = json!({ "browser": a.id, "action": "scroll", "by": 100 });
    // The agent drives: its gesture passes every refusal (no webview in the tests).
    let e = refused(&c, "drive_browser", scroll.clone()).await;
    assert!(e.contains(PASSED), "{e}");
    let core = h.core.clone();
    let v = super::browsers_drive::hand(&core, &pages, &a.id, HandMove::Take).await;
    assert_eq!(v.hand, crate::browserdrive::Hand::User);
    let e = refused(&c, "drive_browser", scroll.clone()).await;
    assert!(e.contains("L’utilisateur a repris la main"), "{e}");
    // Taken twice changes nothing.
    let again = super::browsers_drive::hand(&core, &pages, &a.id, HandMove::Take).await;
    assert_eq!(again, v);
    // « Continuer » with nothing waiting: nothing.
    let v = super::browsers_drive::hand(&core, &pages, &a.id, HandMove::Continue).await;
    assert_eq!(v.hand, crate::browserdrive::Hand::User);
    // « La rendre »: the agent goes on.
    let v = super::browsers_drive::hand(&core, &pages, &a.id, HandMove::Give).await;
    assert_eq!(v.hand, crate::browserdrive::Hand::Agent);
    let e = refused(&c, "drive_browser", scroll).await;
    assert!(e.contains(PASSED), "{e}");
    // A pin nobody kept.
    let e = super::browsers_drive::revisit(&core, &pages, &a.id, 42)
        .await
        .unwrap_err();
    assert!(e.contains("no such pin"), "{e}");
    c.cancel().await.unwrap();
}
