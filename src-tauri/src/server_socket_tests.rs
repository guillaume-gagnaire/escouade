//! `escouade-server --socket` (ESC-706, BC-5, `docs/REMOTE.md` « La socket Unix »): the option and what excludes it,
//! refused under Windows; the folder `0700`, the folders above it, the lock, a dead socket replaced and a live one kept,
//! the socket `0600` and removed at the stop; then a server on a socket: `server-status.json` and `status --json` say it,
//! a device pairs by a code (`pair`'s) and connects through it, `devices` lists it and `devices revoke` closes it.

use crate::server::{self, socket, status, Command};

fn args(line: &str) -> Vec<String> {
    line.split_whitespace().map(String::from).collect()
}

#[test]
fn socket_excludes_the_port_and_the_listen_and_the_command_line_wins_over_the_other_s_variable() {
    let none = |_: &str| None;
    let parsed = server::parse_with(&args("--socket /run/user/1000/escouade/server.sock"), none);
    if cfg!(windows) {
        let e = parsed.unwrap_err();
        assert!(e.contains("Windows") && e.contains("--port"), "{e}");
        assert!(server::parse_with(&[], |n| (n == server::SOCKET_VAR)
            .then(|| "/run/escouade-server/server.sock".into()))
        .is_err());
        return;
    }
    let a = parsed.unwrap();
    assert_eq!(a.command, Command::Run);
    assert_eq!(
        a.socket.as_deref(),
        Some(std::path::Path::new("/run/user/1000/escouade/server.sock"))
    );
    // Both on the command line: refused.
    for bad in [
        "--socket /run/x/s.sock --port 4300",
        "--listen 0.0.0.0 --socket /run/x/s.sock",
        // Not absolute, no name, a colon (ssh -L reads it), too long for a socket.
        "--socket relative.sock",
        "--socket /run/x/",
        "--socket /run/x:y/s.sock",
        "--json",
        "health --json",
    ] {
        assert!(server::parse_with(&args(bad), none).is_err(), "{bad}");
    }
    let long = format!("--socket /{}", "a".repeat(120));
    assert!(server::parse_with(&args(&long), none).is_err());
    // The variable, and a flag of the other kind that wins over it.
    let var =
        |n: &str| (n == server::SOCKET_VAR).then(|| "/run/escouade-server/server.sock".into());
    assert!(server::parse_with(&[], var).unwrap().socket.is_some());
    let port = server::parse_with(&args("--port 4300"), var).unwrap();
    assert_eq!((port.socket, port.port), (None, 4300));
    let port_var = |n: &str| (n == server::PORT_VAR).then(|| "4300".into());
    let sock = server::parse_with(&args("--socket /run/x/s.sock"), port_var).unwrap();
    assert!(sock.socket.is_some());
    assert_eq!(sock.port, server::DEFAULT_PORT);
    // Both variables: refused.
    let both = |n: &str| match n {
        server::SOCKET_VAR => Some("/run/x/s.sock".into()),
        server::PORT_VAR => Some("4300".into()),
        _ => None,
    };
    assert!(server::parse_with(&[], both).is_err());
    // `status --json`.
    let s = server::parse_with(&args("status --json"), none).unwrap();
    assert_eq!(s.command, Command::Status);
    assert!(s.json);
}

#[test]
fn a_folder_above_the_socket_is_the_user_s_or_root_s_and_no_one_else_changes_it_but_a_sticky_one_of_root(
) {
    let me = 1000;
    // Ours, or root's, closed to the others.
    assert_eq!(socket::ancestor_refusal(me, 0o40755, me), None);
    assert_eq!(socket::ancestor_refusal(0, 0o40755, me), None);
    assert_eq!(socket::ancestor_refusal(me, 0o40700, me), None);
    // /tmp: root's, sticky.
    assert_eq!(socket::ancestor_refusal(0, 0o41777, me), None);
    // Another user's; one the group or the others may change; sticky but not root's.
    assert!(socket::ancestor_refusal(1001, 0o40755, me).is_some());
    assert!(socket::ancestor_refusal(me, 0o40775, me).is_some());
    assert!(socket::ancestor_refusal(me, 0o40757, me).is_some());
    assert!(socket::ancestor_refusal(0, 0o40777, me).is_some());
    assert!(socket::ancestor_refusal(me, 0o41777, me).is_some());
    // The socket's own folder: ours, nothing for the others.
    assert_eq!(socket::folder_refusal(me, 0o40700, me), None);
    assert!(socket::folder_refusal(me, 0o40750, me).is_some());
    assert!(socket::folder_refusal(me, 0o40705, me).is_some());
    assert!(socket::folder_refusal(0, 0o40700, me).is_some());
}

