//! The team's file in the core (TS-13): the sort of each key, the YAML read and written, and where it applies — the
//! policies crossed with the managed device's, a value of the managed device that wins, the keys of a project for the
//! team's projects alone (rank 2 under the repository's file, rank 5 over it, refused when changed).

use super::*;
use crate::core_tests::harness;
use crate::locks::{Managed, HARNESSES, PROXY_URL, TELEMETRY};
use escouade_team::settings::{Effective, Status, VersionView};
use serde_json::json;

fn map(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

/// A view of the team's settings, its version 1 in force, as `escouade-team` gives it.
fn view(effective: Effective) -> View {
    View {
        applied: Some(1),
        versions: vec![VersionView {
            number: 1,
            digest: "00".repeat(32),
            by: "Claire Martin".into(),
            author: "11".repeat(16),
            at: 1_800_000_000,
            note: String::new(),
            file: TeamFile::default(),
            signers: vec!["11".repeat(16)],
            need: 1,
            signed_by_me: false,
            status: Status::Applied,
        }],
        effective,
        ..Default::default()
    }
}

#[test]
fn each_key_has_its_sort_from_the_core() {
    let s = |k| CORE_KEYS.sort(k);
    assert_eq!(s("permissions.modes"), Some(Sort::Restrict));
    assert_eq!(s("telemetry.enabled"), Some(Sort::Restrict));
    assert_eq!(s("proxy.url"), Some(Sort::Sensitive));
    assert_eq!(s("proxy.authorities"), Some(Sort::Sensitive));
    assert_eq!(s("board.on_approve"), Some(Sort::Value));
    assert_eq!(s("worktrees.setup"), Some(Sort::Executable));
    assert_eq!(s("launch"), Some(Sort::Executable));
    assert_eq!(s("board.test_command"), Some(Sort::Executable));
    assert_eq!(s("plugins.install"), Some(Sort::Executable));
    assert_eq!(s("no.such.key"), None);
    assert!(is_policy("harnesses.allowed"));
    assert!(!is_policy("board.on_approve"));
}

#[test]
fn the_file_is_read_from_its_yaml_and_written_back() {
    let text = r#"
version: 1
defaults:
  board:
    merge_strategy: squash
enforced:
  harnesses:
    allowed: [claude, codex]
  permissions.modes: [plan, default, acceptEdits]
  board:
    on_approve: pr
  telemetry.enabled: false
  hooks:
    preToolUse: "sh x"
proposed:
  worktrees:
    setup: [npm ci]
reasons:
  harnesses.allowed: "Seuls les fournisseurs sous contrat DPA."
quorum: true
"#;
    let (file, warnings) = parse(text).unwrap();
    assert_eq!(
        file.defaults,
        map(&[("board.merge_strategy", json!("squash"))])
    );
    assert_eq!(
        file.enforced.get("harnesses.allowed"),
        Some(&json!(["claude", "codex"]))
    );
    assert_eq!(
        file.enforced.get("permissions.modes"),
        Some(&json!(["plan", "default", "acceptEdits"]))
    );
    assert_eq!(file.enforced.get("board.on_approve"), Some(&json!("pr")));
    assert_eq!(file.enforced.get("telemetry.enabled"), Some(&json!(false)));
    // A key nobody knows is kept (what never enters reads it) and said.
    assert_eq!(file.enforced.get("hooks.preToolUse"), Some(&json!("sh x")));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("hooks.preToolUse"));
    assert_eq!(
        file.proposed.get("worktrees.setup"),
        Some(&json!(["npm ci"]))
    );
    assert_eq!(file.quorum, Some(true));
    assert_eq!(
        file.reasons["harnesses.allowed"],
        "Seuls les fournisseurs sous contrat DPA."
    );
    // Written back and read again, the same.
    let (again, _) = parse(&to_yaml(&file)).unwrap();
    assert_eq!(again, file);
    // An unreadable file, a version of tomorrow.
    assert!(parse("version: 2\n").is_err());
    assert!(parse("enforced: [").is_err());
    assert!(parse("quorum: maybe\n").is_err());
}

