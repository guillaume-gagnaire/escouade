use super::*;
use crate::locks::{Locks, Managed};
use crate::paths::{test_dir, DataDir};
use escouade_policy::list::{
    DICTATION, MCP_EXTERNAL, PLUGIN_ORIGINS_KEY, POLICIES as KEYS, TEAM_SERVER,
};
use serde_json::{json, Value};
use tauri::test::mock_app;

/// A device set up, with a project and a microphone.
fn set_up() -> Facts {
    Facts {
        projects: 1,
        accounts: 1,
        microphones: Some(1),
        ..Default::default()
    }
}

fn item(v: &DiscoverView, id: Discovery) -> &DiscoverItem {
    v.items.iter().find(|i| i.id == id).unwrap()
}

#[test]
fn every_discovery_has_its_lane_and_the_map_is_in_its_order() {
    let ids: Vec<Discovery> = all().collect();
    // Escouade remote is not in this version: never shown.
    assert!(!ids.contains(&Discovery::Remote));
    assert_eq!(ids.len(), 19);
    assert_eq!(ids[0], Discovery::Ticket);
    assert_eq!(lane_of(Discovery::Dictation), Lane::Code);
    assert_eq!(lane_of(Discovery::Approve), Lane::Ship);
    assert_eq!(lane_of(Discovery::Team), Lane::Drive);
    let v = view(&Onboarding::default(), &set_up(), "2.3.0", true);
    assert_eq!(v.items.len(), 19);
    assert_eq!(
        v.total, 18,
        "no team server: the team is not on this device"
    );
    assert_eq!(v.done, 0);
    assert!(v.dot);
}

#[test]
fn the_real_state_ticks_a_discovery_and_it_stays_done() {
    let mut saved = Onboarding::default();
    let mut f = Facts {
        tickets: 2,
        agents_in_a_project: 2,
        accounts: 2,
        mcp_on: true,
        ..set_up()
    };
    assert!(settle(&mut saved, &f));
    let v = view(&saved, &f, "2.3.0", true);
    for id in [
        Discovery::Ticket,
        Discovery::SecondAgent,
        Discovery::Account,
        Discovery::Mcp,
    ] {
        assert_eq!(item(&v, id).status, DiscoverStatus::Done, "{id:?}");
    }
    assert_eq!(v.done, 4);
    // The tickets deleted, the MCP server off since: what was done stays done.
    f.tickets = 0;
    f.mcp_on = false;
    let v = view(&saved, &f, "2.3.0", true);
    assert_eq!(item(&v, Discovery::Ticket).status, DiscoverStatus::Done);
    assert_eq!(item(&v, Discovery::Mcp).status, DiscoverStatus::Done);
    assert!(!settle(&mut saved, &f), "nothing new");
}

#[test]
fn a_moment_of_the_core_ticks_its_discovery_but_a_click_on_its_card_never_does() {
    assert_eq!(of_event(&Event::TerminalOpened), Some(Discovery::Terminal));
    assert_eq!(
        of_event(&Event::DictationUsed {
            target: "composer",
            model: "base",
            online_model: None
        }),
        Some(Discovery::Dictation)
    );
    assert_eq!(
        of_event(&Event::IntegrationConnected { service: "github" }),
        Some(Discovery::Forge)
    );
    // On by default, the autopilot is discovered once it started a ticket by itself; a ticket started by hand is not it.
    assert_eq!(
        of_event(&Event::TicketStarted { autopilot: true }),
        Some(Discovery::Autopilot)
    );
    assert_eq!(of_event(&Event::TicketStarted { autopilot: false }), None);
    // A ticket source is not a code host; a plugin of the seed is not one of the store.
    assert_eq!(
        of_event(&Event::IntegrationConnected { service: "jira" }),
        None
    );
    assert_eq!(
        of_event(&Event::PluginInstalled { from_store: false }),
        None
    );
    // What the window sees alone: the tab Docs, the editor, a PR. Nothing else is told by it.
    let mut saved = Onboarding::default();
    let f = set_up();
    assert!(apply(
        &mut saved,
        &Act::Seen {
            id: Discovery::Docs
        },
        &f,
        "2.3.0"
    ));
    assert!(!apply(
        &mut saved,
        &Act::Seen {
            id: Discovery::Terminal
        },
        &f,
        "2.3.0"
    ));
    let v = view(&saved, &f, "2.3.0", true);
    assert_eq!(item(&v, Discovery::Docs).status, DiscoverStatus::Done);
    assert_eq!(item(&v, Discovery::Terminal).status, DiscoverStatus::Todo);
}