#[test]
fn status_json_says_whether_it_runs_and_its_socket_and_nothing_else() {
    let none = status::Said::of(None);
    assert_eq!(
        serde_json::to_value(&none).unwrap(),
        serde_json::json!({ "running": false })
    );
    let s = status::Status {
        pid: 42,
        version: "2.3.0".into(),
        api: "/run/user/1000/escouade/server.sock".into(),
        socket: Some("/run/user/1000/escouade/server.sock".into()),
        started_at: 0,
        at: 0,
        busy: Some("turn".into()),
        turns: 3,
        supervisor: Some(status::Supervisor::Systemd),
        update: None,
    };
    assert_eq!(
        serde_json::to_value(status::Said::of(Some(&s))).unwrap(),
        serde_json::json!({
            "running": true,
            "version": "2.3.0",
            "api": "/run/user/1000/escouade/server.sock",
            "socket": "/run/user/1000/escouade/server.sock",
        })
    );
    // An older status, without the socket, still reads.
    let old: status::Status = serde_json::from_value(serde_json::json!({
        "pid": 1, "version": "2.2.0", "api": "127.0.0.1:46480", "startedAt": 0, "at": 0
    }))
    .unwrap();
    assert_eq!(old.socket, None);
}

#[cfg(unix)]
mod unix {
    use super::*;
    use crate::headless_tests::data;
    use crate::remoteapi::devices::Devices;
    use crate::remoteapi::pairing;
    use crate::remoteapi::proto::{self, ToClient, ToServer};
    use crate::remoteapi::server::{Limits, RemoteApi};
    use crate::server::{secrets::FileStore, Endpoint};
    use futures_util::{SinkExt, StreamExt};
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    use std::os::unix::net::UnixListener;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::Message;

    type Ws = tokio_tungstenite::WebSocketStream<tokio::net::UnixStream>;

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    fn mode(p: &Path) -> u32 {
        std::fs::symlink_metadata(p).unwrap().mode() & 0o7777
    }