#[tokio::test]
async fn the_team_applies_to_its_projects_alone_crossed_with_the_managed_device() {
    let h = harness("team-settings-layers");
    let (a, _) = h.project(false).await;
    let b = h.second_project().await;
    // The managed device allows claude and codex, and sets its own proxy.
    h.core.set_managed(Managed {
        values: [
            (
                HARNESSES.to_owned(),
                (
                    json!(["claude", "codex"]),
                    "the machine's policy".to_owned(),
                ),
            ),
            (
                PROXY_URL.to_owned(),
                (
                    json!("http://managed:8080"),
                    "the machine's policy".to_owned(),
                ),
            ),
        ]
        .into_iter()
        .collect(),
        warnings: Vec::new(),
    });
    // The team allows codex and copilot; it consented proxy (this device accepted it) loses to the managed one.
    let effective = Effective {
        enforced: map(&[
            (HARNESSES, json!(["codex", "copilot"])),
            (TELEMETRY, json!(false)),
            (PROXY_URL, json!("http://team:3128")),
            ("git.agent_branch_prefix", json!("acme/")),
        ]),
        defaults: map(&[("board.merge_strategy", json!("rebase"))]),
        ..Default::default()
    };
    h.core
        .apply_team_settings("acme", "Acme", Some(&view(effective)));
    h.core.set_project_team(&a.id, Some("acme".into())).unwrap();

    // The bound crossed for the team's project: codex alone; the other project keeps the device's.
    let la = h.core.locks_for(&a.id);
    let lb = h.core.locks_for(&b.id);
    assert_eq!(
        la.get(HARNESSES).and_then(|l| l.allowed()),
        Some(vec!["codex".to_owned()])
    );
    assert_eq!(
        lb.get(HARNESSES).and_then(|l| l.allowed()),
        Some(vec!["claude".to_owned(), "codex".to_owned()])
    );
    let why = la.check_engine("claude").unwrap_err().to_string();
    assert!(why.contains("Acme"), "{why}");
    assert!(lb.check_engine("claude").is_ok());
    // A value: the managed device's wins.
    assert_eq!(la.text(PROXY_URL), Some("http://managed:8080"));
    // A key of the app: it holds for the app (no spaces yet), imposed by the team.
    assert_eq!(h.core.locks.read().value(TELEMETRY), Some(&json!(false)));
    assert!(h
        .core
        .locks
        .read()
        .get(HARNESSES)
        .is_some_and(|l| l.bounds.len() == 1));

    // The keys of a project: rank 5 over the file, rank 2 where nothing else sets it; the other project untouched.
    let pa = h.core.project(&a.id).unwrap();
    let pb = h.core.project(&b.id).unwrap();
    assert_eq!(pa.agent_branch_prefix, "acme/");
    assert_eq!(
        crate::projectfile::get(&pa, "board.merge_strategy"),
        json!("rebase")
    );
    assert_ne!(pb.agent_branch_prefix, "acme/");
    // What the team imposes is refused when changed by the window.
    let mut changed = pa.clone();
    changed.agent_branch_prefix = "mine/".into();
    let why = h.core.update_project(changed).unwrap_err().to_string();
    assert!(why.contains("Acme"), "{why}");
    // What it recommends, the device changes.
    let mut mine = h.core.project(&a.id).unwrap();
    crate::projectfile::set(&mut mine, "board.merge_strategy", &json!("squash"));
    h.core.update_project(mine).unwrap();

    // No longer of the team: its locks are the app's again.
    h.core.set_project_team(&a.id, None).unwrap();
    assert_eq!(
        h.core
            .locks_for(&a.id)
            .get(HARNESSES)
            .and_then(|l| l.allowed()),
        Some(vec!["claude".to_owned(), "codex".to_owned()])
    );
    // The device leaves the team: nothing of it holds any more.
    h.core.apply_team_settings("acme", "Acme", None);
    assert!(h.core.locks.read().value(TELEMETRY).is_none());
    assert!(h.core.team_locks.read().is_empty());
}

// ── A plugin's setting (ESC-532) ───────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_plugin_s_setting_is_a_value_read_nested_or_dotted_and_written_back() {
    assert_eq!(
        CORE_KEYS.sort("plugins.dev.acme.vpn.routes"),
        Some(Sort::Value)
    );
    assert_eq!(CORE_KEYS.sort("plugins.recommended"), Some(Sort::Value));
    assert_eq!(CORE_KEYS.sort("plugins.install"), Some(Sort::Executable));
    assert_eq!(
        CORE_KEYS.sort("plugins.dev.acme"),
        None,
        "an id alone is no setting"
    );
    assert_eq!(CORE_KEYS.sort("plugins..x.y"), None);
    assert_eq!(
        setting_of("plugins.dev.acme.vpn.routes", "dev.acme.vpn"),
        Some("routes")
    );
    assert_eq!(
        setting_of("plugins.dev.acme.vpn.routes", "dev.acme.vp"),
        None
    );
    let text = r#"
