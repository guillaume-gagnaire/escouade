//! Tests of the `run.js` plugins in WASM (ESC-320): the committed `dist/plugin.mjs` of plugins/team-rules and
//! plugins/slack, unchanged, compiled by the pinned Javy and run by the runtime (`plugins::wasm`); the limits and the
//! restarts; the module precompiled at the installation, then loaded from the cache; `net/fetch`'s hosts, and the
//! addresses it refuses at the connection.
//!
//! They need escouade-js built (scripts/escouade-js.sh, CI's `escouade-js` job): without it, each says so and passes,
//! except in CI where it fails. The pinned Javy is downloaded once, into src-tauri/target/wasm-tests/tools.

use crate::i18n::Lang;
use crate::integrations::secrets::{MemoryStore, SecretStore};
use crate::paths::DataDir;
use crate::plugins::files::AgentPlace;
use crate::plugins::journal::Kind;
use crate::plugins::runtime::{secret_name, CallError, Limits, PhaseName, Runtime, Stop};
use crate::plugins::wasm::{self, Fetcher, Made, Net};
use crate::plugins::Plugins;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Whether the WASM plugins can run here: escouade-js built into this test binary, and the pinned Javy.
pub(crate) fn ready() -> bool {
    if let Err(e) = wasm::escouade_js() {
        assert!(std::env::var_os("CI").is_none(), "{e}");
        eprintln!("skipped: {e}");
        return false;
    }
    if std::env::var_os("ESCOUADE_JAVY").is_none() {
        let shared = DataDir::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("target/wasm-tests"));
        let javy = wasm::Modules::new(&shared, "tests").javy().unwrap();
        std::env::set_var("ESCOUADE_JAVY", javy);
    }
    true
}

struct W {
    rt: Arc<Runtime>,
    store: Arc<MemoryStore>,
    data: DataDir,
    plugins: Arc<Plugins>,
    id: String,
    folder: PathBuf,
}

