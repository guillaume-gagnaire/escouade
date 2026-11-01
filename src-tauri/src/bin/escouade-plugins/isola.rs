//! The isola plugin (`dev.escouade.isola`, INT-14): the launch environment of a worktree that has an `.isola.toml`.
//!
//! The app declares the rest (`contributes.launchEnvironments` of its manifest: the file that says a worktree is
//! isola's, the program, the `up` command a test launch runs). This process does what needs the program: `launch/ls`
//! (the services of the worktree, from `isola ls --json`), `launch/down` and `launch/destroy`. The app names the
//! program it found on the PATH in each call, and the language of its messages; the plugin looks nothing up itself.
//!
//! Only `up` runs the commands the `.isola.toml` declares (its services and their `setup`), and the user approves that
//! file before it starts; `ls`, `down` and `destroy` run no command the file declares. That holds for the current
//! upstream isola, and is to be checked again when isola changes.

use escouade_plugin_protocol::{serve, Refusal};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long `isola ls`, `down` or `destroy` may take.
const LIMIT: Duration = Duration::from_secs(60);

/// The code with which the plugin says that isola failed (a refusal that asking again would not change).
const FAILED: i64 = -32010;

/// What a call is given.
#[derive(Deserialize)]
struct Call {
    /// The isola CLI the app found.
    program: String,
    /// The worktree.
    dir: String,
    /// Its branch (`launch/ls`).
    #[serde(default)]
    branch: String,
    /// "fr" or "en": the language of the messages.
    #[serde(default)]
    lang: String,
}

/// A line of `isola ls --json`: a service of a worktree.
#[derive(Debug, Deserialize)]
struct Entry {
    #[serde(default)]
    worktree: String,
    #[serde(default)]
    service: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    direct_url: String,
}

/// A service of a worktree, for its test launch.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Service {
    name: String,
    /// "running", "stopped"…, as isola says it.
    status: String,
    /// Its address through isola's proxy, else on its own port; empty for a background process.
    url: String,
    /// What tells it is up: its own port (the proxy answers, with an error page, before it does), else `url`.
    probe: String,
}

fn say(lang: &str, fr: String, en: String) -> String {
    if lang == "en" {
        en
    } else {
        fr
    }
}

/// The services `ls` (the output of `isola ls --json`) lists for the worktree on `branch`.
fn services_of(ls: &str, branch: &str, lang: &str) -> Result<Vec<Service>, String> {
    let ls = ls.trim();
    // An empty list may come out as `null`.
    let entries: Vec<Entry> = if ls.is_empty() || ls == "null" {
        Vec::new()
    } else {
        serde_json::from_str(ls).map_err(|e| {
            say(
                lang,
                format!("réponse d'isola illisible : {e}"),
                format!("isola’s answer can’t be read: {e}"),
            )
        })?
    };
    Ok(entries
        .into_iter()
        .filter(|e| e.worktree == branch)
        .map(|e| {
            let url = if e.url.is_empty() {
                e.direct_url.clone()
            } else {
                e.url
            };
            let probe = if e.direct_url.is_empty() {
                url.clone()
            } else {
                e.direct_url
            };
            Service {
                name: e.service,
                status: e.status,
                url,
                probe,
            }
        })
        .collect())
}

/// `isola <args>` in `dir`, `LIMIT` at most: what it printed, else why it failed. Not tied to the plugin's own life:
/// the services `up` starts outlive it on purpose, and `ls`, `down` and `destroy` start none.
fn run(call: &Call, args: &[&str]) -> Result<String, String> {
    let lang = call.lang.as_str();
    let joined = args.join(" ");
    let mut cmd = Command::new(&call.program);
    cmd.args(args)
        .current_dir(&call.dir)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|e| {
        say(
            lang,
            format!("isola {joined} n'a pas pu démarrer : {e}"),
            format!("isola {joined} couldn’t start: {e}"),
        )
    })?;
    // Drained apart: a full pipe would hold isola, and the wait with it.
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut out = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut out);
            }
            out
        })
    };
    let out = drain(child.stdout.take().map(|p| Box::new(p) as _));
    let err = drain(child.stderr.take().map(|p| Box::new(p) as _));
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                let s = LIMIT.as_secs();
                return Err(say(
                    lang,
                    format!("isola {joined} n'a pas répondu en {s} s"),
                    format!("isola {joined} didn’t answer in {s} s"),
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => return Err(e.to_string()),
        }
    };
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr);
        let line = stderr
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_string();
        return Err(say(
            lang,
            format!("isola {joined} a échoué : {line}"),
            format!("isola {joined} failed: {line}"),
        ));
    }
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

