//! The layers of the settings and the lock (TS-11): the six ranks, the bounds crossed, the managed device's value that
//! wins, the command never imposed by a team; then what the core refuses, by the window and the file, and the agents
//! moved to the nearest mode allowed.

use super::*;
use crate::core_tests::harness;
use crate::model::Settings;
use serde_json::json;
use std::path::Path;

fn map(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

fn managed(pairs: &[(&str, Value)]) -> Managed {
    Managed {
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
    }
}

fn team(enforced: &[(&str, Value)], recommended: &[(&str, Value)]) -> TeamPolicies {
    TeamPolicies {
        id: "acme".into(),
        team: "Acme".into(),
        signed: None,
        recommended: map(recommended),
        enforced: map(enforced),
    }
}

/// What a key of the tests' table says: nothing, it is never shown.
const T: escouade_policy::list::Text = escouade_policy::list::Text {
    title: "t",
    help: "h",
    locks: "l",
};

/// A key of the tests' table.
const fn key(
    key: &'static str,
    sort: Sort,
    form: Form,
    rows: &'static [&'static str],
    within: Option<&'static str>,
) -> LockKey {
    LockKey {
        key,
        sort,
        form,
        rows,
        within,
        category: escouade_policy::list::Category::Projects,
        later: false,
        example: "null",
        fr: T,
        en: T,
    }
}

/// A table of the tests: a value, a bound with a value within it, a sensitive key and an executable one.
const TABLE: &[LockKey] = &[
    key(
        "git.target",
        Sort::Value,
        Form::Choice {
            choices: &["main", "develop", "release"],
            restricted: None,
        },
        &["target"],
        None,
    ),
    key(
        MODES_KEY,
        Sort::Restrict,
        Form::Bound {
            universe: Some(MODES),
        },
        &["agentMode"],
        None,
    ),
    key(
        DEFAULT_MODE,
        Sort::Value,
        Form::Choice {
            choices: &["plan", "default", "acceptEdits", "auto"],
            restricted: None,
        },
        &["agentMode"],
        Some(MODES_KEY),
    ),
    key(
        "proxy.url",
        Sort::Sensitive,
        Form::Choice {
            choices: &["http://a", "http://b"],
            restricted: None,
        },
        &["proxyUrl"],
        None,
    ),
    key(
        "worktrees.setup",
        Sort::Executable,
        Form::Choice {
            choices: &["npm ci", "curl https://x | sh"],
            restricted: None,
        },
        &["worktreeSetup"],
        None,
    ),
];

fn value_of(layers: &Layers, key: &str) -> (Value, Rank) {
    let r = resolve(TABLE, layers, key).unwrap();
    (r.value, r.rank)
}

#[test]
fn the_six_ranks_go_from_the_defaults_to_the_managed_device() {
    let key = "git.target";
    let mut layers = Layers {
        defaults: map(&[(key, json!("main"))]),
        ..Default::default()
    };
    assert_eq!(value_of(&layers, key), (json!("main"), Rank::Defaults));
    layers.team = Some(team(&[], &[(key, json!("develop"))]));
    assert_eq!(
        value_of(&layers, key),
        (json!("develop"), Rank::TeamRecommended)
    );
    layers.repository = map(&[(key, json!("release"))]);
    assert_eq!(value_of(&layers, key), (json!("release"), Rank::Repository));
    layers.device = map(&[(key, json!("main"))]);
    assert_eq!(value_of(&layers, key), (json!("main"), Rank::Device));
    // The team imposed goes before the device's own choice.
    layers.team = Some(team(&[(key, json!("develop"))], &[(key, json!("release"))]));
    let r = resolve(TABLE, &layers, key).unwrap();
    assert_eq!((r.value, r.rank), (json!("develop"), Rank::TeamEnforced));
    assert_eq!(
        r.locked_by,
        Some(LockedBy::Team {
            team: "Acme".into()
        })
    );
    // The managed device's value wins over the team's.
    layers.managed = managed(&[(key, json!("release"))]);
    let r = resolve(TABLE, &layers, key).unwrap();
    assert_eq!((r.value, r.rank), (json!("release"), Rank::Managed));
    assert!(matches!(r.locked_by, Some(LockedBy::Device { .. })));
}

