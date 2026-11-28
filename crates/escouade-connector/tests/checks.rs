//! Each `open` through the service against the checks of § 4.4: the ceiling after the shape, the literal address, the
//! address resolved and checked (loopback, link-local, a cloud's metadata, this machine, out of the ranges), the DNS
//! rebinding, the bounds; each allowed or refused one in the journal.

mod common;

use common::*;
use escouade_connector::guard::Guards;
use escouade_connector_proto::RefusalCode;

const PAYLOAD: &[u8] = b"git-upload-pack /acme/app.git";

#[tokio::test]
async fn a_host_of_the_ceiling_is_joined_and_journaled_without_its_content() {
    let echo = echo().await;
    let dns = FakeDns::new();
    dns.set("gitlab.acme.test", &[&["127.0.0.1"]]);
    let mut s = service(Setup::new(
        format!("allow = [\"GitLab.Acme.Test.:{}\"]", echo.port),
        dns.clone(),
    ))
    .await;
    let ch = s.channel().await;
    // The ceiling reaches the server, in shape.
    ch.watch_ceiling()
        .wait_for(|c| {
            c.as_ref().map(|c| c.rules.clone())
                == Some(vec![format!("gitlab.acme.test:{}", echo.port)])
        })
        .await
        .unwrap();
    // The host asked in another case, with a final dot: the same host.
    let back = round_trip(&ch, "GITLAB.acme.test.", echo.port, PAYLOAD).await;
    assert_eq!(back, PAYLOAD);
    let closed = wait_event(&s.lines, "closed", |_| true).await;
    assert_eq!(closed["host"], "gitlab.acme.test");
    assert_eq!(closed["port"], echo.port);
    assert_eq!(closed["address"], "127.0.0.1");
    assert_eq!(closed["agent"], "agent-1");
    assert_eq!(closed["project"], "Démo");
    assert_eq!(closed["bytes_to_host"], PAYLOAD.len());
    assert_eq!(closed["bytes_from_host"], PAYLOAD.len());
    assert!(closed["duration_ms"].is_u64());
    let allowed = s.lines.of("allowed");
    assert_eq!(allowed.len(), 1);
    assert_eq!(allowed[0]["address"], "127.0.0.1");
    let lines = s.stop().await;
    let text = lines.text();
    assert!(!text.contains("git-upload-pack"), "never the content");
}

#[tokio::test]
async fn the_refusals_of_the_checks() {
    let echo = echo().await;
    let p = echo.port;
    let dns = FakeDns::new();
    dns.set("metadata.acme.test", &[&["169.254.169.254"]]);
    dns.set("linklocal.acme.test", &[&["169.254.10.20"]]);
    dns.set("v6local.acme.test", &[&["fe80::1"]]);
    dns.set("mapped.acme.test", &[&["::ffff:169.254.169.254"]]);
    dns.set("ok.acme.test", &[&["127.0.0.1"]]);
    let mut s = service(Setup::new(
        format!(
            "allow = [\"metadata.acme.test:80\", \"linklocal.acme.test:{p}\", \"v6local.acme.test:{p}\", \"mapped.acme.test:{p}\", \"ok.acme.test:{p}\", \"169.254.169.254:80\", \"missing.acme.test:{p}\"]"
        ),
        dns.clone(),
    ))
    .await;
    let ch = s.channel().await;
    let refuse = |host: &'static str, port: u16| {
        let ch = &ch;
        async move {
            let e = ch
                .open(host, port, "agent-1", "Démo")
                .await
                .err()
                .unwrap_or_else(|| panic!("{host}:{port} opened"));
            refused_reason(e)
        }
    };
    // A cloud's metadata, by a name and written as an address.
    let (code, why) = refuse("metadata.acme.test", 80).await;
    assert_eq!(code, RefusalCode::NotAllowed);
    assert!(
        why.contains("169.254.169.254") && why.contains("metadata"),
        "{why}"
    );
    let (code, why) = refuse("169.254.169.254", 80).await;
    assert_eq!(code, RefusalCode::NotAllowed);
    assert!(why.contains("metadata"), "{why}");
    // The link-local, in IPv4, IPv6 and mapped.
    for host in [
        "linklocal.acme.test",
        "v6local.acme.test",
        "mapped.acme.test",
    ] {
        let (code, why) = refuse(host, p).await;
        assert_eq!(code, RefusalCode::NotAllowed, "{host}");
        assert!(why.contains("never joins"), "{host}: {why}");
    }
    // Out of the ceiling: another host, another port, a pattern that only ends alike.
    let (code, why) = refuse("gitlab.acme.test", 22).await;
    assert_eq!(code, RefusalCode::NotAllowed);
    assert!(why.contains("not in the ceiling"), "{why}");
    let (code, _) = refuse("ok.acme.test", 1).await;
    assert_eq!(code, RefusalCode::NotAllowed);
    // A literal address the ceiling does not write, even one that the name resolves to.
    let (code, why) = refuse("127.0.0.1", p).await;
    assert_eq!(code, RefusalCode::NotAllowed);
    assert!(
        why.contains("address the ceiling of this connector does not write"),
        "{why}"
    );
    let (code, _) = refuse("[::1]", p).await;
    assert_eq!(code, RefusalCode::NotAllowed);
    // A name the DNS does not know.
    let (code, why) = refuse("missing.acme.test", p).await;
    assert_eq!(code, RefusalCode::Unresolved);
    assert!(why.contains("not resolved"), "{why}");
    // Not one connection reached the echo host.
    assert_eq!(*echo.accepted.lock().unwrap(), 0);
    // And the one allowed still passes.
    assert_eq!(round_trip(&ch, "ok.acme.test", p, b"x").await, b"x");
    let lines = s.stop().await;
    let refused = lines.of("refused");
    assert_eq!(refused.len(), 10, "{}", lines.text());
    assert!(refused
        .iter()
        .all(|r| r["agent"] == "agent-1" && r["project"] == "Démo" && r["reason"].is_string()));
    assert!(refused.iter().any(|r| r["code"] == "unresolved"));
}