version: 1
defaults:
  plugins:
    dev.acme.vpn:
      routes: 10.1.0.0/16
enforced:
  plugins.dev.acme.vpn.level: high
  plugins:
    dev.acme.vpn:
      verbose: true
"#;
    let (file, warnings) = parse(text).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        file.defaults.get("plugins.dev.acme.vpn.routes"),
        Some(&json!("10.1.0.0/16"))
    );
    assert_eq!(
        file.enforced.get("plugins.dev.acme.vpn.level"),
        Some(&json!("high"))
    );
    assert_eq!(
        file.enforced.get("plugins.dev.acme.vpn.verbose"),
        Some(&json!(true))
    );
    let (again, _) = parse(&to_yaml(&file)).unwrap();
    assert_eq!(again, file);
    // Nothing of what never enters: a value, signed as the others.
    assert!(escouade_team::settings::never(&file, &CORE_KEYS).is_none());
}

fn vpn() -> crate::plugins::manifest::Manifest {
    let v = json!({
        "id": "dev.acme.vpn", "name": "VPN", "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.2.0" },
        "permissions": { "secrets": ["profile:vpn", "token"] },
        "contributes": {
            "profiles": [{ "id": "vpn", "title": "VPN", "nameField": "name", "fields": [{ "key": "name", "type": "string", "title": "Nom" }, { "key": "password", "secret": true, "title": "Mot de passe" }] }],
            "secrets": [{ "key": "token", "title": "Jeton" }],
            "settings": [
                { "key": "routes", "type": "string", "scope": "project", "title": "Routes" },
                { "key": "level", "type": "select", "scope": "project", "title": "Niveau", "options": ["low", "high"] },
                { "key": "port", "type": "number", "scope": "project", "title": "Port", "repo": false },
                { "key": "conf", "type": "string", "scope": "project", "title": "Conf", "multiline": true, "import": [".conf"] },
                { "key": "profile", "type": "profile", "of": "vpn", "scope": "project", "title": "Profil" },
                { "key": "autoStart", "type": "boolean", "title": "Au lancement" }
            ]
        }
    });
    let r = crate::plugins::manifest::check(&v, crate::plugins::manifest::Hand::Third);
    r.manifest.unwrap_or_else(|p| panic!("{p:?}"))
}

#[test]
fn a_plugin_s_key_the_plugin_refuses_is_said_with_why_as_the_other_keys_refused() {
    let file = TeamFile {
        defaults: map(&[
            ("plugins.dev.acme.vpn.routes", json!("10.1.0.0/16")),
            ("plugins.dev.other.thing.x", json!(1)),
        ]),
        enforced: map(&[
            ("plugins.dev.acme.vpn.token", json!("t")),
            ("plugins.dev.acme.vpn.profile", json!("Client A")),
            ("plugins.dev.acme.vpn.port", json!(1)),
            ("plugins.dev.acme.vpn.conf", json!("x")),
            ("plugins.dev.acme.vpn.autoStart", json!(true)),
            ("plugins.dev.acme.vpn.nope", json!(1)),
            ("plugins.dev.acme.vpn.level", json!("max")),
        ]),
        ..Default::default()
    };
    let ids = vec!["dev.acme.vpn".to_owned()];
    // The tests write in French (`escouade_base::i18n`, never set in them).
    let said = plugin_warnings(&file, &ids, |id| (id == "dev.acme.vpn").then(vpn));
    let all = said.join("\n");
    assert_eq!(said.len(), 8, "{all}");
    assert!(!all.contains("plugins.dev.acme.vpn.routes"), "{all}");
    for (key, why) in [
        ("token", "secret"),
        ("profile", "profil"),
        ("port", "repo: false"),
        ("conf", "importe"),
        ("autoStart", "réglage de l’app"),
        ("nope", "ne déclare pas"),
        ("level", "type du réglage (select)"),
    ] {
        assert!(
            said.iter().any(
                |w| w.contains(&format!("enforced.plugins.dev.acme.vpn.{key} refusé"))
                    && w.contains(why)
            ),
            "{key}: {all}"
        );
    }
    assert!(
        said.iter()
            .any(|w| w.contains("plugins.dev.other.thing.x") && w.contains("pas installé")),
        "{all}"
    );
}

