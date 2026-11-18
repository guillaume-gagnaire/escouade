//! A plugin's secret where the system has no keychain (issue #37, ESC-499): « Garder » says why it kept nothing, the
//! file is kept only once chosen, the plugin reads its secret from it by `secrets/get`, nothing of it reaches a log,
//! the plugin's journal or the copies of the state, and the choice is taken back.

use crate::core_tests::harness;
use crate::i18n::Lang;
use crate::integrations::secrets::{memory_of, SecretStore};
use crate::plugin_secrets::{refusal_in, FileSecret, SecretRefused, SecretState};
use crate::plugins::runtime::secret_name;
use serde_json::json;
use std::sync::atomic::Ordering;
use std::sync::Arc;

const ID: &str = "dev.test.vpn";
const SECRET: &str = "hunter2-openvpn-password";

/// What Escouade's own crates log while the test runs (the test's own process under nextest), at every level. The
/// other crates are left out: at « trace », the compiler of the plugins' modules alone says enough to make the test
/// last minutes (it timed out on the CI's Mac).
struct Capture(parking_lot::Mutex<Vec<String>>);

impl log::Log for Capture {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        if !record.target().starts_with("escouade") {
            return;
        }
        self.0.lock().push(format!("{}", record.args()));
    }
    fn flush(&self) {}
}

