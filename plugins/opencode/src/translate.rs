//! OpenCode's events (`opencode serve`, docs/PROTOCOL.md, « OpenCode serve ») translated into the items of the agent's
//! conversation (`escouade_plugin_protocol::engine::Item`), which the core applies: the state of the translation stays
//! here, the agent's in the core.
//!
//! Its tools take the names Claude Code gives them (docs/TODO-1.8.md, « Un moteur par agent »):
//! `bash` is `Bash`, `edit` `Edit`, `write` `Write`, `read` `Read`, `grep` `Grep`, `glob` `Glob`,
//! `list` `LS`, `patch` `Edit`, `webfetch` `WebFetch`, `todowrite` `TodoWrite`, `task` `Task`, and a
//! tool `escouade_<tool>` of Escouade's server `mcp__escouade__<tool>`. A subagent (`task`) has a
//! session of its own, whose events come on the same stream: they are told apart by their
//! `sessionID`, and shown under its `Task`. Its task list comes with `todo.updated`.
//!
//! A recorded stream (`tests/fixtures/opencode/`, one `{"dir", "msg"}` a line) is read as it came:
//! the events, and the session its `POST /session` made.

use crate::rules::{plan_approval, tr, SERVER};
use escouade_plugin_protocol::engine::{hunks_of, Item, TurnUsage};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

/// What the translation keeps of OpenCode's conversation between its events.
#[derive(Default)]
pub struct Translator {
    /// The agent's folder.
    cwd: String,
    /// The language of the lines it writes in the conversation.
    lang: String,
    /// The agent's model, its window read from `contexts`.
    model: String,
    /// The agent's session, once known.
    session: Option<String>,
    /// A turn is known to run: its first event says it to the core.
    active: bool,
    /// A compaction under way (ESC-205): its call to the model writes the summary the context
    /// holds alone afterwards.
    compacting: bool,
    /// The sessions of its subagents, and the call (`Task`) of each.
    children: HashMap<String, String>,
    /// Sessions of subagents not linked to their call yet.
    unlinked: Vec<String>,
    /// The role of each message (`user`, `assistant`): the parts of the user's are its own words.
    roles: HashMap<String, String>,
    /// The messages of the user typed in another client (`opencode attach`), which Escouade did not
    /// show when it sent them: their words are shown as they come.
    elsewhere: HashSet<String>,
    /// The kind of each part (`text`, `reasoning`), for its deltas.
    kinds: HashMap<String, String>,
    /// The tokens and tools of each subagent, by its call.
    child_tokens: HashMap<String, u64>,
    /// The context window of each model of the server (`limit.context`), by its id.
    contexts: HashMap<String, u64>,
    /// The steps already counted (`step-finish` parts, by id): one told again is not counted twice.
    steps: HashSet<String>,
    child_tools: HashMap<String, u64>,
    /// The agent's last message: where a copy forks it, and the plan of the mode Plan.
    last_message: Option<String>,
    /// Its session was seen busy since its last turn ended: its next `session.idle` ends this one.
    busy: bool,
    /// The turn was stopped (`MessageAbortedError`).
    aborted: bool,
    /// The user stopped it.
    interrupted: bool,
    /// What went wrong in the turn, as OpenCode said it first.
    error: Option<String>,
    /// The turn's usage so far, its subagents' included.
    usage: TurnUsage,
    /// The agent's last message was its plan agent's (`plan`): the turn ends on a plan to approve.
    plan_agent: bool,
    /// The last words of the turn after its last tool: the plan of a turn of the plan agent.
    final_text: String,
    /// The calls shown, the parts streaming and the lines said, so that one told again adds nothing.
    called: HashSet<String>,
    streaming: HashSet<String>,
    said: HashSet<String>,
}

/// Where an event of OpenCode belongs.
enum Owner {
    /// The agent's own session.
    Own,
    /// A subagent's, under its call.
    Child(String),
    /// Neither.
    Other,
}

impl Translator {
    /// The translation of an agent in `cwd` on `model`, in the session `session` when it resumes one.
    pub fn new(cwd: &str, model: &str, session: Option<String>, lang: &str) -> Self {
        Self {
            cwd: cwd.into(),
            model: model.into(),
            session,
            lang: lang.into(),
            ..Self::default()
        }
    }

