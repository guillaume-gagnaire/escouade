//! The rules of the driving (ESC-679, AB-7; docs/AGENT-BROWSERS.md § 2 bis.2, § 3.3, § 5.2, § 9): the tiers by kind of
//! row, what is never done even with a leave, the targets, the bounds, the rule of the hand, `wait_user`, the leave to
//! fill a password (C2), and the closed list of the DevTools protocol with the inputs it lets through. The gestures in
//! a real page are the e2e's (agent-browsers-drive.spec.ts).

use crate::browserdrive::*;
use crate::browserread::{guard, Method};
use crate::devpage::PageMode;
use serde_json::json;
use std::time::{Duration, Instant};

fn css(s: &str) -> Target {
    Target::Css(s.into())
}

fn every_act() -> Vec<Act> {
    vec![
        Act::Navigate("http://localhost:4100/".into()),
        Act::Back,
        Act::Forward,
        Act::Reload,
        Act::Wait(None, Duration::from_secs(1)),
        Act::ScrollTo(css("#a")),
        Act::Scroll(200),
        Act::Highlight(css("#a"), "Offre".into()),
        Act::Clear,
        Act::Width(PageMode::Mobile),
        Act::Tab(css("#t")),
        Act::Click(css("#a")),
        Act::Type {
            target: css("#name"),
            text: "Ada".into(),
            submit: false,
        },
        Act::Select(css("#s"), "b".into()),
        Act::Press(Key::Enter),
        Act::Version(2),
        Act::InFrame {
            frame: css("iframe#art"),
            gesture: FrameGesture::ScrollTo,
            target: Some(css("#risques")),
            label: String::new(),
        },
        Act::InFrame {
            frame: css("iframe#art"),
            gesture: FrameGesture::Highlight,
            target: None,
            label: "L’artefact".into(),
        },
    ]
}

#[test]
fn a_gesture_in_a_frame_is_one_to_show() {
    // scroll_to and highlight in a frame of another origin (ESC-682, AB-11): to show, without asking, everywhere.
    let outside = Line::Outside("https://claude.ai".into());
    for act in every_act()
        .into_iter()
        .filter(|a| matches!(a, Act::InFrame { .. }))
    {
        assert_eq!(act.tier(), Tier::Show);
        assert_eq!(decide(&outside, &act, false), Verdict::Go);
        assert_eq!(decide(&Line::Launch, &act, false), Verdict::Go);
        // Not an element of the document: the checks of an element to act on are never asked for it.
        assert_eq!(act.target(), None);
        let kind = Kind {
            password: true,
            file: true,
            ..Kind::default()
        };
        assert_eq!(forbid(&outside, &act, &kind, false), None);
    }
    let names: Vec<&str> = every_act()
        .iter()
        .filter(|a| matches!(a, Act::InFrame { .. }))
        .map(Act::name)
        .collect();
    assert_eq!(names, ["scroll_to", "highlight"]);
    // The answer says where the gesture was made.
    assert_eq!(serde_json::to_value(Reach::Whole).unwrap(), json!("whole"));
    #[cfg(windows)]
    assert_eq!(
        serde_json::to_value(Reach::Inside).unwrap(),
        json!("inside")
    );
}

