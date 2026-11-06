//! Gemini CLI's ways beside ACP (the extension `gemini` of a manifest, INT-10.7), as the core's own client of Gemini CLI
//! 0.66.0 drove it (ESC-273 to 275, its streams in `tests/fixtures/gemini/`):
//! - the updates of its session count while a prompt runs only: `session/load` replays the history after its answer too;
//! - its calls tell no input (`rawInput`): their tool is the start of their id (`run_shell_command__…`), a command is their
//!   title, a file their `locations` or diff, a tool of an MCP server their title (« get_board (escouade MCP Server) »), a
//!   subagent their title (« Delegating to agent 'generalist' », a `think` that would be left out);
//! - its files are named from the agent's folder in a replay: made absolute;
//! - its change of mode is a text (« [MODE_UPDATE] default »): read as such, left out of the thread;
//! - the permission of its `exit_plan_mode` is the card « Plan à approuver », the plan read from the file it names;
//!   approved, Gemini goes back to its `default`, the mode the plan goes on in set once it has;
//! - a refusal of the user's cuts the turn (`session/cancel`), its word sent after;
//! - no questions in ACP: the turn that ends on a block `<escouade_question>` is held for its card's answers, which go on
//!   in the same turn;
//! - its tokens come with the end of its prompt (`usage`, a turn with a question adding them up), no cost; its usage limit
//!   ends the turn « limité », its resume planned when it says, else a minute (an hour for its day).
//!
//! No turn with a real model was ever run: what an account would change is said in `docs/TODO-1.8.md`, point 0.

use crate::texts::tr;
use escouade_plugin_protocol::engine::TurnUsage;
use serde_json::{json, Value};

/// How Gemini says it changed its mode: a text of its own (« [MODE_UPDATE] default »).
pub const MODE_UPDATE: &str = "[MODE_UPDATE] ";

/// How the permission of its `exit_plan_mode` names the plan to approve.
pub const PLAN_APPROVAL: &str = "Requesting plan approval for: ";

/// The permission of its `exit_plan_mode` waiting for its card, among what the agent asked (`Agent`).
pub const PLAN: &str = "gemini/plan";

/// How long after a limit by the minute (a 429, its model's capacity) its agent goes on, when Gemini tells no wait.
pub const RATE_RESUME_MS: i64 = 60_000;

/// How long after its daily quota was met its agent tries again: Gemini tells no reset (its quotas are by the day), and a
/// turn tried for nothing costs none.
pub const DAILY_RESUME_MS: i64 = 3_600_000;

/// The tools of Gemini shown otherwise than as calls (or not at all): its topic of the turn, the end of its mode Plan (its
/// card).
const UNSHOWN: [&str; 2] = ["update_topic", "exit_plan_mode"];

/// Escouade's mode for Gemini's `mode`; None for one it does not know.
pub fn escouade_mode(mode: &str) -> Option<&'static str> {
    match mode {
        "plan" => Some("plan"),
        "autoEdit" => Some("acceptEdits"),
        "yolo" => Some("bypassPermissions"),
        "default" => Some("default"),
        _ => None,
    }
}

/// Escouade's mode of a text of Gemini that tells its change of mode (« [MODE_UPDATE] plan »); None for any other text.
pub fn mode_update(text: &str) -> Option<&'static str> {
    escouade_mode(text.strip_prefix(MODE_UPDATE)?.trim())
}

/// Whether a call (its `toolCallId`) is Gemini's `exit_plan_mode`: the plan to approve.
pub fn is_plan_exit(call: &str) -> bool {
    call.starts_with("exit_plan_mode")
}

/// The file of the plan its `exit_plan_mode` asks to approve, from the call's title.
pub fn plan_file(title: &str) -> Option<&str> {
    title
        .strip_prefix(PLAN_APPROVAL)
        .map(str::trim)
        .filter(|p| !p.is_empty())
}

/// The plan of the permission `params` of its `exit_plan_mode`: the text of the file it names, else its name.
pub fn plan_of(params: &Value) -> String {
    plan_of_agent(params, "")
}

/// `plan_of` for the agent `agent`, as the host of a module wants it (`plugin::agent_file`).
pub fn plan_of_agent(params: &Value, agent: &str) -> String {
    let file = params["toolCall"]["title"].as_str().and_then(plan_file);
    let plan = file
        .and_then(|f| crate::plugin::agent_file(agent, std::path::Path::new(f)))
        .unwrap_or_else(|| file.unwrap_or_default().to_string());
    plan.trim().to_string()
}

