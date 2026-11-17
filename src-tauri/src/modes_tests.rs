//! The mode an agent's harness really runs it in (ESC-489), on every path: a harness whose one mode is Bypass (as
//! Unharnessed, ESC-484) does not start while Bypass is not allowed, and a mode a harness does not have never falls
//! back, unsaid, on one that allows more. The fakes: Minimal's variants (`engine/minimal`), played in the app's process
//! — « Sans Filet » (Bypass alone), « Sans Question » (Plan, Édits auto, Bypass: no « Demander », no Auto), « Sans
//! Mode » (none declared) — and, for what a manifest declares, Unharnessed's harness as its plugin's manifest has it.

use super::*;
use crate::accounts::defaults_with;
use crate::core::AgentOptions;
use crate::core_tests::{harness, Harness};
use crate::engine::minimal::{NO_ASK, ONE_BYPASS, UNDECLARED};
use crate::model::*;
use crate::paths::{test_dir, DataDir};
use crate::tickets_tests::draft;
use serde_json::json;
use tauri::test::mock_app;

/// The harness of the fake `e`, with an account of it serving every project: what a new agent of it goes to.
pub(crate) fn with_account(h: &Harness, e: &dyn Engine) -> EngineId {
    let id = e.id();
    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: format!("compte-{id}"),
        name: format!("Compte {}", e.name()),
        engine: id,
        config_dir: h.dir.join(id.as_str()).to_string_lossy().into(),
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    id
}

/// « Autoriser le mode Bypass », on or off.
pub(crate) fn allow_bypass(h: &Harness, on: bool) {
    let mut s = h.core.settings.read().clone();
    s.allow_bypass = Some(on);
    h.core.save_settings(s).unwrap();
}

