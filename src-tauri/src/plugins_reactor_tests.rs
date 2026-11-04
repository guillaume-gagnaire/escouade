//! Tests of the `run.wasm` plugins (ESC-331, docs/PLUGINS.md §8.3): the module of tests/fixtures/plugins/reactor, a Rust
//! reactor whose bytes are committed, in the app's host: `serve` on one channel, one turn for what came together,
//! `net/fetch` beside the thread, `escouade.call`, the limits, the runtime, the cache, the bench and `validate`.
//!
//! No escouade-js nor Javy here: a reactor is compiled by nothing but Cranelift.

use crate::i18n::Lang;
use crate::integrations::secrets::MemoryStore;
use crate::paths::DataDir;
use crate::plugins::bench::{cli, run, Options, Verdict};
use crate::plugins::journal::Kind;
use crate::plugins::manifest::Hand;
use crate::plugins::runtime::{CallError, Limits, Runtime};
use crate::plugins::wasm::{self, Calls, Fault, Fetcher, Frame, Instance, Made, Net, Wiring};
use crate::plugins::Plugins;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/plugins/reactor")
}

fn module() -> wasmtime::Module {
    wasm::reactor(&std::fs::read(fixture().join(wasm::MODULE_FILE)).unwrap()).unwrap()
}

/// A copy of the fixture, in a folder of its own: its manifest, its module, its `Cargo.toml`.
fn copied(name: &str) -> PathBuf {
    let to = std::env::temp_dir()
        .join(format!(
            "plugin-reactor-{name}-{}",
            uuid::Uuid::new_v4().simple()
        ))
        .join("reactor");
    std::fs::create_dir_all(&to).unwrap();
    for f in ["escouade-plugin.json", "Cargo.toml", wasm::MODULE_FILE] {
        std::fs::copy(fixture().join(f), to.join(f)).unwrap();
    }
    to
}

/// The module's instance on a thread of its own, as `spawn_wasm` makes it, on the module of the fixture.
struct Served {
    tx: Option<UnboundedSender<Frame>>,
    lines: Receiver<String>,
    refused: Arc<Mutex<Vec<String>>>,
    ended: Receiver<Option<Fault>>,
}

fn line(id: u64, method: &str, params: Value) -> Frame {
    let text = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
    (format!("{text}\n").into_bytes(), None)
}

impl Served {
    /// `first`, written before the thread starts, comes to it together.
    fn start(
        fetcher: Fetcher,
        limits: wasm::Limits,
        calls: Option<Calls>,
        first: Vec<Frame>,
    ) -> Self {
        let (tx, rx) = unbounded_channel::<Frame>();
        for f in first {
            tx.send(f).unwrap();
        }
        let back = tx.downgrade();
        let (out, lines) = channel();
        let (done, ended) = channel();
        let refused = Arc::new(Mutex::new(Vec::new()));
        let said = refused.clone();
        let m = module();
        std::thread::spawn(move || {
            let fault = match Instance::start_with(&m, &BTreeMap::new(), limits, calls) {
                Err(f) => Some(f),
                Ok(instance) => wasm::serve(
                    instance,
                    fetcher,
                    Wiring {
                        rx,
                        back,
                        line: Box::new(move |l| {
                            let _ = out.send(l.to_string());
                        }),
                        stderr: Box::new(|_| {}),
                        refused: Arc::new(move |l| said.lock().unwrap().push(l.to_string())),
                    },
                ),
            };
            let _ = done.send(fault);
        });
        Self {
            tx: Some(tx),
            lines,
            refused,
            ended,
        }
    }

    fn plain(limits: wasm::Limits, calls: Option<Calls>) -> Self {
        Self::start(no_network(), limits, calls, Vec::new())
    }

    fn send(&self, id: u64, method: &str, params: Value) {
        let _ = self.tx.as_ref().unwrap().send(line(id, method, params));
    }

    fn answer(&self) -> Value {
        let l = self
            .lines
            .recv_timeout(Duration::from_secs(20))
            .expect("an answer");
        serde_json::from_str(&l).expect("JSON")
    }

    fn call(&self, id: u64, method: &str, params: Value) -> Value {
        self.send(id, method, params);
        let a = self.answer();
        assert_eq!(a["id"], json!(id), "{a}");
        a["result"].clone()
    }