/// The ids of the models a session offers (`models` of `session/new`: `availableModels`).
pub fn models_of(models: &Value) -> Vec<String> {
    models["availableModels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["modelId"].as_str().map(str::to_string))
        .collect()
}

/// What the end of a prompt says of its tokens (`usage`), as a turn counts them: the cached ones apart from the input, the
/// model's thoughts in its output. No cost: Gemini tells none.
pub fn spent(usage: &Value) -> TurnUsage {
    let n = |k: &str| usage[k].as_u64().unwrap_or(0);
    let cached = n("cachedReadTokens");
    TurnUsage {
        input: n("inputTokens").saturating_sub(cached),
        output: n("outputTokens") + n("thoughtTokens"),
        cache: cached,
        cost: 0.0,
    }
}

/// The usage limit an error of Gemini tells (its words read in its code, 0.66.0: a 429, « You have exhausted your daily
/// quota on this model. », « Please retry in 30.5s », its model's capacity): in how many ms to try again, the wait it tells
/// else a minute (an hour for its daily quota). None for another error.
pub fn limit_of(code: Option<i64>, error: &str) -> Option<i64> {
    let lower = error.to_lowercase();
    let daily = lower.contains("daily quota") || lower.contains("perday");
    let limited = daily
        || code == Some(429)
        || lower.contains("usage limit")
        || crate::limits::limit_of(error).is_some();
    if !limited {
        return None;
    }
    let told = lower.split_once("retry in ").and_then(|(_, rest)| {
        let digits: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let n: f64 = digits.parse().ok()?;
        let unit = rest[digits.len()..].trim_start();
        let ms = if unit.starts_with("ms") {
            n
        } else {
            n * 1000.0
        };
        (ms > 0.0).then_some(ms.ceil() as i64)
    });
    Some(told.unwrap_or(if daily {
        DAILY_RESUME_MS
    } else {
        RATE_RESUME_MS
    }))
}

/// The tool of a call of Gemini, the start of its id (`run_shell_command__run_shell_command_…`).
fn tool_name(id: &str) -> &str {
    id.split_once("__").map_or("", |(name, _)| name)
}

/// The tool and server of an MCP call's title (« get_board (escouade MCP Server) »).
fn mcp_title(title: &str) -> Option<(&str, &str)> {
    let (tool, rest) = title.split_once(" (")?;
    let server = rest.strip_suffix(" MCP Server)")?;
    Some((tool.trim(), server.trim()))
}

/// The subagent of a call's title (« Delegating to agent 'generalist' »).
fn subagent_title(title: &str) -> Option<&str> {
    title
        .strip_prefix("Delegating to agent ")
        .map(|a| a.trim().trim_matches(['\'', '"']))
}

/// Gemini's call `u` (a `tool_call` or `tool_call_update`) in the form Cursor's reads (`cursor::tool_of`), in the agent's
/// folder `cwd`; None for a tool shown otherwise.
pub fn normalized(u: &Value, cwd: &str) -> Option<Value> {
    let id = u["toolCallId"].as_str().unwrap_or_default();
    let tool = tool_name(id);
    if UNSHOWN.contains(&tool) {
        return None;
    }
    let mut n = u.clone();
    let title = u["title"].as_str().unwrap_or_default();
    if n["rawInput"].is_null() {
        if tool.starts_with("mcp_") {
            if let Some((name, server)) = mcp_title(title) {
                n["rawInput"] =
                    json!({ "providerIdentifier": server, "toolName": name, "args": {} });
            }
        } else if tool == "invoke_agent" || u["kind"] == "think" {
            // A subagent: Cursor's `think` is left out of the thread, its `task` is not.
            n["kind"] = json!("other");
            if !title.is_empty() {
                let agent = subagent_title(title).unwrap_or(title);
                n["rawInput"] = json!({
                    "_toolName": "task", "description": agent, "prompt": title, "subagentType": agent,
                });
            }
        } else if !tool.is_empty()
            && !matches!(
                u["kind"].as_str(),
                Some("execute" | "edit" | "read" | "search" | "fetch" | "delete")
            )
        {
            // Its own tool (`enter_plan_mode`, `activate_skill`…), shown by its name.
            if u.get("kind").is_some() || u["sessionUpdate"] == "tool_call" {
                n["rawInput"] = json!({ "_toolName": tool, "reason": title });
            }
        }
    } else if n["kind"] == "think" && tool == "invoke_agent" {
        n["kind"] = json!("other");
    }
    // Its files named from the agent's folder.
    if let Some(content) = n["content"].as_array_mut() {
        for c in content.iter_mut().filter(|c| c["type"] == "diff") {
            if let Some(p) = c["path"].as_str() {
                c["path"] = json!(crate::tools::absolute(p, cwd));
            }
        }
    }
    if let Some(list) = n["locations"].as_array_mut() {
        for l in list.iter_mut() {
            if let Some(p) = l["path"].as_str() {
                l["path"] = json!(crate::tools::absolute(p, cwd));
            }
        }
    }
    Some(n)
}

/// What the card of a turn stopped by Gemini's rate limit says (`said`: Gemini's own words).
pub fn rate_limited(lang: &str, said: &str) -> String {
    tr(
        lang,
        format!("Limite d’usage de Gemini atteinte : {said}"),
        format!("Gemini’s usage limit reached: {said}"),
    )
}

/// No session opened for the agent: what Gemini said (no account, say).
pub fn session_refused(lang: &str, e: &str) -> String {
    tr(
        lang,
        format!("Gemini CLI n’a pas ouvert la session de l’agent : {e}. S’il n’est pas connecté, lance gemini dans un terminal pour te connecter à ton compte Google, ou donne-lui une clé d’API par GEMINI_API_KEY."),
        format!("Gemini CLI didn’t open the agent’s session: {e}. If it isn’t signed in, run gemini in a terminal to sign in to your Google account, or give it an API key by GEMINI_API_KEY."),
    )
}

/// Refused: the ACP of the process is not the one Escouade speaks.
pub fn protocol_refused(lang: &str, version: &str) -> String {
    let ours = crate::PROTOCOL;
    tr(
        lang,
        format!("Gemini CLI parle ACP {version}, Escouade ACP {ours} : mets Gemini CLI à jour (npm install -g @google/gemini-cli)."),
        format!("Gemini CLI speaks ACP {version}, Escouade ACP {ours}: update Gemini CLI (npm install -g @google/gemini-cli)."),
    )
}

#[cfg(test)]
#[path = "gemini_tests.rs"]
mod tests;
