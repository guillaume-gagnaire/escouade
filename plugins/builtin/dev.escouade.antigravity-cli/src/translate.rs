//! Antigravity CLI's events (`agy -p`, stream-json) as the items of the conversation. `agy` tells its turn step by step
//! (`step_update`: `user_input`, `agent_response` with the deltas of its text, `tool` and `subagent` with their call, each
//! `ACTIVE` then `DONE` or `ERROR`). They are put in ACP's form and read by the ACP client with Cursor's extensions
//! (`escouade_acp::Agent`, extension `agy`):
//! - its text is `agent_message_chunk`, its reasoning (read by the client in its transcript) `agent_thought_chunk`;
//! - a call is `tool_call` then `tool_call_update`, named by its step (`words::call_id`), its kind and input as
//!   `words::acp_call` says, a write's diff from its arguments;
//! - the permissions and questions its client asks once the turn is over, and the plan of the mode `plan`, are requests of
//!   the same form;
//! - its tokens are its steps' (`usage`), told with the end of the turn; it tells no cost.
//!
//! A recorded stream (`tests/fixtures/agy/`, the events as `agy` wrote them) is read as it came (`recorded`): its `init`
//! opens the session, its `user_input` the turn, its `result` ends it. The core reads them so, as a library.

use crate::words::{self, Usage};
use escouade_acp::limits::limit_of;
use escouade_acp::Agent;
use escouade_plugin_protocol::engine::{
    AcpDecl, Attach, HarnessDecl, Item, LaunchParams, TurnUsage,
};
use serde_json::{json, Value};
use std::collections::HashSet;

/// How the ACP client reads what is put in its form: Cursor's requests, no file through it (the fence is in its rules).
fn decl() -> HarnessDecl {
    HarnessDecl {
        acp: Some(AcpDecl {
            fs: false,
            fence_on_permissions: true,
            read_only_mode: Some("ask".into()),
            extensions: vec!["agy".into()],
            ..AcpDecl::default()
        }),
        ..serde_json::from_value(
            json!({ "id": "agy", "name": "Antigravity CLI", "speaker": "Antigravity" }),
        )
        .expect("a harness")
    }
}

/// The time (ms since the epoch).
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

/// What a call of `kind` said, in Cursor's form: a command's output and code (`Command exited with code 0.\n\nCommand
/// output:\n…`), its refusal on its error.
pub fn output(kind: &Value, said: &str, failed: bool) -> Value {
    if kind != "execute" {
        return json!({ "content": said });
    }
    if failed {
        return json!({ "stderr": said, "content": said });
    }
    // In its interface's transcript, after the times it ran (`Created At: …`).
    let code = said
        .split_once("Command exited with code ")
        .and_then(|(_, r)| r.split(['.', ' ']).next())
        .and_then(|c| c.parse::<i64>().ok());
    let text = said
        .split_once("Command output:\n")
        .map_or(said, |(_, o)| o)
        .trim_end();
    json!({ "stdout": text, "content": text, "exitCode": code })
}

/// The translation of one agent.
pub struct Translator {
    acp: Agent,
    lang: String,
    session: Option<String>,
    /// The calls told so far, by their ids.
    told: HashSet<String>,
    /// What the turn spent so far (a turn that asked the user goes on after its answers).
    spent: Usage,
}

impl Translator {
    pub fn new(a: &Attach) -> Self {
        Translator {
            acp: Agent::new(a, &decl(), env!("CARGO_PKG_VERSION")),
            lang: a.lang.clone(),
            session: a.session.clone(),
            told: HashSet::new(),
            spent: Usage::default(),
        }
    }

    /// What the launch says of the agent: its fence, read-only.
    pub fn launch(&mut self, p: &LaunchParams) {
        self.lang = p.lang.clone();
        // Its lines are the harness's, which speaks no ACP.
        let _ = self.acp.launch(p);
    }

