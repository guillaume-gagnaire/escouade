//! Mistral Vibe's ways beside ACP (the extension `vibe` of a manifest, INT-10.6), as the core's own client of Vibe 3.0.3
//! drove it (ESC-265 to 268, its streams in `tests/fixtures/vibe/`):
//! - its own tools are named in `_meta.tool_name`: `agents.spawn` is a subagent (`Task`), a tool of an MCP server is
//!   `mcp_<server>.<tool>`, its task list (`vibe.todo`) is told by `plan` and its question (`ui.ask_user_question`) by
//!   `elicitation/create`, both left out as calls, as its search of tools (`search_tool_functions`);
//! - its files are named from the agent's folder (`b.txt`): made absolute; a command's code is `returncode`;
//! - its permission names its call alone (`toolCall.toolCallId`): the call as its `tool_call` told it is put back in it,
//!   and a write's diff, told by its `tool_call` only, is kept for its result;
//! - its questions are a form (`elicitation/create`, asked for by `elicitation.form` in `initialize`);
//! - it has no mode Plan: Escouade's is played here (each prompt says to plan without changing anything, its writes
//!   refused, the turn's last words the plan to approve);
//! - a refusal of the user's cuts the turn (`session/cancel`): Vibe would otherwise try the command again under other
//!   forms, each asking again (nine times in the real run, ESC-265);
//! - its cost and tokens (`usage_update`) are the session's so far: a turn's are what they grew by (ESC-267); its rate
//!   limit (`RATE_LIMITED`) ends the turn « limité », resumed a minute later.

use crate::texts::tr;
use serde_json::{json, Value};

/// What every prompt says in Escouade's mode Plan (Vibe has none): the model plans without changing anything, its last
/// words the plan.
pub const PLAN_MODE: &str = "<escouade_plan_mode>\nYou are in plan mode: study what is asked and propose a plan, without changing anything (no file written, no command that writes). End your turn with the whole plan, in Markdown, as your last words: the user approves it before any change.\n</escouade_plan_mode>";

/// What an approved plan is followed by.
pub const PLAN_GO: &str = "The plan is approved: carry it out now.";

/// Vibe's « Always allow », kept in the agent's own `VIBE_HOME`.
pub const PERMANENT: &str = "allow_always_permanent";

/// Vibe's error of a rate limit (`acp/exceptions.py`, 3.0.3): Vibe has tried again for 5 minutes already before it
/// gives it.
pub const RATE_LIMITED: i64 = -31001;

/// How long after a rate limit the agent goes on: Vibe tells no reset, and Mistral's limits are by the minute.
pub const RATE_RESUME_MS: i64 = 60_000;

/// The tools of Vibe shown otherwise than as calls (or not at all).
const UNSHOWN: [&str; 3] = ["ui.ask_user_question", "vibe.todo", "search_tool_functions"];

/// The agent of Vibe for its ACP mode `agent` (the manifest's): with a fence, `auto-approve` stays in `ask`, whose
/// asks Escouade's rules accept but those naming its paths. Its `auto-approve` lets every call through, the `denylist`
/// of its file tools too (read in Vibe 3.0.3, `ToolGate.BYPASS`, and tried: a file of the fence read).
pub fn agent_of(agent: &str, fenced: bool) -> String {
    if agent == "auto-approve" && fenced {
        "ask".into()
    } else {
        agent.into()
    }
}

/// The MCP server and tool of Vibe's name for it (`mcp_escouade.get_board`).
fn mcp_tool(name: &str) -> Option<(&str, &str)> {
    name.strip_prefix("mcp_")?.split_once('.')
}

