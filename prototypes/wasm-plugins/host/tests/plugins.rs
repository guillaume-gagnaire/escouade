//! ESC-319: the committed bundles of dev.escouade.team-rules and dev.escouade.slack, unchanged, compiled by the pinned
//! Javy and run by the prototype's runtime. Needs the Javy of the README (ESCOUADE_JAVY).

use escouade_wasm_host::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn compiled(plugin: &str, tag: &str) -> Vec<u8> {
    let bundle =
        std::fs::read_to_string(repo().join("plugins").join(plugin).join("dist/plugin.mjs"))
            .unwrap();
    let work =
        std::env::temp_dir().join(format!("esc319-test-{}-{plugin}-{tag}", std::process::id()));
    let wasm = Compiler::pinned().compile(&bundle, &work).unwrap();
    let _ = std::fs::remove_dir_all(&work);
    wasm
}

fn hello(id: &str) -> Value {
    json!({
        "protocolVersion": 1,
        "escouade": { "version": "2.2.0" },
        "plugin": { "id": id, "version": "2.0.0" },
        "capabilities": { "batch": true, "secrets": true, "settings": true, "log": true },
    })
}

fn ask(command: &str) -> Value {
    json!({
        "project": { "id": "p1", "name": "Escouade" },
        "request": { "id": "r1", "kind": "command", "tool": "Bash", "input": { "command": command } },
    })
}

#[test]
fn team_rules_decides_in_wasm_well_within_500_ms() {
    let module = load(&compiled("team-rules", "decide")).unwrap();
    let grants = Grants {
        id: "dev.escouade.team-rules".into(),
        folder: repo().join("plugins/team-rules"),
        ..Default::default()
    };
    let start = Instant::now();
    let mut p = Plugin::start(&module, grants, Limits::default()).unwrap();
    let init = p
        .request("initialize", hello("dev.escouade.team-rules"))
        .unwrap();
    assert_eq!(init["protocolVersion"], 1);
    let ready = start.elapsed();

    let cases = [
        ("rm -rf build", "deny"),
        ("npm test; git push --force origin main", "deny"),
        ("git status && git diff --stat", "allow"),
        ("git diff > ~/.bashrc", "ask"),
        ("git log --output=/tmp/x", "ask"),
        ("npm testing", "ask"),
    ];
    for (command, expected) in cases {
        let t = Instant::now();
        let answer = p.request("permissions/decide", ask(command)).unwrap();
        let took = t.elapsed();
        assert_eq!(answer["decision"], expected, "{command}: {answer}");
        assert!(took < Duration::from_millis(500), "{command} took {took:?}");
    }
    let denied = p.request("permissions/decide", ask("git push -f")).unwrap();
    assert_eq!(
        denied["message"],
        "No force push in this team: push a new commit instead."
    );
    // An edit is not a command: the user is asked.
    let edit = json!({ "request": { "id": "r2", "kind": "edit", "tool": "Edit", "input": {} } });
    assert_eq!(
        p.request("permissions/decide", edit).unwrap()["decision"],
        "ask"
    );
    eprintln!(
        "team-rules: ready in {ready:?}, memory {} KiB",
        p.memory() >> 10
    );
    assert!(p.request("shutdown", Value::Null).is_ok());
}

/// A fake Slack: answers `{"ok":true}` to each request, and keeps them.
fn fake_slack() -> (String, Arc<Mutex<Vec<(String, String, String)>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(vec![]));
    let kept = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut r = BufReader::new(stream.try_clone().unwrap());
            let mut first = String::new();
            r.read_line(&mut first).unwrap();
            let mut len = 0;
            let mut auth = String::new();
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
            write!(
                s,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}",
                answer.len()
            )
            .unwrap();
        }
    });
    (base, seen)
}

