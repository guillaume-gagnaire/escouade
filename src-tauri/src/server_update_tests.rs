//! The server updates itself (ESC-702, `server::autoupdate`): the timer of its checks, its channel, the automatic off,
//! the restart at rest only, never an older version, nothing replaced in the image, a refused signature never
//! downloaded again, the request of an administrator, `escouade-server config`.

use crate::paths::DataDir;
use crate::server::autoupdate::{self, Choice, Phase, ServerUpdate, Source};
use crate::server::status::Supervisor;
use crate::server::update;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CURRENT: &str = "2.3.0";

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/server-update")
            .join(name),
    )
    .unwrap()
}

fn channel(version: &str) -> Vec<u8> {
    json!({ "version": version, "platforms": {} })
        .to_string()
        .into_bytes()
}

/// A fake release on 127.0.0.1: each path of `files` answered with its bytes, any other 404; the paths asked noted.
fn releases(files: Vec<(String, Vec<u8>)>) -> (String, Arc<Mutex<Vec<String>>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/r", listener.local_addr().unwrap());
    let asked = Arc::new(Mutex::new(Vec::new()));
    let noted = asked.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let text = String::from_utf8_lossy(&request).to_string();
            let path = text
                .lines()
                .next()
                .and_then(|l| l.split(' ').nth(1))
                .unwrap_or("")
                .to_string();
            noted.lock().unwrap().push(path.clone());
            let (status, body) = match files.iter().find(|(p, _)| format!("/r/{p}") == path) {
                Some((_, b)) => ("200 OK", b.clone()),
                None => ("404 Not Found", Vec::new()),
            };
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .as_bytes(),
            );
            let _ = stream.write_all(&body);
        }
    });
    (base, asked)
}

/// A release whose latest.json says `latest` and beta.json `beta` (when given), with the signed archive of 9.9.9, and the
/// same served as 9.9.10's (its signature names 9.9.9: refused).
fn release(latest: &str, beta: Option<&str>) -> (String, Arc<Mutex<Vec<String>>>) {
    let archive = fixture("escouade-server-9.9.9-linux-x64.tar.gz");
    let sig = fixture("escouade-server-9.9.9-linux-x64.tar.gz.sig");
    let name = |v: &str| format!("download/v{v}/{}", update::archive_name(v, "x64"));
    let mut files = vec![
        ("latest/download/latest.json".to_string(), channel(latest)),
        (name("9.9.9"), archive.clone()),
        (format!("{}.sig", name("9.9.9")), sig.clone()),
        (name("9.9.10"), archive),
        (format!("{}.sig", name("9.9.10")), sig),
    ];
    if let Some(b) = beta {
        files.push(("latest/download/beta.json".into(), channel(b)));
    }
    releases(files)
}

