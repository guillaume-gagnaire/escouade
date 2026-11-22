//! TS-14 against the real `escouade-team-server`: the team's private store. An administrator declares the team's key
//! and its namespace; a member proposes a version of a plugin of the namespace, its archive encrypted on the server;
//! the administrator publishes it only when their own checks find the same digest, signed by the team's key; a member
//! reads it and opens its files. Refused: a namespace that is Escouade's own, a plugin outside the namespace, a
//! signature by another key, a member who writes what an administrator alone writes, a revocation list whose serial
//! goes back. The backup key revokes the key of every day. The server reads no id of a plugin, nor its archive.
//! Every key is drawn by the test: none is in the repository.

mod common;

use common::*;
use escouade_team::crypto::Role;
use escouade_team::plugins::{self, Entry, Refused, Store, Submission, PLUGINS_STREAM};
use escouade_team::signing::SigningKey;
use escouade_team::{Error, Team};
use sha2::{Digest, Sha256};

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

async fn sync(t: &mut Team) -> Store {
    plugins::sync(t).await.expect("the store is read")
}

/// Alice (administrator, the owner) and Bob (a member), with the team's key of every day and its backup key.
async fn two(w: &World) -> (Team, Team, SigningKey, SigningKey) {
    let (pa, pb) = (w.place(), w.place());
    let mut alice = w.create(&pa, member(1)).await;
    let bob = w.join(&mut alice, &pb, member(2), Role::Member).await;
    let (daily, backup) = (
        SigningKey::generate().unwrap(),
        SigningKey::generate().unwrap(),
    );
    (alice, bob, daily, backup)
}

/// A proposal of `plugin`@`version`, its archive `archive` (no module: a declarative plugin).
fn proposal(plugin: &str, version: &str, archive: &[u8]) -> Submission {
    let archive_sha256 = sha(archive);
    Submission {
        plugin: plugin.into(),
        version: version.into(),
        tag: format!("v{version}"),
        name: serde_json::json!("Règles"),
        description: serde_json::Value::Null,
        archive: [0; 16],
        module: None,
        digest: sha(format!("{archive_sha256}-").as_bytes()),
        archive_sha256,
        module_sha256: None,
        checks: vec![],
    }
}

fn signer(key: &SigningKey) -> impl Fn(&[u8], &str) -> Result<String, Error> + '_ {
    move |bytes, comment| key.sign(bytes, comment)
}

/// The revocation list of serial `serial`, signed by `key`.
fn list(
    key: &SigningKey,
    serial: u64,
    entries: serde_json::Value,
    revoked: &[String],
) -> (Vec<u8>, String) {
    let mut json = serde_json::json!({
        "serial": serial,
        "issued_at": "2026-11-21T10:00:00Z",
        "entries": entries,
    });
    if !revoked.is_empty() {
        json["keys_revoked"] = serde_json::json!(revoked);
    }
    let bytes = serde_json::to_vec(&json).unwrap();
    let sig = key
        .sign(&bytes, &format!("escouade-revocations-v1 {serial} fr.acme"))
        .unwrap();
    (bytes, sig)
}

fn server_bytes(w: &World) -> Vec<u8> {
    fn walk(dir: &std::path::Path, out: &mut Vec<u8>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if let Ok(b) = std::fs::read(&p) {
                out.extend(b);
            }
        }
    }
    let mut out = Vec::new();
    walk(&w.server.folder, &mut out);
    out
}