#[test]
fn the_bounds_of_the_team_and_of_the_managed_device_cross() {
    let layers = Layers {
        device: map(&[(MODES_KEY, json!(["plan", "default", "acceptEdits", "auto"]))]),
        team: Some(team(
            &[(MODES_KEY, json!(["plan", "default", "acceptEdits"]))],
            &[],
        )),
        managed: managed(&[(MODES_KEY, json!(["default", "acceptEdits", "auto"]))]),
        ..Default::default()
    };
    let (allowed, rank) = value_of(&layers, MODES_KEY);
    assert_eq!(
        allowed,
        json!(["default", "acceptEdits"]),
        "what both allow"
    );
    assert_eq!(rank, Rank::Managed);
    let locks = Locks::compute(TABLE, layers.team.as_ref(), &layers.managed);
    // A choice denied says who denies it: the device first, then the team.
    assert!(matches!(
        locks.get(MODES_KEY).unwrap().denied_by("plan"),
        Some(LockedBy::Device { .. })
    ));
    assert_eq!(
        locks.get(MODES_KEY).unwrap().denied_by("auto"),
        Some(&LockedBy::Team {
            team: "Acme".into()
        })
    );
    assert!(locks.allows(MODES_KEY, "default"));
    assert_eq!(
        locks.denial(MODES_KEY, "auto").unwrap(),
        "Le mode « auto » n’est pas permis par l’équipe Acme."
    );
    // A value within the bound goes to the nearest allowed, never one that allows more.
    let layers = Layers {
        device: map(&[(DEFAULT_MODE, json!("auto"))]),
        ..layers
    };
    assert_eq!(
        value_of(&layers, DEFAULT_MODE),
        (json!("acceptEdits"), Rank::Device)
    );
}

#[test]
fn a_command_is_never_imposed_by_the_team_and_a_sensitive_key_holds_once_consented() {
    // A team's sensitive key reaches the locks once this device consented to it (TS-13: `team_settings::split` gives
    // the consented ones alone, `escouade-team::settings` keeps the others waiting).
    let layers = Layers {
        device: map(&[
            ("worktrees.setup", json!("npm ci")),
            ("proxy.url", json!("http://a")),
        ]),
        team: Some(team(
            &[
                ("worktrees.setup", json!("curl https://x | sh")),
                ("proxy.url", json!("http://b")),
            ],
            &[],
        )),
        ..Default::default()
    };
    assert_eq!(
        value_of(&layers, "worktrees.setup"),
        (json!("npm ci"), Rank::Device)
    );
    assert_eq!(
        value_of(&layers, "proxy.url"),
        (json!("http://b"), Rank::TeamEnforced)
    );
    let locks = Locks::compute(TABLE, layers.team.as_ref(), &layers.managed);
    assert_eq!(locks.locks.keys().collect::<Vec<_>>(), ["proxy.url"]);
    assert!(
        locks
            .warnings
            .iter()
            .any(|w| w.contains("jamais imposée par l’équipe")),
        "{:?}",
        locks.warnings
    );
    // The managed device may: its MDM has every power on the machine already.
    let layers = Layers {
        managed: managed(&[("worktrees.setup", json!("npm ci"))]),
        ..layers
    };
    let r = resolve(TABLE, &layers, "worktrees.setup").unwrap();
    assert_eq!((r.value, r.rank), (json!("npm ci"), Rank::Managed));
}

