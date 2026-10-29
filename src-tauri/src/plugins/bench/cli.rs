//! `escouade-plugin`, the command of the plugins' authors (PLG-3, ESC-175): `test` plays the
//! conformance bench on a folder, `validate` reads its manifest, `dev` starts it under the fake
//! Escouade and sends it what is typed (facts, decisions, calls), showing what comes back.

use super::fake::{Reply, Seen, Session};
use super::{decision_request, fact, run, Options};
use crate::i18n::Lang;
use crate::plugins::manifest::{self, Manifest};
use crate::plugins::runtime::{program_path, timeout_of, PROTOCOL};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn usage(lang: Lang) -> String {
    tr_in!(
        lang,
        "escouade-plugin : tester un plugin d'Escouade sans l'app.

  escouade-plugin test <dossier> [--json] [--measure [n]]
      Joue le banc de conformité : initialisation, version refusée, délais, un fait reçu,
      une décision, un secret, plantage, fermeture. Code de sortie 1 si un scénario échoue.
  escouade-plugin validate <dossier>
      Lit le manifeste (escouade-plugin.json) et dit ce qui ne va pas, champ par champ.
  escouade-plugin dev <dossier> [--secret <clé>=<valeur>]...
      Le faux Escouade : lance le plugin, puis envoie ce que tu tapes :
        fact <nom> [json]       un fait (events/fact), ex. fact ticket.column
        decide [json]           une demande d'autorisation (permissions/decide)
        call <méthode> [json]   un appel, et sa réponse
        notify <méthode> [json] une notification
        restart                 tue le plugin et le relance
        quit                    shutdown, puis la fin

Options : --lang fr|en
",
        "escouade-plugin: test an Escouade plugin without the app.

  escouade-plugin test <folder> [--json] [--measure [n]]
      Plays the conformance bench: initialization, version refused, delays, a fact received,
      a decision, a secret, crash, shutdown. Exit code 1 when a scenario fails.
  escouade-plugin validate <folder>
      Reads the manifest (escouade-plugin.json) and says what is wrong, field by field.
  escouade-plugin dev <folder> [--secret <key>=<value>]...
      The fake Escouade: starts the plugin, then sends what you type:
        fact <name> [json]       a fact (events/fact), e.g. fact ticket.column
        decide [json]            a permission request (permissions/decide)
        call <method> [json]     a call, and its answer
        notify <method> [json]   a notification
        restart                  kills the plugin and starts it again
        quit                     shutdown, then the end

Options: --lang fr|en
"
    )
}

/// Runs the command of `args` (without the program's name); gives its exit code.
pub fn main(args: Vec<String>) -> i32 {
    let mut lang = crate::i18n::system();
    if let Some(i) = args.iter().position(|a| a == "--lang") {
        if let Some(l) = args.get(i + 1) {
            lang = crate::i18n::of_locale(l);
        }
    }
    let Some(command) = args.first().map(String::as_str) else {
        eprint!("{}", usage(lang));
        return 2;
    };
    if matches!(command, "-h" | "--help" | "help") {
        print!("{}", usage(lang));
        return 0;
    }
    let Some(folder) = args
        .get(1)
        .filter(|a| !a.starts_with("--"))
        .map(PathBuf::from)
    else {
        eprint!("{}", usage(lang));
        return 2;
    };
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
    let flag = |name: &str| args.iter().any(|a| a == name);
    match command {
        "test" => {
            let measure = args
                .iter()
                .position(|a| a == "--measure")
                .map(|i| args.get(i + 1).and_then(|n| n.parse().ok()).unwrap_or(1000));
            let options = Options {
                lang,
                builtin: flag("--builtin"),
                measure: measure.unwrap_or(0),
                ..Options::default()
            };
            let report = rt.block_on(run(&folder, &options));
            if flag("--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).unwrap_or_default()
                );
            } else {
                print!("{}", report.render(lang));
            }
            i32::from(!report.passed())
        }
        "validate" => validate(&folder, lang, flag("--builtin")),
        "dev" => {
            let mut secrets = HashMap::new();
            for (i, a) in args.iter().enumerate() {
                if a == "--secret" {
                    if let Some((k, v)) = args.get(i + 1).and_then(|kv| kv.split_once('=')) {
                        secrets.insert(k.to_string(), v.to_string());
                    }
                }
            }
            rt.block_on(dev(&folder, lang, secrets, flag("--builtin")))
        }
        _ => {
            eprint!("{}", usage(lang));
            2
        }
    }
}

/// The manifest of `folder`, or what is wrong with it, said on the standard error.
fn read_manifest(folder: &Path, lang: Lang, builtin: bool) -> Option<Manifest> {
    let path = folder.join(manifest::FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return None;
        }
    };
    let read = manifest::read(&text, builtin);
    for f in &read.ignored {
        eprintln!(
            "{}",
            tr_in!(lang, "champ ignoré : {f}", "ignored field: {f}")
        );
    }
    match read.manifest {
        Ok(m) => Some(m),
        Err(problems) => {
            for p in problems {
                eprintln!("✗ {}", p.message(lang));
            }
            None
        }
    }
}

fn validate(folder: &Path, lang: Lang, builtin: bool) -> i32 {
    let Some(m) = read_manifest(folder, lang, builtin) else {
        return 1;
    };
    if let Some(why) = m.refusal(env!("CARGO_PKG_VERSION"), lang) {
        eprintln!("✗ {why}");
        return 1;
    }
    println!(
        "{}",
        tr_in!(
            lang,
            "✓ {} {} : le manifeste est valide",
            "✓ {} {}: the manifest holds",
            m.id,
            m.version
        )
    );
    0
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
    program: &Path,
    secrets: &HashMap<String, String>,
) -> Option<Session> {
    let s = match Session::start(
        folder,
        m,
        program,
        env!("CARGO_PKG_VERSION"),
        secrets.clone(),
        None,
    ) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {e}", program.display());
            return None;
        }
    };
    println!(
        "{} (pid {})",
        program.display(),
        s.pid.map(|p| p.to_string()).unwrap_or_default()
    );
    let mut seen = s.watch();
    tokio::spawn(async move {
        while let Some(x) = seen.recv().await {
            println!("{}", show_seen(&x));
        }
    });
    let hello = Session::hello(m, env!("CARGO_PKG_VERSION"), PROTOCOL);
    println!("→ initialize");
    let reply = s.call("initialize", &hello, timeout_of("initialize")).await;
    println!("{}", show(&reply));
    Some(s)
}

async fn dev(folder: &Path, lang: Lang, secrets: HashMap<String, String>, builtin: bool) -> i32 {
    let Some(m) = read_manifest(folder, lang, builtin) else {
        return 1;
    };
    let Some(run) = m.run.as_ref() else {
        eprintln!(
            "{}",
            tr_in!(
                lang,
                "plugin déclaratif : aucun programme à lancer",
                "a declarative plugin: no program to start"
            )
        );
        return 1;
    };
    let escouade_dir = Options::default().escouade_dir;
    let program = program_path(folder, run, &escouade_dir);
    let Some(mut s) = begin(folder, &m, &program, &secrets).await else {
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
                match begin(folder, &m, &program, &secrets).await {
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
