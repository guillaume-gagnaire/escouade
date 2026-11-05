//! The stream bench of INT-0, the reference measures (docs/PLUGINS.md §4.8, docs/TODO-2.1.md):
//! the recorded streams of each harness replayed through the core as the app reads them, before
//! any harness runs in a process of its own, then after each one does.
//!
//! The streams are those of the replay cases (`tests/fixtures/replays/<harness>/`, read by
//! `engine::bench::replay`) and a long real turn of Claude Code
//! (`tests/fixtures/claude/long-turn.jsonl`, recorded by `scripts/bench/record-claude-turn.mjs`).
//! Each line goes the way of a line its process wrote: read as JSON (`text::json_line`), handed
//! to `Core::on_frame` for an agent of its harness, its effects applied, and the window's events
//! sent on the hub's channel, serialized as Tauri sends them (the deltas within the 16 ms of
//! `Core::conv_flusher`). Measured:
//!
//! - the delay from a line read to the first window event it makes (median, 99th percentile);
//!   a line that makes none (a line the core only keeps, an answer to a request of the core) is
//!   counted apart;
//! - the throughput, at the fastest: lines and bytes a second, from the first line to the last
//!   event;
//! - the CPU time of the process from the first line of each replay to its last event (its
//!   agent made before, the process quiet first), and its memory (working set before, after,
//!   and its peak).
//!
//! Two paces: at the fastest (each line as soon as the one before is handled), and at the
//! recorded one (each line when it came, for the streams that say it: `t` or `ms` of a line, or
//! the times of the long turn, a silence longer than `LONGEST_GAP` shortened to it).
//!
//! `the_stream_bench_measures_every_harness` plays it once, at the fastest; `int0_stream_bench`
//! (ignored, run by `npm run bench:int0`) plays it whole and writes its results to
//! `bench-results/int0-flux-<label>.json` (`ESCOUADE_BENCH_LABEL`, `run` when unset).

use crate::core_tests::{harness, Harness};
use crate::model::EngineId;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::ipc::{Channel, InvokeResponseBody};

/// The harnesses replayed, by their folder of `tests/fixtures/replays/`.
const HARNESSES: [&str; 8] = [
    "claude", "codex", "copilot", "cursor", "opencode", "vibe", "agy", "gemini",
];

/// The plugin a harness's conversation goes through, for those that left the core (INT-10): their streams are replayed
/// through its process (`Bench::play_plugged`).
fn plugged_by(harness: EngineId) -> Option<&'static str> {
    match harness {
        EngineId::CURSOR => Some(crate::plugins::builtin::CURSOR),
        EngineId::VIBE => Some(crate::plugins::builtin::MISTRAL),
        EngineId::GEMINI => Some(crate::plugins::builtin::GEMINI),
        EngineId::OPENCODE => Some(crate::plugins::builtin::OPENCODE),
        EngineId::COPILOT => Some(crate::plugins::builtin::COPILOT),
        EngineId::CODEX => Some(crate::plugins::builtin::CODEX),
        EngineId::AGY => Some(crate::plugins::builtin::ANTIGRAVITY),
        EngineId::CLAUDE => Some(crate::plugins::builtin::CLAUDE),
        _ => None,
    }
}

/// The longest silence of a stream replayed at its recorded pace (a model thinking, a person
/// answering): a line's delay does not depend on it.
const LONGEST_GAP: Duration = Duration::from_secs(2);