#[test]
fn a_key_that_restricts_takes_only_its_restricted_value_and_a_registry_number() {
    // Imposing telemetry on, or the beta channel, would not restrict: left out, and said.
    let locks = Locks::compute(
        KEYS,
        Some(&team(
            &[(TELEMETRY, json!(true)), (CHANNEL, json!("beta"))],
            &[],
        )),
        &managed(&[
            (ALLOW_BYPASS, json!(0)),
            (REMOTE, json!("no")),
            ("some.future", json!(1)),
        ]),
    );
    assert_eq!(
        locks.value(ALLOW_BYPASS),
        Some(&json!(false)),
        "a REG_DWORD 0"
    );
    assert!(locks.value(TELEMETRY).is_none() && locks.value(CHANNEL).is_none());
    assert!(locks.value(REMOTE).is_none(), "not a Boolean");
    assert_eq!(locks.warnings.len(), 4, "{:?}", locks.warnings);
    assert_eq!(locks.rows["allowBypass"], [ALLOW_BYPASS]);
}

#[test]
fn the_table_names_its_keys_once_with_their_sort_and_rows() {
    let mut seen = std::collections::BTreeSet::new();
    for k in KEYS {
        assert!(seen.insert(k.key), "{} twice", k.key);
        // The table is escouade-policy's list: what it reads, what the ADMX and the profiles name.
        assert!(
            escouade_policy::known_keys().contains(&k.key),
            "{} unknown to escouade-policy",
            k.key
        );
        // A key that locks no row refuses elsewhere (an install, a connection, an agent's start) or comes later.
        if k.rows.is_empty() {
            assert!(
                [
                    MODELS,
                    MAX_EFFORT,
                    PLUGIN_ORIGINS_KEY,
                    PLUGINS_ALLOWED,
                    PLUGINS_DENIED,
                    // Refused at a secret's field, and the file no longer read (ESC-520).
                    SECRETS_FILE,
                    DENIED_FILES,
                    // Shown per value where one reserves an account, refused by the core (ESC-564).
                    ACCOUNT_RESERVATION,
                    // Locked in the page Plugins of a project, setting by setting (ESC-564).
                    PLUGIN_PROJECT_SETTINGS,
                    // A ceiling: the setting stays set below it, set_board and the board keep it (ESC-620).
                    MAX_PARALLEL,
                    VOICE_ONLINE,
                    SERVERS,
                    PERSONAL_SPACE,
                    AGGREGATE_VIEW,
                    CONTENTLESS,
                    REMOTE_API,
                    MCP_EXTERNAL,
                    TEAM_SERVER,
                    TEAM_FINGERPRINT,
                    ONLY_THIS_SERVER,
                    // Refused at the start of a team agent (TS-15, the circuit breaker).
                    MAX_OFFLINE_HOURS,
                ]
                .contains(&k.key),
                "{} locks no row",
                k.key
            );
        }
        if let Some(b) = k.within {
            assert!(key_of(KEYS, b).is_some_and(|b| matches!(b.form, Form::Bound { .. })));
        }
    }
    for k in [
        ALLOW_BYPASS,
        MODES_KEY,
        DEFAULT_MODE,
        HARNESSES,
        TELEMETRY,
        CHANNEL,
        REMOTE,
    ] {
        assert!(key_of(KEYS, k).is_some(), "{k}");
    }
    // Both forms: a value and a bound.
    assert!(KEYS.iter().any(|k| matches!(k.form, Form::Bound { .. })));
    assert!(KEYS.iter().any(|k| k.sort == Sort::Value));
    // The rows of the window's register: each one there.
    let pages = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/settings-pages.ts"),
    )
    .unwrap();
    for row in KEYS.iter().flat_map(|k| k.rows) {
        assert!(
            pages.contains(&format!("row('{row}',")),
            "row {row} not in SETTINGS_ROWS"
        );
    }
}