#[test]
fn dismissed_it_is_never_offered_again_until_put_back() {
    let mut saved = Onboarding::default();
    let f = set_up();
    apply(
        &mut saved,
        &Act::Dismiss {
            id: Discovery::Voice,
        },
        &f,
        "2.3.0",
    );
    let v = view(&saved, &f, "2.3.0", true);
    assert_eq!(item(&v, Discovery::Voice).status, DiscoverStatus::Dismissed);
    assert!(!v.tips.contains(&Discovery::Voice));
    // Still on the map, counted among what this device has.
    assert_eq!(v.total, 18);
    apply(
        &mut saved,
        &Act::Restore {
            id: Discovery::Voice,
        },
        &f,
        "2.3.0",
    );
    let v = view(&saved, &f, "2.3.0", true);
    assert_eq!(item(&v, Discovery::Voice).status, DiscoverStatus::Todo);
    // A discovery done is never dismissed.
    let f = Facts {
        tickets: 1,
        ..set_up()
    };
    apply(
        &mut saved,
        &Act::Dismiss {
            id: Discovery::Ticket,
        },
        &f,
        "2.3.0",
    );
    assert_eq!(
        item(&view(&saved, &f, "2.3.0", true), Discovery::Ticket).status,
        DiscoverStatus::Done
    );
}

#[test]
fn a_bubble_is_shown_once_and_hide_all_stops_bubbles_and_the_dot() {
    let mut saved = Onboarding::default();
    let f = set_up();
    assert!(view(&saved, &f, "2.3.0", true)
        .tips
        .contains(&Discovery::Dictation));
    apply(
        &mut saved,
        &Act::Tipped {
            id: Discovery::Dictation,
        },
        &f,
        "2.3.0",
    );
    let v = view(&saved, &f, "2.3.0", true);
    assert!(!v.tips.contains(&Discovery::Dictation));
    // Not dismissed for that: still to try on the map.
    assert_eq!(item(&v, Discovery::Dictation).status, DiscoverStatus::Todo);
    apply(&mut saved, &Act::Hide { hidden: true }, &f, "2.3.0");
    let v = view(&saved, &f, "2.3.0", true);
    assert!(v.hidden);
    assert!(!v.dot);
    assert!(v.tips.is_empty());
    // The map is still there, every discovery on it.
    assert_eq!(v.items.len(), 19);
    apply(&mut saved, &Act::Hide { hidden: false }, &f, "2.3.0");
    assert!(view(&saved, &f, "2.3.0", true).dot);
}

#[test]
fn what_needs_a_first_gesture_says_it() {
    let saved = Onboarding::default();
    let f = set_up();
    let v = view(&saved, &f, "2.3.0", true);
    assert_eq!(
        item(&v, Discovery::Autopilot).after,
        Some(Discovery::Ticket)
    );
    assert_eq!(item(&v, Discovery::PrView).after, Some(Discovery::Forge));
    assert_eq!(item(&v, Discovery::OpenPr).after, Some(Discovery::Forge));
    assert_eq!(item(&v, Discovery::PrReview).after, Some(Discovery::PrView));
    let f = Facts {
        tickets: 1,
        forge_connected: true,
        ..set_up()
    };
    let v = view(&saved, &f, "2.3.0", true);
    assert_eq!(item(&v, Discovery::Autopilot).after, None);
    assert_eq!(item(&v, Discovery::PrView).after, None);
}

#[test]
fn a_discovery_shows_only_where_it_can_be_done() {
    let saved = Onboarding::default();
    // No microphone: neither the dictation nor the voice.
    let f = Facts {
        microphones: Some(0),
        ..set_up()
    };
    let v = view(&saved, &f, "2.3.0", true);
    for id in [Discovery::Dictation, Discovery::Voice] {
        assert_eq!(item(&v, id).status, DiscoverStatus::Absent);
        assert_eq!(item(&v, id).absent, Some(Absent::NoMicrophone));
        assert!(!v.tips.contains(&id), "never offered");
    }
    // Not counted among what this device has.
    assert_eq!(v.total, 16);
    // Unknown (a client without the voice): not hidden for that.
    let v = view(
        &saved,
        &Facts {
            microphones: None,
            ..set_up()
        },
        "2.3.0",
        true,
    );
    assert_eq!(item(&v, Discovery::Dictation).status, DiscoverStatus::Todo);
    // A team server known: the team is.
    let v = view(
        &saved,
        &Facts {
            team_server_known: true,
            ..set_up()
        },
        "2.3.0",
        true,
    );
    assert_eq!(item(&v, Discovery::Team).status, DiscoverStatus::Todo);
    assert_eq!(item(&v, Discovery::Team).absent, None);
    // What a policy denies, with its words.
    let f = Facts {
        denied: vec![(Discovery::Mcp, "Imposé par la stratégie".into())],
        ..set_up()
    };
    let v = view(&saved, &f, "2.3.0", true);
    assert_eq!(
        item(&v, Discovery::Mcp).absent,
        Some(Absent::Policy {
            why: "Imposé par la stratégie".into()
        })
    );
    // Done stays done even when a policy comes after.
    let mut done = Onboarding::default();
    done.discover.insert(Discovery::Mcp, DiscoverOutcome::Done);
    assert_eq!(
        item(&view(&done, &f, "2.3.0", true), Discovery::Mcp).status,
        DiscoverStatus::Done
    );
}