/// A new agent of `engine` in the project `p`, as the window makes it (« Autres façons de créer un agent »).
async fn agent_of(h: &Harness, p: &Project, engine: EngineId) -> AgentMeta {
    h.core
        .create_agent_with(
            &p.id,
            AgentOptions {
                engine: Some(engine),
                select: true,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
}

/// The notices of the agent's conversation.
fn notices(h: &Harness, id: &str) -> Vec<String> {
    h.items(id)
        .into_iter()
        .filter(|i| i["kind"] == "notice")
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect()
}

/// Whether the agent's turn reached its harness: a reply of Minimal's.
fn answered(h: &Harness, id: &str) -> bool {
    h.items(id).iter().any(|i| i["kind"] == "text")
}

const ONLY_BYPASS: &str = "ne tourne qu’en Bypass";

// ---------- the rule ----------

#[test]
fn a_harness_runs_in_the_mode_asked_if_it_has_it_else_its_one_mode_else_the_closest_that_allows_no_more(
) {
    let all = ["auto", "default", "plan", "acceptEdits", BYPASS];
    for m in all {
        assert_eq!(runs_in(m, &all).as_deref(), Some(m), "its own");
    }
    // One mode alone: the one it runs in, whatever was asked.
    for m in all {
        assert_eq!(runs_in(m, &[BYPASS]).as_deref(), Some(BYPASS), "{m}");
        assert_eq!(runs_in(m, &["default"]).as_deref(), Some("default"), "{m}");
    }
    // None declared: Bypass.
    assert_eq!(runs_in("default", &[]).as_deref(), Some(BYPASS));
    // Several, not the one asked: the closest that never allows more, never Bypass for a mode that is not it.
    let no_ask = ["plan", "acceptEdits", BYPASS];
    assert_eq!(runs_in("default", &no_ask).as_deref(), Some("plan"));
    assert_eq!(runs_in("auto", &no_ask).as_deref(), Some("acceptEdits"));
    // Every one allows more: none, the agent does not start.
    assert_eq!(runs_in("plan", &["acceptEdits", BYPASS]), None);
    assert_eq!(runs_in("default", &["auto", BYPASS]), None);
    // A mode Escouade does not know is taken as « Demander ».
    assert_eq!(runs_in("yolo", &no_ask).as_deref(), Some("plan"));
}

/// One list of the modes: the policies' (`permissions.modes`) is the core's, and the one a plugin's declaration is read
/// against holds the same modes in the same order (ESC-497).
#[test]
fn the_modes_are_one_list_for_the_core_the_policies_and_the_plugins() {
    assert!(std::ptr::eq(ORDER, escouade_policy::list::MODES));
    assert!(std::ptr::eq(ORDER, crate::locks::MODES));
    assert_eq!(ORDER, escouade_plugins::described::MODES);
    assert_eq!(ORDER.last(), Some(&BYPASS));
}

#[test]
fn the_refusals_say_why_and_what_to_do_in_french_and_in_english() {
    let fr = judge(&ONE_BYPASS, "default", false, Lang::Fr)
        .unwrap_err()
        .to_string();
    assert_eq!(
        fr,
        "Sans Filet ne tourne qu’en Bypass (il ne demande jamais avant d’agir) : active « Autoriser le mode Bypass » dans Réglages › Agents, ou choisis un autre harness."
    );
    let en = judge(&ONE_BYPASS, "default", false, Lang::En)
        .unwrap_err()
        .to_string();
    assert_eq!(
        en,
        "Sans Filet only runs in Bypass (it never asks before it acts): turn on “Allow Bypass mode” in Settings › Agents, or choose another harness."
    );
    assert_eq!(
        judge(&ONE_BYPASS, "default", true, Lang::Fr).unwrap(),
        BYPASS
    );
    // None declared: held for Bypass, said.
    let none = judge(&UNDECLARED, "acceptEdits", false, Lang::Fr)
        .unwrap_err()
        .to_string();
    assert!(
        none.starts_with("Sans Mode ne déclare aucun mode : Escouade le tient pour Bypass."),
        "{none}"
    );
    let none = judge(&UNDECLARED, "acceptEdits", false, Lang::En)
        .unwrap_err()
        .to_string();
    assert!(
        none.starts_with("Sans Mode declares no mode, so Escouade holds it as Bypass."),
        "{none}"
    );
    // Several modes: its Bypass refused as any harness's is; the others go.
    let bypass = judge(&NO_ASK, BYPASS, false, Lang::Fr)
        .unwrap_err()
        .to_string();
    assert!(bypass.contains("Autoriser le mode Bypass"), "{bypass}");
    assert_eq!(
        judge(&NO_ASK, "auto", false, Lang::Fr).unwrap(),
        "acceptEdits"
    );
    // No mode as careful.
    assert_eq!(
        none_as_careful(Lang::Fr, "X", "plan"),
        "X n’a aucun mode qui autorise aussi peu que « Plan » : choisis un de ses modes, ou un autre harness."
    );
    assert_eq!(
        none_as_careful(Lang::En, "X", "default"),
        "X has no mode that allows as little as “Ask”: choose one of its modes, or another harness."
    );
    for lang in [Lang::Fr, Lang::En] {
        let said = settled(lang, "X", "default", BYPASS, true);
        assert!(said.contains("Bypass"), "{said}");
    }
    assert!(
        crate::i18n::check::french_in(&settled(Lang::En, "X", "auto", "acceptEdits", false))
            .is_none()
    );
    assert!(crate::i18n::check::french_in(&only_bypass(Lang::En, "X", false)).is_none());
    assert!(crate::i18n::check::french_in(&missing(Lang::En, "X", "default")).is_none());
    assert!(crate::i18n::check::french_in(&no_discussion(Lang::En, "X")).is_none());
}

/// What a plugin's manifest declares: its modes Escouade knows, in their order; none, or none it knows, and the
/// harness is held for Bypass, its one mode saying why.
#[test]
fn a_manifest_that_declares_no_mode_escouade_knows_is_held_for_bypass() {
    let (rt, _, _) = crate::plugins_acp_tests::runtime("esc489-manifest");
    let described = |id: &str, modes: serde_json::Value| -> &'static dyn Engine {
        let decl = json!({
            "id": id, "name": "Unharnessed", "locate": { "names": ["unharnessed"] },
            "caps": { "modes": modes }, "acp": { "args": ["--mode", "acp"] }
        });
        let decl = serde_json::from_value(decl).unwrap();
        Box::leak(Box::new(crate::plugins::harness::ProcessEngine::described(
            rt.clone(),
            "dev.example.unharnessed",
            decl,
        )))
    };
    // Unharnessed's, as its manifest has it (ESC-484): Bypass alone, with what it means there.
    let one = described(
        "one",
        json!([{ "id": BYPASS, "meaning": { "fr": "Ne demande jamais.", "en": "Never asks." } }]),
    );
    assert!(one.declares_modes());
    let caps = one.caps();
    assert_eq!(
        caps.modes.iter().map(|m| m.id).collect::<Vec<_>>(),
        [BYPASS]
    );
    assert_eq!(caps.modes[0].meaning.as_deref(), Some("Ne demande jamais."));
    // Its new agents: in Bypass, the mode it runs in, never « Demander ».
    assert_eq!(defaults_with(None, one.id(), "").mode, BYPASS);
    let refused = judge(
        one,
        &defaults_with(None, one.id(), "").mode,
        false,
        Lang::Fr,
    )
    .unwrap_err()
    .to_string();
    assert!(
        refused.starts_with("Unharnessed ne tourne qu’en Bypass"),
        "{refused}"
    );
    // None declared, or none Escouade knows: Bypass, said.
    for (id, modes) in [("none", json!([])), ("unknown", json!(["yolo"]))] {
        let e = described(id, modes);
        assert!(!e.declares_modes(), "{id}");
        let caps = e.caps();
        assert_eq!(
            caps.modes.iter().map(|m| m.id).collect::<Vec<_>>(),
            [BYPASS],
            "{id}"
        );
        assert!(
            caps.modes[0]
                .meaning
                .as_deref()
                .is_some_and(|m| m.contains("ne déclare aucun mode")),
            "{id}: {:?}",
            caps.modes[0]
        );
        let refused = judge(e, "default", false, Lang::Fr)
            .unwrap_err()
            .to_string();
        assert!(refused.contains("ne déclare aucun mode"), "{refused}");
    }
    // A mode it does not know, beside those it does: left out.
    let some = described("some", json!(["yolo", "default", "plan"]));
    assert!(some.declares_modes());
    let ids: Vec<&str> = some.caps().modes.iter().map(|m| m.id).collect();
    assert_eq!(ids, ["default", "plan"]);
}

/// The mode of a new agent: one of its harness's, never one that allows more than the one chosen at the first start,
/// as before for the harnesses of Escouade.
#[test]
fn a_new_agent_gets_a_mode_its_harness_has_and_that_allows_no_more_than_the_one_chosen() {
    assert_eq!(
        defaults_with(None, ONE_BYPASS.id(), "acceptEdits").mode,
        BYPASS
    );
    assert_eq!(defaults_with(None, UNDECLARED.id(), "").mode, BYPASS);
    let no_ask = |chosen| defaults_with(None, NO_ASK.id(), chosen).mode;
    assert_eq!(no_ask("acceptEdits"), "acceptEdits");
    assert_eq!(no_ask("auto"), "acceptEdits");
    assert_eq!(no_ask("default"), "plan");
    assert_eq!(no_ask(BYPASS), "acceptEdits", "Bypass is never a default");
    // Escouade's own, unchanged: Claude Code takes them all; Cursor, Gemini… have no Auto, and start in « Demander ».
    for chosen in ["default", "plan", "acceptEdits", "auto"] {
        assert_eq!(defaults_with(None, EngineId::CLAUDE, chosen).mode, chosen);
    }
    assert_eq!(defaults_with(None, EngineId::CLAUDE, BYPASS).mode, "auto");
    for e in [
        EngineId::CURSOR,
        EngineId::GEMINI,
        EngineId::VIBE,
        EngineId::OPENCODE,
    ] {
        assert_eq!(defaults_with(None, e, "auto").mode, "default", "{e}");
        assert_eq!(
            defaults_with(None, e, "acceptEdits").mode,
            "acceptEdits",
            "{e}"
        );
        assert_eq!(defaults_with(None, e, BYPASS).mode, "default", "{e}");
    }
    // An account's own mode its harness does not have: the one it runs in.
    let account = Account {
        engine: NO_ASK.id(),
        default_mode: "default".into(),
        ..Default::default()
    };
    assert_eq!(defaults_with(Some(&account), NO_ASK.id(), "").mode, "plan");
}

// ---------- the paths ----------

/// A new agent of the window: shown in the mode its harness runs in (Bypass, not « Demander »); Bypass not allowed, its
/// first message does not start it, and the conversation says why and what to do; allowed, it starts, in Bypass.
#[tokio::test]
async fn a_new_agent_of_a_harness_whose_one_mode_is_bypass_does_not_start_without_bypass() {
    let h = harness("esc489-window");
    let (p, _) = h.project(false).await;
    let e = with_account(&h, &ONE_BYPASS);
    assert!(!h.core.bypass_allowed());
    let a = agent_of(&h, &p, e).await;
    assert_eq!(a.mode, BYPASS, "its real mode");
    let refused = h
        .core
        .send_message(&a.id, "Bonjour".into(), Vec::new())
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains(ONLY_BYPASS), "{refused}");
    assert!(!h.alive(&a.id));
    assert!(!answered(&h, &a.id));
    // Nor by its warm-up, nor by a start of its own.
    assert!(h.core.ensure_process(&a.id).await.is_err());
    assert!(!h.alive(&a.id));
    // Allowed: it starts, in the mode it runs in.
    allow_bypass(&h, true);
    h.core
        .send_message(&a.id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("its answer", |h| answered(h, &a.id)).await;
    assert_eq!(h.agent(&a.id).mode, BYPASS);
}

/// An agent asked for in a mode its harness does not have (a copy, a tool, the Kanban): the mode its harness runs it in
/// is its own, and its conversation says it; on a harness of several modes, never one that allows more.
#[tokio::test]
async fn an_agent_asked_for_in_a_mode_its_harness_lacks_is_in_the_one_it_runs_in_said() {
    let h = harness("esc489-asked");
    let (p, _) = h.project(false).await;
    let e = with_account(&h, &NO_ASK);
    let made = |mode: &'static str| {
        let h = &h;
        let p = &p;
        async move {
            h.core
                .create_agent_with(
                    &p.id,
                    AgentOptions {
                        engine: Some(e),
                        mode: Some(mode.into()),
                        ..Default::default()
                    },
                )
                .await
                .unwrap()
                .meta
        }
    };
    let a = made("default").await;
    assert_eq!(a.mode, "plan");
    let said = notices(&h, &a.id);
    assert!(
        said.iter().any(|n| n == "Sans Question n’a pas le mode « Demander » : l’agent tourne dans le plus proche qui n’autorise pas plus, « Plan »."),
        "{said:?}"
    );
    let b = made("auto").await;
    assert_eq!(b.mode, "acceptEdits");
    // It starts there.
    h.core
        .send_message(&b.id, "Bonjour".into(), Vec::new())
        .await
        .unwrap();
    h.wait("its answer", |h| answered(h, &b.id)).await;
    assert_eq!(h.agent(&b.id).mode, "acceptEdits");
}