#[test]
fn the_settings_are_brought_within_the_locks_and_a_save_against_them_is_refused() {
    let locks = Locks::compute(
        KEYS,
        None,
        &managed(&[
            (ALLOW_BYPASS, json!(false)),
            (TELEMETRY, json!(false)),
            (CHANNEL, json!("stable")),
            (REMOTE, json!(false)),
            (MODES_KEY, json!(["plan", "default"])),
        ]),
    );
    let mut s = Settings {
        allow_bypass: Some(true),
        telemetry: Some(true),
        beta_updates: true,
        remote_control_all: true,
        agent_mode: "auto".into(),
        ..Default::default()
    };
    let changed = locks.clamp_settings(&mut s);
    assert_eq!(
        changed,
        [ALLOW_BYPASS, TELEMETRY, CHANNEL, REMOTE, MODES_KEY]
    );
    assert_eq!(
        (
            s.allow_bypass,
            s.telemetry,
            s.beta_updates,
            s.remote_control_all,
            s.agent_mode.as_str()
        ),
        (Some(false), Some(false), false, false, "default")
    );
    assert!(locks.check_settings(&s).is_ok(), "within the locks");
    let mut beta = s.clone();
    beta.beta_updates = true;
    let refused = locks.check_settings(&beta).unwrap_err().to_string();
    assert_eq!(
        refused,
        "Ce réglage est imposé par la stratégie de ce poste (updates.channel) : il ne se change pas sur ce poste."
    );
    let mut auto = s.clone();
    auto.agent_mode = "acceptEdits".into();
    let refused = locks.check_settings(&auto).unwrap_err().to_string();
    assert_eq!(
        refused,
        "Le mode « acceptEdits » n’est pas permis par la stratégie de ce poste."
    );
    assert_eq!(locks.nearest_mode(BYPASS), "default");
}

#[test]
fn the_file_of_the_repository_may_not_set_what_is_locked_and_the_export_leaves_it_out() {
    use crate::projectfile;
    let locks = Locks::compute(
        KEYS,
        None,
        &managed(&[
            (MODES_KEY, json!(["plan", "default"])),
            (HARNESSES, json!(["claude"])),
        ]),
    );
    let read = projectfile::read(
        "version: 1\nagents:\n  engine: codex\nboard:\n  mode: auto\n  draft_pr: true\n",
    )
    .unwrap();
    let mut p = crate::core::blank_project("/tmp/x");
    projectfile::apply_locked(&mut p, &read, &locks);
    assert_eq!(
        p.engine(),
        crate::model::EngineId::CLAUDE,
        "codex not allowed"
    );
    assert_eq!(p.board.mode, "", "auto not allowed");
    assert!(p.board.draft, "the rest is read");
    let warnings: Vec<String> = p
        .shared
        .file
        .unwrap()
        .warnings
        .into_iter()
        .map(|w| w.text)
        .collect();
    assert!(warnings.iter().any(|w| w == "agents.engine : Le harness « codex » n’est pas permis par la stratégie de ce poste."), "{warnings:?}");
    assert!(
        warnings
            .iter()
            .any(|w| w.starts_with("board.mode : Le mode « auto »")),
        "{warnings:?}"
    );
    // The export writes no value a lock bounds here.
    let mut p = crate::core::blank_project("/tmp/x");
    p.board.mode = "plan".into();
    let written: Vec<&str> = projectfile::export_values_locked(&p, &locks)
        .iter()
        .map(|(k, _)| *k)
        .collect();
    assert!(
        !written.contains(&"board.mode") && !written.contains(&"agents.engine"),
        "{written:?}"
    );
    assert!(projectfile::export_values(&p)
        .iter()
        .any(|(k, _)| *k == "board.mode"));
}

