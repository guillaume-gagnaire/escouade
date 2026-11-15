//! The team server's client in the core (TS-3): the addresses and links it reads, the QR code, and two data folders
//! of one machine that share its keychain (the app's `SecretStore`) against the real `escouade-team-server`: one
//! creates the team and invites, the other joins, the first one's watch lets it in.

use std::sync::Arc;
use std::time::Duration;

use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::teams::{address, deep_link, is_loopback, parse_link, qr_svg, Teams, FORK};

#[test]
fn an_address_is_https_but_on_this_machine() {
    assert_eq!(
        address(" team.acme.fr/ ").unwrap(),
        ("https://team.acme.fr".into(), "team.acme.fr".into())
    );
    assert_eq!(
        address("https://team.acme.fr:8443").unwrap(),
        (
            "https://team.acme.fr:8443".into(),
            "team.acme.fr:8443".into()
        )
    );
    assert_eq!(
        address("127.0.0.1:4121").unwrap(),
        ("http://127.0.0.1:4121".into(), "127.0.0.1:4121".into())
    );
    assert_eq!(
        address("http://localhost:4121").unwrap().0,
        "http://localhost:4121"
    );
    assert_eq!(
        address("https://127.0.0.1:4121").unwrap().0,
        "https://127.0.0.1:4121"
    );
    for bad in [
        "",
        "http://team.acme.fr",
        "team.acme.fr/join/x",
        "ftp://x",
        "https://u@team.acme.fr",
    ] {
        assert!(address(bad).is_err(), "{bad}");
    }
    assert!(is_loopback("[::1]:80") && is_loopback("::1") && is_loopback("LOCALHOST"));
    assert!(!is_loopback("team.acme.fr") && !is_loopback("10.0.0.1:80"));
}

#[test]
fn a_link_is_read_pasted_or_opened_and_shown_as_a_qr_code() {
    let url = "https://team.acme.fr/join/00112233445566778899aabbccddeeff#s=AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI&t=AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM";
    let link = parse_link(url).unwrap();
    assert_eq!(link.server, "team.acme.fr");
    // Pasted with a break in it, or opened by the server's page.
    assert_eq!(
        parse_link(&format!(" {}\n{} ", &url[..40], &url[40..])).unwrap(),
        link
    );
    let deep = deep_link(&link);
    assert!(
        deep.starts_with("escouade://team/join/team.acme.fr/00112233445566778899aabbccddeeff#s=")
    );
    assert_eq!(parse_link(&deep).unwrap(), link);
    // Without its secret: not a link.
    assert!(parse_link(url.split('#').next().unwrap()).is_err());
    let svg = qr_svg(url);
    assert!(svg.contains("<svg") && svg.contains("#000000"));
}

