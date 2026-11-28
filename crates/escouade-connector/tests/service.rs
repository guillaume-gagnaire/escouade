//! The service's life: `connector.toml` read again while it runs, the schedule of reconnection (in simulated time), a
//! cut that closes the streams, the versions the server refuses, a server that no longer knows it, the pairing changed
//! beside it.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::*;
use escouade_connector::keys::Pairing;
use escouade_connector_proto::{Keypair, RefusalCode, Version};
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn connector_toml_is_read_again_while_it_runs() {
    let echo = echo().await;
    let p = echo.port;
    let dns = FakeDns::new();
    dns.set("a.acme.test", &[&["127.0.0.1"]]);
    dns.set("b.acme.test", &[&["127.0.0.1"]]);
    let mut s = service(Setup::new(format!("allow = [\"a.acme.test:{p}\"]"), dns)).await;
    let ch = s.channel().await;
    let e = ch.open("b.acme.test", p, "a", "p").await.err().unwrap();
    assert_eq!(refused_reason(e).0, RefusalCode::NotAllowed);

    // The IT adds b: said in the journal, sent to the server, let through.
    std::fs::write(
        &s.config_path,
        format!("allow = [\"a.acme.test:{p}\", \"b.acme.test:{p}\"]"),
    )
    .unwrap();
    let reloaded = wait_event(&s.lines, "config-reloaded", |_| true).await;
    assert_eq!(reloaded["added"][0], format!("b.acme.test:{p}"));
    assert_eq!(reloaded["changed"][0], "ceiling");
    let mut seen = ch.watch_ceiling();
    tokio::time::timeout(
        Duration::from_secs(10),
        seen.wait_for(|c| c.as_ref().is_some_and(|c| c.rules.len() == 2)),
    )
    .await
    .expect("the new ceiling reaches the server")
    .unwrap();
    let mut held = ch.open("b.acme.test", p, "agent-7", "Démo").await.unwrap();

    // A file that no longer reads: said, and the last one read kept.
    std::fs::write(&s.config_path, "allow = [\"b.acme.test\"]").unwrap();
    let bad = wait_event(&s.lines, "config-error", |_| true).await;
    assert!(bad["message"]
        .as_str()
        .unwrap()
        .contains("the last one read is kept"));
    assert_eq!(round_trip(&ch, "a.acme.test", p, b"still").await, b"still");

    // b taken out: the stream open to it is closed, and b is refused again.
    std::fs::write(&s.config_path, format!("allow = [\"a.acme.test:{p}\"]")).unwrap();
    let closed = wait_event(&s.lines, "closed", |e| e["host"] == "b.acme.test").await;
    assert!(
        closed["reason"]
            .as_str()
            .unwrap()
            .contains("no longer lets it through"),
        "{closed}"
    );
    assert_eq!(closed["agent"], "agent-7");
    let mut rest = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(10), held.read_to_end(&mut rest)).await;
    let e = ch.open("b.acme.test", p, "a", "p").await.err().unwrap();
    assert_eq!(refused_reason(e).0, RefusalCode::NotAllowed);

    // The bounds read again too.
    std::fs::write(
        &s.config_path,
        format!("allow = [\"a.acme.test:{p}\"]\n[limits]\nmax_streams = 1"),
    )
    .unwrap();
    let r = wait_event(&s.lines, "config-reloaded", |e| e["changed"][0] == "limits").await;
    assert_eq!(r["max_streams"], 1);
    let _one = ch.open("a.acme.test", p, "a", "p").await.unwrap();
    let e = ch.open("a.acme.test", p, "a", "p").await.err().unwrap();
    assert_eq!(refused_reason(e).0, RefusalCode::Limits);
    s.stop().await;
}

fn deltas(tries: &[tokio::time::Instant]) -> Vec<u128> {
    tries
        .windows(2)
        .map(|w| (w[1] - w[0]).as_millis())
        .collect()
}

