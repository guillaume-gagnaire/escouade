//! Recorded real streams of a harness, replayed through its translation and compared with the
//! items expected (docs/ENGINES.md, « Les flux enregistrés »).
//!
//! A case is a JSON file of `tests/fixtures/replays/<engine>/`:
//!
//! ```json
//! {
//!   "about": "What was recorded, with which version, and what it shows.",
//!   "flux": "../../real-run-send-message.jsonl",
//!   "agent": { "cwd": "C:\\Users\\dev\\repo" },
//!   "items": [ { "kind": "text", "text": "Bonjour", "parent": null } ],
//!   "agentAfter": { "sessionId": "…", "status": "done" }
//! }
//! ```
//!
//! - `flux`: the stream as it was recorded, one frame (a JSON value) a line, left as it came;
//!   relative to the case.
//! - `agent` (optional): what the agent is before the first frame, over a new agent's defaults,
//!   as its state is saved (`AgentMeta`, camelCase).
//! - `items`: the conversation the frames make, in order, without its `id` nor its times (`ts`,
//!   `startedAt`, `endedAt`), made up at each replay. Each expected item is compared with the one
//!   made by what it says only: a key it leaves out is not compared (an object's keys within it
//!   neither; a list has its length). Numbers are compared to their last bit.
//! - `agentAfter` (optional): what the agent is after the last frame, compared the same way
//!   (without its times either).
//!
//! With `ESCOUADE_BLESS=1`, a case is written again from what its frames make (its `items` and the
//! keys of its `agentAfter`, or a few of them when it has none) instead of compared: the diff is
//! then read before it is committed.

use crate::agent::{AgentRt, Effects};
use crate::engine::Engine;
use crate::model::AgentMeta;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// The times of an item, made up at each replay (its `id` too, at its top).
const TIMES: [&str; 3] = ["ts", "startedAt", "endedAt"];

/// `v` without the times of the replay (`TIMES`), at any depth.
fn strip_times(v: &mut Value) {
    match v {
        Value::Object(o) => {
            for k in TIMES {
                o.remove(k);
            }
            o.values_mut().for_each(strip_times);
        }
        Value::Array(a) => a.iter_mut().for_each(strip_times),
        _ => {}
    }
}

/// The keys of the agent a blessed case keeps when it says none.
const AFTER: [&str; 6] = [
    "sessionId",
    "lastEntry",
    "status",
    "touchedFiles",
    "tokens",
    "cost",
];

/// The folder of the cases of `engine` (`claude`).
pub(crate) fn cases_of(engine: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("replays")
        .join(engine)
}

/// What the frames of a case made: the conversation without its volatile keys, and the agent.
pub(crate) struct Replayed {
    pub items: Vec<Value>,
    pub agent: Value,
}

/// The frames of `flux`, one a line, through the translation of `engine`, for an agent that is
/// `agent` (over a new agent's defaults) before the first.
pub(crate) fn replay(engine: &dyn Engine, flux: &str, agent: &Value, conv: &Path) -> Replayed {
    let mut meta = serde_json::to_value(AgentMeta {
        id: crate::model::new_id(),
        ..Default::default()
    })
    .unwrap();
    if let Some(over) = agent.as_object() {
        for (k, v) in over {
            meta[k] = v.clone();
        }
    }
    let meta: AgentMeta = serde_json::from_value(meta).expect("the case's agent");
    let mut rt = AgentRt::new(meta, conv);
    for (n, line) in flux
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let frame: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {} of the flux is no JSON: {e}", n + 1));
        engine.translate(&mut rt, &frame, &mut Effects::default());
    }
    // A harness that tells no duration of its turns has them timed by Escouade, from the start of
    // the turn (made up at each replay as the times are): there must be one, and it is not kept.
    let timed = matches!(
        engine.id(),
        crate::model::EngineId::COPILOT
            | crate::model::EngineId::OPENCODE
            | crate::model::EngineId::CURSOR
            | crate::model::EngineId::VIBE
            | crate::model::EngineId::AGY
            | crate::model::EngineId::GEMINI
    );
    let items = rt
        .conv
        .items()
        .into_iter()
        .map(|mut item| {
            if let Some(o) = item.as_object_mut() {
                o.remove("id");
                if timed && o.get("kind").and_then(Value::as_str) == Some("turn") {
                    assert!(
                        o.get("durationMs").is_some_and(Value::is_u64),
                        "a turn of {} with no duration: {o:?}",
                        engine.id().as_str()
                    );
                    o.insert("durationMs".into(), Value::Null);
                }
            }
            strip_times(&mut item);
            item
        })
        .collect();
    // The times of its plan too (its subagents' `startedAt`, `endedAt`).
    let mut agent = serde_json::to_value(&rt.meta).unwrap();
    strip_times(&mut agent);
    Replayed { items, agent }
}