    fn chmod(p: &Path, m: u32) {
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(m)).unwrap();
    }

    /// A short root of the test's (a socket's path is at most 103 bytes).
    fn root(name: &str) -> PathBuf {
        crate::paths::test_dir(name)
    }

    #[test]
    fn the_folder_is_made_0700_and_one_open_to_others_or_below_a_folder_others_change_is_refused() {
        let r = root("sock-dir");
        let path = r.join("run").join("server.sock");
        let (_l, bound) = socket::bind(&path).unwrap();
        assert_eq!(mode(&r.join("run")), 0o700, "the folder made 0700");
        assert!(std::fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_socket());
        assert_eq!(mode(&path), 0o600, "the socket 0600");
        drop(bound);
        assert!(!path.exists(), "removed at the stop");
        // Its folder opened to the group: refused, left as it is.
        chmod(&r.join("run"), 0o750);
        let e = socket::bind(&path).unwrap_err().to_string();
        assert!(
            e.contains("chmod 700") || e.contains("open to others"),
            "{e}"
        );
        assert_eq!(mode(&r.join("run")), 0o750);
        chmod(&r.join("run"), 0o700);
        // A folder above it that others may change (even sticky, not root's): refused, nothing made below it.
        let open = r.join("open");
        std::fs::create_dir(&open).unwrap();
        chmod(&open, 0o1777);
        let below = open.join("escouade").join("server.sock");
        let e = socket::bind(&below).unwrap_err().to_string();
        assert!(e.contains(&open.display().to_string()), "{e}");
        assert!(!open.join("escouade").exists(), "nothing made in it");
        chmod(&open, 0o755);
        // A link in place of the folder: refused.
        let real = r.join("real");
        std::fs::create_dir(&real).unwrap();
        chmod(&real, 0o700);
        std::os::unix::fs::symlink(&real, r.join("link")).unwrap();
        assert!(socket::bind(&r.join("link").join("s.sock")).is_err());
        // Something else at the socket's path: never removed.
        let file = r.join("run").join("file.sock");
        std::fs::write(&file, "x").unwrap();
        assert!(socket::bind(&file).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "x");
    }

    #[test]
    fn the_lock_keeps_a_second_server_out_a_dead_socket_is_replaced_and_a_live_one_kept() {
        let r = root("sock-lock");
        let path = r.join("run").join("server.sock");
        let (_l, first) = socket::bind(&path).unwrap();
        // Its lock held: another server is refused, the socket left.
        let e = socket::bind(&path).unwrap_err().to_string();
        assert!(
            e.contains("un autre serveur écoute sur") || e.contains("another server listens on"),
            "{e}"
        );
        assert!(path.exists());
        drop(first);
        drop(_l);
        // A dead socket (its server killed: the file left, nothing answers, no lock): replaced.
        let dead = UnixListener::bind(&path).unwrap();
        drop(dead);
        assert!(path.exists());
        let (_l2, second) = socket::bind(&path).unwrap();
        assert_eq!(mode(&path), 0o600);
        drop(second);
        drop(_l2);
        // A live one without the lock (another program answers there): kept.
        let live = UnixListener::bind(&path).unwrap();
        let e = socket::bind(&path).unwrap_err().to_string();
        assert!(
            e.contains("un autre serveur écoute sur") || e.contains("another server listens on"),
            "{e}"
        );
        assert!(std::fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_socket());
        drop(live);
    }

    async fn open(path: &Path) -> Ws {
        let stream = tokio::net::UnixStream::connect(path).await.unwrap();
        let url = format!("ws://127.0.0.1{}", proto::PATH);
        tokio_tungstenite::client_async(url, stream)
            .await
            .unwrap()
            .0
    }

    async fn send(ws: &mut Ws, m: &ToServer) {
        ws.send(Message::text(serde_json::to_string(m).unwrap()))
            .await
            .unwrap();
    }

    async fn next(ws: &mut Ws) -> Option<ToClient> {
        loop {
            match tokio::time::timeout(Duration::from_secs(60), ws.next()).await {
                Ok(Some(Ok(Message::Text(t)))) => return serde_json::from_str(&t).ok(),
                Ok(Some(Ok(Message::Close(_)))) | Ok(None) | Ok(Some(Err(_))) => return None,
                Ok(Some(Ok(_))) => continue,
                Err(_) => panic!("nothing came in 60 s"),
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_server_on_a_socket_says_it_pairs_a_device_through_it_and_devices_follows() {
        let (_dir, data) = data("server-socket");
        let r = root("sock-serve");
        let path = r.join("escouade").join("server.sock");
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Arc<RemoteApi>>();
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        let served = data.clone();
        let endpoint = Endpoint {
            socket: Some(path.clone()),
            ..Endpoint::default()
        };
        let task = tokio::spawn(async move {
            server::serve(
                served,
                endpoint,
                Limits::default(),
                move |api| {
                    let _ = ready_tx.send(api.clone());
                },
                async move {
                    let _ = stop_rx.await;
                    "SIGTERM"
                },
            )
            .await
        });
        let api = ready_rx.await.expect("the server is ready");
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(&r.join("escouade")), 0o700);
        // server-status.json says the socket, and `status --json` with it.
        let mut seen = None;
        for _ in 0..100 {
            if let Some(s) = status::read(&data) {
                seen = Some(s);
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let s = seen.expect("the status is written");
        let shown = path.display().to_string();
        assert_eq!(s.socket.as_deref(), Some(shown.as_str()));
        assert_eq!(s.api, shown);
        assert!(status::healthy(&s, crate::model::now_ms(), status::alive).is_ok());
        assert!(status::socket_there(&path).is_ok());
        let said = status::Said::of(Some(&s));
        assert_eq!(said.socket.as_deref(), Some(shown.as_str()));
        // The commands beside it follow it: health, status, pair, devices, phone (each exit code 0), with the server on its
        // socket and no port anywhere.
        let folder = data.root().display().to_string();
        for line in [
            "health",
            "status",
            "status --json",
            "pair",
            "devices",
            "phone",
            "phone status",
        ] {
            let mut args: Vec<String> = line.split(' ').map(String::from).collect();
            args.extend(["--data-dir".to_string(), folder.clone()]);
            let code = tokio::task::spawn_blocking(move || server::main(args))
                .await
                .unwrap();
            assert_eq!(code, 0, "escouade-server {line} on a server of a socket");
        }
        // The socket gone (another program removed it): health says it is not healthy.
        std::fs::rename(&path, r.join("moved.sock")).unwrap();
        let args = vec!["health".to_string(), "--data-dir".into(), folder.clone()];
        let code = tokio::task::spawn_blocking(move || server::main(args))
            .await
            .unwrap();
        assert_eq!(code, 1, "health without its socket");
        assert!(status::socket_there(&path).is_err());
        std::fs::rename(r.join("moved.sock"), &path).unwrap();
        // A second server on the same socket (another data folder): refused, the first one untouched.
        let (_dir2, data2) = crate::headless_tests::data("server-socket-2");
        let refused = server::serve(
            data2,
            Endpoint {
                socket: Some(path.clone()),
                ..Endpoint::default()
            },
            Limits::default(),
            |_| {},
            async { "SIGTERM" },
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(
            refused.contains("un autre serveur écoute sur")
                || refused.contains("another server listens on"),
            "{refused}"
        );
        // `pair` (a code of the folder), given through the socket: the device's token.
        let code = pairing::create(&data, crate::model::now_ms()).unwrap();
        let mut ws = open(&path).await;
        send(
            &mut ws,
            &ToServer::Pair {
                code: code.text.clone(),
                name: "Poste".into(),
                key: None,
            },
        )
        .await;
        let token = match next(&mut ws).await.expect("an answer to the pairing") {
            ToClient::Paired { token, .. } => token,
            other => panic!("{other:?}"),
        };
        // Its handshake through the socket.
        let mut ws = open(&path).await;
        send(
            &mut ws,
            &ToServer::Hello {
                version: VERSION.into(),
                token: token.clone(),
                session: None,
            },
        )
        .await;
        assert!(
            matches!(next(&mut ws).await, Some(ToClient::Welcome { .. })),
            "welcomed through the socket"
        );
        // `devices`, read from the folder as the command does, lists it; `devices revoke` closes its connection.
        let store = Arc::new(FileStore::open(&data).unwrap());
        let devices = Devices::load(&data, store);
        let listed = devices.list();
        let device = listed
            .iter()
            .find(|d| d.name == "Poste")
            .expect("the device listed")
            .id
            .clone();
        assert!(devices.revoke(&device).unwrap());
        let mut closed = false;
        for _ in 0..200 {
            match tokio::time::timeout(Duration::from_millis(100), ws.next()).await {
                Ok(Some(Ok(Message::Text(_)))) | Ok(Some(Ok(Message::Ping(_)))) | Err(_) => {}
                _ => {
                    closed = true;
                    break;
                }
            }
        }
        assert!(closed, "the revoked device's connection is closed");
        drop(api);
        stop_tx.send(()).unwrap();
        task.await.unwrap().unwrap();
        assert!(!path.exists(), "the socket removed at the clean stop");
        assert!(status::read(&data).is_none());
    }
}
