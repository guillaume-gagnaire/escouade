//! The team's private store in the core (TS-14), against the real `escouade-team-server`: two data folders, Alice the
//! administrator and Bob a member. Alice makes the team's key (never on the server: sealed by its password on her
//! machine, the backup key in a folder of her choice) and its namespace; Bob proposes a plugin of his folder, the
//! store's checks played on his machine; Alice plays them again on hers and signs only when the digests are the same;
//! Bob installs it by his own gesture, after the window of its permissions. Refused in hard: a namespace that is
//! Escouade's own, a plugin outside the namespace, a namespace that holds a plugin installed from another origin, a
//! team's plugin in place of one of the same id from elsewhere. The team's revocation list disables the version at
//! the next reading, its serial going up; the team's policy bounds what its store installs. Every key is drawn by the
//! test.

use std::path::Path;

use escouade_team::settings::{Effective, Status, TeamFile, VersionView, View};
use serde_json::json;

use crate::i18n::Lang;
use crate::plugins::{Origin, State};
use crate::teamdocs_tests::{pair, Side};

const PASSWORD: &str = "un mot de passe assez long";

/// A declarative plugin `id`@`version` in `dir`.
fn plugin(dir: &Path, id: &str, version: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&json!({
            "id": id,
            "name": { "fr": "Règles d’Acme", "en": "Acme’s rules" },
            "version": version,
            "escouade": { "api": "1", "min": "2.0.0" },
            "contributes": {}
        }))
        .unwrap(),
    )
    .unwrap();
}

fn folder(side: &Side, name: &str) -> std::path::PathBuf {
    side.h.core.data.root().join("sources").join(name)
}

/// Alice's key and namespace `fr.acme.*`, the backup key in her « clé USB ».
async fn key(alice: &Side, team: &str) {
    let usb = alice.h.core.data.root().join("usb");
    let (view, backup) = alice
        .h
        .core
        .team_plugin_key_create(team, "fr.acme.*", PASSWORD, &usb)
        .await
        .unwrap();
    let k = view.key.expect("the key is declared");
    assert_eq!(k.namespace, "fr.acme.*");
    assert_eq!(k.ids.len(), 2, "the key of every day and its backup");
    assert!(k.here);
    assert!(Path::new(&backup).starts_with(&usb));
    let backup_text = std::fs::read_to_string(&backup).unwrap();
    assert!(
        escouade_team::signing::SigningKey::open(&backup_text, PASSWORD).is_ok(),
        "the backup key opens with the password"
    );
}

async fn sync(side: &Side, team: &str) {
    side.teams.sync(team).await.unwrap();
}