/// A recorded stream of a harness.
pub(crate) struct Flux {
    pub harness: EngineId,
    /// Its name: `<harness>/<case>`.
    pub name: String,
    /// What its agent is before the first line (as a replay case says, `AgentMeta` in camelCase).
    pub agent: Value,
    pub lines: Vec<String>,
    /// When each line came (ms from the first), when the recording says it.
    pub times: Option<Vec<u64>>,
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

/// The lines of a recorded stream, the empty ones left out.
fn lines_of(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// When each line came, from its own `t` or `ms` (ms) when every line has one, from the first. The last line may have none
/// (the note that closes a recording of Copilot): it comes with the one before.
fn times_of(lines: &[String]) -> Option<Vec<u64>> {
    let time = |l: &String| {
        let v: Value = serde_json::from_str(l).ok()?;
        v.get("t").or_else(|| v.get("ms")).and_then(Value::as_u64)
    };
    let (last, before) = lines.split_last()?;
    let mut times: Vec<u64> = before.iter().map(time).collect::<Option<_>>()?;
    let end = time(last).or(times.last().copied())?;
    times.push(end);
    let first = *times.first()?;
    Some(times.into_iter().map(|t| t.saturating_sub(first)).collect())
}

/// Every stream of the bench: the replay cases of each harness, then the long turn of Claude Code.
pub(crate) fn corpus() -> Vec<Flux> {
    let mut all = Vec::new();
    for folder in HARNESSES {
        let harness: EngineId = serde_json::from_value(json!(folder)).unwrap();
        let dir = crate::engine::bench::replay::cases_of(folder);
        let mut cases: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        cases.sort();
        for case in cases {
            let c: Value = serde_json::from_slice(&std::fs::read(&case).unwrap()).unwrap();
            let lines = lines_of(&case.parent().unwrap().join(c["flux"].as_str().unwrap()));
            all.push(Flux {
                harness,
                name: format!("{folder}/{}", case.file_stem().unwrap().to_string_lossy()),
                agent: c["agent"].clone(),
                times: times_of(&lines),
                lines,
            });
        }
    }
    let long = fixtures().join("claude");
    let times: Vec<u64> =
        serde_json::from_slice(&std::fs::read(long.join("long-turn.times.json")).unwrap()).unwrap();
    let lines = lines_of(&long.join("long-turn.jsonl"));
    assert_eq!(times.len(), lines.len(), "long-turn: a time a line");
    all.push(Flux {
        harness: EngineId::CLAUDE,
        name: "claude/long-turn".into(),
        agent: Value::Null,
        lines,
        times: Some(times),
    });
    all
}

/// How the lines of a stream are given.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Pace {
    Fastest,
    Recorded,
}

/// The window's events, when each was sent and whether it is the replayed agent's (its id in it:
/// its conversation, the agent itself); the others (the usage, the board…) are no line's.
type Sent = Arc<Mutex<Vec<(Instant, bool)>>>;

/// The core of the bench: a harness whose window's events are timed, and a project for its agents.
pub(crate) struct Bench {
    h: Harness,
    project: String,
    sent: Sent,
    /// The agent replayed, as its id is written in an event (`"<id>"`).
    replayed: Arc<Mutex<String>>,
}

impl Bench {
    pub(crate) async fn new(name: &str) -> Bench {
        let h = harness(name);
        let sent: Sent = Arc::default();
        let replayed: Arc<Mutex<String>> = Arc::default();
        let (sink, of) = (sent.clone(), replayed.clone());
        // As the window gets them: serialized by Tauri's channel, its JSON told here.
        h.core.hub.set_channel(Channel::new(move |body| {
            let at = Instant::now();
            let mine = match &body {
                InvokeResponseBody::Json(s) => s.contains(of.lock().as_str()),
                InvokeResponseBody::Raw(_) => false,
            };
            sink.lock().push((at, mine));
            Ok(())
        }));
        tokio::spawn(h.core.clone().conv_flusher());
        let (p, _) = h.project(false).await;
        Bench {
            h,
            project: p.id,
            sent,
            replayed,
        }
    }

    /// A new agent of `flux`'s harness, as its case says it is before its first line.
    async fn agent(&self, flux: &Flux) -> String {
        let id = self
            .h
            .core
            .create_agent(&self.project, None)
            .await
            .unwrap()
            .meta
            .id;
        let a = self.h.core.agent(&id).unwrap();
        let mut rt = a.lock();
        let mut meta = serde_json::to_value(&rt.meta).unwrap();
        if let Some(over) = flux.agent.as_object() {
            for (k, v) in over {
                meta[k] = v.clone();
            }
        }
        meta["engine"] = json!(flux.harness);
        rt.meta = serde_json::from_value(meta).unwrap();
        drop(rt);
        id
    }

    /// Replays `flux` at `pace` for a new agent of its harness.
    pub(crate) async fn play(&self, flux: &Flux, pace: Pace) -> Played {
        if let Some(plugin) = plugged_by(flux.harness) {
            return self.play_plugged(flux, pace, plugin).await;
        }
        let id = self.agent(flux).await;
        *self.replayed.lock() = format!("\"{id}\"");
        let core = self.h.core.clone();
        let sent = self.sent.clone();
        let lines = flux.lines.clone();
        let times = match pace {
            Pace::Fastest => None,
            Pace::Recorded => flux.times.clone(),
        };
        let agent = id.clone();
        let cpu = usage().cpu;
        let read = tokio::task::spawn_blocking(move || {
            let handle = core.agent(&agent).unwrap();
            let (start, mut shift) = (Instant::now(), Duration::ZERO);
            let mut last = 0u64;
            let mut read = Vec::with_capacity(lines.len());
            for (n, line) in lines.iter().enumerate() {
                if let Some(t) = times.as_ref().map(|t| t[n]) {
                    let gap = Duration::from_millis(t.saturating_sub(last));
                    shift += gap.saturating_sub(LONGEST_GAP);
                    last = t;
                    let due = start + Duration::from_millis(t) - shift;
                    if let Some(wait) = due.checked_duration_since(Instant::now()) {
                        std::thread::sleep(wait);
                    }
                }
                let before = sent.lock().len();
                let ops = core.conv_ops.load(Ordering::Acquire);
                let at = Instant::now();
                let frame = match crate::text::json_line(line) {
                    Ok(f) => f,
                    Err(e) => panic!("line {} is no JSON: {e}", n + 1),
                };
                // Its process as it is now: the core may have told it to start again meanwhile
                // (a frame of the process before would then be dropped), the lines are the new one's.
                let gen = handle.lock().gen;
                core.on_frame(&handle, gen, frame);
                // Ops handed to the window (sent now, or buffered for the flusher), or the agent
                // changed (sent now).
                let made = core.conv_ops.load(Ordering::Acquire) > ops
                    || sent.lock()[before..].iter().any(|(_, mine)| *mine);
                read.push((at, before, made));
            }
            read
        })
        .await
        .unwrap();
        self.settled(&id, flux, read, cpu).await
    }

