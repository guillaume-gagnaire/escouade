use super::*;
use crate::paths::{test_dir, DataDir};
use tauri::test::mock_app;

/// A new installation's first start: nothing set up.
fn fresh() -> Facts {
    Facts {
        fresh: true,
        ..Default::default()
    }
}

fn status_of(v: &OnboardingView, step: OnboardingStep) -> StepStatus {
    v.steps.iter().find(|s| s.id == step).unwrap().status
}

#[test]
fn a_new_installation_opens_the_journey_at_its_first_station() {
    let saved = Onboarding::default();
    let v = view(&saved, &fresh(), true);
    assert!(v.open);
    assert_eq!(v.trigger, Some(Trigger::First));
    assert_eq!(v.resume_at, Some(OnboardingStep::Harness));
    assert!(v.steps.iter().all(|s| s.status == StepStatus::Todo));
    // Never for a client of the remote API.
    assert!(!view(&saved, &fresh(), false).open);
}

#[test]
fn a_device_already_set_up_never_has_it_imposed() {
    // An installation that ran before the journey existed (its settings had a mode), or that has a project.
    let saved = Onboarding::default();
    let old = Facts {
        fresh: false,
        accounts: 1,
        mode_chosen: true,
        projects: 2,
        first_reply: true,
        telemetry_answered: true,
        forge_connected: true,
        ..Default::default()
    };
    assert!(!view(&saved, &old, true).open);
    let with_project = Facts {
        projects: 1,
        ..fresh()
    };
    assert!(!view(&saved, &with_project, true).open);
    // Unless the user opens it again: every station shown, from the first, with what is set in place.
    let mut again = Onboarding::default();
    apply(&mut again, &Act::Redo, &old, 10);
    let v = view(&again, &old, true);
    assert!(v.open);
    assert_eq!(v.trigger, Some(Trigger::Redo));
    assert_eq!(v.resume_at, Some(OnboardingStep::Harness));
    assert!(v.steps.iter().all(|s| s.status == StepStatus::Done));
}

#[test]
fn a_station_set_elsewhere_ticks_itself_and_stays_done() {
    let mut saved = Onboarding {
        started: Some(1),
        ..Default::default()
    };
    // An account added in Réglages › Comptes et harness: the harness's and the account's stations are done.
    let facts = Facts {
        accounts: 1,
        ..fresh()
    };
    let v = view(&saved, &facts, true);
    assert_eq!(status_of(&v, OnboardingStep::Harness), StepStatus::Done);
    assert_eq!(status_of(&v, OnboardingStep::Account), StepStatus::Done);
    assert_eq!(v.resume_at, Some(OnboardingStep::Defaults));
    // Kept: the account removed later does not open them again (§ 5.2).
    assert!(settle(&mut saved, &facts));
    let removed = fresh();
    let v = view(&saved, &removed, true);
    assert_eq!(status_of(&v, OnboardingStep::Account), StepStatus::Done);
    assert_eq!(v.resume_at, Some(OnboardingStep::Defaults));
}

#[test]
fn left_half_way_it_opens_again_at_the_first_station_neither_done_nor_passed() {
    let facts = Facts {
        accounts: 1,
        ..fresh()
    };
    let mut saved = Onboarding::default();
    apply(
        &mut saved,
        &Act::Step {
            step: OnboardingStep::Defaults,
            outcome: StepOutcome::Skipped,
        },
        &facts,
        5,
    );
    // Kept in the state, read back as it was written.
    let back: Onboarding = serde_json::from_value(serde_json::to_value(&saved).unwrap()).unwrap();
    assert_eq!(back, saved);
    let v = view(&back, &facts, true);
    assert!(v.open);
    assert_eq!(v.trigger, Some(Trigger::Resume));
    assert_eq!(status_of(&v, OnboardingStep::Defaults), StepStatus::Skipped);
    assert_eq!(v.resume_at, Some(OnboardingStep::Hookups));
}