/// Vibe's call `u` (a `tool_call` or `tool_call_update`) in the form Cursor's reads (`cursor::tool_of`), in the agent's
/// folder `cwd`; None for a tool shown otherwise.
pub fn normalized(u: &Value, cwd: &str) -> Option<Value> {
    let tool = u["_meta"]["tool_name"].as_str().unwrap_or_default();
    if UNSHOWN.contains(&tool) {
        return None;
    }
    let mut n = u.clone();
    let raw = &u["rawInput"];
    if tool == "agents.spawn" {
        n["kind"] = json!("other");
        if raw.is_object() {
            n["rawInput"] = json!({
                "_toolName": "task", "description": raw["task"], "prompt": raw["task"],
                "subagentType": raw["agent"],
            });
        }
    } else if let Some((server, name)) = mcp_tool(tool) {
        if raw.is_object() {
            n["rawInput"] =
                json!({ "providerIdentifier": server, "toolName": name, "args": raw.clone() });
        }
    } else if !tool.is_empty()
        && !matches!(
            u["kind"].as_str(),
            Some("execute" | "edit" | "read" | "search" | "fetch" | "delete" | "think")
        )
        && raw.is_object()
    {
        // Its own tool (`skill`, `run_typescript`, `cron`…), shown by its name.
        n["rawInput"]["_toolName"] = json!(tool);
    }
    // Its files named from the agent's folder.
    if let Some(content) = n["content"].as_array_mut() {
        for c in content.iter_mut().filter(|c| c["type"] == "diff") {
            if let Some(p) = c["path"].as_str() {
                c["path"] = json!(crate::tools::absolute(p, cwd));
            }
        }
    }
    // A command's code (`returncode`), as Cursor's (`exitCode`).
    if let Some(code) = n["rawOutput"]["returncode"].as_i64() {
        n["rawOutput"]["exitCode"] = json!(code);
    }
    Some(n)
}

/// The first diff of a call's `content`.
pub fn diff_in(content: &Value) -> Option<&Value> {
    content.as_array()?.iter().find(|c| c["type"] == "diff")
}

/// `call` with what `update` tells of it (its keys over the call's).
pub fn merged(call: &mut Value, update: &Value) {
    if !call.is_object() {
        *call = json!({});
    }
    for (k, v) in update.as_object().into_iter().flatten() {
        if !v.is_null() && k != "sessionUpdate" {
            call[k] = v.clone();
        }
    }
}

/// The value `id` of the options `options` (`configOptions` of `session/new`) is set to.
pub fn option_value<'a>(options: &'a Value, id: &str) -> Option<&'a str> {
    options
        .as_array()?
        .iter()
        .find(|o| o["id"] == id)?
        .get("currentValue")?
        .as_str()
}