/// Replays the case `case` through `engine` and compares what it made with what it expects (or
/// writes it again, `ESCOUADE_BLESS`). Panics on the first difference, saying where.
pub(crate) fn check(engine: &dyn Engine, case: &Path) {
    let name = case.display();
    let mut c: Value =
        serde_json::from_slice(&std::fs::read(case).unwrap_or_else(|e| panic!("{name}: {e}")))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    let flux_path = case.parent().unwrap().join(
        c["flux"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: no flux")),
    );
    let flux = std::fs::read_to_string(&flux_path)
        .unwrap_or_else(|e| panic!("{name}: {}: {e}", flux_path.display()));
    let conv = crate::paths::test_dir(&format!(
        "replay-{}-{}",
        engine.id().as_str(),
        case.file_stem().unwrap().to_string_lossy()
    ));
    let made = replay(engine, &flux, &c["agent"], &conv);
    if std::env::var_os("ESCOUADE_BLESS").is_some() {
        c["items"] = Value::Array(made.items);
        let keys: Vec<String> = match c["agentAfter"].as_object() {
            Some(o) => o.keys().cloned().collect(),
            None => AFTER.iter().map(|k| k.to_string()).collect(),
        };
        let after: Map<String, Value> = keys
            .into_iter()
            .map(|k| {
                let v = made.agent[&k].clone();
                (k, v)
            })
            .collect();
        c["agentAfter"] = Value::Object(after);
        let mut text = serde_json::to_string_pretty(&c).unwrap();
        text.push('\n');
        std::fs::write(case, text).unwrap();
        return;
    }
    let expected = c["items"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: no items"));
    let kinds = |items: &[Value]| -> Vec<String> {
        items
            .iter()
            .map(|i| i["kind"].as_str().unwrap_or("?").to_string())
            .collect()
    };
    assert_eq!(
        kinds(expected),
        kinds(&made.items),
        "{name}: the items made are not those expected (their kinds, in order)"
    );
    for (n, (want, got)) in expected.iter().zip(&made.items).enumerate() {
        if let Err(at) = within(want, got, "") {
            panic!(
                "{name}: item {n} ({}), at {at}\nexpected: {want:#}\nmade: {got:#}",
                got["kind"]
            );
        }
    }
    if let Err(at) = within(&c["agentAfter"], &made.agent, "") {
        panic!("{name}: the agent after the flux, at {at}");
    }
}

/// Whether `got` says all that `want` says (`want`'s keys only; a list has its length); Err: the
/// path where it does not, with both values.
pub(crate) fn within(want: &Value, got: &Value, at: &str) -> Result<(), String> {
    match (want, got) {
        (Value::Null, _) if at.is_empty() => Ok(()),
        (Value::Object(w), Value::Object(g)) => w.iter().try_for_each(|(k, v)| {
            within(v, g.get(k).unwrap_or(&Value::Null), &format!("{at}.{k}"))
        }),
        (Value::Array(w), Value::Array(g)) if w.len() == g.len() => w
            .iter()
            .zip(g)
            .enumerate()
            .try_for_each(|(i, (w, g))| within(w, g, &format!("{at}[{i}]"))),
        _ if want == got => Ok(()),
        // Read back from the case, a float may differ by its last bit (serde_json parses fast).
        (Value::Number(w), Value::Number(g))
            if w.as_f64().zip(g.as_f64()).is_some_and(|(w, g)| {
                (w - g).abs() <= 1e-12 * w.abs().max(g.abs()).max(1e-300)
            }) =>
        {
            Ok(())
        }
        _ => Err(format!(
            "{}: expected {}, made {}",
            if at.is_empty() { "." } else { at },
            json!(want),
            json!(got)
        )),
    }
}

/// Every case of the folder of `engine` (`cases_of`), checked; panics when there is none.
pub(crate) fn check_all(engine: &dyn Engine, folder: &str) {
    let dir = cases_of(folder);
    let mut cases: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no case in {}", dir.display());
    for case in cases {
        check(engine, &case);
    }
}

#[test]
fn an_item_is_within_another_by_the_keys_it_says_and_a_list_by_its_length() {
    let got = json!({ "kind": "tool", "name": "Bash", "input": { "command": "ls", "cwd": "/" }, "status": "ok" });
    assert_eq!(within(&json!({ "kind": "tool" }), &got, ""), Ok(()));
    assert_eq!(
        within(&json!({ "input": { "command": "ls" } }), &got, ""),
        Ok(())
    );
    assert_eq!(
        within(&json!({ "input": { "command": "pwd" } }), &got, ""),
        Err(r#".input.command: expected "pwd", made "ls""#.into())
    );
    // A key the item has not is null there.
    assert!(within(&json!({ "parent": "toolu_1" }), &got, "").is_err());
    assert_eq!(within(&json!({ "parent": null }), &got, ""), Ok(()));
    assert!(within(&json!([1, 2]), &json!([1, 2, 3]), "x").is_err());
    // A float read back may differ by its last bit, never more.
    assert_eq!(
        within(
            &json!(0.0011167399999999995),
            &json!(0.0011167399999999997),
            ""
        ),
        Ok(())
    );
    assert!(within(&json!(0.00111674), &json!(0.00111675), "").is_err());
}