fn holds(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

async fn an_administrator_publishes_what_a_member_proposed(backend: Backend) {
    let w = World::on(&backend).await;
    let (mut alice, mut bob, daily, backup) = two(&w).await;
    alice.serve(NOW).await.unwrap();

    // Before the key, nothing can be proposed.
    let archive = b"l'archive du plugin fr.acme.rules, chiffree".to_vec();
    let e = plugins::submit(
        &mut bob,
        proposal("fr.acme.rules", "1.0.0", &archive),
        &archive,
        None,
        NOW,
    )
    .await
    .unwrap_err();
    assert!(e.to_string().contains("no key"), "{e}");

    // The key and its namespace, declared by an administrator alone; Escouade's own namespace never.
    let e = plugins::declare(&mut bob, vec![daily.public()], "fr.acme.*", NOW)
        .await
        .unwrap_err();
    assert!(matches!(e, Error::Crypto(_)), "{e}");
    for ns in ["dev.escouade.*", "dev.*", "acme"] {
        assert!(
            plugins::declare(&mut alice, vec![daily.public()], ns, NOW)
                .await
                .is_err(),
            "{ns}"
        );
    }
    plugins::declare(
        &mut alice,
        vec![daily.public(), backup.public()],
        "fr.acme.*",
        NOW,
    )
    .await
    .unwrap();
    let store = sync(&mut bob).await;
    let key = store.key.clone().expect("Bob reads the team's key");
    assert_eq!(key.namespace, "fr.acme.*");
    assert_eq!(key.ids, vec![daily.id(), backup.id()]);
    assert_eq!(key.by, member(1).to_string());

    // Bob proposes: outside the namespace, refused before anything leaves.
    let e = plugins::submit(
        &mut bob,
        proposal("com.other.rules", "1.0.0", &archive),
        &archive,
        None,
        NOW,
    )
    .await
    .unwrap_err();
    assert!(e.to_string().contains("namespace"), "{e}");
    let e = plugins::submit(
        &mut bob,
        proposal("dev.escouade.rules", "1.0.0", &archive),
        &archive,
        None,
        NOW,
    )
    .await
    .unwrap_err();
    assert!(e.to_string().contains("namespace"), "{e}");
    let wanted = proposal("fr.acme.rules", "1.0.0", &archive);
    let (id, store) = plugins::submit(&mut bob, wanted.clone(), &archive, None, NOW)
        .await
        .unwrap();
    assert_eq!(store.proposed.len(), 1);
    assert_eq!(store.proposed[0].id, id);

    // Alice's checks found another digest: nothing is signed.
    let e = plugins::publish(&mut alice, &id, &"f".repeat(64), &signer(&daily), NOW)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("digests differ"), "{e}");
    // Signed by a key the team did not declare: nothing is published.
    let stranger = SigningKey::generate().unwrap();
    let e = plugins::publish(&mut alice, &id, &wanted.digest, &signer(&stranger), NOW)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("declared key"), "{e}");
    // Bob may not publish.
    assert!(
        plugins::publish(&mut bob, &id, &wanted.digest, &signer(&daily), NOW)
            .await
            .is_err()
    );
    let store = plugins::publish(&mut alice, &id, &wanted.digest, &signer(&daily), NOW)
        .await
        .unwrap();
    assert!(store.proposed.is_empty(), "the proposal is done");

    let store = sync(&mut bob).await;
    let p = store
        .version("fr.acme.rules", "1.0.0")
        .expect("Bob reads the version");
    assert_eq!(p.signer, daily.id());
    assert_eq!(p.statement.namespace, "fr.acme");
    assert_eq!(p.statement.digest, wanted.digest);
    assert_eq!(p.statement.by, member(1).to_string());
    assert_eq!(store.latest().len(), 1);
    let (got, module) = plugins::files(
        &mut bob,
        p.archive,
        &p.statement.archive_sha256,
        p.module,
        p.statement.module_sha256.as_deref(),
    )
    .await
    .unwrap();
    assert_eq!(got, archive, "Bob opens the archive");
    assert_eq!(module, None);
    assert!(store.refusals.is_empty(), "{:?}", store.refusals);

    // The server reads neither the plugin's id nor its archive.
    if matches!(backend, Backend::Sqlite) {
        let disk = server_bytes(&w);
        assert!(!holds(&disk, b"fr.acme.rules"));
        assert!(!holds(&disk, &archive));
        assert!(!holds(&disk, b"fr.acme.*"));
    }
    w.finish().await;
}