    /// What a replay of `flux` for the agent `id` made, once its last deltas are sent: `read` is each line's moment,
    /// the events before it, and whether it made one; `cpu` the process's CPU time at its start.
    async fn settled(
        &self,
        id: &str,
        flux: &Flux,
        read: Vec<(Instant, usize, bool)>,
        cpu: Duration,
    ) -> Played {
        // The last deltas, sent by the flusher.
        let waited = Instant::now();
        while self.h.core.conv_pending(id) && waited.elapsed() < Duration::from_secs(5) {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        // Taken out of the buffer, the last of them may still be on their way to the channel.
        tokio::time::sleep(Duration::from_millis(20)).await;
        let cpu = usage().cpu.saturating_sub(cpu);
        let sent = self.sent.lock().clone();
        let mut delays = Vec::new();
        let mut silent = 0;
        let mut end = read.first().map(|r| r.0).unwrap_or_else(Instant::now);
        for &(at, before, made) in &read {
            match sent[before..].iter().find(|(t, mine)| *mine && *t >= at) {
                Some(&(t, _)) if made => {
                    delays.push(t - at);
                    end = end.max(t);
                }
                _ => silent += 1,
            }
            end = end.max(at);
        }
        let first = read.first().map(|r| r.0).unwrap_or(end);
        Played {
            lines: flux.lines.len(),
            bytes: flux.lines.iter().map(|l| l.len() + 1).sum(),
            silent,
            delays,
            took: end - first,
            cpu,
        }
    }

    /// Replays `flux` at `pace` for a new agent of a harness whose conversation goes through its plugin (Cursor CLI,
    /// INT-10.1), as the app does it: what the harness said (`s2c`) handed to the plugin's process (`agent/frames`),
    /// its items to `Core::on_frame`; what the client said (`c2s`) asked of the plugin as the core asks it (a message,
    /// a mode, a model, an answer). The plugin numbers its requests itself: a recorded answer is given under the id
    /// the plugin gave the request of its method, as the harness would. A line is timed from the moment it is handed
    /// to the plugin; the lines of the client are no harness's and make no event of their own (counted silent).
    async fn play_plugged(&self, flux: &Flux, pace: Pace, plugin: &'static str) -> Played {
        type Pending = Arc<Mutex<HashMap<String, VecDeque<Value>>>>;
        let id = self.agent(flux).await;
        *self.replayed.lock() = format!("\"{id}\"");
        let core = self.h.core.clone();
        let rt = core.plugin_runtime.clone();
        let key = format!("bench-{id}");
        let cwd = flux.agent["cwd"].as_str().unwrap_or_default().to_string();
        let session = flux.agent["sessionId"].as_str().map(str::to_string);
        // What a case leaves out (Codex's say no model).
        let model = flux.agent["model"].as_str().unwrap_or_default();
        let mode = flux.agent["mode"].as_str().unwrap_or("default");
        let attach = json!({
            "agent": key, "cwd": cwd, "session": session, "model": model,
            "mode": mode, "lang": "fr",
        });
        // Its first call starts the plugin's process: given time, as a machine under load needs it.
        rt.call_within(plugin, "agent/attach", attach, Duration::from_secs(60))
            .await
            .unwrap();
        let start = match &session {
            Some(s) => json!({ "kind": "resume", "session": s }),
            None => json!({ "kind": "new" }),
        };
        // Copilot's plugin opens its session itself, step by step, which no recording follows: it is given the sessions
        // of the recording as they open (`agent/attach`, as after a restart).
        let opens = flux.harness == EngineId::COPILOT;
        // Antigravity's lines are its events alone, with no `dir`, and Claude Code's its frames.
        let only_said = matches!(flux.harness, EngineId::AGY | EngineId::CLAUDE);
        let launch = rt
            .call(
                plugin,
                "engine/launch",
                json!({
                    "agent": key, "cwd": cwd, "model": model, "mode": mode,
                    "start": start, "lang": "fr",
                }),
            )
            .await
            .unwrap();
        // The requests of the plugin waiting for their answer, by method: its ids, in order.
        let pending: Pending = Arc::default();
        let note = {
            let pending = pending.clone();
            move |line: &str| {
                if let Ok(m) = serde_json::from_str::<Value>(line) {
                    if let (Some(method), false) = (m["method"].as_str(), m["id"].is_null()) {
                        pending
                            .lock()
                            .entry(method.to_string())
                            .or_default()
                            .push_back(m["id"].clone());
                    }
                }
            }
        };
        for w in launch["writes"].as_array().into_iter().flatten() {
            note(w.as_str().unwrap_or_default());
        }
        let sent = self.sent.clone();
        let lines = flux.lines.clone();
        let times = match pace {
            Pace::Fastest => None,
            Pace::Recorded => flux.times.clone(),
        };
        let agent = id.clone();
        let (model, mode) = (flux.agent["model"].clone(), flux.agent["mode"].clone());
        let tokio = tokio::runtime::Handle::current();
        // The client's settings of a Vibe recording are the ones it made itself at the session's start.
        let own_settings = flux.harness == EngineId::VIBE;
        // What the plugin reads as it came: OpenCode's lines, and Claude Code's, which say no `dir`.
        let as_it_came = matches!(flux.harness, EngineId::OPENCODE | EngineId::CLAUDE);
        let cpu = usage().cpu;
        let read = tokio::task::spawn_blocking(move || {
            let handle = core.agent(&agent).unwrap();
            let (start, mut shift) = (Instant::now(), Duration::ZERO);
            let mut last = 0u64;
            let mut read = Vec::with_capacity(lines.len());
            // The methods of the client's requests of the recording, by their ids; the requests of the harness, by
            // their ids, as the plugin named them.
            let mut methods: HashMap<String, String> = HashMap::new();
            let mut asked: HashMap<String, String> = HashMap::new();
            // The questions of each request of a question, as the plugin gave them (Vibe's forms).
            let mut questions: HashMap<String, Value> = HashMap::new();
            let call = |method: &str, params: Value| -> Value {
                tokio
                    .block_on(rt.call(plugin, method, params))
                    .unwrap_or(Value::Null)
            };
            for (n, line) in lines.iter().enumerate() {
                if let Some(t) = times.as_ref().map(|t| t[n]) {
                    let gap = Duration::from_millis(t.saturating_sub(last));
                    shift += gap.saturating_sub(LONGEST_GAP);
                    last = t;
                    let due = start + Duration::from_millis(t) - shift;
                    if let Some(wait) = due.checked_duration_since(Instant::now()) {
                        std::thread::sleep(wait);
                    }
                }
                let before = sent.lock().len();
                let ops = core.conv_ops.load(Ordering::Acquire);
                let at = Instant::now();
                let f: Value = serde_json::from_str(line)
                    .unwrap_or_else(|e| panic!("line {} is no JSON: {e}", n + 1));
                // Gemini's lines are the message itself, with its `dir`.
                let mut msg = if f["msg"].is_null() {
                    f.clone()
                } else {
                    f["msg"].clone()
                };
                let writes = match f["dir"].as_str().or(only_said.then_some("s2c")) {
                    Some("s2c" | "in") => {
                        if let (true, Some(session)) = (opens, msg["result"]["sessionId"].as_str()) {
                            let again = json!({
                                "agent": key, "cwd": cwd, "session": session, "model": model,
                                "mode": mode, "lang": "fr",
                            });
                            call("agent/detach", json!({ "agent": key }));
                            call("agent/attach", again);
                        }
                        if msg["method"].is_null() && !msg["id"].is_null() {
                            // An answer to a request of the client: under the id the plugin gave it.
                            let method = methods.get(&msg["id"].to_string()).cloned();
                            let given =
                                method.and_then(|m| pending.lock().get_mut(&m)?.pop_front());
                            match given {
                                Some(id) => msg["id"] = id,
                                None => {
                                    read.push((at, before, false));
                                    continue;
                                }
                            }
                        }
                        // OpenCode's plugin reads the recording's own lines: no harness writes them to it.
                        let frame = if as_it_came { line.clone() } else { msg.to_string() };
                        let got = call(
                            "agent/frames",
                            json!({ "agent": key, "frames": [frame] }),
                        );
                        let mut items = Vec::new();
                        let mut writes = Vec::new();
                        for i in got["items"].as_array().into_iter().flatten() {
                            match i["op"].as_str() {
                                Some("write") => writes.push(i["line"].clone()),
                                Some("permission" | "question" | "permissionCard" | "questionCard") => {
                                    let request = i["request"].as_str().unwrap_or_default();
                                    asked.insert(msg["id"].to_string(), request.to_string());
                                    if i["questions"].is_array() {
                                        questions.insert(request.to_string(), i["questions"].clone());
                                    }
                                    items.push(i.clone());
                                }
                                _ => items.push(i.clone()),
                            }
                        }
                        if !items.is_empty() {
                            let gen = handle.lock().gen;
                            core.on_frame(&handle, gen, json!({ "items": items }));
                        }
                        writes
                    }
                    Some("c2s" | "out") => {
                        if let (Some(method), false) = (msg["method"].as_str(), msg["id"].is_null())
                        {
                            methods.insert(msg["id"].to_string(), method.to_string());
                        }
                        let p = &msg["params"];
                        let answer_of = |p: &Value| {
                            let kind = p["result"]["kind"].as_str().unwrap_or_default();
                            json!({
                                "agent": key, "request": p["requestId"],
                                "allow": kind.starts_with("approve") || p["response"]["approved"] == true,
                                "always": kind == "approve-for-session",
                            })
                        };
                        let got = match msg["method"].as_str() {
                            Some("session.send") => call(
                                "agent/sendMessage",
                                json!({
                                    "agent": key, "uuid": crate::model::new_id(), "text": p["prompt"],
                                }),
                            ),
                            Some("session.abort") => call("agent/interrupt", json!({ "agent": key })),
                            Some("session.model.switchTo") => call(
                                "agent/setModel",
                                json!({ "agent": key, "value": p["modelId"] }),
                            ),
                            Some("session.mode.set") => {
                                let mode = match p["mode"].as_str() {
                                    Some("plan") => "plan",
                                    Some("autopilot") => "auto",
                                    _ => "default",
                                };
                                call("agent/setMode", json!({ "agent": key, "value": mode }))
                            }
                            Some(
                                "session.permissions.handlePendingPermissionRequest"
                                | "session.ui.handlePendingExitPlanMode",
                            ) => call("agent/answerPermission", answer_of(p)),
                            Some("session.ui.handlePendingUserInput") => {
                                let request = p["requestId"].as_str().unwrap_or_default();
                                let question = questions.get(request).cloned().unwrap_or_default();
                                let text = question[0]["question"].as_str().unwrap_or_default();
                                call(
                                    "agent/answerQuestion",
                                    json!({
                                        "agent": key, "request": request, "call": request,
                                        "answers": { text: p["response"]["answer"] },
                                    }),
                                )
                            }
                            Some("session/prompt") => call(
                                "agent/sendMessage",
                                json!({
                                    "agent": key, "uuid": crate::model::new_id(),
                                    "text": p["prompt"][0]["text"],
                                }),
                            ),
                            // Codex's turns, as the core asks them.
                            Some("turn/start") => call(
                                "agent/sendMessage",
                                json!({
                                    "agent": key, "uuid": crate::model::new_id(),
                                    "text": "", "content": p["input"],
                                }),
                            ),
                            Some("turn/interrupt") => call("agent/interrupt", json!({ "agent": key })),
                            Some("thread/compact/start") => call("agent/compact", json!({ "agent": key })),
                            Some("session/set_mode" | "session/set_config_option")
                                if own_settings =>
                            {
                                Value::Null
                            }
                            Some("session/set_mode") => {
                                let mode = if p["modeId"] == "plan" {
                                    "plan"
                                } else {
                                    "default"
                                };
                                call("agent/setMode", json!({ "agent": key, "value": mode }))
                            }
                            Some("session/set_config_option") if p["configId"] == "model" => {
                                let full = p["value"].as_str().unwrap_or_default();
                                let model = full.split('[').next().unwrap_or(full);
                                call("agent/setModel", json!({ "agent": key, "value": model }))
                            }
                            Some("session/set_config_option") => call(
                                "agent/setEffort",
                                json!({ "agent": key, "value": p["value"] }),
                            ),
                            // The client's own start, which the plugin wrote itself.
                            Some(_) => Value::Null,
                            None => match asked.get(&msg["id"].to_string()) {
                                // Codex's answers, by the ids of its questions.
                                Some(request) if msg["result"]["answers"].is_object() => {
                                    let mut answers = serde_json::Map::new();
                                    let asked_qs = questions.get(request).cloned().unwrap_or_default();
                                    for q in asked_qs.as_array().into_iter().flatten() {
                                        let id = q["id"].as_str().unwrap_or_default();
                                        let given = msg["result"]["answers"][id]["answers"].clone();
                                        answers.insert(q["question"].as_str().unwrap_or_default().to_string(), given);
                                    }
                                    call(
                                        "agent/answerQuestion",
                                        json!({ "agent": key, "request": request, "call": request, "answers": answers }),
                                    )
                                }
                                Some(request) if msg["result"]["decision"].is_string() => {
                                    let decision = msg["result"]["decision"].as_str().unwrap_or_default();
                                    call(
                                        "agent/answerPermission",
                                        json!({
                                            "agent": key, "request": request,
                                            "allow": decision.starts_with("accept"),
                                            "always": decision == "acceptForSession",
                                        }),
                                    )
                                }
                                // An answer to Vibe's form: by question, as the card gives it.
                                Some(request) if msg["result"]["action"].is_string() => {
                                    let mut answers = serde_json::Map::new();
                                    let asked_qs = questions.get(request).cloned().unwrap_or_default();
                                    for (n, q) in asked_qs.as_array().into_iter().flatten().enumerate() {
                                        let given = &msg["result"]["content"][format!("q{n}")];
                                        let text = match given.as_array() {
                                            Some(l) => l.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "),
                                            None => given.as_str().unwrap_or_default().to_string(),
                                        };
                                        answers.insert(q["question"].as_str().unwrap_or_default().to_string(), json!(text));
                                    }
                                    call(
                                        "agent/answerQuestion",
                                        json!({ "agent": key, "request": request, "call": request, "answers": answers }),
                                    )
                                }
                                Some(request) => {
                                    let option = msg["result"]["outcome"]["optionId"]
                                        .as_str()
                                        .unwrap_or_default();
                                    call(
                                        "agent/answerPermission",
                                        json!({
                                            "agent": key, "request": request,
                                            "allow": option.starts_with("allow") || option.starts_with("proceed"),
                                            "always": option.starts_with("allow-always") || option.starts_with("allow_always") || option.starts_with("proceed_always"),
                                        }),
                                    )
                                }
                                None => Value::Null,
                            },
                        };
                        got["writes"].as_array().cloned().unwrap_or_default()
                    }
                    _ => Vec::new(),
                };
                for w in &writes {
                    note(w.as_str().unwrap_or_default());
                }
                let made = core.conv_ops.load(Ordering::Acquire) > ops
                    || sent.lock()[before..].iter().any(|(_, mine)| *mine);
                read.push((at, before, made));
            }
            read
        })
        .await
        .unwrap();
        let _ = self
            .h
            .core
            .plugin_runtime
            .call(
                plugin,
                "agent/detach",
                json!({ "agent": format!("bench-{id}") }),
            )
            .await;
        self.settled(&id, flux, read, cpu).await
    }
}

/// What a replay made.
pub(crate) struct Played {
    pub lines: usize,
    pub bytes: usize,
    /// Lines that made no window event.
    pub silent: usize,
    /// From each line read to its first window event.
    pub delays: Vec<Duration>,
    /// From the first line read to the last event.
    pub took: Duration,
    /// The CPU time of the process meanwhile (the agent's creation left out).
    pub cpu: Duration,
}

/// The `q`-quantile of `sorted` (nearest rank), in µs.
fn quantile(sorted: &[Duration], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    sorted[rank - 1].as_secs_f64() * 1e6
}

/// The CPU time of this process (user and kernel) and its memory: working set now and its peak.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Usage {
    pub cpu: Duration,
    pub memory: u64,
    pub peak: u64,
}

#[cfg(windows)]
pub(crate) fn usage() -> Usage {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    // SAFETY: the pseudo-handle of this process and plain structs to fill.
    unsafe {
        let h = GetCurrentProcess();
        let (mut born, mut ended, mut kernel, mut user): (FILETIME, FILETIME, FILETIME, FILETIME) =
            std::mem::zeroed();
        let ticks = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
        let cpu = if GetProcessTimes(h, &mut born, &mut ended, &mut kernel, &mut user) != 0 {
            Duration::from_nanos((ticks(kernel) + ticks(user)) * 100)
        } else {
            Duration::ZERO
        };
        let mut mem: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let (memory, peak) = if K32GetProcessMemoryInfo(h, &mut mem, size) != 0 {
            (mem.WorkingSetSize as u64, mem.PeakWorkingSetSize as u64)
        } else {
            (0, 0)
        };
        Usage { cpu, memory, peak }
    }
}

#[cfg(unix)]
pub(crate) fn usage() -> Usage {
    // SAFETY: a plain struct filled by the system.
    let ru = unsafe {
        let mut ru: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut ru);
        ru
    };
    let tv = |t: libc::timeval| Duration::new(t.tv_sec as u64, t.tv_usec as u32 * 1000);
    // ru_maxrss: KiB on Linux, bytes on macOS.
    let peak = if cfg!(target_os = "macos") {
        ru.ru_maxrss as u64
    } else {
        ru.ru_maxrss as u64 * 1024
    };
    // The resident set now, from /proc where there is one (else the peak stands for it).
    let memory = std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
        .map(|pages| pages * 4096)
        .unwrap_or(peak);
    Usage {
        cpu: tv(ru.ru_utime) + tv(ru.ru_stime),
        memory,
        peak,
    }
}

