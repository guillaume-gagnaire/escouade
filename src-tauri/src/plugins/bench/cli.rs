//! `escouade-plugin`, the command of the plugins' authors (PLG-3, ESC-175): what runs a plugin, on the app's host.
//! `test` plays the conformance bench on a folder, `dev` starts it under the fake Escouade and sends it what is typed
//! (facts, decisions, calls), showing what comes back. The rest of the command (`validate`, `build`, `--version`) needs
//! no host and lives in `escouade_plugin_cli::cli` (ESC-328), which the store's image builds alone.

use super::fake::{Reply, Seen, Session};
use super::{decision_request, fact, program_of, run, Options, Program};

use crate::plugins::manifest::Manifest;
use crate::plugins::runtime::{timeout_of, PROTOCOL};
use escouade_plugin_cli::cli::{self, read_manifest, usage, Call};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Runs the command of `args` (without the program's name); gives its exit code.
pub fn main(args: Vec<String>) -> i32 {
    cli::main(args, Some(&host))
}

/// `test` and `dev`.
fn host(call: &Call) -> i32 {
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let (folder, lang, hand) = (call.folder, call.lang, call.hand);
    if call.command == "test" {
        let measure = call.args.iter().position(|a| a == "--measure").map(|i| {
            call.args
                .get(i + 1)
                .and_then(|n| n.parse().ok())
                .unwrap_or(1000)
        });
        let options = Options {
            lang,
            hand,
            app_version: call.app_version.to_string(),
            measure: measure.unwrap_or(0),
            module: call.value("--wasm").map(PathBuf::from),
            offline: call.flag("--offline"),
            accounts: call
                .args
                .iter()
                .enumerate()
                .filter(|(_, a)| *a == "--account")
                .filter_map(|(i, _)| call.args.get(i + 1))
                .map(|url| crate::forge::host_of(url))
                .collect(),
            ..Options::default()
        };
        let report = rt.block_on(run(folder, &options));
        if call.flag("--json") {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_default()
            );
        } else {
            print!("{}", report.render(lang));
        }
        return i32::from(!report.passed());
    }
    let mut secrets = HashMap::new();
    for (i, a) in call.args.iter().enumerate() {
        if a == "--secret" {
            if let Some((k, v)) = call.args.get(i + 1).and_then(|kv| kv.split_once('=')) {
                secrets.insert(k.to_string(), v.to_string());
            }
        }
    }
    let o = Options {
        lang,
        hand,
        app_version: call.app_version.to_string(),
        ..Options::default()
    };
    rt.block_on(dev(folder, &o, secrets))
}

/// `[json]` after a command's words: an object, or nothing.
fn json_arg(rest: &str) -> Result<Value, String> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(rest).map_err(|e| e.to_string())
}

fn show(reply: &Reply) -> String {
    match reply {
        Reply::Answered {
            result: Ok(v),
            elapsed,
        } => format!("← {v}  ({} ms)", elapsed.as_millis()),
        Reply::Answered {
            result: Err(e),
            elapsed,
        } => format!(
            "← error {} {}  ({} ms)",
            e.code,
            e.message,
            elapsed.as_millis()
        ),
        Reply::Silent => "← (no answer)".into(),
        Reply::Ended { code, tail } => format!(
            "← (ended, {}) {}",
            code.map_or("no code".into(), |c| format!("code {c}")),
            tail.join(" | ")
        ),
    }
}

fn show_seen(s: &Seen) -> String {
    match s {
        Seen::Secret { key, own: true } => format!("→ secrets/get {key:?}"),
        Seen::Secret { key, own: false } => {
            format!("→ secrets/get {key:?}: refused, not one of its own")
        }
        Seen::Log { level, message } => format!("→ log [{level}] {message}"),
        Seen::Stderr { line } => format!("  stderr: {line}"),
        Seen::Unserved { method } => format!("→ {method}: unknown method"),
        Seen::Notification { method } => format!("→ notification {method}"),
        Seen::Stray { line } => format!("→ (answers nothing) {line}"),
    }
}

/// Starts the plugin and initializes it, showing the answer; none when it did not start.
async fn begin(
    folder: &Path,
    m: &Manifest,
    program: &Program,
    o: &Options,
    secrets: &HashMap<String, String>,
) -> Option<Session> {
    let s = match program.start(folder, m, o, secrets.clone(), None) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {e}", program.path().display());
            return None;
        }
    };
    match s.pid {
        Some(pid) => println!("{} (pid {pid})", program.path().display()),
        None => println!("{} (WASM)", program.path().display()),
    }
    let mut seen = s.watch();
    tokio::spawn(async move {
        while let Some(x) = seen.recv().await {
            println!("{}", show_seen(&x));
        }
    });
    let hello = Session::hello(m, &o.app_version, PROTOCOL);
    println!("→ initialize");
    let reply = s.call("initialize", &hello, timeout_of("initialize")).await;
    println!("{}", show(&reply));
    Some(s)
}