fn handle(method: &str, params: Value) -> Result<Value, Refusal> {
    let call = || {
        serde_json::from_value::<Call>(params.clone()).map_err(|e| Refusal {
            code: -32602,
            message: format!("invalid params: {e}"),
        })
    };
    let failed = |message: String| Refusal {
        code: FAILED,
        message,
    };
    match method {
        "launch/ls" => {
            let call = call()?;
            if !Path::new(&call.dir).is_dir() {
                return Err(failed(format!("{} is not a folder", call.dir)));
            }
            let ls = run(&call, &["ls", "--json"]).map_err(failed)?;
            let services = services_of(&ls, &call.branch, &call.lang).map_err(failed)?;
            Ok(json!(services))
        }
        "launch/down" => run(&call()?, &["down"])
            .map(|_| Value::Null)
            .map_err(failed),
        "launch/destroy" => run(&call()?, &["destroy"])
            .map(|_| Value::Null)
            .map_err(failed),
        other => Err(Refusal::not_found(other)),
    }
}

pub fn run_plugin() -> i32 {
    let stdin = std::io::stdin();
    match serve(stdin.lock(), std::io::stdout().lock(), json!({}), handle) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("escouade-plugins isola: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_worktrees_services_open_on_their_proxy_address_and_are_probed_on_their_own() {
        let ls = r#"[
            {"worktree":"main","service":"web","port":3100,"status":"running","pid":1,"url":"http://main.demo.localhost:3000","direct_url":"http://localhost:3100"},
            {"worktree":"ticket/dem-1","service":"api","port":8117,"status":"running","pid":2,"direct_url":"http://localhost:8117"},
            {"worktree":"ticket/dem-1","service":"web","port":3117,"status":"running","pid":3,"url":"http://ticket-dem-1.demo.localhost:3000","direct_url":"http://localhost:3117"},
            {"worktree":"ticket/dem-1","service":"worker","port":0,"status":"stopped","pid":0}
        ]"#;
        let s = services_of(ls, "ticket/dem-1", "en").unwrap();
        let got: Vec<(&str, &str, &str, &str)> = s
            .iter()
            .map(|s| {
                (
                    s.name.as_str(),
                    s.status.as_str(),
                    s.url.as_str(),
                    s.probe.as_str(),
                )
            })
            .collect();
        // The proxy answers (502) before the service does: the service itself is probed.
        assert_eq!(
            got,
            [
                (
                    "api",
                    "running",
                    "http://localhost:8117",
                    "http://localhost:8117"
                ),
                (
                    "web",
                    "running",
                    "http://ticket-dem-1.demo.localhost:3000",
                    "http://localhost:3117"
                ),
                ("worker", "stopped", "", ""),
            ]
        );
        assert!(services_of("null\n", "main", "en").unwrap().is_empty());
        assert!(services_of("", "main", "en").unwrap().is_empty());
        assert!(services_of("[]", "main", "en").unwrap().is_empty());
        assert!(services_of("isola: not a repo", "main", "en").is_err());
    }

    #[test]
    fn a_call_without_its_program_or_for_an_unknown_method_is_refused() {
        assert_eq!(
            handle("launch/ls", json!({})).unwrap_err().code,
            -32602,
            "the params are required"
        );
        assert_eq!(
            handle("launch/up", json!({})).unwrap_err().code,
            escouade_plugin_protocol::code::METHOD_NOT_FOUND
        );
    }
}