/// The values the option `id` offers.
pub fn option_values(options: &Value, id: &str) -> Vec<String> {
    options
        .as_array()
        .into_iter()
        .flatten()
        .find(|o| o["id"] == id)
        .and_then(|o| o["options"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v["value"].as_str().map(str::to_string))
        .collect()
}

/// The thinking level for Escouade's `effort` among `levels` (`thinking`: `off`, `high`): `low` is `off`, any other
/// its highest; None without an effort or levels.
pub fn thinking_of(effort: &str, levels: &[String]) -> Option<String> {
    if effort.is_empty() || levels.is_empty() {
        return None;
    }
    match effort {
        "low" => levels.iter().find(|l| *l == "off").cloned(),
        _ => levels.iter().rev().find(|l| *l != "off").cloned(),
    }
}

/// The questions of an `elicitation/create` (its `requestedSchema`), as the card shows them: each by its key (`q0`…),
/// its text, its choices, and whether it takes several.
pub fn elicitation_questions(params: &Value) -> Vec<(String, Value)> {
    let schema = &params["requestedSchema"];
    let mut keys: Vec<(&String, &Value)> = schema["properties"]
        .as_object()
        .into_iter()
        .flatten()
        .collect();
    // `q0`, `q1`… in their order, `q10` after `q9`.
    keys.sort_by_key(|(k, _)| {
        (
            k.trim_start_matches('q').parse::<u32>().unwrap_or(u32::MAX),
            k.to_string(),
        )
    });
    keys.into_iter()
        .map(|(key, p)| {
            let multi = p["type"] == "array";
            let choices = if multi {
                &p["items"]["anyOf"]
            } else {
                &p["oneOf"]
            };
            let options: Vec<Value> = choices
                .as_array()
                .into_iter()
                .flatten()
                .map(|c| {
                    let label = c["title"].as_str().or(c["const"].as_str()).unwrap_or("");
                    json!({ "label": label, "description": "" })
                })
                .collect();
            let text = p["description"]
                .as_str()
                .or(p["title"].as_str())
                .or(params["message"].as_str())
                .unwrap_or_default();
            (
                key.clone(),
                json!({
                    "question": text, "header": "", "multiSelect": multi, "options": options,
                }),
            )
        })
        .collect()
}

/// The `content` of the answer to an `elicitation/create` from a card's `answers` (by question, several choices joined
/// by « , »): each by its key, a choice by its value (`const`), several as a list; a written answer as it is (Vibe takes
/// it, tried).
pub fn elicitation_content(params: &Value, answers: &Value) -> Value {
    let props = &params["requestedSchema"]["properties"];
    let mut content = serde_json::Map::new();
    for (key, q) in elicitation_questions(params) {
        let text = q["question"].as_str().unwrap_or_default();
        let answer = answers[text]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string();
        let p = &props[&key];
        let multi = p["type"] == "array";
        let choices = if multi {
            &p["items"]["anyOf"]
        } else {
            &p["oneOf"]
        };
        let value_of = |label: &str| {
            choices
                .as_array()
                .into_iter()
                .flatten()
                .find(|c| c["title"].as_str() == Some(label) || c["const"].as_str() == Some(label))
                .and_then(|c| c["const"].as_str())
                .unwrap_or(label)
                .to_string()
        };
        let value = if multi {
            json!(answer
                .split(", ")
                .filter(|a| !a.is_empty())
                .map(value_of)
                .collect::<Vec<_>>())
        } else {
            json!(value_of(&answer))
        };
        content.insert(key, value);
    }
    Value::Object(content)
}

/// What a session of Vibe has spent so far (its `usage_update`, which a `session/load` sends again with what the
/// session spent before).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Spent {
    pub cost: f64,
    /// Its input tokens, those read from the cache included.
    pub prompt: u64,
    pub cached: u64,
    pub completion: u64,
}

impl Spent {
    /// What a `usage_update` says.
    pub fn of(u: &Value) -> Spent {
        let n = |k: &str| u["_meta"][k].as_u64().unwrap_or(0);
        Spent {
            cost: u["cost"]["amount"].as_f64().unwrap_or(0.0),
            prompt: n("promptTokens"),
            cached: n("cachedTokens"),
            completion: n("completionTokens"),
        }
    }

    /// What was spent from `before` to `self` (none of what a new session counts again).
    pub fn since(self, before: Spent) -> Spent {
        Spent {
            cost: (self.cost - before.cost).max(0.0),
            prompt: self.prompt.saturating_sub(before.prompt),
            cached: self.cached.saturating_sub(before.cached),
            completion: self.completion.saturating_sub(before.completion),
        }
    }
}

/// Whether a refusal's message says something of the user's own: not Escouade's words for a refusal without one
/// (« Refusé par … via Escouade. »), which would start a turn nobody asked for.
pub fn worded(message: &str) -> bool {
    let m = message.trim();
    !m.is_empty()
        && !REFUSED.contains(&m)
        && !m.ends_with("via Escouade.")
        && !m.ends_with("through Escouade.")
}

/// Escouade's words for a refusal without one, in its languages (the core's `agent::refused_action`).
const REFUSED: [&str; 2] = [
    "L'utilisateur a refusé cette action.",
    "The user refused this action.",
];

/// What the card of a turn stopped by Vibe's rate limit says (`said`: Vibe's own words).
pub fn rate_limited(lang: &str, said: Option<&str>) -> String {
    let said = said.unwrap_or("Rate limit exceeded");
    tr(
        lang,
        format!("Limite de débit de Mistral atteinte : {said}"),
        format!("Mistral’s rate limit reached: {said}"),
    )
}