fn card(v: &OnboardingView, h: Hookup) -> StepStatus {
    v.hookups.iter().find(|c| c.id == h).unwrap().status
}

#[test]
fn what_to_plug_in_comes_after_the_starting_settings_and_before_the_project_nothing_ticked() {
    let at = |s| STEPS.iter().position(|x| *x == s).unwrap();
    assert_eq!(
        at(OnboardingStep::Hookups),
        at(OnboardingStep::Defaults) + 1
    );
    assert_eq!(at(OnboardingStep::Project), at(OnboardingStep::Hookups) + 1);
    // A new installation: the three cards, none ticked, none done.
    let v = view(&Onboarding::default(), &fresh(), true);
    assert_eq!(
        v.hookups.iter().map(|c| c.id).collect::<Vec<_>>(),
        [Hookup::Sources, Hookup::Voice, Hookup::Forge]
    );
    assert!(v.hookups.iter().all(|c| c.status == StepStatus::Todo));
    assert_eq!(status_of(&v, OnboardingStep::Hookups), StepStatus::Todo);
    // Kept in the state as it was written, read back by a version that has the cards.
    let old: Onboarding =
        serde_json::from_value(serde_json::json!({ "version": 1, "steps": { "harness": "done" } }))
            .unwrap();
    assert!(old.hookups.is_empty());
}

#[test]
fn a_card_passed_waits_in_the_discovery_one_plugged_in_stays_and_the_station_passed_passes_them_all(
) {
    let facts = Facts {
        accounts: 1,
        mode_chosen: true,
        ..fresh()
    };
    let mut saved = Onboarding::default();
    // The voice plugged in at the station, the sources passed by their « Plus tard ».
    let card_act = |h, outcome| Act::Hookup { hookup: h, outcome };
    apply(
        &mut saved,
        &card_act(Hookup::Voice, StepOutcome::Done),
        &facts,
        1,
    );
    apply(
        &mut saved,
        &card_act(Hookup::Sources, StepOutcome::Skipped),
        &facts,
        2,
    );
    let v = view(&saved, &facts, true);
    assert_eq!(card(&v, Hookup::Voice), StepStatus::Done);
    assert_eq!(card(&v, Hookup::Sources), StepStatus::Skipped);
    assert_eq!(card(&v, Hookup::Forge), StepStatus::Todo);
    // « Plus tard » on a card plugged in never undoes it.
    apply(
        &mut saved,
        &card_act(Hookup::Voice, StepOutcome::Skipped),
        &facts,
        3,
    );
    assert_eq!(
        card(&view(&saved, &facts, true), Hookup::Voice),
        StepStatus::Done
    );
    // The station passed whole: the card left untouched is passed with it.
    apply(
        &mut saved,
        &Act::Step {
            step: OnboardingStep::Hookups,
            outcome: StepOutcome::Skipped,
        },
        &facts,
        4,
    );
    let v = view(&saved, &facts, true);
    assert_eq!(card(&v, Hookup::Forge), StepStatus::Skipped);
    assert_eq!(card(&v, Hookup::Voice), StepStatus::Done);
    // « Tout passer » on a new installation: every card passed, to find again in the discovery.
    let mut all = Onboarding::default();
    apply(&mut all, &Act::SkipAll, &fresh(), 9);
    let v = view(&all, &fresh(), true);
    assert!(v.hookups.iter().all(|c| c.status == StepStatus::Skipped));
}

#[test]
fn what_is_plugged_in_elsewhere_ticks_its_card_and_the_station_and_a_policy_locks_the_voice() {
    let facts = Facts {
        forge_connected: true,
        ..fresh()
    };
    let v = view(&Onboarding::default(), &facts, true);
    assert_eq!(card(&v, Hookup::Forge), StepStatus::Done);
    assert_eq!(card(&v, Hookup::Sources), StepStatus::Todo);
    assert_eq!(status_of(&v, OnboardingStep::Hookups), StepStatus::Done);
    let off = Facts {
        dictation_off: true,
        ..fresh()
    };
    assert_eq!(
        card(&view(&Onboarding::default(), &off, true), Hookup::Voice),
        StepStatus::Policy
    );
}