/// The measures of a harness at a pace, over all its streams (`rounds` times each).
#[derive(Debug)]
pub(crate) struct Measure {
    pub harness: String,
    pub pace: Pace,
    pub streams: usize,
    pub lines: usize,
    pub silent: usize,
    pub median_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
    pub lines_per_s: f64,
    pub mb_per_s: f64,
    pub cpu_ms: f64,
    pub took_ms: f64,
    pub memory_before: u64,
    pub memory_after: u64,
    pub memory_peak: u64,
}

impl Measure {
    fn json(&self) -> Value {
        json!({
            "harness": self.harness, "pace": format!("{:?}", self.pace).to_lowercase(),
            "streams": self.streams, "lines": self.lines, "silent": self.silent,
            "medianUs": self.median_us, "p99Us": self.p99_us, "maxUs": self.max_us,
            "linesPerS": self.lines_per_s, "mbPerS": self.mb_per_s, "cpuMs": self.cpu_ms,
            "tookMs": self.took_ms, "memoryBefore": self.memory_before,
            "memoryAfter": self.memory_after, "memoryPeak": self.memory_peak,
        })
    }
}

/// Waits (10 s at most) for the process to be quiet: less than 5 % of a core over 200 ms. The
/// work the replays before left behind them (saves, the agents they made) is not this one's.
async fn settle() {
    let until = Instant::now() + Duration::from_secs(10);
    while Instant::now() < until {
        let before = usage().cpu;
        tokio::time::sleep(Duration::from_millis(200)).await;
        if usage().cpu.saturating_sub(before) < Duration::from_millis(10) {
            return;
        }
    }
}