fn slack_grants(network: &[&str], base: &str) -> Grants {
    Grants {
        id: "dev.escouade.slack".into(),
        network: network.iter().map(|s| s.to_string()).collect(),
        secrets: HashMap::from([("token".into(), "xoxb-test-token".into())]),
        folder: repo().join("plugins/slack"),
        resolve: HashMap::from([("slack.com".into(), base.to_string())]),
        ..Default::default()
    }
}

fn to_review() -> Value {
    json!({
        "name": "ticket.column",
        "project": { "id": "p1", "name": "Escouade" },
        "ticket": { "key": "ESC-319", "title": "WASM <plugins>" },
        "data": { "from": "doing", "to": "review" },
    })
}

#[test]
fn slack_posts_by_a_fetch_the_host_serves() {
    let (base, seen) = fake_slack();
    let module = load(&compiled("slack", "post")).unwrap();
    let mut p = Plugin::start(
        &module,
        slack_grants(&["slack.com"], &base),
        Limits::default(),
    )
    .unwrap();
    p.request("initialize", hello("dev.escouade.slack"))
        .unwrap();
    let t = Instant::now();
    p.notify("events/fact", to_review()).unwrap();
    let took = t.elapsed();

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "one message posted: {:?}", p.journal);
    let (line, auth, body) = &seen[0];
    assert_eq!(line, "POST /api/chat.postMessage HTTP/1.1");
    assert_eq!(auth, "Bearer xoxb-test-token");
    let body: Value = serde_json::from_str(body).unwrap();
    assert_eq!(body["channel"], "escouade-dev");
    assert_eq!(
        body["text"],
        ":test_tube: *ESC-319* to test: WASM &lt;plugins&gt;"
    );
    assert_eq!(
        p.calls.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(),
        ["secrets/get", "net/fetch"]
    );
    assert!(
        p.journal
            .iter()
            .any(|l| l.level == "info" && l.message == "posted in #escouade-dev"),
        "{:?}",
        p.journal
    );
    eprintln!("slack: a fact posted in {took:?}");
}

#[test]
fn slack_is_refused_any_host_outside_its_permissions() {
    let (base, seen) = fake_slack();
    let module = load(&compiled("slack", "refused")).unwrap();
    // Its manifest would say hooks.slack.com only: slack.com is another host.
    let mut p = Plugin::start(
        &module,
        slack_grants(&["hooks.slack.com"], &base),
        Limits::default(),
    )
    .unwrap();
    p.request("initialize", hello("dev.escouade.slack"))
        .unwrap();
    p.notify("events/fact", to_review()).unwrap();
    assert!(
        seen.lock().unwrap().is_empty(),
        "nothing reached the fake Slack"
    );
    let refused = p
        .calls
        .iter()
        .find(|c| c.0 == "net/fetch")
        .and_then(|c| c.1.clone())
        .unwrap();
    assert_eq!(
        refused,
        "slack.com is not in permissions.network of dev.escouade.slack"
    );
    assert!(
        p.journal
            .iter()
            .any(|l| l.level == "error" && l.message.starts_with("Slack could not be reached")),
        "{:?}",
        p.journal
    );
}