#[test]
fn the_band_of_a_release_shows_on_a_device_set_up_before_it_once() {
    let mut saved = Onboarding::default();
    let f = set_up();
    assert_eq!(release_of("2.3.0-beta.4"), "2.3");
    assert_eq!(band(&saved, &f, "2.3.1", true), Some("2.3".into()));
    // Never for another release, a client of the remote API, a new installation or a device with no project.
    assert_eq!(band(&saved, &f, "2.2.1", true), None);
    assert_eq!(band(&saved, &f, "2.3.0", false), None);
    assert_eq!(
        band(
            &saved,
            &Facts {
                fresh: true,
                ..set_up()
            },
            "2.3.0",
            true
        ),
        None
    );
    assert_eq!(
        band(
            &saved,
            &Facts {
                projects: 0,
                ..set_up()
            },
            "2.3.0",
            true
        ),
        None
    );
    // Closed: never again for this release, even after a minor version.
    apply(&mut saved, &Act::BandSeen, &f, "2.3.0");
    assert_eq!(band(&saved, &f, "2.3.2", true), None);
    // What it tells is new in it.
    let v = view(&Onboarding::default(), &f, "2.3.0", true);
    assert!(item(&v, Discovery::PrView).new);
    assert!(!item(&v, Discovery::Ticket).new);
}

/// The core of a data folder, as the app starts on it.
fn load(data: &DataDir) -> std::sync::Arc<Core> {
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data.clone());
    std::mem::forget(app);
    core
}

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

#[test]
fn the_core_reads_the_policies_and_keeps_what_was_done_and_dismissed_across_a_restart() {
    let dir = test_dir("discover-core");
    let data = DataDir::new(dir.join("data"));
    let core = load(&data);
    // A moment of the core: the terminal opened.
    core.track(Event::TerminalOpened);
    core.discover_act(
        Act::Dismiss {
            id: Discovery::Voice,
        },
        Some(1),
    );
    core.discover_act(Act::Hide { hidden: true }, Some(1));
    let v = core.discover_view(Some(1));
    assert_eq!(item(&v, Discovery::Terminal).status, DiscoverStatus::Done);
    // A new installation keeps its own release as seen: the band is never for it.
    assert!(v.band.is_none());
    core.save_now();
    drop(core);
    let core = load(&data);
    let v = core.discover_view(Some(1));
    assert_eq!(item(&v, Discovery::Terminal).status, DiscoverStatus::Done);
    assert_eq!(item(&v, Discovery::Voice).status, DiscoverStatus::Dismissed);
    assert!(v.hidden);
    assert!(v.tips.is_empty());
    // The policies of the device: the dictation cut, the MCP server denied, the plugins of the store refused, a team
    // server enrolled.
    *core.locks.write() = policy(&[
        (DICTATION, json!(false)),
        (MCP_EXTERNAL, json!("denied")),
        (PLUGIN_ORIGINS_KEY, json!(["folder"])),
        (TEAM_SERVER, json!("https://equipe.exemple.dev")),
    ]);
    let v = core.discover_view(Some(1));
    for id in [
        Discovery::Dictation,
        Discovery::Voice,
        Discovery::Mcp,
        Discovery::Plugin,
    ] {
        assert!(
            matches!(item(&v, id).absent, Some(Absent::Policy { .. })),
            "{id:?}: {:?}",
            item(&v, id)
        );
    }
    assert_eq!(item(&v, Discovery::Team).status, DiscoverStatus::Todo);
}

#[test]
fn a_source_of_tickets_passed_at_the_first_launch_waits_on_the_map_until_one_is_connected() {
    // ESC-512: the card « Une source de tickets » passed comes back here, in Planifier, to try.
    let saved = Onboarding::default();
    let v = view(&saved, &set_up(), "2.3.0", true);
    let it = item(&v, Discovery::Sources);
    assert_eq!(it.lane, Lane::Plan);
    assert_eq!(it.status, DiscoverStatus::Todo);
    // An account of a source connected, wherever it was: done.
    let f = Facts {
        sources_connected: true,
        ..set_up()
    };
    assert_eq!(
        item(&view(&saved, &f, "2.3.0", true), Discovery::Sources).status,
        DiscoverStatus::Done
    );
}
