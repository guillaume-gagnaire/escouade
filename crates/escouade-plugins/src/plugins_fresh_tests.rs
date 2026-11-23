//! The freshness of the revocation list (audit SEC-15, ESC-596): an attestation taken for its own list alone (its serial,
//! its fingerprint, a freshness key), the age of the list held, and a store that reads the attestation beside the copy
//! on escouade.dev, here a fake mirror of the test. The vectors are those of tests/fixtures/plugins/revocations-fresh.json,
//! made by store/server/utils/revocation-fresh.ts: the app and the workflow read the same attestations. No network.

use crate::i18n::Lang;
use crate::paths::DataDir;
use crate::store::{self, list_age, Store};
use crate::store_keys::{Key, Role};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/plugins/revocations-fresh.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn s(v: &Value, path: &str) -> String {
    v.pointer(path).and_then(Value::as_str).unwrap().to_string()
}

fn key(public: String, roles: &[Role]) -> Key {
    Key {
        public,
        roles: roles.to_vec(),
    }
}

#[test]
fn an_attestation_holds_for_its_own_list_alone() {
    let v = vectors();
    let fresh = vec![key(s(&v, "/freshKey"), &[Role::Freshness])];
    let list = s(&v, "/list");
    let check = |att: &str, keys: &[Key], serial: u64| {
        store::check_freshness(
            keys,
            list.as_bytes(),
            serial,
            s(&v, &format!("/{att}/json")).as_bytes(),
            &s(&v, &format!("/{att}/minisig")),
        )
    };
    assert_eq!(
        check("good", &fresh, 3).as_deref(),
        Ok("2026-11-16T06:00:00Z")
    );
    // Of another list: another fingerprint, another serial.
    assert!(check("other", &fresh, 3)
        .unwrap_err()
        .contains("fingerprint"));
    assert!(check("serial4", &fresh, 3).unwrap_err().contains("n° 4"));
    assert!(check("good", &fresh, 4).is_err());
    // Signed by the key of the lists: no freshness key; and none embedded, none taken.
    assert!(check("byListKey", &fresh, 3).is_err());
    // The attestation is judged by the freshness key's role alone: the same freshness key given another role (the
    // revocation key's, the backup's) does not attest, and the key of the lists never does.
    for roles in [
        &[Role::Revocation][..],
        &[Role::Backup],
        &[Role::ThirdParty, Role::Revocation],
    ] {
        let e = check("good", &[key(s(&v, "/freshKey"), roles)], 3).unwrap_err();
        assert!(e.contains("OutOfRole"), "{roles:?}: {e}");
    }
    assert!(check("good", &[key(s(&v, "/listKey"), &[Role::Revocation])], 3).is_err());
    assert!(check("good", &[], 3).is_err());
    // What is signed, changed: the date or the fingerprint.
    let changed = s(&v, "/good/json").replace("2026-11-16", "2026-12-16");
    assert!(store::check_freshness(
        &fresh,
        list.as_bytes(),
        3,
        changed.as_bytes(),
        &s(&v, "/good/minisig")
    )
    .is_err());
}

fn at(d: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(d)
        .unwrap()
        .with_timezone(&chrono::Utc)
}

#[test]
fn a_list_is_old_after_7_days_without_a_new_one_or_an_attestation() {
    let issued = Some("2026-11-02T10:00:00Z");
    // With a freshness key: the later of the list's date and of its attestation.
    let (stale, since) = list_age(issued, None, None, true, at("2026-11-09T09:00:00Z"));
    assert!(!stale);
    assert_eq!(since.as_deref(), Some("2026-11-02T10:00:00+00:00"));
    assert!(list_age(issued, None, None, true, at("2026-11-09T11:00:00Z")).0);
    let attested = Some("2026-11-16T06:00:00Z");
    assert!(!list_age(issued, attested, None, true, at("2026-11-22T06:00:00Z")).0);
    assert!(list_age(issued, attested, None, true, at("2026-11-23T07:00:00Z")).0);
    // Read every 6 hours does not make it fresh: an old list served again stays old.
    assert!(
        list_age(
            issued,
            None,
            Some("2026-11-23T06:00:00Z"),
            true,
            at("2026-11-23T07:00:00Z")
        )
        .0
    );
    // Without a freshness key (as before the user makes it): the age of its last reading.
    let read = Some("2026-11-20T00:00:00Z");
    assert!(!list_age(issued, None, read, false, at("2026-11-26T00:00:00Z")).0);
    assert!(list_age(issued, None, read, false, at("2026-11-27T01:00:00Z")).0);
    // Never read: not said old (the store's error says it).
    assert_eq!(
        list_age(None, None, None, false, at("2030-01-01T00:00:00Z")),
        (false, None)
    );
}

