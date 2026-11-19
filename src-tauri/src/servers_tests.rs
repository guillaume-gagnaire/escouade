//! « Se connecter à un serveur » (ESC-537, `docs/SERVER-UX.md` § 2 and 3): the hosts of `~/.ssh/config` (`Include`,
//! `ProxyJump`, patterns), a free port, the tunnel watched and started again, and each state of a connection against a
//! real `escouade-server` (its core in this process) behind a fake `ssh` (`tests/fixtures/fake-ssh.mjs`): never a real
//! SSH, never a host key accepted, the token kept in the keychain (memory here), one per server.

use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::paths::DataDir;
use crate::remoteapi::pairing;
use crate::remoteapi::server::{Limits, RemoteApi};
use crate::server;
use crate::servers::sshconfig::{self, SshHost};
use crate::servers::tunnel::{self, classify, Dest, Ssh, Tunnel, Why};
use crate::servers::{find_code, Outcome, Problem, Servers, TunnelState};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------- ~/.ssh/config ----------

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

#[test]
fn the_hosts_of_the_ssh_config_are_read_with_their_includes_and_defaults_patterns_left_out() {
    let home = crate::paths::test_dir("sshconfig");
    let ssh = home.join(".ssh");
    write(
        &ssh.join("config"),
        "# Mes serveurs\n\
         Include conf.d/*.conf\n\
         Host acme-escouade\n\
         \x20 HostName escouade.acme.internal\n\
         \x20 User escouade\n\
         \n\
         Host vps-perso perso2\n\
         \tHostName=vps.example.net\n\
         \tPort 2222\n\
         \n\
         Host labo-gpu\n\
         \x20 HostName 10.4.0.12\n\
         \x20 User gpu\n\
         \x20 ProxyJump bastion-acme\n\
         \n\
         Host sans-hostname\n\
         \x20 User x\n\
         \n\
         Host *.acme.internal !secret.acme.internal\n\
         \x20 User motif\n\
         \n\
         Match host labo-gpu\n\
         \x20 User jamais\n\
         \n\
         Host *\n\
         \x20 User defaut\n\
         \x20 Port 22\n",
    );
    write(
        &ssh.join("conf.d").join("10-bastion.conf"),
        "Host bastion-acme\n  HostName \"bastion.acme.example\"\n  Port 2201\n",
    );
    // Not read: not a `.conf`.
    write(
        &ssh.join("conf.d").join("notes.txt"),
        "Host notes\n  HostName n\n",
    );
    let hosts = sshconfig::hosts(&ssh.join("config"));
    let names: Vec<&str> = hosts.iter().map(|h| h.alias.as_str()).collect();
    // In their order, the included file where it stands; no pattern, none without HostName.
    assert_eq!(
        names,
        [
            "bastion-acme",
            "acme-escouade",
            "vps-perso",
            "perso2",
            "labo-gpu"
        ]
    );
    let get = |a: &str| hosts.iter().find(|h| h.alias == a).unwrap().clone();
    assert_eq!(
        get("bastion-acme"),
        SshHost {
            alias: "bastion-acme".into(),
            host_name: "bastion.acme.example".into(),
            user: Some("defaut".into()),
            port: Some(2201),
            proxy_jump: None,
        }
    );
    // The first value wins: its own User, the Port of `Host *`.
    let acme = get("acme-escouade");
    assert_eq!(acme.user.as_deref(), Some("escouade"));
    assert_eq!(acme.port, Some(22));
    // `Keyword=value`, two names on a line.
    assert_eq!(get("perso2").host_name, "vps.example.net");
    assert_eq!(get("perso2").port, Some(2222));
    // ProxyJump; Match is not read.
    let labo = get("labo-gpu");
    assert_eq!(labo.proxy_jump.as_deref(), Some("bastion-acme"));
    assert_eq!(labo.user.as_deref(), Some("gpu"));
    // A config that is not there: nothing.
    assert!(sshconfig::hosts(&home.join("nowhere")).is_empty());
}

#[test]
fn an_include_that_includes_itself_ends() {
    let home = crate::paths::test_dir("sshconfig-loop");
    let ssh = home.join(".ssh");
    write(
        &ssh.join("config"),
        "Include config\nHost a\n  HostName a.example\n",
    );
    let hosts = sshconfig::hosts(&ssh.join("config"));
    assert_eq!(hosts.first().map(|h| h.alias.as_str()), Some("a"));
}

#[test]
fn patterns_match_as_ssh_reads_them() {
    let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert!(sshconfig::host_matches(
        &p(&["*.acme.internal"]),
        "x.acme.internal"
    ));
    assert!(!sshconfig::host_matches(
        &p(&["*.acme.internal", "!x.acme.internal"]),
        "x.acme.internal"
    ));
    assert!(sshconfig::host_matches(&p(&["vps-?"]), "vps-1"));
    assert!(!sshconfig::host_matches(&p(&["vps-?"]), "vps-12"));
    assert!(sshconfig::glob("*", ""));
}

// ---------- a host typed, the arguments of ssh, its failures ----------

