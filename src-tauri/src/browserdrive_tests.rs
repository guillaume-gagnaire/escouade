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
    ]
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
    assert_eq!(d.done("browser:1", "Défile"), 1);
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