#[tokio::test]
async fn the_loopback_is_refused_by_default() {
    let echo = echo().await;
    let dns = FakeDns::new();
    dns.set("loop.acme.test", &[&["127.0.0.1"]]);
    dns.set("loop6.acme.test", &[&["::1"]]);
    let mut setup = Setup::new(
        format!(
            "allow = [\"loop.acme.test:{p}\", \"loop6.acme.test:{p}\", \"127.0.0.1:{p}\", \"localhost:{p}\"]",
            p = echo.port
        ),
        dns.clone(),
    );
    setup.guards = Guards::default();
    let mut s = service(setup).await;
    let ch = s.channel().await;
    for host in ["loop.acme.test", "loop6.acme.test", "127.0.0.1"] {
        let e = ch.open(host, echo.port, "a", "p").await.err().unwrap();
        let (code, why) = refused_reason(e);
        assert_eq!(code, RefusalCode::NotAllowed, "{host}");
        assert!(why.contains("the loopback"), "{host}: {why}");
    }
    assert_eq!(*echo.accepted.lock().unwrap(), 0);
    s.stop().await;
}

#[tokio::test]
async fn the_address_resolved_is_the_one_joined_no_rebinding() {
    let echo = echo().await;
    let p = echo.port;
    let dns = FakeDns::new();
    // A name that answers an allowed address, then a cloud's metadata to whoever asks again.
    dns.set("rebind.acme.test", &[&["127.0.0.1"], &["169.254.169.254"]]);
    // A name that answers the metadata first, then an allowed address: the second answer is never asked.
    dns.set("late.acme.test", &[&["169.254.169.254"], &["127.0.0.1"]]);
    let mut s = service(Setup::new(
        format!("allow = [\"rebind.acme.test:{p}\", \"late.acme.test:{p}\"]"),
        dns.clone(),
    ))
    .await;
    let ch = s.channel().await;
    assert_eq!(round_trip(&ch, "rebind.acme.test", p, b"one").await, b"one");
    assert_eq!(
        dns.calls("rebind.acme.test"),
        1,
        "resolved once for one open"
    );
    assert_eq!(
        *echo.accepted.lock().unwrap(),
        1,
        "the address checked is the one joined"
    );
    let allowed = wait_event(&s.lines, "allowed", |e| e["host"] == "rebind.acme.test").await;
    assert_eq!(allowed["address"], "127.0.0.1");
    // The next open resolves again, and the new answer is checked as the first.
    let e = ch
        .open("rebind.acme.test", p, "a", "p")
        .await
        .err()
        .unwrap();
    assert!(refused_reason(e).1.contains("metadata"));
    assert_eq!(dns.calls("rebind.acme.test"), 2);
    let e = ch.open("late.acme.test", p, "a", "p").await.err().unwrap();
    assert!(refused_reason(e).1.contains("metadata"));
    assert_eq!(dns.calls("late.acme.test"), 1);
    assert_eq!(*echo.accepted.lock().unwrap(), 1);
    s.stop().await;
}