    /// The user stopped the turn.
    pub(crate) fn interrupted(&mut self) {
        self.interrupted = true;
    }

    /// A prompt goes out: its failure is this turn's.
    pub(crate) fn began(&mut self) {
        self.active = true;
    }

    /// The agent's model changed.
    pub(crate) fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    /// Given again after the plugin started again, in a turn: its next `session.idle` ends it.
    pub(crate) fn in_turn(&mut self) {
        self.active = true;
        self.busy = true;
    }

    /// One event of its session (or a subagent's) on its `opencode serve`.
    pub fn frame(&mut self, f: &Value, out: &mut Vec<Item>) {
        // A recorded stream: the events, and the session its `POST /session` made.
        if let Some(dir) = f.get("dir").and_then(Value::as_str) {
            let msg = &f["msg"];
            if dir == "s2c" {
                if msg["event"].is_object() {
                    self.frame(&msg["event"], out);
                } else if msg["path"] == "/session" && msg["body"]["id"].is_string() {
                    let session = json!({ "type": "escouade.session", "properties": {
                        "sessionID": msg["body"]["id"], "info": msg["body"], "start": "new",
                    } });
                    self.frame(&session, out);
                }
            }
            return;
        }
        let Some(kind) = f["type"].as_str() else {
            return;
        };
        let p = &f["properties"];
        let sid = p["sessionID"]
            .as_str()
            .or(p["part"]["sessionID"].as_str())
            .or(p["info"]["sessionID"].as_str());
        let owner = self.owner(sid);
        let parent = match &owner {
            Owner::Child(call) => Some(call.clone()),
            _ => None,
        };
        let own = matches!(owner, Owner::Own);
        match kind {
            "escouade.session" => self.on_session(p, out),
            "escouade.model" => {
                if let Some(model) = p["model"].as_str() {
                    self.model = model.to_string();
                    out.push(Item::Model {
                        model: model.to_string(),
                    });
                }
            }
            "escouade.contexts" => {
                if let Some(all) = p["contexts"].as_object() {
                    self.contexts = all
                        .iter()
                        .filter_map(|(k, v)| Some((k.clone(), v.as_u64()?)))
                        .collect();
                }
                if let Some(window) = self.contexts.get(&self.model).copied() {
                    out.push(Item::Context {
                        tokens: None,
                        window: Some(window),
                    });
                }
            }
            "escouade.mode" => {
                if let Some(mode) = p["mode"].as_str() {
                    out.push(Item::Mode {
                        mode: mode.to_string(),
                    });
                }
            }
            "escouade.prompt.failed" if self.active => {
                self.error = p["message"].as_str().map(str::to_string);
                self.turn_end(out);
            }
            "session.created" => {
                let info = &p["info"];
                let (Some(child), Some(parent)) = (info["id"].as_str(), info["parentID"].as_str())
                else {
                    return;
                };
                if !matches!(self.owner(Some(parent)), Owner::Other)
                    && !self.children.contains_key(child)
                {
                    self.unlinked.push(child.to_string());
                }
            }
            "message.updated" => {
                let info = &p["info"];
                let Some(id) = info["id"].as_str() else {
                    return;
                };
                let role = info["role"].as_str().unwrap_or("assistant");
                self.roles.insert(id.to_string(), role.to_string());
                // A subagent's message tells the model it runs on, as the agent's is written.
                if let (Some(call), "assistant", Some(model)) =
                    (parent.as_deref(), role, info["modelID"].as_str())
                {
                    let model = match info["providerID"].as_str() {
                        Some(provider) => format!("{provider}/{model}"),
                        None => model.to_string(),
                    };
                    out.push(Item::Subagent {
                        call: call.to_string(),
                        model,
                    });
                }
                if own && role == "user" && p["typedElsewhere"] == true {
                    self.elsewhere.insert(id.to_string());
                }
                if own && role == "assistant" {
                    self.last_message = Some(id.to_string());
                    self.plan_agent = info["agent"] == "plan" || info["mode"] == "plan";
                    self.begins(out);
                    if let Some(e) = info["error"].as_object() {
                        self.on_error(&Value::Object(e.clone()));
                    }
                }
            }
            "message.part.updated" if !matches!(owner, Owner::Other) => {
                self.on_part(&p["part"], parent, out);
            }
            // A subagent's session not linked to its call yet: its cost is the turn's all the same.
            "message.part.updated"
                if p["part"]["type"] == "step-finish"
                    && sid.is_some_and(|s| self.unlinked.iter().any(|c| c == s)) =>
            {
                self.on_step(&p["part"]);
            }
            "message.part.delta" if !matches!(owner, Owner::Other) => {
                let (Some(part), Some(text)) = (p["partID"].as_str(), p["delta"].as_str()) else {
                    return;
                };
                if p["field"] != "text" || text.is_empty() {
                    return;
                }
                let msg = p["messageID"].as_str().unwrap_or_default();
                if self.roles.get(msg).is_some_and(|r| r == "user") {
                    return;
                }
                self.streaming.insert(part.to_string());
                out.push(Item::Delta {
                    id: part.to_string(),
                    text: text.to_string(),
                    thinking: self.kinds.get(part).is_some_and(|k| k == "reasoning"),
                    parent,
                });
            }
            "permission.asked" if !matches!(owner, Owner::Other) => {
                self.on_permission(p, parent, out)
            }
            "question.asked" if !matches!(owner, Owner::Other) => self.on_question(p, parent, out),
            "permission.replied" | "question.replied" | "question.rejected" => {
                // Answered elsewhere (`opencode attach`): its card says so and goes.
                let Some(rid) = p["requestID"].as_str() else {
                    return;
                };
                let (decision, answers) = match kind {
                    "permission.replied" => (
                        Some(match p["reply"].as_str() {
                            Some("always") => "always".to_string(),
                            Some("reject") => "deny".to_string(),
                            _ => "allow".to_string(),
                        }),
                        None,
                    ),
                    "question.replied" => (None, Some(p["answers"].clone())),
                    _ => (None, None),
                };
                out.push(Item::Answered {
                    request: rid.to_string(),
                    decision,
                    answers,
                });
            }
            "todo.updated" if own => {
                let todos: Vec<Value> = p["todos"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|t| {
                        let status = match t["status"].as_str() {
                            Some("in_progress") => "in_progress",
                            Some("completed" | "cancelled") => "completed",
                            _ => "pending",
                        };
                        json!({ "content": t["content"], "status": status, "activeForm": t["content"] })
                    })
                    .collect();
                out.push(Item::Todos {
                    todos: Value::Array(todos),
                });
            }
            "session.status" if own => {
                if p["status"]["type"] == "busy" {
                    self.busy = true;
                    self.begins(out);
                }
            }
            "session.compacted" if own => {
                self.compacting = false;
                out.push(Item::Notice {
                    text: tr(
                        &self.lang,
                        "Contexte compacté".into(),
                        "Context compacted".into(),
                    ),
                    level: "info".into(),
                });
            }
            "session.idle" if own => {
                if std::mem::take(&mut self.busy) {
                    self.turn_end(out);
                }
            }
            "session.error" if own => {
                if p["error"]["name"] == "MessageAbortedError" {
                    self.aborted = true;
                } else {
                    self.on_error(&p["error"]);
                }
            }
            _ => {}
        }
    }

