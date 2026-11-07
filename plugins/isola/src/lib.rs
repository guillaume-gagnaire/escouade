//! The isola plugin (`dev.escouade.isola`, INT-14): the launch environment of a worktree that has an `.isola.toml`.
//!
//! The app declares the rest (`contributes.launchEnvironments` of its manifest: the file that says a worktree is
//! isola's, the program, the `up` command a test launch runs). This module does what needs the program: `launch/ls`
//! (the services of the worktree, from `isola ls --json`), `launch/down` and `launch/destroy`.
//!
//! A module runs nothing itself. It asks the host (`proc/spawn`, docs/PLUGINS.md §8.4.7), which runs what
//! `permissions.commands` names and nothing else, and finds the program on the PATH. The app lends the worktree for the
//! time of a call, under the name it gives in `agent`: the module never holds a path. What the program writes and its end
//! come back as `proc/out` and `proc/exit`.
//!
//! Only `up` runs the commands the `.isola.toml` declares (its services and their `setup`), and the user approves that
//! file before it starts; `ls`, `down` and `destroy` run no command the file declares. That holds for the current
//! upstream isola, and is to be checked again when isola changes.

use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::serve::{self, Cx, Plugin, Refusal};
use escouade_plugin_kit::time::timeout;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::poll_fn;
use std::sync::Mutex;
use std::task::{Poll, Waker};
use std::time::Duration;

/// How long `isola ls`, `down` or `destroy` may take.
const LIMIT: Duration = Duration::from_secs(60);

/// How long a program that was killed may take to say it ended.
const KILLED: Duration = Duration::from_secs(5);

/// The code with which the plugin says that isola failed (a refusal that asking again would not change).
const FAILED: i64 = -32010;

/// What a call is given.
#[derive(Deserialize)]
struct Call {
    /// The name of the worktree the app lent for this call.
    agent: String,
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

/// A program under way: what it wrote so far, its end, and who waits for it.
#[derive(Default)]
struct Run {
    out: String,
    err: String,
    /// Its exit code, none when it was killed; `ended` says it came.
    code: Option<i64>,
    ended: bool,
    waker: Option<Waker>,
}

/// The plugin: the programs under way, by what `proc/spawn` answered. `proc/out` may come before that answer.
#[derive(Default)]
pub struct Isola {
    runs: Mutex<HashMap<u64, Run>>,
}

fn proc_of(params: &Value) -> Option<u64> {
    params.get("proc").and_then(Value::as_u64)
}

impl Isola {
    fn with_run(&self, id: u64, then: impl FnOnce(&mut Run)) {
        let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
        then(runs.entry(id).or_default());
    }

    /// `proc/out`: what a program wrote.
    fn wrote(&self, params: &Value) {
        let (Some(id), Some(data)) = (proc_of(params), params["data"].as_str()) else {
            return;
        };
        self.with_run(id, |run| {
            let into = if params["stream"] == "stderr" {
                &mut run.err
            } else {
                &mut run.out
            };
            into.push_str(data);
        });
    }

    /// `proc/exit`: a program ended, after all it wrote.
    fn ended(&self, params: &Value) {
        let Some(id) = proc_of(params) else { return };
        self.with_run(id, |run| {
            run.code = params["code"].as_i64();
            run.ended = true;
            if let Some(waker) = run.waker.take() {
                waker.wake();
            }
        });
    }

    /// Waits for the end of the program `id`.
    async fn exited(&self, id: u64) -> Run {
        poll_fn(|task| {
            let mut runs = self.runs.lock().unwrap_or_else(|e| e.into_inner());
            let run = runs.entry(id).or_default();
            if run.ended {
                Poll::Ready(runs.remove(&id).unwrap_or_default())
            } else {
                run.waker = Some(task.waker().clone());
                Poll::Pending
            }
        })
        .await
    }