    /// Why its thread ended.
    fn fault(&self) -> Fault {
        self.ended
            .recv_timeout(Duration::from_secs(20))
            .expect("its thread ended")
            .expect("by a fault")
    }
}

fn no_network() -> Fetcher {
    Fetcher::new("dev.test.reactor", &[], &Net::default())
}

/// A fake service: `/hold-<ms>` answers after that long, whatever else is under way.
struct Fake {
    fetcher: Fetcher,
    /// The most requests held at once.
    most: Arc<AtomicUsize>,
}

fn fake() -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (now, most) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let top = most.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (now, most) = (now.clone(), most.clone());
            std::thread::spawn(move || {
                let mut writer = stream.try_clone().unwrap();
                let mut reader = BufReader::new(stream);
                let mut request = String::new();
                if reader.read_line(&mut request).unwrap_or(0) == 0 {
                    return;
                }
                let mut header = String::new();
                while reader.read_line(&mut header).unwrap_or(0) > 2 {
                    header.clear();
                }
                let held = now.fetch_add(1, Ordering::SeqCst) + 1;
                most.fetch_max(held, Ordering::SeqCst);
                if let Some(ms) = request
                    .split("/hold-")
                    .nth(1)
                    .and_then(|r| r.split(' ').next())
                {
                    std::thread::sleep(Duration::from_millis(ms.parse().unwrap()));
                }
                now.fetch_sub(1, Ordering::SeqCst);
                let _ = writer.write_all(
                    b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\nok",
                );
            });
        }
    });
    let net = Net {
        routes: HashMap::from([("hooks.example.dev".to_string(), base)]),
        dns: None,
    };
    Fake {
        fetcher: Fetcher::new("dev.test.reactor", &["hooks.example.dev".to_string()], &net),
        most: top,
    }
}

fn url(path: &str) -> Value {
    json!({ "url": format!("https://hooks.example.dev/{path}") })
}

#[test]
fn what_comes_together_makes_one_turn_and_its_state_stays_from_a_turn_to_the_next() {
    let first = (1..=3)
        .map(|i| line(i, "probe/turns", json!({})))
        .collect::<Vec<_>>();
    let s = Served::start(no_network(), wasm::Limits::default(), None, first);
    for i in 1..=3 {
        let a = s.answer();
        assert_eq!(a["id"], json!(i), "{a}");
        // One turn for the three lines, and no first line of a bundle before them.
        assert_eq!(
            a["result"],
            json!({ "turns": 1, "lines": 3, "boot": false }),
            "{a}"
        );
    }
    let next = s.call(4, "probe/turns", json!({}));
    assert_eq!(next, json!({ "turns": 2, "lines": 1, "boot": false }));
}

#[test]
fn a_slow_fetch_holds_back_neither_the_next_nor_the_thread() {
    let f = fake();
    let s = Served::start(f.fetcher, wasm::Limits::default(), None, Vec::new());
    assert!(s.call(1, "initialize", json!({})).is_object());
    let started = Instant::now();
    s.send(10, "probe/fetch", url("hold-1500"));
    s.send(11, "probe/fetch", url("quick"));
    s.send(12, "probe/turns", json!({}));
    let mut order = Vec::new();
    while order.len() < 3 {
        let a = s.answer();
        order.push((a["id"].as_u64().unwrap(), started.elapsed()));
        if a["id"] == json!(10) || a["id"] == json!(11) {
            assert_eq!(a["result"]["result"]["status"], 200, "{a}");
            assert_eq!(a["result"]["result"]["body"], "ok", "{a}");
        }
    }
    let ids: Vec<u64> = order.iter().map(|o| o.0).collect();
    assert_eq!(ids, vec![12, 11, 10], "{order:?}");
    assert!(order[2].1 >= Duration::from_millis(1500), "{order:?}");
}

#[test]
fn the_end_of_a_fetch_wakes_the_thread_and_a_stopped_plugin_does_not_wait_for_the_next() {
    let f = fake();
    let mut s = Served::start(f.fetcher, wasm::Limits::default(), None, Vec::new());
    // Nothing else comes: only the end of the fetch can run the turn that answers.
    s.send(1, "probe/fetch", url("hold-50"));
    assert_eq!(s.answer()["result"]["result"]["status"], 200);
    // Closed with a fetch of 30 s under way: the thread ends at once.
    s.send(2, "probe/fetch", url("hold-30000"));
    assert!(s.call(3, "probe/turns", json!({}))["turns"].is_number());
    s.tx.take();
    let t = Instant::now();
    let ended = s.ended.recv_timeout(Duration::from_secs(5));
    assert_eq!(ended, Ok(None), "closed by its host");
    assert!(t.elapsed() < Duration::from_secs(2), "{:?}", t.elapsed());
}

