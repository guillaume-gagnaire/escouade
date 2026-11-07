//! The terminal views of the harnesses on this client (`view/translate`, docs/PLUGINS.md §8): what Cursor CLI, Mistral Vibe and
//! Gemini CLI say by their own interface, read by the core a line at a time and translated here. Each line is put in ACP's
//! form and read by the client ([`Speaker`], which Antigravity CLI's view shares), or told as it is: a row that ended
//! already, the user's message, what waits for the user.
//!
//! - **Cursor**: its transcript (`~/.cursor/projects/<folder>/agent-transcripts/<id>/<id>.jsonl`). A line is a message
//!   (`{role, message: {content}}`), the user's (its text wrapped in `<timestamp>` and `<user_query>`) or the agent's (its
//!   texts, and its tools as `tool_use`, their results never written), or the end of a turn (`{type: "turn_ended", status}`:
//!   `success`, `aborted` when stopped by Ctrl+C, `error`). Nothing says that a permission or a question waits for the user,
//!   nor tokens.
//! - **Vibe**: the journal of its session's store (`logs/session/unified/<id>/journal/<sequence>.jsonl`). A line is a record;
//!   those read are its `projection_delta`, the changes of the session's public state (`append_entry`, `replace_entry`,
//!   `set_envelope`), and its `callback_registered` / `callback_resolved`.
//! - **Gemini**: its hooks, which the core hands on (a note `hook`): its log, which writes a message anew each time it
//!   changes, is not read.

use crate::tools::absolute;
use crate::Agent;
use escouade_plugin_protocol::engine::{AcpDecl, Attach, HarnessDecl, Item, TurnUsage, ViewParams};
use serde_json::{json, Value};

/// The session the updates of a view are in: only the client's to read.
const SESSION: &str = "view";

/// The client, reading what a view puts in ACP's form as a process's. Its own translation of a turn, its rows and its end;
/// its session and its lines are nothing to the core here.
pub struct Speaker {
    acp: Agent,
}

/// How the client reads a view: `extension` as a harness of the kind says it, no file through it.
pub fn decl(extension: &str, speaker: &str) -> HarnessDecl {
    HarnessDecl {
        acp: Some(AcpDecl {
            fs: false,
            extensions: vec![extension.to_string()],
            ..AcpDecl::default()
        }),
        ..serde_json::from_value(json!({ "id": "view", "name": speaker, "speaker": speaker }))
            .expect("a harness")
    }
}

impl Speaker {
    pub fn new(p: &ViewParams, decl: &HarnessDecl) -> Self {
        let a = Attach {
            agent: p.agent.clone(),
            cwd: p.cwd.clone(),
            model: p.model.clone(),
            mode: p.mode.clone(),
            lang: p.lang.clone(),
            ..Attach::default()
        };
        let mut acp = Agent::new(&a, decl, "");
        let _ = acp.session_opened(SESSION);
        Speaker { acp }
    }

    /// What is the conversation's of what the client says.
    fn shown(items: Vec<Item>) -> Vec<Item> {
        items
            .into_iter()
            .filter(|i| {
                !matches!(
                    i,
                    Item::Write { .. } | Item::Start { .. } | Item::Session { .. }
                )
            })
            .collect()
    }

    /// A turn begins.
    pub fn begin(&mut self) {
        self.acp.prompt_begun();
    }

    /// An update of its session, in ACP's form.
    pub fn update(&mut self, u: Value) -> Vec<Item> {
        let frame =
            json!({ "method": "session/update", "params": { "sessionId": SESSION, "update": u } });
        Self::shown(self.acp.heard(frame))
    }

    /// A message of the harness in ACP's form, as it is: a recorded stream's.
    pub fn heard(&mut self, msg: Value) -> Vec<Item> {
        Self::shown(self.acp.heard(msg))
    }

    /// What it wrote or thought so far is whole.
    pub fn close(&mut self) -> Vec<Item> {
        Self::shown(self.acp.text_whole())
    }

    /// A whole text of the agent.
    pub fn say(&mut self, text: &str) -> Vec<Item> {
        let chunk = json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": text } });
        let mut out = self.update(chunk);
        out.extend(self.close());
        out
    }

    /// The turn is over.
    pub fn end(&mut self, cancelled: bool, error: Option<String>) -> Vec<Item> {
        Self::shown(
            self.acp
                .prompt_ended(cancelled, TurnUsage::default(), error, None),
        )
    }
}

/// What the note of the core `e` says of the end of a turn.
fn end_of(e: &Value) -> Option<String> {
    e["error"].as_str().map(str::to_string)
}