#[test]
fn a_frame_is_reached_by_the_closed_list_alone() {
    use crate::browserread::{auto_attach, guard_frame, pick_frame, Frame};
    // Target.setAutoAttach: the iframes of the document, never paused, flat; nothing else of the Target domain.
    assert_eq!(
        Method::parse("Target.setAutoAttach").unwrap(),
        Method::TargetSetAutoAttach
    );
    for name in [
        "Target.getTargets",
        "Target.attachToTarget",
        "Target.createTarget",
        "Target.setDiscoverTargets",
        "Target.sendMessageToTarget",
    ] {
        assert!(Method::parse(name).is_err(), "{name}");
    }
    assert!(guard(Method::TargetSetAutoAttach, &auto_attach(), None).is_ok());
    for bad in [
        json!({ "autoAttach": true, "waitForDebuggerOnStart": true, "flatten": true, "filter": [{ "type": "iframe", "exclude": false }] }),
        json!({ "autoAttach": true, "waitForDebuggerOnStart": false, "flatten": true }),
        json!({ "autoAttach": true, "waitForDebuggerOnStart": false, "flatten": true, "filter": [{ "type": "page", "exclude": false }] }),
    ] {
        assert!(
            guard(Method::TargetSetAutoAttach, &bad, None).is_err(),
            "{bad}"
        );
    }
    // In a frame's session: its tree, Escouade's world, Escouade's scripts in that world; no input, capture, network.
    let drive = expression(&json!({ "run": "scroll_to", "target": { "css": "#risques" } }));
    let eval = |e: &str, ctx: i64| {
        guard_frame(
            Method::RuntimeEvaluate,
            &json!({ "expression": e, "contextId": ctx, "returnByValue": true, "silent": true }),
            Some(12),
        )
    };
    assert!(eval(&drive, 12).is_ok());
    assert!(eval(&drive, 3).is_err(), "the frame's own world");
    assert!(eval("document.body.innerText", 12).is_err());
    assert!(guard_frame(Method::PageGetFrameTree, &json!({}), None).is_ok());
    assert!(guard_frame(
        Method::PageCreateIsolatedWorld,
        &json!({ "frameId": "F", "worldName": "escouade-agent" }),
        None
    )
    .is_ok());
    for (m, p) in [
        (
            Method::InputDispatchMouseEvent,
            json!({ "type": "mousePressed", "x": 1.0, "y": 1.0 }),
        ),
        (Method::InputInsertText, json!({ "text": "x" })),
        (Method::PageCaptureScreenshot, json!({})),
        (Method::PageNavigate, json!({ "url": "https://claude.ai/" })),
        (Method::NetworkEnable, json!({})),
        (Method::RuntimeEnable, json!({})),
        (Method::AccessibilityGetFullAxTree, json!({})),
        (Method::TargetSetAutoAttach, auto_attach()),
    ] {
        assert!(guard_frame(m, &p, Some(12)).is_err(), "{}", m.name());
    }
    // Which frame is the <iframe>'s: of its origin, by rank; a lone frame whose address went elsewhere.
    let f = |id: &str, url: &str, s: Option<&str>| Frame {
        id: id.into(),
        url: url.into(),
        session: s.map(String::from),
    };
    let frames = [
        f("A", "http://localhost:4100/ad", None),
        f("B", "https://a.claudeusercontent.com/x?v=2", Some("S1")),
        f("C", "https://a.claudeusercontent.com/y", Some("S2")),
    ];
    let pick = |src: &str, rank: usize| pick_frame(&frames, src, rank).map(|f| f.id.as_str());
    assert_eq!(pick("http://localhost:4100/ad", 0), Some("A"));
    assert_eq!(pick("https://a.claudeusercontent.com/x", 0), Some("B"));
    assert_eq!(pick("https://a.claudeusercontent.com/x", 1), Some("C"));
    assert_eq!(pick("https://a.claudeusercontent.com/x", 2), None);
    assert_eq!(pick("https://elsewhere.example/", 0), None);
    assert_eq!(pick("about:blank", 0), None);
    let lone = [f("Z", "https://b.example/after-redirect", Some("S9"))];
    assert_eq!(
        pick_frame(&lone, "https://a.example/", 0).map(|f| f.id.as_str()),
        Some("Z")
    );
    assert_eq!(pick_frame(&[], "https://a.example/", 0), None);
}

#[test]
fn the_tiers_by_kind_of_row() {
    let outside = Line::Outside("https://claude.ai".into());
    for act in every_act() {
        let name = act.name();
        let show = act.tier() == Tier::Show;
        // On the launch: everything but version, without asking.
        let want = if matches!(act, Act::Version(_)) {
            Verdict::Refused(Refusal::PageOnly)
        } else {
            Verdict::Go
        };
        assert_eq!(decide(&Line::Launch, &act, false), want, "launch {name}");
        // Outside the launch: to show without asking, to act with the leave (C4).
        let want = match (&act, show) {
            (Act::Version(_), _) => Verdict::Refused(Refusal::PageOnly),
            (_, true) => Verdict::Go,
            (_, false) => Verdict::Leave,
        };
        assert_eq!(decide(&outside, &act, false), want, "outside {name}");
        if !matches!(act, Act::Version(_)) {
            assert_eq!(
                decide(&outside, &act, true),
                Verdict::Go,
                "outside with leave {name}"
            );
        }
        // An agent's page: to show, tab and version; never to act, nor its history.
        let want = match &act {
            Act::Version(_) | Act::Tab(_) => Verdict::Go,
            Act::Navigate(_) | Act::Back | Act::Forward => Verdict::Refused(Refusal::TabOnly),
            a if a.tier() == Tier::Act => Verdict::Refused(Refusal::ActOnPage),
            _ => Verdict::Go,
        };
        assert_eq!(decide(&Line::AgentPage, &act, true), want, "page {name}");
    }
    // The tiers themselves.
    let acting: Vec<&str> = every_act()
        .iter()
        .filter(|a| a.tier() == Tier::Act)
        .map(Act::name)
        .collect();
    assert_eq!(
        acting,
        ["navigate", "tab", "click", "type", "select", "press"]
    );
}