async fn what_an_administrator_alone_writes_is_refused_from_a_member(backend: Backend) {
    let w = World::on(&backend).await;
    let (mut alice, mut bob, daily, _) = two(&w).await;
    alice.serve(NOW).await.unwrap();
    plugins::declare(&mut alice, vec![daily.public()], "fr.acme.*", NOW)
        .await
        .unwrap();
    // Bob slips in a key of his own and a version signed by it: every device refuses both at the reading.
    let mine = SigningKey::generate().unwrap();
    let key = plugins::seal(
        &bob,
        &Entry::Key {
            keys: vec![mine.public()],
            namespace: "fr.acme.*".into(),
            at: NOW,
        },
    )
    .unwrap();
    bob.client()
        .append_project(&PLUGINS_STREAM.0, &key)
        .await
        .unwrap();
    let statement = plugins::Statement {
        format: 1,
        id: "fr.acme.evil".into(),
        version: "1.0.0".into(),
        tag: "v1.0.0".into(),
        name: serde_json::Value::Null,
        description: serde_json::Value::Null,
        namespace: "fr.acme".into(),
        team: hex::encode(alice.fingerprint().unwrap().0),
        archive_sha256: "0".repeat(64),
        module_sha256: None,
        digest: "1".repeat(64),
        signed_at: NOW,
        by: member(2).to_string(),
    };
    let bytes = statement.bytes().unwrap();
    let forged = plugins::seal(
        &alice,
        &Entry::Publish {
            submission: [9; 16],
            minisig: mine.sign(&bytes, &statement.comment()).unwrap(),
            statement: bytes.clone(),
            archive: [1; 16],
            module: None,
            at: NOW,
        },
    )
    .unwrap();
    alice
        .client()
        .append_project(&PLUGINS_STREAM.0, &forged)
        .await
        .unwrap();
    // An administrator's version, signed by the team's key but outside its namespace.
    let mut outside = statement.clone();
    outside.id = "com.other.evil".into();
    let obytes = outside.bytes().unwrap();
    let out = plugins::seal(
        &alice,
        &Entry::Publish {
            submission: [8; 16],
            minisig: daily.sign(&obytes, &outside.comment()).unwrap(),
            statement: obytes,
            archive: [1; 16],
            module: None,
            at: NOW,
        },
    )
    .unwrap();
    alice
        .client()
        .append_project(&PLUGINS_STREAM.0, &out)
        .await
        .unwrap();

    for t in [&mut alice, &mut bob] {
        let store = sync(t).await;
        assert_eq!(
            store.key.as_ref().unwrap().ids,
            vec![daily.id()],
            "the team's key stays"
        );
        assert!(store.published.is_empty());
        let reasons: Vec<&Refused> = store.refusals.iter().map(|r| &r.reason).collect();
        assert_eq!(reasons.len(), 3, "{reasons:?}");
        assert_eq!(reasons[0], &Refused::NotAdmin);
        assert_eq!(reasons[1], &Refused::Signature);
        assert!(matches!(reasons[2], Refused::OutsideNamespace { id } if id == "com.other.evil"));
    }
    w.finish().await;
}