/// A mirror of the test: the files of `files` by their path, 404 for any other.
fn mirror(files: Vec<(&'static str, String)>) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for mut s in l.incoming().flatten() {
            let mut first = String::new();
            let mut r = BufReader::new(s.try_clone().unwrap());
            let _ = r.read_line(&mut first);
            loop {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
            }
            let path = first.split_whitespace().nth(1).unwrap_or("").to_string();
            let reply = match files.iter().find(|(p, _)| *p == path) {
                Some((_, body)) => format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                ),
                None => "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                    .to_string(),
            };
            let _ = s.write_all(reply.as_bytes());
        }
    });
    format!("http://127.0.0.1:{port}")
}

fn data(name: &str) -> (DataDir, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "plugins-fresh-{name}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(root.join("store")).unwrap();
    (DataDir::new(root.clone()), root)
}

/// The store reads the attestation beside the copy, keeps it, and reads it again at the next start; one of another list
/// is not taken. Its own process (nextest): the keys of the tests are in its environment.
#[tokio::test(flavor = "multi_thread")]
async fn the_store_takes_the_attestation_of_its_list_beside_the_copy() {
    let v = vectors();
    std::env::set_var("ESCOUADE_STORE_TEST_KEY", s(&v, "/listKey"));
    std::env::set_var("ESCOUADE_FRESHNESS_TEST_KEY", s(&v, "/freshKey"));
    let http = reqwest::Client::new();
    // The store does not answer; its copy serves the list, then the attestation of another list.
    let dead = "http://127.0.0.1:9".to_string();
    let base = mirror(vec![
        ("/revocations.json", s(&v, "/list")),
        ("/revocations.json.minisig", s(&v, "/listSig")),
        ("/revocations-fresh.json", s(&v, "/other/json")),
        ("/revocations-fresh.json.minisig", s(&v, "/other/minisig")),
    ]);
    let (dir, root) = data("other");
    let store = Store::load(&dir, dead.clone(), base, "2.2.6");
    store.refresh(&http, Lang::Fr).await;
    let list = store.view().revocations.unwrap();
    assert_eq!(list.serial, Some(3));
    assert_eq!(list.from.as_deref(), Some("mirror"));
    assert_eq!(
        list.holds_since.as_deref(),
        Some("2026-11-02T10:00:00+00:00"),
        "the attestation of another list is taken"
    );
    assert!(!root.join("store/revocations-fresh.json").exists());
    // Its own: taken, kept, read again at the next start.
    let base = mirror(vec![
        ("/revocations.json", s(&v, "/list")),
        ("/revocations.json.minisig", s(&v, "/listSig")),
        ("/revocations-fresh.json", s(&v, "/good/json")),
        ("/revocations-fresh.json.minisig", s(&v, "/good/minisig")),
    ]);
    let store = Store::load(&dir, dead.clone(), base, "2.2.6");
    store.refresh(&http, Lang::Fr).await;
    let list = store.view().revocations.unwrap();
    assert_eq!(
        list.holds_since.as_deref(),
        Some("2026-11-16T06:00:00+00:00")
    );
    let again = Store::load(&dir, dead, "http://127.0.0.1:9".into(), "2.2.6");
    assert_eq!(
        again.view().revocations.unwrap().holds_since.as_deref(),
        Some("2026-11-16T06:00:00+00:00")
    );
    // An attestation changed on the disk is not taken at the start.
    std::fs::write(
        root.join("store/revocations-fresh.json"),
        s(&v, "/other/json"),
    )
    .unwrap();
    let changed = Store::load(
        &dir,
        "http://127.0.0.1:9".into(),
        "http://127.0.0.1:9".into(),
        "2.2.6",
    );
    assert_eq!(
        changed.view().revocations.unwrap().holds_since.as_deref(),
        Some("2026-11-02T10:00:00+00:00")
    );
    let _ = std::fs::remove_dir_all(root);
}