/// A view of a harness of this client.
pub struct View {
    of: Of,
    acp: Speaker,
    /// Gemini: each call started (`BeforeTool`), its id, tool and input, until its `AfterTool`.
    calls: Vec<(String, String, Value)>,
    next: u64,
    /// Vibe: the last turn of its journal ended, which a segment read again does not start again.
    ended: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Of {
    Cursor,
    Vibe,
    Gemini,
}

impl View {
    /// The view of the agent `p`; None for a harness that has none.
    pub fn new(p: &ViewParams, speaker: &str) -> Option<Self> {
        let (of, extension) = match p.harness.as_str() {
            "cursor" => (Of::Cursor, "cursor"),
            // What their interfaces say is no ACP: read as Cursor's, none of their process's rules applying.
            "vibe" => (Of::Vibe, "agy"),
            "gemini" => (Of::Gemini, "agy"),
            _ => return None,
        };
        Some(View {
            of,
            acp: Speaker::new(p, &decl(extension, speaker)),
            calls: Vec::new(),
            next: p.known.len() as u64,
            ended: None,
        })
    }

    /// One line of its log, or a note of the core.
    pub fn line(&mut self, line: &str, p: &ViewParams, out: &mut Vec<Item>) {
        let Ok(e) = serde_json::from_str::<Value>(line) else {
            return;
        };
        match e["escouade"].as_str() {
            Some("frame") => out.extend(self.acp.heard(e["frame"].clone())),
            Some("end") => {
                self.calls.clear();
                out.extend(self.acp.end(p.interrupted, end_of(&e)));
            }
            Some("hook") => self.gemini(&e["hook"], p, out),
            _ => match self.of {
                Of::Cursor => self.cursor(&e, p, out),
                Of::Vibe => self.vibe(&e, p, out),
                Of::Gemini => {}
            },
        }
    }

    /// The user's message: a turn starts.
    fn typed(&mut self, id: Option<String>, text: String, out: &mut Vec<Item>) {
        self.acp.begin();
        out.push(Item::Wrote);
        out.push(Item::Typed {
            id,
            text,
            images: 0,
            prompt: None,
        });
    }

