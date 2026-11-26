//! SEC-10 (ESC-595): the proxy's password in the keychain, never in the settings' file nor in the window, and its
//! migration from the address an earlier version wrote.

use crate::core::Core;
use crate::integrations::secrets::{memory_of, MemoryStore, SecretStore};
use crate::model::Settings;
use crate::paths::{test_dir, DataDir};
use crate::proxysecret::{on_load, on_save};
use escouade_model::proxy::SECRET_ENTRY;
use std::sync::atomic::Ordering;
use tauri::test::mock_app;

fn proxied(url: &str) -> Settings {
    Settings {
        proxy_url: url.into(),
        ..Default::default()
    }
}

fn absent() -> MemoryStore {
    let store = MemoryStore::default();
    store.absent.store(true, Ordering::SeqCst);
    store
}

#[test]
fn a_password_in_the_address_moves_to_the_keychain_at_the_start() {
    let store = MemoryStore::default();
    let mut s = proxied("http://ada:secret@127.0.0.1:9000");
    assert!(on_load(&mut s, &store), "the file is written again");
    assert_eq!(store.entry(SECRET_ENTRY).as_deref(), Some("secret"));
    // In memory, the whole address: the requests and the agents' variables need it.
    assert_eq!(s.proxy_url, "http://ada:secret@127.0.0.1:9000");
    assert!(s.proxy_password_in_file.is_empty());
    let file = serde_json::to_string(&s).unwrap();
    assert!(!file.contains("secret"), "{file}");
}

#[test]
fn without_a_keychain_the_password_stays_in_the_file_apart() {
    let store = absent();
    let mut s = proxied("http://ada:secret@127.0.0.1:9000");
    assert!(on_load(&mut s, &store));
    assert_eq!(s.proxy_password_in_file, "secret");
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["proxyUrl"], "http://ada@127.0.0.1:9000");
    // Read again at the next start: the password back in the address, from the file apart.
    let mut again: Settings = serde_json::from_value(v).unwrap();
    assert!(!on_load(&mut again, &store));
    assert_eq!(again.proxy_url, "http://ada:secret@127.0.0.1:9000");
    assert_eq!(again.proxy_password_in_file, "secret");
}

#[test]
fn a_password_kept_in_the_file_apart_moves_to_a_keychain_that_came_since() {
    let store = MemoryStore::default();
    let mut s = proxied("http://ada@127.0.0.1:9000");
    s.proxy_password_in_file = "secret".into();
    assert!(on_load(&mut s, &store));
    assert_eq!(store.entry(SECRET_ENTRY).as_deref(), Some("secret"));
    assert!(s.proxy_password_in_file.is_empty());
    assert_eq!(s.proxy_url, "http://ada:secret@127.0.0.1:9000");
}

#[test]
fn an_address_without_a_password_changes_nothing() {
    let store = MemoryStore::default();
    for url in ["", "http://127.0.0.1:9000", "http://ada@127.0.0.1:9000"] {
        let mut s = proxied(url);
        assert!(!on_load(&mut s, &store), "{url}");
        assert_eq!(s.proxy_url, url);
    }
    assert!(store.all().is_empty());
}

#[test]
fn the_window_saves_the_address_without_its_password_and_the_password_stays() {
    let store = MemoryStore::default();
    store.set(SECRET_ENTRY, "secret").unwrap();
    let before = proxied("http://ada:secret@127.0.0.1:9000");
    // The window got `http://ada@…` and gives it back, maybe another setting changed.
    let mut s = proxied("http://ada@127.0.0.1:9000");
    on_save(&before, &mut s, &store);
    assert_eq!(s.proxy_url, "http://ada:secret@127.0.0.1:9000");
    assert_eq!(store.entry(SECRET_ENTRY).as_deref(), Some("secret"));
    // Another port, the same user: the password still holds.
    let mut s = proxied("http://ada@127.0.0.1:9001");
    on_save(&before, &mut s, &store);
    assert_eq!(s.proxy_url, "http://ada:secret@127.0.0.1:9001");
}

#[test]
fn a_new_password_goes_to_the_keychain_and_a_removed_one_leaves_it() {
    let store = MemoryStore::default();
    let before = proxied("http://ada:old@127.0.0.1:9000");
    store.set(SECRET_ENTRY, "old").unwrap();
    let mut s = proxied("http://ada:new@127.0.0.1:9000");
    on_save(&before, &mut s, &store);
    assert_eq!(store.entry(SECRET_ENTRY).as_deref(), Some("new"));
    // Another user's name: no password of the one before.
    let mut s = proxied("http://bob@127.0.0.1:9000");
    on_save(&proxied("http://ada:new@127.0.0.1:9000"), &mut s, &store);
    assert_eq!(s.proxy_url, "http://bob@127.0.0.1:9000");
    assert_eq!(store.entry(SECRET_ENTRY), None);
    // No proxy at all.
    store.set(SECRET_ENTRY, "x").unwrap();
    let mut s = proxied("");
    on_save(&proxied("http://ada:x@127.0.0.1:9000"), &mut s, &store);
    assert_eq!(store.entry(SECRET_ENTRY), None);
}

#[test]
fn saved_without_a_keychain_a_new_password_goes_to_the_file_apart() {
    let store = absent();
    let mut s = proxied("http://ada:secret@127.0.0.1:9000");
    on_save(&proxied(""), &mut s, &store);
    assert_eq!(s.proxy_password_in_file, "secret");
    // The window's copy of the file apart is never taken: the backend's is.
    let mut s = proxied("http://ada@127.0.0.1:9000");
    s.proxy_password_in_file = "forged".into();
    let mut before = proxied("http://ada:secret@127.0.0.1:9000");
    before.proxy_password_in_file = "secret".into();
    on_save(&before, &mut s, &store);
    assert_eq!(s.proxy_password_in_file, "secret");
    assert_eq!(s.proxy_url, "http://ada:secret@127.0.0.1:9000");
}