async fn until_tries(dial: &MemDial, n: usize) {
    for _ in 0..100_000 {
        if dial.tries.lock().unwrap().len() >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("not {n} tries");
}

#[tokio::test(start_paused = true)]
async fn the_reconnection_follows_the_schedule_in_simulated_time() {
    let dns = FakeDns::new();
    let mut s = service(Setup::new("allow = []", dns)).await;
    *s.dial.down.lock().unwrap() = true;
    // The first try may already be under way: count from the ones that fail.
    until_tries(&s.dial, 9).await;
    let tries = s.dial.tries.lock().unwrap().clone();
    assert_eq!(
        deltas(&tries[1..9]),
        [500, 1000, 2000, 4000, 5000, 5000, 5000],
        "250 ms, 500 ms, 1 s, 2 s, 4 s, then every 5 s"
    );
    let first = s.lines.of("unreachable");
    assert!(first.iter().any(|e| e["retry_in_ms"] == 250));
    assert!(first.iter().any(|e| e["retry_in_ms"] == 5000));
    // Back up: the channel opens at the next try; cut, the schedule starts again at 250 ms.
    *s.dial.down.lock().unwrap() = false;
    let ch = s.channel().await;
    let up = s.dial.tries.lock().unwrap().len();
    ch.close();
    until_tries(&s.dial, up + 1).await;
    let tries = s.dial.tries.lock().unwrap().clone();
    assert_eq!((tries[up] - tries[up - 1]).as_millis(), 250);
    let _ = s.channel().await;
    let lines = s.stop().await;
    let cut = lines.of("cut");
    assert!(
        cut.iter().any(|e| e["retry_in_ms"] == 250),
        "{}",
        lines.text()
    );
}

#[tokio::test]
async fn a_cut_closes_the_streams_open_and_takes_none_up() {
    let echo = echo().await;
    let dns = FakeDns::new();
    dns.set("a.acme.test", &[&["127.0.0.1"]]);
    let mut s = service(Setup::new(
        format!("allow = [\"a.acme.test:{}\"]", echo.port),
        dns,
    ))
    .await;
    let ch = s.channel().await;
    let mut held = ch.open("a.acme.test", echo.port, "a", "p").await.unwrap();
    wait_event(&s.lines, "allowed", |_| true).await;
    ch.close();
    let closed = wait_event(&s.lines, "closed", |_| true).await;
    assert_eq!(closed["host"], "a.acme.test");
    wait_event(&s.lines, "cut", |_| true).await;
    let mut rest = Vec::new();
    let read = tokio::time::timeout(Duration::from_secs(10), held.read_to_end(&mut rest)).await;
    assert!(read.is_ok(), "the stream ends with the channel");
    // The connector opens a new channel; the old stream is not in it.
    let again = s.channel().await;
    assert_eq!(
        round_trip(&again, "a.acme.test", echo.port, b"new").await,
        b"new"
    );
    s.stop().await;
}

#[tokio::test(start_paused = true)]
async fn a_version_too_old_is_said_with_what_to_do() {
    let dns = FakeDns::new();
    let mut setup = Setup::new("allow = []", dns);
    let ours = escouade_connector::version();
    setup.server_version = Version::new(ours.major, ours.minor + 3, 0);
    let s = service(setup).await;
    until_tries(&s.dial, 2).await;
    let tries = s.dial.tries.lock().unwrap().clone();
    assert_eq!(
        (tries[1] - tries[0]).as_secs(),
        60,
        "tried again every minute"
    );
    let lines = s.stop().await;
    let refused = &lines.of("version-refused")[0];
    assert_eq!(refused["verdict"], "TooOld");
    let said = refused["message"].as_str().unwrap();
    assert!(
        said.contains("too old") && said.contains("escouade-connector update --to"),
        "{said}"
    );
}

#[tokio::test]
async fn a_server_older_than_the_connector_refuses_it_and_an_outdated_one_is_accepted() {
    let ours = escouade_connector::version();
    let dns = FakeDns::new();
    let mut setup = Setup::new("allow = []", dns.clone());
    setup.server_version = Version::new(ours.major, ours.minor.saturating_sub(1), 0);
    if ours.minor > 0 {
        let s = service(setup).await;
        let e = wait_event(&s.lines, "version-refused", |_| true).await;
        assert_eq!(e["verdict"], "TooNew");
        assert!(e["message"]
            .as_str()
            .unwrap()
            .contains("update escouade-server"));
        s.stop().await;
    }
    let mut setup = Setup::new("allow = []", dns);
    setup.server_version = Version::new(ours.major, ours.minor + 2, 0);
    let mut s = service(setup).await;
    let _ch = s.channel().await;
    let e = wait_event(&s.lines, "outdated", |_| true).await;
    assert!(e["message"].as_str().unwrap().contains("to update"), "{e}");
    let connected = &s.lines.of("connected")[0];
    assert_eq!(connected["verdict"], "Outdated");
    s.stop().await;
}

#[tokio::test(start_paused = true)]
async fn a_server_that_no_longer_knows_it_is_said_and_tried_every_minute() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("connector.toml");
    std::fs::write(&config_path, "allow = []").unwrap();
    let state = escouade_connector::keys::State::new(dir.path().join("state"));
    let keys = Keypair::generate();
    // The server pins no key: removed from the space.
    let (mut server, dial) = mem_server(SERVER_V, Arc::new(Mutex::new(None)));
    state
        .save(
            &Pairing {
                server: SERVER_URL.into(),
                server_key: server.keys.public(),
                connector_key: keys.public(),
            },
            &keys,
        )
        .unwrap();
    let (journal, lines) = escouade_connector::journal::Journal::memory();
    let (stop, stop_rx) = tokio::sync::watch::channel(false);
    let task = tokio::spawn(escouade_connector::run::run(
        escouade_connector::run::Options {
            config_path,
            state,
            journal,
            resolver: FakeDns::new(),
            guards: Default::default(),
            version: escouade_connector::version(),
            system_roots: false,
            reload_every: Duration::from_millis(50),
        },
        dial.clone(),
        stop_rx,
    ));
    until_tries(&dial, 2).await;
    let tries = dial.tries.lock().unwrap().clone();
    assert_eq!((tries[1] - tries[0]).as_secs(), 60);
    let refused = server.refused.recv().await.unwrap();
    assert!(refused.is_failed_attempt());
    let said = &lines.of("not-known")[0];
    assert!(
        said["message"].as_str().unwrap().contains("pair again"),
        "{said}"
    );
    let _ = stop.send(true);
    let _ = task.await;
}