#[test]
fn a_host_typed_is_read_and_one_that_would_be_an_option_or_a_command_is_refused() {
    assert_eq!(
        Dest::parse(" escouade@vps.example.net:2222 ").unwrap(),
        Dest {
            target: "escouade@vps.example.net".into(),
            port: Some(2222)
        }
    );
    assert_eq!(Dest::parse("acme-escouade").unwrap().port, None);
    assert_eq!(Dest::parse("[::1]:22").unwrap().target, "[::1]");
    assert_eq!(Dest::parse("fe80::1").unwrap().port, None);
    for bad in [
        "",
        "-oProxyCommand=calc",
        "a b",
        "host;rm -rf",
        "x:99999",
        "x:0",
        "user@",
        "$(id)",
    ] {
        assert!(Dest::parse(bad).is_err(), "{bad} should be refused");
    }
}

#[test]
fn the_tunnel_asks_nothing_accepts_no_key_and_listens_on_the_loopback_alone() {
    let dest = Dest::parse("acme-escouade").unwrap();
    let args = tunnel::forward_args(&dest, 47831, 46480).join(" ");
    assert!(args.contains("-N"));
    assert!(args.contains("-L 127.0.0.1:47831:127.0.0.1:46480"));
    assert!(args.contains("BatchMode=yes"));
    assert!(args.contains("StrictHostKeyChecking=yes"));
    assert!(args.contains("ExitOnForwardFailure=yes"));
    assert!(!args
        .to_ascii_lowercase()
        .contains("stricthostkeychecking=no"));
    assert!(!args.contains("accept-new"));
    assert!(args.ends_with(" acme-escouade"));
    let typed = Dest::parse("escouade@vps.example.net:2222").unwrap();
    assert!(tunnel::forward_args(&typed, 1, 2)
        .join(" ")
        .contains("-p 2222"));
    assert_eq!(
        tunnel::shown_command(&dest, 47831, 46480),
        "ssh -N -L 47831:127.0.0.1:46480 acme-escouade"
    );
}

#[test]
fn what_ssh_writes_is_read_into_the_states_of_a_connection() {
    assert_eq!(
        classify("No ED25519 host key is known for x and you have requested strict checking.\nHost key verification failed."),
        Why::UnknownHostKey
    );
    assert_eq!(
        classify("@    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @\nHost key verification failed."),
        Why::ChangedHostKey
    );
    assert_eq!(
        classify("ssh: connect to host escouade.acme.internal port 22: Connection timed out"),
        Why::Unreachable
    );
    assert_eq!(
        classify("ssh: Could not resolve hostname nulle-part: Name or service not known"),
        Why::Unreachable
    );
    assert_eq!(
        classify("escouade@x: Permission denied (publickey)."),
        Why::Auth
    );
    assert_eq!(
        classify("channel 2: open failed: connect failed: Connection refused"),
        Why::NoEscouade
    );
    assert_eq!(
        classify("channel 2: open failed: administratively prohibited: open failed"),
        Why::Forwarding
    );
    assert_eq!(
        classify("bind [127.0.0.1]:4: Address already in use\nCould not request local forwarding."),
        Why::PortBusy
    );
    assert!(Why::Unreachable.passing());
    assert!(!Why::UnknownHostKey.passing());
    assert!(!Why::Auth.passing());
    // What is shown leaves the `debug1:` lines out.
    assert_eq!(
        tunnel::shown_output(&["debug1: x".into(), "ssh: refused".into()]),
        "ssh: refused"
    );
}

#[test]
fn a_free_port_of_the_loopback_is_given() {
    let a = tunnel::free_port().unwrap();
    assert!(a > 0);
    // Free indeed: it can be bound.
    std::net::TcpListener::bind(("127.0.0.1", a)).unwrap();
}

#[test]
fn the_code_escouade_server_pair_prints_is_found() {
    assert_eq!(
        find_code("Code d’appairage : K7Q4-MXR9\n\nValable une fois").as_deref(),
        Some("K7Q4-MXR9")
    );
    assert_eq!(
        find_code("Pairing code: AB0D-EFGH"),
        None,
        "0 is not of the alphabet"
    );
    assert_eq!(find_code("nothing"), None);
}

// ---------- against a server, behind the fake ssh ----------

fn fake_ssh(dir: &Path, ports: &str, server: Option<(&Path, &Path)>) -> Ssh {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("fake-ssh.mjs");
    let mut env = vec![
        ("FAKE_SSH_DIR".into(), dir.as_os_str().to_owned()),
        ("FAKE_SSH_PORTS".into(), ports.into()),
    ];
    if let Some((program, data)) = server {
        env.push(("FAKE_SSH_SERVER".into(), program.as_os_str().to_owned()));
        env.push(("FAKE_SSH_DATA".into(), data.as_os_str().to_owned()));
    }
    Ssh {
        program: "node".into(),
        prefix: vec![script.into_os_string()],
        env,
    }
}