#[test]
fn a_fetch_elsewhere_than_slack_com_is_refused_even_by_a_lookalike() {
    // The same check on names a plugin could try: a subdomain of the host is allowed, a lookalike is not.
    let bundle = r#"
import { createInterface } from "node:readline";
const out = (m) => process.stdout.write(JSON.stringify(m) + "\n");
createInterface({ input: process.stdin }).on("line", async (line) => {
  const m = JSON.parse(line);
  if (m.method !== "try") return;
  const results = [];
  for (const url of m.params.urls) {
    try { results.push((await fetch(url, { method: "POST", body: "{}" })).status); }
    catch (e) { results.push(e.message); }
  }
  out({ jsonrpc: "2.0", id: m.id, result: results });
});
"#;
    let work = std::env::temp_dir().join(format!("esc319-test-{}-lookalike", std::process::id()));
    let wasm = Compiler::pinned().compile(bundle, &work).unwrap();
    let (base, seen) = fake_slack();
    let mut grants = slack_grants(&["slack.com"], &base);
    grants.resolve.insert("api.slack.com".into(), base.clone());
    let mut p = Plugin::start(&load(&wasm).unwrap(), grants, Limits::default()).unwrap();
    let urls = [
        "https://slack.com/api/x",
        "https://api.slack.com/api/x",
        "https://slack.com.evil.example/api/x",
        "https://evilslack.com/api/x",
        "https://xoxb@evil.example/api/x",
        "http://slack.com/api/x",
        "https://127.0.0.1/api/x",
    ];
    let r = p.request("try", json!({ "urls": urls })).unwrap();
    assert_eq!(r[0], 200);
    assert_eq!(r[1], 200);
    assert_eq!(
        r[2],
        "slack.com.evil.example is not in permissions.network of dev.escouade.slack"
    );
    assert_eq!(
        r[3],
        "evilslack.com is not in permissions.network of dev.escouade.slack"
    );
    assert_eq!(
        r[4],
        "evil.example is not in permissions.network of dev.escouade.slack"
    );
    assert_eq!(r[5], "only https: http://slack.com/api/x");
    assert_eq!(
        r[6],
        "127.0.0.1 is not in permissions.network of dev.escouade.slack"
    );
    assert_eq!(seen.lock().unwrap().len(), 2);
}

#[test]
fn a_plugin_reads_only_the_files_of_its_folder() {
    let bundle = r#"
import { readFileSync } from "node:fs";
import { createInterface } from "node:readline";
createInterface({ input: process.stdin }).on("line", (line) => {
  const m = JSON.parse(line);
  if (m.method !== "read") return;
  const r = m.params.paths.map((p) => { try { return JSON.parse(readFileSync(new URL(p, import.meta.url), "utf8")).deny ? "rules" : "other"; } catch (e) { return e.code; } });
  process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id: m.id, result: r }) + "\n");
});
"#;
    let work = std::env::temp_dir().join(format!("esc319-test-{}-files", std::process::id()));
    let wasm = Compiler::pinned().compile(bundle, &work).unwrap();
    let grants = Grants {
        id: "x".into(),
        folder: repo().join("plugins/team-rules"),
        ..Default::default()
    };
    let mut p = Plugin::start(&load(&wasm).unwrap(), grants, Limits::default()).unwrap();
    let paths = [
        "../rules.json",
        "/plugin/rules.json",
        "../escouade-plugin.json",
        "../../slack/channels.json",
        "../../../package.json",
        "/etc/passwd",
        "../package.json",
    ];
    let r = p.request("read", json!({ "paths": paths })).unwrap();
    assert_eq!(r[0], "rules");
    assert_eq!(r[1], "rules");
    assert_eq!(r[2], "ENOENT", "its manifest is not given");
    assert_eq!(r[3], "ENOENT", "another plugin's file");
    assert_eq!(r[4], "ENOENT");
    assert_eq!(r[5], "ENOENT");
    assert_eq!(
        r[6], "other",
        "package.json of its folder is a JSON file of its own"
    );
}

#[test]
fn a_loop_without_end_is_stopped_and_memory_is_bounded() {
    let bundle = r#"
import { createInterface } from "node:readline";
createInterface({ input: process.stdin }).on("line", (line) => {
  const m = JSON.parse(line);
  if (m.method === "spin") for (;;) {}
  if (m.method === "grow") { const a = []; for (;;) a.push(new Array(100000).fill(1)); }
});
"#;
    let work = std::env::temp_dir().join(format!("esc319-test-{}-limits", std::process::id()));
    let module = load(&Compiler::pinned().compile(bundle, &work).unwrap()).unwrap();
    let limits = Limits {
        memory: 32 << 20,
        call: Duration::from_millis(300),
    };
    let grants = Grants {
        id: "x".into(),
        folder: work.clone(),
        ..Default::default()
    };

    let mut p = Plugin::start(&module, grants.clone(), limits).unwrap();
    let t = Instant::now();
    assert!(matches!(
        p.request("spin", Value::Null),
        Err(Fault::Timeout)
    ));
    assert!(
        t.elapsed() < Duration::from_millis(700),
        "{:?}",
        t.elapsed()
    );
    assert!(
        matches!(p.request("spin", Value::Null), Err(Fault::Crashed(_))),
        "the instance is done"
    );

    let mut p = Plugin::start(&module, grants.clone(), limits).unwrap();
    let r = p.request("grow", Value::Null);
    assert!(matches!(r, Err(Fault::Crashed(_))), "{r:?}");
    assert!(p.memory() <= 32 << 20);

    // A new instance after a crash: Escouade's restart.
    let mut p = Plugin::start(&module, grants, limits).unwrap();
    assert!(p.notify("events/fact", json!({ "name": "x" })).is_ok());
}

