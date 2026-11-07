//! Tests of the permission rules (PLG-8, ESC-180): the fake plugin (tests/fixtures/fake-plugin.mjs,
//! `--decide`) that allows, refuses, keeps silent, crashes or leaves it to the user; two plugins
//! that disagree; « … par <plugin> via Escouade » on the card and told to the harness; a private
//! project's requests kept from a plugin not on for it; on the eight fake harnesses (Claude Code,
//! Codex, Copilot CLI, OpenCode, Cursor CLI, Mistral Vibe, Antigravity CLI, Gemini CLI).

use crate::core::AgentOptions;
use crate::core_tests::{harness, Harness};
use crate::engines_tests::accounts_for_every_harness;
use crate::i18n::Lang;
use crate::model::*;
use crate::plugins::journal::Kind;
use crate::plugins::runtime::PhaseName;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures")
}

/// The folder of the fake plugin `id`, its manifest given `permissions`.
fn folder_of(h: &Harness, id: &str, name: &str, args: &[&str], permissions: Value) -> PathBuf {
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let dir = h.dir.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["fake-plugin.mjs", "fake-plugin.cmd", "fake-plugin"] {
        std::fs::copy(fixtures().join(f), dir.join(f)).unwrap();
    }
    let manifest = json!({
        "id": id,
        "name": name,
        "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.1.0" },
        "run": { "command": "fake-plugin", "windows": "fake-plugin.cmd", "args": args },
        "permissions": permissions,
    });
    std::fs::write(
        dir.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    dir
}

/// Installs the folder `src` as the app does: read, then installed.
fn install(h: &Harness, src: &Path) {
    let token = h.core.plugins.inspect_folder(src, Lang::En).unwrap().token;
    h.core.install_plugin(&token, Lang::En).unwrap();
}

/// The fake plugin `id`, named `name`, that decides as `args` say (`--decide allow`…), loaded in
/// development in the harness's core.
fn decider(h: &Harness, id: &str, name: &str, args: &[&str]) {
    let dir = folder_of(h, id, name, args, json!({ "decide": true }));
    h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    // Started as the app starts them, before the first permission.
    crate::plugins::decide::warm(&h.core.plugins, &h.core.plugin_runtime);
}

/// The requests the plugin `id` was asked to decide, as it kept them.
async fn asked_of(h: &Harness, id: &str) -> Vec<Value> {
    let state = h
        .core
        .plugin_runtime
        .call(id, "test/state", json!({}))
        .await
        .unwrap_or_else(|e| panic!("{e}: {:#?}", h.core.plugin_runtime.journal(id)));
    state["decisions"].as_array().cloned().unwrap_or_default()
}

/// The decisions written in the plugin `id`'s journal.
fn decisions_in(h: &Harness, id: &str) -> Vec<String> {
    h.core
        .plugin_runtime
        .journal(id)
        .into_iter()
        .filter(|e| e.kind == Kind::Decision)
        .map(|e| e.text)
        .collect()
}

fn permission(h: &Harness, id: &str) -> Option<Value> {
    h.items(id)
        .into_iter()
        .rfind(|i| i["kind"] == "permission" && i["toolName"] != "ExitPlanMode")
}

/// A notification that calls the user to a permission (« Autoriser Bash : … ? »).
fn asks_user(alert: &str) -> bool {
    alert.contains("Autoriser ") || alert.contains("Allow ")
}