#[test]
fn a_plugin_has_sixteen_fetches_under_way_at_most_and_each_is_answered() {
    let f = fake();
    let most = f.most.clone();
    let s = Served::start(f.fetcher, wasm::Limits::default(), None, Vec::new());
    for id in 0..24 {
        s.send(id, "probe/fetch", url("hold-120"));
    }
    let mut seen: Vec<u64> = (0..24)
        .map(|_| {
            let a = s.answer();
            assert_eq!(a["result"]["result"]["status"], 200, "{a}");
            a["id"].as_u64().unwrap()
        })
        .collect();
    seen.sort_unstable();
    assert_eq!(seen, (0..24).collect::<Vec<u64>>());
    let most = most.load(Ordering::SeqCst);
    assert!((2..=16).contains(&most), "{most} at once");
}

#[test]
fn a_fetch_refused_goes_to_the_journal_from_beside_the_thread() {
    let s = Served::plain(wasm::Limits::default(), None);
    let a = s.call(1, "probe/fetch", url("x"));
    assert_eq!(a["error"]["code"], wasm::REFUSED, "{a}");
    let said = s.refused.lock().unwrap().clone();
    assert_eq!(
        said,
        vec!["net/fetch: hooks.example.dev is not in permissions.network of dev.test.reactor"]
    );
}

fn echo() -> Calls {
    Arc::new(|method, params| match params.get("fail") {
        Some(why) => Err(why.as_str().unwrap_or("").to_string()),
        None => Ok(json!({ "method": method, "params": params })),
    })
}

#[test]
fn escouade_call_answers_within_the_turn() {
    let s = Served::plain(wasm::Limits::default(), Some(echo()));
    let r = s.call(1, "probe/call", json!({ "a": [1, 2] }));
    let expected = json!({ "result": { "method": "probe/echo", "params": { "a": [1, 2] } } });
    assert_eq!(r["answer"], expected, "{r}");
    assert_eq!(r["took"], expected.to_string().len());
    // Its refusal is said to the module, which goes on.
    let r = s.call(2, "probe/call", json!({ "fail": "not in permissions" }));
    assert_eq!(r["answer"], json!({ "error": "not in permissions" }), "{r}");
    // An answer too big for its room says what it needs, and writes nothing.
    let needed =
        json!({ "result": { "method": "probe/echo", "params": "a text longer than its room" } });
    let r = s.call(3, "probe/small", json!({}));
    assert_eq!(r["took"], -(needed.to_string().len() as i64), "{r}");
}

#[test]
fn escouade_call_without_a_method_to_serve_says_so() {
    let s = Served::plain(wasm::Limits::default(), None);
    let r = s.call(1, "probe/call", json!({}));
    assert_eq!(r["answer"], json!({ "error": "no call probe/echo" }), "{r}");
}

#[test]
fn a_pointer_out_of_its_memory_is_a_fault_of_the_module() {
    let s = Served::plain(wasm::Limits::default(), Some(echo()));
    s.send(1, "probe/wild", json!({}));
    match s.fault() {
        Fault::Crashed(_) => {}
        f => panic!("{f:?}"),
    }
}

#[test]
fn the_time_of_escouade_call_counts_in_the_turn() {
    let slow: Calls = Arc::new(|_, _| {
        std::thread::sleep(Duration::from_millis(200));
        Ok(json!(null))
    });
    let limits = wasm::Limits {
        turn: Duration::from_millis(50),
        ..wasm::Limits::default()
    };
    let s = Served::plain(limits, Some(slow));
    s.send(1, "probe/call", json!({}));
    assert_eq!(s.fault(), Fault::Timeout(Duration::from_millis(50)));
}