fn capture() -> Option<&'static Capture> {
    let c: &'static Capture = Box::leak(Box::new(Capture(parking_lot::Mutex::new(Vec::new()))));
    log::set_logger(c).ok()?;
    log::set_max_level(log::LevelFilter::Trace);
    Some(c)
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_keychain_keep_says_why_and_the_chosen_file_keeps_what_the_plugin_reads() {
    let logs = capture();
    let h = harness("plugin-secrets-absent");
    let keychain = memory_of(&h.core.data);
    keychain.absent.store(true, Ordering::SeqCst);
    let dir = h.dir.join("vpn");
    let run = crate::plugins_runtime_tests::fake_module(&dir, &[]);
    std::fs::write(
        dir.join("escouade-plugin.json"),
        json!({
            "id": ID, "name": "VPN", "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.1.0" },
            "run": run,
            "permissions": { "secrets": ["password"] },
            "contributes": { "secrets": [{ "key": "password", "title": "Mot de passe" }] },
        })
        .to_string(),
    )
    .unwrap();
    let id = h.core.plugins.load_dev(&dir, Lang::Fr).unwrap().id;
    crate::plugins_runtime_tests::warm(&h.core.plugin_runtime, &id);
    let states = |h: &crate::core_tests::Harness| {
        h.core
            .plugin_secret_states(ID, vec!["password".to_string()])
    };

    // « Garder »: nothing kept, and it says why (issue #37: it said nothing).
    let refused = h
        .core
        .plugin_secret_set(ID, "password", SECRET, false)
        .unwrap_err();
    assert_eq!(
        refused,
        SecretRefused {
            message: "Pas de trousseau système sur ce poste : le secret n’a pas été gardé.".into(),
            keychain_refused: true,
        }
    );
    assert_eq!(
        states(&h),
        [SecretState {
            key: "password".into(),
            kept: false,
            in_file: false
        }]
    );
    // Never taken in silence: no file before the choice.
    assert!(!h.core.data.plugin_secrets_file().exists());
    assert!(!h.core.secrets_file().chosen);
    // The plugin is told none, as before (`null`).
    let got = h
        .core
        .plugin_runtime
        .call(
            ID,
            "test/do",
            json!({ "behave": "secret", "key": "password" }),
        )
        .await
        .unwrap();
    assert_eq!(got, json!({ "result": null }));

    // « Garder dans un fichier lisible de toi seul »: kept, said where.
    h.core
        .plugin_secret_set(ID, "password", SECRET, true)
        .unwrap();
    assert_eq!(
        states(&h),
        [SecretState {
            key: "password".into(),
            kept: true,
            in_file: true
        }]
    );
    let file = h.core.secrets_file();
    assert!(file.chosen && !file.keychain);
    assert_eq!(
        file.secrets,
        [FileSecret {
            plugin: ID.into(),
            plugin_name: "VPN".into(),
            key: "password".into()
        }]
    );
    assert_eq!(
        file.path,
        h.core.data.plugin_secrets_file().display().to_string()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(h.core.data.plugin_secrets_file())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    // The plugin reads it by `secrets/get`, unchanged.
    let got = h
        .core
        .plugin_runtime
        .call(
            ID,
            "test/do",
            json!({ "behave": "secret", "key": "password" }),
        )
        .await
        .unwrap();
    assert_eq!(got, json!({ "result": SECRET }));
    // Another secret of the window (an account's token) still goes to the keychain alone: refused, never to the file.
    assert!(h
        .core
        .secrets
        .set("forge:gitlab.com", "gl-token-123456")
        .is_err());
    assert_eq!(h.core.kept.names(), [secret_name(ID, "password")]);

    // Never in the copies of the state, never in the plugin's journal, never in the app's log.
    escouade_model::backups::back_up_all(h.core.data.root(), chrono::Utc::now());
    for entry in walk(&h.core.data.root().join(escouade_model::backups::DIR)) {
        let bytes = std::fs::read(&entry).unwrap();
        let text = escouade_model::backups::read_copy(&entry)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());
        assert!(!text.contains(SECRET), "{}", entry.display());
        let name = entry.file_name().unwrap().to_string_lossy();
        assert!(!name.starts_with("plugin-secrets"), "{}", entry.display());
    }
    let journal = h.core.plugin_runtime.journal(ID);
    assert!(
        journal.iter().all(|e| !e.text.contains(SECRET)),
        "{journal:#?}"
    );
    if let Some(logs) = logs {
        let said = logs.0.lock().clone();
        assert!(said.iter().any(|l| l.contains("refused")), "{said:#?}");
        assert!(said.iter().all(|l| !l.contains(SECRET)), "{said:#?}");
    }

    // Read again by the next start of the app (the same data folder).
    let again = crate::integrations::kept::Kept::new(
        Arc::new(crate::integrations::secrets::MemoryStore::default()),
        &h.core.data,
    );
    assert_eq!(
        again.get(&secret_name(ID, "password")).unwrap().as_deref(),
        Some(SECRET)
    );

    // A keychain that comes: nothing moves by itself; « Déplacer vers le trousseau » moves it.
    keychain.absent.store(false, Ordering::SeqCst);
    assert!(h.core.secrets_file().keychain);
    assert!(states(&h)[0].in_file);
    h.core.plugin_secret_move(ID, "password").unwrap();
    assert_eq!(
        keychain.entry(&secret_name(ID, "password")).as_deref(),
        Some(SECRET)
    );
    assert_eq!(
        states(&h),
        [SecretState {
            key: "password".into(),
            kept: true,
            in_file: false
        }]
    );

    // Taken back in Réglages: the file goes, with what it held.
    keychain.absent.store(true, Ordering::SeqCst);
    h.core
        .plugin_secret_set(ID, "password", SECRET, true)
        .unwrap();
    h.core.secrets_file_remove().unwrap();
    assert!(!h.core.data.plugin_secrets_file().exists());
    assert!(!h.core.secrets_file().chosen);
    assert!(!states(&h)[0].kept);
}