/// No session opened for the agent: what Vibe said (no API key, say).
pub fn session_refused(lang: &str, e: &str) -> String {
    tr(
        lang,
        format!("Mistral Vibe n’a pas ouvert la session de l’agent : {e}. S’il n’a pas de clé d’API, lance vibe --setup dans un terminal."),
        format!("Mistral Vibe didn’t open the agent’s session: {e}. If it has no API key, run vibe --setup in a terminal."),
    )
}

/// Refused: the ACP of the process is not the one Escouade speaks.
pub fn protocol_refused(lang: &str, version: &str) -> String {
    let ours = crate::PROTOCOL;
    tr(
        lang,
        format!("Mistral Vibe parle ACP {version}, Escouade ACP {ours} : mets Mistral Vibe à jour (uv tool upgrade mistral-vibe)."),
        format!("Mistral Vibe speaks ACP {version}, Escouade ACP {ours}: update Mistral Vibe (uv tool upgrade mistral-vibe)."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_is_answered_by_the_keys_and_values_of_its_form() {
        let single = json!({ "requestedSchema": { "properties": {
            "q0": { "description": "blue or red?", "oneOf": [ { "const": "blue", "title": "blue" }, { "const": "red", "title": "red" } ] },
        } } });
        let qs = elicitation_questions(&single);
        assert_eq!(qs[0].0, "q0");
        assert_eq!(qs[0].1["options"][1]["label"], "red");
        assert_eq!(
            elicitation_content(&single, &json!({ "blue or red?": "red" })),
            json!({ "q0": "red" })
        );
        assert_eq!(
            elicitation_content(&single, &json!({ "blue or red?": "vert" })),
            json!({ "q0": "vert" })
        );
        let multi = json!({ "requestedSchema": { "properties": {
            "q0": { "description": "Fruits ?", "type": "array",
                    "items": { "anyOf": [ { "const": "p", "title": "Pomme" }, { "const": "o", "title": "Orange" } ] } },
        } } });
        assert_eq!(elicitation_questions(&multi)[0].1["multiSelect"], true);
        assert_eq!(
            elicitation_content(&multi, &json!({ "Fruits ?": "Pomme, Orange" })),
            json!({ "q0": ["p", "o"] })
        );
    }

    #[test]
    fn the_thinking_of_an_effort_and_the_agent_of_a_mode() {
        let levels = vec!["off".to_string(), "high".to_string()];
        assert_eq!(thinking_of("low", &levels).as_deref(), Some("off"));
        assert_eq!(thinking_of("medium", &levels).as_deref(), Some("high"));
        assert_eq!(thinking_of("", &levels), None);
        assert_eq!(agent_of("auto-approve", true), "ask");
        assert_eq!(agent_of("auto-approve", false), "auto-approve");
        assert_eq!(agent_of("accept-edits", true), "accept-edits");
    }

    #[test]
    fn its_calls_are_put_in_cursors_form() {
        let spawn = normalized(
            &json!({ "_meta": { "tool_name": "agents.spawn" }, "rawInput": { "task": "t", "agent": "explore" } }),
            "C:\\repo",
        )
        .unwrap();
        assert_eq!(spawn["rawInput"]["_toolName"], "task");
        assert_eq!(spawn["rawInput"]["subagentType"], "explore");
        for tool in UNSHOWN {
            assert!(normalized(&json!({ "_meta": { "tool_name": tool } }), "C:\\repo").is_none());
        }
        let w = normalized(
            &json!({ "kind": "edit", "_meta": { "tool_name": "write_file" },
                     "content": [{ "type": "diff", "path": "b.txt", "newText": "x" }],
                     "rawOutput": { "returncode": 1 } }),
            "C:\\repo",
        )
        .unwrap();
        assert_eq!(w["content"][0]["path"], "C:\\repo\\b.txt");
        assert_eq!(w["rawOutput"]["exitCode"], 1);
        assert!(!worded("Refusé par Guillaume via Escouade."));
        assert!(!worded("L'utilisateur a refusé cette action."));
        assert!(worded("Pas ce fichier"));
    }
}