/// A server of version `CURRENT` whose program is in a folder of its own (with its program, as installed), on the
/// release at `base`, under `supervisor`, putting versions in place when `installs`.
fn server(
    name: &str,
    base: &str,
    supervisor: Option<Supervisor>,
    installs: bool,
) -> (Arc<ServerUpdate>, PathBuf, DataDir) {
    let dir = crate::paths::test_dir(&format!("autoupdate-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    let program = dir.join("program");
    std::fs::create_dir_all(&program).unwrap();
    std::fs::write(program.join(update::PROGRAM), "the old program").unwrap();
    let data = DataDir::new(dir.join("data"));
    std::fs::create_dir_all(data.root()).unwrap();
    let source = Source {
        client: reqwest::Client::builder().no_proxy().build().unwrap(),
        base: base.to_string(),
        key: String::from_utf8(fixture("test-key.pub")).unwrap(),
        arch: "x64".into(),
    };
    let u = ServerUpdate::with_periods(
        CURRENT,
        data.clone(),
        source,
        autoupdate::target_in(&program, supervisor, installs),
        Duration::from_secs(120),
        Duration::from_secs(900),
    );
    (u, program, data)
}

fn on() -> Choice {
    Choice {
        automatic: true,
        beta: false,
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn program_of(dir: &Path) -> String {
    std::fs::read_to_string(dir.join(update::PROGRAM)).unwrap()
}

#[test]
fn it_looks_two_minutes_after_its_start_then_every_quarter_of_an_hour() {
    assert_eq!(autoupdate::FIRST, Duration::from_secs(120));
    assert_eq!(autoupdate::EVERY, Duration::from_secs(900));
    let (first, every) = (autoupdate::FIRST, autoupdate::EVERY);
    let started = Instant::now();
    let at = |s: u64| started + Duration::from_secs(s);
    // Not before two minutes, then at once.
    assert!(!autoupdate::due(started, None, at(119), first, every));
    assert!(autoupdate::due(started, None, at(120), first, every));
    // Once it looked, a quarter of an hour after the last look.
    assert!(!autoupdate::due(
        started,
        Some(at(120)),
        at(1019),
        first,
        every
    ));
    assert!(autoupdate::due(
        started,
        Some(at(120)),
        at(1020),
        first,
        every
    ));
}

#[test]
fn its_switch_and_its_channel_are_the_settings_of_the_app_within_the_policies() {
    use crate::locks::{Locks, Managed, KEYS};
    let none = Locks::compute(KEYS, None, &Managed::default());
    // As the app: on, stable, by default.
    let s = crate::model::Settings::default();
    assert_eq!(autoupdate::choice_of(&s, &none), on());
    let beta = crate::model::Settings {
        beta_updates: true,
        auto_update: false,
        ..Default::default()
    };
    assert_eq!(
        autoupdate::choice_of(&beta, &none),
        Choice {
            automatic: false,
            beta: true
        }
    );
    // A policy that turns the automatic off, or keeps to the stable channel, wins over the settings.
    let mut managed = Managed::default();
    managed.values.insert(
        crate::locks::AUTO_UPDATE.into(),
        (Value::Bool(false), "test".into()),
    );
    managed.values.insert(
        crate::locks::CHANNEL.into(),
        (Value::String("stable".into()), "test".into()),
    );
    let locked = Locks::compute(KEYS, None, &managed);
    let all_on = crate::model::Settings {
        beta_updates: true,
        ..Default::default()
    };
    assert_eq!(
        autoupdate::choice_of(&all_on, &locked),
        Choice {
            automatic: false,
            beta: false
        }
    );
}

#[test]
fn it_finds_checks_and_puts_a_version_of_its_channel_in_place_keeping_the_old_files() {
    let (base, _) = release("9.9.9", None);
    let (u, dir, _) = server("found", &base, Some(Supervisor::Systemd), true);
    assert_eq!(runtime().block_on(u.check(on())), Phase::Ready);
    let s = u.state();
    assert_eq!(s.available.as_deref(), Some("9.9.9"));
    assert_eq!(s.channel, "stable");
    assert!(s.error.is_none(), "{s:?}");
    // The archive's program in place, the old one kept.
    assert_ne!(program_of(&dir), "the old program");
    assert_eq!(
        std::fs::read_to_string(dir.join(format!("{}.old", update::PROGRAM))).unwrap(),
        "the old program"
    );
    assert!(dir.join("seed").is_dir());
}

#[test]
fn the_automatic_off_says_the_version_and_changes_nothing() {
    let (base, asked) = release("9.9.9", None);
    let (u, dir, _) = server("off", &base, Some(Supervisor::Systemd), true);
    let off = Choice {
        automatic: false,
        beta: false,
    };
    assert_eq!(runtime().block_on(u.check(off)), Phase::Available);
    let s = u.state();
    assert_eq!(s.available.as_deref(), Some("9.9.9"));
    assert_eq!(
        s.command.as_deref(),
        Some("escouade-server update --to 9.9.9")
    );
    assert_eq!(program_of(&dir), "the old program");
    // latest.json read, no archive downloaded.
    assert!(
        asked.lock().unwrap().iter().all(|p| p.ends_with(".json")),
        "{:?}",
        asked.lock().unwrap()
    );
}

#[test]
fn it_follows_its_channel_and_never_an_older_version() {
    // latest.json older than the program, beta.json newer.
    let (base, _) = release("1.0.0", Some("9.9.9"));
    let (u, dir, _) = server("channel", &base, Some(Supervisor::Systemd), true);
    let rt = runtime();
    // The stable channel: nothing newer, nothing changed (never 1.0.0).
    assert_eq!(rt.block_on(u.check(on())), Phase::UpToDate);
    assert!(u.state().available.is_none());
    assert_eq!(program_of(&dir), "the old program");
    // The beta channel: the higher of the two, put in place.
    let beta = Choice {
        automatic: true,
        beta: true,
    };
    assert_eq!(rt.block_on(u.check(beta)), Phase::Ready);
    assert_eq!(u.state().available.as_deref(), Some("9.9.9"));
    // Only beta.json older too: still nothing.
    let (base, _) = release("1.0.0", Some("2.2.19"));
    let (u, dir, _) = server("older", &base, Some(Supervisor::Systemd), true);
    assert_eq!(rt.block_on(u.check(beta)), Phase::UpToDate);
    assert_eq!(program_of(&dir), "the old program");
}

#[test]
fn in_the_image_nothing_is_replaced_and_the_new_image_is_said() {
    let (base, asked) = release("9.9.9", None);
    let (u, dir, _) = server("image", &base, Some(Supervisor::Docker), true);
    assert_eq!(runtime().block_on(u.check(on())), Phase::Image);
    let s = u.state();
    assert!(s.image && !s.installs, "{s:?}");
    assert_eq!(s.available.as_deref(), Some("9.9.9"));
    assert_eq!(
        s.command.as_deref(),
        Some("docker pull ghcr.io/guillaume-gagnaire/escouade/escouade-server:9.9.9 && docker compose up -d")
    );
    assert_eq!(program_of(&dir), "the old program");
    assert!(asked.lock().unwrap().iter().all(|p| p.ends_with(".json")));
    // Nor at an administrator's request.
    let e = u.ask("9.9.9").unwrap_err();
    assert!(e.to_string().contains("docker pull"), "{e}");
}

#[test]
fn without_systemd_it_says_the_version_and_changes_nothing() {
    let (base, _) = release("9.9.9", None);
    let (u, dir, _) = server("nosystemd", &base, None, false);
    assert_eq!(runtime().block_on(u.check(on())), Phase::Available);
    assert_eq!(program_of(&dir), "the old program");
    assert!(u.ask("9.9.9").is_err());
}

#[test]
fn a_signature_refused_changes_nothing_and_is_never_downloaded_again_by_itself() {
    let (base, asked) = release("9.9.10", None);
    let (u, dir, _) = server("refused", &base, Some(Supervisor::Systemd), true);
    let rt = runtime();
    assert_eq!(rt.block_on(u.check(on())), Phase::Failed);
    let s = u.state();
    assert!(
        s.error.as_deref().unwrap_or("").contains("signature"),
        "{s:?}"
    );
    assert_eq!(program_of(&dir), "the old program");
    let archives = |a: &Arc<Mutex<Vec<String>>>| {
        a.lock()
            .unwrap()
            .iter()
            .filter(|p| p.ends_with(".tar.gz"))
            .count()
    };
    assert_eq!(archives(&asked), 1);
    // The next look reads latest.json, never the archive again.
    rt.block_on(u.check(on()));
    assert_eq!(archives(&asked), 1);
    assert_eq!(program_of(&dir), "the old program");
}

#[test]
fn a_version_ready_restarts_at_rest_only_and_the_next_start_says_it() {
    let (base, _) = release("9.9.9", None);
    let (u, _, data) = server("rest", &base, Some(Supervisor::Systemd), true);
    let rt = runtime();
    // Nothing ready: nothing to restart.
    assert!(!u.at_rest(None));
    assert_eq!(rt.block_on(u.check(on())), Phase::Ready);
    // Busy: it waits, and says why.
    for why in ["turn", "question", "background", "launch", "terminal"] {
        assert!(!u.at_rest(Some(why)), "{why}");
        assert_eq!(u.state().phase, Phase::Ready);
        assert_eq!(u.state().busy.as_deref(), Some(why));
    }
    // At rest: it stops, the restart written for the next start.
    let fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let restarting = {
        let (u, fired) = (u.clone(), fired.clone());
        rt.spawn(async move {
            u.restarting().await;
            fired.store(true, std::sync::atomic::Ordering::SeqCst);
        })
    };
    assert!(u.at_rest(None));
    rt.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), restarting)
            .await
            .unwrap()
            .unwrap()
    });
    assert!(fired.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(u.state().phase, Phase::Restarting);
    let pending: autoupdate::Pending =
        serde_json::from_slice(&std::fs::read(data.root().join(autoupdate::PENDING)).unwrap())
            .unwrap();
    assert_eq!(
        (pending.from.as_str(), pending.to.as_str()),
        (CURRENT, "9.9.9")
    );
    // Started again at another version than the one it put (the update did not take): never tried again by itself.
    let (again, dir, _) = server_on("rest-again", &base, data.clone());
    again.after_start();
    assert!(!data.root().join(autoupdate::PENDING).exists());
    assert_eq!(rt.block_on(again.check(on())), Phase::Failed);
    assert_eq!(program_of(&dir), "the old program");
}

/// The server of `server`, on the data folder `data`.
fn server_on(name: &str, base: &str, data: DataDir) -> (Arc<ServerUpdate>, PathBuf, DataDir) {
    let (_, dir, _) = server(name, base, Some(Supervisor::Systemd), true);
    let source = Source {
        client: reqwest::Client::builder().no_proxy().build().unwrap(),
        base: base.to_string(),
        key: String::from_utf8(fixture("test-key.pub")).unwrap(),
        arch: "x64".into(),
    };
    let u = ServerUpdate::with_periods(
        CURRENT,
        data.clone(),
        source,
        autoupdate::target_in(&dir, Some(Supervisor::Systemd), true),
        Duration::from_secs(120),
        Duration::from_secs(900),
    );
    (u, dir, data)
}

#[test]
fn an_administrator_asks_a_version_never_older_and_it_is_put_in_place_with_the_automatic_off() {
    let (base, _) = release("1.0.0", None);
    let (u, dir, _) = server("asked", &base, Some(Supervisor::Systemd), true);
    // Older, the same, or not a version: refused, nothing asked.
    assert!(u.ask("1.0.0").is_err());
    assert!(u.ask(CURRENT).is_err());
    assert!(u.ask("9.9.9; rm -rf /").is_err());
    let off = Choice {
        automatic: false,
        beta: false,
    };
    // Asked: put in place at the next look, though the automatic is off and latest.json says older.
    assert_eq!(u.ask("9.9.9").unwrap(), "9.9.9");
    assert_eq!(runtime().block_on(u.check(off)), Phase::Ready);
    assert_ne!(program_of(&dir), "the old program");
}

#[test]
fn config_writes_the_two_settings_and_keeps_the_rest_of_the_file() {
    let (_, _, data) = server("config", "http://127.0.0.1:9/r", None, false);
    std::fs::write(
        data.settings_file(),
        r#"{"language":"fr","autoUpdate":true,"unknownKey":7}"#,
    )
    .unwrap();
    autoupdate::write_settings(
        &data,
        Some(false),
        Some(true),
        &crate::locks::Managed::default(),
    )
    .unwrap();
    let v: Value = serde_json::from_slice(&std::fs::read(data.settings_file()).unwrap()).unwrap();
    assert_eq!(v["autoUpdate"], json!(false));
    assert_eq!(v["betaUpdates"], json!(true));
    assert_eq!(v["language"], json!("fr"));
    assert_eq!(v["unknownKey"], json!(7));
    // A policy that keeps to the stable channel refuses the beta's, nothing written.
    let mut managed = crate::locks::Managed::default();
    managed.values.insert(
        crate::locks::CHANNEL.into(),
        (Value::String("stable".into()), "test".into()),
    );
    std::fs::write(data.settings_file(), r#"{"betaUpdates":false}"#).unwrap();
    assert!(autoupdate::write_settings(&data, None, Some(true), &managed).is_err());
    let v: Value = serde_json::from_slice(&std::fs::read(data.settings_file()).unwrap()).unwrap();
    assert_eq!(v["betaUpdates"], json!(false));
}

#[test]
fn the_command_line_reads_config() {
    use crate::server::{parse_with, Command};
    let parse = |line: &str| {
        let args: Vec<String> = line.split_whitespace().map(String::from).collect();
        parse_with(&args, |_| None).map(|a| a.command)
    };
    assert_eq!(
        parse("config"),
        Ok(Command::Config {
            automatic: None,
            beta: None
        })
    );
    assert_eq!(
        parse("config automatic off channel beta"),
        Ok(Command::Config {
            automatic: Some(false),
            beta: Some(true)
        })
    );
    assert_eq!(
        parse("config channel stable"),
        Ok(Command::Config {
            automatic: None,
            beta: Some(false)
        })
    );
    assert!(parse("config automatic maybe").is_err());
    assert!(parse("config channel").is_err());
    assert!(crate::server::help().contains("escouade-server config"));
}
