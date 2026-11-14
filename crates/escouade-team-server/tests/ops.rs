//! What the operator sees: the binary's `init` and `serve`, TLS, the base in WAL, the metrics and the logs without
//! any id.

#[macro_use]
mod common;

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::Duration;

use common::*;
use escouade_team_server::db::sqlite::Sqlite;
use escouade_team_server::db::Kind;
use escouade_team_server::util::random;
use escouade_team_server::Server;
use reqwest::{Method, StatusCode};

#[tokio::test]
async fn the_base_is_in_wal() {
    let dir = tempfile::tempdir().unwrap();
    let base = Sqlite::open(&dir.path().join("team.db")).unwrap();
    assert_eq!(base.journal_mode().unwrap(), "wal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_metrics_say_no_id() {
    let s = TestServer::start(&Backend::Sqlite).await;
    let alice = s.new_team().await;
    let bob = s.add_device(&alice).await;
    let project = random::<16>();
    let file = random::<16>();
    alice.message(b"m").await;
    alice.commit(0, b"c").await.ok();
    bob.commit(0, b"c").await.refused();
    bob.append_project(&project, b"op").await.ok();
    assert_eq!(bob.put_file(&file, vec![1; 100]).await, StatusCode::CREATED);
    bob.get_file(&file).await;
    alice.revoke(&bob).await;
    alice
        .client
        .status(
            Method::GET,
            &format!("/v1/files/{}", hex::encode(random::<16>())),
            None,
        )
        .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = s.server.metrics_router();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let text = reqwest::get(format!("http://{addr}/metrics"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    for needle in [
        "/v1/projects/{project}/stream",
        "/v1/files/{file}",
        "/v1/devices/{device}/revoke",
    ] {
        assert!(text.contains(needle), "{needle} in\n{text}");
    }
    assert!(text.contains("escouade_team_epoch_conflicts_total 1"));
    assert!(text.contains("escouade_team_teams 1"));
    assert!(text.contains("escouade_team_devices 2"));
    assert!(text.contains("escouade_team_revoked_devices 1"));
    assert!(text.contains("escouade_team_files_written_total 1"));
    // No id of a team, a device, a project or a file, in hex or otherwise.
    for id in [
        &alice.team,
        &alice.id,
        &bob.id,
        &project.to_vec(),
        &file.to_vec(),
    ] {
        assert!(!text.contains(&hex::encode(id)), "an id in the metrics");
    }
    assert!(!text.contains(alice.client.token.as_ref().unwrap()));
    // The exposed address does not serve them.
    assert_eq!(
        s.client().status(Method::GET, "/metrics", None).await.0,
        StatusCode::NOT_FOUND
    );
    s.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tls_with_a_given_certificate() {
    let dir = tempfile::tempdir().unwrap();
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let (cert_path, key_path) = (dir.path().join("cert.pem"), dir.path().join("key.pem"));
    std::fs::write(&cert_path, cert.cert.pem()).unwrap();
    std::fs::write(&key_path, cert.signing_key.serialize_pem()).unwrap();
    let tls = escouade_team_server::transport::listen::tls_config(&cert_path, &key_path).unwrap();

    let data = tempfile::tempdir().unwrap();
    let (config, _) = config(&Backend::Sqlite, data.path());
    let server = std::sync::Arc::new(Server::open(config).await.unwrap());
    let s = TestServer::serve(server, data, None, Some(tls)).await;
    let port = s.addr.port();
    let http = reqwest::Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(cert.cert.pem().as_bytes()).unwrap())
        .build()
        .unwrap();
    let r = http
        .get(format!("https://localhost:{port}/healthz"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    // A device enrols and signs in over TLS.
    let mut client = Client::new(&format!("https://localhost:{port}"));
    client.http = http;
    let token = s.team_token().await;
    let mut device = Device::enroll_on(client, &token).await.unwrap();
    device.sign_in().await.unwrap();
    assert_eq!(device.message(b"over TLS").await, 1);
    // In clear on a TLS port: no answer.
    assert!(reqwest::get(format!("http://localhost:{port}/healthz"))
        .await
        .is_err());
    s.finish().await;
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_escouade-team-server"))
}

/// A free port: in the worktree's range when `ESCOUADE_PORT_BASE` is set, else one the system gives.
fn free_port() -> u16 {
    if let (Ok(base), Ok(end)) = (
        std::env::var("ESCOUADE_PORT_BASE"),
        std::env::var("ESCOUADE_PORT_END"),
    ) {
        if let (Ok(base), Ok(end)) = (base.parse::<u16>(), end.parse::<u16>()) {
            for port in base..=end {
                if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
                    return port;
                }
            }
        }
    }
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn init_then_serve() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let out = bin().args(["init", "--data"]).arg(&data).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let token = stdout
        .lines()
        .map(str::trim)
        .find(|l| l.len() == 43 && !l.contains(' '))
        .expect("the token is printed")
        .to_string();
    assert!(data.join("team.db").exists());

    let port = free_port();
    let mut child = bin()
        .args(["serve", "--data"])
        .arg(&data)
        .args(["--listen", &format!("127.0.0.1:{port}")])
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let base = format!("http://127.0.0.1:{port}");
    let mut up = false;
    for _ in 0..100 {
        if reqwest::get(format!("{base}/healthz"))
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            up = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(up, "serve answers /healthz");

    // The token of init creates the team.
    let mut device = Device::enroll_on(Client::new(&base), &token).await.unwrap();
    device.sign_in().await.unwrap();
    let project = random::<16>();
    device.append_team(Kind::Message, None, b"x").await.ok();
    device.append_project(&project, b"y").await.ok();
    child.kill().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let _ = child.wait();
    let mut log = String::new();
    for line in BufReader::new(stdout)
        .lines()
        .chain(BufReader::new(stderr).lines())
        .map_while(Result::ok)
    {
        log.push_str(&line);
        log.push('\n');
    }
    // The log says the routes, never an id or a token.
    assert!(log.contains("/v1/projects/{project}/stream"), "{log}");
    for id in [&device.team, &device.id, &project.to_vec()] {
        assert!(!log.contains(&hex::encode(id)), "an id in the log:\n{log}");
    }
    assert!(!log.contains(&token));
    assert!(!log.contains(device.client.token.as_ref().unwrap()));
}