#[test]
fn the_limits_of_a_bundle_hold_for_a_module() {
    let limits = wasm::Limits {
        turn: Duration::from_millis(100),
        ..wasm::Limits::default()
    };
    let s = Served::plain(limits, None);
    s.send(1, "probe/spin", json!({}));
    assert_eq!(s.fault(), Fault::Timeout(Duration::from_millis(100)));

    let limits = wasm::Limits {
        memory: 8 << 20,
        ..wasm::Limits::default()
    };
    let s = Served::plain(limits, None);
    s.send(1, "probe/grow", json!({}));
    assert!(matches!(s.fault(), Fault::Crashed(_)));
    // A decision has its own, shorter time: seen on the line, as for a bundle.
    let limits = wasm::Limits {
        decision: Duration::from_millis(60),
        ..wasm::Limits::default()
    };
    let s = Served::plain(limits, None);
    s.send(1, "permissions/decide", json!({}));
    s.send(2, "probe/spin", json!({}));
    assert!(matches!(s.fault(), Fault::Timeout(_)));
}

struct W {
    rt: Arc<Runtime>,
    plugins: Arc<Plugins>,
    store: Arc<MemoryStore>,
    data: DataDir,
    folder: PathBuf,
}

const ID: &str = "dev.test.reactor";

fn loaded(name: &str, limits: Limits) -> W {
    let folder = copied(name);
    let data = DataDir::new(folder.parent().unwrap().join("data"));
    let plugins = Plugins::load(data.clone(), "2.2.0");
    plugins.load_dev(&folder, Lang::En).unwrap();
    let store = Arc::new(MemoryStore::default());
    let rt = Runtime::new(plugins.clone(), store.clone(), &data, "2.2.0", limits);
    W {
        rt,
        plugins,
        store,
        data,
        folder,
    }
}