#[tokio::test]
async fn the_pairing_changed_beside_the_service() {
    let dns = FakeDns::new();
    let mut setup = Setup::new("allow = []", dns);
    setup.paired = false;
    let mut s = service(setup).await;
    wait_event(&s.lines, "not-paired", |_| true).await;
    assert!(
        s.dial.tries.lock().unwrap().is_empty(),
        "nothing tried unpaired"
    );
    // `pair` beside it: a new pair of keys and the server's key kept; the server pins the new key.
    let keys = Keypair::generate();
    *s.pinned.lock().unwrap() = Some(keys.public());
    s.state
        .save(
            &Pairing {
                server: SERVER_URL.into(),
                server_key: s.server.keys.public(),
                connector_key: keys.public(),
            },
            &keys,
        )
        .unwrap();
    wait_event(&s.lines, "paired", |_| true).await;
    let ch = s.channel().await;
    // `unpair` beside it: the channel closed, nothing tried any more.
    s.state.remove().unwrap();
    let cut = wait_event(&s.lines, "cut", |_| true).await;
    assert!(
        cut["message"].as_str().unwrap().contains("pairing changed"),
        "{cut}"
    );
    let _ = tokio::time::timeout(Duration::from_secs(10), ch.closed())
        .await
        .expect("closed");
    let tries = s.dial.tries.lock().unwrap().len();
    for _ in 0..1000 {
        if s.lines.of("not-paired").len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let lines = s.stop().await;
    assert_eq!(lines.of("not-paired").len(), 2, "{}", lines.text());
    assert_eq!(tries, 1, "one try, while paired");
    assert!(!lines.text().contains(&hex::encode(keys.private_bytes())));
}

#[tokio::test]
async fn another_server_in_connector_toml_is_said() {
    let dns = FakeDns::new();
    let s = service(Setup::new(
        "server = \"wss://other.test/connector\"\nallow = []\nauto_update = true",
        dns,
    ))
    .await;
    let e = wait_event(&s.lines, "server-mismatch", |_| true).await;
    assert_eq!(e["paired"], SERVER_URL);
    assert_eq!(e["configured"], "wss://other.test/connector");
    let auto = wait_event(&s.lines, "auto-update", |_| true).await;
    assert!(auto["message"]
        .as_str()
        .unwrap()
        .contains("does not update itself yet"));
    assert!(s.dial.tries.lock().unwrap().is_empty());
    // Set right, the channel opens.
    std::fs::write(
        &s.config_path,
        format!("server = \"{SERVER_URL}\"\nallow = []"),
    )
    .unwrap();
    wait_event(&s.lines, "connected", |_| true).await;
    assert_eq!(s.dial.urls.lock().unwrap()[0], SERVER_URL);
    s.stop().await;
}