    /// `isola <args>` in the worktree lent for `call`, `LIMIT` at most, its input closed: what it printed, else why it
    /// failed. The host ties it to this instance; the services `up` starts are not its children: `ls`, `down` and
    /// `destroy` start none.
    async fn run(&self, cx: &Cx, call: &Call, args: &[&str]) -> Result<String, String> {
        let lang = call.lang.as_str();
        let joined = args.join(" ");
        let spawned = cx
            .request(
                "proc/spawn",
                json!({
                    "command": "isola",
                    "args": args,
                    "dir": "agent",
                    "agent": call.agent,
                    "env": { "NO_COLOR": "1" },
                }),
            )
            .await
            .map_err(|e| {
                let e = e.message;
                say(
                    lang,
                    format!("isola {joined} n'a pas pu démarrer : {e}"),
                    format!("isola {joined} couldn’t start: {e}"),
                )
            })?;
        let Some(id) = proc_of(&spawned) else {
            return Err(format!("isola {joined}: no program from the host"));
        };
        // It may have ended already: nothing to close then.
        let _ = cx
            .request("proc/write", json!({ "proc": id, "close": true }))
            .await;
        let Ok(run) = timeout(LIMIT, self.exited(id)).await else {
            let _ = cx.request("proc/kill", json!({ "proc": id })).await;
            let _ = timeout(KILLED, self.exited(id)).await;
            let s = LIMIT.as_secs();
            return Err(say(
                lang,
                format!("isola {joined} n'a pas répondu en {s} s"),
                format!("isola {joined} didn’t answer in {s} s"),
            ));
        };
        if run.code != Some(0) {
            let line = run
                .err
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
        Ok(run.out)
    }

    async fn handle(&self, cx: &Cx, method: &str, params: Value) -> Result<Value, Refusal> {
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
            "proc/out" => {
                self.wrote(&params);
                Ok(Value::Null)
            }
            "proc/exit" => {
                self.ended(&params);
                Ok(Value::Null)
            }
            "launch/ls" => {
                let call = call()?;
                let ls = self
                    .run(cx, &call, &["ls", "--json"])
                    .await
                    .map_err(failed)?;
                let services = services_of(&ls, &call.branch, &call.lang).map_err(failed)?;
                Ok(json!(services))
            }
            "launch/down" => self
                .run(cx, &call()?, &["down"])
                .await
                .map(|_| Value::Null)
                .map_err(failed),
            "launch/destroy" => self
                .run(cx, &call()?, &["destroy"])
                .await
                .map(|_| Value::Null)
                .map_err(failed),
            other => Err(Refusal::not_found(other)),
        }
    }
}

impl Plugin for Isola {
    fn call<'a>(
        &'a self,
        cx: &'a Cx,
        method: &'a str,
        params: Value,
    ) -> BoxFuture<'a, Result<Value, Refusal>> {
        Box::pin(self.handle(cx, method, params))
    }
}

pub fn run() -> i32 {
    serve::run("isola", Isola::default())
}