    /// A turn runs: said once.
    fn begins(&mut self, out: &mut Vec<Item>) {
        if !std::mem::replace(&mut self.active, true) {
            out.push(Item::Start { session: None });
        }
    }

    /// Whose an event of the session `sid` is.
    fn owner(&self, sid: Option<&str>) -> Owner {
        let Some(s) = sid else {
            return Owner::Other;
        };
        if let Some(call) = self.children.get(s) {
            return Owner::Child(call.clone());
        }
        if self.unlinked.iter().any(|c| c == s) {
            return Owner::Other;
        }
        match self.session.as_deref() {
            Some(own) if own == s => Owner::Own,
            // Not known yet: the first session it hears of (a recorded stream).
            None => Owner::Own,
            _ => Owner::Other,
        }
    }

    /// The agent's session, new, resumed or copied.
    fn on_session(&mut self, p: &Value, out: &mut Vec<Item>) {
        let Some(id) = p["sessionID"].as_str().or(p["info"]["id"].as_str()) else {
            return;
        };
        let start = p["start"].as_str().unwrap_or("new");
        if self.session.as_deref() != Some(id) {
            if self.session.is_some() && start == "new" {
                out.push(Item::Notice {
                    text: tr(
                        &self.lang,
                        "La session OpenCode n’a pas pu être reprise : nouvelle session (contexte vidé)".into(),
                        "The OpenCode session couldn’t be resumed: new session (context cleared)".into(),
                    ),
                    level: "warn".into(),
                });
            }
            self.session = Some(id.to_string());
        }
        out.push(Item::Session {
            session: id.to_string(),
        });
    }