/// Restarted, an agent made before the fix (« Demander » on a harness whose one mode is Bypass, kept so in the state)
/// is not taken up as it was: its mode becomes the one its harness runs in, said, and it does not start without
/// Bypass; another, of a harness of several modes, in « Auto » it does not have, goes on in « Édits auto », said.
#[tokio::test]
async fn an_agent_resumed_after_a_restart_is_judged_in_the_mode_its_harness_runs_in() {
    let dir = test_dir("esc489-restart");
    let data = DataDir::new(dir.join("data"));
    data.ensure().unwrap();
    let cwd = crate::core_tests::repo(&dir.join("repo"));
    let cwd = cwd.to_string_lossy().into_owned();
    let account = |e: EngineId| {
        json!({ "id": format!("compte-{e}"), "name": "Compte", "engine": e.as_str(),
                "configDir": dir.join(e.as_str()).to_string_lossy(), "active": true })
    };
    std::fs::write(
        data.settings_file(),
        json!({ "claudePath": crate::core_tests::fake_cli(), "sound": false, "allowBypass": false, "agentMode": "auto",
                "accounts": [account(ONE_BYPASS.id()), account(NO_ASK.id())] })
        .to_string(),
    )
    .unwrap();
    let agent = |id: &str, e: EngineId, mode: &str| {
        json!({ "id": id, "projectId": "p", "name": id, "model": "", "cwd": cwd, "createdAt": 2,
                "engine": e.as_str(), "account": format!("compte-{e}"), "mode": mode })
    };
    std::fs::write(
        data.state_file(),
        json!({
            "projects": [{ "id": "p", "name": "p", "path": cwd, "color": "red", "createdAt": 1 }],
            "agents": [agent("a1", ONE_BYPASS.id(), "default"), agent("a2", NO_ASK.id(), "auto")],
            "tickets": []
        })
        .to_string(),
    )
    .unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data);
    assert!(!core.bypass_allowed());
    let refused = core
        .send_message("a1", "Tu es là ?".into(), Vec::new())
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains(ONLY_BYPASS), "{refused}");
    let a1 = core.agent("a1").unwrap();
    assert_eq!(a1.lock().meta.mode, BYPASS, "shown as it runs");
    let said: Vec<String> = a1
        .lock()
        .conv
        .items()
        .into_iter()
        .filter_map(|i| i["text"].as_str().map(str::to_string))
        .collect();
    assert!(
        said.iter().any(|t| t == "Sans Filet n’a que le mode « Bypass » : l’agent tourne dans ce mode, pas en « Demander »."),
        "{said:?}"
    );
    assert!(a1.lock().proc.is_none());
    core.send_message("a2", "Tu es là ?".into(), Vec::new())
        .await
        .unwrap();
    assert_eq!(core.agent("a2").unwrap().lock().meta.mode, "acceptEdits");
    std::mem::forget(app);
}

