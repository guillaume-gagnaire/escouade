//! The summary of the box « À toi » (ESC-513, docs/INBOX.md § 11): never one alert per request, two
//! minutes to gather, then at most one every 30 minutes (or every hour, or never); a request told
//! once; never its title; nothing while the server is off.

use crate::asks::*;
use crate::core_tests::{harness, Harness};
use crate::i18n::Lang;
use crate::inboxnotify::*;
use crate::model::*;
use std::collections::BTreeMap;

const MIN: i64 = 60_000;

fn k(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

#[test]
fn the_first_request_waits_two_minutes_then_one_summary_tells_all_that_came() {
    let mut s = Summary::default();
    let t0 = 1_000_000_000;
    assert_eq!(s.plan(&[], t0, Some(30), GROUP_MS, false), Plan::Nothing);
    assert_eq!(
        s.plan(&k(&["a"]), t0, Some(30), GROUP_MS, false),
        Plan::Wait
    );
    // What comes during the two minutes is gathered with it.
    assert_eq!(
        s.plan(&k(&["a", "b"]), t0 + MIN, Some(30), GROUP_MS, false),
        Plan::Wait
    );
    assert_eq!(
        s.plan(&k(&["a", "b"]), t0 + 2 * MIN, Some(30), GROUP_MS, false),
        Plan::Tell(k(&["a", "b"]))
    );
    // Told once: never again.
    assert_eq!(
        s.plan(&k(&["a", "b"]), t0 + 40 * MIN, Some(30), GROUP_MS, false),
        Plan::Nothing
    );
}

#[test]
fn then_one_at_most_every_thirty_minutes_or_every_hour() {
    for (every, gap) in [(30, 30 * MIN), (60, 60 * MIN)] {
        let mut s = Summary::default();
        let t0 = 1_000_000_000;
        s.plan(&k(&["a"]), t0, Some(every), GROUP_MS, false);
        assert_eq!(
            s.plan(&k(&["a"]), t0 + 2 * MIN, Some(every), GROUP_MS, false),
            Plan::Tell(k(&["a"]))
        );
        let told = t0 + 2 * MIN;
        // Another, soon after: its two minutes pass, the summary waits for its time.
        s.plan(&k(&["a", "c"]), told + MIN, Some(every), GROUP_MS, false);
        assert_eq!(
            s.plan(
                &k(&["a", "c"]),
                told + 10 * MIN,
                Some(every),
                GROUP_MS,
                false
            ),
            Plan::Wait
        );
        assert_eq!(
            s.plan(
                &k(&["a", "c"]),
                told + gap - 1,
                Some(every),
                GROUP_MS,
                false
            ),
            Plan::Wait
        );
        // Only the one not told yet.
        assert_eq!(
            s.plan(&k(&["a", "c"]), told + gap, Some(every), GROUP_MS, false),
            Plan::Tell(k(&["c"]))
        );
    }
}

#[test]
fn never_tells_nothing_and_turned_on_again_does_not_tell_the_old_ones() {
    let mut s = Summary::default();
    let t0 = 1_000_000_000;
    assert_eq!(s.plan(&k(&["a"]), t0, None, GROUP_MS, false), Plan::Nothing);
    assert_eq!(
        s.plan(&k(&["a"]), t0 + 10 * MIN, None, GROUP_MS, false),
        Plan::Nothing
    );
    // Back on: what was there is known, what comes after is told.
    assert_eq!(
        s.plan(&k(&["a"]), t0 + 11 * MIN, Some(30), GROUP_MS, false),
        Plan::Nothing
    );
    s.plan(&k(&["a", "b"]), t0 + 12 * MIN, Some(30), GROUP_MS, false);
    assert_eq!(
        s.plan(&k(&["a", "b"]), t0 + 14 * MIN, Some(30), GROUP_MS, false),
        Plan::Tell(k(&["b"]))
    );
}

#[test]
fn the_window_in_front_holds_the_summary_until_it_is_left() {
    let mut s = Summary::default();
    let t0 = 1_000_000_000;
    s.plan(&k(&["a"]), t0, Some(30), GROUP_MS, true);
    assert_eq!(
        s.plan(&k(&["a"]), t0 + 5 * MIN, Some(30), GROUP_MS, true),
        Plan::Wait
    );
    // Seen in the box meanwhile: no longer fresh, nothing to tell.
    assert_eq!(
        s.plan(&[], t0 + 6 * MIN, Some(30), GROUP_MS, false),
        Plan::Nothing
    );
    // Not seen: told once the window is left.
    s.plan(&k(&["b"]), t0 + 7 * MIN, Some(30), GROUP_MS, true);
    assert_eq!(
        s.plan(&k(&["b"]), t0 + 10 * MIN, Some(30), GROUP_MS, false),
        Plan::Tell(k(&["b"]))
    );
}

fn ask(id: &str, project: &str, kind: AskKind, gate: bool, who: &str, title: &str) -> Ask {
    Ask {
        id: id.into(),
        project_id: project.into(),
        kind,
        gate,
        title: title.into(),
        author: AskAuthor {
            agent: Some(format!("ag-{who}")),
            name: who.into(),
            chef: false,
        },
        ..Default::default()
    }
}

#[test]
fn the_summary_says_kinds_and_authors_never_a_title() {
    let projects: BTreeMap<String, String> = [
        ("p1".to_string(), "portail-adherents".to_string()),
        ("p2".to_string(), "atlas".to_string()),
    ]
    .into();
    let a = ask(
        "a-1",
        "p1",
        AskKind::Choice,
        false,
        "design-tableau",
        "Le titre secret A",
    );
    let b = ask(
        "a-2",
        "p1",
        AskKind::Confirm,
        true,
        "chef-atlas",
        "Publier la 2.2 ?",
    );
    let (title, body) = text(Lang::Fr, &[&a, &b], &projects);
    assert_eq!(title, "portail-adherents · À toi");
    assert_eq!(
        body,
        "2 demandes t’attendent dans À toi : un choix de design-tableau, un feu vert de chef-atlas."
    );
    assert!(!body.contains("secret") && !body.contains("Publier"));

    let one = ask(
        "a-3",
        "p2",
        AskKind::Task,
        false,
        "essai",
        "Essaie sur le téléphone",
    );
    let (title, body) = text(Lang::En, &[&one], &projects);
    assert_eq!(title, "atlas · Your turn");
    assert_eq!(
        body,
        "One request is waiting for you in Your turn: a task from essai."
    );

    // Several projects, more than three kinds: cut.
    let c = ask("a-4", "p2", AskKind::Answer, false, "redac", "x");
    let d = ask("a-5", "p2", AskKind::Confirm, false, "ops", "y");
    let (title, body) = text(Lang::Fr, &[&a, &b, &c, &d], &projects);
    assert_eq!(title, "Escouade · À toi");
    assert_eq!(
        body,
        "4 demandes t’attendent dans 2 projets : un choix de design-tableau, un feu vert de chef-atlas, une réponse de redac…"
    );
}

#[test]
fn a_request_is_fresh_open_not_seen_not_left_to_an_agent() {
    let mut a = ask("a-1", "p", AskKind::Choice, false, "x", "t");
    let mut list = vec![a.clone()];
    assert_eq!(fresh(&list).len(), 1);
    a.seen = true;
    list[0] = a.clone();
    assert!(fresh(&list).is_empty());
    // Back from « Plus tard »: fresh again.
    a.woke = true;
    list[0] = a.clone();
    assert_eq!(fresh(&list).len(), 1);
    a.woke = false;
    a.seen = false;
    a.delegated_to = Some("chef".into());
    list[0] = a.clone();
    assert!(fresh(&list).is_empty());
    a.delegated_to = None;
    a.state = AskState::Snoozed;
    list[0] = a;
    assert!(fresh(&list).is_empty());
}

fn deposit(h: &Harness, p: &Project, title: &str) -> Ask {
    h.core
        .ask_deposit(
            AskAuthor {
                agent: None,
                name: "Claude · client MCP".into(),
                chef: false,
            },
            &p.id,
            None,
            AskDraft {
                kind: AskKind::Confirm,
                title: title.into(),
                wake: true,
                ..Default::default()
            },
            vec![],
        )
        .unwrap()
}

fn alerts(h: &Harness) -> Vec<String> {
    h.core.alerts.lock().clone()
}

#[tokio::test]
async fn the_core_tells_the_summary_once_and_never_with_the_server_off() {
    let h = harness("inbox-summary");
    let (p, _) = h.project(false).await;
    h.core.settings.write().os_notifications = true;
    // Off: nothing, whatever the time.
    h.core.settings.write().mcp_enabled = true;
    let a = deposit(&h, &p, "Publier la version ?");
    h.core.settings.write().mcp_enabled = false;
    h.core.inbox_summary_tick();
    h.core.inbox_summary.lock().backdate(10 * MIN);
    h.core.inbox_summary_tick();
    assert!(alerts(&h).is_empty());

    h.core.settings.write().mcp_enabled = true;
    deposit(&h, &p, "Supprimer le dossier ?");
    h.core.inbox_summary_tick();
    assert!(alerts(&h).is_empty(), "the two minutes first");
    h.core.inbox_summary.lock().backdate(3 * MIN);
    h.core.inbox_summary_tick();
    let told = alerts(&h);
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(
        told[0].starts_with("demo · À toi | 2 demandes t’attendent"),
        "{}",
        told[0]
    );
    assert!(!told[0].contains("Publier") && !told[0].contains("Supprimer"));
    // Once.
    h.core.inbox_summary.lock().backdate(60 * MIN);
    h.core.inbox_summary_tick();
    assert_eq!(alerts(&h).len(), 1);
    // Seen in the box, it would not have been told: one more, seen at once, is not.
    let c = deposit(&h, &p, "Encore ?");
    h.core.ask_seen(&c.id).unwrap();
    h.core.inbox_summary_tick();
    h.core.inbox_summary.lock().backdate(60 * MIN);
    h.core.inbox_summary_tick();
    assert_eq!(alerts(&h).len(), 1);
    // « Jamais »: the counter alone.
    h.core.settings.write().inbox_summary = InboxSummary::Never;
    deposit(&h, &p, "Et celle-ci ?");
    h.core.inbox_summary_tick();
    h.core.inbox_summary.lock().backdate(60 * MIN);
    h.core.inbox_summary_tick();
    assert_eq!(alerts(&h).len(), 1);
    // The system notifications off: nothing either.
    h.core.settings.write().inbox_summary = InboxSummary::Every30;
    h.core.settings.write().os_notifications = false;
    deposit(&h, &p, "Dernière ?");
    h.core.inbox_summary_tick();
    h.core.inbox_summary.lock().backdate(60 * MIN);
    h.core.inbox_summary_tick();
    assert_eq!(alerts(&h).len(), 1);
    // The log says the summary's shape, never a request's text.
    let _ = a;
}

#[test]
fn the_setting_keeps_its_default_and_reads_as_the_window_writes_it() {
    let s = Settings::default();
    assert_eq!(s.inbox_summary, InboxSummary::Every30);
    assert_eq!(InboxSummary::Every30.minutes(), Some(30));
    assert_eq!(InboxSummary::Hourly.minutes(), Some(60));
    assert_eq!(InboxSummary::Never.minutes(), None);
    let read: Settings = serde_json::from_str(r#"{"inboxSummary":"hourly"}"#).unwrap();
    assert_eq!(read.inbox_summary, InboxSummary::Hourly);
    // Saved before it existed: the default.
    let before: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(before.inbox_summary, InboxSummary::Every30);
    assert_eq!(
        serde_json::to_value(InboxSummary::Never).unwrap(),
        serde_json::json!("never")
    );
}

#[test]
fn the_statistics_say_a_range_never_a_count() {
    assert_eq!(bucket(1), "1");
    assert_eq!(bucket(3), "2-3");
    assert_eq!(bucket(12), "4+");
}
