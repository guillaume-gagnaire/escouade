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
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures")
}

/// The fake plugin `id`, named `name`, that decides as `args` say (`--decide allow`…), loaded in
/// development in the harness's core.
fn decider(h: &Harness, id: &str, name: &str, args: &[&str]) {
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
        "permissions": { "decide": true },
    });
    std::fs::write(
        dir.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
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
                .find(|a| a.engine == EngineId::Cursor)
                .unwrap();
            cursor.id = "principal-cursor".into();
            cursor.name = "Principal".into();
            cursor.config_dir.clear();
            // Vibe, Antigravity and Gemini on their Principal, as their tests have them.
            for (engine, program) in [
                (EngineId::Vibe, crate::engine::vibe::bench::fake_vibe()),
                (EngineId::Agy, crate::engine::agy::bench::fake_agy()),
                (
                    EngineId::Gemini,
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
            (EngineId::Claude, "permission"),
            (EngineId::Codex, "commande"),
            (EngineId::Copilot, "commande"),
            (EngineId::Opencode, "commande"),
            (EngineId::Cursor, "commande"),
            (EngineId::Vibe, "commande"),
            (EngineId::Agy, "commande"),
            (EngineId::Gemini, "commande"),
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
    let h = harness("plg8-team-rules");
    let src = fixtures().join("plugins/examples/team-rules");
    let dir = h.dir.join("team-rules");
    std::fs::create_dir_all(&dir).unwrap();
    for f in std::fs::read_dir(&src).unwrap() {
        let f = f.unwrap();
        std::fs::copy(f.path(), dir.join(f.file_name())).unwrap();
    }
    h.core.plugins.load_dev(&dir, Lang::En).unwrap();
    crate::plugins::decide::warm(&h.core.plugins, &h.core.plugin_runtime);
    let name = h
        .core
        .plugins
        .get("dev.exemple.team-rules", crate::i18n::ui())
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