async fn server() -> (
    Arc<escouade_team_server::Server>,
    String,
    tokio::task::JoinHandle<()>,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let config = escouade_team_server::config::Config::in_folder(dir.path().join("server"));
    let server = Arc::new(escouade_team_server::Server::open(config).await.unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let s = server.clone();
    let task = tokio::spawn(async move {
        let _ = s.serve(listener, None, std::future::pending()).await;
    });
    (server, format!("127.0.0.1:{}", addr.port()), task, dir)
}

fn teams(root: &std::path::Path, store: Arc<dyn SecretStore>) -> Arc<Teams> {
    Arc::new(Teams::open(
        root.to_path_buf(),
        store,
        Arc::new(crate::model::Settings::default),
        Arc::new(|| {}),
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_data_folders_of_one_machine_a_team_an_invitation_and_its_admission() {
    let (server, host, task, dir) = server().await;
    // One keychain for both, as on a machine: each data folder's names apart.
    let store = Arc::new(MemoryStore::default());
    let alice = teams(&dir.path().join("a"), store.clone());
    let bob = teams(&dir.path().join("b"), store.clone());

    let token = server.new_team_token().await.unwrap();
    let created = alice.create(&host, &token, "ACME", None).await.unwrap();
    assert_eq!(created.words.len(), 24);
    assert_eq!(created.team.fingerprint.len(), 6);
    assert_eq!(created.team.role, "admin");
    assert_eq!(created.team.admins, 1);
    assert!(!created.team.recovery_confirmed && created.team.words_pending);
    let id = created.team.id.clone();
    // Three words typed back: a wrong one is refused, the right ones confirm and the words are let go.
    let w = &created.words;
    assert!(!alice
        .confirm_words(
            &id,
            &[(0, w[0].clone()), (5, "x".into()), (9, w[9].clone())]
        )
        .unwrap());
    assert!(alice
        .confirm_words(
            &id,
            &[(0, w[0].clone()), (5, w[5].clone()), (9, w[9].clone())]
        )
        .unwrap());
    let listed = alice.list().await;
    assert!(listed[0].recovery_confirmed && !listed[0].words_pending);
    // The keys are in the keychain, under the folder's prefix.
    let names: Vec<String> = store.all().into_keys().collect();
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(&format!("escouade-team.{id}.keys"))),
        "{names:?}"
    );

    // The invitation, 72 h, ready at once; Bob's folder joins by its link, Alice's watch lets him in.
    let invite = alice.invite(&id, "member", None).await.unwrap();
    assert!(invite.ready && invite.qr.contains("<svg"));
    assert!(invite
        .deep_link
        .starts_with(&format!("escouade://team/join/{host}/")));
    let preview = bob.join_preview(&invite.deep_link).unwrap();
    assert_eq!(preview.fingerprint, created.team.fingerprint);
    assert!(!preview.own_device);
    let waiting = bob.join(&invite.url, "").await.unwrap();
    assert!(!waiting.joined);
    assert_eq!(waiting.name, host, "named after its server when not named");
    let watch = tokio::spawn(alice.clone().watch(Duration::from_millis(200)));
    let mut joined = None;
    for _ in 0..100 {
        let v = bob.sync(&id).await.unwrap();
        if v.joined {
            joined = Some(v);
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let joined = joined.expect("Bob is let in by Alice's watch");
    assert_eq!(joined.role, "member");
    assert_eq!(joined.fingerprint, created.team.fingerprint);
    // Two devices of one keychain, each its own keys.
    let keys = store
        .all()
        .into_keys()
        .filter(|n| n.ends_with(&format!("escouade-team.{id}.keys")))
        .count();
    assert_eq!(keys, 2);
    // Used: a third folder is told.
    let carol = teams(&dir.path().join("c"), store.clone());
    let refused = carol.join(&invite.url, "ACME").await.unwrap_err();
    assert!(refused.contains("déjà servi"), "{refused}");
    // Bob, a member, cannot invite a new member.
    assert!(bob
        .invite(&id, "member", None)
        .await
        .unwrap_err()
        .contains("administrateur"));

    // The folder opened again finds the team; forgotten, its keys leave the keychain.
    watch.abort();
    let again = teams(&dir.path().join("b"), store.clone());
    assert!(again.list().await[0].joined);
    drop(bob);
    again.forget(&id).await.unwrap();
    assert!(again.list().await.is_empty());
    assert_eq!(
        store
            .all()
            .into_keys()
            .filter(|n| n.ends_with(&format!("escouade-team.{id}.keys")))
            .count(),
        1
    );
    assert!(FORK.contains("deux versions"));
    task.abort();
}

#[test]
fn the_status_says_offline_and_the_changes_waiting() {
    use crate::teams::SyncStatusView;
    assert_eq!(
        SyncStatusView::of(Some(false), 3, 1).label,
        "Hors ligne · 3 changements en attente"
    );
    assert_eq!(
        SyncStatusView::of(Some(false), 1, 1).label,
        "Hors ligne · 1 changement en attente"
    );
    assert_eq!(SyncStatusView::of(Some(false), 0, 1).label, "Hors ligne");
    assert_eq!(
        SyncStatusView::of(Some(true), 2, 1).label,
        "2 changements en attente"
    );
    assert_eq!(SyncStatusView::of(Some(true), 0, 1).label, "");
    assert_eq!(SyncStatusView::of(None, 0, 0).label, "");
}

/// TS-5: a project's operation reaches the other data folder by the live feed, not by the watch (60 s here).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_projects_operation_comes_by_the_live_feed() {
    let (server, host, task, dir) = server().await;
    let store = Arc::new(MemoryStore::default());
    let alice = teams(&dir.path().join("a"), store.clone());
    let bob = teams(&dir.path().join("b"), store.clone());
    let token = server.new_team_token().await.unwrap();
    let id = alice
        .create(&host, &token, "ACME", None)
        .await
        .unwrap()
        .team
        .id;
    let invite = alice.invite(&id, "member", None).await.unwrap();
    bob.join(&invite.url, "").await.unwrap();
    alice.sync(&id).await.unwrap();
    assert!(bob.sync(&id).await.unwrap().joined);

    // A project of the whole team, followed by Bob's folder.
    let project = {
        let team = alice.team(&id).unwrap();
        let mut t = team.lock().await;
        let p = t.create_project(None).await.unwrap();
        t.sync_project(p).await.unwrap();
        p
    };
    {
        let team = bob.team(&id).unwrap();
        let mut t = team.lock().await;
        t.follow(project).unwrap();
        t.sync_project(project).await.unwrap();
    }
    let watch_a = tokio::spawn(alice.clone().watch(Duration::from_secs(60)));
    let watch_b = tokio::spawn(bob.clone().watch(Duration::from_secs(60)));
    // Both watches past their first round: the live feeds are up.
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let ticket = {
        let team = alice.team(&id).unwrap();
        let mut t = team.lock().await;
        let ticket = t
            .create_ticket(
                project,
                [("title".to_string(), serde_json::json!("En direct"))]
                    .into_iter()
                    .collect(),
            )
            .unwrap();
        t.flush_project(project).await.unwrap();
        ticket
    };
    let status = alice.status(&id).await.unwrap();
    assert_eq!(
        (status.online, status.pending, status.label.as_str()),
        (Some(true), 0, "")
    );
    let mut seen = false;
    for _ in 0..50 {
        {
            let team = bob.team(&id).unwrap();
            let mut t = team.lock().await;
            if t.replica(&project)
                .unwrap()
                .object(&ticket)
                .unwrap()
                .is_some()
            {
                seen = true;
            }
        }
        if seen {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(seen, "the operation came by the live feed within 5 s");
    watch_a.abort();
    watch_b.abort();
    bob.forget(&id).await.unwrap();
    task.abort();
}