fn journal_has(rt: &Runtime, kind: Kind, text: &str) -> bool {
    rt.journal(ID)
        .iter()
        .any(|e| e.kind == kind && e.text.contains(text))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_module_runs_in_the_host_of_the_app_and_a_fault_makes_another_instance() {
    let mut limits = Limits::default();
    limits.wasm.turn = Duration::from_millis(200);
    let w = loaded("runtime", limits);
    let r = w.rt.call(ID, "probe/turns", json!({})).await.unwrap();
    assert_eq!(r["boot"], false, "{r}");
    assert!(
        journal_has(&w.rt, Kind::Start, "in WASM, isolated"),
        "{:?}",
        w.rt.journal(ID)
    );
    assert!(journal_has(&w.rt, Kind::Stderr, "reactor: first turn"));

    let turns = r["turns"].as_u64().unwrap();
    let r = w.rt.call(ID, "probe/spin", json!({})).await;
    assert!(
        matches!(r, Err(CallError::Crashed | CallError::Timeout)),
        "{r:?}"
    );
    let started = Instant::now();
    loop {
        // A new instance: its turns start again, and `initialize` was played again.
        if let Ok(v) = w.rt.call(ID, "probe/turns", json!({})).await {
            assert!(v["turns"].as_u64().unwrap() <= turns + 1, "{v}");
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "no new instance"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(journal_has(
        &w.rt,
        Kind::Crash,
        "a turn ran past 200 ms: Escouade stopped its instance"
    ));
    w.rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_module_is_precompiled_once_by_its_sha256_and_again_for_a_new_version_of_the_app() {
    let w = loaded("cache", Limits::default());
    let dir = w.rt.modules().dir().to_path_buf();
    let sum = wasm::sha256(&std::fs::read(w.folder.join(wasm::MODULE_FILE)).unwrap());
    let names = || -> Vec<String> {
        let mut n: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        n.sort();
        n
    };
    // At the installation (plugin_install calls `prepare`), then from the cache.
    assert_eq!(
        w.rt.prepare(ID).unwrap().join().unwrap(),
        Ok(Made::Precompiled)
    );
    assert_eq!(names(), vec![format!("{sum}-2.2.0.cwasm")]);
    assert_eq!(w.rt.prepare(ID).unwrap().join().unwrap(), Ok(Made::Cache));
    w.rt.call(ID, "probe/turns", json!({})).await.unwrap();
    assert!(journal_has(
        &w.rt,
        Kind::Start,
        "its precompiled module loaded from the cache"
    ));
    w.rt.shutdown();

    // A new version of the app: precompiled again, the old one taken out.
    let rt = Runtime::new(
        w.plugins.clone(),
        w.store.clone(),
        &w.data,
        "2.2.1",
        Limits::default(),
    );
    rt.prepare_all();
    assert_eq!(names(), vec![format!("{sum}-2.2.1.cwasm")]);
    // Another module is another key.
    let mut bytes = std::fs::read(w.folder.join(wasm::MODULE_FILE)).unwrap();
    bytes.extend_from_slice(&[0, 3, 1, b'x', 0]); // A custom section: the same code, other bytes.
    std::fs::write(w.folder.join(wasm::MODULE_FILE), &bytes).unwrap();
    assert_eq!(
        rt.prepare(ID).unwrap().join().unwrap(),
        Ok(Made::Precompiled)
    );
    assert_eq!(names().len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_without_its_module_or_with_no_reactor_is_refused_and_says_why() {
    let w = loaded("missing", Limits::default());
    std::fs::remove_file(w.folder.join(wasm::MODULE_FILE)).unwrap();
    let e = w.rt.call(ID, "probe/turns", json!({})).await.unwrap_err();
    assert!(matches!(e, CallError::Stopped(_)), "{e:?}");
    assert!(
        journal_has(&w.rt, Kind::Stop, "plugin.wasm"),
        "{:?}",
        w.rt.journal(ID)
    );
    w.rt.shutdown();

    // A program with a main, as a module: not a reactor.
    let why = wasm::reactor(&exporting("_start")).unwrap_err();
    assert!(why.contains("it exports _start"), "{why}");
    let why = wasm::reactor(&exporting("_initialize")).unwrap_err();
    assert!(why.contains("no export escouade-turn"), "{why}");
}

/// A module with one export, a function that does nothing.
fn exporting(name: &str) -> Vec<u8> {
    let mut m = b"\0asm\x01\0\0\0\x01\x04\x01\x60\0\0\x03\x02\x01\0\x07".to_vec();
    m.push(name.len() as u8 + 4);
    m.push(1);
    m.push(name.len() as u8);
    m.extend_from_slice(name.as_bytes());
    m.extend_from_slice(b"\0\0\x0a\x04\x01\x02\0\x0b");
    m
}

fn bench_options() -> Options {
    Options {
        lang: Lang::En,
        hand: Hand::Third,
        app_version: "2.2.0".into(),
        initialize: Duration::from_secs(5),
        grace: Duration::from_secs(1),
        ..Options::default()
    }
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_bench_plays_a_run_wasm_plugin_as_a_run_js_one() {
    let folder = copied("bench");
    let report = run(&folder, &bench_options()).await;
    assert!(report.passed(), "{}", report.render(Lang::En));

    // The module given (`--wasm`) is played in place of the plugin's.
    let elsewhere = folder.parent().unwrap().join("the-store.wasm");
    std::fs::rename(folder.join(wasm::MODULE_FILE), &elsewhere).unwrap();
    let report = run(
        &folder,
        &Options {
            module: Some(elsewhere),
            ..bench_options()
        },
    )
    .await;
    assert!(report.passed(), "{}", report.render(Lang::En));

    // Without it, the bench says what to do.
    let report = run(&folder, &bench_options()).await;
    assert!(!report.passed());
    let first = report.render(Lang::En);
    assert!(first.contains("wasm32-wasip1"), "{first}");
    assert!(report.outcomes.iter().any(|o| o.verdict == Verdict::Fail));
}

#[test]
fn validate_takes_a_run_wasm_plugin_and_judges_its_module() {
    let folder = copied("validate");
    let d = folder.to_string_lossy().into_owned();
    let validate = || cli::main(args(&["validate", &d, "--escouade", "2.2.0"]));
    assert_eq!(validate(), 0);
    // Its module is not needed for that: `build` makes it, and the store.
    std::fs::remove_file(folder.join(wasm::MODULE_FILE)).unwrap();
    assert_eq!(validate(), 0);
    // A module that the app could not run is said now.
    std::fs::write(folder.join(wasm::MODULE_FILE), b"no module").unwrap();
    assert_eq!(validate(), 1);
}

#[test]
fn a_module_reads_every_byte_it_is_given_from_a_line_cut_in_two() {
    // Two pieces, two turns: the line is whole on the second one.
    let text = json!({ "jsonrpc": "2.0", "id": 1, "method": "probe/turns", "params": {} });
    let text = format!("{text}\n").into_bytes();
    let (a, b) = text.split_at(10);
    let s = Served::start(
        no_network(),
        wasm::Limits::default(),
        None,
        vec![(a.to_vec(), None)],
    );
    let tx = s.tx.as_ref().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    tx.send((b.to_vec(), None)).unwrap();
    assert_eq!(s.answer()["result"]["turns"], 2);
}