#[tokio::test]
async fn a_team_s_plugin_setting_reaches_its_projects_alone() {
    let h = harness("team-settings-plugins");
    let (a, _) = h.project(false).await;
    let b = h.second_project().await;
    let effective = Effective {
        enforced: map(&[("plugins.dev.acme.vpn.level", json!("high"))]),
        defaults: map(&[
            ("plugins.dev.acme.vpn.routes", json!("10.1.0.0/16")),
            ("board.merge_strategy", json!("rebase")),
        ]),
        ..Default::default()
    };
    h.core
        .apply_team_settings("acme", "Acme", Some(&view(effective)));
    h.core.set_project_team(&a.id, Some("acme".into())).unwrap();
    let pa = h.core.project(&a.id).unwrap();
    let t = pa
        .shared
        .team_plugins
        .clone()
        .expect("the team's keys of its plugins");
    assert_eq!(t.team, "Acme");
    assert_eq!(
        t.enforced.get("plugins.dev.acme.vpn.level"),
        Some(&json!("high"))
    );
    assert_eq!(
        t.defaults.get("plugins.dev.acme.vpn.routes"),
        Some(&json!("10.1.0.0/16"))
    );
    assert!(!t.defaults.contains_key("board.merge_strategy"));
    // A project's key of the team still applies as before.
    assert_eq!(
        crate::projectfile::get(&pa, "board.merge_strategy"),
        json!("rebase")
    );
    // What the plugin is given, by setting.
    let v = plugin_values(&t, "dev.acme.vpn");
    assert_eq!(v.team, "Acme");
    assert_eq!(v.enforced.get("level"), Some(&json!("high")));
    assert_eq!(v.defaults.get("routes"), Some(&json!("10.1.0.0/16")));
    assert!(plugin_values(&t, "dev.other.x").enforced.is_empty());
    // The other project, of no team: nothing.
    assert!(h.core.project(&b.id).unwrap().shared.team_plugins.is_none());
    // The device leaves the team: nothing of it holds any more.
    h.core.apply_team_settings("acme", "Acme", None);
    assert!(h.core.project(&a.id).unwrap().shared.team_plugins.is_none());
}

// ── The mode « auto » imposed (SR3-07) ───────────────────────────────────────────────────────────────────────────────

/// « auto » as the agents' default asks the consent of the device, as a sensitive key; a stricter mode does not.
#[test]
fn the_mode_auto_as_the_default_asks_the_consent_of_the_device() {
    let mode = escouade_policy::list::DEFAULT_MODE;
    assert_eq!(CORE_KEYS.sort(mode), Some(Sort::Value));
    assert!(CORE_KEYS.asks(mode, &json!("auto")));
    for stricter in ["plan", "default", "acceptEdits"] {
        assert!(!CORE_KEYS.asks(mode, &json!(stricter)), "{stricter}");
    }
    assert!(CORE_KEYS.asks(PROXY_URL, &json!("http://proxy:3128")));
    assert!(!CORE_KEYS.asks(TELEMETRY, &json!(false)));
    // Whose passages the team's machine takes on its own (SR3-05): a value of the team, neither a lock nor a
    // project's key.
    assert_eq!(
        CORE_KEYS.sort(crate::teamhandoff::MACHINE_TAKES),
        Some(Sort::Value)
    );
}

/// The first time a team's version waits for this device's consent to « auto », the box « À toi » says it, once.
#[tokio::test]
async fn the_mode_auto_waiting_for_consent_is_said_in_the_box_once() {
    let h = harness("team-settings-auto");
    let (a, _) = h.project(false).await;
    h.core.set_project_team(&a.id, Some("acme".into())).unwrap();
    let pending = |value: &str| Effective {
        pending: vec![escouade_team::settings::Pending {
            part: escouade_team::settings::Part::Enforced,
            key: escouade_policy::list::DEFAULT_MODE.into(),
            value: json!(value),
            sort: Sort::Value,
            fingerprint: format!("print-{value}"),
        }],
        ..Default::default()
    };
    let told = |h: &crate::core_tests::Harness| {
        h.core
            .asks_all()
            .into_iter()
            .filter(|x| x.title.contains("auto") && x.title.contains("Acme"))
            .count()
    };
    h.core
        .apply_team_settings("acme", "Acme", Some(&view(pending("auto"))));
    assert_eq!(told(&h), 1);
    // Not applied meanwhile: the device's mode stays its own.
    assert!(h
        .core
        .locks_for(&a.id)
        .get(escouade_policy::list::DEFAULT_MODE)
        .is_none());
    // Read again: not said twice.
    h.core
        .apply_team_settings("acme", "Acme", Some(&view(pending("auto"))));
    assert_eq!(told(&h), 1);
}