#[test]
fn what_is_never_done_even_with_a_leave() {
    let outside = Line::Outside("https://claude.ai".into());
    let field = Kind {
        tag: "input".into(),
        kind: "text".into(),
        editable: true,
        ..Default::default()
    };
    let typing = Act::Type {
        target: css("#f"),
        text: "x".into(),
        submit: false,
    };
    let click = Act::Click(css("#f"));
    // A plain field and a plain button.
    assert_eq!(forbid(&Line::Launch, &typing, &field, false), None);
    assert_eq!(forbid(&outside, &click, &Kind::default(), false), None);
    // A file: never, by click or by type, anywhere.
    let file = Kind {
        file: true,
        ..field.clone()
    };
    for line in [Line::Launch, outside.clone()] {
        assert_eq!(forbid(&line, &click, &file, true), Some(Forbidden::File));
        assert_eq!(forbid(&line, &typing, &file, true), Some(Forbidden::File));
    }
    // A download: never clicked nor pressed.
    let download = Kind {
        download: true,
        ..Default::default()
    };
    assert_eq!(
        forbid(&Line::Launch, &click, &download, true),
        Some(Forbidden::Download)
    );
    assert_eq!(
        forbid(&Line::Launch, &Act::Press(Key::Enter), &download, true),
        Some(Forbidden::Download)
    );
    // A field of a form that goes to another origin.
    let cross = Kind {
        cross_form: true,
        ..field.clone()
    };
    for act in [&typing, &click, &Act::Press(Key::Enter)] {
        assert_eq!(
            forbid(&Line::Launch, act, &cross, true),
            Some(Forbidden::CrossForm)
        );
    }
    // A password: on the launch with the user's leave (C2), asked without it; outside it never, even with one.
    let password = Kind {
        password: true,
        kind: "password".into(),
        ..field.clone()
    };
    assert_eq!(forbid(&Line::Launch, &typing, &password, true), None);
    assert_eq!(
        forbid(&Line::Launch, &typing, &password, false),
        Some(Forbidden::PasswordAsk)
    );
    assert_eq!(
        forbid(&outside, &typing, &password, true),
        Some(Forbidden::PasswordOutside)
    );
    // Typing where there is no text field, choosing outside a list, a tab that is not declared.
    assert_eq!(
        forbid(&Line::Launch, &typing, &Kind::default(), true),
        Some(Forbidden::NotText)
    );
    assert_eq!(
        forbid(
            &Line::Launch,
            &Act::Select(css("#s"), "a".into()),
            &field,
            true
        ),
        Some(Forbidden::NotSelect)
    );
    assert_eq!(
        forbid(
            &Line::AgentPage,
            &Act::Tab(css("#t")),
            &Kind::default(),
            true
        ),
        Some(Forbidden::NotTab)
    );
    let tab = Kind {
        tab: true,
        ..Default::default()
    };
    assert_eq!(
        forbid(&Line::AgentPage, &Act::Tab(css("#t")), &tab, true),
        None
    );
    // Showing is never refused for what an element is.
    assert_eq!(
        forbid(&Line::Launch, &Act::ScrollTo(css("#f")), &file, false),
        None
    );
}

#[test]
fn a_secret_of_escouade_is_never_typed() {
    let secrets = vec!["s3cret-of-the-mcp-token".to_string(), "ab".to_string()];
    assert!(holds_secret(
        "mon jeton : s3cret-of-the-mcp-token",
        &secrets
    ));
    // A value too short to be told apart is not one.
    assert!(!holds_secret("ab cd", &secrets));
    // What has the form of a key or a token, known or not.
    assert!(holds_secret(
        "ghp_0123456789abcdefghijklmnopqrstuvwxyzAB",
        &[]
    ));
    assert!(holds_secret(
        "Authorization: Bearer abcdefghijklmnop0123",
        &[]
    ));
    assert!(!holds_secret("Ada Lovelace", &secrets));
}