    fn cursor(&mut self, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        // Written since it started: it is past its question about the folder.
        out.push(Item::Spoke);
        if e["type"] == "turn_ended" {
            if !p.active {
                return;
            }
            let (interrupted, error) = match e["status"].as_str() {
                Some("aborted") => (true, None),
                Some("success") | None => (false, None),
                Some(other) => (
                    false,
                    Some(e["error"].as_str().unwrap_or(other).to_string()),
                ),
            };
            return out.push(Item::ViewEnd {
                interrupted,
                duration: None,
                error,
            });
        }
        let blocks = e["message"]["content"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        match e["role"].as_str() {
            Some("user") => {
                let text: Vec<String> = blocks
                    .iter()
                    .filter_map(|b| b["text"].as_str())
                    .map(typed_text)
                    .filter(|t| !t.is_empty())
                    .collect();
                if !text.is_empty() {
                    self.typed(None, text.join("\n\n"), out);
                }
            }
            Some("assistant") => {
                out.push(Item::Runs);
                for b in blocks {
                    match b["type"].as_str() {
                        Some("text") => {
                            let text = b["text"].as_str().unwrap_or_default();
                            if !text.trim().is_empty() {
                                out.extend(self.acp.say(text));
                            }
                        }
                        Some("tool_use") => {
                            let (name, input) =
                                tui_tool(b["name"].as_str().unwrap_or("?"), &b["input"]);
                            // Its result is never written: shown as done, with nothing to show.
                            out.push(Item::Row {
                                id: None,
                                name,
                                input,
                                status: "ok".into(),
                            });
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn vibe(&mut self, record: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        // Written since it started: it is up.
        out.push(Item::Spoke);
        match record["type"].as_str() {
            Some("projection_delta") => {}
            // A permission or a question it waits on, answered in its interface.
            Some("callback_registered") => {
                return vibe_wait(&record["payload"]["routing"], &p.cwd, out);
            }
            Some("callback_resolved") => return out.push(Item::Goes),
            _ => return,
        }
        for d in record["payload"]["delta"].as_array().into_iter().flatten() {
            match d["op"].as_str() {
                Some("append_entry" | "replace_entry") => self.vibe_entry(&d["entry"], p, out),
                Some("set_envelope") => self.vibe_envelope(&d["state"], p, out),
                _ => {}
            }
        }
    }

    /// An entry of its history, added or replaced.
    fn vibe_entry(&mut self, e: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        let Some(id) = e["id"].as_str() else {
            return;
        };
        match (e["type"].as_str(), e["role"].as_str()) {
            (Some("message"), Some("user")) => {
                let text = content_text(e);
                if !text.trim().is_empty() {
                    self.typed(Some(id.to_string()), text, out);
                }
            }
            (Some("message"), Some("assistant")) => {
                let text = content_text(e);
                if text.trim().is_empty() || e["outcome"]["type"] == "discarded" {
                    return;
                }
                out.push(Item::Runs);
                out.extend(self.acp.say(&text));
            }
            (Some("effect"), _) if e["detail"]["kind"] == "tool" => {
                let detail = &e["detail"];
                let name = detail["toolName"].as_str().unwrap_or("?");
                let Some((name, input)) = journal_tool(name, &detail["input"], &p.cwd) else {
                    return;
                };
                let state = &e["state"];
                let done = e["generationStatus"] == "completed" || state["status"] == "completed";
                let failed = state["display"]["success"] == false
                    || matches!(
                        state["decision"].as_str(),
                        Some("deny" | "denied" | "reject")
                    );
                let status = match (done, failed) {
                    (false, _) => "running",
                    (true, false) => "ok",
                    (true, true) => "error",
                };
                out.extend(self.acp.close());
                out.push(Item::Row {
                    id: Some(id.to_string()),
                    name,
                    input,
                    status: status.into(),
                });
            }
            _ => {}
        }
    }

    /// The state of its session: what it waits on, and the end of its turn.
    fn vibe_envelope(&mut self, state: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        let open = state["activeCallbacks"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["state"]["status"].as_str().is_none_or(|s| s == "open"));
        if let Some(c) = open {
            return vibe_wait(c, &p.cwd, out);
        }
        out.push(Item::Goes);
        let turn = &state["latestTurn"];
        let idle = state["session"]["status"]["type"] == "idle";
        let id = turn["id"].as_str().map(str::to_string);
        // A turn already ended (a segment read again) does not start again.
        if id.is_some() && id == self.ended {
            return;
        }
        match turn["status"].as_str() {
            Some("in_progress") => out.push(Item::Runs),
            Some(end @ ("completed" | "interrupted" | "failed")) if idle => {
                if !p.active {
                    return;
                }
                let error = (end == "failed")
                    .then(|| turn["error"]["message"].as_str().unwrap_or(end).to_string());
                self.ended = id;
                out.push(Item::ViewEnd {
                    interrupted: end == "interrupted",
                    duration: None,
                    error,
                });
            }
            _ => {}
        }
    }

    /// One of the hooks of its Gemini interface.
    fn gemini(&mut self, h: &Value, p: &ViewParams, out: &mut Vec<Item>) {
        let event = h["escouade_hook_event"]
            .as_str()
            .or(h["hook_event_name"].as_str())
            .unwrap_or_default();
        match event {
            "BeforeAgent" => {
                self.calls.clear();
                let text = h["prompt"].as_str().unwrap_or_default().trim().to_string();
                if !text.is_empty() {
                    self.typed(None, text, out);
                }
            }
            "BeforeTool" => {
                out.push(Item::Runs);
                let name = h["tool_name"].as_str().unwrap_or_default().to_string();
                let input = h["tool_input"].clone();
                self.next += 1;
                let id = format!("gemini-tui-{}-{}", p.agent, self.next);
                let mut call = acp_call(&name, &input, &h["mcp_context"]);
                call["toolCallId"] = json!(id);
                call["sessionUpdate"] = json!("tool_call");
                call["status"] = json!("pending");
                out.extend(self.acp.close());
                out.extend(self.acp.update(call));
                self.calls.push((id, name, input));
            }
            "AfterTool" => {
                let name = h["tool_name"].as_str().unwrap_or_default();
                let input = &h["tool_input"];
                let at = self
                    .calls
                    .iter()
                    .position(|(_, n, i)| n == name && i == input)
                    .or_else(|| self.calls.iter().position(|(_, n, _)| n == name));
                if let Some(at) = at {
                    let (id, name, input) = self.calls.remove(at);
                    out.push(Item::Goes);
                    let (said, failed) = response_text(&h["tool_response"]);
                    let mut end = acp_call(&name, &input, &h["mcp_context"]);
                    end["sessionUpdate"] = json!("tool_call_update");
                    end["toolCallId"] = json!(id);
                    end["status"] = json!(if failed { "failed" } else { "completed" });
                    end["rawOutput"] = output(&end["kind"], &said, failed);
                    if end["kind"] == "execute" && !failed {
                        end["rawOutput"] = json!({ "stdout": said, "content": said });
                    }
                    out.extend(self.acp.update(end));
                }
            }
            "Notification" if h["notification_type"] == "ToolPermission" => {
                out.push(Item::Waits {
                    body: h["message"].as_str().unwrap_or_default().to_string(),
                });
            }
            "AfterAgent" => {
                let text = h["prompt_response"].as_str().unwrap_or_default();
                if !text.trim().is_empty() {
                    out.extend(self.acp.say(text));
                }
                self.calls.clear();
                out.push(Item::ViewEnd {
                    interrupted: false,
                    duration: None,
                    error: None,
                });
            }
            _ => {}
        }
    }
}

/// What a call of `kind` said, in Cursor's form: a command's output and code, its refusal on its error.
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

/// The text the user typed, out of what Cursor's interface wrapped it in.
pub fn typed_text(text: &str) -> String {
    let mut t = text.trim();
    // The interface puts what it expanded (the file of a command typed: `<cursor_commands>`) before the query.
    if let Some((_, rest)) = t.split_once("<user_query>") {
        t = rest.rsplit_once("</user_query>").map_or(rest, |(q, _)| q);
        return t.trim().to_string();
    }
    if let Some(rest) = t.strip_prefix("<timestamp>") {
        t = rest
            .split_once("</timestamp>")
            .map_or(rest, |(_, r)| r)
            .trim();
    }
    if let Some(rest) = t.strip_prefix("<user_query>") {
        t = rest.strip_suffix("</user_query>").unwrap_or(rest);
    }
    t.trim().to_string()
}

/// A tool of Cursor's interface under the name Claude Code gives it, its input as Escouade shows it.
pub fn tui_tool(name: &str, input: &Value) -> (String, Value) {
    let path = input["path"].clone();
    match name {
        "Shell" => (
            "Bash".into(),
            json!({ "command": input["command"], "description": input["description"] }),
        ),
        "Read" => ("Read".into(), json!({ "file_path": path })),
        "Write" => ("Write".into(), json!({ "file_path": path })),
        "StrReplace" | "Edit" | "MultiEdit" => ("Edit".into(), json!({ "file_path": path })),
        "Delete" => ("Delete".into(), json!({ "file_path": path })),
        "Grep" => (
            "Grep".into(),
            json!({ "pattern": input["pattern"], "path": path }),
        ),
        "Glob" => (
            "Glob".into(),
            json!({ "pattern": input["glob_pattern"].as_str().or(input["pattern"].as_str()) }),
        ),
        "WebFetch" => ("WebFetch".into(), json!({ "url": input["url"] })),
        "Task" => ("Task".into(), input.clone()),
        _ => (name.to_string(), input.clone()),
    }
}

/// The tools of Vibe shown otherwise (its question, its tasks) or not at all (its search of tools).
const UNSHOWN: [&str; 3] = ["ask_user_question", "todo", "search_tool_functions"];

/// A tool of Vibe's journal (`file_system.bash`, `read_file`, `mcp_escouade.get_board`…) under the name Claude Code gives
/// it, its input as Escouade shows it; None for a tool not shown.
pub fn journal_tool(name: &str, input: &Value, cwd: &str) -> Option<(String, Value)> {
    if let Some(rest) = name.strip_prefix("mcp_") {
        if let Some((server, tool)) = rest.split_once('.') {
            return Some((format!("mcp__{server}__{tool}"), input.clone()));
        }
    }
    let bare = name.rsplit('.').next().unwrap_or(name);
    if UNSHOWN.contains(&bare) {
        return None;
    }
    // Its file, as its tools name it (`file_path`, `filePath` in its public state), from the agent's folder.
    let file = ["file_path", "filePath", "path"]
        .iter()
        .find_map(|k| input[*k].as_str())
        .map(|p| json!(absolute(p, cwd)))
        .unwrap_or(Value::Null);
    Some(match bare {
        "bash" | "git_bash" | "powershell" | "windows_shell" => {
            ("Bash".into(), json!({ "command": input["command"] }))
        }
        "read_file" => ("Read".into(), json!({ "file_path": file.clone() })),
        "write_file" => ("Write".into(), json!({ "file_path": file.clone() })),
        "edit" | "search_replace" => ("Edit".into(), json!({ "file_path": file.clone() })),
        "grep" => (
            "Grep".into(),
            json!({ "pattern": input["pattern"], "path": input["path"] }),
        ),
        "web_fetch" => ("WebFetch".into(), json!({ "url": input["url"] })),
        "web_search" => ("WebSearch".into(), json!({ "query": input["query"] })),
        "task" | "spawn" => (
            "Task".into(),
            json!({ "description": input["task"], "prompt": input["task"], "subagent_type": input["agent"] }),
        ),
        _ => (bare.to_string(), input.clone()),
    })
}

/// The texts of an entry's `content` blocks, joined.
fn content_text(entry: &Value) -> String {
    entry["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| b["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// A callback Vibe waits on (`callback`: a permission, `detail.kind: approval`, or a question, `user_input`): the card says
/// it waits, and what for.
fn vibe_wait(c: &Value, cwd: &str, out: &mut Vec<Item>) {
    let detail = &c["detail"];
    let title = || c["title"].as_str().unwrap_or_default().to_string();
    out.push(Item::Runs);
    match detail["kind"].as_str() {
        Some("approval") => {
            let effect = &detail["effect"];
            let tool = effect["toolName"].as_str().unwrap_or_default();
            out.push(match journal_tool(tool, &effect["input"], cwd) {
                Some((tool, input)) => Item::WaitsOn { tool, input },
                None => Item::Waits { body: title() },
            });
        }
        _ => out.push(Item::Waits {
            body: detail["request"]["questions"][0]["question"]
                .as_str()
                .map_or_else(title, str::to_string),
        }),
    }
}

/// A path of Gemini's tools' input.
fn path_in(input: &Value) -> Option<&str> {
    ["file_path", "absolute_path", "dir_path", "path"]
        .iter()
        .find_map(|k| input[*k].as_str())
}

/// A call of Gemini's tool `name` (its `input`; `mcp`: its server's context, for an MCP tool) in ACP's form.
pub fn acp_call(name: &str, input: &Value, mcp: &Value) -> Value {
    let path = path_in(input);
    let (kind, raw) = if mcp.is_object() {
        (
            "other",
            json!({
                "providerIdentifier": mcp["server_name"], "toolName": mcp["tool_name"].as_str().unwrap_or(name),
                "args": input,
            }),
        )
    } else {
        match name {
            "run_shell_command" => ("execute", json!({ "command": input["command"] })),
            "write_file" | "replace" => ("edit", json!({ "path": path })),
            "read_file" | "read_many_files" => ("read", json!({ "path": path })),
            "list_directory" | "glob" | "grep_search" | "search_file_content" => (
                "search",
                json!({ "pattern": input["pattern"].as_str().unwrap_or_default(), "path": path }),
            ),
            "web_fetch" | "google_web_search" => (
                "fetch",
                json!({ "url": input["url"].as_str().or(input["query"].as_str()).or(input["prompt"].as_str()) }),
            ),
            "invoke_agent" => (
                "other",
                json!({
                    "_toolName": "task",
                    "description": input["agent_name"].as_str().unwrap_or("subagent"),
                    "prompt": input["prompt"].as_str().or(input["query"].as_str()),
                    "subagentType": input["agent_name"],
                }),
            ),
            _ => {
                let mut raw = if input.is_object() {
                    input.clone()
                } else {
                    json!({})
                };
                raw["_toolName"] = json!(name);
                ("other", raw)
            }
        }
    };
    let mut call = json!({ "kind": kind, "title": name, "rawInput": raw });
    let diff = match (name, path) {
        ("write_file", Some(p)) => input["content"]
            .as_str()
            .map(|t| json!({ "type": "diff", "path": p, "oldText": null, "newText": t })),
        ("replace", Some(p)) => {
            match (input["old_string"].as_str(), input["new_string"].as_str()) {
                (Some(o), Some(n)) => {
                    Some(json!({ "type": "diff", "path": p, "oldText": o, "newText": n }))
                }
                _ => None,
            }
        }
        _ => None,
    };
    if let Some(d) = diff {
        call["content"] = json!([d]);
    }
    call
}

/// What a call answered (`tool_response`: its `llmContent`, a text or parts, else what it showed), and whether it failed.
pub fn response_text(r: &Value) -> (String, bool) {
    let failed = !r["error"].is_null() && r.get("error").is_some();
    let text = match &r["llmContent"] {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p["text"].as_str().or(p.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => r["returnDisplay"].as_str().unwrap_or_default().to_string(),
    };
    let text = if failed && text.is_empty() {
        r["error"]["message"]
            .as_str()
            .or(r["error"].as_str())
            .unwrap_or("error")
            .to_string()
    } else {
        text
    };
    (text, failed)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