/// The whole way: a settings' file of an earlier version with the password in the address; the core loaded on it
/// moves it to the keychain and writes the file again without it; the window's save keeps it; a core loaded again has
/// it back.
#[tokio::test]
async fn the_core_migrates_the_proxys_password_and_keeps_it_out_of_the_file() {
    let data = DataDir::new(test_dir("esc595-proxy-password"));
    data.ensure().unwrap();
    std::fs::write(
        data.settings_file(),
        r#"{ "proxyUrl": "http://ada:s3cret@127.0.0.1:9000", "noProxy": "localhost" }"#,
    )
    .unwrap();
    let app = mock_app();
    let (core, _rx) = Core::load(app.handle().clone(), data.clone());
    let file = std::fs::read_to_string(data.settings_file()).unwrap();
    assert!(!file.contains("s3cret"), "{file}");
    assert!(file.contains("http://ada@127.0.0.1:9000"), "{file}");
    assert_eq!(
        memory_of(&data).entry(SECRET_ENTRY).as_deref(),
        Some("s3cret")
    );
    assert_eq!(
        core.settings.read().proxy_url,
        "http://ada:s3cret@127.0.0.1:9000"
    );
    // The window's copy: as `get_settings` gives it, without the password.
    let window: Settings =
        serde_json::from_value(serde_json::to_value(core.settings.read().clone()).unwrap())
            .unwrap();
    assert_eq!(window.proxy_url, "http://ada@127.0.0.1:9000");
    let mut changed = window.clone();
    changed.sound = !changed.sound;
    core.save_settings(changed).unwrap();
    assert_eq!(
        core.settings.read().proxy_url,
        "http://ada:s3cret@127.0.0.1:9000"
    );
    let file = std::fs::read_to_string(data.settings_file()).unwrap();
    assert!(!file.contains("s3cret"), "{file}");
    drop(core);
    let (again, _rx) = Core::load(mock_app().handle().clone(), data.clone());
    assert_eq!(
        again.settings.read().proxy_url,
        "http://ada:s3cret@127.0.0.1:9000"
    );
}

/// A password kept in the keychain and gone from it at the next start is told apart from an address that never had one.
#[test]
fn a_password_kept_in_the_keychain_and_gone_is_missing_and_never_one_that_never_was() {
    let store = MemoryStore::default();
    let mut s = proxied("http://ada:secret@127.0.0.1:9000");
    assert!(on_load(&mut s, &store));
    assert!(s.proxy_password_kept);
    let file = serde_json::to_value(&s).unwrap();
    assert_eq!(file["proxyPasswordKept"], true);
    // The keychain emptied meanwhile.
    store.delete(SECRET_ENTRY).unwrap();
    let mut again: Settings = serde_json::from_value(file).unwrap();
    assert!(!on_load(&mut again, &store));
    assert!(again.proxy_password_missing);
    assert_eq!(again.proxy_url, "http://ada@127.0.0.1:9000");
    assert!(
        again.proxy_held_back(),
        "never direct is on by default with a proxy"
    );
    again.proxy_never_direct = Some(false);
    assert!(!again.proxy_held_back());
    // Never written: the next start judges again.
    assert!(serde_json::to_value(&again)
        .unwrap()
        .get("proxyPasswordMissing")
        .is_none());
    // A proxy with a user's name alone, no password ever: nothing is missing.
    let mut user_only = proxied("http://ada@127.0.0.1:9000");
    assert!(!on_load(&mut user_only, &MemoryStore::default()));
    assert!(!user_only.proxy_password_missing);
    // The password typed again: found, no longer missing.
    let mut typed = proxied("http://ada:new@127.0.0.1:9000");
    on_save(&again, &mut typed, &store);
    assert!(!typed.proxy_password_missing);
    assert!(typed.proxy_password_kept);
    assert_eq!(store.entry(SECRET_ENTRY).as_deref(), Some("new"));
}

/// SEC-14: with « Ne jamais sortir sans le proxy » on, an agent whose proxy's password is missing is not started — never
/// launched without the proxy, nor with an address the proxy refuses — and says why; off, it starts.
#[tokio::test]
async fn an_agent_does_not_start_while_the_proxys_password_is_missing_and_never_direct_is_on() {
    let h = crate::core_tests::harness("esc595-proxy-held-back");
    let (p, dir) = h.project(false).await;
    // As at a start of the app, before any agent: the password found missing, then the agent made. Set after, the
    // warm-up of the agent just made could start its process first, and the message go to it (the CI's macOS, ESC-650).
    {
        let mut s = h.core.settings.write();
        s.proxy_url = "http://ada@127.0.0.1:9".into();
        s.proxy_password_kept = true;
        s.proxy_password_missing = true;
    }
    let a = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
    let launches = h.launches(&dir).len();
    let e = h
        .core
        .send_message(&a, "Bonjour".into(), vec![])
        .await
        .unwrap_err();
    let said = format!("{e:#}");
    assert!(
        said.contains("« Ne jamais sortir sans le proxy » est allumé"),
        "{said}"
    );
    assert_eq!(h.launches(&dir).len(), launches, "nothing launched");
    // Turned off: the agent starts.
    h.core.settings.write().proxy_never_direct = Some(false);
    h.turn(&a, "Bonjour").await;
    assert!(h.launches(&dir).len() > launches);
    h.core.shutdown();
}