fn texts(h: &Harness, id: &str) -> String {
    h.items(id)
        .iter()
        .filter(|i| i["kind"] == "text")
        .filter_map(|i| i["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

async fn ask_permission(h: &Harness, id: &str, message: &str) {
    h.core
        .send_message(id, message.into(), vec![])
        .await
        .unwrap();
    h.wait("the permission", |h| permission(h, id).is_some())
        .await;
}

/// Waits for the plugins' delay to pass, and the request to be still waiting for the user.
async fn left_to_the_user(h: &Harness, id: &str) {
    tokio::time::sleep(Duration::from_millis(900)).await;
    h.wait("the user asked", |h| {
        h.alerts().iter().any(|a| asks_user(a))
    })
    .await;
    let card = permission(h, id).unwrap();
    assert!(card["decision"].is_null(), "{card:#}");
    assert_eq!(h.agent(id).status, AgentStatus::Waiting);
}

fn phase(h: &Harness, id: &str) -> PhaseName {
    h.core.plugin_runtime.status(id).phase
}

async fn running(h: &Harness, id: &str, when: &str) {
    h.wait(when, |h| phase(h, id) == PhaseName::Running).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_that_decides_runs_before_any_request_as_soon_as_it_is_ready_and_on() {
    let h = harness("plg8-warm");
    let id = "dev.test.warm";
    let src = folder_of(
        &h,
        id,
        "Warm",
        &["--decide", "allow"],
        json!({ "decide": true }),
    );

    // Installed from a folder (the store's and git's go through the same `install`).
    install(&h, &src);
    running(&h, id, "started at its installation").await;

    // Switched off, it stops and stays so; on again, it runs.
    h.core.plugins.choose(id, |c| c.enabled = false).unwrap();
    h.core.plugin_changed(id);
    assert_eq!(phase(&h, id), PhaseName::Idle);
    h.core.plugins.choose(id, |c| c.enabled = true).unwrap();
    h.core.plugin_changed(id);
    running(&h, id, "started when switched on again").await;

    // Its permission taken back, it decides nothing: it stops. Given again, it runs.
    h.core
        .plugins
        .choose(id, |c| c.revoked.push("decide".into()))
        .unwrap();
    h.core.plugin_changed(id);
    assert_eq!(phase(&h, id), PhaseName::Idle);
    h.core.plugins.choose(id, |c| c.revoked.clear()).unwrap();
    h.core.plugin_changed(id);
    running(&h, id, "started with its permission given again").await;

    // Updated: a new version installed over it, running (under Windows it holds its folder).
    let manifest = src.join("escouade-plugin.json");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, text.replace("1.0.0", "1.0.1")).unwrap();
    install(&h, &src);
    running(&h, id, "started at its update").await;
    assert_eq!(h.core.plugins.get(id, Lang::En).unwrap().version, "1.0.1");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_in_development_that_decides_runs_again_once_read_again() {
    let h = harness("plg8-warm-dev");
    let id = "dev.test.warm-dev";
    decider(&h, id, "Warm", &["--decide", "allow"]);
    running(&h, id, "started with the app").await;
    let before = h.core.plugin_runtime.status(id).pid;

    let manifest = h.dir.join(id).join("escouade-plugin.json");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, text.replace("1.0.0", "1.0.1")).unwrap();
    h.core.plugins.reload(&[id.to_string()]);
    h.wait("started anew, read again", |h| {
        let s = h.core.plugin_runtime.status(id);
        s.phase == PhaseName::Running && s.pid != before
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_that_does_not_decide_does_not_start_for_all_that() {
    let h = harness("plg8-warm-not");
    let id = "dev.test.nodecide";
    let src = folder_of(&h, id, "Other", &[], json!({ "events": ["app.started"] }));
    install(&h, &src);
    h.core.plugins.choose(id, |c| c.enabled = false).unwrap();
    h.core.plugin_changed(id);
    h.core.plugins.choose(id, |c| c.enabled = true).unwrap();
    h.core.plugin_changed(id);
    // Long enough for a start under way to show.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(phase(&h, id), PhaseName::Idle);
    assert!(h.core.plugin_runtime.journal(id).is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_allows_a_permission_and_the_card_says_it_was_through_escouade() {
    let h = harness("plg8-allow");
    decider(&h, "dev.test.allow", "Règles", &["--decide", "allow"]);
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    ask_permission(&h, &id, "permission").await;
    h.wait("allowed by the plugin", |h| {
        permission(h, &id).is_some_and(|c| c["decision"] == "allow")
    })
    .await;
    let card = permission(&h, &id).unwrap();
    assert_eq!(card["by"], "Règles", "{card:#}");
    h.wait("the turn over", |h| !h.agent(&id).status.is_active())
        .await;
    assert!(texts(&h, &id).contains("Commande exécutée."));
    // Nobody was called for it.
    assert!(h.alerts().iter().all(|a| !asks_user(a)), "{:?}", h.alerts());
    // It was asked with the request, its project and its agent, never the conversation.
    let asked = asked_of(&h, "dev.test.allow").await;
    assert_eq!(asked.len(), 1, "{asked:#?}");
    let a = &asked[0];
    crate::plugins_protocol_tests::holds(a, "permissions/decide", "params");
    assert_eq!(a["request"]["kind"], "command");
    assert_eq!(a["request"]["tool"], "Bash");
    assert_eq!(a["request"]["input"]["command"], "rm -rf build");
    assert_eq!(a["project"]["id"], p.id);
    assert_eq!(a["project"]["private"], false);
    assert_eq!(a["agent"]["id"], id);
    let journal = decisions_in(&h, "dev.test.allow");
    assert!(
        journal.len() == 1 && journal[0].starts_with("allowed Bash rm -rf build"),
        "{journal:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refusal_tells_the_harness_who_refused_and_why_and_wins_over_an_allowance() {
    let h = harness("plg8-deny");
    // Two plugins that disagree: the refusal wins, whichever comes first.
    decider(&h, "dev.test.allow", "Permissif", &["--decide", "allow"]);
    decider(
        &h,
        "dev.test.deny",
        "Règles de l'équipe",
        &["--decide", "deny", "--decide-message", "pas de rm -rf ici"],
    );
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    ask_permission(&h, &id, "permission").await;
    h.wait("refused by the plugin", |h| {
        permission(h, &id).is_some_and(|c| c["decision"] == "deny")
    })
    .await;
    let card = permission(&h, &id).unwrap();
    assert_eq!(card["by"], "Règles de l'équipe", "{card:#}");
    assert_eq!(card["message"], "pas de rm -rf ici");
    h.wait("the turn over", |h| !h.agent(&id).status.is_active())
        .await;
    // Claude is told who refused, never that the user did.
    assert!(
        texts(&h, &id).contains("Refusé par Règles de l'équipe via Escouade : pas de rm -rf ici"),
        "{}",
        texts(&h, &id)
    );
    assert!(decisions_in(&h, "dev.test.allow")[0].starts_with("allowed"));
    assert!(decisions_in(&h, "dev.test.deny")[0].starts_with("refused Bash rm -rf build"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_that_asks_keeps_silent_or_crashes_leaves_the_request_to_the_user() {
    for (mode, said) in [
        ("ask", "left Bash rm -rf build to the user"),
        ("silent", "no answer in time"),
        ("crash", "did not decide"),
    ] {
        let h = harness(&format!("plg8-{mode}"));
        decider(&h, "dev.test.leave", "Muet", &["--decide", mode]);
        let (p, _) = h.project(false).await;
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        ask_permission(&h, &id, "permission").await;
        left_to_the_user(&h, &id).await;
        h.wait("its decision in the journal", |h| {
            !decisions_in(h, "dev.test.leave").is_empty()
        })
        .await;
        let journal = decisions_in(&h, "dev.test.leave");
        assert!(journal[0].contains(said), "{mode}: {journal:?}");
        // The user answers, as without plugins.
        let req = permission(&h, &id).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        h.core.answer_permission(&id, &req, "allow", None).unwrap();
        h.wait("the turn over", |h| !h.agent(&id).status.is_active())
            .await;
        let card = permission(&h, &id).unwrap();
        assert!(card["by"].is_null(), "{mode}: {card:#}");
        assert!(texts(&h, &id).contains("Commande exécutée."));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_private_project_and_a_question_are_not_given_to_a_plugin() {
    let h = harness("plg8-private");
    decider(&h, "dev.test.allow", "Règles", &["--decide", "allow"]);
    let (p, _) = h.project(false).await;
    h.core
        .projects
        .write()
        .iter_mut()
        .find(|x| x.id == p.id)
        .unwrap()
        .private = true;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    ask_permission(&h, &id, "permission").await;
    left_to_the_user(&h, &id).await;
    assert!(asked_of(&h, "dev.test.allow").await.is_empty());
    // A permission taken back: neither in a project where it is on.
    let q = h.second_project().await;
    h.core
        .plugins
        .choose("dev.test.allow", |c| c.revoked.push("decide".into()))
        .unwrap();
    let other = h.core.create_agent(&q.id, None).await.unwrap().meta.id;
    ask_permission(&h, &other, "permission").await;
    left_to_the_user(&h, &other).await;
    assert!(asked_of(&h, "dev.test.allow").await.is_empty());
}

/// An agent of `engine` in `project`, on the account of its fake.
async fn agent_of(h: &Harness, project: &str, engine: EngineId) -> String {
    h.core
        .create_agent_with(
            project,
            AgentOptions {
                engine: Some(engine),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .meta
        .id
}

#[tokio::test(flavor = "multi_thread")]
async fn the_plugins_decide_for_every_harness() {
    for (verdict, args) in [
        ("allow", vec!["--decide", "allow"]),
        ("deny", vec!["--decide", "deny", "--decide-message", "non"]),
    ] {
        let h = harness(&format!("plg8-harnesses-{verdict}"));
        accounts_for_every_harness(&h);
        // Cursor's Principal, signed in by its own CLI (no key asked for), as its tests have it.
        h.set_settings(|s| {
            let cursor = s
                .accounts
                .iter_mut()
                .find(|a| a.engine == EngineId::CURSOR)
                .unwrap();
            cursor.id = "principal-cursor".into();
            cursor.name = "Principal".into();
            cursor.config_dir.clear();
            // Vibe, Antigravity and Gemini on their Principal, as their tests have them.
            for (engine, program) in [
                (EngineId::VIBE, crate::engine::vibe::bench::fake_vibe()),
                (EngineId::AGY, crate::engine::agy::bench::fake_agy()),
                (
                    EngineId::GEMINI,
                    crate::engine::gemini::bench::fake_gemini(),
                ),
            ] {
                s.accounts.push(Account {
                    id: format!("principal-{engine}"),
                    name: "Principal".into(),
                    engine,
                    claude_path: program,
                    active: true,
                    ..Default::default()
                });
            }
        });
        decider(&h, "dev.test.rules", "Règles", &args);
        let (p, _) = h.project(false).await;
        for (engine, message) in [
            (EngineId::CLAUDE, "permission"),
            (EngineId::CODEX, "commande"),
            (EngineId::COPILOT, "commande"),
            (EngineId::OPENCODE, "commande"),
            (EngineId::CURSOR, "commande"),
            (EngineId::VIBE, "commande"),
            (EngineId::AGY, "commande"),
            (EngineId::GEMINI, "commande"),
        ] {
            let id = agent_of(&h, &p.id, engine).await;
            assert_eq!(h.agent(&id).engine, engine);
            ask_permission(&h, &id, message).await;
            h.wait(&format!("{engine}: decided by the plugin"), |h| {
                permission(h, &id).is_some_and(|c| c["decision"] == verdict)
            })
            .await;
            let card = permission(&h, &id).unwrap();
            assert_eq!(card["by"], "Règles", "{engine}: {card:#}");
            h.wait(&format!("{engine}: the turn over"), |h| {
                !h.agent(&id).status.is_active()
            })
            .await;
        }
        let asked = asked_of(&h, "dev.test.rules").await;
        assert_eq!(asked.len(), 8, "{asked:#?}");
        assert!(
            asked.iter().all(|a| a["request"]["kind"] == "command"),
            "{asked:#?}"
        );
        assert!(h.alerts().iter().all(|a| !asks_user(a)), "{:?}", h.alerts());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_team_rules_example_refuses_rm_rf_with_its_reason() {
    #[cfg(windows)]
    crate::paths::trim_test_path();
    // team-rules is a run.js plugin since 2.0.0 (ESC-326): it runs in WASM, on escouade-js and the pinned Javy shared
    // by the tests (`ready` fetches it once, before the 60 s of the waits below).
    if !crate::plugins_wasm_tests::ready() {
        return;
    }
    let h = harness("plg8-team-rules");
    // Escouade's own plugin (plugins/team-rules, ESC-182 and ESC-314) as its repository holds it: its program
    // is dist/plugin.mjs, built and committed.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../plugins/team-rules");
    let dir = h.dir.join("team-rules");
    std::fs::create_dir_all(dir.join("dist")).unwrap();
    for f in ["escouade-plugin.json", "rules.json", "dist/plugin.mjs"] {
        std::fs::copy(src.join(f), dir.join(f)).unwrap();
    }
    // Its manifest asks for the release that serves WASM; this test plays the decision, not that version check.
    let manifest = dir.join("escouade-plugin.json");
    let mut m: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
    m["escouade"]["min"] = serde_json::json!(env!("CARGO_PKG_VERSION"));
    std::fs::write(&manifest, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    // Its module made now, before the waits' 60 s: Javy and wasmtime's compilation of a debug build take long on a
    // busy machine (the CI's, 2026-11-02), as the app makes it at an install.
    h.core
        .plugin_runtime
        .prepare("dev.escouade.team-rules")
        .expect("a run.js plugin")
        .join()
        .unwrap()
        .expect("its module");
    crate::plugins::decide::warm(&h.core.plugins, &h.core.plugin_runtime);
    let name = h
        .core
        .plugins
        .get("dev.escouade.team-rules", crate::i18n::ui())
        .unwrap()
        .name;
    let (p, _) = h.project(false).await;
    let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    ask_permission(&h, &id, "permission").await;
    h.wait("refused by the team's rules", |h| {
        permission(h, &id).is_some_and(|c| c["decision"] == "deny")
    })
    .await;
    let card = permission(&h, &id).unwrap();
    assert_eq!(card["by"], name.as_str(), "{card:#}");
    assert_eq!(card["message"], "The team's rules forbid `rm -rf`.");
    h.wait("the turn over", |h| !h.agent(&id).status.is_active())
        .await;
    assert!(texts(&h, &id).contains("via Escouade : The team's rules forbid"));
}