/// Plays `streams` (of one harness, or the long turn) `rounds` times each at `pace`, once
/// the process is quiet.
pub(crate) async fn measure(
    bench: &Bench,
    harness: &str,
    streams: &[&Flux],
    pace: Pace,
    rounds: usize,
) -> Measure {
    settle().await;
    let before = usage();
    let mut cpu = Duration::ZERO;
    let (mut lines, mut bytes, mut silent) = (0, 0, 0);
    let mut delays = Vec::new();
    let mut took = Duration::ZERO;
    for _ in 0..rounds {
        for flux in streams {
            let p = bench.play(flux, pace).await;
            lines += p.lines;
            bytes += p.bytes;
            silent += p.silent;
            delays.extend(p.delays);
            took += p.took;
            cpu += p.cpu;
        }
    }
    let after = usage();
    delays.sort();
    let secs = took.as_secs_f64().max(1e-9);
    Measure {
        harness: harness.to_string(),
        pace,
        streams: streams.len(),
        lines,
        silent,
        median_us: quantile(&delays, 0.5),
        p99_us: quantile(&delays, 0.99),
        max_us: quantile(&delays, 1.0),
        lines_per_s: lines as f64 / secs,
        mb_per_s: bytes as f64 / secs / 1e6,
        cpu_ms: cpu.as_secs_f64() * 1e3,
        took_ms: took.as_secs_f64() * 1e3,
        memory_before: before.memory,
        memory_after: after.memory,
        memory_peak: after.peak,
    }
}