#[tokio::test]
async fn forbidden_addresses_among_several_are_left_out() {
    let echo = echo().await;
    let dns = FakeDns::new();
    dns.set("mixed.acme.test", &[&["169.254.169.254", "127.0.0.1"]]);
    let mut s = service(Setup::new(
        format!("allow = [\"mixed.acme.test:{}\"]", echo.port),
        dns,
    ))
    .await;
    let ch = s.channel().await;
    assert_eq!(
        round_trip(&ch, "mixed.acme.test", echo.port, b"y").await,
        b"y"
    );
    let allowed = wait_event(&s.lines, "allowed", |_| true).await;
    assert_eq!(allowed["address"], "127.0.0.1");
    let said = allowed["addresses_refused"][0].as_str().unwrap();
    assert!(
        said.contains("169.254.169.254") && said.contains("metadata"),
        "{said}"
    );
    s.stop().await;
}

#[tokio::test]
async fn out_of_the_ranges_is_refused() {
    let echo = echo().await;
    let dns = FakeDns::new();
    dns.set("ranged.acme.test", &[&["127.0.0.1"]]);
    let mut s = service(Setup::new(
        format!(
            "allow = [\"ranged.acme.test:{p}\", \"127.0.0.1:{p}\"]\nranges = [\"10.20.0.0/16\"]",
            p = echo.port
        ),
        dns,
    ))
    .await;
    let ch = s.channel().await;
    for host in ["ranged.acme.test", "127.0.0.1"] {
        let e = ch.open(host, echo.port, "a", "p").await.err().unwrap();
        let (code, why) = refused_reason(e);
        assert_eq!(code, RefusalCode::NotAllowed);
        assert!(why.contains("out of the ranges"), "{host}: {why}");
    }
    assert_eq!(*echo.accepted.lock().unwrap(), 0);
    s.stop().await;
}

#[tokio::test]
async fn this_machine_is_refused() {
    // This machine's address towards the outside (a UDP socket connected sends nothing).
    let Some(local) = std::net::UdpSocket::bind("0.0.0.0:0").ok().and_then(|s| {
        s.connect("192.0.2.1:9").ok()?;
        s.local_addr()
            .ok()
            .map(|a| a.ip())
            .filter(|ip| !ip.is_unspecified())
    }) else {
        eprintln!("no address towards the outside on this machine: skipped");
        return;
    };
    let dns = FakeDns::new();
    let ip = local.to_string();
    dns.set("self.acme.test", &[&[ip.as_str()]]);
    let mut s = service(Setup::new(
        format!("allow = [\"self.acme.test:22\", \"{ip}:22\"]"),
        dns,
    ))
    .await;
    let ch = s.channel().await;
    for host in ["self.acme.test", ip.as_str()] {
        let e = ch.open(host, 22, "a", "p").await.err().unwrap();
        let (code, why) = refused_reason(e);
        assert_eq!(code, RefusalCode::NotAllowed);
        assert!(why.contains("own machine"), "{host}: {why}");
    }
    s.stop().await;
}

#[tokio::test]
async fn the_streams_open_at_once_are_bounded() {
    let echo = echo().await;
    let dns = FakeDns::new();
    dns.set("a.acme.test", &[&["127.0.0.1"]]);
    let mut s = service(Setup::new(
        format!(
            "allow = [\"a.acme.test:{}\"]
[limits]
max_streams = 1",
            echo.port
        ),
        dns,
    ))
    .await;
    let ch = s.channel().await;
    let held = ch.open("a.acme.test", echo.port, "a", "p").await.unwrap();
    let e = ch
        .open("a.acme.test", echo.port, "a", "p")
        .await
        .err()
        .unwrap();
    let (code, why) = refused_reason(e);
    assert_eq!(code, RefusalCode::Limits);
    assert!(why.contains("1 streams at most"), "{why}");
    drop(held);
    wait_event(&s.lines, "closed", |_| true).await;
    // The stream closed, its place is free again.
    assert_eq!(round_trip(&ch, "a.acme.test", echo.port, b"z").await, b"z");
    let lines = s.stop().await;
    assert_eq!(
        lines
            .of("refused")
            .iter()
            .filter(|r| r["code"] == "limits")
            .count(),
        1
    );
}

#[tokio::test]
async fn the_opens_a_second_are_bounded_refused_ones_included() {
    let dns = FakeDns::new();
    let mut s = service(Setup::new(
        "allow = [\"a.acme.test:22\"]
[limits]
opens_per_second = 2",
        dns,
    ))
    .await;
    let ch = s.channel().await;
    // Two opens out of the ceiling (refused, counted), then a third in the same second: the bound.
    for _ in 0..2 {
        let e = ch.open("b.acme.test", 22, "a", "p").await.err().unwrap();
        assert_eq!(refused_reason(e).0, RefusalCode::NotAllowed);
    }
    let e = ch.open("b.acme.test", 22, "a", "p").await.err().unwrap();
    assert_eq!(refused_reason(e).0, RefusalCode::Limits);
    s.stop().await;
}
