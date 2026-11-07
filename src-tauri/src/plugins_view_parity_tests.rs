//! The parity of the views (ESC-352, docs/PLUGINS.md §8): what each terminal view makes of the recordings of its tests, by
//! the entries and hooks the core receives, against `tests/fixtures/plugins/parity/view.json`, written by the translation
//! the core linked (`ESCOUADE_VIEW_PARITY=record`). The ids and times the core makes are normalized; everything else
//! of the conversation, of the agent and of each step counts.

use crate::agent::{AgentRt, Effects};
use crate::model::{AgentMeta, EngineId};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;

/// What a step gives to read.
enum Step {
    Log(Value),
    Hook(Value),
}

/// A recording of the tests of the views: the agent it is read by, then its steps.
struct Recording {
    name: &'static str,
    engine: EngineId,
    cwd: &'static str,
    session: Option<&'static str>,
    steps: Vec<Step>,
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures")
}

fn read(path: &str) -> Vec<Value> {
    std::fs::read_to_string(fixtures().join(path))
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn logs(path: &str) -> Vec<Step> {
    read(path).into_iter().map(Step::Log).collect()
}

/// A hook, as the core receives it: the event named by `escouade-hook <event>`.
fn named(event: &str, payload: &Value) -> Step {
    let mut h = payload.clone();
    h[crate::tui::EVENT_KEY] = json!(event);
    Step::Hook(h)
}

/// Its hooks are all that « Vue Gemini » reads: the same turn as `agent/gemini_tui_tests.rs` plays by hand.
fn gemini_steps() -> Vec<Step> {
    let h = |event: &str, rest: Value| {
        let mut p = json!({ "session_id": "s-gemini" });
        for (k, v) in rest.as_object().unwrap() {
            p[k] = v.clone();
        }
        named(event, &p)
    };
    let write = json!({ "file_path": "C:\\w\\a.txt", "content": "x\n" });
    vec![
        h("SessionStart", json!({})),
        h("BeforeAgent", json!({ "prompt": "Va" })),
        h(
            "BeforeTool",
            json!({ "tool_name": "run_shell_command", "tool_input": { "command": "ls" } }),
        ),
        h(
            "Notification",
            json!({ "notification_type": "ToolPermission", "message": "ls ?" }),
        ),
        h(
            "AfterTool",
            json!({
                "tool_name": "run_shell_command", "tool_input": { "command": "ls" },
                "tool_response": { "llmContent": "a.txt" },
            }),
        ),
        h(
            "BeforeTool",
            json!({ "tool_name": "write_file", "tool_input": write }),
        ),
        h(
            "AfterTool",
            json!({
                "tool_name": "write_file", "tool_input": write,
                "tool_response": { "llmContent": "", "error": { "message": "denied" } },
            }),
        ),
        h("AfterAgent", json!({ "prompt_response": "Fini." })),
        h("BeforeAgent", json!({ "prompt": "Encore" })),
        h("BeforeAgent", json!({ "prompt": "Une autre" })),
        h("AfterAgent", json!({ "prompt_response": "" })),
    ]
}

fn recordings() -> Vec<Recording> {
    let recording = |name, engine, cwd, session, steps| Recording {
        name,
        engine,
        cwd,
        session,
        steps,
    };
    let codex = Some("01a1466d-084c-71a3-acb9-dadb322d905f");
    let copilot = Some("0a108c31-29e9-484a-8014-52a9fd3eaa88");
    let copilot_cwd = "C:\\Users\\user\\AppData\\Local\\Temp\\esc23\\wt-agent1";
    let cursor = Some("6753be15-7845-4e8d-abf9-3c3ed0985561");
    let cursor_cwd = "C:\\Users\\user\\AppData\\Local\\Temp\\esc24\\wt-agent1";
    let claude = Some("55555555-2222-3333-4444-555555555555");
    let vibe = Some("9a9345e7-3db5-2873-705d-f50226790eba");
    let agy = Some("434725b7-c90f-46a3-8a2b-d41956239965");
    let agy_cwd = "C:\\tmp\\esc272\\w1";

    let mut codex_hooks: Vec<Step> = read("codex/hooks-payloads.jsonl")
        .iter()
        .map(|l| Step::Hook(l["payload"].clone()))
        .collect();
    codex_hooks.extend(logs("codex/tui-rollout.jsonl"));
    let mut copilot_hooks: Vec<Step> = read("copilot/tui-hooks.jsonl")
        .iter()
        .filter_map(|l| {
            Some(named(
                l["hook"].as_str()?.strip_prefix("dir:")?,
                &l["input"],
            ))
        })
        .collect();
    copilot_hooks.extend(logs("copilot/tui-events.jsonl"));
    let mut claude_steps = logs("tui-transcript.jsonl");
    claude_steps.extend(read("tui-hooks.jsonl").into_iter().map(Step::Hook));
    let mut agy_hooks: Vec<Step> = read("agy/tui-hook-payloads.jsonl")
        .iter()
        .map(|l| named(l["event"].as_str().unwrap_or_default(), l))
        .collect();
    agy_hooks.extend(logs("agy/tui-transcript-question-interrupted.jsonl"));

    vec![
        recording(
            "claude",
            EngineId::CLAUDE,
            "/work/demo",
            claude,
            claude_steps,
        ),
        recording(
            "claude-background",
            EngineId::CLAUDE,
            "/work/demo",
            claude,
            logs("tui-background-command.jsonl"),
        ),
        recording(
            "codex",
            EngineId::CODEX,
            "/work/demo",
            codex,
            logs("codex/tui-rollout.jsonl"),
        ),
        recording(
            "codex-hooks",
            EngineId::CODEX,
            "/work/demo",
            None,
            codex_hooks,
        ),
        recording(
            "copilot",
            EngineId::COPILOT,
            copilot_cwd,
            copilot,
            logs("copilot/tui-events.jsonl"),
        ),
        recording(
            "copilot-hooks",
            EngineId::COPILOT,
            copilot_cwd,
            copilot,
            copilot_hooks,
        ),
        recording(
            "cursor-tui",
            EngineId::CURSOR,
            cursor_cwd,
            cursor,
            logs("cursor/transcript-tui.jsonl"),
        ),
        recording(
            "cursor-print",
            EngineId::CURSOR,
            cursor_cwd,
            cursor,
            logs("cursor/transcript-print.jsonl"),
        ),
        recording(
            "cursor-subagent",
            EngineId::CURSOR,
            cursor_cwd,
            cursor,
            logs("cursor/transcript-subagent.jsonl"),
        ),
        recording(
            "vibe-tool",
            EngineId::VIBE,
            "C:\\x\\repo",
            vibe,
            logs("vibe/tui-journal-tool.jsonl"),
        ),
        recording(
            "vibe-permission",
            EngineId::VIBE,
            "C:\\x\\repo",
            vibe,
            logs("vibe/tui-journal-permission.jsonl"),
        ),
        recording(
            "vibe-interrupted",
            EngineId::VIBE,
            "C:\\x\\repo",
            vibe,
            logs("vibe/tui-journal-interrupted.jsonl"),
        ),
        recording(
            "agy-command",
            EngineId::AGY,
            agy_cwd,
            agy,
            logs("agy/tui-transcript-command-resumed.jsonl"),
        ),
        recording("agy-question", EngineId::AGY, agy_cwd, None, agy_hooks),
        recording(
            "agy-write",
            EngineId::AGY,
            agy_cwd,
            agy,
            logs("agy/tui-transcript-write.jsonl"),
        ),
        recording("gemini", EngineId::GEMINI, "C:\\w", None, gemini_steps()),
    ]
}

/// The ids the core made, the same from a run to the next, and its times.
#[derive(Default)]
struct Normal {
    ids: HashMap<String, String>,
}

impl Normal {
    fn id(&mut self, s: &str) -> Value {
        let next = self.ids.len();
        json!(self
            .ids
            .entry(s.to_string())
            .or_insert_with(|| format!("id-{next}")))
    }

    fn of(&mut self, v: &Value) -> Value {
        match v {
            Value::Object(o) => {
                let mut out = Map::new();
                for (k, v) in o {
                    let shown = match k.as_str() {
                        "ts" | "durationMs" | "startedAt" | "endedAt" | "since" => {
                            json!(v.is_number())
                        }
                        _ => self.of(v),
                    };
                    out.insert(k.clone(), shown);
                }
                Value::Object(out)
            }
            Value::Array(a) => Value::Array(a.iter().map(|v| self.of(v)).collect()),
            Value::String(s) if made_by_the_core(s) => self.id(s),
            // A sum of costs, whose order a map chooses.
            Value::Number(n) if n.is_f64() => {
                json!((n.as_f64().unwrap_or_default() * 1e6).round() / 1e6)
            }
            _ => v.clone(),
        }
    }
}

/// An id the core or a view made: of `new_id`, a uuid, or a row of a view.
fn made_by_the_core(s: &str) -> bool {
    if s.starts_with("gemini-tui-") || s.starts_with("acp-text-") {
        return true;
    }
    let digits = s.chars().filter(|c| *c != '-').count();
    matches!(digits, 12 | 32) && s.chars().all(|c| c == '-' || c.is_ascii_hexdigit())
}

fn played(r: &Recording) -> Value {
    let dir = std::env::temp_dir().join(format!("escouade-view-parity-{}", uuid::Uuid::new_v4()));
    let meta = AgentMeta {
        id: "a1".into(),
        engine: r.engine,
        cwd: r.cwd.into(),
        session_id: r.session.map(str::to_string),
        ..Default::default()
    };
    let mut a = AgentRt::new(meta.clone(), &dir);
    a.start_tui();
    let (view, _) = crate::tui::view_of(&meta).unwrap();
    let mut normal = Normal::default();
    let mut steps = Vec::new();
    let mut fx = Effects::default();
    for step in &r.steps {
        let out = match step {
            Step::Log(e) => {
                view.on_log_entry(&mut a, e, &mut fx);
                Value::Null
            }
            Step::Hook(h) => {
                let o = view.on_hook(&mut a, h, &mut fx);
                json!({ "stopped": o.stopped, "transcript": o.transcript })
            }
        };
        let shown = json!({
            "status": format!("{:?}", a.meta.status),
            "prompts": a.meta.prompts,
            "end": fx.turn_end.take().map(|e| format!("{e:?}")),
            "alert": fx.notify.take().map(|n| n.body),
            "out": out,
            "turns": fx.turns.drain(..).map(|t| json!([t.model, t.input, t.cache, t.output, t.cost])).collect::<Vec<_>>(),
            "rate": fx.rate.take().map(|r| r.len()),
        });
        steps.push(normal.of(&shown));
    }
    a.stop_tui(&mut fx);
    let conv: Vec<Value> = a.conv.items().iter().map(|i| normal.of(i)).collect();
    let _ = std::fs::remove_dir_all(&dir);
    json!({
        "steps": steps,
        "end": fx.turn_end.take().map(|e| format!("{e:?}")),
        "conv": conv,
        "agent": {
            "status": format!("{:?}", a.meta.status), "prompts": a.meta.prompts,
            "cost": normal.of(&json!(a.meta.cost)), "tokens": a.meta.tokens, "session": a.meta.session_id, "entry": a.meta.last_entry,
            "mode": a.meta.mode, "plan": a.meta.plan.is_some(),
        },
    })
}

#[test]
fn each_view_reads_its_recordings_as_the_linked_translation_did() {
    let path = fixtures().join("plugins/parity/view.json");
    let all: Map<String, Value> = recordings()
        .iter()
        .map(|r| (r.name.to_string(), played(r)))
        .collect();
    if std::env::var("ESCOUADE_VIEW_PARITY").as_deref() == Ok("record") {
        let text = serde_json::to_string_pretty(&Value::Object(all)).unwrap();
        std::fs::write(&path, text + "\n").unwrap();
        return;
    }
    let want: Map<String, Value> =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(want.len(), all.len());
    for (name, got) in &all {
        let want = &want[name];
        for key in ["agent", "end"] {
            assert_eq!(&got[key], &want[key], "{name}: {key}");
        }
        for key in ["steps", "conv"] {
            let (got, want) = (got[key].as_array().unwrap(), want[key].as_array().unwrap());
            for (i, (g, w)) in got.iter().zip(want).enumerate() {
                assert_eq!(g, w, "{name}: {key} {i}");
            }
            assert_eq!(got.len(), want.len(), "{name}: {key}");
        }
    }
}

/// INT-0 for the views (`--ignored`, in release): each recording read line by line, as the core reads it, `ESCOUADE_VIEW_ROUNDS`
/// times: what a line takes, from its entry to its effects on the agent. Its other half is the same test on the commit before
/// ESC-352, the translation linked.
#[test]
#[ignore]
fn int0_view_bench() {
    let rounds: usize = std::env::var("ESCOUADE_VIEW_ROUNDS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(200);
    for r in recordings() {
        let mut each = Vec::new();
        let started = std::time::Instant::now();
        for _ in 0..rounds {
            let dir =
                std::env::temp_dir().join(format!("escouade-view-int0-{}", uuid::Uuid::new_v4()));
            let meta = AgentMeta {
                id: "a1".into(),
                engine: r.engine,
                cwd: r.cwd.into(),
                session_id: r.session.map(str::to_string),
                ..Default::default()
            };
            let mut a = AgentRt::new(meta.clone(), &dir);
            a.start_tui();
            let (view, _) = crate::tui::view_of(&meta).unwrap();
            let mut fx = Effects::default();
            for step in &r.steps {
                let at = std::time::Instant::now();
                match step {
                    Step::Log(e) => view.on_log_entry(&mut a, e, &mut fx),
                    Step::Hook(h) => {
                        view.on_hook(&mut a, h, &mut fx);
                    }
                }
                each.push(at.elapsed());
                fx = Effects::default();
            }
            drop(a);
            let _ = std::fs::remove_dir_all(&dir);
        }
        let total = started.elapsed().as_secs_f64();
        each.sort();
        let at = |p: usize| each[(each.len() * p / 100).min(each.len() - 1)].as_secs_f64() * 1e6;
        println!(
            "{:<20} {:>6} lines  p50 {:>8.1} us  p99 {:>9.1} us  {:>8.0} lines/s",
            r.name,
            each.len(),
            at(50),
            at(99),
            each.len() as f64 / total
        );
    }
}

/// An instance gone between two turns (a fault, its plugin stopped): the next one starts on what the conversation holds, and
/// nothing is shown twice.
#[test]
fn a_view_goes_on_on_another_instance_and_shows_nothing_twice() {
    let r = recordings()
        .into_iter()
        .find(|r| r.name == "copilot")
        .unwrap();
    let dir = std::env::temp_dir().join(format!("escouade-view-parity-{}", uuid::Uuid::new_v4()));
    let meta = AgentMeta {
        id: "a1".into(),
        engine: r.engine,
        cwd: r.cwd.into(),
        session_id: r.session.map(str::to_string),
        ..Default::default()
    };
    let mut a = AgentRt::new(meta.clone(), &dir);
    a.start_tui();
    let (view, _) = crate::tui::view_of(&meta).unwrap();
    let mut fx = Effects::default();
    for (i, step) in r.steps.iter().enumerate() {
        if i == r.steps.len() / 2 {
            crate::plugins::view::forget(crate::plugins::builtin::COPILOT);
        }
        if let Step::Log(e) = step {
            view.on_log_entry(&mut a, e, &mut fx);
        }
    }
    let items = a.conv.items();
    let mut ids: Vec<&str> = items.iter().filter_map(|i| i["id"].as_str()).collect();
    let all = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), all, "an item told twice");
    assert_eq!(a.meta.prompts, 2);
    let notices = items.iter().filter(|i| i["kind"] == "notice").count();
    assert_eq!(notices, 0, "{items:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