fn offer(kind: Hookup, id: &str, installed: bool, seed: bool, denied: Option<&str>) -> Offer {
    Offer {
        kind,
        id: id.into(),
        name: id.into(),
        plugin: Some(format!("dev.escouade.{id}")),
        installed,
        seed,
        denied: denied.map(Into::into),
    }
}

#[test]
fn the_station_installs_the_plugin_of_what_is_chosen_and_nothing_else() {
    let offers = [
        offer(Hookup::Sources, "trello", false, true, None),
        offer(Hookup::Sources, "jira", false, true, None),
        offer(Hookup::Sources, "linear", true, true, None),
        offer(Hookup::Sources, "clickup", false, false, None),
        offer(Hookup::Voice, "openai", false, true, Some("imposé")),
        offer(Hookup::Forge, "gitlab", false, true, None),
    ];
    // Trello chosen: Trello's plugin, from the seed; not Jira's, not GitLab's.
    assert_eq!(
        to_install(&offers, Hookup::Sources, "trello"),
        Ok(Some("dev.escouade.trello".into()))
    );
    // Installed already, or not in the seed (the store's, from Plugins): nothing.
    assert_eq!(to_install(&offers, Hookup::Sources, "linear"), Ok(None));
    assert_eq!(to_install(&offers, Hookup::Sources, "clickup"), Ok(None));
    // Denied by a policy: refused, with why.
    assert_eq!(
        to_install(&offers, Hookup::Voice, "openai"),
        Err("imposé".into())
    );
    // Not offered by that card: refused.
    assert!(to_install(&offers, Hookup::Forge, "trello").is_err());
    assert!(to_install(&offers, Hookup::Sources, "evil").is_err());
}

#[test]
fn passed_it_never_comes_back_by_itself_and_the_statistics_are_asked_once_more() {
    let facts = fresh();
    let mut saved = Onboarding::default();
    apply(&mut saved, &Act::SkipAll, &facts, 7);
    assert_eq!(saved.finished, Some(7));
    assert!(saved.steps.values().all(|o| *o == StepOutcome::Skipped));
    let v = view(&saved, &facts, true);
    assert!(!v.open);
    // The statistics' question passed: asked once more at a later start, then never again.
    assert!(v.ask_telemetry_again);
    apply(&mut saved, &Act::TelemetryAskedAgain, &facts, 8);
    assert!(!view(&saved, &facts, true).ask_telemetry_again);
}

#[test]
fn later_never_undoes_a_station_done() {
    let facts = Facts {
        mode_chosen: true,
        ..fresh()
    };
    let mut saved = Onboarding::default();
    apply(
        &mut saved,
        &Act::Step {
            step: OnboardingStep::Defaults,
            outcome: StepOutcome::Skipped,
        },
        &facts,
        1,
    );
    assert_eq!(
        saved.steps.get(&OnboardingStep::Defaults),
        Some(&StepOutcome::Done)
    );
}

#[test]
fn what_a_policy_imposes_is_shown_locked() {
    let facts = Facts {
        telemetry_imposed: true,
        mode_imposed: true,
        // The core brings the settings within the policy: the statistics are answered for the user.
        telemetry_answered: true,
        mode_chosen: true,
        ..fresh()
    };
    let v = view(&Onboarding::default(), &facts, true);
    assert_eq!(status_of(&v, OnboardingStep::Defaults), StepStatus::Policy);
    assert_eq!(status_of(&v, OnboardingStep::Telemetry), StepStatus::Policy);
    assert!(!v.ask_telemetry_again);
}

/// The core of a data folder, as the app starts on it.
fn load(data: &DataDir) -> std::sync::Arc<Core> {
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data.clone());
    std::mem::forget(app);
    core
}