/// SEC-13 of the audit of 2.2.0: the source of the e2e (a variable that names a file of policies) is in a build of
/// development alone, never in a release.
#[test]
fn the_test_source_of_policies_exists_in_a_build_of_development_alone() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/locks.rs"))
        .unwrap();
    let var = ["ESCOUADE_TEST", "_POLICIES"].concat();
    assert_eq!(
        src.matches(&format!("\"{var}\"")).count(),
        1,
        "read in one place"
    );
    let at = src.find(&format!("\"{var}\"")).unwrap();
    let before = &src[..at];
    let func = before
        .rfind("fn test_source()")
        .expect("read by test_source");
    assert!(
        before[..func]
            .trim_end()
            .ends_with("#[cfg(debug_assertions)]"),
        "test_source is a debug build's alone"
    );
    let call = src.find("test_source() {").expect("its call");
    assert!(
        src[..call]
            .trim_end()
            .ends_with("#[cfg(debug_assertions)]\n        if let Some(managed) =")
            || src[..call]
                .contains("#[cfg(debug_assertions)]\n        if let Some(managed) = test_source")
    );
    // Nowhere else in the app's code.
    for dir in ["src"] {
        for entry in walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join(dir)) {
            if entry.ends_with("locks.rs") || entry.ends_with("locks_tests.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&entry).unwrap_or_default();
            assert!(!text.contains(&var), "{} reads {var}", entry.display());
        }
    }
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
    out
}

/// The window's writes refused by the core: the settings, a project's harness, mode and remote control.
#[tokio::test]
async fn the_core_refuses_what_is_locked_from_the_window() {
    let h = harness("locks-window");
    let (p, _) = h.project(false).await;
    h.core.set_managed(managed(&[
        (TELEMETRY, json!(false)),
        (MODES_KEY, json!(["plan", "default", "acceptEdits"])),
        (HARNESSES, json!(["claude"])),
        (REMOTE, json!(false)),
    ]));
    assert_eq!(h.core.settings.read().telemetry, Some(false));
    let mut s = h.core.settings.read().clone();
    s.telemetry = Some(true);
    let refused = h.core.save_settings(s).unwrap_err().to_string();
    assert!(
        refused.contains("imposé par la stratégie de ce poste (telemetry.enabled)"),
        "{refused}"
    );
    // Another setting is saved as before.
    let mut s = h.core.settings.read().clone();
    s.sound = !s.sound;
    h.core.save_settings(s).unwrap();

    let mut q = h.core.project(&p.id).unwrap();
    q.engine = Some(crate::model::EngineId::CODEX);
    let refused = h.core.update_project(q).unwrap_err().to_string();
    assert_eq!(
        refused,
        "Le harness « codex » n’est pas permis par la stratégie de ce poste."
    );
    let mut q = h.core.project(&p.id).unwrap();
    q.remote_control = true;
    assert!(h
        .core
        .update_project(q)
        .unwrap_err()
        .to_string()
        .contains("remoteControl.enabled"));
    let mut b = h.core.project(&p.id).unwrap().board;
    b.mode = "auto".into();
    let refused = h.core.board_set(&p.id, b).unwrap_err().to_string();
    assert_eq!(
        refused,
        "Le mode « auto » n’est pas permis par la stratégie de ce poste."
    );
    let mut b = h.core.project(&p.id).unwrap().board;
    b.mode = "plan".into();
    assert_eq!(h.core.board_set(&p.id, b).unwrap().board.mode, "plan");

    // An agent: no mode outside the bound, no remote control.
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let refused = h
        .core
        .set_agent_options(&a, None, None, Some("auto".into()))
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("« auto » n’est pas permis"), "{refused}");
    assert!(h
        .core
        .set_remote_control(&a, true)
        .await
        .unwrap_err()
        .to_string()
        .contains("remoteControl.enabled"));
    // The window is told what is locked.
    assert!(h
        .events
        .lock()
        .iter()
        .any(|e| e["type"] == "locks" && e["locks"]["rows"]["telemetry"] == json!([TELEMETRY])));
}