/// A change of mode: only one of its harness's, judged; on a harness whose one mode is Bypass, « Demander » is refused
/// (it would run in Bypass all the same), and Bypass while it is not allowed.
#[tokio::test]
async fn a_change_of_mode_is_one_of_its_harness_s_and_judged() {
    let h = harness("esc489-change-mode");
    let (p, _) = h.project(false).await;
    let one = with_account(&h, &ONE_BYPASS);
    let several = with_account(&h, &NO_ASK);
    let a = agent_of(&h, &p, one).await;
    let set = |id: String, mode: &'static str| {
        let h = &h;
        async move {
            h.core
                .set_agent_options(&id, None, None, Some(mode.into()))
                .await
                .map_err(|e| e.to_string())
        }
    };
    let refused = set(a.id.clone(), "default").await.unwrap_err();
    assert_eq!(
        refused,
        "Sans Filet n’a pas le mode « Demander » : choisis un de ses modes."
    );
    let refused = set(a.id.clone(), BYPASS).await.unwrap_err();
    assert!(refused.contains(ONLY_BYPASS), "{refused}");
    let b = agent_of(&h, &p, several).await;
    assert_eq!(
        b.mode, "acceptEdits",
        "the mode of the first start, `auto`, as it runs it"
    );
    assert!(set(b.id.clone(), "default").await.is_err());
    assert!(set(b.id.clone(), "auto").await.is_err());
    set(b.id.clone(), "plan").await.unwrap();
    assert_eq!(h.agent(&b.id).mode, "plan");
    assert!(set(b.id.clone(), BYPASS)
        .await
        .unwrap_err()
        .contains("Autoriser le mode Bypass"));
    allow_bypass(&h, true);
    set(b.id.clone(), BYPASS).await.unwrap();
    // Escouade's own harnesses: a mode Claude Code has goes as before.
    let c = h.core.create_agent(&p.id, None).await.unwrap().meta;
    set(c.id.clone(), "default").await.unwrap();
}

