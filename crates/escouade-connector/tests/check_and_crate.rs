//! `escouade-connector check` against hosts on 127.0.0.1, and what the crate is made of: nothing of the core nor of
//! Tauri, no socket that listens.

mod common;

use std::path::Path;
use std::time::Duration;

use common::*;
use escouade_connector::commands::check;
use escouade_connector::config;
use escouade_connector::guard::Guards;

#[tokio::test]
async fn check_says_each_host_of_the_ceiling() {
    let echo = echo().await;
    // A port where nothing answers: a listener bound then dropped.
    let closed = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let dns = FakeDns::new();
    dns.set("ok.acme.test", &[&["127.0.0.1"]]);
    dns.set("down.acme.test", &[&["127.0.0.1"]]);
    dns.set("meta.acme.test", &[&["169.254.169.254"]]);
    let c = config::parse(
        &format!(
            "allow = [\"ok.acme.test:{p}\", \"down.acme.test:{closed}\", \"meta.acme.test:80\", \"gone.acme.test:22\", \"*.npm.acme.test:443\"]",
            p = echo.port
        ),
        Path::new("connector.toml"),
    )
    .unwrap();
    let tests = Guards {
        loopback_for_tests: true,
    };
    let lines = check(&c, dns.as_ref(), &tests, Duration::from_secs(5)).await;
    let line = |rule: &str| {
        lines
            .iter()
            .find(|l| l.rule.starts_with(rule))
            .unwrap()
            .clone()
    };
    assert!(line("ok.acme.test").ok, "{:?}", line("ok.acme.test"));
    assert!(line("ok.acme.test").text.contains("answers at 127.0.0.1"));
    let down = line("down.acme.test");
    assert!(
        !down.ok && down.text.contains("does not answer"),
        "{down:?}"
    );
    let meta = line("meta.acme.test");
    assert!(!meta.ok && meta.text.contains("metadata"), "{meta:?}");
    let gone = line("gone.acme.test");
    assert!(!gone.ok && gone.text.contains("not resolved"), "{gone:?}");
    let pattern = line("*.npm.acme.test");
    assert!(pattern.ok && pattern.text.contains("a pattern"));
    // Without the option of the tests, the loopback is refused by check too.
    let lines = check(&c, dns.as_ref(), &Guards::default(), Duration::from_secs(5)).await;
    let ok = lines
        .iter()
        .find(|l| l.rule.starts_with("ok.acme.test"))
        .unwrap();
    assert!(!ok.ok && ok.text.contains("the loopback"), "{ok:?}");
}

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn nothing_of_the_core_nor_of_tauri() {
    let manifest = std::fs::read_to_string(crate_dir().join("Cargo.toml")).unwrap();
    let deps: Vec<&str> = manifest
        .lines()
        .filter(|l| !l.starts_with('#') && l.contains('='))
        .filter_map(|l| l.split('=').next())
        .map(str::trim)
        .collect();
    for dep in &deps {
        assert!(!dep.contains("tauri"), "{dep}");
        assert!(
            !dep.starts_with("escouade") || *dep == "escouade-connector-proto",
            "{dep}: only the protocol's crate"
        );
    }
    assert!(deps.contains(&"escouade-connector-proto"));
}

#[test]
fn it_listens_on_no_port() {
    for entry in std::fs::read_dir(crate_dir().join("src")).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        // The program's code, not its unit tests (which find this machine's address by a UDP socket).
        let text = text.split("#[cfg(test)]").next().unwrap();
        // No listener, nor the server's side of the protocol (`Incoming::accept` takes a stream the server opened).
        for word in [
            "TcpListener",
            "UdpSocket",
            ".listen(",
            "accept_websocket",
            "accept_connector",
            "tls_acceptor",
        ] {
            assert!(!text.contains(word), "{} holds {word}", path.display());
        }
    }
}

#[test]
fn the_example_reads() {
    let path = crate_dir().join("connector.example.toml");
    let (c, _) = config::load(&path).unwrap();
    assert_eq!(c.ceiling.rules.len(), 2);
    assert_eq!(c.bounds, config::Bounds::default());
    assert!(c.server.is_none() && c.proxy.is_none() && !c.auto_update);
}