/// Bob proposes `id`@`version` from his folder; Alice signs it.
async fn publish(alice: &Side, bob: &Side, team: &str, id: &str, version: &str) {
    let dir = folder(bob, &format!("{id}-{version}"));
    plugin(&dir, id, version);
    let s = bob
        .h
        .core
        .team_plugin_submit(team, &dir, &format!("v{version}"))
        .await
        .unwrap();
    assert!(s.report.ok, "{:?}", s.report.checks);
    let proposal = s.proposal.expect("proposed");
    sync(alice, team).await;
    alice
        .h
        .core
        .team_plugin_sign(team, &proposal, PASSWORD, true)
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_member_proposes_an_administrator_signs_and_the_member_installs_by_his_gesture() {
    let (_server, alice, bob, team) = pair("tplug-go").await;
    // Before the key: nothing to propose.
    let dir = folder(&bob, "rules");
    plugin(&dir, "fr.acme.rules", "1.0.0");
    let e = bob
        .h
        .core
        .team_plugin_submit(&team, &dir, "v1.0.0")
        .await
        .unwrap_err();
    assert!(
        e.contains("pas encore de clé") || e.contains("no key"),
        "{e}"
    );
    // A member does not make the team's key; Escouade's own namespace never.
    let e = bob
        .h
        .core
        .team_plugin_key_create(&team, "fr.acme.*", PASSWORD, &folder(&bob, "usb"))
        .await
        .unwrap_err();
    assert!(!e.is_empty());
    let e = alice
        .h
        .core
        .team_plugin_key_create(&team, "dev.escouade.*", PASSWORD, &folder(&alice, "usb"))
        .await
        .unwrap_err();
    assert!(e.contains("dev.escouade"), "{e}");
    let e = alice
        .h
        .core
        .team_plugin_key_create(&team, "fr.acme.*", "court", &folder(&alice, "usb"))
        .await
        .unwrap_err();
    assert!(e.contains("caractères") || e.contains("characters"), "{e}");
    key(&alice, &team).await;
    // The key of every day stays on Alice's machine, sealed: neither in clear nor on the server.
    let sealed = alice
        .h
        .core
        .data
        .root()
        .join("team-signing")
        .join(&team)
        .join("team.key");
    assert!(sealed.is_file());

    // Bob's checks: the version must be the tag's.
    let s = bob
        .h
        .core
        .team_plugin_submit(&team, &dir, "v2.0.0")
        .await
        .unwrap();
    assert!(!s.report.ok);
    assert!(s.proposal.is_none(), "nothing is sent when a check fails");
    let s = bob
        .h
        .core
        .team_plugin_submit(&team, &dir, "v1.0.0")
        .await
        .unwrap();
    assert!(s.report.ok, "{:?}", s.report.checks);
    assert_eq!(
        s.report.checks.len(),
        escouade_plugin_cli::team::CHECKS.len()
    );
    let proposal = s.proposal.unwrap();
    // A wrong password signs nothing.
    sync(&alice, &team).await;
    let e = alice
        .h
        .core
        .team_plugin_sign(&team, &proposal, "un autre mot de passe", true)
        .await
        .unwrap_err();
    assert!(e.contains("mot de passe") || e.contains("password"), "{e}");
    // Without the administrator's agreement that this device compiles the member's plugin: nothing is signed (ESC-648).
    let e = alice
        .h
        .core
        .team_plugin_sign(&team, &proposal, PASSWORD, false)
        .await
        .unwrap_err();
    assert!(e.contains("compile"), "{e}");
    assert_eq!(
        alice
            .h
            .core
            .team_plugins(&team)
            .await
            .unwrap()
            .proposed
            .len(),
        1
    );
    // A member does not sign.
    assert!(bob
        .h
        .core
        .team_plugin_sign(&team, &proposal, PASSWORD, true)
        .await
        .is_err());
    let (report, view) = alice
        .h
        .core
        .team_plugin_sign(&team, &proposal, PASSWORD, true)
        .await
        .unwrap();
    assert_eq!(
        report.digest, s.report.digest,
        "the same digest on both machines"
    );
    assert!(view.proposed.is_empty());

    // Bob sees it, « ACME · privé », and installs it by his gesture: the preview first, then the install.
    sync(&bob, &team).await;
    let view = bob.h.core.team_plugins(&team).await.unwrap();
    // The team as this member named it when he joined (here, its server).
    assert!(!view.name.is_empty());
    let row = view
        .plugins
        .iter()
        .find(|r| r.id == "fr.acme.rules")
        .unwrap();
    assert_eq!(
        (row.version.as_str(), row.installed.as_deref()),
        ("1.0.0", None)
    );
    assert!(!row.name.is_empty());
    let preview = bob
        .h
        .core
        .team_plugin_inspect(&team, "fr.acme.rules", None)
        .await
        .unwrap();
    assert!(
        bob.h.core.plugins.get("fr.acme.rules", Lang::Fr).is_none(),
        "nothing is installed before the member says so"
    );
    let installed = bob.h.core.install_plugin(&preview.token, Lang::Fr).unwrap();
    assert!(
        matches!(&installed.origin, Origin::Team { team: t, name, .. } if *t == team && *name == view.name)
    );
    assert_eq!(crate::locks::origin_name(&installed.origin), "team-store");
    let row = bob
        .h
        .core
        .team_plugins(&team)
        .await
        .unwrap()
        .plugins
        .remove(0);
    assert_eq!(row.installed.as_deref(), Some("1.0.0"));

    // A newer version: the tag above the last, or refused.
    let dir2 = folder(&bob, "rules-again");
    plugin(&dir2, "fr.acme.rules", "1.0.0");
    let again = bob
        .h
        .core
        .team_plugin_submit(&team, &dir2, "v1.0.0")
        .await
        .unwrap();
    assert!(
        !again.report.ok,
        "a version not above the last one published"
    );
    let c = again
        .report
        .checks
        .iter()
        .find(|c| c.key == "version")
        .unwrap();
    assert!(!c.ok);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_app_refuses_in_hard_what_is_outside_the_namespace_or_of_another_origin() {
    let (_server, alice, bob, team) = pair("tplug-hard").await;
    // A plugin of another origin in fr.acme.* on Alice's machine: she cannot take that namespace.
    let local = folder(&alice, "local");
    plugin(&local, "fr.acme.local", "1.0.0");
    alice
        .h
        .core
        .plugins
        .install_folder(&local, Lang::Fr)
        .unwrap();
    let e = alice
        .h
        .core
        .team_plugin_key_create(&team, "fr.acme.*", PASSWORD, &folder(&alice, "usb"))
        .await
        .unwrap_err();
    assert!(e.contains("fr.acme.local"), "{e}");
    alice.h.core.plugins.remove("fr.acme.local", false).unwrap();
    key(&alice, &team).await;

    // Outside the namespace: refused when it is proposed.
    let other = folder(&bob, "other");
    plugin(&other, "com.other.rules", "1.0.0");
    let e = bob
        .h
        .core
        .team_plugin_submit(&team, &other, "v1.0.0")
        .await
        .unwrap_err();
    assert!(e.contains("com.other.rules"), "{e}");
    // Escouade's own id: refused by the checks themselves (anyone's hand).
    let first = folder(&bob, "first");
    plugin(&first, "dev.escouade.rules", "1.0.0");
    let s = bob
        .h
        .core
        .team_plugin_submit(&team, &first, "v1.0.0")
        .await
        .unwrap();
    assert!(!s.report.ok);

    publish(&alice, &bob, &team, "fr.acme.rules", "1.0.0").await;
    sync(&bob, &team).await;
    // Bob has a plugin of the same id from elsewhere: the team's never takes its place.
    let mine = folder(&bob, "mine");
    plugin(&mine, "fr.acme.rules", "0.1.0");
    bob.h.core.plugins.install_folder(&mine, Lang::Fr).unwrap();
    let view = bob.h.core.team_plugins(&team).await.unwrap();
    assert_eq!(view.conflict.as_deref(), Some("fr.acme.rules"));
    assert!(view.plugins[0].elsewhere);
    let e = bob
        .h
        .core
        .team_plugin_inspect(&team, "fr.acme.rules", None)
        .await
        .unwrap_err();
    assert!(
        e.message.contains("autre origine") || e.message.contains("another origin"),
        "{}",
        e.message
    );
    bob.h.core.plugins.remove("fr.acme.rules", false).unwrap();

    // A statement signed by the team's key but read for another namespace, or the archive changed: refused by the app
    // whatever the team's client says.
    let of = bob.teams.plugins_store(&team).await.unwrap();
    let p = of.store.version("fr.acme.rules", "1.0.0").unwrap().clone();
    let (archive, module) = bob
        .teams
        .plugins_files(
            &team,
            p.archive,
            &p.statement.archive_sha256,
            p.module,
            None,
        )
        .await
        .unwrap();
    let fetched = |prefix: &str, archive: Vec<u8>| crate::plugins::team::TeamFetched {
        team: team.clone(),
        name: of.name.clone(),
        mark: of.mark.clone(),
        prefix: prefix.into(),
        keys: of.store.trusted_keys(),
        statement: p.statement_bytes.clone(),
        minisig: p.minisig.clone(),
        archive,
        module: module.clone(),
    };
    for (prefix, bytes) in [
        ("com.other", archive.clone()),
        ("fr.acme", b"autre".to_vec()),
    ] {
        assert!(bob
            .h
            .core
            .plugins
            .inspect_team(fetched(prefix, bytes), Lang::Fr)
            .is_err());
    }
    let stranger = escouade_team::signing::SigningKey::generate().unwrap();
    let mut f = fetched("fr.acme", archive.clone());
    f.keys = vec![stranger.public()];
    assert!(
        bob.h.core.plugins.inspect_team(f, Lang::Fr).is_err(),
        "another key"
    );
    assert!(bob
        .h
        .core
        .plugins
        .inspect_team(fetched("fr.acme", archive), Lang::Fr)
        .is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_digest_that_differs_is_never_signed() {
    let (_server, alice, bob, team) = pair("tplug-digest").await;
    key(&alice, &team).await;
    // Bob's machine says a digest its archive does not give (a build that is not reproducible, or a lie).
    let dir = folder(&bob, "rules");
    plugin(&dir, "fr.acme.rules", "1.0.0");
    let gz = escouade_plugin_cli::team::pack(&dir, Lang::Fr).unwrap();
    let sha = crate::plugins::store::sha256(&gz);
    let submission = escouade_team::plugins::Submission {
        plugin: "fr.acme.rules".into(),
        version: "1.0.0".into(),
        tag: "v1.0.0".into(),
        name: json!("Règles"),
        description: json!(null),
        archive: [0; 16],
        module: None,
        archive_sha256: sha,
        module_sha256: None,
        digest: "f".repeat(64),
        checks: vec![],
    };
    let (proposal, _) = bob
        .teams
        .plugins_submit(&team, submission, &gz, None)
        .await
        .unwrap();
    sync(&alice, &team).await;
    let e = alice
        .h
        .core
        .team_plugin_sign(&team, &proposal, PASSWORD, true)
        .await
        .unwrap_err();
    assert!(
        e.contains("empreintes diffèrent") || e.contains("digests differ"),
        "{e}"
    );
    assert!(alice
        .h
        .core
        .team_plugins(&team)
        .await
        .unwrap()
        .plugins
        .is_empty());
    alice
        .h
        .core
        .team_plugin_dismiss(&team, &proposal)
        .await
        .unwrap();
}

/// A view of the team's settings, its version 1 in force.
fn settings(enforced: serde_json::Value) -> View {
    let mut effective = Effective::default();
    for (k, v) in enforced.as_object().unwrap() {
        effective.enforced.insert(k.clone(), v.clone());
    }
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_revocation_list_disables_and_the_policy_bounds_what_the_store_installs() {
    let (_server, alice, bob, team) = pair("tplug-revoke").await;
    key(&alice, &team).await;
    publish(&alice, &bob, &team, "fr.acme.rules", "1.0.0").await;
    publish(&alice, &bob, &team, "fr.acme.lint", "1.0.0").await;
    sync(&bob, &team).await;

    // The team's policy: its store not among the origins allowed, then one id denied.
    bob.h.core.apply_team_settings(
        &team,
        "ACME",
        Some(&settings(
            json!({ "plugins.allowedOrigins": ["store-verified"] }),
        )),
    );
    let preview = bob
        .h
        .core
        .team_plugin_inspect(&team, "fr.acme.rules", None)
        .await
        .unwrap();
    let e = bob
        .h
        .core
        .install_plugin(&preview.token, Lang::Fr)
        .unwrap_err();
    assert!(e.message.contains("team-store"), "{}", e.message);
    bob.h.core.apply_team_settings(
        &team,
        "ACME",
        Some(&settings(json!({ "plugins.denied": ["fr.acme.lint"] }))),
    );
    let lint = bob
        .h
        .core
        .team_plugin_inspect(&team, "fr.acme.lint", None)
        .await
        .unwrap();
    assert!(bob.h.core.install_plugin(&lint.token, Lang::Fr).is_err());
    let preview = bob
        .h
        .core
        .team_plugin_inspect(&team, "fr.acme.rules", None)
        .await
        .unwrap();
    bob.h.core.install_plugin(&preview.token, Lang::Fr).unwrap();
    assert_eq!(
        bob.h
            .core
            .plugins
            .get("fr.acme.rules", Lang::Fr)
            .unwrap()
            .state,
        State::Ready
    );

    // Alice withdraws it: the team's list, serial 1, signed; Bob's next reading disables it.
    assert!(bob
        .h
        .core
        .team_plugin_revoke(&team, "fr.acme.rules", None, "x", PASSWORD, None)
        .await
        .is_err());
    let view = alice
        .h
        .core
        .team_plugin_revoke(
            &team,
            "fr.acme.rules",
            None,
            "Fuite d’un jeton",
            PASSWORD,
            None,
        )
        .await
        .unwrap();
    assert_eq!(view.serial, Some(1));
    sync(&bob, &team).await;
    bob.h.core.team_plugins_after_read(&team);
    let got = bob.h.core.plugins.get("fr.acme.rules", Lang::Fr).unwrap();
    assert!(
        matches!(&got.state, State::Disabled { reason } if reason.contains("Fuite")),
        "{:?}",
        got.state
    );
    // The circuit breaker of TS-15 reads the same list: withdrawn in every version, the plugin is a lock that names the
    // list and its reason, refused at its install whatever its origin.
    assert!(matches!(
        bob.h.core.locks.read().plugin_cut("fr.acme.rules"),
        Some(crate::locks::LockedBy::Withdrawn { reason, .. }) if reason.contains("Fuite")
    ));
    assert_eq!(bob.h.core.team_withdrawn(&team).len(), 1);
    let view = bob.h.core.team_plugins(&team).await.unwrap();
    assert!(view
        .plugins
        .iter()
        .any(|r| r.id == "fr.acme.rules" && r.withdrawn.is_some()));
    assert!(bob
        .h
        .core
        .team_plugin_inspect(&team, "fr.acme.rules", None)
        .await
        .is_err());
    // Its serial only goes up; the list keeps what it withdrew.
    let view = alice
        .h
        .core
        .team_plugin_revoke(
            &team,
            "fr.acme.lint",
            Some(vec!["1.0.0".into()]),
            "Retirée",
            PASSWORD,
            None,
        )
        .await
        .unwrap();
    assert_eq!(view.serial, Some(2));
    assert_eq!(
        view.plugins
            .iter()
            .filter(|r| r.withdrawn.is_some())
            .count(),
        2
    );
    // The team's list applies to its own plugins alone: a plugin of the same id from elsewhere on another device stays.
    assert!(alice
        .h
        .core
        .plugins
        .get("fr.acme.rules", Lang::Fr)
        .is_none());
}

/// An agent never publishes, signs or withdraws a plugin of the team (§7): no tool of the MCP server reaches the store.
#[test]
fn no_tool_of_the_mcp_server_touches_the_team_s_store() {
    let names: Vec<String> = crate::mcp::tools::Tools::tool_router()
        .list_all()
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();
    assert!(names.len() > 20, "the tools are listed");
    for n in &names {
        for word in [
            "team_plugin",
            "publish",
            "sign_",
            "plugin_revoke",
            "revocation",
            "namespace",
            "plugin_install",
        ] {
            assert!(!n.contains(word), "{n} holds {word}");
        }
    }
}