/// `escouade-server`'s API on `data`, on a free port: its port, and what stops it.
async fn started(data: DataDir) -> (Arc<RemoteApi>, tokio::sync::oneshot::Sender<()>) {
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        server::serve(
            data,
            server::Endpoint::port(0),
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
    (ready_rx.await.expect("the server is ready"), stop_tx)
}

struct Bench {
    servers: Arc<Servers>,
    store: Arc<MemoryStore>,
    data: DataDir,
    root: PathBuf,
    dir: PathBuf,
    port: u16,
    _stop: tokio::sync::oneshot::Sender<()>,
}

async fn bench(name: &str, version: &str, refuse: Option<&'static str>) -> Bench {
    bench_with(name, version, refuse, "", false).await
}

/// `bench`, the fake `ssh` mapping also the ports `extra` (`<remote>=<real>,…`).
async fn bench_ports(
    name: &str,
    version: &str,
    refuse: Option<&'static str>,
    extra: &str,
) -> Bench {
    bench_with(name, version, refuse, extra, false).await
}

/// `bench_ports`, its fake `ssh` starting a process of its own for each of its tunnels (`FAKE_SSH_CHILD`) when
/// `child`.
async fn bench_with(
    name: &str,
    version: &str,
    refuse: Option<&'static str>,
    extra: &str,
    child: bool,
) -> Bench {
    let root = crate::paths::test_dir(&format!("servers-{name}"));
    let data = DataDir::new(root.join("server-data"));
    data.ensure().unwrap();
    let (api, stop) = started(data.clone()).await;
    let port = api.addr().port();
    let dir = root.join("ssh");
    std::fs::create_dir_all(&dir).unwrap();
    let mut ssh = fake_ssh(
        &dir,
        &format!("{}={port},{extra}", tunnel::SERVER_PORT),
        None,
    );
    if child {
        ssh.env.push(("FAKE_SSH_CHILD".into(), "1".into()));
    }
    let client = root.join("client");
    std::fs::create_dir_all(&client).unwrap();
    let config = root.join(".ssh").join("config");
    write(
        &config,
        "Host travail\n  HostName escouade.acme.internal\nHost ailleurs\n  HostName autre.example.net\n",
    );
    let store = Arc::new(MemoryStore::default());
    let servers = Servers::load(
        &client,
        store.clone(),
        ssh,
        version.into(),
        Arc::new(move |host: &str| {
            match refuse {
            Some(r) if host.contains(r) => anyhow::bail!("Le serveur {host} est refusé : la stratégie de ce poste ne permet que *.acme.internal."),
            _ => Ok(()),
        }
        }),
        Some(config),
    );
    Bench {
        servers,
        store,
        data,
        root: client,
        dir,
        port,
        _stop: stop,
    }
}

fn ok<T: serde::Serialize + std::fmt::Debug>(o: Outcome<T>) -> T {
    match o {
        Outcome::Ok { value } => value,
        Outcome::Problem { problem } => panic!("a problem: {problem:?}"),
    }
}

fn problem_of<T: serde::Serialize>(o: Outcome<T>) -> Problem {
    match o {
        Outcome::Ok { .. } => panic!("passed"),
        Outcome::Problem { problem } => problem,
    }
}

async fn until(what: &str, mut f: impl FnMut() -> bool) {
    for _ in 0..300 {
        if f() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("{what}: not in 30 s");
}

#[tokio::test(flavor = "multi_thread")]
async fn pairing_by_a_code_keeps_the_token_in_the_keychain_and_the_window_goes_on_the_server_and_back(
) {
    let b = bench("pair", VERSION, None).await;
    // Step « Connexion »: Escouade answers, its version said.
    let reached = ok(b.servers.attempt("travail", None, None).await);
    assert_eq!(reached.version, VERSION);
    assert_eq!(reached.name, "travail");
    assert_eq!(reached.host, "escouade.acme.internal");
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    // A wrong code: said as such; the attempt stays for the next.
    match problem_of(
        b.servers
            .pair(&reached.attempt, "ZZZZ-ZZZZ", "Travail", "Poste")
            .await,
    ) {
        Problem::Code { reason, .. } => assert_eq!(reason, "wrong"),
        p => panic!("{p:?}"),
    }
    let id = ok(b
        .servers
        .pair(
            &reached.attempt,
            &code.text,
            "Travail",
            "DESKTOP-TEST · Windows",
        )
        .await);
    // The token in the keychain, one entry for this server; nowhere in servers.json.
    let token = b.store.get(&format!("server-token-{id}")).unwrap().unwrap();
    assert!(!token.is_empty());
    let saved = std::fs::read_to_string(b.root.join(crate::servers::FILE)).unwrap();
    assert!(!saved.contains(&token));
    assert!(saved.contains("Travail"));
    assert!(!b.root.join(crate::servers::TOKENS_FILE).exists());
    // Its version and its machine, known.
    let view = b.servers.view();
    assert_eq!(view.servers.len(), 1);
    assert_eq!(view.servers[0].known.version.as_deref(), Some(VERSION));
    assert!(view.servers[0].known.machine.is_some());
    assert_eq!(view.active, None);
    // The window on it: the link it connects by, through the tunnel, with the token.
    ok(b.servers.open(&id).await);
    let link = b.servers.link().expect("a link");
    assert!(link.url.starts_with("ws://127.0.0.1:"));
    assert_eq!(link.token, token);
    assert_eq!(link.tunnel, TunnelState::Up);
    assert_eq!(b.servers.active().as_deref(), Some(id.as_str()));
    // Back on this computer: no link for the window; the tunnel stays up, the server still a space (ESC-539).
    b.servers.leave();
    assert!(b.servers.link().is_none());
    assert_eq!(b.servers.active(), None);
    assert_eq!(
        b.servers.link_of(&id).map(|l| l.tunnel),
        Some(TunnelState::Up)
    );
    assert_eq!(b.servers.tunnels(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_code_is_got_over_ssh_and_an_expired_one_is_said_so() {
    let b = bench("over-ssh", VERSION, None).await;
    let program = crate::paths::test_dir("servers-over-ssh-prog").join(if cfg!(windows) {
        "pair.cmd"
    } else {
        "pair.sh"
    });
    // A stand-in of `escouade-server pair`: the code made on the server's data, printed.
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    if cfg!(windows) {
        std::fs::write(
            &program,
            format!("@echo Code d'appairage : {}\r\n", code.text),
        )
        .unwrap();
    } else {
        std::fs::write(
            &program,
            format!("#!/bin/sh\necho 'Code d’appairage : {}'\n", code.text),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    let ssh = fake_ssh(
        &b.dir,
        &format!("{}={}", tunnel::SERVER_PORT, b.port),
        Some((&program, b.data.root())),
    );
    let servers = Servers::load(
        &b.root,
        b.store.clone(),
        ssh,
        VERSION.into(),
        Arc::new(|_: &str| Ok(())),
        None,
    );
    let reached = ok(servers
        .attempt("escouade@vps.example.net", None, None)
        .await);
    let got = ok(servers.code_over_ssh(&reached.attempt).await);
    assert_eq!(got, code.text);
    ok(servers.pair(&reached.attempt, &got, "Perso", "Poste").await);
    // A code that waits no longer (used): « relance escouade-server pair ».
    let again = ok(servers
        .attempt("escouade@vps.example.net", None, None)
        .await);
    match problem_of(servers.pair(&again.attempt, &got, "Perso", "Poste").await) {
        Problem::Code { reason, .. } => assert_eq!(reason, "none"),
        p => panic!("{p:?}"),
    }
    // Without escouade-server on the host: « pas d'Escouade ».
    let bare = Servers::load(
        &b.root,
        b.store.clone(),
        fake_ssh(&b.dir, &format!("{}={}", tunnel::SERVER_PORT, b.port), None),
        VERSION.into(),
        Arc::new(|_: &str| Ok(())),
        None,
    );
    let r = ok(bare.attempt("travail", None, None).await);
    assert!(matches!(
        problem_of(bare.code_over_ssh(&r.attempt).await),
        Problem::NoEscouade { .. }
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn each_failure_before_the_pairing_is_its_own_state() {
    let b = bench("states", VERSION, Some("autre.example.net")).await;
    // The policy, before any connection: no ssh run.
    match problem_of(b.servers.attempt("ailleurs", None, None).await) {
        Problem::Policy { message } => assert!(message.contains("autre.example.net"), "{message}"),
        p => panic!("{p:?}"),
    }
    assert!(!b.dir.join("runs.log").exists(), "ssh was not run");
    // An unknown host key: never accepted.
    match problem_of(b.servers.attempt("labo-inconnu", None, None).await) {
        Problem::HostKey {
            changed,
            output,
            command,
        } => {
            assert!(!changed);
            assert!(output.contains("Host key verification failed"), "{output}");
            assert!(command.starts_with("ssh -N -L "), "{command}");
        }
        p => panic!("{p:?}"),
    }
    assert!(matches!(
        problem_of(b.servers.attempt("labo-change", None, None).await),
        Problem::HostKey { changed: true, .. }
    ));
    // Unreachable, with what ssh said.
    match problem_of(
        b.servers
            .attempt("injoignable.example.net", None, None)
            .await,
    ) {
        Problem::Unreachable { why, output, .. } => {
            assert_eq!(why, Why::Unreachable);
            assert!(output.contains("Connection timed out"), "{output}");
        }
        p => panic!("{p:?}"),
    }
    assert!(matches!(
        problem_of(b.servers.attempt("refuse.example.net", None, None).await),
        Problem::Unreachable { why: Why::Auth, .. }
    ));
    // Nothing on the server's port: no Escouade there.
    let closed = tunnel::free_port().unwrap();
    match problem_of(b.servers.attempt("travail", Some(closed), None).await) {
        Problem::NoEscouade { output, .. } => assert!(output.contains("open failed"), "{output}"),
        p => panic!("{p:?}"),
    }
    // A host that would be an option of ssh.
    assert!(matches!(
        problem_of(b.servers.attempt("-oProxyCommand=calc", None, None).await),
        Problem::Refused { .. }
    ));
    // Every run asked no question and accepted no key.
    let runs = std::fs::read_to_string(b.dir.join("runs.log")).unwrap();
    for line in runs.lines() {
        assert!(
            line.contains("BatchMode=yes") && line.contains("StrictHostKeyChecking=yes"),
            "{line}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn another_version_is_refused_with_the_side_to_update() {
    let b = bench("version", "0.0.1", None).await;
    match problem_of(b.servers.attempt("travail", None, None).await) {
        Problem::Version {
            server,
            client,
            update,
            server_update,
        } => {
            assert_eq!(server, VERSION);
            assert_eq!(client, "0.0.1");
            assert_eq!(update, "client");
            // The client is the older: no command for the server (ESC-515).
            assert_eq!(server_update, None);
        }
        p => panic!("{p:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tunnel_cut_is_started_again_on_its_port_and_a_revoked_token_says_when_and_by_whom() {
    let b = bench("cut", VERSION, None).await;
    let reached = ok(b.servers.attempt("travail", None, None).await);
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    let id = ok(b
        .servers
        .pair(&reached.attempt, &code.text, "Travail", "Poste A")
        .await);
    ok(b.servers.open(&id).await);
    let port = b.servers.link().unwrap().url;
    // The tunnel cut, and the host unreachable for a while: the link says it starts again.
    std::fs::write(b.dir.join("down"), "").unwrap();
    std::fs::write(b.dir.join("cut"), "").unwrap();
    let servers = b.servers.clone();
    until("the tunnel said down", || {
        matches!(servers.link().map(|l| l.tunnel), Some(TunnelState::Starting { tries, .. }) if tries >= 1)
    })
    .await;
    // Back: up again, on the same port.
    std::fs::remove_file(b.dir.join("down")).unwrap();
    b.servers.retry();
    until("the tunnel up again", || {
        servers.link().map(|l| l.tunnel) == Some(TunnelState::Up)
    })
    .await;
    assert_eq!(b.servers.link().unwrap().url, port);
    // Forgotten here while on it (S15): revoked there through its tunnel, back on this computer, the token gone from
    // the keychain, the line gone.
    assert!(
        b.servers.forget(&id).await,
        "the server revoked this computer"
    );
    assert_eq!(b.servers.active(), None);
    assert!(b.servers.link().is_none());
    assert!(b
        .store
        .get(&format!("server-token-{id}"))
        .unwrap()
        .is_none());
    assert!(b.servers.view().servers.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_token_revoked_on_the_server_is_said_revoked_when_the_window_goes_on_it() {
    let b = bench("revoked", VERSION, None).await;
    let reached = ok(b.servers.attempt("travail", None, None).await);
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    let id = ok(b
        .servers
        .pair(&reached.attempt, &code.text, "Travail", "Poste B")
        .await);
    let token = b.servers.token(&id).unwrap();
    let tunnel = Tunnel::start(
        &fake_ssh(&b.dir, &format!("{}={}", tunnel::SERVER_PORT, b.port), None),
        &Dest::parse("travail").unwrap(),
        tunnel::SERVER_PORT,
        None,
    )
    .unwrap();
    let stream = tunnel.listening(tunnel::READY_WAIT).await.unwrap();
    // Forgotten by an unpair of this very token (as another device would revoke it): the trace says by « Poste B ».
    crate::servers::client::unpair(stream, tunnel.port, &token)
        .await
        .unwrap();
    tunnel.stop();
    match problem_of(b.servers.open(&id).await) {
        Problem::Revoked { revoked } => {
            let revoked = revoked.expect("when and by whom");
            assert_eq!(revoked.device, "Poste B");
            assert_eq!(revoked.by.as_deref(), Some("Poste B"));
            assert!(revoked.at > 0);
        }
        p => panic!("{p:?}"),
    }
    // The state stays with the server, for Réglages › Serveurs.
    assert!(matches!(
        b.servers.view().servers[0].known.problem,
        Some(Problem::Revoked { .. })
    ));
    assert_eq!(b.servers.active(), None);
}

#[test]
fn a_tunnel_of_an_ssh_that_is_not_there_says_so() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let ssh = Ssh {
            program: crate::paths::test_dir("no-ssh").join("ssh-absent"),
            prefix: Vec::new(),
            env: Vec::new(),
        };
        match Tunnel::start(&ssh, &Dest::parse("x").unwrap(), 1, None) {
            Err(f) => assert_eq!(f.why, Why::NoSsh),
            Ok(_) => panic!("started"),
        }
    });
}

#[test]
fn the_policy_of_this_computer_reads_hosts_and_domains() {
    use crate::locks::ssh_host_of;
    assert_eq!(
        ssh_host_of("ssh://escouade@Vps.Example.net:2222/"),
        "vps.example.net"
    );
    assert_eq!(
        ssh_host_of("escouade.acme.internal"),
        "escouade.acme.internal"
    );
    assert_eq!(ssh_host_of("[::1]:22"), "::1");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_server_answers_a_ping_at_once_and_says_the_machine_it_runs_on() {
    use crate::remoteapi::proto::{ToClient, ToServer};
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let b = bench("ping", VERSION, None).await;
    let reached = ok(b.servers.attempt("travail", None, None).await);
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    let id = ok(b
        .servers
        .pair(&reached.attempt, &code.text, "Travail", "Poste")
        .await);
    let token = b.servers.token(&id).unwrap();
    let tcp = tokio::net::TcpStream::connect(("127.0.0.1", b.port))
        .await
        .unwrap();
    let (mut ws, _) =
        tokio_tungstenite::client_async(format!("ws://127.0.0.1:{}/remote", b.port), tcp)
            .await
            .unwrap();
    let send = |m: &ToServer| Message::text(serde_json::to_string(m).unwrap());
    ws.send(send(&ToServer::Hello {
        version: VERSION.into(),
        token,
        session: None,
    }))
    .await
    .unwrap();
    let mut welcome = None;
    while let Some(Ok(Message::Text(t))) = ws.next().await {
        if let Ok(m @ ToClient::Welcome { .. }) = serde_json::from_str::<ToClient>(&t) {
            welcome = Some(m);
            break;
        }
    }
    let Some(ToClient::Welcome { machine, .. }) = welcome else {
        panic!("no welcome")
    };
    assert_eq!(machine, Some(crate::remoteapi::this_machine()));
    ws.send(send(&ToServer::Ping { id: 7 })).await.unwrap();
    let pong = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(Ok(m)) = ws.next().await {
            if let Message::Text(t) = m {
                if let Ok(ToClient::Pong { id }) = serde_json::from_str::<ToClient>(&t) {
                    return id;
                }
            }
        }
        0
    })
    .await
    .unwrap();
    assert_eq!(pong, 7);
}

// ---------- the ports relayed (ESC-538) ----------

/// A server of the test that answers each connection with `hello`: a test server of an agent, or a sign-in waiting
/// for its return on `localhost`.
async fn answering(hello: &'static [u8]) -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut s, _)) = listener.accept().await {
            use tokio::io::AsyncWriteExt;
            let _ = s.write_all(hello).await;
        }
    });
    port
}

async fn read_from(port: u16) -> String {
    use tokio::io::AsyncReadExt;
    let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    let mut got = vec![0u8; 64];
    let n = tokio::time::timeout(Duration::from_secs(20), s.read(&mut got))
        .await
        .unwrap()
        .unwrap();
    String::from_utf8_lossy(&got[..n]).to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_port_of_the_server_is_relayed_to_this_computer_and_a_sign_in_on_the_same_port_or_not_at_all(
) {
    // The server's test server, on a port this computer has taken too (they are the same machine here): another port.
    let dev = answering(b"vite").await;
    // A sign-in waiting on the server's port 1455 (Codex), which the fake ssh maps to a port of the test.
    let login = answering(b"callback").await;
    let wanted = tunnel::free_port().unwrap();
    let b = bench_ports("relay", VERSION, None, &format!("{wanted}={login}")).await;
    // Not on a server: nothing relayed.
    match b.servers.relay(dev, false).await {
        Err(Problem::Refused { code, .. }) => assert_eq!(code, "relay"),
        r => panic!("{r:?}"),
    }
    let reached = ok(b.servers.attempt("travail", None, None).await);
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    let id = ok(b
        .servers
        .pair(&reached.attempt, &code.text, "Travail", "Poste")
        .await);
    ok(b.servers.open(&id).await);
    let relayed = b.servers.relay(dev, false).await.unwrap();
    assert_eq!(relayed.remote, dev);
    assert_ne!(relayed.local, dev);
    assert_eq!(read_from(relayed.local).await, "vite");
    // Asked again: the same relay.
    assert_eq!(b.servers.relay(dev, false).await.unwrap(), relayed);
    assert_eq!(b.servers.relays(), vec![relayed]);
    // Its forward as the tunnel's: on the loopback alone, nothing asked, no key accepted.
    let runs = std::fs::read_to_string(b.dir.join("runs.log")).unwrap();
    let line = runs
        .lines()
        .find(|l| l.contains(&format!("127.0.0.1:{}:127.0.0.1:{dev}", relayed.local)))
        .expect("the relay's ssh");
    assert!(
        line.contains("BatchMode=yes") && line.contains("StrictHostKeyChecking=yes"),
        "{line}"
    );
    // A sign-in's return, on the same port: relayed there, or refused when this computer has it taken.
    let same = b.servers.relay(wanted, true).await.unwrap();
    assert_eq!(same.local, wanted);
    assert_eq!(read_from(wanted).await, "callback");
    match b.servers.relay(dev, true).await {
        Err(Problem::Refused { code, .. }) => assert_eq!(code, "relay-busy"),
        r => panic!("{r:?}"),
    }
    // Back on this computer: every relay stopped.
    b.servers.leave();
    assert!(b.servers.relays().is_empty());
    until("the relay's port let go", || {
        std::net::TcpListener::bind(("127.0.0.1", relayed.local)).is_ok()
    })
    .await;
}

/// A server older than this client says how to put it at the client's version (ESC-515): the client keeps it in its
/// problem, for « Mettre à jour le serveur ».
#[test]
fn a_refusal_of_the_versions_keeps_how_to_update_the_server() {
    let how = crate::remoteapi::proto::server_update("2.3.1", false);
    let refused = crate::servers::client::Refusal::Refused {
        code: "version".into(),
        error: "Le serveur est en 2.3.0, ce client en 2.3.1 : mettre à jour le serveur.".into(),
        server_version: Some("2.3.0".into()),
        update: Some("server".into()),
        reason: None,
        revoked: None,
        server_update: Some(how.clone()),
    };
    match Problem::of_refusal(refused, None, "2.3.1") {
        Problem::Version {
            server,
            client,
            update,
            server_update,
        } => {
            assert_eq!(
                (server.as_str(), client.as_str(), update.as_str()),
                ("2.3.0", "2.3.1", "server")
            );
            assert_eq!(server_update, Some(how));
        }
        p => panic!("{p:?}"),
    }
    // As the window reads it.
    let json = serde_json::to_value(Problem::Version {
        server: "2.3.0".into(),
        client: "2.3.1".into(),
        update: "server".into(),
        server_update: Some(crate::remoteapi::proto::server_update("2.3.1", true)),
    })
    .unwrap();
    assert_eq!(json["serverUpdate"]["how"], "image");
}

// ---------- several servers at once (ESC-539) ----------

#[tokio::test(flavor = "multi_thread")]
async fn two_servers_keep_each_their_tunnel_and_a_cut_of_one_never_touches_the_other() {
    // Two `escouade-server`, behind one fake ssh: `travail` on the server port, `perso` on the next.
    let b = bench("two", VERSION, None).await;
    let data2 = DataDir::new(b.root.join("..").join("server-data-2"));
    data2.ensure().unwrap();
    let (api2, _stop2) = started(data2.clone()).await;
    let ssh = fake_ssh(
        &b.dir,
        &format!(
            "{}={},{}={}",
            tunnel::SERVER_PORT,
            b.port,
            tunnel::SERVER_PORT + 1,
            api2.addr().port()
        ),
        None,
    );
    let config = b.root.join("two-config");
    write(
        &config,
        "Host travail\n  HostName escouade.acme.internal\nHost perso\n  HostName vps.example.net\n",
    );
    let client = b.root.join("two");
    std::fs::create_dir_all(&client).unwrap();
    let store = Arc::new(MemoryStore::default());
    let servers = Servers::load(
        &client,
        store.clone(),
        ssh,
        VERSION.into(),
        Arc::new(|_: &str| Ok(())),
        Some(config),
    );
    let reached = ok(servers.attempt("travail", None, None).await);
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    let work = ok(servers
        .pair(&reached.attempt, &code.text, "Travail", "Poste")
        .await);
    let reached = ok(servers
        .attempt("perso", Some(tunnel::SERVER_PORT + 1), None)
        .await);
    let code = pairing::create(&data2, crate::model::now_ms()).unwrap();
    let home = ok(servers
        .pair(&reached.attempt, &code.text, "Perso", "Poste")
        .await);
    assert_ne!(work, home);

    // At the start, each server gets its tunnel, kept up.
    servers.resume();
    assert_eq!(servers.tunnels(), 2);
    let up = |id: &str| servers.link_of(id).map(|l| l.tunnel) == Some(TunnelState::Up);
    until("both tunnels up", || up(&work) && up(&home)).await;
    let links = servers.links();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].server.known.name, "Travail");
    assert_ne!(links[0].url, links[1].url, "a port each");
    let work_url = servers.link_of(&work).unwrap().url;

    // The window on Travail: through the tunnel kept up, not another.
    ok(servers.open(&work).await);
    assert_eq!(servers.link().unwrap().url, work_url);
    assert_eq!(servers.tunnels(), 2);

    // Perso cut, and unreachable a while: it tries again on its own; Travail's tunnel stays up the whole time.
    std::fs::write(b.dir.join("down-perso"), "").unwrap();
    std::fs::write(b.dir.join("cut-perso"), "").unwrap();
    let mut seen_down = false;
    for _ in 0..300 {
        assert!(up(&work), "Travail's tunnel never goes down");
        if matches!(servers.link_of(&home).map(|l| l.tunnel), Some(TunnelState::Starting { tries, .. }) if tries >= 1)
        {
            seen_down = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(seen_down, "Perso's tunnel said down");
    // A handshake through Travail's tunnel meanwhile: it answers, with this computer's token.
    let port: u16 = work_url
        .trim_start_matches("ws://127.0.0.1:")
        .split('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    crate::servers::client::hello(stream, port, VERSION, &servers.token(&work).unwrap())
        .await
        .expect("Travail answers while Perso is cut");
    // Perso back, tried again on its own (« Réessayer » on its pile): up, Travail untouched.
    std::fs::remove_file(b.dir.join("down-perso")).unwrap();
    servers.retry_of(&home);
    until("Perso up again", || up(&home)).await;
    assert!(up(&work));
    assert_eq!(servers.link_of(&work).unwrap().url, work_url);

    // Back on this computer: both spaces stay connected.
    servers.leave();
    assert_eq!(servers.tunnels(), 2);
    // Perso forgotten here (S15): its tunnel alone stops.
    assert!(servers.forget(&home).await, "Perso revoked this computer");
    assert_eq!(servers.tunnels(), 1);
    assert_eq!(servers.links().len(), 1);
    assert!(up(&work));
    servers.shutdown();
    assert_eq!(servers.tunnels(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_server_without_a_token_is_not_reached_at_the_start() {
    let b = bench("ensure", VERSION, None).await;
    let reached = ok(b.servers.attempt("travail", None, None).await);
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    let id = ok(b
        .servers
        .pair(&reached.attempt, &code.text, "Travail", "Poste")
        .await);
    // Its token gone from the keychain: no tunnel, said revoked.
    b.store.delete(&format!("server-token-{id}")).unwrap();
    b.servers.resume();
    assert_eq!(b.servers.tunnels(), 0);
    match b.servers.link_of(&id).map(|l| l.tunnel) {
        Some(TunnelState::Failed {
            problem: Problem::Revoked { .. },
        }) => {}
        t => panic!("{t:?}"),
    }
}

// ---------- no tunnel outlives its owner (ESC-541) ----------

/// The pids the fake `ssh` of `dir` noted: its forwards, and what they started.
fn noted_pids(dir: &Path) -> Vec<u32> {
    std::fs::read_to_string(dir.join("pids.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect()
}

/// Whether the process `pid` runs.
fn alive(pid: u32) -> bool {
    #[cfg(windows)]
    // SAFETY: a handle opened, read and closed here.
    unsafe {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(h, &mut code) != 0;
        CloseHandle(h);
        ok && code == STILL_ACTIVE as u32
    }
    #[cfg(unix)]
    {
        // A zombie is gone, as far as this test goes: `ps` says its state.
        let out = std::process::Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let stat = String::from_utf8_lossy(&out.stdout);
        out.status.success() && !stat.trim().is_empty() && !stat.trim().starts_with('Z')
    }
}

/// Waits (30 s at most) until none of `pids` runs.
async fn all_gone(what: &str, pids: &[u32]) {
    let until = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < until {
        if !pids.iter().any(|p| alive(*p)) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let left: Vec<u32> = pids.iter().copied().filter(|p| alive(*p)).collect();
    // Ended by their pids, which this test started: nothing of it outlives it, even when it fails.
    for pid in &left {
        #[cfg(windows)]
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
        #[cfg(unix)]
        // SAFETY: a signal to a process this test started.
        unsafe {
            libc::kill(*pid as i32, libc::SIGKILL);
        }
    }
    panic!("{what}: still running after 30 s: {left:?}");
}

/// Two servers paired, each with its tunnel kept up (ESC-539), the window on one, a port of it relayed (ESC-538) and the
/// dialog open (a tunnel of its own): the pids of the fake `ssh` of each, and of what each started.
async fn tunnels_open(b: &Bench) -> Vec<u32> {
    let reached = ok(b.servers.attempt("travail", None, None).await);
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    let id = ok(b
        .servers
        .pair(&reached.attempt, &code.text, "Travail", "Poste")
        .await);
    ok(b.servers.open(&id).await);
    assert_eq!(b.servers.link().unwrap().tunnel, TunnelState::Up);
    let dev = answering(b"vite").await;
    let relayed = b.servers.relay(dev, false).await.unwrap();
    assert_eq!(read_from(relayed.local).await, "vite");
    // Another space, its tunnel kept up beside the first.
    let reached = ok(b.servers.attempt("ailleurs", None, None).await);
    let code = pairing::create(&b.data, crate::model::now_ms()).unwrap();
    let other = ok(b
        .servers
        .pair(&reached.attempt, &code.text, "Ailleurs", "Poste")
        .await);
    b.servers.ensure(&other);
    let servers = b.servers.clone();
    until("the other space's tunnel up", || {
        servers.link_of(&other).map(|l| l.tunnel) == Some(TunnelState::Up)
    })
    .await;
    assert_eq!(b.servers.tunnels(), 2);
    ok(b.servers.attempt("travail", None, None).await);
    let pids = noted_pids(&b.dir);
    // The pairings', the two spaces', the relay's and the dialog's: six forwards at least, with a process each.
    assert!(pids.len() >= 12, "{pids:?}");
    pids
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tunnel_whose_owner_goes_without_stopping_it_ends_with_what_ssh_started() {
    let b = bench_with("abandoned", VERSION, None, "", true).await;
    let pids = tunnels_open(&b).await;
    let servers = Arc::downgrade(&b.servers);
    // The owner gone, never told to stop (no `leave`, no `shutdown`, no `cancel`).
    drop(b);
    assert!(
        servers.upgrade().is_none(),
        "nothing else holds the servers"
    );
    all_gone("the tunnels of servers dropped", &pids).await;
}

/// How the app ends in [`the_app_that_ends_leaves_no_tunnel_behind`]'s process: `shutdown` then `exit`, or `exit`
/// alone (the Windows update leaves so: `updates::leave`).
const EXIT_HELPER: &str = "ESCOUADE_TEST_SERVERS_EXIT";

/// The app of [`the_app_that_ends_leaves_no_tunnel_behind`]: run in a process of its own by it alone.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "run by the_app_that_ends_leaves_no_tunnel_behind in a process of its own"]
async fn an_app_with_tunnels_that_ends() {
    let Some(how) = std::env::var_os(EXIT_HELPER) else {
        return;
    };
    let how = how.to_string_lossy().into_owned();
    let (mode, out) = how.split_once('|').unwrap();
    let b = bench_with("exit", VERSION, None, "", true).await;
    // The pids, where the test reads them: this process's temporary folder goes with it.
    let pids = tunnels_open(&b).await;
    let pids: Vec<String> = pids.iter().map(u32::to_string).collect();
    std::fs::write(
        out,
        pids.join(
            "
",
        ),
    )
    .unwrap();
    if mode == "shutdown" {
        b.servers.shutdown();
    }
    // As the app does: no drop, no runtime shut down.
    std::process::exit(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_app_that_ends_leaves_no_tunnel_behind() {
    let dir = crate::paths::test_dir("servers-app-exit");
    // `exit` alone: the Windows update's way out, where the system ends the tunnels with the app's Job.
    let modes: &[&str] = if cfg!(windows) {
        &["shutdown", "exit"]
    } else {
        &["shutdown"]
    };
    for mode in modes {
        let out = dir.join(format!("{mode}.txt"));
        // Nextest's pipes are not handed down: a process left behind would hold them (« leaky »).
        #[cfg(windows)]
        let kept = crate::test_tmp::KeptHere::std_handles();
        let mut app = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "servers_tests::an_app_with_tunnels_that_ends",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .env(EXIT_HELPER, format!("{mode}|{}", out.display()))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        #[cfg(windows)]
        drop(kept);
        let mut ended = None;
        for _ in 0..600 {
            if let Some(status) = app.try_wait().unwrap() {
                ended = Some(status);
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if ended.is_none() {
            let _ = app.kill();
            panic!("{mode}: the app did not end in 60 s");
        }
        assert!(ended.unwrap().success(), "{mode}: the app failed");
        let pids: Vec<u32> = std::fs::read_to_string(&out)
            .unwrap()
            .lines()
            .filter_map(|l| l.trim().parse().ok())
            .collect();
        assert!(pids.len() >= 12, "{mode}: {pids:?}");
        all_gone(&format!("the tunnels of an app ended by {mode}"), &pids).await;
    }
}