#[test]
fn a_first_start_in_an_empty_data_folder_opens_it_keeps_it_and_resumes_it() {
    let dir = test_dir("onboarding-first");
    let data = DataDir::new(dir.join("data"));
    let core = load(&data);
    let v = core.onboarding_view();
    assert!(v.open);
    assert_eq!(v.trigger, Some(Trigger::First));
    // A station passed, then the app closed half-way: kept in state.json.
    core.onboarding_act(Act::Step {
        step: OnboardingStep::Harness,
        outcome: StepOutcome::Skipped,
    });
    core.save_now();
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.state_file()).unwrap()).unwrap();
    assert_eq!(state["onboarding"]["version"], serde_json::json!(VERSION));
    assert_eq!(
        state["onboarding"]["steps"]["harness"],
        serde_json::json!("skipped")
    );
    drop(core);
    // The next start opens it again, at the first station neither done nor passed.
    let core = load(&data);
    let v = core.onboarding_view();
    assert!(v.open);
    assert_eq!(v.trigger, Some(Trigger::Resume));
    assert_eq!(v.resume_at, Some(OnboardingStep::Account));
    // An account added elsewhere (Réglages): its station ticks itself.
    core.settings
        .write()
        .accounts
        .push(crate::accounts::principal());
    let v = core.onboarding_view();
    assert_eq!(status_of(&v, OnboardingStep::Account), StepStatus::Done);
    assert_eq!(v.resume_at, Some(OnboardingStep::Defaults));
    // Ended: not opened again.
    assert!(!core.onboarding_act(Act::Finish).open);
    core.save_now();
    drop(core);
    assert!(!load(&data).onboarding_view().open);
}

#[test]
fn an_installation_set_up_before_never_opens_it_unless_asked() {
    let dir = test_dir("onboarding-set-up");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    // Settings of before the journey: a mode is given them (SEC-09), the device is set up.
    std::fs::write(
        data.settings_file(),
        serde_json::json!({ "claudePath": crate::core_tests::fake_cli(), "sound": false })
            .to_string(),
    )
    .unwrap();
    let core = load(&data);
    assert!(!core.onboarding_view().open);
    let v = core.onboarding_act(Act::Redo);
    assert!(v.open);
    assert_eq!(v.trigger, Some(Trigger::Redo));
    assert_eq!(status_of(&v, OnboardingStep::Defaults), StepStatus::Done);
}

#[test]
fn the_offers_are_read_in_the_registry_the_seed_and_the_policies() {
    let dir = test_dir("onboarding-offers");
    let data = DataDir::new(dir.join("data"));
    let core = load(&data);
    let offers = core.onboarding_offers();
    let of = |k: Hookup| {
        offers
            .iter()
            .filter(|o| o.kind == k)
            .map(|o| o.id.as_str())
            .collect::<Vec<_>>()
    };
    // The sources as the registry declares them, in its order; each with its plugin.
    let sources: Vec<String> = crate::integrations::source::declared()
        .into_iter()
        .map(|d| d.id)
        .collect();
    assert_eq!(
        of(Hookup::Sources),
        sources.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(of(Hookup::Voice), crate::plugins::sort::VOICES.to_vec());
    assert_eq!(
        of(Hookup::Forge),
        crate::plugins::sort::FORGES
            .iter()
            .map(|(k, _)| *k)
            .collect::<Vec<_>>()
    );
    assert!(offers
        .iter()
        .all(|o| o.plugin.is_some() && o.denied.is_none()));
    // A policy that turns the voice on line off: every voice service said denied, by whom.
    let managed = crate::locks::Managed {
        values: [
            (VOICE_ONLINE, serde_json::json!(false)),
            (
                escouade_policy::list::PLUGINS_DENIED,
                serde_json::json!(["dev.escouade.trello"]),
            ),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), (v, "HKLM".to_owned())))
        .collect(),
        warnings: Vec::new(),
    };
    *core.locks.write() =
        crate::locks::Locks::compute(escouade_policy::list::POLICIES, None, &managed);
    let offers = core.onboarding_offers();
    let denied = |k: Hookup, id: &str| {
        offers
            .iter()
            .find(|o| o.kind == k && o.id == id)
            .unwrap()
            .denied
            .clone()
    };
    assert!(offers
        .iter()
        .filter(|o| o.kind == Hookup::Voice)
        .all(|o| o.denied.is_some()));
    assert!(denied(Hookup::Sources, "trello")
        .unwrap()
        .contains("dev.escouade.trello"));
    assert_eq!(denied(Hookup::Sources, "jira"), None);
    // The core refuses what a policy denies, whatever the window shows.
    assert!(core.onboarding_need(Hookup::Sources, "trello").is_err());
    assert!(core.onboarding_need(Hookup::Voice, "openai").is_err());
}

