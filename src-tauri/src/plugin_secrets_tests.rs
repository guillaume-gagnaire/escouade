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
            .plugin_secret_states(ID, vec!["password".to_string()], &[])
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
            in_file: false,
            size: None
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
            in_file: true,
            size: None
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
            in_file: false,
            size: None
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

/// A secret on several lines (`multiline`, ESC-526): kept, its page reads its lines and bytes, never its text; over
/// 64 KiB, « Garder » keeps nothing and says why.
#[tokio::test(flavor = "multi_thread")]
async fn a_secret_on_several_lines_tells_its_size_never_its_text_and_holds_64_kib() {
    use crate::plugin_secrets::{multiline_secrets, SecretSize};
    use escouade_plugins::secrets::SECRET_MAX;
    let h = harness("plugin-secrets-multiline");
    let conf = "client\ndev tun\nproto udp\nremote vpn.example 1194\n";
    h.core.plugin_secret_set(ID, "ovpn", conf, false).unwrap();
    h.core
        .plugin_secret_set(ID, "password", SECRET, false)
        .unwrap();
    let keys = vec!["password".to_string(), "ovpn".to_string()];
    let states = h.core.plugin_secret_states(ID, keys, &["ovpn".to_string()]);
    assert_eq!(states[0].size, None, "a secret on one line says no size");
    assert_eq!(
        states[1].size,
        Some(SecretSize {
            lines: 4,
            bytes: conf.len()
        })
    );
    let shown = serde_json::to_string(&states).unwrap();
    assert!(
        !shown.contains("remote") && !shown.contains(SECRET),
        "{shown}"
    );
    assert!(
        shown.contains(&format!(r#""size":{{"lines":4,"bytes":{}}}"#, conf.len())),
        "{shown}"
    );

    // Over 64 KiB: refused, said, and what was kept stays.
    let big = "x".repeat(SECRET_MAX + 1);
    let refused = h
        .core
        .plugin_secret_set(ID, "ovpn", &big, false)
        .unwrap_err();
    assert!(!refused.keychain_refused);
    assert!(refused.message.contains("64"), "{}", refused.message);
    let kept = h
        .core
        .plugin_secret_states(ID, vec!["ovpn".into()], &["ovpn".into()]);
    assert_eq!(kept[0].size.map(|s| s.lines), Some(4));
    let exact = "y".repeat(SECRET_MAX);
    h.core.plugin_secret_set(ID, "ovpn", &exact, false).unwrap();

    use crate::plugins::manifest::{read, Hand};
    let text = json!({
        "id": ID, "name": "VPN", "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.1.0" },
        "contributes": { "secrets": [
            { "key": "password", "title": "Mot de passe" },
            { "key": "ovpn", "title": "Conf", "multiline": true, "import": [".ovpn"] }
        ] }
    })
    .to_string();
    let manifest = read(&text, Hand::Third).manifest.unwrap();
    assert_eq!(multiline_secrets(Some(&manifest)), ["ovpn"]);
    assert!(multiline_secrets(None).is_empty());
}

/// A configuration of `n` bytes with its certificate, of lines like an OpenVPN one's.
fn openvpn_config(n: usize) -> String {
    let mut c = String::from("client\nremote vpn.example.test 1194\n<ca>\n");
    while c.len() < n {
        c.push_str("MIIDdzCCAl+gAwIBAgIUP1c2xQ8fakefakefakefakefakefake\n");
    }
    c.truncate(n);
    c
}

/// A secret of 6 KB (ESC-527): kept in pieces where Windows refuses more than 1,280 characters, read whole by the
/// plugin, each long line masked in its journal; « Oublier », « Déplacer vers le trousseau » and the uninstall take
/// every piece with it; more than 64 KiB is refused with its reason.
#[tokio::test(flavor = "multi_thread")]
async fn a_long_secret_is_kept_in_pieces_read_whole_and_forgotten_whole() {
    let h = harness("plugin-secrets-long");
    let keychain = memory_of(&h.core.data);
    let dir = h.dir.join("vpn");
    let run = crate::plugins_runtime_tests::fake_module(&dir, &[]);
    std::fs::write(
        dir.join("escouade-plugin.json"),
        json!({
            "id": ID, "name": "VPN", "version": "1.0.0",
            "escouade": { "api": "1", "min": "2.1.0" },
            "run": run,
            "permissions": { "secrets": ["config", "config#a1"] },
            "contributes": { "secrets": [{ "key": "config", "title": "Configuration" }] },
        })
        .to_string(),
    )
    .unwrap();
    let id = h.core.plugins.load_dev(&dir, Lang::Fr).unwrap().id;
    crate::plugins_runtime_tests::warm(&h.core.plugin_runtime, &id);
    let name = secret_name(ID, "config");
    let config = openvpn_config(6 * 1024);

    h.core
        .plugin_secret_set(ID, "config", &config, false)
        .unwrap();
    // In pieces, each under Windows' limit.
    assert!(keychain
        .entry(&name)
        .unwrap()
        .starts_with("escouade-pieces:v1:a:6:"));
    assert_eq!(keychain.all().len(), 7);
    assert!(keychain
        .all()
        .values()
        .all(|v| v.encode_utf16().count() <= 1280));
    // Its page (ESC-526) reads it whole as one secret on several lines: its lines and bytes, never its text.
    let state = &h
        .core
        .plugin_secret_states(ID, vec!["config".into()], &["config".into()])[0];
    assert!(state.kept);
    assert_eq!(
        state.size,
        Some(crate::plugin_secrets::SecretSize::of(&config))
    );
    assert_eq!(state.size.map(|s| s.bytes), Some(6 * 1024));
    // The plugin reads it whole, by `secrets/get`, unchanged.
    let got = h
        .core
        .plugin_runtime
        .call(
            ID,
            "test/do",
            json!({ "behave": "secret", "key": "config" }),
        )
        .await
        .unwrap();
    assert_eq!(got, json!({ "result": config }));
    // A line of it in its journal: masked.
    h.core
        .plugin_runtime
        .call(
            ID,
            "test/do",
            json!({ "behave": "stderr", "lines": ["remote vpn.example.test 1194", "short: client"] }),
        )
        .await
        .unwrap();
    for _ in 0..200 {
        if h.core
            .plugin_runtime
            .journal(ID)
            .iter()
            .any(|e| e.text.contains("short: client"))
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let journal = h.core.plugin_runtime.journal(ID);
    assert!(journal.iter().any(|e| e.text == "••••"), "{journal:#?}");
    assert!(
        journal.iter().all(|e| !e.text.contains("vpn.example.test")),
        "{journal:#?}"
    );
    // A key with `#` is never the plugin's, even listed: it would read a piece.
    let piece = h
        .core
        .plugin_runtime
        .call(
            ID,
            "test/do",
            json!({ "behave": "secret", "key": "config#a1" }),
        )
        .await;
    assert!(format!("{piece:?}").contains("-32001"), "{piece:?}");
    let refused = h
        .core
        .plugin_secret_states(ID, vec!["config".into()], &[])
        .len();
    assert_eq!(refused, 1);

    // A piece gone: the plugin is told the keychain's error, never a secret cut short.
    keychain.delete(&format!("{name}#a3")).unwrap();
    let broken = h
        .core
        .plugin_runtime
        .call(
            ID,
            "test/do",
            json!({ "behave": "secret", "key": "config" }),
        )
        .await;
    assert!(format!("{broken:?}").contains("-32002"), "{broken:?}");
    // Kept again, then « Oublier »: nothing left.
    h.core
        .plugin_secret_set(ID, "config", &config, false)
        .unwrap();
    h.core.plugin_secret_set(ID, "config", "", false).unwrap();
    assert!(keychain.all().is_empty(), "{:?}", keychain.all().keys());

    // More than 64 KiB: refused, said, nothing written.
    let refused = h
        .core
        .plugin_secret_set(ID, "config", &openvpn_config(64 * 1024 + 1), false)
        .unwrap_err();
    assert_eq!(
        refused,
        SecretRefused {
            message: "Ce secret dépasse 64 Kio : il n’a pas été gardé.".into(),
            keychain_refused: false,
        }
    );
    let refused = h
        .core
        .plugin_secret_set(ID, "config", &openvpn_config(64 * 1024 + 1), true)
        .unwrap_err();
    assert!(!refused.keychain_refused);
    assert!(keychain.all().is_empty() && !h.core.data.plugin_secrets_file().exists());
    // 64 KiB exactly: kept.
    h.core
        .plugin_secret_set(ID, "config", &openvpn_config(64 * 1024), false)
        .unwrap();

    // In the file (no keychain), then « Déplacer vers le trousseau »: in pieces there, out of the file.
    h.core.plugin_secret_set(ID, "config", "", false).unwrap();
    h.core
        .plugin_secret_set(ID, "config", &config, true)
        .unwrap();
    assert!(keychain.all().is_empty());
    h.core.plugin_secret_move(ID, "config").unwrap();
    assert_eq!(keychain.all().len(), 7);
    assert!(!h.core.kept.in_file(&name));
    assert_eq!(
        h.core.secrets.get(&name).unwrap().as_deref(),
        Some(config.as_str())
    );

    // Uninstalled: the header and every piece go, another plugin's secret stays.
    keychain
        .set("plugin:dev.test.other:token", "other-5678")
        .unwrap();
    assert!(h.core.plugin_uninstall(ID, false).unwrap());
    assert_eq!(
        keychain.all().into_keys().collect::<Vec<_>>(),
        ["plugin:dev.test.other:token"]
    );
}

#[test]
fn a_secret_too_long_is_said_in_english_too() {
    let e: anyhow::Error = escouade_plugins::secrets::TooLong.into();
    assert_eq!(
        refusal_in(Lang::En, &e, ""),
        "This secret is longer than 64 KiB: it was not kept."
    );
}