    pub fn set_mode(&mut self, mode: &str) {
        let _ = self.acp.set_mode(mode);
    }

    pub fn set_model(&mut self, model: &str) {
        let _ = self.acp.set_model(model);
    }

    /// The conversation is open: the agent's from now on.
    pub fn open(&mut self, session: &str) -> Vec<Item> {
        self.session = Some(session.to_string());
        translated(self.acp.session_opened(session))
    }

    /// A turn begins.
    pub fn begin(&mut self) {
        self.acp.prompt_begun();
    }

    /// Tokens of the turn.
    pub fn spend(&mut self, u: Usage) {
        self.spent.input += u.input;
        self.spent.output += u.output;
        self.spent.cached += u.cached;
    }

    /// The end of a prompt: `error` is `agy`'s own words; its usage limit ends the turn « limité », its resume planned (or
    /// another account, ESC-271). Its tokens for its card and the stats (no cost: `agy` tells none).
    pub fn end(&mut self, cancelled: bool, error: Option<&str>) -> Vec<Item> {
        let s = std::mem::take(&mut self.spent);
        let usage = TurnUsage {
            input: s.input.saturating_sub(s.cached),
            output: s.output,
            cache: s.cached,
            cost: 0.0,
        };
        let wait = error.and_then(limit_of);
        let limited = wait.map(|w| now_ms() + w);
        let error = match (wait, error) {
            (Some(_), Some(e)) => Some(words::limited_error(&self.lang, e.trim())),
            (None, Some(e)) => Some(e.to_string()),
            _ => None,
        };
        translated(self.acp.prompt_ended(cancelled, usage, error, limited))
    }

    /// A request to the user, as ACP has it; with what the rules answered without a card, if they did.
    pub fn ask(&mut self, method: &str, id: &str, params: Value) -> (Vec<Item>, Option<bool>) {
        let items = self
            .acp
            .heard(json!({ "method": method, "id": id, "params": params }));
        let mut allowed = None;
        for i in &items {
            if let Item::Write { line } = i {
                let answer: Value = serde_json::from_str(line).unwrap_or_default();
                let option = answer["result"]["outcome"]["optionId"].as_str();
                if answer["id"] == id {
                    allowed = Some(option.is_some_and(|o| o.starts_with("allow")));
                }
            }
        }
        (translated(items), allowed)
    }

    /// The plan of a turn in the mode Plan to approve: it is whole, what comes once it is approved is another text.
    pub fn plan(&mut self, id: &str, plan: &str) -> Vec<Item> {
        let mut out = translated(self.acp.text_whole());
        let frame = json!({ "method": "cursor/create_plan", "id": id, "params": { "plan": plan } });
        out.extend(translated(self.acp.heard(frame)));
        out
    }

    /// `request`, the card of the client, is answered.
    pub fn answered(&mut self, request: &str) {
        self.acp.forget(request);
    }