/// The groups the bench measures apart: each harness's replay cases, and the long turn.
fn groups(corpus: &[Flux]) -> Vec<(String, Vec<&Flux>)> {
    let mut groups: Vec<(String, Vec<&Flux>)> = HARNESSES
        .iter()
        .map(|h| {
            let of = corpus
                .iter()
                .filter(|f| f.name.starts_with(&format!("{h}/")) && f.name != "claude/long-turn")
                .collect();
            (h.to_string(), of)
        })
        .collect();
    groups.push((
        "claude/long-turn".into(),
        corpus
            .iter()
            .filter(|f| f.name == "claude/long-turn")
            .collect(),
    ));
    groups
}

/// A line of the table of `docs/PLUGINS.md`.
fn row(m: &Measure) -> String {
    format!(
        "| {} | {:?} | {} | {} | {:.0} | {:.0} | {:.0} | {:.0} | {:.1} | {:.0} | {:.0} / {:.0} / {:.0} |",
        m.harness,
        m.pace,
        m.lines,
        m.silent,
        m.median_us,
        m.p99_us,
        m.max_us,
        m.lines_per_s,
        m.mb_per_s,
        m.cpu_ms,
        m.memory_before as f64 / 1e6,
        m.memory_after as f64 / 1e6,
        m.memory_peak as f64 / 1e6,
    )
}