async fn the_revocation_list_never_goes_back_and_the_backup_key_takes_over(backend: Backend) {
    let w = World::on(&backend).await;
    let (mut alice, mut bob, daily, backup) = two(&w).await;
    alice.serve(NOW).await.unwrap();
    plugins::declare(
        &mut alice,
        vec![daily.public(), backup.public()],
        "fr.acme.*",
        NOW,
    )
    .await
    .unwrap();
    let entry = serde_json::json!([{
        "id": "fr.acme.rules", "versions": "*", "action": "disable",
        "reason": { "fr": "Fuite", "en": "Leak" }, "date": "2026-11-21"
    }]);
    let (json, sig) = list(&daily, 1, entry.clone(), &[]);
    plugins::revoke(&mut alice, json.clone(), sig, NOW)
        .await
        .unwrap();
    assert_eq!(sync(&mut bob).await.revocations.unwrap().serial, 1);
    // The same serial again, or one below: refused.
    let (j, s) = list(&daily, 1, serde_json::json!([]), &[]);
    let e = plugins::revoke(&mut alice, j, s, NOW).await.unwrap_err();
    assert!(e.to_string().contains("does not go past"), "{e}");
    // Signed by a key the team did not declare.
    let stranger = SigningKey::generate().unwrap();
    let (j, s) = list(&stranger, 5, serde_json::json!([]), &[]);
    assert!(plugins::revoke(&mut alice, j, s, NOW).await.is_err());
    // Bob may not write it, even signed.
    let (j, s) = list(&daily, 6, serde_json::json!([]), &[]);
    assert!(plugins::revoke(&mut bob, j, s, NOW).await.is_err());

    // A list below the last, slipped in by the server's hand (an administrator's old entry played again): refused at
    // the reading, the list taken stays.
    let (j2, s2) = list(&daily, 2, entry.clone(), &[]);
    plugins::revoke(&mut alice, j2, s2, NOW).await.unwrap();
    let (j1, s1) = list(&daily, 1, serde_json::json!([]), &[]);
    let old = plugins::seal(
        &alice,
        &Entry::Revocations {
            json: j1,
            minisig: s1,
            at: NOW,
        },
    )
    .unwrap();
    alice
        .client()
        .append_project(&PLUGINS_STREAM.0, &old)
        .await
        .unwrap();
    let store = sync(&mut bob).await;
    assert_eq!(store.revocations.as_ref().unwrap().serial, 2);
    assert!(matches!(
        store.refusals.last().unwrap().reason,
        Refused::Older { serial: 1, seen: 2 }
    ));

    // The key of every day is lost: the backup key revokes it, and only the backup key signs from then on.
    let (j3, s3) = list(&backup, 3, entry, &[daily.id()]);
    plugins::revoke(&mut alice, j3, s3, NOW).await.unwrap();
    let store = sync(&mut bob).await;
    assert_eq!(store.trusted_keys(), vec![backup.public()]);
    let archive = b"fr.acme.rules 1.1.0".to_vec();
    let wanted = proposal("fr.acme.rules", "1.1.0", &archive);
    let (id, _) = plugins::submit(&mut bob, wanted.clone(), &archive, None, NOW)
        .await
        .unwrap();
    let e = plugins::publish(&mut alice, &id, &wanted.digest, &signer(&daily), NOW)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("declared key"), "{e}");
    plugins::publish(&mut alice, &id, &wanted.digest, &signer(&backup), NOW)
        .await
        .unwrap();
    let store = sync(&mut bob).await;
    assert_eq!(
        store.version("fr.acme.rules", "1.1.0").unwrap().signer,
        backup.id()
    );
    w.finish().await;
}

async fn a_proposal_is_set_aside_by_its_author_or_an_administrator(backend: Backend) {
    let w = World::on(&backend).await;
    let (mut alice, mut bob, daily, _) = two(&w).await;
    alice.serve(NOW).await.unwrap();
    plugins::declare(&mut alice, vec![daily.public()], "fr.acme.*", NOW)
        .await
        .unwrap();
    let a = b"one".to_vec();
    let (one, _) = plugins::submit(
        &mut alice,
        proposal("fr.acme.one", "1.0.0", &a),
        &a,
        None,
        NOW,
    )
    .await
    .unwrap();
    let (two, _) = plugins::submit(
        &mut bob,
        proposal("fr.acme.two", "1.0.0", &a),
        &a,
        None,
        NOW,
    )
    .await
    .unwrap();
    // Bob may not set aside Alice's; his own, yes; Alice, any.
    plugins::dismiss(&mut bob, &one, "non", NOW).await.unwrap();
    assert_eq!(sync(&mut bob).await.proposed.len(), 2);
    plugins::dismiss(&mut bob, &two, "retirée", NOW)
        .await
        .unwrap();
    plugins::dismiss(&mut alice, &one, "refusée", NOW)
        .await
        .unwrap();
    let store = sync(&mut bob).await;
    assert!(store.proposed.is_empty());
    assert_eq!(store.refusals.len(), 1);
    assert_eq!(store.refusals[0].reason, Refused::NotAdmin);
    w.finish().await;
}

both!(
    an_administrator_publishes_what_a_member_proposed,
    what_an_administrator_alone_writes_is_refused_from_a_member,
    the_revocation_list_never_goes_back_and_the_backup_key_takes_over,
    a_proposal_is_set_aside_by_its_author_or_an_administrator,
);