/// `secrets.fileFallback` off, by this device's policy or by a team's (ESC-520): « Garder dans un fichier » is refused
/// with who denies it, no secret goes to the file, the file chosen before is no longer read but kept, to move or erase;
/// the policy taken off, it is read again.
#[tokio::test(flavor = "multi_thread")]
async fn a_policy_that_denies_the_file_refuses_it_and_no_longer_reads_the_one_chosen() {
    use crate::locks::{Managed, TeamPolicies, SECRETS_FILE};
    for team in [false, true] {
        let h = harness(&format!("plugin-secrets-denied-{team}"));
        let keychain = memory_of(&h.core.data);
        keychain.absent.store(true, Ordering::SeqCst);
        let name = secret_name(ID, "password");
        // Chosen before the policy.
        h.core
            .plugin_secret_set(ID, "password", SECRET, true)
            .unwrap();
        let says = if team {
            // A team's key of the app (not of its projects): it holds while the device is in the team.
            h.core.set_team_policies(Some(TeamPolicies {
                id: "acme".into(),
                team: "Acme".into(),
                signed: None,
                recommended: Default::default(),
                enforced: [(SECRETS_FILE.to_owned(), json!(false))].into(),
            }));
            "par l’équipe Acme"
        } else {
            h.core.set_managed(Managed {
                values: [(
                    SECRETS_FILE.to_owned(),
                    (json!(false), "the machine's policy".to_owned()),
                )]
                .into(),
                warnings: Vec::new(),
            });
            "par la stratégie de ce poste"
        };
        let refused = h
            .core
            .plugin_secret_set(ID, "other", "x-secret-x", true)
            .unwrap_err();
        assert!(!refused.keychain_refused);
        for part in ["est interdit", says, SECRETS_FILE] {
            assert!(refused.message.contains(part), "{}", refused.message);
        }
        // The one chosen: no longer read, still there, nothing more in it.
        assert_eq!(h.core.secrets.get(&name).unwrap(), None);
        let file = h.core.secrets_file();
        assert!(file.chosen && file.secrets.len() == 1);
        // Its field says it is in the file still, to move it to a keychain.
        assert_eq!(
            h.core
                .plugin_secret_states(ID, vec!["password".to_string()]),
            [SecretState {
                key: "password".into(),
                kept: true,
                in_file: true
            }]
        );
        // Taken off: read again.
        if team {
            h.core.set_team_policies(None);
        } else {
            h.core.set_managed(Managed::default());
        }
        assert_eq!(h.core.secrets.get(&name).unwrap().as_deref(), Some(SECRET));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_keychain_that_refuses_says_its_reason_without_the_secret() {
    let h = harness("plugin-secrets-refused");
    let keychain = memory_of(&h.core.data);
    keychain.refuse.store(true, Ordering::SeqCst);
    let refused = h
        .core
        .plugin_secret_set(ID, "password", SECRET, false)
        .unwrap_err();
    assert!(refused.keychain_refused);
    assert_eq!(
        refused.message,
        "Le trousseau système a refusé le secret (keychain out of reach) : il n’a pas été gardé."
    );
    // A keychain that says it kept it and does not: said too.
    keychain.refuse.store(false, Ordering::SeqCst);
    keychain.lose.store(true, Ordering::SeqCst);
    let refused = h
        .core
        .plugin_secret_set(ID, "password", SECRET, false)
        .unwrap_err();
    assert!(refused.keychain_refused, "{refused:?}");
    assert!(!refused.message.contains(SECRET));
    assert!(!h.core.data.plugin_secrets_file().exists());
}

#[test]
fn the_causes_are_said_in_english_too() {
    let none: anyhow::Error = escouade_plugins::secrets::NoKeychain.into();
    assert_eq!(
        refusal_in(Lang::En, &none, SECRET),
        "No system keychain on this machine: the secret was not kept."
    );
    let other = anyhow::anyhow!("locked, {SECRET} refused");
    assert_eq!(
        refusal_in(Lang::En, &other, SECRET),
        "The system keychain refused the secret (locked, •••• refused): it was not kept."
    );
}

#[test]
fn the_copies_of_the_state_never_take_the_file_of_the_secrets() {
    assert!(!escouade_model::backups::FILES.contains(&"plugin-secrets.json"));
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .flat_map(|e| {
            let p = e.path();
            if p.is_dir() {
                walk(&p)
            } else {
                vec![p]
            }
        })
        .collect()
}