/// A change of account (same harness, `set_agent_account`): the agent's next start is judged as the first.
#[tokio::test]
async fn a_change_of_account_does_not_start_an_agent_whose_harness_runs_in_bypass_alone() {
    let h = harness("esc489-account");
    let (p, _) = h.project(false).await;
    let e = with_account(&h, &ONE_BYPASS);
    let mut s = h.core.settings.read().clone();
    s.accounts.push(Account {
        id: "second".into(),
        name: "Second".into(),
        engine: e,
        config_dir: h.dir.join("second").to_string_lossy().into(),
        ..Default::default()
    });
    h.core.save_settings(s).unwrap();
    let a = agent_of(&h, &p, e).await;
    h.core.set_agent_account(&a.id, "second").await.unwrap();
    assert_eq!(h.agent(&a.id).account, "second");
    let refused = h
        .core
        .send_message(&a.id, "Bonjour".into(), Vec::new())
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains(ONLY_BYPASS), "{refused}");
    assert!(!h.alive(&a.id));
}

/// A ticket of the Kanban, by the autopilot or by « Lancer » (the tool `start_ticket` goes there too): on a harness
/// whose one mode is Bypass, without Bypass, blocked with why, no agent made; Bypass allowed, « Reprendre » starts it
/// in Bypass.
#[tokio::test]
async fn a_ticket_on_a_harness_whose_one_mode_is_bypass_is_blocked_with_why() {
    let h = harness("esc489-ticket");
    let (p, _) = h.project(false).await;
    let e = with_account(&h, &ONE_BYPASS);
    h.set_board(&p.id, |s| {
        s.autopilot = false;
        s.mode = "acceptEdits".into();
    });
    let ticket = |title: &str| {
        let mut d = draft(title, &[], 5);
        d.engine = Some(Some(e));
        d
    };
    // By the autopilot.
    let t = h.core.ticket_create(&p.id, ticket("Pilote")).await.unwrap();
    h.set_board(&p.id, |s| s.autopilot = true);
    h.wait_ticket(&t.id, "blocked", |t| t.blocked.is_some())
        .await;
    let blocked = h.ticket(&t.id);
    assert!(blocked.agent_id.is_none(), "{blocked:?}");
    assert!(
        blocked.blocked.as_deref().unwrap().contains(ONLY_BYPASS),
        "{blocked:?}"
    );
    assert!(h.core.project_agents(&p.id).is_empty());
    // By « Lancer ».
    h.set_board(&p.id, |s| s.autopilot = false);
    let u = h
        .core
        .ticket_create(&p.id, ticket("À la main"))
        .await
        .unwrap();
    h.core.ticket_start(&u.id).unwrap();
    h.wait_ticket(&u.id, "blocked", |t| t.blocked.is_some())
        .await;
    assert!(h.ticket(&u.id).blocked.unwrap().contains(ONLY_BYPASS));
    assert!(h.core.project_agents(&p.id).is_empty());
    // Allowed, taken up: its agent in Bypass, said in its conversation (the board said « Édits auto »).
    allow_bypass(&h, true);
    h.core.ticket_resume(&u.id).await.unwrap();
    h.wait_ticket(&u.id, "its agent", |t| t.agent_id.is_some())
        .await;
    let agent = h.ticket(&u.id).agent_id.unwrap();
    assert_eq!(h.agent(&agent).mode, BYPASS);
    assert!(
        notices(&h, &agent)
            .iter()
            .any(|n| n.contains("n’a que le mode « Bypass »")),
        "{:?}",
        notices(&h, &agent)
    );
}