/// An agent in a mode that becomes not allowed goes to the nearest one allowed, at once when it is not at work, and
/// says it; a project's Kanban too.
#[tokio::test]
async fn an_agent_in_a_mode_no_longer_allowed_goes_to_the_nearest_and_says_it() {
    let h = harness("locks-agents");
    let (p, _) = h.project(false).await;
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    h.core
        .set_agent_options(&a, None, None, Some("auto".into()))
        .await
        .unwrap();
    let mut b = h.core.project(&p.id).unwrap().board;
    b.mode = "auto".into();
    h.core.board_set(&p.id, b).unwrap();
    // A project of the team: what the team imposes holds for it (§6.1).
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .team = Some("acme".into());

    h.core.set_team_policies(Some(team(
        &[(MODES_KEY, json!(["plan", "default", "acceptEdits"]))],
        &[],
    )));
    // Its mode changed first, then said in its conversation.
    h.wait("the agent to say its mode changed", |h| {
        h.items(&a).iter().any(|i| i["kind"] == "notice")
    })
    .await;
    assert_eq!(h.agent(&a).mode, "acceptEdits");
    let said = h.items(&a);
    assert!(
        said.iter().any(|i| i["kind"] == "notice"
            && i["text"] == "Le mode « auto » n’est pas permis par l’équipe Acme. L’agent passe en mode « acceptEdits », le plus proche permis."),
        "{said:?}"
    );
    assert_eq!(h.core.project(&p.id).unwrap().board.mode, "acceptEdits");
    // The team gone, nothing is locked any more.
    h.core.set_team_policies(None);
    assert!(h.core.locks.read().locks.is_empty());
}

/// The bounds crossed, on combinations drawn by the thousand (TS-18; the app has no proptest, so a small generator of
/// its own, as the fuzzing's tests of `escouade-team`): the modes resolved are within every bound that sets them and
/// within the device's own list; the default mode resolved is one every bound allows (when one does), the device's
/// own when it is allowed, else never one that allows more than it.
#[test]
fn the_bounds_crossed_hold_on_every_combination_drawn() {
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let subset = |bits: u64| -> Vec<&'static str> {
        MODES
            .iter()
            .enumerate()
            .filter(|(i, _)| bits >> i & 1 == 1)
            .map(|(_, m)| *m)
            .collect()
    };
    let rank = |m: &str| MODES.iter().position(|x| *x == m).unwrap();
    for _ in 0..3000 {
        let r = next();
        let own = (r & 1 == 1).then(|| subset(r >> 1));
        let by_team = (r >> 6 & 1 == 1).then(|| subset(r >> 7));
        let by_device = (r >> 12 & 1 == 1).then(|| subset(r >> 13));
        let chosen = MODES[(r >> 20) as usize % MODES.len()];
        let mut device = vec![(DEFAULT_MODE, json!(chosen))];
        if let Some(o) = &own {
            device.push((MODES_KEY, json!(o)));
        }
        let layers = Layers {
            device: map(&device),
            team: by_team
                .as_ref()
                .map(|b| team(&[(MODES_KEY, json!(b))], &[])),
            managed: by_device
                .as_ref()
                .map_or_else(Managed::default, |b| managed(&[(MODES_KEY, json!(b))])),
            ..Default::default()
        };
        let allowed = |m: &str| {
            by_team.as_ref().is_none_or(|b| b.contains(&m))
                && by_device.as_ref().is_none_or(|b| b.contains(&m))
        };
        let case = format!("own {own:?}, team {by_team:?}, device {by_device:?}, chosen {chosen}");
        if let Some(r) = resolve(TABLE, &layers, MODES_KEY) {
            for m in r.value.as_array().expect("a list") {
                let m = m.as_str().expect("a mode");
                assert!(allowed(m), "{m} past a bound: {case}");
                if by_team.is_some() || by_device.is_some() {
                    assert!(
                        own.as_ref().is_none_or(|o| o.contains(&m)),
                        "{m} not the device's: {case}"
                    );
                }
            }
        }
        let resolved = value_of(&layers, DEFAULT_MODE).0;
        let d = resolved.as_str().expect("a mode");
        if !MODES.iter().any(|m| allowed(m)) {
            continue;
        }
        assert!(allowed(d), "the default {d} past a bound: {case}");
        if allowed(chosen) {
            assert_eq!(d, chosen, "{case}");
        } else if MODES.iter().any(|m| allowed(m) && rank(m) <= rank(chosen)) {
            assert!(
                rank(d) <= rank(chosen),
                "{d} allows more than {chosen}: {case}"
            );
        }
    }
}