#[test]
fn the_targets() {
    let keys = |k: &str| -> Result<String, String> {
        match k {
            "criteria.2" => Ok("#crit-3".into()),
            "subject" => Ok("#subject".into()),
            _ => Err("subject, criteria.2".into()),
        }
    };
    let k: &dyn Fn(&str) -> Result<String, String> = &keys;
    assert_eq!(target("#tarifs", None), Ok(css("#tarifs")));
    assert_eq!(
        target("main .offre > button", None),
        Ok(css("main .offre > button"))
    );
    assert_eq!(
        target("text=Envoyer", None),
        Ok(Target::Text("Envoyer".into()))
    );
    assert_eq!(
        target("text=\"Offre Pro\"", None),
        Ok(Target::Text("Offre Pro".into()))
    );
    assert_eq!(target("", None), Err(BadTarget::Empty));
    assert_eq!(target("text=", None), Err(BadTarget::Empty));
    assert_eq!(target(&"a".repeat(201), None), Err(BadTarget::TooLong));
    assert_eq!(target("div:has(input)", None), Err(BadTarget::Selector));
    assert_eq!(target("a b c d e", None), Err(BadTarget::Selector));
    // A key of the page's template, on an agent's page alone.
    assert_eq!(target("criteria.2", Some(k)), Ok(css("#crit-3")));
    assert_eq!(target("subject", Some(k)), Ok(css("#subject")));
    assert_eq!(
        target("criteria.9", Some(k)),
        Err(BadTarget::Key("subject, criteria.2".into()))
    );
    // A bare word that is no key stays an element's name.
    assert_eq!(target("main", Some(k)), Ok(css("main")));
    assert!(is_key("verdict.a") && is_key("nodes.B-2") && !is_key("#x") && !is_key("a b"));
}