#[test]
fn two_compilations_of_a_bundle_give_the_same_module() {
    // What the store publishes and what the local review recompiles: the same bytes, so the same empreinte.
    let a = compiled("team-rules", "repro-a");
    let b = compiled("team-rules", "repro-b");
    assert_eq!(sha256(&a), sha256(&b));
}

#[test]
fn an_instance_that_lives_long_does_not_grow() {
    // One instance answers every call (escouade-turn), without evaluating its module again: its memory stays.
    let module = load(&compiled("team-rules", "long")).unwrap();
    let grants = Grants {
        id: "dev.escouade.team-rules".into(),
        folder: repo().join("plugins/team-rules"),
        ..Default::default()
    };
    let mut p = Plugin::start(&module, grants, Limits::default()).unwrap();
    p.request("initialize", hello("dev.escouade.team-rules"))
        .unwrap();
    for _ in 0..200 {
        p.request("permissions/decide", ask("git status")).unwrap();
    }
    let before = p.memory();
    for i in 0..3000 {
        let a = p
            .request(
                "permissions/decide",
                ask(if i % 2 == 0 { "npm test" } else { "rm -rf /" }),
            )
            .unwrap();
        assert!(a["decision"] == "allow" || a["decision"] == "deny");
    }
    assert_eq!(p.memory(), before);
}

#[test]
fn a_redirection_is_never_followed_out_of_the_permitted_hosts() {
    // slack.com answers 302 to another host: the plugin gets the 302, nothing goes elsewhere.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let elsewhere = TcpListener::bind("127.0.0.1:0").unwrap();
    let elsewhere_addr = elsewhere.local_addr().unwrap();
    elsewhere.set_nonblocking(true).unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut r = BufReader::new(stream.try_clone().unwrap());
            let mut l = String::new();
            while r.read_line(&mut l).unwrap() > 2 {
                l.clear();
            }
            let mut s = stream;
            write!(s, "HTTP/1.1 302 Found\r\nlocation: http://{elsewhere_addr}/steal\r\ncontent-length: 0\r\nconnection: close\r\n\r\n").unwrap();
        }
    });
    let bundle = r#"
import { createInterface } from "node:readline";
createInterface({ input: process.stdin }).on("line", async (line) => {
  const m = JSON.parse(line);
  if (m.method !== "try") return;
  const r = await fetch("https://slack.com/api/x");
  process.stdout.write(JSON.stringify({ jsonrpc: "2.0", id: m.id, result: [r.status, r.headers.get("location")] }) + "\n");
});
"#;
    let work = std::env::temp_dir().join(format!("esc319-test-{}-redirect", std::process::id()));
    let wasm = Compiler::pinned().compile(bundle, &work).unwrap();
    let mut p = Plugin::start(
        &load(&wasm).unwrap(),
        slack_grants(&["slack.com"], &base),
        Limits::default(),
    )
    .unwrap();
    let r = p.request("try", Value::Null).unwrap();
    assert_eq!(r[0], 302);
    assert_eq!(r[1], format!("http://{elsewhere_addr}/steal"));
    assert!(
        elsewhere.accept().is_err(),
        "nothing reached the other host"
    );
}