    /// One step of the turn (`step_update`): `escouade`, what its client read of it in its transcript.
    pub fn step(&mut self, s: &Value) -> Vec<Item> {
        let Some(session) = self
            .session
            .clone()
            .or_else(|| s["conversation_id"].as_str().map(str::to_string))
        else {
            return Vec::new();
        };
        let update = |u: Value| json!({ "method": "session/update", "params": { "sessionId": session, "update": u } });
        let state = s["state"].as_str().unwrap_or_default();
        let mut out = Vec::new();
        match s["step_type"].as_str().unwrap_or_default() {
            // A turn under way (one a recorded stream tells, or that started without the core).
            "user_input" => {
                self.acp.prompt_begun();
                out.push(Item::Start {
                    session: Some(session.clone()),
                });
            }
            "agent_response" => {
                if let Some(th) = s["escouade"]["thinking"].as_str() {
                    let chunk = json!({ "sessionUpdate": "agent_thought_chunk", "content": { "type": "text", "text": th } });
                    out.extend(self.acp.heard(update(chunk)));
                    out.extend(self.acp.text_whole());
                }
                let delta = s["text_delta"].as_str().unwrap_or_default();
                // The `DONE` of a text ends it with a line break alone.
                if !(delta.is_empty() || (state == "DONE" && delta.trim().is_empty())) {
                    let chunk = json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": delta } });
                    out.extend(self.acp.heard(update(chunk)));
                }
                // Each step's text an item of its own.
                if state == "DONE" {
                    out.extend(self.acp.text_whole());
                }
            }
            "tool" | "subagent" => {
                let Some(step) = s["step_index"].as_u64() else {
                    return Vec::new();
                };
                let id = words::call_id(&session, step);
                let name = s["tool_name"].as_str().unwrap_or_default();
                let args = match &s["escouade"]["args"] {
                    a if a.is_object() => a.clone(),
                    _ => s["tool_info"]["parameters"].clone(),
                };
                let mut call = words::acp_call(name, &args);
                call["toolCallId"] = json!(id);
                if self.told.insert(id) {
                    let mut start = call.clone();
                    start["sessionUpdate"] = json!("tool_call");
                    start["status"] = json!("pending");
                    if let Some(o) = start.as_object_mut() {
                        o.remove("content");
                    }
                    out.extend(self.acp.heard(update(start)));
                }
                if state == "ACTIVE" {
                    return translated(out);
                }
                let failed = state == "ERROR";
                let info = &s["tool_info"];
                let said = if failed {
                    info["error"]["message"]
                        .as_str()
                        .unwrap_or("error")
                        .to_string()
                } else if let Some(sub) = s["subagent_info"]["subagents"].as_array() {
                    sub.iter()
                        .filter_map(|a| a["initial_prompt"].as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    info["output"].as_str().unwrap_or_default().to_string()
                };
                let mut end = call;
                end["sessionUpdate"] = json!("tool_call_update");
                end["status"] = json!(if failed { "failed" } else { "completed" });
                end["rawOutput"] = output(&end["kind"], &said, failed);
                // An end that tells no call (a subagent's): its start's input kept.
                if !args.is_object() {
                    if let Some(o) = end.as_object_mut() {
                        o.remove("rawInput");
                    }
                }
                // A write whose text `agy` did not tell (a recorded stream): written, its diff unknown.
                if !failed && end["kind"] == "edit" && words::diff_of(name, &args).is_none() {
                    if let Some(path) = words::path_of(&args) {
                        end["content"] =
                            json!([{ "type": "diff", "path": path, "oldText": "", "newText": "" }]);
                    }
                }
                out.extend(self.acp.heard(update(end)));
            }
            _ => {}
        }
        translated(out)
    }

    /// One event of a recorded stream.
    pub fn recorded(&mut self, f: &Value) -> Vec<Item> {
        match f["event"].as_str().unwrap_or_default() {
            "init" => {
                let id = f["conversation_id"].as_str().unwrap_or_default();
                self.open(id)
            }
            "step_update" => self.step(&f["step_update"]),
            "result" => {
                let r = &f["result"];
                let u = &r["usage"];
                self.spent.input += u["input_tokens"].as_u64().unwrap_or(0);
                self.spent.cached += u["cache_read_tokens"].as_u64().unwrap_or(0);
                self.spent.output += u["output_tokens"].as_u64().unwrap_or(0)
                    + u["thinking_tokens"].as_u64().unwrap_or(0);
                let error =
                    (r["status"] == "ERROR").then(|| r["error"].as_str().unwrap_or("error"));
                self.end(false, error)
            }
            _ => Vec::new(),
        }
    }
}

/// The items of the client, but its lines: `agy` speaks no ACP.
fn translated(items: Vec<Item>) -> Vec<Item> {
    items
        .into_iter()
        .filter(|i| !matches!(i, Item::Write { .. }))
        .collect()
}