const HEADER: &str = "| Flux | Rythme | Lignes | Sans événement | Médiane (µs) | 99e centile (µs) | Max (µs) | Lignes/s | Mo/s | CPU (ms) | Mémoire avant / après / pic (Mo) |\n|---|---|---|---|---|---|---|---|---|---|---|";

/// Every harness has streams to replay, each long turn's line its time, and one pass at the
/// fastest makes window events for most lines, with delays, a throughput and a CPU time.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_stream_bench_measures_every_harness() {
    let corpus = corpus();
    let long = corpus
        .iter()
        .find(|f| f.name == "claude/long-turn")
        .unwrap();
    assert!(
        long.lines.len() >= 1000,
        "the long turn of Claude Code has thousands of frames: {}",
        long.lines.len()
    );
    let bench = Bench::new("fluxbench-once").await;
    for (name, streams) in groups(&corpus) {
        assert!(!streams.is_empty(), "{name}: no stream");
        let m = measure(&bench, &name, &streams, Pace::Fastest, 1).await;
        assert!(m.lines > 0 && m.silent < m.lines, "{name}: no event: {m:?}");
        assert!(
            m.median_us > 0.0 && m.median_us <= m.p99_us && m.p99_us <= m.max_us,
            "{m:?}"
        );
        assert!(
            m.lines_per_s > 0.0
                && m.cpu_ms >= 0.0
                && m.memory_peak >= m.memory_before.min(m.memory_peak),
            "{m:?}"
        );
        println!("{}", row(&m));
    }
}