// === ON-2 (ESC-505): the first project and the first agent ===

#[test]
fn the_first_agent_is_a_station_between_the_project_and_the_statistics_done_by_a_reply() {
    let at = |s| STEPS.iter().position(|x| *x == s).unwrap();
    assert_eq!(at(OnboardingStep::Agent), at(OnboardingStep::Project) + 1);
    assert_eq!(at(OnboardingStep::Telemetry), at(OnboardingStep::Agent) + 1);
    // A project open, no reply yet: the journey goes on at the first agent.
    let facts = Facts {
        accounts: 1,
        mode_chosen: true,
        projects: 1,
        ..fresh()
    };
    // « Ce que tu veux brancher » (ESC-512) passed: nothing plugged in.
    let saved = Onboarding {
        started: Some(1),
        steps: [(OnboardingStep::Hookups, StepOutcome::Skipped)].into(),
        ..Default::default()
    };
    let v = view(&saved, &facts, true);
    assert_eq!(status_of(&v, OnboardingStep::Agent), StepStatus::Todo);
    assert_eq!(v.resume_at, Some(OnboardingStep::Agent));
    // Its first turn ended well: done, wherever it was sent from.
    let replied = Facts {
        first_reply: true,
        ..facts
    };
    let v = view(&saved, &replied, true);
    assert_eq!(status_of(&v, OnboardingStep::Agent), StepStatus::Done);
    assert_eq!(v.resume_at, Some(OnboardingStep::Telemetry));
}