/// The mode of the Kanban written in `.escouade.yml` (`board.mode`), a mode its tickets' harness does not have: never
/// taken for true. « Auto » on a harness of several modes without it nor « Demander »: its ticket's agent in « Plan », said.
#[tokio::test]
async fn the_mode_of_a_shared_file_its_harness_lacks_runs_as_the_closest_that_allows_no_more() {
    let h = harness("esc489-file");
    let r = crate::core_tests::repo(&h.dir);
    std::fs::write(
        r.join(crate::projectfile::FILE),
        "version: 1\nboard:\n  mode: auto\n",
    )
    .unwrap();
    let p = h
        .core
        .create_project(&r.to_string_lossy(), "demo", "", false, None, true)
        .await
        .unwrap();
    assert_eq!(p.board.mode, "auto");
    let e = with_account(&h, &NO_ASK);
    h.set_board(&p.id, |s| s.autopilot = false);
    let mut d = draft("Partagé", &[], 5);
    d.engine = Some(Some(e));
    let t = h.core.ticket_create(&p.id, d).await.unwrap();
    h.core.ticket_start(&t.id).unwrap();
    h.wait_ticket(&t.id, "its agent", |t| {
        t.agent_id.is_some() || t.blocked.is_some()
    })
    .await;
    // Started: its agent made (Minimal writes no report, which blocks it after its turn, for another reason).
    let t = h.ticket(&t.id);
    let agent = t.agent_id.clone().unwrap_or_else(|| panic!("{t:?}"));
    // « Auto » on a model that does not take it is « Demander » first (`ticket_agent_options`), then its closest.
    assert_eq!(h.agent(&agent).mode, "plan");
    assert!(notices(&h, &agent).iter().any(|n| n.contains("« Plan »")));
    // On a harness whose one mode is Bypass: blocked.
    let one = with_account(&h, &ONE_BYPASS);
    let mut d = draft("Partagé sans filet", &[], 5);
    d.engine = Some(Some(one));
    let u = h.core.ticket_create(&p.id, d).await.unwrap();
    h.core.ticket_start(&u.id).unwrap();
    h.wait_ticket(&u.id, "blocked", |t| t.blocked.is_some())
        .await;
    assert!(h.ticket(&u.id).blocked.unwrap().contains(ONLY_BYPASS));
}