#[test]
fn press_takes_the_closed_list_alone() {
    for name in [
        "Enter",
        "Escape",
        "Tab",
        "ArrowUp",
        "ArrowDown",
        "ArrowLeft",
        "ArrowRight",
        "PageUp",
        "PageDown",
        "Home",
        "End",
    ] {
        let k = Key::parse(name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(k.codes().0, name);
    }
    for name in ["F12", "Delete", "Control", "a", "Meta", ""] {
        assert!(Key::parse(name).is_none(), "{name}");
    }
}

#[test]
fn four_gestures_a_second_and_six_hundred_an_hour() {
    let mut pace = Pace::default();
    let t0 = Instant::now();
    for i in 0..4 {
        assert_eq!(pace.take(t0 + Duration::from_millis(i * 10)), Ok(()), "{i}");
    }
    assert_eq!(pace.take(t0 + Duration::from_millis(50)), Err(1));
    assert_eq!(pace.take(t0 + Duration::from_millis(1010)), Ok(()));
    // 600 in an hour, at most.
    let mut pace = Pace::default();
    for i in 0..600u64 {
        assert_eq!(
            pace.take(t0 + Duration::from_millis(i * 300)),
            Ok(()),
            "{i}"
        );
    }
    let wait = pace.take(t0 + Duration::from_secs(200)).unwrap_err();
    assert!(wait > 3000, "{wait}");
    assert_eq!(pace.take(t0 + Duration::from_secs(3601)), Ok(()));
}

#[test]
fn the_rule_of_the_hand() {
    let d = Drives::default();
    let now = Instant::now();
    assert_eq!(d.take("browser:1", "esc-1", now), May::Go);
    assert_eq!(d.hand("browser:1"), Some(Hand::Agent));
    assert_eq!(d.done("browser:1", "Défile", &json!({})), 1);
    // The user's first gesture (or « Reprendre la main »): every gesture refused, the hand theirs.
    assert!(d.user_took("browser:1"));
    assert!(!d.user_took("browser:1"));
    assert_eq!(d.blocked("browser:1"), Some(May::UserHasHand));
    assert_eq!(
        d.take("browser:1", "esc-1", now + Duration::from_secs(1)),
        May::UserHasHand
    );
    assert_eq!(d.hand("browser:1"), Some(Hand::User));
    // « La rendre »: the agent may again, and is the one to tell.
    assert_eq!(
        d.moved("browser:1", HandMove::Give),
        (true, Some("esc-1".to_string()))
    );
    assert_eq!(d.blocked("browser:1"), None);
    // What the user did while the hand was theirs is forgotten once, when it comes back.
    assert!(d.take_handed_back("browser:1"));
    assert!(!d.take_handed_back("browser:1"));
    assert_eq!(
        d.take("browser:1", "esc-1", now + Duration::from_secs(2)),
        May::Go
    );
    // « Reprendre la main » from the band.
    assert_eq!(
        d.moved("browser:1", HandMove::Take),
        (true, Some("esc-1".to_string()))
    );
    assert_eq!(d.view("browser:1").hand, Hand::User);
    // « Ne plus laisser piloter »: until the row closes; « La rendre » no longer gives it back.
    d.moved("browser:1", HandMove::Stop);
    assert_eq!(d.moved("browser:1", HandMove::Give), (false, None));
    assert_eq!(
        d.take("browser:1", "esc-1", now + Duration::from_secs(3)),
        May::Stopped
    );
    assert!(d.view("browser:1").stopped);
    d.forget("browser:1");
    assert_eq!(
        d.take("browser:1", "esc-1", now + Duration::from_secs(4)),
        May::Go
    );
    // The pace, per row.
    let t = now + Duration::from_secs(10);
    for _ in 0..4 {
        assert_eq!(d.take("browser:1", "esc-1", t), May::Go);
    }
    assert_eq!(d.take("browser:1", "esc-1", t), May::Slow(1));
    assert_eq!(d.take("browser:2", "esc-1", t), May::Go);
    // What the page does during an agent's gesture, and for a moment after, is the agent's.
    assert!(d.acted_lately("browser:1", CHOICE_QUIET, t + Duration::from_millis(500)));
    assert!(!d.acted_lately("browser:1", CHOICE_QUIET, t + Duration::from_secs(3)));
    assert!(!d.acted_lately("browser:9", CHOICE_QUIET, t));
}

#[test]
fn a_new_version_of_the_artifact_is_reloaded_while_the_hand_is_the_agents() {
    let d = Drives::default();
    // Opened, never driven yet: the reload is a step of the band, the agent's name on it.
    assert_eq!(
        d.reloaded("browser:1", "esc-1", "Recharge : version 2"),
        Some(1)
    );
    let v = d.view("browser:1");
    assert_eq!((v.step, v.what.as_str()), (1, "Recharge : version 2"));
    assert_eq!(v.agent.as_deref(), Some("esc-1"));
    // Not a gesture of its pace: four of its own still go in the same second.
    let now = Instant::now();
    for _ in 0..4 {
        assert_eq!(d.take("browser:1", "esc-1", now), May::Go);
    }
    // The user has the hand: the row is left as they have it.
    d.user_took("browser:1");
    assert_eq!(
        d.reloaded("browser:1", "esc-1", "Recharge : version 3"),
        None
    );
    assert_eq!(d.view("browser:1").step, 1);
    // Given back, it reloads again; the driving ended, never.
    d.moved("browser:1", HandMove::Give);
    assert_eq!(
        d.reloaded("browser:1", "esc-1", "Recharge : version 3"),
        Some(2)
    );
    d.moved("browser:1", HandMove::Stop);
    assert_eq!(
        d.reloaded("browser:1", "esc-1", "Recharge : version 4"),
        None
    );
}

#[tokio::test]
async fn wait_user_ends_by_continue_by_the_hand_or_by_the_row() {
    let d = Drives::default();
    d.take("k", "esc-1", Instant::now());
    let rx = d.wait("k");
    assert!(d.view("k").waiting);
    assert_eq!(d.moved("k", HandMove::Continue), (true, None));
    assert_eq!(rx.await.unwrap(), WaitEnd::Continued);
    assert!(!d.view("k").waiting);
    // The user's gesture ends it: took_hand.
    let rx = d.wait("k");
    d.user_took("k");
    assert_eq!(rx.await.unwrap(), WaitEnd::TookHand);
    // « Continuer » with nothing waiting changes nothing.
    assert_eq!(d.moved("k", HandMove::Continue), (false, None));
    // The row closes.
    d.moved("k", HandMove::Give);
    let rx = d.wait("k");
    d.forget("k");
    assert_eq!(rx.await.unwrap(), WaitEnd::TookHand);
}

#[test]
fn a_password_waits_for_the_users_leave_once_per_browser() {
    let d = Drives::default();
    assert_eq!(d.password("browser:1", "esc-1"), Ok(false));
    assert_eq!(d.view("browser:1").password_asks, ["esc-1"]);
    // Asked again: still waiting, one request.
    assert_eq!(d.password("browser:1", "esc-1"), Ok(false));
    assert_eq!(d.view("browser:1").password_asks.len(), 1);
    assert!(d.answer_password("browser:1", "esc-1", true));
    assert!(!d.answer_password("browser:1", "esc-1", true));
    assert_eq!(d.password("browser:1", "esc-1"), Ok(true));
    // Another browser asks again; a refusal holds.
    assert_eq!(d.password("browser:2", "esc-1"), Ok(false));
    assert!(d.answer_password("browser:2", "esc-1", false));
    assert_eq!(d.password("browser:2", "esc-1"), Err(()));
    // The leave falls with the browser.
    d.forget("browser:1");
    assert_eq!(d.password("browser:1", "esc-1"), Ok(false));
}

#[test]
fn the_closed_list_of_the_devtools_protocol() {
    // Nothing outside the list: no cookie, storage, header, body, file nor download, no script of the page's world.
    for name in [
        "Network.getCookies",
        "Network.getAllCookies",
        "Storage.getCookies",
        "Network.getResponseBody",
        "Network.setExtraHTTPHeaders",
        "DOMStorage.getDOMStorageItems",
        "IndexedDB.requestData",
        "DOM.setFileInputFiles",
        "Page.setDownloadBehavior",
        "Browser.setDownloadBehavior",
        "Runtime.callFunctionOn",
        "Runtime.compileScript",
        "Page.addScriptToEvaluateOnNewDocument",
        "Fetch.enable",
        "Target.attachToTarget",
        "Input.dispatchDragEvent",
    ] {
        assert!(Method::parse(name).is_err(), "{name}");
    }
    // The driving's inputs: a click, a wheel, a key of the list, a bounded text.
    let ok = |m: Method, p: serde_json::Value| guard(m, &p, Some(7));
    assert!(ok(
        Method::InputDispatchMouseEvent,
        json!({ "type": "mousePressed", "x": 10.0, "y": 20.0, "button": "left", "clickCount": 1 })
    )
    .is_ok());
    assert!(ok(
        Method::InputDispatchMouseEvent,
        json!({ "type": "mouseDragged", "x": 10.0, "y": 20.0 })
    )
    .is_err());
    assert!(ok(
        Method::InputDispatchMouseEvent,
        json!({ "type": "mousePressed", "x": -5.0, "y": 20.0 })
    )
    .is_err());
    assert!(ok(
        Method::InputDispatchKeyEvent,
        json!({ "type": "keyDown", "key": "Enter", "code": "Enter", "windowsVirtualKeyCode": 13, "text": "\r" })
    )
    .is_ok());
    assert!(ok(
        Method::InputDispatchKeyEvent,
        json!({ "type": "keyDown", "key": "F12", "code": "F12" })
    )
    .is_err());
    assert!(ok(
        Method::InputDispatchKeyEvent,
        json!({ "type": "keyDown", "key": "Enter", "modifiers": 2 })
    )
    .is_err());
    assert!(ok(
        Method::InputDispatchKeyEvent,
        json!({ "type": "char", "key": "Enter" })
    )
    .is_err());
    assert!(ok(Method::InputInsertText, json!({ "text": "Ada" })).is_ok());
    assert!(ok(Method::InputInsertText, json!({ "text": "a".repeat(2001) })).is_err());
    assert!(ok(
        Method::EmulationSetDeviceMetricsOverride,
        json!({ "width": 390, "height": 700 })
    )
    .is_ok());
    assert!(ok(
        Method::EmulationSetDeviceMetricsOverride,
        json!({ "width": 10 })
    )
    .is_err());
    // Runtime.evaluate: Escouade's libraries alone (the driving's among them), in Escouade's world alone.
    let drive = expression(&json!({ "run": "where" }));
    let eval = |e: &str, ctx: i64| {
        guard(
            Method::RuntimeEvaluate,
            &json!({ "expression": e, "contextId": ctx, "returnByValue": true, "silent": true }),
            Some(7),
        )
    };
    assert!(eval(&drive, 7).is_ok());
    assert!(eval(&drive, 1).is_err(), "the page's world");
    assert!(eval("document.cookie", 7).is_err());
    assert!(eval(&format!("localStorage.token;{drive}"), 7).is_err());
}

#[test]
fn the_librarys_expression_carries_values_never_code() {
    let e = expression(&json!({ "run": "locate", "target": { "css": "\");alert(1);(\"" } }));
    assert!(e.starts_with(&format!("({LIB}")));
    // The argument is JSON: a quote of the agent's stays inside its string.
    assert!(
        e.ends_with(r#"({"run":"locate","target":{"css":"\");alert(1);(\""}})"#),
        "{e}"
    );
}

// ---------- what the user sees (ESC-680, AB-8) ----------

#[test]
fn the_overlay_draws_a_node_of_escouade_s_world_and_a_quad_and_nothing_else() {
    let ok = |m: Method, p: serde_json::Value| guard(m, &p, Some(7));
    // The two domains, without parameters.
    for m in [
        Method::DomEnable,
        Method::OverlayEnable,
        Method::OverlayHideHighlight,
    ] {
        assert!(ok(m, json!({})).is_ok(), "{}", m.name());
        assert!(
            ok(m, json!({ "includeWhitespace": "all" })).is_err(),
            "{}",
            m.name()
        );
    }
    // A node: the object Escouade's world found, and how to paint it; never a node id nor a selector of the page's.
    let config =
        json!({ "showInfo": false, "contentColor": { "r": 59, "g": 130, "b": 246, "a": 0.16 } });
    assert!(ok(
        Method::OverlayHighlightNode,
        json!({ "objectId": "-3141.7.2", "highlightConfig": config })
    )
    .is_ok());
    assert!(ok(
        Method::OverlayHighlightNode,
        json!({ "highlightConfig": config })
    )
    .is_err());
    assert!(ok(
        Method::OverlayHighlightNode,
        json!({ "objectId": "", "highlightConfig": config })
    )
    .is_err());
    assert!(ok(
        Method::OverlayHighlightNode,
        json!({ "objectId": "1", "selector": "#tarifs", "highlightConfig": config })
    )
    .is_err());
    // The ghost cursor: four points of the view, two colours.
    let quad = [100.0, 200.0, 100.0, 219.0, 105.5, 214.5, 113.0, 213.5];
    assert!(ok(
        Method::OverlayHighlightQuad,
        json!({ "quad": quad, "color": { "r": 59, "g": 130, "b": 246, "a": 0.95 }, "outlineColor": { "r": 255, "g": 255, "b": 255, "a": 1.0 } })
    )
    .is_ok());
    assert!(ok(Method::OverlayHighlightQuad, json!({ "quad": [1, 2, 3] })).is_err());
    assert!(ok(
        Method::OverlayHighlightQuad,
        json!({ "quad": [0, 0, 0, 1e9, 0, 0, 0, 0] })
    )
    .is_err());
    assert!(ok(
        Method::OverlayHighlightQuad,
        json!({ "quad": quad, "frameId": "x" })
    )
    .is_err());
    // The other drawings of the overlay stay out of the list (the inspector's, the debugger's).
    for name in [
        "Overlay.setInspectMode",
        "Overlay.setPausedInDebuggerMessage",
        "Overlay.highlightRect",
        "DOM.getDocument",
        "DOM.querySelector",
        "DOM.getOuterHTML",
    ] {
        assert!(Method::parse(name).is_err(), "{name}");
    }
}

#[test]
fn the_ghost_cursor_glides_at_the_projects_colour_with_the_agents_initials() {
    // The projects' palette writes oklch (theme.ts): its orange, and the reference red of CSS Color 4.
    let (r, g, b) = srgb("oklch(0.72 0.12 48)").unwrap();
    assert!(r > g && g > b, "an orange: {r} {g} {b}");
    let (r, g, b) = srgb("oklch(62.8% 0.2577 29.23)").unwrap();
    assert!(r >= 250 && g <= 6 && b <= 6, "red: {r} {g} {b}");
    assert_eq!(hex("#3B82F6"), "#3b82f6");
    assert_eq!(hex("oklch(nope)"), "#7c6cf0");
    assert_eq!(srgb("rgb(1, 2, 3)"), None);
    // The initials, as the projects' tiles make them.
    assert_eq!(initials("esc-412"), "E4");
    assert_eq!(initials("pagination"), "Pa");
    assert_eq!(initials("chef portail"), "CP");
    assert_eq!(initials("élan"), "Él");
    // The middle of a box.
    assert_eq!(
        centre(&json!({ "x": 10, "y": 20, "width": 100, "height": 40 })),
        Some((60.0, 40.0))
    );
    assert_eq!(centre(&json!(null)), None);
    // Each cursor its turn: a later one ends the former's.
    let d = Drives::default();
    let (from, one) = d.cursor_to("k", (1.0, 2.0));
    assert_eq!(from, None);
    let (from, two) = d.cursor_to("k", (3.0, 4.0));
    assert_eq!(from, Some((1.0, 2.0)));
    assert!(!d.cursor_still("k", one));
    assert!(d.cursor_still("k", two));
    // What it highlights, drawn again after the cursor; none once cleared.
    d.set_mark("k", Some(json!({ "css": "#tarifs" })));
    assert_eq!(d.mark("k"), Some(json!({ "css": "#tarifs" })));
    d.set_mark("k", None);
    assert_eq!(d.mark("k"), None);
}

/// Windows' overlay: the project's colour as the protocol's RGBA, the arrow's quad, the glide.
#[cfg(windows)]
#[test]
fn the_overlay_paints_the_arrow_at_the_projects_colour_and_glides_it() {
    // Another value is Escouade's violet.
    assert_eq!(
        rgba("#3B82F6", 0.5),
        json!({ "r": 59, "g": 130, "b": 246, "a": 0.5 })
    );
    assert_eq!(
        rgba("red", 1.0),
        json!({ "r": 124, "g": 108, "b": 240, "a": 1.0 })
    );
    // Its tip where the agent acts; the quad the guard lets through.
    let a = arrow(10.0, 20.0);
    assert_eq!((a[0], a[1]), (10.0, 20.0));
    assert!(a[3] > a[1] && a[6] > a[0]);
    assert!(guard(
        Method::OverlayHighlightQuad,
        &json!({ "quad": arrow(100.0, 200.0), "color": rgba("#3b82f6", 0.95), "outlineColor": rgba("#ffffff", 1.0) }),
        Some(7)
    )
    .is_ok());
    // A first one appears there; the next glides from the former, slowing, and ends there.
    assert_eq!(glide(None, (5.0, 5.0), 7), vec![(5.0, 5.0)]);
    assert_eq!(glide(Some((5.0, 5.0)), (5.0, 5.0), 7), vec![(5.0, 5.0)]);
    let g = glide(Some((0.0, 0.0)), (100.0, 50.0), 7);
    assert_eq!(g.len(), 7);
    assert_eq!(*g.last().unwrap(), (100.0, 50.0));
    assert!(g[0].0 > 100.0 / 7.0, "fast first: {g:?}");
    assert!(g.windows(2).all(|w| w[1].0 >= w[0].0));
}

#[test]
fn each_gesture_is_a_numbered_pin_with_where_the_page_was() {
    let d = Drives::default();
    let now = Instant::now();
    d.take("page:p-3", "esc-1", now);
    let at =
        json!({ "scrollY": 840.4, "height": 2400, "view": 700, "url": "escouade-page://p-3/" });
    assert_eq!(d.done("page:p-3", "Défile jusqu’à « Critères »", &at), 1);
    assert_eq!(
        d.done("page:p-3", "Met en évidence « Critère 3 »", &json!({})),
        2
    );
    let v = d.view("page:p-3");
    assert_eq!(v.step, 2);
    assert_eq!(v.what, "Met en évidence « Critère 3 »");
    assert_eq!(
        v.pins[0],
        Pin {
            n: 1,
            what: "Défile jusqu’à « Critères »".into(),
            y: 840,
            height: 2400,
            view: 700
        }
    );
    // A page that said nothing of where it was: at the top.
    assert_eq!((v.pins[1].y, v.pins[1].height), (0, 0));
    assert_eq!(d.pin("page:p-3", 1).map(|p| p.y), Some(840));
    assert_eq!(d.pin("page:p-3", 9), None);
    // The oldest go past the bound; the numbers go on.
    for _ in 0..PINS_MAX + 5 {
        d.done("page:p-3", "Défile", &json!({}));
    }
    let v = d.view("page:p-3");
    assert_eq!(v.pins.len(), PINS_MAX);
    assert_eq!(v.pins.last().unwrap().n, v.step);
    assert_eq!(d.pin("page:p-3", 1), None);
    // They fall with the row.
    d.forget("page:p-3");
    assert!(d.view("page:p-3").pins.is_empty());
}

#[test]
fn taking_the_hand_back_stops_a_gesture_not_begun_yet() {
    // « Reprendre la main » between the check of a gesture and the gesture itself: the core makes none.
    let d = Drives::default();
    assert_eq!(d.take("browser:1", "esc-1", Instant::now()), May::Go);
    assert_eq!(d.still("browser:1"), May::Go);
    d.moved("browser:1", HandMove::Take);
    assert_eq!(d.still("browser:1"), May::UserHasHand);
    // Given back: the next one goes.
    d.moved("browser:1", HandMove::Give);
    assert_eq!(d.still("browser:1"), May::Go);
    d.moved("browser:1", HandMove::Stop);
    assert_eq!(d.still("browser:1"), May::Stopped);
    // A row nobody drives: nothing stops it.
    assert_eq!(d.still("browser:2"), May::Go);
}