#[test]
fn a_folder_is_read_for_its_tests_and_whether_escouade_can_write_in_it() {
    let dir = test_dir("onboarding-folder");
    std::fs::create_dir_all(&dir).unwrap();
    let before: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    let c = check_folder(&dir, true);
    assert_eq!(
        c,
        FolderCheck {
            exists: true,
            writable: true,
            tests: false
        }
    );
    // The probe of the writing leaves nothing behind.
    let after: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(before, after);
    // A package.json's tests need Node; npm's placeholder is none.
    std::fs::write(
        dir.join("package.json"),
        r#"{"scripts":{"test":"echo \"Error: no test specified\" && exit 1"}}"#,
    )
    .unwrap();
    assert!(!check_folder(&dir, true).tests);
    std::fs::write(dir.join("package.json"), r#"{"scripts":{"test":"vitest"}}"#).unwrap();
    assert!(check_folder(&dir, true).tests);
    assert!(!check_folder(&dir, false).tests);
    // A build of another language: its tests whatever Node says.
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    assert!(check_folder(&dir, false).tests);
    // A folder that is not there.
    let gone = check_folder(&dir.join("absent"), true);
    assert!(!gone.exists && !gone.writable && !gone.tests);
}

#[test]
fn the_address_of_a_clone_is_checked_and_its_refusals_say_what_to_do() {
    assert!(clone_address("https://github.com/acme/api.git").is_ok());
    assert!(clone_address("git@github.com:acme/api.git").is_ok());
    assert!(clone_address("")
        .unwrap_err()
        .contains("Donne l’adresse du dépôt"));
    assert!(clone_address("C:/dev/api")
        .unwrap_err()
        .contains("Un dossier de ta machine"));
    assert!(clone_address("https://ada:secret@github.com/acme/api.git")
        .unwrap_err()
        .contains("Pas de mot de passe"));
    assert!(clone_address("ftp://example.com/acme/api.git")
        .unwrap_err()
        .contains("pas en ftp"));
    // git's words for a host it cannot reach: offline, not refused.
    assert!(offline_in(
        "fatal: unable to access 'https://github.com/acme/api.git/': Could not resolve host: github.com"
    ));
    assert!(!offline_in("remote: Repository not found."));
}

#[tokio::test]
async fn a_clone_goes_into_a_new_folder_and_one_that_fails_says_why_and_leaves_nothing() {
    let dir = test_dir("onboarding-clone");
    std::fs::create_dir_all(&dir).unwrap();
    // A folder there already, not empty: refused, left as it is.
    std::fs::create_dir_all(dir.join("api")).unwrap();
    std::fs::write(dir.join("api").join("mien.txt"), "à moi").unwrap();
    let c = clone("https://example.invalid/acme/api.git", &dir, "api").await;
    assert_eq!(c.path, None);
    assert_eq!(c.failure.as_ref().unwrap().kind, "place");
    assert!(dir.join("api").join("mien.txt").is_file());
    // A bad name of folder.
    let c = clone("https://example.invalid/acme/api.git", &dir, "../x").await;
    assert_eq!(c.failure.unwrap().kind, "place");
    // A bad address: said before git runs.
    let c = clone("ftp://example.invalid/acme/api.git", &dir, "api2").await;
    assert_eq!(c.failure.unwrap().kind, "address");
    // A host git cannot reach: it fails, says why, and leaves no folder half made.
    let c = clone("https://127.0.0.1:9/acme/api.git", &dir, "api3").await;
    let f = c.failure.unwrap();
    assert!(
        ["offline", "error", "access"].contains(&f.kind.as_str()),
        "{f:?}"
    );
    assert!(!f.detail.is_empty());
    assert!(!dir.join("api3").exists());
}

#[tokio::test]
async fn the_demo_project_is_made_once_known_by_its_view_and_removed_whole() {
    let dir = test_dir("onboarding-demo");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    std::fs::write(
        data.settings_file(),
        serde_json::json!({ "claudePath": crate::core_tests::fake_cli(), "sound": false })
            .to_string(),
    )
    .unwrap();
    let core = load(&data);
    let at = crate::demo::dir(data.root());
    assert!(crate::demo::same(
        Path::new(&core.onboarding_view().demo),
        &at
    ));
    let p = core.demo_create("teal").await.unwrap();
    assert!(crate::demo::same(Path::new(&p.path), &at));
    assert_eq!(p.name, crate::demo::FOLDER);
    assert!(p.worktree_per_agent);
    // Asked again: the same project, not a second one.
    let again = core.demo_create("teal").await.unwrap();
    assert_eq!(again.id, p.id);
    assert_eq!(core.projects.read().len(), 1);
    // Another project is never removed by it.
    let other = dir.join("autre");
    std::fs::create_dir_all(&other).unwrap();
    let o = core
        .create_project(
            &other.to_string_lossy(),
            "autre",
            "blue",
            false,
            None,
            false,
        )
        .await
        .unwrap();
    assert!(core.demo_remove(&o.id).await.is_err());
    assert!(other.is_dir());
    // « Supprimer »: its project and its folder gone, the other project there still.
    core.demo_remove(&p.id).await.unwrap();
    assert!(!at.exists());
    let left: Vec<String> = core.projects.read().iter().map(|p| p.id.clone()).collect();
    assert_eq!(left, vec![o.id.clone()]);
    assert!(other.is_dir());
}