escouade_plugin_kit::protocol::module!(run);

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use escouade_plugin_kit::protocol::code;
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, Lines};

    /// The plugin on its loop, and the end of the host.
    struct Host {
        to: DuplexStream,
        from: Lines<BufReader<DuplexStream>>,
    }

    impl Host {
        fn start() -> Self {
            let (to, input) = tokio::io::duplex(1 << 16);
            let (output, from) = tokio::io::duplex(1 << 16);
            let plugin = Arc::new(Isola::default());
            tokio::spawn(async move { serve::serve_on(plugin, input, output).await });
            Self {
                to,
                from: BufReader::new(from).lines(),
            }
        }

        async fn send(&mut self, v: Value) {
            self.to
                .write_all(format!("{v}\n").as_bytes())
                .await
                .unwrap();
        }

        async fn next(&mut self) -> Value {
            let line = self.from.next_line().await.unwrap().expect("a line");
            serde_json::from_str(&line).unwrap()
        }

        async fn answer(&mut self, asked: &Value, result: Value) {
            self.send(json!({ "jsonrpc": "2.0", "id": asked["id"], "result": result }))
                .await;
        }

        async fn refuse(&mut self, asked: &Value, message: &str) {
            self.send(json!({
                "jsonrpc": "2.0",
                "id": asked["id"],
                "error": { "code": -32003, "message": message },
            }))
            .await;
        }

        async fn tell(&mut self, method: &str, params: Value) {
            self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
                .await;
        }

        async fn call(&mut self, method: &str, lang: &str) {
            self.send(json!({
                "jsonrpc": "2.0",
                "id": "call",
                "method": method,
                "params": { "agent": "wt", "branch": "feat", "lang": lang },
            }))
            .await;
        }

        /// The call's program is started, `proc` 7, its input closed.
        async fn started(&mut self, args: Value) {
            let spawn = self.next().await;
            assert_eq!(spawn["method"], "proc/spawn");
            let p = &spawn["params"];
            assert_eq!(p["command"], "isola");
            assert_eq!(p["args"], args);
            assert_eq!(
                (p["dir"].as_str(), p["agent"].as_str()),
                (Some("agent"), Some("wt")),
                "a root, never a path"
            );
            assert_eq!(p["env"], json!({ "NO_COLOR": "1" }));
            self.answer(&spawn, json!({ "proc": 7 })).await;
            let write = self.next().await;
            assert_eq!(write["method"], "proc/write");
            assert_eq!(write["params"], json!({ "proc": 7, "close": true }));
            self.answer(&write, Value::Null).await;
        }
    }

    #[tokio::test]
    async fn a_call_gathers_what_the_program_writes_until_its_end() {
        let mut h = Host::start();
        h.call("launch/ls", "en").await;
        h.started(json!(["ls", "--json"])).await;
        let out = r#"[{"worktree":"feat","service":"web","status":"running","url":"http://a"}]"#;
        // Cut anywhere: what comes is a stream.
        let (a, b) = out.split_at(11);
        for data in [a, b] {
            h.tell(
                "proc/out",
                json!({ "proc": 7, "stream": "stdout", "data": data }),
            )
            .await;
        }
        h.tell("proc/exit", json!({ "proc": 7, "code": 0 })).await;
        let done = h.next().await;
        assert_eq!(
            done["result"],
            json!([{ "name": "web", "status": "running", "url": "http://a", "probe": "http://a" }])
        );
    }

    #[tokio::test]
    async fn the_end_of_a_program_before_its_number_is_kept() {
        let mut h = Host::start();
        h.call("launch/down", "en").await;
        let spawn = h.next().await;
        h.tell("proc/exit", json!({ "proc": 7, "code": 0 })).await;
        h.answer(&spawn, json!({ "proc": 7 })).await;
        let write = h.next().await;
        // Ended already: the host has no such program any more.
        h.refuse(&write, "no program 7").await;
        let done = h.next().await;
        assert_eq!(done["id"], "call");
        assert_eq!(done["result"], Value::Null);
    }

    #[tokio::test]
    async fn its_failure_says_the_last_line_of_its_errors_in_the_language_of_the_call() {
        let mut h = Host::start();
        h.call("launch/down", "fr").await;
        h.started(json!(["down"])).await;
        h.tell(
            "proc/out",
            json!({ "proc": 7, "stream": "stderr", "data": "first\nError: no docker\n\n" }),
        )
        .await;
        h.tell("proc/exit", json!({ "proc": 7, "code": 1 })).await;
        let e = h.next().await;
        assert_eq!(e["error"]["code"], FAILED);
        assert_eq!(
            e["error"]["message"],
            "isola down a échoué : Error: no docker"
        );

        let mut h = Host::start();
        h.call("launch/destroy", "en").await;
        h.started(json!(["destroy"])).await;
        // Killed: it has no code.
        h.tell("proc/exit", json!({ "proc": 7, "code": null }))
            .await;
        assert_eq!(h.next().await["error"]["message"], "isola destroy failed: ");
    }

    #[tokio::test]
    async fn a_program_the_host_refuses_says_why() {
        let mut h = Host::start();
        h.call("launch/down", "en").await;
        let spawn = h.next().await;
        h.refuse(&spawn, "no program isola on this machine").await;
        let e = h.next().await;
        assert_eq!(e["error"]["code"], FAILED);
        assert_eq!(
            e["error"]["message"],
            "isola down couldn’t start: no program isola on this machine"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_program_past_the_limit_is_killed_and_said() {
        let mut h = Host::start();
        h.call("launch/ls", "fr").await;
        h.started(json!(["ls", "--json"])).await;
        let kill = h.next().await;
        assert_eq!(kill["method"], "proc/kill");
        assert_eq!(kill["params"], json!({ "proc": 7 }));
        h.answer(&kill, Value::Null).await;
        h.tell("proc/exit", json!({ "proc": 7, "code": null }))
            .await;
        assert_eq!(
            h.next().await["error"]["message"],
            "isola ls --json n'a pas répondu en 60 s"
        );
    }

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

    #[tokio::test]
    async fn a_call_without_its_worktree_or_for_an_unknown_method_is_refused() {
        let mut h = Host::start();
        h.send(json!({ "jsonrpc": "2.0", "id": 1, "method": "launch/ls", "params": {} }))
            .await;
        assert_eq!(
            h.next().await["error"]["code"],
            -32602,
            "the params are required"
        );
        h.send(json!({ "jsonrpc": "2.0", "id": 2, "method": "launch/up", "params": {} }))
            .await;
        assert_eq!(h.next().await["error"]["code"], code::METHOD_NOT_FOUND);
    }
}