    /// What OpenCode said went wrong: its first message (a second one adds its stack).
    fn on_error(&mut self, error: &Value) {
        if self.error.is_some() || error["name"] == "MessageAbortedError" {
            return;
        }
        let message = error["data"]["message"]
            .as_str()
            .or(error["message"].as_str())
            .or(error["name"].as_str())
            .unwrap_or_default();
        let first = message.lines().next().unwrap_or_default().trim();
        if !first.is_empty() {
            self.error = Some(first.to_string());
        }
    }

    /// A part of a message, as it starts, grows and ends.
    fn on_part(&mut self, part: &Value, parent: Option<String>, out: &mut Vec<Item>) {
        let Some(id) = part["id"].as_str() else {
            return;
        };
        let msg = part["messageID"].as_str().unwrap_or_default();
        if self.roles.get(msg).is_some_and(|r| r == "user") {
            // A compaction starts, asked (`/compact`, ESC-205) or by OpenCode itself: a message
            // of the user's that holds a part `compaction` (`session.compacted` tells its end).
            if part["type"] == "compaction" {
                if parent.is_none() && self.said.insert(id.to_string()) {
                    self.compacting = true;
                    out.push(Item::Notice {
                        text: tr(
                            &self.lang,
                            "Compaction du contexte…".into(),
                            "Compacting the context…".into(),
                        ),
                        level: "info".into(),
                    });
                }
            } else if parent.is_none() && self.elsewhere.contains(msg) {
                // A text of a message typed in another client (`opencode attach`): the user's message
                // in the conversation, once (Escouade showed its own when it sent them).
                let text = part["text"].as_str().unwrap_or_default();
                if part["type"] == "text"
                    && part["synthetic"] != true
                    && !text.trim().is_empty()
                    && self.said.insert(id.to_string())
                {
                    out.push(Item::User {
                        id: id.to_string(),
                        text: text.to_string(),
                    });
                }
            }
            return;
        }
        let main = parent.is_none();
        let kind = part["type"].as_str().unwrap_or_default();
        self.kinds.insert(id.to_string(), kind.to_string());
        match kind {
            "text" | "reasoning" => {
                if part["synthetic"] == true || part["ignored"] == true {
                    return;
                }
                let text = part["text"].as_str().unwrap_or_default();
                let done = part["time"]["end"].is_number();
                if !done {
                    // Its first words make its item; the next say it goes on, its deltas telling them.
                    let first = self.streaming.insert(id.to_string());
                    out.push(Item::Delta {
                        id: id.to_string(),
                        text: if first {
                            text.to_string()
                        } else {
                            String::new()
                        },
                        thinking: kind == "reasoning",
                        parent,
                    });
                    return;
                }
                if kind == "text" && main && !text.trim().is_empty() {
                    self.final_text = text.to_string();
                }
                let (id, text) = (Some(id.to_string()), text.to_string());
                out.push(if kind == "text" {
                    Item::Text { text, parent, id }
                } else {
                    Item::Thinking { text, parent, id }
                });
            }
            "tool" => self.on_tool(part, parent, out),
            "step-finish" => {
                let Some((input, cache, output)) = self.on_step(part) else {
                    return;
                };
                match parent {
                    None => {
                        // After a compaction, its summary is what the context holds.
                        out.push(Item::Context {
                            tokens: Some(if self.compacting {
                                output
                            } else {
                                input + cache
                            }),
                            window: self.contexts.get(&self.model).copied(),
                        });
                    }
                    Some(call) => {
                        *self.child_tokens.entry(call).or_default() += input + cache + output;
                    }
                }
            }
            "patch" => {
                for f in part["files"].as_array().into_iter().flatten() {
                    if let Some(path) = f.as_str() {
                        out.push(Item::Touched {
                            path: path.to_string(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    /// A call to the model over (a part `step-finish`): its tokens and its cost added to the turn's,
    /// once (told again, a stream read anew, it is not counted twice). Its input, cache and output
    /// tokens; none when it was counted already.
    fn on_step(&mut self, part: &Value) -> Option<(u64, u64, u64)> {
        if let Some(pid) = part["id"].as_str() {
            if self.steps.len() > 4096 {
                self.steps.clear();
            }
            if !self.steps.insert(pid.to_string()) {
                return None;
            }
        }
        let t = &part["tokens"];
        let n = |v: &Value| v.as_u64().unwrap_or(0);
        let (input, output) = (n(&t["input"]), n(&t["output"]) + n(&t["reasoning"]));
        let cache = n(&t["cache"]["read"]) + n(&t["cache"]["write"]);
        let u = &mut self.usage;
        u.input += input;
        u.cache += cache;
        u.output += output;
        u.cost += part["cost"].as_f64().unwrap_or(0.0);
        Some((input, cache, output))
    }

    /// A part `tool`: its call shown once it runs, its result once it ends.
    fn on_tool(&mut self, part: &Value, parent: Option<String>, out: &mut Vec<Item>) {
        let Some(call) = part["callID"].as_str().or(part["id"].as_str()) else {
            return;
        };
        let tool = part["tool"].as_str().unwrap_or_default();
        let state = &part["state"];
        let status = state["status"].as_str().unwrap_or("pending");
        // Its card is the question's.
        if tool == "question" || status == "pending" {
            return;
        }
        let (name, input) = tool_of(tool, &state["input"], &self.cwd);
        if tool == "task" {
            if let Some(child) = state["metadata"]["sessionId"].as_str() {
                if !self.children.contains_key(child) {
                    self.unlinked.retain(|c| c != child);
                    self.children.insert(child.to_string(), call.to_string());
                }
            }
        }
        if self.called.insert(call.to_string()) {
            match &parent {
                // What was said before a tool was a step on the way, not the final reply.
                None => self.final_text.clear(),
                Some(call) => {
                    *self.child_tools.entry(call.clone()).or_default() += 1;
                }
            }
            out.push(Item::Tool {
                id: call.to_string(),
                name,
                input: input.clone(),
                parent,
                unplanned: tool == "todowrite",
            });
        }
        if !matches!(status, "completed" | "error") {
            return;
        }
        let meta = &state["metadata"];
        let (failed, mut text) = if status == "error" {
            (
                true,
                state["error"].as_str().unwrap_or_default().to_string(),
            )
        } else {
            let exit = meta["exit"].as_i64();
            (
                tool == "bash" && exit.is_some_and(|c| c != 0),
                state["output"].as_str().unwrap_or_default().to_string(),
            )
        };
        let path = input["file_path"].as_str().unwrap_or_default().to_string();
        let result = match tool {
            "edit" | "multiedit" if !failed => {
                let diff = meta["filediff"]["patch"]
                    .as_str()
                    .or(meta["diff"].as_str())
                    .unwrap_or_default();
                json!({ "filePath": path, "structuredPatch": hunks_of(diff) })
            }
            "write" if !failed => json!({
                "type": "create", "filePath": path, "content": input["content"], "structuredPatch": [],
            }),
            "task" => {
                if let Some(report) = tag(&text, "task_result") {
                    text = report.to_string();
                }
                json!({
                    "totalTokens": self.child_tokens.get(call),
                    "totalToolUseCount": self.child_tools.get(call),
                })
            }
            _ => Value::Null,
        };
        if !failed && matches!(tool, "edit" | "multiedit" | "write") && !path.is_empty() {
            out.push(Item::Touched { path });
        }
        out.push(Item::ToolDone {
            id: call.to_string(),
            output: text,
            error: failed,
            result,
        });
    }

    /// A permission OpenCode asks (`permission.asked`): a command, a change of files, the web, a
    /// path outside the agent's folder.
    fn on_permission(&mut self, p: &Value, parent: Option<String>, out: &mut Vec<Item>) {
        let Some(rid) = p["id"].as_str().map(str::to_string) else {
            return;
        };
        let call = p["tool"]["callID"].as_str().unwrap_or(&rid).to_string();
        let meta = &p["metadata"];
        let strings = |v: &Value| -> Vec<String> {
            v.as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        };
        let patterns = strings(&p["patterns"]);
        let always = strings(&p["always"]);
        let first = patterns.first().cloned().unwrap_or_default();
        let mut description: Option<String> = None;
        let mut hunks = None;
        let (tool, input) = match p["permission"].as_str().unwrap_or_default() {
            "bash" => {
                let command = meta["command"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| patterns.join(" "));
                ("Bash".to_string(), json!({ "command": command }))
            }
            "edit" | "write" => {
                let path = meta["filepath"]
                    .as_str()
                    .or(meta["filePath"].as_str())
                    .map(str::to_string)
                    .unwrap_or(first);
                hunks = meta["diff"].as_str().map(hunks_of);
                ("Edit".to_string(), json!({ "file_path": path }))
            }
            "webfetch" => ("WebFetch".to_string(), json!({ "url": first })),
            "external_directory" => {
                let list = patterns.join(", ");
                description = Some(tr(
                    &self.lang,
                    format!("Hors du dossier de l’agent : {list}"),
                    format!("Outside the agent’s folder: {list}"),
                ));
                (
                    "Permissions".to_string(),
                    json!({ "permissions": patterns }),
                )
            }
            other => tool_of(other, &json!({ "patterns": patterns }), &self.cwd),
        };
        // What « Toujours autoriser » keeps for the rest of the session, when it is not the request itself.
        if !always.is_empty() && always != patterns {
            let kept = always.join(", ");
            let line = tr(
                &self.lang,
                format!("« Toujours autoriser » retiendra : {kept}"),
                format!("“Always allow” will keep: {kept}"),
            );
            description = Some(match description {
                Some(d) => format!("{d}\n{line}"),
                None => line,
            });
        }
        out.push(Item::Permission {
            request: rid,
            call,
            tool: Some(tool),
            input: Some(input),
            description,
            reason: None,
            diff: None,
            can_always: !always.is_empty(),
            parent,
            hunks,
            always_by_harness: true,
            ends_stream: false,
        });
    }

    /// A question of OpenCode (`question.asked`, its `question` tool), as the card of a question
    /// of Claude Code's shows it.
    fn on_question(&mut self, p: &Value, parent: Option<String>, out: &mut Vec<Item>) {
        let Some(rid) = p["id"].as_str().map(str::to_string) else {
            return;
        };
        let call = p["tool"]["callID"].as_str().unwrap_or(&rid).to_string();
        let questions: Vec<Value> = p["questions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|q| {
                let options: Vec<Value> = q["options"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|o| json!({ "label": o["label"], "description": o["description"].as_str().unwrap_or_default() }))
                    .collect();
                json!({
                    "question": q["question"], "header": q["header"].as_str().unwrap_or_default(),
                    "multiSelect": q["multiple"] == true, "options": options,
                })
            })
            .collect();
        out.push(Item::Question {
            request: rid,
            call,
            questions: Value::Array(questions),
            parent,
        });
    }

    /// The end of the agent's turn (`session.idle`, or a prompt refused).
    fn turn_end(&mut self, out: &mut Vec<Item>) {
        let interrupted =
            std::mem::take(&mut self.interrupted) || std::mem::take(&mut self.aborted);
        let error = std::mem::take(&mut self.error).filter(|_| !interrupted);
        self.busy = false;
        let plan_agent = std::mem::take(&mut self.plan_agent);
        let last = self.last_message.clone();
        // The mode Plan's plan: the turn stays open on its card until it is approved, as Claude
        // Code's `ExitPlanMode` does (its work, or more planning, is the turn's next part).
        if plan_agent && !interrupted && error.is_none() {
            let plan = self.final_text.trim().to_string();
            if let (false, Some(msg)) = (plan.is_empty(), &last) {
                let rid = plan_approval(msg);
                if self.said.insert(rid.clone()) {
                    out.push(Item::Permission {
                        request: rid.clone(),
                        call: rid,
                        tool: Some("ExitPlanMode".into()),
                        input: Some(json!({ "plan": plan })),
                        description: None,
                        reason: None,
                        diff: None,
                        can_always: true,
                        parent: None,
                        hunks: None,
                        always_by_harness: true,
                        ends_stream: true,
                    });
                    return;
                }
            }
        }
        let entry = last.filter(|_| error.is_none());
        let usage = std::mem::take(&mut self.usage);
        self.child_tokens.clear();
        self.child_tools.clear();
        self.final_text.clear();
        self.active = false;
        out.push(Item::Turn {
            interrupted,
            limited: None,
            usage,
            error,
            entry,
        });
    }
}

/// The text between the tags `name` of `text`.
fn tag<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let (_, rest) = text.split_once(&format!("<{name}>"))?;
    let (inner, _) = rest.split_once(&format!("</{name}>"))?;
    Some(inner.trim()).filter(|s| !s.is_empty())
}

/// A path as OpenCode gave it, absolute (it may give one relative to the agent's folder). One of
/// Windows (`C:\…`, `\\server\…`) or of a POSIX system (`/…`) is absolute on every system, and a
/// relative one is joined with the separator of the agent's folder's kind: an OpenCode of WSL, or
/// a stream recorded elsewhere, reads the same.
pub fn absolute(path: &str, cwd: &str) -> String {
    let of_windows = |p: &str| {
        let b = p.as_bytes();
        (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/'))
            || p.starts_with('\\')
    };
    if path.is_empty() || cwd.is_empty() || of_windows(path) || path.starts_with('/') {
        return path.to_string();
    }
    let sep = if of_windows(cwd) { '\\' } else { '/' };
    format!("{}{sep}{path}", cwd.trim_end_matches(['/', '\\']))
}

/// The first file a patch of `patch` / `apply_patch` changes (`*** Update File: <path>`).
pub fn patch_file(text: &str) -> Option<&str> {
    text.lines().find_map(|l| {
        ["*** Update File: ", "*** Add File: ", "*** Delete File: "]
            .iter()
            .find_map(|p| l.strip_prefix(p))
            .map(str::trim)
    })
}

/// The tool Claude Code would have called for OpenCode's `tool`, and its input as Claude Code's.
pub fn tool_of(tool: &str, input: &Value, cwd: &str) -> (String, Value) {
    let path = |k: &str| absolute(input[k].as_str().unwrap_or_default(), cwd);
    let (name, input) = match tool {
        "bash" => (
            "Bash",
            json!({ "command": input["command"], "description": input["description"] }),
        ),
        "edit" | "multiedit" => (
            "Edit",
            json!({
                "file_path": path("filePath"), "old_string": input["oldString"],
                "new_string": input["newString"], "replace_all": input["replaceAll"],
            }),
        ),
        "write" => (
            "Write",
            json!({ "file_path": path("filePath"), "content": input["content"] }),
        ),
        "read" => (
            "Read",
            json!({ "file_path": path("filePath"), "offset": input["offset"], "limit": input["limit"] }),
        ),
        "grep" => (
            "Grep",
            json!({ "pattern": input["pattern"], "path": input["path"], "glob": input["include"] }),
        ),
        "glob" => (
            "Glob",
            json!({ "pattern": input["pattern"], "path": input["path"] }),
        ),
        "list" => ("LS", json!({ "path": input["path"] })),
        "patch" | "apply_patch" => {
            let text = input["patchText"].as_str().unwrap_or_default();
            let file = patch_file(text).map(|f| absolute(f, cwd));
            ("Edit", json!({ "file_path": file }))
        }
        "webfetch" => (
            "WebFetch",
            json!({ "url": input["url"], "prompt": input["format"] }),
        ),
        "websearch" => ("WebSearch", json!({ "query": input["query"] })),
        "todowrite" => {
            let todos: Vec<Value> = input["todos"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|t| json!({ "content": t["content"], "status": t["status"], "activeForm": t["content"] }))
                .collect();
            ("TodoWrite", json!({ "todos": todos }))
        }
        "task" => (
            "Task",
            json!({
                "description": input["description"], "prompt": input["prompt"],
                "subagent_type": input["subagent_type"],
            }),
        ),
        "skill" => ("Skill", json!({ "skill": input["name"] })),
        other => {
            // A tool of Escouade's server: `escouade_<tool>`.
            if let Some(t) = other.strip_prefix(&format!("{SERVER}_")) {
                return (format!("mcp__{SERVER}__{t}"), input.clone());
            }
            return (other.to_string(), input.clone());
        }
    };
    let mut input = input;
    if let Some(o) = input.as_object_mut() {
        o.retain(|_, v| !v.is_null());
    }
    (name.to_string(), input)
}

/// `BTreeMap` of contexts as the event `escouade.contexts` says them.
pub(crate) fn contexts_event(sid: &str, contexts: &BTreeMap<String, u64>) -> Value {
    json!({ "type": "escouade.contexts", "properties": { "sessionID": sid, "contexts": contexts } })
}