async fn dev(folder: &Path, o: &Options, secrets: HashMap<String, String>) -> i32 {
    let lang = o.lang;
    let Some(m) = read_manifest(folder, lang, o.hand) else {
        return 1;
    };
    // As Escouade would: not a plugin of another version's range (`--escouade` plays another).
    if let Some(why) = m.refusal(&o.app_version, lang) {
        eprintln!("✗ {why}");
        return 1;
    }
    if m.run.is_none() {
        eprintln!(
            "{}",
            tr_in!(
                lang,
                "plugin déclaratif : aucun programme à lancer",
                "a declarative plugin: no program to start"
            )
        );
        return 1;
    }
    let program = match program_of(folder, &m, o) {
        Ok(p) => p,
        Err((said, missing)) => {
            eprintln!("✗ {said}");
            if !missing.is_empty() {
                eprintln!("  {missing}");
            }
            return 1;
        }
    };
    let Some(mut s) = begin(folder, &m, &program, o, &secrets).await else {
        return 1;
    };
    // What is typed, read off the runtime's threads.
    let (tx, mut lines) = tokio::sync::mpsc::unbounded_channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    loop {
        print!("> ");
        let _ = std::io::stdout().flush();
        let Some(line) = lines.recv().await else {
            break;
        };
        let line = line.trim();
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        match word {
            "" => {}
            "quit" | "exit" => break,
            "restart" => {
                s.kill();
                s.wait_end(Duration::from_secs(5)).await;
                match begin(folder, &m, &program, o, &secrets).await {
                    Some(n) => s = n,
                    None => return 1,
                }
            }
            "fact" => {
                let (name, extra) = rest.trim().split_once(' ').unwrap_or((rest.trim(), ""));
                if name.is_empty() {
                    eprintln!("fact <name> [json]");
                    continue;
                }
                match json_arg(extra) {
                    Ok(extra) => {
                        let mut f = fact(name);
                        if let (Some(f), Some(extra)) = (f.as_object_mut(), extra.as_object()) {
                            f.extend(extra.clone());
                        }
                        println!("→ events/fact {f}");
                        s.notify("events/fact", &f);
                    }
                    Err(e) => eprintln!("{e}"),
                }
            }
            "decide" => match json_arg(rest) {
                Ok(extra) => {
                    let mut d = decision_request();
                    if let (Some(d), Some(extra)) = (d.as_object_mut(), extra.as_object()) {
                        d.extend(extra.clone());
                    }
                    let method = "permissions/decide";
                    let delay = timeout_of(method);
                    let reply = s.call(method, &d, delay + Duration::from_secs(5)).await;
                    println!("{}", show(&reply));
                    if let Reply::Answered { elapsed, .. } = reply {
                        if elapsed > delay {
                            println!(
                                "{}",
                                tr_in!(
                                    lang,
                                    "  (au-delà de son délai de {} ms : l'utilisateur aurait été questionné)",
                                    "  (past its delay of {} ms: the user would have been asked)",
                                    delay.as_millis()
                                )
                            );
                        }
                    }
                }
                Err(e) => eprintln!("{e}"),
            },
            "call" | "notify" => {
                let (method, extra) = rest.trim().split_once(' ').unwrap_or((rest.trim(), ""));
                if method.is_empty() {
                    eprintln!("{word} <method> [json]");
                    continue;
                }
                match json_arg(extra) {
                    Ok(params) if word == "call" => {
                        let reply = s.call(method, &params, timeout_of(method)).await;
                        println!("{}", show(&reply));
                    }
                    Ok(params) => {
                        s.notify(method, &params);
                    }
                    Err(e) => eprintln!("{e}"),
                }
            }
            _ => eprint!("{}", usage(lang)),
        }
        if let Some(code) = s.ended() {
            println!(
                "{}",
                tr_in!(
                    lang,
                    "le plugin s'est terminé ({:?}) ; `restart` le relance",
                    "the plugin ended ({:?}); `restart` starts it again",
                    code
                )
            );
        }
    }
    println!("→ shutdown");
    let reply = s
        .call("shutdown", &Value::Null, timeout_of("shutdown"))
        .await;
    println!("{}", show(&reply));
    s.wait_end(Duration::from_secs(1)).await;
    s.kill();
    0
}