/// A handover (ESC-88) and a change of harness (« Continuer sur ») go to the closest mode that never allows more: an
/// agent in « Édits auto » does not go to a harness that runs in Bypass alone; one in Bypass does while Bypass is
/// allowed, not once it is not, refused before anything stops.
#[tokio::test]
async fn a_handover_never_takes_a_harness_s_one_mode_that_allows_more_nor_bypass_not_allowed() {
    let h = harness("esc489-handover");
    let (p, _) = h.project(false).await;
    let e = with_account(&h, &ONE_BYPASS);
    let mut s = h.core.settings.read().clone();
    s.engine_order = vec![EngineId::CLAUDE, e];
    h.core.save_settings(s).unwrap();
    let a = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                mode: Some("acceptEdits".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    assert!(
        h.core.handover_targets(&a.id).unwrap().contains(&e),
        "{:?}",
        h.core.handover_targets(&a.id)
    );
    let refused = h
        .core
        .continue_on_engine(&a.id, e)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("n’a aucun mode qui autorise aussi peu"),
        "{refused}"
    );
    assert_eq!(h.agent(&a.id).engine, EngineId::CLAUDE);
    // In Bypass, allowed: handed over; then not allowed: refused before anything changes.
    allow_bypass(&h, true);
    let b = h
        .core
        .create_agent_with(
            &p.id,
            AgentOptions {
                mode: Some(BYPASS.into()),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta;
    allow_bypass(&h, false);
    let refused = h
        .core
        .continue_on_engine(&b.id, e)
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains(ONLY_BYPASS), "{refused}");
    assert_eq!(h.agent(&b.id).engine, EngineId::CLAUDE);
    allow_bypass(&h, true);
    h.core.continue_on_engine(&b.id, e).await.unwrap();
    assert_eq!(
        (h.agent(&b.id).engine, h.agent(&b.id).mode.as_str()),
        (e, BYPASS)
    );
}

/// The planned resume after the usage limit (the app's own task, `resume_due`): judged as any start, said.
#[tokio::test]
async fn the_planned_resume_does_not_start_an_agent_whose_harness_runs_in_bypass_alone() {
    let h = harness("esc489-resume");
    let (p, _) = h.project(false).await;
    let e = with_account(&h, &ONE_BYPASS);
    let a = agent_of(&h, &p, e).await;
    h.core.settings.write().auto_resume = true;
    h.core.agent(&a.id).unwrap().lock().meta.resume_at = Some(1);
    h.core.resume_due().await;
    assert!(!h.alive(&a.id));
    assert!(!answered(&h, &a.id));
    let said = notices(&h, &a.id);
    assert!(
        said.iter()
            .any(|n| n.starts_with("Reprise automatique impossible") && n.contains(ONLY_BYPASS)),
        "{said:?}"
    );
}

/// A discussion of the Docs tab (read-only, ESC-261) keeps a mode that changes nothing: none on a harness whose one
/// mode is Bypass (refused, said); « Plan » on one that has no « Demander ».
#[tokio::test]
async fn a_discussion_runs_only_in_a_mode_that_changes_nothing() {
    use crate::discussion::{Kind, Start};
    let h = harness("esc489-discussion");
    let (p, _) = h.project(false).await;
    let one = with_account(&h, &ONE_BYPASS);
    let several = with_account(&h, &NO_ASK);
    allow_bypass(&h, true);
    let start = |engine| Start {
        kind: Kind::Discussion,
        subject: "Une spec".into(),
        message: Some("Lis le code.".into()),
        engine: Some(engine),
        ..Default::default()
    };
    let refused = h
        .core
        .discussion_start(&p.id, start(one))
        .await
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("n’a ni « Demander » ni « Plan »"),
        "{refused}"
    );
    assert!(h
        .core
        .project_agents(&p.id)
        .iter()
        .all(|a| a.lock().meta.engine != one));
    let d = h
        .core
        .discussion_start(&p.id, start(several))
        .await
        .unwrap();
    assert_eq!(h.agent(&d.meta.id).mode, "plan");
}

/// What the installation of a plugin shows of its harness's modes (`plugins::asks`): one that runs in Bypass alone, one
/// that declares no mode Escouade knows (held for Bypass); nothing for one of several modes.
#[test]
fn the_installation_says_a_harness_runs_in_bypass_alone_or_declares_no_mode() {
    let words = |modes: serde_json::Value| {
        let decl: escouade_plugin_protocol::engine::HarnessDecl = serde_json::from_value(json!({
            "id": "unharnessed", "name": "Unharnessed", "caps": { "modes": modes },
            "acp": { "args": ["--mode", "acp"] }
        }))
        .unwrap();
        escouade_plugins::described::told_words(&decl)
    };
    assert_eq!(words(json!([BYPASS])), ["harnessBypass:Unharnessed"]);
    assert_eq!(words(json!([])), ["harnessNoMode:Unharnessed"]);
    assert_eq!(words(json!(["yolo"])), ["harnessNoMode:Unharnessed"]);
    assert!(words(json!(["default", BYPASS])).is_empty());
    // Not one a user may take back: it is what the harness is.
    assert!(!crate::plugins::revocable("harnessBypass:Unharnessed"));
}