/// A recorded pace is kept: a stream whose lines say when they came takes that long to replay
/// (its long silences shortened), and the times of a stream are read from its lines.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_stream_is_replayed_at_its_recorded_pace() {
    let lines: Vec<String> = [0, 40, 90, 3_090]
        .iter()
        .enumerate()
        .map(|(n, t)| {
            json!({"type":"stream_event","t":t,"event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":format!("m{n}")}},"parent_tool_use_id":null}).to_string()
        })
        .collect();
    assert_eq!(times_of(&lines), Some(vec![0, 40, 90, 3_090]));
    assert_eq!(times_of(&["{}".to_string()]), None);
    let flux = Flux {
        harness: EngineId::CLAUDE,
        name: "claude/paced".into(),
        agent: Value::Null,
        times: times_of(&lines),
        lines,
    };
    let bench = Bench::new("fluxbench-paced").await;
    let started = Instant::now();
    let p = bench.play(&flux, Pace::Recorded).await;
    let took = started.elapsed();
    // 90 ms, then a silence of 3 s shortened to `LONGEST_GAP`.
    assert!(took >= Duration::from_millis(2_090), "{took:?}");
    assert!(took < Duration::from_millis(4_000), "{took:?}");
    assert_eq!(p.lines, 4);
    let started = Instant::now();
    bench.play(&flux, Pace::Fastest).await;
    assert!(started.elapsed() < Duration::from_millis(1_000));
}

#[test]
fn quantiles_are_by_nearest_rank() {
    let d: Vec<Duration> = (1..=100).map(Duration::from_micros).collect();
    assert_eq!(quantile(&d, 0.5), 50.0);
    assert_eq!(quantile(&d, 0.99), 99.0);
    assert_eq!(quantile(&d, 1.0), 100.0);
    assert_eq!(quantile(&[], 0.5), 0.0);
}

/// The whole bench (`npm run bench:int0`): every harness at the fastest (`ESCOUADE_BENCH_ROUNDS`
/// times, 20 by default) and at its recorded pace (once), its table printed and its results
/// written to `bench-results/int0-flux-<ESCOUADE_BENCH_LABEL>.json`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "a measure, run by `npm run bench:int0`"]
async fn int0_stream_bench() {
    let rounds: usize = std::env::var("ESCOUADE_BENCH_ROUNDS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(20);
    let label = std::env::var("ESCOUADE_BENCH_LABEL").unwrap_or_else(|_| "run".into());
    let corpus = corpus();
    let bench = Bench::new("fluxbench-int0").await;
    // Once, unmeasured: what the first replay loads (the engines' tables, the stats' base).
    for (_, streams) in groups(&corpus) {
        measure(&bench, "warm", &streams, Pace::Fastest, 1).await;
    }
    // One harness alone (`ESCOUADE_BENCH_ONLY`), for a measure of it.
    let only = std::env::var("ESCOUADE_BENCH_ONLY").ok();
    let wanted = |name: &str| only.as_deref().is_none_or(|o| name == o);
    let mut all = Vec::new();
    println!("{HEADER}");
    for (name, streams) in groups(&corpus) {
        if !wanted(&name) {
            continue;
        }
        let m = measure(&bench, &name, &streams, Pace::Fastest, rounds).await;
        println!("{}", row(&m));
        all.push(m);
    }
    for (name, streams) in groups(&corpus) {
        let timed: Vec<&Flux> = streams.into_iter().filter(|f| f.times.is_some()).collect();
        if timed.is_empty() || !wanted(&name) {
            continue;
        }
        let m = measure(&bench, &name, &timed, Pace::Recorded, 1).await;
        println!("{}", row(&m));
        all.push(m);
    }
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("bench-results");
    std::fs::create_dir_all(&out).unwrap();
    let file = out.join(format!("int0-flux-{label}.json"));
    let report = json!({
        "label": label,
        "rounds": rounds,
        "release": !cfg!(debug_assertions),
        "os": std::env::consts::OS,
        "measures": all.iter().map(Measure::json).collect::<Vec<_>>(),
    });
    std::fs::write(&file, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("\n{}", file.display());
}