/// A `run.js` plugin loaded in development: `manifest` given its `run`, `bundle` its `dist/plugin.mjs`, `files`
/// at its root.
fn plugin(
    name: &str,
    mut manifest: Value,
    bundle: &str,
    files: &[(&str, &str)],
    limits: Limits,
) -> W {
    let base = std::env::temp_dir().join(format!(
        "plugin-wasm-{name}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let folder = base.join("plugin");
    std::fs::create_dir_all(folder.join("dist")).unwrap();
    std::fs::write(folder.join("dist/plugin.mjs"), bundle).unwrap();
    for (f, text) in files {
        std::fs::write(folder.join(f), text).unwrap();
    }
    manifest["run"] = json!({ "js": "dist/plugin.mjs" });
    std::fs::write(
        folder.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let data = DataDir::new(base.join("data"));
    let plugins = Plugins::load(data.clone(), "2.2.0");
    plugins.load_dev(&folder, Lang::En).unwrap();
    let store = Arc::new(MemoryStore::default());
    let rt = Runtime::new(plugins.clone(), store.clone(), &data, "2.2.0", limits);
    // Its module made first (as at its installation): the calls' delays are not spent compiling it.
    let id = manifest["id"].as_str().unwrap();
    if let Some(h) = rt.prepare(id) {
        let _ = h.join();
    }
    W {
        id: manifest["id"].as_str().unwrap().to_string(),
        rt,
        store,
        data,
        plugins,
        folder,
    }
}

/// One of Escouade's own plugins (plugins/<dir>), its committed bundle and its files, its manifest on `run.js`.
fn own(dir: &str, limits: Limits) -> W {
    let src = root().join("plugins").join(dir);
    let mut manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(src.join("escouade-plugin.json")).unwrap())
            .unwrap();
    manifest.as_object_mut().unwrap().remove("$schema");
    let bundle = std::fs::read_to_string(src.join("dist/plugin.mjs")).unwrap();
    let files: Vec<(String, String)> = std::fs::read_dir(&src)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| {
            n.ends_with(".json")
                && n != "escouade-plugin.json"
                && n != "package.json"
                && n != "tsconfig.json"
        })
        .map(|n| (n.clone(), std::fs::read_to_string(src.join(&n)).unwrap()))
        .collect();
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    plugin(dir, manifest, &bundle, &files, limits)
}

async fn until(what: &str, f: impl Fn() -> bool) {
    let started = Instant::now();
    while !f() {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "{what}: not in time"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn journal_has(w: &W, kind: Kind, text: &str) -> bool {
    w.rt.journal(&w.id)
        .iter()
        .any(|e| e.kind == kind && e.text.contains(text))
}

fn ask(command: &str) -> Value {
    json!({
        "project": { "id": "p1", "name": "Escouade" },
        "request": { "id": "r1", "kind": "command", "tool": "Bash", "input": { "command": command } },
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn team_rules_decides_in_wasm_by_the_runtime_within_500_ms() {
    if !ready() {
        return;
    }
    let w = own("team-rules", Limits::default());
    assert_eq!(
        w.rt.prepare(&w.id).unwrap().join().unwrap(),
        Ok(Made::Cache)
    );
    w.rt.start(&w.id).unwrap();
    until("running", || w.rt.status(&w.id).phase == PhaseName::Running).await;
    assert!(journal_has(
        &w,
        Kind::Start,
        "in WASM, isolated: its precompiled module loaded from the cache"
    ));
    assert_eq!(w.rt.status(&w.id).pid, None, "no process");
    let cases = [
        ("rm -rf build", "deny"),
        ("npm test; git push --force origin main", "deny"),
        ("git status && git diff --stat", "allow"),
        ("git diff > ~/.bashrc", "ask"),
        ("npm testing", "ask"),
    ];
    for (command, expected) in cases {
        let t = Instant::now();
        let answer =
            w.rt.call(&w.id, "permissions/decide", ask(command))
                .await
                .unwrap();
        let took = t.elapsed();
        assert_eq!(answer["decision"], expected, "{command}: {answer}");
        assert!(took < Duration::from_millis(500), "{command} took {took:?}");
        eprintln!("{command}: {took:?}");
    }
    let denied =
        w.rt.call(&w.id, "permissions/decide", ask("git push -f"))
            .await
            .unwrap();
    assert_eq!(
        denied["message"],
        "No force push in this team: push a new commit instead."
    );
    w.rt.shutdown();
    assert!(journal_has(&w, Kind::Stop, "answered shutdown"));
}

/// A fake Slack: answers `{"ok":true}` to each request, and keeps them (first line, authorization, body).
type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

fn fake_slack() -> (String, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen: Seen = Arc::new(Mutex::new(vec![]));
    let kept = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut r = BufReader::new(stream.try_clone().unwrap());
            let mut first = String::new();
            r.read_line(&mut first).unwrap();
            let (mut len, mut auth) = (0, String::new());
            loop {
                let mut h = String::new();
                r.read_line(&mut h).unwrap();
                if h.trim().is_empty() {
                    break;
                }
                let (k, v) = h.split_once(':').unwrap();
                match k.to_ascii_lowercase().as_str() {
                    "content-length" => len = v.trim().parse().unwrap(),
                    "authorization" => auth = v.trim().into(),
                    _ => {}
                }
            }
            let mut body = vec![0; len];
            r.read_exact(&mut body).unwrap();
            kept.lock().unwrap().push((
                first.trim().into(),
                auth,
                String::from_utf8(body).unwrap(),
            ));
            let answer = r#"{"ok":true}"#;
            let mut s = stream;
            let _ = write!(
                s,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}",
                answer.len()
            );
        }
    });
    (base, seen)
}

fn to_review() -> Value {
    json!({
        "name": "ticket.column",
        "project": { "id": "p1", "name": "Escouade" },
        "ticket": { "key": "ESC-320", "title": "WASM <plugins>" },
        "data": { "from": "doing", "to": "review" },
    })
}

fn slack(network: &[&str]) -> (W, Seen) {
    let w = {
        let src = root().join("plugins/slack");
        let mut m: Value = serde_json::from_str(
            &std::fs::read_to_string(src.join("escouade-plugin.json")).unwrap(),
        )
        .unwrap();
        m.as_object_mut().unwrap().remove("$schema");
        m["permissions"]["network"] = json!(network);
        let bundle = std::fs::read_to_string(src.join("dist/plugin.mjs")).unwrap();
        let channels = std::fs::read_to_string(src.join("channels.json")).unwrap();
        plugin(
            "slack",
            m,
            &bundle,
            &[("channels.json", &channels)],
            Limits::default(),
        )
    };
    let (base, seen) = fake_slack();
    // slack.com is served by the fake Slack, in http: the permission is judged on its name first.
    w.rt.set_net(Net {
        routes: HashMap::from([("slack.com".to_string(), base)]),
        dns: None,
        ..Net::default()
    });
    w.store
        .set(&secret_name(&w.id, "token"), "xoxb-test-token")
        .unwrap();
    (w, seen)
}

#[tokio::test(flavor = "multi_thread")]
async fn slack_posts_in_wasm_by_net_fetch_to_a_fake_slack() {
    if !ready() {
        return;
    }
    let (w, seen) = slack(&["slack.com"]);
    w.rt.notify(&w.id, "events/fact", to_review()).unwrap();
    until("posted", || !seen.lock().unwrap().is_empty()).await;
    let (line, auth, body) = seen.lock().unwrap()[0].clone();
    assert_eq!(line, "POST /api/chat.postMessage HTTP/1.1");
    assert_eq!(auth, "Bearer xoxb-test-token");
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["channel"], "escouade-dev");
    assert_eq!(
        body["text"],
        ":test_tube: *ESC-320* to test: WASM &lt;plugins&gt;"
    );
    until("its log", || {
        journal_has(&w, Kind::Message, "posted in #escouade-dev")
    })
    .await;
    // Its secret is masked in its journal, as a process's.
    assert!(!w
        .rt
        .journal(&w.id)
        .iter()
        .any(|e| e.text.contains("xoxb-test-token")));
    w.rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn slack_is_refused_any_host_but_those_of_its_permissions() {
    if !ready() {
        return;
    }
    // Its manifest says hooks.slack.com only: slack.com is another host.
    let (w, seen) = slack(&["hooks.slack.com"]);
    w.rt.notify(&w.id, "events/fact", to_review()).unwrap();
    until("refused", || {
        journal_has(
            &w,
            Kind::Refused,
            "net/fetch: slack.com is not in permissions.network of dev.escouade.slack",
        )
    })
    .await;
    until("its error", || {
        journal_has(&w, Kind::Message, "Slack could not be reached")
    })
    .await;
    assert!(
        seen.lock().unwrap().is_empty(),
        "nothing reached the fake Slack"
    );
    w.rt.shutdown();
}

/// ESC-326: the two plugins of Escouade's own hand, as their folders hold them (`run.js`, no launch script), run on a
/// machine whose PATH holds no `node`: team-rules decides, slack posts to a fake Slack (never slack.com). Each test of
/// the suite runs in its own process (nextest), so the PATH changed here is this test's alone.
#[tokio::test(flavor = "multi_thread")]
async fn the_two_plugins_of_the_app_run_in_wasm_without_node_in_the_path() {
    if !ready() {
        return;
    }
    for dir in ["slack", "team-rules"] {
        let folder = root().join("plugins").join(dir);
        let m: Value = serde_json::from_str(
            &std::fs::read_to_string(folder.join("escouade-plugin.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(m["version"], "2.0.0", "{dir}");
        assert_eq!(
            m["run"],
            json!({ "js": "dist/plugin.mjs" }),
            "{dir}: run.js alone"
        );
        assert!(!folder.join(dir).exists() && !folder.join(format!("{dir}.cmd")).exists());
    }
    let empty = std::env::temp_dir().join(format!("no-node-{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&empty).unwrap();
    std::env::set_var("PATH", &empty);
    assert!(
        std::process::Command::new("node")
            .arg("-v")
            .output()
            .is_err(),
        "node must not be reachable"
    );

    let w = own("team-rules", Limits::default());
    w.rt.start(&w.id).unwrap();
    until("running", || w.rt.status(&w.id).phase == PhaseName::Running).await;
    assert_eq!(w.rt.status(&w.id).pid, None, "no process");
    let denied =
        w.rt.call(&w.id, "permissions/decide", ask("git push --force"))
            .await
            .unwrap();
    assert_eq!(denied["decision"], "deny");
    let allowed =
        w.rt.call(&w.id, "permissions/decide", ask("git status"))
            .await
            .unwrap();
    assert_eq!(allowed["decision"], "allow");
    w.rt.shutdown();

    let (w, seen) = slack(&["slack.com"]);
    w.rt.notify(&w.id, "events/fact", to_review()).unwrap();
    until("posted", || !seen.lock().unwrap().is_empty()).await;
    assert_eq!(
        seen.lock().unwrap()[0].0,
        "POST /api/chat.postMessage HTTP/1.1"
    );
    w.rt.shutdown();
}

/// A bundle of the tests, without the SDK, on the host's own transport (`__esc` of the prelude, ESC-321): it
/// answers `initialize`, `ping`, and what makes it misbehave.
const PROBE: &str = r#"
import { readFileSync } from "node:fs";
const out = (m) => __esc.write(JSON.stringify(m));
let calls = 0;
__esc.listen((line) => {
  const m = JSON.parse(line);
  calls++;
  if (m.method === "initialize") out({ jsonrpc: "2.0", id: m.id, result: { protocolVersion: 1, capabilities: {} } });
  if (m.method === "ping") out({ jsonrpc: "2.0", id: m.id, result: calls });
  if (m.method === "shutdown") out({ jsonrpc: "2.0", id: m.id, result: null });
  if (m.method === "spin") for (;;) {}
  if (m.method === "grow") { const a = []; for (;;) a.push(new Array(100000).fill(1)); }
  if (m.method === "boom") throw new Error("boom");
  if (m.method === "read") {
    const r = m.params.paths.map((p) => { try { return JSON.parse(readFileSync(new URL(p, import.meta.url), "utf8")).deny ? "rules" : "other"; } catch (e) { return e.code; } });
    out({ jsonrpc: "2.0", id: m.id, result: r });
  }
  if (m.method === "try") (async () => {
    const results = [];
    for (const url of m.params.urls) {
      try { const r = await fetch(url, { method: "POST", body: "{}" }); results.push([r.status, r.headers.get("location")]); }
      catch (e) { results.push(e.message); }
    }
    out({ jsonrpc: "2.0", id: m.id, result: results });
  })();
});
"#;

fn probe(name: &str, network: &[&str], limits: Limits) -> W {
    let manifest = json!({
        "id": "dev.test.probe",
        "name": "Sonde",
        "version": "1.0.0",
        "escouade": { "api": "1", "min": "2.1.0" },
        "permissions": { "network": network },
    });
    let rules = std::fs::read_to_string(root().join("plugins/team-rules/rules.json")).unwrap();
    plugin(
        name,
        manifest,
        PROBE,
        &[("rules.json", &rules), ("other.json", "{}")],
        limits,
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn a_loop_without_end_a_memory_that_grows_and_a_crash_are_caught_and_the_instance_made_again()
{
    if !ready() {
        return;
    }
    let mut limits = Limits::default();
    limits.wasm.turn = Duration::from_millis(300);
    limits.wasm.memory = 32 << 20;
    let w = probe("faults", &[], limits);
    assert_eq!(w.rt.call(&w.id, "ping", json!({})).await.unwrap(), 2);

    // A loop without end: stopped by the epoch, a new instance made (its state starts again).
    let t = Instant::now();
    let r = w.rt.call(&w.id, "spin", json!({})).await;
    assert!(
        matches!(r, Err(CallError::Crashed | CallError::Timeout)),
        "{r:?}"
    );
    assert!(t.elapsed() < Duration::from_secs(4), "{:?}", t.elapsed());
    until("its crash said", || {
        journal_has(
            &w,
            Kind::Crash,
            "a turn ran past 300 ms: Escouade stopped its instance",
        )
    })
    .await;
    until("restarted", || {
        journal_has(&w, Kind::Start, "starting again (1 of 3 within 5 minutes)")
    })
    .await;
    assert_eq!(
        w.rt.call(&w.id, "ping", json!({})).await.unwrap(),
        2,
        "a new instance"
    );

    // A memory that grows past its 32 MiB.
    let r = w.rt.call(&w.id, "grow", json!({})).await;
    assert!(
        matches!(r, Err(CallError::Crashed | CallError::Timeout)),
        "{r:?}"
    );
    until("its crash said", || {
        w.rt.journal(&w.id)
            .iter()
            .filter(|e| {
                e.kind == Kind::Crash
                    && e.text.contains("its instance crashed")
                    && e.text.contains("out of memory")
            })
            .count()
            == 1
    })
    .await;
    until("restarted", || {
        journal_has(&w, Kind::Start, "starting again (2 of 3")
    })
    .await;
    assert_eq!(w.rt.call(&w.id, "ping", json!({})).await.unwrap(), 2);

    // An exception it does not catch.
    assert!(w.rt.call(&w.id, "boom", json!({})).await.is_err());
    until("its crash said", || journal_has(&w, Kind::Crash, "boom")).await;
    until("restarted", || {
        journal_has(&w, Kind::Start, "starting again (3 of 3")
    })
    .await;
    assert_eq!(w.rt.call(&w.id, "ping", json!({})).await.unwrap(), 2);

    // Its budget spent: one more crash stops it, and its journal says so.
    assert!(w.rt.call(&w.id, "boom", json!({})).await.is_err());
    until("stopped", || w.rt.status(&w.id).phase == PhaseName::Stopped).await;
    assert_eq!(w.rt.status(&w.id).stop, Some(Stop::Crashed { times: 4 }));
    until("said", || {
        journal_has(&w, Kind::Stop, "stopped after 4 crashes within 5 minutes")
    })
    .await;
    // « Relancer » starts it again.
    w.rt.restart(&w.id).unwrap();
    assert_eq!(w.rt.call(&w.id, "ping", json!({})).await.unwrap(), 2);
    w.rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_module_is_precompiled_at_the_installation_loaded_from_the_cache_and_made_again_after_an_update(
) {
    if !ready() {
        return;
    }
    let w = probe("cache", &[], Limits::default());
    let dir = w.rt.modules().dir().to_path_buf();
    // At the installation (plugin_install calls `prepare`): Javy, then Cranelift, both kept.
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        w.rt.prepare(&w.id).unwrap().join().unwrap(),
        Ok(Made::Compiled)
    );
    let names = || -> Vec<String> {
        let mut n: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        n.sort();
        n
    };
    let kept = names();
    assert_eq!(kept.len(), 2, "{kept:?}");
    assert!(kept.iter().any(|n| n.ends_with(".wasm")));
    assert!(kept.iter().any(|n| n.ends_with("-2.2.0.cwasm")));
    // Its start: from the cache, in a few milliseconds.
    assert_eq!(
        w.rt.prepare(&w.id).unwrap().join().unwrap(),
        Ok(Made::Cache)
    );
    assert_eq!(w.rt.call(&w.id, "ping", json!({})).await.unwrap(), 2);
    assert!(journal_has(
        &w,
        Kind::Start,
        "its precompiled module loaded from the cache"
    ));
    w.rt.shutdown();

    // A new version of the app: the module is precompiled again, the old one taken out.
    let rt = Runtime::new(
        w.plugins.clone(),
        w.store.clone(),
        &w.data,
        "2.2.1",
        Limits::default(),
    );
    rt.prepare_all();
    let after = names();
    assert!(
        after.iter().any(|n| n.ends_with("-2.2.1.cwasm")),
        "{after:?}"
    );
    assert!(
        !after.iter().any(|n| n.ends_with("-2.2.0.cwasm")),
        "{after:?}"
    );
    assert_eq!(after.len(), 2);
    assert!(rt.journal(&w.id).iter().any(|e| e
        .text
        .contains("ready: its module precompiled for this version of Escouade")));
    // A bundle that changes is another module.
    std::fs::write(
        w.folder.join("dist/plugin.mjs"),
        format!("{PROBE}\n// changed\n"),
    )
    .unwrap();
    assert_eq!(
        rt.prepare(&w.id).unwrap().join().unwrap(),
        Ok(Made::Compiled)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_reads_only_the_json_files_of_its_folder() {
    if !ready() {
        return;
    }
    let w = probe("files", &[], Limits::default());
    let paths = [
        "../rules.json",
        "/plugin/rules.json",
        "../escouade-plugin.json",
        "../../slack/channels.json",
        "/etc/passwd",
        "../other.json",
    ];
    let r =
        w.rt.call(&w.id, "read", json!({ "paths": paths }))
            .await
            .unwrap();
    assert_eq!(
        r,
        json!(["rules", "rules", "ENOENT", "ENOENT", "ENOENT", "other"])
    );
    w.rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bundle_that_imports_what_a_plugin_in_wasm_has_not_is_not_started_and_says_why() {
    if !ready() {
        return;
    }
    let manifest = json!({ "id": "dev.test.child", "name": "Enfant", "version": "1.0.0", "escouade": { "api": "1", "min": "2.1.0" } });
    let w = plugin(
        "child",
        manifest,
        "import { spawn } from \"node:child_process\";\nspawn('calc');\n",
        &[],
        Limits::default(),
    );
    assert!(w.rt.call(&w.id, "ping", json!({})).await.is_err());
    until("refused", || w.rt.status(&w.id).phase == PhaseName::Stopped).await;
    let Some(Stop::Refused { reason }) = w.rt.status(&w.id).stop else {
        panic!()
    };
    assert!(
        reason.contains(
            "the bundle imports node:child_process, which a plugin in WASM does not have"
        ),
        "{reason}"
    );
}

/// A server that keeps whether anything reached it.
fn listener() -> (u16, TcpListener) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    (l.local_addr().unwrap().port(), l)
}

#[tokio::test(flavor = "multi_thread")]
async fn net_fetch_refuses_at_the_connection_a_declared_host_that_resolves_to_127_0_0_1() {
    if !ready() {
        return;
    }
    let (port, l) = listener();
    // slack.com is declared, and its DNS answers 127.0.0.1: the connection is refused, nothing reaches the port.
    let w = probe("loopback", &["slack.com", "localhost"], Limits::default());
    w.rt.set_net(Net {
        routes: HashMap::new(),
        dns: Some(Arc::new(|host: &str| match host {
            "slack.com" => vec!["127.0.0.1".parse().unwrap()],
            other => std::net::ToSocketAddrs::to_socket_addrs(&(other, 0))
                .map(|a| a.map(|s| s.ip()).collect())
                .unwrap_or_default(),
        })),
        ..Net::default()
    });
    let urls = [
        format!("https://slack.com:{port}/api/x"),
        format!("https://localhost:{port}/x"),
    ];
    let r =
        w.rt.call(&w.id, "try", json!({ "urls": urls }))
            .await
            .unwrap();
    assert_eq!(
        r[0],
        "slack.com resolves to 127.0.0.1, an address of this machine or of the local network"
    );
    // localhost, by the system's resolver: 127.0.0.1 (and ::1).
    let local = r[1].as_str().unwrap();
    assert!(
        local.starts_with("localhost resolves to ") && local.contains("127.0.0.1"),
        "{local}"
    );
    assert!(l.accept().is_err(), "nothing reached the port");
    until("its journal", || {
        journal_has(
            &w,
            Kind::Refused,
            "net/fetch: slack.com resolves to 127.0.0.1",
        )
    })
    .await;
    w.rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn net_fetch_refuses_every_address_of_the_machine_and_of_the_local_network() {
    let refused = [
        "127.0.0.1",
        "127.8.9.1",
        "10.0.0.1",
        "172.16.5.4",
        "192.168.1.1",
        "169.254.169.254",
        "0.0.0.0",
        "0.1.2.3",
        "224.0.0.1",
        "239.255.255.250",
        "255.255.255.255",
        "100.64.0.1",
        "::1",
        "::",
        "fe80::1",
        "fc00::1",
        "fd12:3456::1",
        "ff02::1",
        "fec0::1",
        "::ffff:127.0.0.1",
        "::ffff:10.1.2.3",
        "::ffff:169.254.169.254",
        "::127.0.0.1",
        "64:ff9b::7f00:1",
        "64:ff9b::a00:1",
    ];
    for a in refused {
        assert!(wasm::forbidden(a.parse().unwrap()), "{a}");
    }
    for a in [
        "1.1.1.1",
        "93.184.215.14",
        "100.128.0.1",
        "2606:4700::1111",
        "::ffff:1.1.1.1",
        "64:ff9b::101:101",
    ] {
        assert!(!wasm::forbidden(a.parse().unwrap()), "{a}");
    }
    // Each kind, through the connection's own resolver: mapped and translated IPv4 too.
    let (port, l) = listener();
    let answers: HashMap<&str, Vec<IpAddr>> = HashMap::from([
        ("a.exemple.dev", vec!["::ffff:127.0.0.1".parse().unwrap()]),
        (
            "b.exemple.dev",
            vec!["fe80::1".parse().unwrap(), "10.1.2.3".parse().unwrap()],
        ),
        (
            "c.exemple.dev",
            vec!["0.0.0.0".parse().unwrap(), "::".parse().unwrap()],
        ),
        (
            "d.exemple.dev",
            vec!["224.0.0.1".parse().unwrap(), "ff02::1".parse().unwrap()],
        ),
        (
            "e.exemple.dev",
            vec!["::1".parse().unwrap(), "fd00::1".parse().unwrap()],
        ),
    ]);
    let answers = Arc::new(answers);
    let net = Net {
        routes: HashMap::new(),
        dns: Some(Arc::new(move |h: &str| {
            answers.get(h).cloned().unwrap_or_default()
        })),
        ..Net::default()
    };
    let f = Fetcher::new("dev.test.probe", &["exemple.dev".to_string()], &net);
    for host in ["a", "b", "c", "d", "e"] {
        let url = format!("https://{host}.exemple.dev:{port}/x");
        let (code, message) = f.fetch(&json!({ "url": url })).await.unwrap_err();
        assert_eq!(code, wasm::REFUSED, "{host}: {message}");
        assert!(
            message.contains("an address of this machine or of the local network"),
            "{message}"
        );
    }
    assert!(l.accept().is_err());
    // And by name, before any resolution.
    let names = [
        (
            "https://127.0.0.1/x",
            "127.0.0.1 is not in permissions.network of dev.test.probe",
        ),
        (
            "https://[::1]/x",
            "[::1] is not in permissions.network of dev.test.probe",
        ),
        (
            "https://exemple.dev.evil.example/x",
            "exemple.dev.evil.example is not in permissions.network of dev.test.probe",
        ),
        (
            "https://evilexemple.dev/x",
            "evilexemple.dev is not in permissions.network of dev.test.probe",
        ),
        (
            "https://exemple.dev@evil.example/x",
            "evil.example is not in permissions.network of dev.test.probe",
        ),
        ("http://exemple.dev/x", "only https: http://exemple.dev"),
    ];
    for (url, said) in names {
        assert_eq!(
            f.fetch(&json!({ "url": url })).await.unwrap_err(),
            (wasm::REFUSED, said.to_string()),
            "{url}"
        );
    }
    assert!(f.covers("api.exemple.dev"));
    assert!(f.covers("exemple.dev"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_redirection_is_never_followed_out_of_the_permitted_hosts() {
    if !ready() {
        return;
    }
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.local_addr().unwrap());
    let (port, elsewhere) = listener();
    std::thread::spawn(move || {
        for stream in server.incoming().flatten() {
            let mut r = BufReader::new(stream.try_clone().unwrap());
            let mut l = String::new();
            while r.read_line(&mut l).unwrap_or(0) > 2 {
                l.clear();
            }
            let mut s = stream;
            let _ = write!(s, "HTTP/1.1 302 Found\r\nlocation: http://127.0.0.1:{port}/steal\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        }
    });
    let w = probe("redirect", &["slack.com"], Limits::default());
    w.rt.set_net(Net {
        routes: HashMap::from([("slack.com".to_string(), base)]),
        dns: None,
        ..Net::default()
    });
    let r =
        w.rt.call(&w.id, "try", json!({ "urls": ["https://slack.com/api/x"] }))
            .await
            .unwrap();
    assert_eq!(r[0], json!([302, format!("http://127.0.0.1:{port}/steal")]));
    assert!(
        elsewhere.accept().is_err(),
        "nothing reached the other host"
    );
    w.rt.shutdown();
}

// ---- The SDK on both transports (ESC-321) ----------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum Transport {
    Node,
    Wasm,
}

/// tests/fixtures/plugins/sdk-both, a plugin written for the SDK (its committed bundle), copied to a folder of the
/// test's own: run by Node (`run.command`) or in WASM (`run.js`), its rules.json pointed at a fake hook.
fn sdk_both(t: Transport) -> (W, Seen) {
    // Else cmd.exe may no longer find node (see `trim_test_path`).
    #[cfg(windows)]
    crate::paths::trim_test_path();
    let src = root().join("tests/fixtures/plugins/sdk-both");
    let base = std::env::temp_dir().join(format!(
        "plugin-sdk-{t:?}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let folder = base.join("plugin");
    for f in ["dist/plugin.mjs", "data/words.txt", "run", "run.cmd"] {
        let to = folder.join(f);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(src.join(f), to).unwrap();
    }
    let (hook, seen) = fake_slack();
    let endpoint = match t {
        Transport::Node => hook.clone(),
        // In WASM, by net/fetch: the host of permissions.network, routed to the fake hook.
        Transport::Wasm => "https://hooks.example.dev".to_string(),
    };
    std::fs::write(
        folder.join("rules.json"),
        json!({ "endpoint": endpoint, "deny": "rm -rf" }).to_string(),
    )
    .unwrap();
    let mut m: Value =
        serde_json::from_str(&std::fs::read_to_string(src.join("escouade-plugin.json")).unwrap())
            .unwrap();
    if t == Transport::Node {
        m["run"] = json!({ "command": "run", "windows": "run.cmd" });
    }
    std::fs::write(
        folder.join("escouade-plugin.json"),
        serde_json::to_string_pretty(&m).unwrap(),
    )
    .unwrap();
    let data = DataDir::new(base.join("data"));
    let plugins = Plugins::load(data.clone(), "2.2.0");
    plugins.load_dev(&folder, Lang::En).unwrap();
    let store = Arc::new(MemoryStore::default());
    let rt = Runtime::new(
        plugins.clone(),
        store.clone(),
        &data,
        "2.2.0",
        Limits::default(),
    );
    let id = m["id"].as_str().unwrap().to_string();
    if t == Transport::Wasm {
        rt.set_net(Net {
            routes: HashMap::from([("hooks.example.dev".to_string(), hook)]),
            dns: None,
            ..Net::default()
        });
        if let Some(h) = rt.prepare(&id) {
            let _ = h.join();
        }
    }
    store.set(&secret_name(&id, "token"), "s3cret").unwrap();
    (
        W {
            rt,
            store,
            data,
            plugins,
            id,
            folder,
        },
        seen,
    )
}

/// The same scenario on either transport: a decision by its rules.json (readFileSync), a fact that reads its files
/// (p.file, one refused, one missing), its secret, and posts by the standard fetch; its journal; shutdown.
async fn the_sdk_plays(t: Transport) {
    let (w, seen) = sdk_both(t);
    w.rt.start(&w.id).unwrap();
    until("running", || w.rt.status(&w.id).phase == PhaseName::Running).await;
    assert_eq!(w.rt.status(&w.id).pid.is_some(), t == Transport::Node);
    let denied =
        w.rt.call(&w.id, "permissions/decide", ask("rm -rf build"))
            .await
            .unwrap();
    assert_eq!(
        denied,
        json!({ "decision": "deny", "message": "no rm -rf here" })
    );
    let asked =
        w.rt.call(&w.id, "permissions/decide", ask("npm test"))
            .await
            .unwrap();
    assert_eq!(asked, json!({ "decision": "ask" }));

    w.rt.notify(&w.id, "events/fact", to_review()).unwrap();
    until("posted", || !seen.lock().unwrap().is_empty()).await;
    let (line, auth, body) = seen.lock().unwrap()[0].clone();
    assert_eq!(line, "POST /hook HTTP/1.1");
    assert_eq!(auth, "Bearer s3cret");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({ "ticket": "ESC-320", "words": ["un", "deux"], "refused": -32003, "missing": -32005 })
    );
    until("its log", || {
        journal_has(&w, Kind::Message, "[info] posted 200 ok=true")
    })
    .await;
    assert!(journal_has(
        &w,
        Kind::Refused,
        "files/read: \"../escouade-plugin.json\" is not a file of the plugin's folder"
    ));
    assert!(!w
        .rt
        .journal(&w.id)
        .iter()
        .any(|e| e.text.contains("s3cret")));
    w.rt.shutdown();
    assert!(journal_has(&w, Kind::Stop, "answered shutdown"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_written_for_the_sdk_runs_unchanged_by_node() {
    the_sdk_plays(Transport::Node).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plugin_written_for_the_sdk_runs_unchanged_in_wasm() {
    if !ready() {
        return;
    }
    the_sdk_plays(Transport::Wasm).await;
}

/// An old bundle, built with the SDK before 0.3 (its own stdin), is not run in WASM: the reason is in its journal.
#[tokio::test(flavor = "multi_thread")]
async fn a_bundle_that_reads_stdin_itself_says_to_build_it_again() {
    if !ready() {
        return;
    }
    let w = plugin(
        "old-sdk",
        json!({ "id": "dev.test.old", "name": "Vieux", "version": "1.0.0", "escouade": { "api": "1", "min": "2.1.0" } }),
        "import { createInterface } from \"node:readline\";\ncreateInterface({ input: process.stdin }).on(\"line\", () => {});\n",
        &[],
        Limits::default(),
    );
    let _ = w.rt.start(&w.id);
    until("it stopped", || {
        w.rt.status(&w.id).phase == PhaseName::Stopped
    })
    .await;
    let journal: Vec<String> =
        w.rt.journal(&w.id)
            .iter()
            .map(|e| format!("{:?} {}", e.kind, e.text))
            .collect();
    assert!(
        journal
            .iter()
            .any(|l| l.contains("build the plugin with @escouade-dev/plugin 0.3 or later")),
        "{journal:#?}"
    );
    w.rt.shutdown();
}

/// ESC-331: one host for `run.js` and `run.wasm`: a bundle's fetch runs beside its thread too.
#[tokio::test(flavor = "multi_thread")]
async fn the_timers_a_binary_fetch_a_stream_and_a_socket_of_the_sdk_run_in_wasm() {
    use futures_util::{SinkExt, StreamExt};
    if !ready() {
        return;
    }
    let src = root().join("tests/fixtures/plugins/sdk-net");
    let mut manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(src.join("escouade-plugin.json")).unwrap())
            .unwrap();
    manifest.as_object_mut().unwrap().remove("run");
    let bundle = std::fs::read_to_string(src.join("dist/plugin.mjs")).unwrap();
    let w = plugin("sdk-net", manifest, &bundle, &[], Limits::default());

    // A server of bytes, of events; a socket.
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in l.incoming().flatten() {
            std::thread::spawn(move || {
                let mut r = BufReader::new(stream.try_clone().unwrap());
                let (mut first, mut len) = (String::new(), 0);
                r.read_line(&mut first).unwrap();
                loop {
                    let mut h = String::new();
                    r.read_line(&mut h).unwrap();
                    if h.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; len];
                r.read_exact(&mut body).unwrap();
                let mut s = stream;
                if first.contains("/events") {
                    let _ = write!(s, "HTTP/1.1 200 OK\r\n\r\ndata: é\n\n");
                    let _ = s.flush();
                    std::thread::sleep(Duration::from_millis(50));
                    let _ = write!(s, "data: à\n\n");
                } else {
                    let _ = write!(
                        s,
                        "HTTP/1.1 200 OK\r\nconnection: close\r\ncontent-length: {}\r\n\r\n",
                        body.len()
                    );
                    let _ = s.write_all(&body);
                }
            });
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let socket = format!("ws://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        while let Ok((tcp, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
                while let Some(Ok(m)) = ws.next().await {
                    if m.is_text() {
                        let _ = ws.send(m).await;
                    }
                }
            });
        }
    });
    w.rt.set_net(Net {
        routes: HashMap::from([
            ("api.exemple.dev".to_string(), base),
            ("live.exemple.dev".to_string(), socket),
        ]),
        ..Net::default()
    });

    let started = Instant::now();
    let fired =
        w.rt.call(&w.id, "later", json!({ "ms": 200 }))
            .await
            .unwrap();
    assert_eq!(fired, "fired");
    assert!(started.elapsed() >= Duration::from_millis(190));
    let bytes = json!({ "url": "https://api.exemple.dev/echo", "bytes": [255, 0, 1, 2] });
    assert_eq!(
        w.rt.call(&w.id, "bytes", bytes).await.unwrap(),
        json!([255, 0, 1, 2])
    );
    let url = json!({ "url": "https://api.exemple.dev/events" });
    assert_eq!(
        w.rt.call(&w.id, "stream", url).await.unwrap(),
        json!({ "status": 200, "text": "data: é\n\ndata: à\n\n" })
    );
    let hello = json!({ "url": "wss://live.exemple.dev/", "text": "bonjour" });
    let said = w.rt.call(&w.id, "socket", hello).await.unwrap();
    assert_eq!(said["got"], "bonjour", "{said}");
    assert!(said["closed"]["code"].is_u64(), "{said}");
    w.rt.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_slow_fetch_of_a_bundle_holds_back_neither_the_next_nor_the_thread() {
    if !ready() {
        return;
    }
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in l.incoming().flatten() {
            std::thread::spawn(move || {
                let mut r = BufReader::new(stream.try_clone().unwrap());
                let (mut first, mut len) = (String::new(), 0);
                r.read_line(&mut first).unwrap();
                loop {
                    let mut h = String::new();
                    r.read_line(&mut h).unwrap();
                    if h.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                }
                r.read_exact(&mut vec![0; len]).unwrap();
                if first.contains("/slow") {
                    std::thread::sleep(Duration::from_millis(1500));
                }
                let mut s = stream;
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                );
            });
        }
    });
    let w = probe("slow-fetch", &["exemple.dev"], Limits::default());
    w.rt.set_net(Net {
        routes: HashMap::from([("api.exemple.dev".to_string(), base)]),
        dns: None,
        ..Net::default()
    });
    let started = Instant::now();
    let (rt, id) = (w.rt.clone(), w.id.clone());
    let slow = tokio::spawn(async move {
        let urls = json!({ "urls": ["https://api.exemple.dev/slow"] });
        let r = rt.call(&id, "try", urls).await.unwrap();
        (r, started.elapsed())
    });
    // Wait for the first to be under way, then the second and a plain call go past it.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let urls = json!({ "urls": ["https://api.exemple.dev/quick"] });
    let quick = w.rt.call(&w.id, "try", urls).await.unwrap();
    assert!(w.rt.call(&w.id, "ping", json!({})).await.is_ok());
    let passed = started.elapsed();
    assert_eq!(quick[0][0], 200, "{quick}");
    let (slow, took) = slow.await.unwrap();
    assert_eq!(slow[0][0], 200, "{slow}");
    assert!(passed < took, "{passed:?} then {took:?}");
    assert!(took >= Duration::from_millis(1500), "{took:?}");
    w.rt.shutdown();
}

/// A plugin written for the SDK, in WASM (ESC-333): it reads the folder of an agent, writes there, is refused what is
/// not its own, runs a declared program, and is refused another.
#[tokio::test(flavor = "multi_thread")]
async fn a_run_js_reads_the_folder_of_its_agent_and_runs_a_declared_command() {
    if !ready() {
        return;
    }
    let (w, _seen) = sdk_both(Transport::Wasm);
    let base = w.folder.parent().unwrap().canonicalize().unwrap();
    for d in ["wt", "other"] {
        std::fs::create_dir_all(base.join(d)).unwrap();
    }
    std::fs::write(base.join("wt/notes.txt"), "dans son dossier").unwrap();
    std::fs::write(base.join("other/secret.txt"), "à un autre").unwrap();
    let roots =
        w.rt.place_agent(
            &w.id,
            "a",
            AgentPlace {
                cwd: base.join("wt"),
                ..AgentPlace::default()
            },
        )
        .unwrap();
    assert_eq!(roots.keys().collect::<Vec<_>>(), vec!["agent"]);
    let r =
        w.rt.call_within(
            &w.id,
            "probe/host",
            json!({ "agent": "a", "outside": base.join("other/secret.txt") }),
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    assert_eq!(r["text"], "dans son dossier", "{r}");
    assert_eq!(r["names"], json!(["notes.txt", "written.txt"]), "{r}");
    assert_eq!(r["refused"], -32003, "{r}");
    assert!(r["out"].as_str().unwrap().starts_with("git version"), "{r}");
    assert_eq!(r["exit"], 0, "{r}");
    assert_eq!(r["denied"], -32003, "{r}");
    assert_eq!(
        std::fs::read_to_string(base.join("wt/written.txt")).unwrap(),
        "dans son dossier!"
    );
    assert!(journal_has(&w, Kind::Refused, "fs/read: refused"));
    assert!(journal_has(
        &w,
        Kind::Message,
        "fs/write agent: wrote 17 bytes"
    ));
    assert!(journal_has(
        &w,
        Kind::Message,
        "proc/spawn: ran git --version"
    ));
    w.rt.shutdown();
}
