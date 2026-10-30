//! Tests of the plugins' runtime (PLG-2, ESC-174) on the fake plugin (tests/fixtures/fake-plugin.mjs,
//! started by its `.cmd` or its script without extension as a real plugin's program would be):
//! started at its first activation, `initialize` and the version of the protocol, calls and batches
//! of raw JSON, the default answer past a delay, the restarts after a crash and the queue kept
//! meanwhile, the memory limit, the journal, the secrets, and `shutdown` then the end of the job.

use crate::i18n::Lang;
use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::paths::DataDir;
use crate::plugins::journal::{Kind, MASK};
use crate::plugins::runtime::{
    secret_name, CallError, Limits, PhaseName, Runtime, Stop, METHOD_NOT_FOUND, NOT_YOURS,
};
use crate::plugins::Plugins;
use serde_json::value::RawValue;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const ID: &str = "dev.test.fake";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

struct Fake {
    rt: Arc<Runtime>,
    store: Arc<MemoryStore>,
    data: DataDir,
    plugins: Arc<Plugins>,
}

/// The fake plugin installed in development from a folder of the test's own, its program started
/// with `args`.
fn fake(name: &str, args: &[&str], limits: Limits) -> Fake {
    // Else cmd.exe may no longer find node (see `trim_test_path`).
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let base = std::env::temp_dir().join(format!(
        "plugin-runtime-{name}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let src = base.join("plugin");
    std::fs::create_dir_all(&src).unwrap();
    for f in ["fake-plugin.mjs", "fake-plugin.cmd", "fake-plugin"] {
        std::fs::copy(root().join("tests/fixtures").join(f), src.join(f)).unwrap();
    }
    let manifest = json!({
        "id": ID,
        "name": "Faux plugin",
        "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.1.0" },
        "run": { "command": "fake-plugin", "windows": "fake-plugin.cmd", "args": args },
        "activation": ["event:ticket.column"],
        "permissions": { "secrets": ["token", "account:*"] }
    });
    std::fs::write(
        src.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let data = DataDir::new(base.join("data"));
    let plugins = Plugins::load(data.clone(), "2.1.1");
    plugins.load_dev(&src, Lang::En).unwrap();
    let store = Arc::new(MemoryStore::default());
    let rt = Runtime::new(plugins.clone(), store.clone(), &data, "2.1.1", limits);
    Fake {
        rt,
        store,
        data,
        plugins,
    }
}

/// Limits for a test: its memory read often, room for a slow machine to start node.
fn quick() -> Limits {
    Limits {
        sample: Duration::from_millis(100),
        initialize: Duration::from_secs(20),
        shutdown: Duration::from_secs(2),
        ..Limits::default()
    }
}

async fn until(what: &str, f: impl Fn() -> bool) {
    let started = Instant::now();
    while !f() {
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "{what}: not in time"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    }
    #[cfg(unix)]
    {
        // SAFETY: signal 0 only asks whether the process exists.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

/// The pid of node, the fake plugin itself (under `cmd` on Windows).
async fn node_pid(rt: &Runtime) -> u32 {
    let state = rt
        .call(ID, "test/state", json!({}))
        .await
        .unwrap_or_else(|e| panic!("{e}: {:#?}", rt.journal(ID)));
    state["pid"].as_u64().unwrap() as u32
}

fn journal_has(rt: &Runtime, kind: Kind, text: &str) -> bool {
    rt.journal(ID)
        .iter()
        .any(|e| e.kind == kind && e.text.contains(text))
}

fn behave(what: &str) -> Value {
    json!({ "behave": what })
}

#[tokio::test(flavor = "multi_thread")]
async fn it_starts_at_its_first_activation_only_in_a_job_that_takes_its_tree() {
    let f = fake("lazy", &[], quick());
    assert_eq!(f.rt.status(ID).phase, PhaseName::Idle);
    assert!(f.rt.activate("event:agent.created").is_empty());
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(f.rt.status(ID).phase, PhaseName::Idle, "nothing started");
    assert!(f.rt.journal(ID).is_empty());

    assert_eq!(f.rt.activate("event:ticket.column"), [ID]);
    until("running", || f.rt.status(ID).phase == PhaseName::Running).await;
    assert!(
        f.rt.activate("event:ticket.column").is_empty(),
        "it runs already"
    );
    assert_eq!(f.rt.status(ID).capabilities, json!({ "batch": true }));
    assert!(journal_has(&f.rt, Kind::Start, "started"));
    assert!(journal_has(&f.rt, Kind::Ready, "protocol 1"));

    let echo = f.rt.call(ID, "echo", json!({ "a": 1 })).await.unwrap();
    assert_eq!(echo, json!({ "a": 1 }));

    // Under `cmd` (Windows), node is a child of the process started: the job takes both.
    let node = node_pid(&f.rt).await;
    let started = f.rt.status(ID).pid.unwrap();
    assert!(alive(node) && alive(started));
    f.rt.stop(ID);
    until("its tree gone", || !alive(node) && !alive(started)).await;
    assert_eq!(f.rt.status(ID).stop, Some(Stop::Asked));
    assert_eq!(
        f.rt.call(ID, "echo", json!({})).await,
        Err(CallError::Stopped(Stop::Asked))
    );

    assert!(matches!(
        f.rt.call("dev.test.absent", "echo", json!({})).await,
        Err(CallError::Unavailable(_))
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn initialize_negotiates_the_version_of_the_protocol() {
    let f = fake("version", &["--protocol", "2"], quick());
    let refused = f.rt.call(ID, "echo", json!({})).await;
    let Err(CallError::Stopped(Stop::Refused { reason })) = refused else {
        panic!("{refused:?}");
    };
    assert!(reason.contains("version 2"), "{reason}");
    assert!(reason.contains("version 1"), "{reason}");
    assert_eq!(f.rt.status(ID).phase, PhaseName::Stopped);
    assert!(journal_has(&f.rt, Kind::Stop, "version 2 of the protocol"));

    let limits = Limits {
        initialize: Duration::from_millis(1500),
        ..quick()
    };
    let f = fake("ignored", &["--ignore-initialize"], limits);
    let refused = f.rt.call(ID, "echo", json!({})).await;
    let Err(CallError::Stopped(Stop::Refused { reason })) = refused else {
        panic!("{refused:?}");
    };
    assert!(reason.contains("no answer to initialize"), "{reason}");
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_and_batches_go_both_ways_as_raw_json() {
    let f = fake("batch", &[], quick());
    let answers =
        f.rt.call_batch(
            ID,
            vec![
                ("echo".into(), json!({ "n": 1 })),
                (
                    "test/do".into(),
                    json!({ "behave": "answer", "say": "salut" }),
                ),
                ("nothing/here".into(), json!({})),
            ],
            Duration::from_secs(20),
        )
        .await;
    assert_eq!(answers[0], Ok(json!({ "n": 1 })));
    assert_eq!(answers[1], Ok(json!({ "ok": true, "said": "salut" })));
    assert!(
        matches!(&answers[2], Err(CallError::Remote { code, .. }) if *code == METHOD_NOT_FOUND),
        "{:?}",
        answers[2]
    );

    // A frame of a harness goes as it came, newlines and all, and the answer comes back raw.
    let frames = RawValue::from_string(
        "{\"frames\": [\n {\"type\":\"text\",\"text\":\"a\\nb\"},\n {\"type\":\"tool\"}\n]}".into(),
    )
    .unwrap();
    let answer =
        f.rt.call_raw(ID, "frames", &frames, Duration::from_secs(5))
            .await
            .unwrap();
    assert_eq!(answer.get(), r#"{"count":2}"#);

    // Its messages go to its journal.
    f.rt.call(
        ID,
        "test/do",
        json!({ "behave": "log", "message": "bonjour" }),
    )
    .await
    .unwrap();
    until("its log", || {
        journal_has(&f.rt, Kind::Message, "[info] bonjour")
    })
    .await;
    f.rt.notify(ID, "agent/hello", json!({ "agent": "a1" }))
        .unwrap();
    assert_eq!(f.rt.call(ID, "echo", json!(2)).await, Ok(json!(2)));
}

#[tokio::test(flavor = "multi_thread")]
async fn past_its_delay_a_call_takes_the_default_answer() {
    let f = fake("silent", &["--silent-decisions"], quick());
    f.rt.call(ID, "echo", json!({})).await.unwrap();

    let started = Instant::now();
    let silent =
        f.rt.call_within(ID, "test/do", behave("silent"), Duration::from_millis(300))
            .await;
    assert_eq!(silent, Err(CallError::Timeout));
    assert!(started.elapsed() >= Duration::from_millis(300));
    assert!(journal_has(
        &f.rt,
        Kind::Timeout,
        "test/do: no answer within 300 ms"
    ));

    // A decision has a short delay (500 ms): past it, the default applies.
    let started = Instant::now();
    let decided =
        f.rt.call_or(
            ID,
            "permissions/decide",
            json!({ "tool": "Bash" }),
            json!({ "decision": "ask" }),
        )
        .await;
    assert_eq!(decided, json!({ "decision": "ask" }));
    let took = started.elapsed();
    assert!(
        took >= Duration::from_millis(500) && took < Duration::from_secs(3),
        "{took:?}"
    );
    // It still answers what comes next.
    assert_eq!(f.rt.call(ID, "echo", json!(1)).await, Ok(json!(1)));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_restarts_it_three_times_within_five_minutes_then_stops_it() {
    let f = fake("crash", &[], quick());
    for n in 1..=3 {
        f.rt.call(ID, "echo", json!({})).await.unwrap();
        let crashed = f.rt.call(ID, "test/do", behave("crash")).await;
        assert_eq!(crashed, Err(CallError::Crashed));
        until("started again", || {
            let s = f.rt.status(ID);
            s.phase == PhaseName::Running && s.restarts == n
        })
        .await;
    }
    assert!(journal_has(&f.rt, Kind::Crash, "code 3"));
    assert!(journal_has(&f.rt, Kind::Crash, "crashing on purpose"));
    assert!(journal_has(&f.rt, Kind::Stderr, "crashing on purpose"));
    assert!(journal_has(&f.rt, Kind::Start, "starting again (3 of 3"));

    let crashed = f.rt.call(ID, "test/do", behave("crash")).await;
    assert_eq!(crashed, Err(CallError::Crashed));
    until("stopped", || f.rt.status(ID).phase == PhaseName::Stopped).await;
    assert_eq!(f.rt.status(ID).stop, Some(Stop::Crashed { times: 4 }));
    until("its stop told", || {
        journal_has(
            &f.rt,
            Kind::Stop,
            "stopped after 4 crashes within 5 minutes",
        )
    })
    .await;
    let started = Instant::now();
    assert_eq!(
        f.rt.call(ID, "echo", json!({})).await,
        Err(CallError::Stopped(Stop::Crashed { times: 4 }))
    );
    assert!(started.elapsed() < Duration::from_millis(100), "at once");

    // « Relancer »: started again, its crashes forgotten.
    f.rt.restart(ID).unwrap();
    assert_eq!(f.rt.call(ID, "echo", json!(1)).await, Ok(json!(1)));
    assert_eq!(f.rt.status(ID).restarts, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn while_it_restarts_messages_wait_and_what_was_attached_is_given_again() {
    let f = fake(
        "attach",
        &[],
        Limits {
            queue: 2,
            ..quick()
        },
    );
    // One process for several agents, told apart by their ids.
    for agent in ["agent-1", "agent-2"] {
        f.rt.attach(ID, agent, "attach", json!({ "key": agent }))
            .await
            .unwrap();
    }
    let before = node_pid(&f.rt).await;
    f.rt.detach(ID, "agent-gone");

    assert_eq!(
        f.rt.call(ID, "test/do", behave("crash")).await,
        Err(CallError::Crashed)
    );
    assert_eq!(f.rt.status(ID).phase, PhaseName::Restarting);
    // Three while it starts again, in a queue of two: the oldest is dropped.
    let (a, b, c) = tokio::join!(
        f.rt.call(ID, "echo", json!("a")),
        f.rt.call(ID, "echo", json!("b")),
        f.rt.call(ID, "echo", json!("c")),
    );
    assert_eq!(a, Err(CallError::Dropped));
    assert_eq!(b, Ok(json!("b")));
    assert_eq!(c, Ok(json!("c")));
    assert!(journal_has(&f.rt, Kind::Dropped, "the oldest message"));

    let state = f.rt.call(ID, "test/state", json!({})).await.unwrap();
    assert_ne!(state["pid"].as_u64().unwrap() as u32, before);
    assert_eq!(state["attached"], json!(["agent-1", "agent-2"]));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_loop_takes_the_default_and_shutdown_ends_its_job() {
    let f = fake("loop", &[], quick());
    let node = node_pid(&f.rt).await;
    let looping =
        f.rt.call_within(ID, "test/do", behave("loop"), Duration::from_millis(300))
            .await;
    assert_eq!(looping, Err(CallError::Timeout));
    assert_eq!(
        f.rt.call_or(ID, "permissions/decide", json!({}), json!("default"))
            .await,
        json!("default"),
        "busy: nothing more is answered"
    );

    let rt = f.rt.clone();
    let started = Instant::now();
    tokio::task::spawn_blocking(move || rt.shutdown())
        .await
        .unwrap();
    let took = started.elapsed();
    assert!(took < Duration::from_secs(4), "{took:?}");
    until("node gone", || !alive(node)).await;
    assert!(journal_has(
        &f.rt,
        Kind::Timeout,
        "shutdown: no answer within 2000 ms"
    ));
    assert_eq!(f.rt.status(ID).stop, Some(Stop::Shutdown));
    assert_eq!(
        f.rt.call(ID, "echo", json!(1)).await,
        Err(CallError::Stopped(Stop::Shutdown))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn at_the_close_shutdown_is_asked_first() {
    let f = fake("shutdown", &[], quick());
    let node = node_pid(&f.rt).await;
    let rt = f.rt.clone();
    let started = Instant::now();
    tokio::task::spawn_blocking(move || rt.shutdown())
        .await
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(journal_has(&f.rt, Kind::Stop, "answered shutdown"));
    until("node gone", || !alive(node)).await;
    assert!(
        !f.rt.journal(ID).iter().any(|e| e.kind == Kind::Crash),
        "an end asked for is no crash"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn past_its_memory_limit_it_is_stopped() {
    let f = fake(
        "grow",
        &[],
        Limits {
            memory: 200 << 20,
            ..quick()
        },
    );
    let node = node_pid(&f.rt).await;
    let _ =
        f.rt.call_within(
            ID,
            "test/do",
            json!({ "behave": "grow", "mb": 500 }),
            Duration::from_secs(30),
        )
        .await;
    until("stopped", || f.rt.status(ID).phase == PhaseName::Stopped).await;
    let Some(Stop::Memory { bytes, limit }) = f.rt.status(ID).stop else {
        panic!("{:?}", f.rt.status(ID));
    };
    assert_eq!(limit, 200 << 20);
    assert!(bytes > limit);
    until("its stop told", || {
        journal_has(&f.rt, Kind::Stop, "past the limit of 200 MB")
    })
    .await;
    until("node gone", || !alive(node)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn its_standard_error_goes_to_its_journal_its_secrets_masked() {
    let f = fake("stderr", &[], quick());
    f.store
        .set(&secret_name(ID, "token"), "s3cr3t-value")
        .unwrap();
    let got =
        f.rt.call(ID, "test/do", json!({ "behave": "secret", "key": "token" }))
            .await
            .unwrap();
    assert_eq!(got, json!({ "result": "s3cr3t-value" }));
    f.rt.call(
        ID,
        "test/do",
        json!({ "behave": "stderr", "lines": ["hello from stderr", "leaked: s3cr3t-value"] }),
    )
    .await
    .unwrap();
    until("its lines", || {
        journal_has(&f.rt, Kind::Stderr, &format!("leaked: {MASK}"))
    })
    .await;
    assert!(journal_has(&f.rt, Kind::Stderr, "hello from stderr"));
    assert!(journal_has(
        &f.rt,
        Kind::Message,
        "was given its secret \"token\""
    ));
    let file = std::fs::read_to_string(f.data.plugin_logs_dir().join(format!("{ID}.log"))).unwrap();
    assert!(file.contains("stderr hello from stderr"), "{file}");
    assert!(!file.contains("s3cr3t-value"), "{file}");
    assert!(f
        .rt
        .journal(ID)
        .iter()
        .all(|e| !e.text.contains("s3cr3t-value")));
}

#[tokio::test(flavor = "multi_thread")]
async fn it_gets_its_own_secrets_and_never_another_s() {
    let f = fake("steal", &[], quick());
    f.store.set(&secret_name(ID, "token"), "mine-1234").unwrap();
    f.store
        .set(&secret_name("dev.test.other", "token"), "other-5678")
        .unwrap();
    f.store.set("github", "account-9012").unwrap();

    for key in ["plugin:dev.test.other:token", "webhook", "github", ""] {
        let got =
            f.rt.call(ID, "test/do", json!({ "behave": "steal", "key": key }))
                .await
                .unwrap();
        assert_eq!(got["error"]["code"], json!(NOT_YOURS), "{key}: {got}");
        assert!(!got.to_string().contains("other-5678"));
    }
    assert!(journal_has(
        &f.rt,
        Kind::Refused,
        "asked for the secret \"plugin:dev.test.other:token\", which is not one of its own"
    ));
    // A key its pattern takes in is still read under its own name: not the integration's.
    let got =
        f.rt.call(
            ID,
            "test/do",
            json!({ "behave": "steal", "key": "account:github" }),
        )
        .await
        .unwrap();
    assert_eq!(got, json!({ "result": null }));
    let got =
        f.rt.call(ID, "test/do", json!({ "behave": "secret", "key": "token" }))
            .await
            .unwrap();
    assert_eq!(got, json!({ "result": "mine-1234" }));
}

/// The round trip of a call and the flow of raw frames, on this machine (ESC-203): printed, to
/// set beside INT-0's figures and §2.2 of docs/PLUGINS.md (43 µs a round trip, measured in Rust).
#[tokio::test(flavor = "multi_thread")]
async fn round_trips_and_throughput_are_measured() {
    let f = fake("measure", &[], quick());
    f.rt.call(ID, "echo", json!({})).await.unwrap();
    let mut times = Vec::new();
    for i in 0..2000 {
        let started = Instant::now();
        f.rt.call(ID, "echo", json!({ "i": i })).await.unwrap();
        times.push(started.elapsed());
    }
    times.sort();
    let median = times[times.len() / 2];
    let p99 = times[times.len() * 99 / 100];

    // Batches of 64 frames of about 1 KiB, as a read of a harness's output gives them.
    let frame = json!({ "type": "assistant", "text": "x".repeat(1000) }).to_string();
    let batch = format!("{{\"frames\":[{}]}}", vec![frame.as_str(); 64].join(","));
    let params = RawValue::from_string(batch.clone()).unwrap();
    let started = Instant::now();
    for _ in 0..200 {
        let answer =
            f.rt.call_raw(ID, "frames", &params, Duration::from_secs(10))
                .await
                .unwrap();
        assert_eq!(answer.get(), r#"{"count":64}"#);
    }
    let took = started.elapsed();
    let mb = (batch.len() * 200) as f64 / (1 << 20) as f64;
    let rate = mb / took.as_secs_f64();
    let frames = 64.0 * 200.0 / took.as_secs_f64();
    eprintln!(
        "plugin round trip: median {median:?}, p99 {p99:?}; frames: {rate:.1} MB/s, {frames:.0} frames/s"
    );
    assert!(p99 < Duration::from_millis(50), "p99 {p99:?}");
    assert!(rate > 1.0, "{rate:.1} MB/s");
}

/// Réglages › Plugins (PLG-4): a permission taken back, the app's switch, the settings and
/// « Démarrer » reach the process once its host is forgotten; the journal outlives the host.
#[tokio::test(flavor = "multi_thread")]
async fn the_users_choices_reach_its_process_once_its_host_is_forgotten() {
    let f = fake("choices", &[], quick());
    f.store.set(&secret_name(ID, "token"), "mine-1234").unwrap();
    let secret = || {
        f.rt.call(ID, "test/do", json!({ "behave": "secret", "key": "token" }))
    };
    assert_eq!(secret().await.unwrap(), json!({ "result": "mine-1234" }));

    // Its settings, as the user set them.
    f.plugins
        .choose(ID, |c| {
            c.settings.insert("channel".into(), json!("#escouade"));
        })
        .unwrap();
    let got = f.rt.call(ID, "test/do", behave("settings")).await.unwrap();
    assert_eq!(got, json!({ "result": { "channel": "#escouade" } }));

    // The secret taken back: the next process is no longer given it.
    f.plugins
        .choose(ID, |c| c.revoked.push("secrets:token".into()))
        .unwrap();
    f.rt.forget(ID);
    assert!(journal_has(&f.rt, Kind::Stop, "stopped"));
    let got = secret().await.unwrap();
    assert_eq!(got["error"]["code"], json!(NOT_YOURS), "{got}");

    // Switched off: nothing of it runs.
    f.plugins.choose(ID, |c| c.enabled = false).unwrap();
    f.rt.forget(ID);
    let err = f.rt.call(ID, "echo", json!({})).await.unwrap_err();
    assert!(
        matches!(&err, CallError::Unavailable(why) if why.contains("switched off")),
        "{err:?}"
    );
    assert!(f.rt.start(ID).is_err());
    // Its journal is read from its file meanwhile: the earlier runs are still there.
    assert!(journal_has(&f.rt, Kind::Ready, ""));

    // Switched on again, « Démarrer » starts it without a call.
    f.plugins.choose(ID, |c| c.enabled = true).unwrap();
    f.rt.start(ID).unwrap();
    until("running", || f.rt.status(ID).phase == PhaseName::Running).await;
    f.rt.shutdown();
}

/// A folder in development read again (PLG-4, the review of PLG-2): its process stops and its host
/// is forgotten, so that the next process is given the manifest of now, not the one it started with.
#[tokio::test(flavor = "multi_thread")]
async fn a_folder_in_development_read_again_stops_its_process_for_the_new_manifest() {
    let f = fake("dev-reload", &[], quick());
    f.store.set(&secret_name(ID, "token"), "mine-1234").unwrap();
    f.store
        .set(&secret_name(ID, "other"), "other-5678")
        .unwrap();
    let ask = |key: &'static str| {
        f.rt.call(ID, "test/do", json!({ "behave": "secret", "key": key }))
    };
    assert_eq!(
        ask("token").await.unwrap(),
        json!({ "result": "mine-1234" })
    );
    let refused = ask("other").await.unwrap();
    assert_eq!(refused["error"]["code"], json!(NOT_YOURS));

    // The manifest of the folder now asks for `other` too.
    let folder = PathBuf::from(f.plugins.get(ID, Lang::En).unwrap().folder);
    let file = folder.join("escouade-plugin.json");
    let mut manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    manifest["permissions"]["secrets"] = json!(["token", "other"]);
    let rev = f.plugins.revision();
    std::fs::write(&file, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    until("read again", || f.plugins.revision() > rev).await;
    until("stopped", || f.rt.status(ID).phase == PhaseName::Idle).await;
    assert_eq!(
        ask("other").await.unwrap(),
        json!({ "result": "other-5678" })
    );
    f.rt.shutdown();
}

/// An event of the watcher of a folder in development that changed nothing (FSEvents delivers late
/// the writes made just before the watch began; a file rewritten as it was): nothing is read again,
/// and its process goes on, the same one.
#[tokio::test(flavor = "multi_thread")]
async fn an_event_that_changed_nothing_leaves_its_process_running() {
    let f = fake("dev-same", &[], quick());
    let pid = node_pid(&f.rt).await;

    // Told directly, as a late event would.
    f.plugins.reload(&[ID.to_string()]);
    // The manifest written again, the same, byte for byte: a real event of the watcher.
    let folder = PathBuf::from(f.plugins.get(ID, Lang::En).unwrap().folder);
    let file = folder.join("escouade-plugin.json");
    let rev = f.plugins.revision();
    std::fs::write(&file, std::fs::read(&file).unwrap()).unwrap();
    until("the event read", || f.plugins.revision() > rev).await;

    assert_eq!(f.rt.status(ID).phase, PhaseName::Running);
    assert_eq!(node_pid(&f.rt).await, pid, "the same process");
    assert!(!journal_has(&f.rt, Kind::Stop, "stopped"));
    f.rt.shutdown();
}

/// A plugin given by a relative folder (`escouade-plugin test slack`, run beside it) starts the
/// program of that folder: the path is absolute, never looked for again from the folder it starts
/// in, where Unix would find `slack/slack/slack` (ESC-182).
#[test]
fn a_relative_folder_gives_an_absolute_program() {
    let run = crate::plugins::manifest::Run {
        command: "slack".into(),
        args: vec![],
        by_system: Default::default(),
    };
    let here = std::env::current_dir().unwrap();
    let p = crate::plugins::runtime::program_path(Path::new("slack"), &run, Path::new("app"));
    assert!(p.is_absolute(), "{}", p.display());
    assert_eq!(p, here.join("slack").join("slack"));
    let app = crate::plugins::manifest::Run {
        command: "{escouade}/escouade-plugins".into(),
        ..run
    };
    let p = crate::plugins::runtime::program_path(Path::new("slack"), &app, Path::new("app"));
    assert_eq!(p, here.join("app").join("escouade-plugins"));
}
